use crate::model::{
    expr_id_u32, CodegenComponent, CodegenComponentDescriptor, CodegenComponentField,
    CodegenComponentTargetKind, CodegenDeclaration, CodegenDeclarationKind, CodegenElement,
    CodegenEntrypoint, CodegenExpression, CodegenExpressionKind, CodegenMatchArm, CodegenModule,
    CodegenModuleProvenance, CodegenParam, CodegenProgram, CodegenProperty, CodegenRecordField,
    CodegenReference, CodegenSourceEntry, CodegenStatement, CodegenTypeRef, CodegenUnionCase,
};
use crate::options::CodegenError;
use nx_api::{LibraryArtifact, ProgramArtifact};
use nx_diagnostics::{Diagnostic, Label, Severity, TextSpan};
use nx_hir::{
    ast, EffectiveField, ExprId, Item, LocalDefinitionId, LoweredModule, Name, Param,
    PreparedBinding, PreparedBindingOrigin, PreparedBindingTarget, PreparedItemKind,
    PreparedModule, PreparedNamespace, PropertyEntry, RecordField,
};
use nx_interpreter::{ResolvedItemKind, ResolvedModule, ResolvedModuleSource, RuntimeModuleId};
use nx_types::{ModuleArtifact, TypeEnvironment};
use rustc_hash::FxHashSet;

/// Builds a target-neutral code generation program from a resolved program artifact.
pub fn build_codegen_program(artifact: &ProgramArtifact) -> Result<CodegenProgram, CodegenError> {
    let static_errors = artifact
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .cloned()
        .collect::<Vec<_>>();
    if !static_errors.is_empty() {
        return Err(CodegenError::new(static_errors));
    }

    let mut diagnostics = Vec::new();
    let mut modules = Vec::new();
    for module in artifact.resolved_program.modules() {
        match build_module(artifact, module, &mut diagnostics) {
            Some(module) => modules.push(module),
            None => {}
        }
    }

    let mut entrypoints = artifact
        .resolved_program
        .entry_functions
        .iter()
        .map(|(name, reference)| CodegenEntrypoint {
            name: name.clone(),
            reference: reference_from_resolved_module(
                artifact,
                reference.module_id,
                reference.definition_id,
                name,
                reference.kind,
            ),
        })
        .collect::<Vec<_>>();
    entrypoints.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let mut component_entrypoints = artifact
        .resolved_program
        .entry_components
        .iter()
        .map(|(name, reference)| CodegenEntrypoint {
            name: name.clone(),
            reference: reference_from_resolved_module(
                artifact,
                reference.module_id,
                reference.definition_id,
                name,
                reference.kind,
            ),
        })
        .collect::<Vec<_>>();
    component_entrypoints.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    if !diagnostics.is_empty() {
        return Err(CodegenError::new(diagnostics));
    }

    let source_entries = artifact
        .source_entries()
        .into_iter()
        .map(|entry| CodegenSourceEntry {
            identity: entry.identity.to_string(),
            source: entry.source.to_string(),
        })
        .collect();

    Ok(CodegenProgram {
        fingerprint: artifact.fingerprint,
        modules,
        entrypoints,
        component_entrypoints,
        source_entries,
    })
}

fn build_module(
    artifact: &ProgramArtifact,
    module: &ResolvedModule,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenModule> {
    let Some(module_artifact) = module_artifact_for(artifact, module) else {
        diagnostics.push(missing_semantic_data_diagnostic(
            module,
            "module artifact",
            empty_span(),
        ));
        return None;
    };
    let Some(lowered_module) = module_artifact.lowered_module.as_ref() else {
        diagnostics.push(missing_semantic_data_diagnostic(
            module,
            "lowered module",
            empty_span(),
        ));
        return None;
    };

    let mut declarations = Vec::new();
    for (index, item) in lowered_module.items().iter().enumerate() {
        let definition_id = LocalDefinitionId::new(index as u32);
        let reference = reference_from_item(module.id, definition_id, item);
        if let Some(declaration) = build_declaration(
            artifact,
            module,
            lowered_module.as_ref(),
            &module_artifact.type_env,
            reference,
            item,
            diagnostics,
        ) {
            declarations.push(declaration);
        }
    }

    let mut imports = artifact
        .resolved_program
        .imported_items(module.id)
        .map(|imports| {
            imports
                .iter()
                .map(|(visible_name, reference)| {
                    reference_from_resolved_module(
                        artifact,
                        reference.module_id,
                        reference.definition_id,
                        visible_name,
                        reference.kind,
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    imports.sort_by(|lhs, rhs| {
        lhs.module_id
            .as_u32()
            .cmp(&rhs.module_id.as_u32())
            .then_with(|| lhs.definition_id.index().cmp(&rhs.definition_id.index()))
            .then_with(|| lhs.name.cmp(&rhs.name))
    });

    Some(CodegenModule {
        id: module.id,
        provenance: provenance(&module.source),
        declarations,
        imports,
    })
}

#[derive(Debug, Default)]
struct LexicalScope {
    scopes: Vec<FxHashSet<String>>,
}

impl LexicalScope {
    fn new() -> Self {
        Self {
            scopes: vec![FxHashSet::default()],
        }
    }

    fn push(&mut self) {
        self.scopes.push(FxHashSet::default());
    }

    fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn insert(&mut self, name: impl Into<String>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into());
        }
    }

    fn contains(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

fn build_declaration(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    reference: CodegenReference,
    item: &Item,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenDeclaration> {
    let span = item_span(item);
    let kind = match item {
        Item::Function(function) => {
            let mut scope = LexicalScope::new();
            for param in &function.params {
                scope.insert(param.name.as_str());
            }
            let Some(body) = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                function.body,
                &mut scope,
                diagnostics,
            ) else {
                return None;
            };
            CodegenDeclarationKind::Function {
                params: build_params(artifact, resolved_module, &function.params, diagnostics)?,
                body,
                return_type: type_env.get_expr_type(function.body).cloned(),
            }
        }
        Item::Value(value) => {
            let mut scope = LexicalScope::new();
            let Some(expr) = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                value.value,
                &mut scope,
                diagnostics,
            ) else {
                return None;
            };
            CodegenDeclarationKind::Value {
                value: expr,
                ty: type_env.get_expr_type(value.value).cloned(),
            }
        }
        Item::Enum(enum_def) => CodegenDeclarationKind::Enum {
            members: enum_def
                .members
                .iter()
                .map(|member| member.name.as_str().to_string())
                .collect(),
        },
        Item::Record(record) => CodegenDeclarationKind::Record {
            fields: build_record_fields(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                &record.properties,
                diagnostics,
            )?,
        },
        Item::Union(union_def) => CodegenDeclarationKind::Union {
            cases: union_def
                .cases
                .iter()
                .map(|case| {
                    Some(CodegenUnionCase {
                        name: case.name.as_str().to_string(),
                        fields: build_union_case_fields(
                            artifact,
                            resolved_module,
                            lowered_module,
                            type_env,
                            &case.fields,
                            diagnostics,
                        )?,
                        span: case.span,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        },
        Item::TypeAlias(_) => CodegenDeclarationKind::TypeAlias,
        Item::Component(component) => CodegenDeclarationKind::Component(build_component(
            artifact,
            resolved_module,
            lowered_module,
            type_env,
            component,
            diagnostics,
        )?),
    };

    Some(CodegenDeclaration {
        reference,
        span,
        kind,
    })
}

fn build_component(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    component: &nx_hir::Component,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenComponent> {
    let prepared = prepared_module_for(artifact, resolved_module);
    let contract = match nx_hir::effective_component_contract(&prepared, component) {
        Ok(contract) => contract,
        Err(error) => {
            diagnostics.push(component_resolution_diagnostic(resolved_module, &error));
            return None;
        }
    };

    let mut prop_scope = LexicalScope::new();
    let props = build_effective_component_fields(
        artifact,
        resolved_module,
        &contract.props,
        &mut prop_scope,
        diagnostics,
    )?;

    let mut state_scope = LexicalScope::new();
    for prop in &props {
        state_scope.insert(prop.name.as_str());
    }
    let state = build_declared_component_fields(
        artifact,
        resolved_module,
        lowered_module,
        type_env,
        &component.state,
        &mut state_scope,
        diagnostics,
    )?;

    let body = match component.body {
        Some(body) => {
            let mut body_scope = LexicalScope::new();
            for prop in &props {
                body_scope.insert(prop.name.as_str());
            }
            for field in &state {
                body_scope.insert(field.name.as_str());
            }
            Some(build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                body,
                &mut body_scope,
                diagnostics,
            )?)
        }
        None => None,
    };

    Some(CodegenComponent {
        is_abstract: component.is_abstract,
        is_external: component.is_external,
        props,
        state,
        body,
    })
}

fn build_params(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    params: &[Param],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenParam>> {
    params
        .iter()
        .map(|param| {
            Some(CodegenParam {
                name: param.name.as_str().to_string(),
                ty: param.ty.clone(),
                resolved_ty: build_type_ref(artifact, resolved_module, &param.ty, diagnostics)?,
                is_content: param.is_content,
                span: param.span,
            })
        })
        .collect()
}

fn build_effective_component_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    fields: &[EffectiveField],
    scope: &mut LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenComponentField>> {
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(owner_module) = artifact
            .resolved_program
            .module_by_prepared_identity(&field.module_identity)
        else {
            diagnostics.push(missing_semantic_data_diagnostic(
                resolved_module,
                &format!("component field owner module '{}'", field.module_identity),
                field.span,
            ));
            return None;
        };
        let default = match field.default.as_ref() {
            Some(default) => Some(build_expression_for_module_identity(
                artifact,
                resolved_module,
                &default.module_identity,
                default.expr_id,
                scope,
                diagnostics,
            )?),
            None => None,
        };
        mapped.push(CodegenComponentField {
            name: field.name.as_str().to_string(),
            ty: field.ty.clone(),
            resolved_ty: build_type_ref(artifact, owner_module, &field.ty, diagnostics)?,
            is_content: field.is_content,
            is_required: field.is_required,
            default,
            owner_module_id: owner_module.id,
            span: field.span,
        });
        scope.insert(field.name.as_str());
    }
    Some(mapped)
}

fn build_declared_component_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    fields: &[RecordField],
    scope: &mut LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenComponentField>> {
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let default = match field.default {
            Some(default) => Some(build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                default,
                scope,
                diagnostics,
            )?),
            None => None,
        };
        mapped.push(CodegenComponentField {
            name: field.name.as_str().to_string(),
            ty: field.ty.clone(),
            resolved_ty: build_type_ref(artifact, resolved_module, &field.ty, diagnostics)?,
            is_content: field.is_content,
            is_required: field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_)),
            default,
            owner_module_id: resolved_module.id,
            span: field.span,
        });
        scope.insert(field.name.as_str());
    }
    Some(mapped)
}

fn build_expression_for_module_identity(
    artifact: &ProgramArtifact,
    diagnostic_module: &ResolvedModule,
    module_identity: &str,
    expr_id: ExprId,
    scope: &mut LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpression> {
    let Some(owner_module) = artifact
        .resolved_program
        .module_by_prepared_identity(module_identity)
    else {
        diagnostics.push(missing_semantic_data_diagnostic(
            diagnostic_module,
            &format!("expression owner module '{}'", module_identity),
            empty_span(),
        ));
        return None;
    };
    let Some(module_artifact) = module_artifact_for(artifact, owner_module) else {
        diagnostics.push(missing_semantic_data_diagnostic(
            owner_module,
            "module artifact",
            empty_span(),
        ));
        return None;
    };
    let Some(lowered_module) = module_artifact.lowered_module.as_ref() else {
        diagnostics.push(missing_semantic_data_diagnostic(
            owner_module,
            "lowered module",
            empty_span(),
        ));
        return None;
    };
    build_expression(
        artifact,
        owner_module,
        lowered_module.as_ref(),
        &module_artifact.type_env,
        expr_id,
        scope,
        diagnostics,
    )
}

fn build_record_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    fields: &[nx_hir::RecordField],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenRecordField>> {
    let mut scope = LexicalScope::new();
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let default = match field.default {
            Some(default) => Some(build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                default,
                &mut scope,
                diagnostics,
            )?),
            None => None,
        };
        mapped.push(CodegenRecordField {
            name: field.name.as_str().to_string(),
            ty: field.ty.clone(),
            resolved_ty: build_type_ref(artifact, resolved_module, &field.ty, diagnostics)?,
            is_content: field.is_content,
            is_required: field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_)),
            default,
            span: field.span,
        });
        scope.insert(field.name.as_str());
    }
    Some(mapped)
}

fn build_union_case_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    fields: &[nx_hir::UnionCaseField],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenRecordField>> {
    let mut scope = LexicalScope::new();
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let default = match field.default {
            Some(default) => Some(build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                default,
                &mut scope,
                diagnostics,
            )?),
            None => None,
        };
        mapped.push(CodegenRecordField {
            name: field.name.as_str().to_string(),
            ty: field.ty.clone(),
            resolved_ty: build_type_ref(artifact, resolved_module, &field.ty, diagnostics)?,
            is_content: field.is_content,
            is_required: field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_)),
            default,
            span: field.span,
        });
        scope.insert(field.name.as_str());
    }
    Some(mapped)
}

fn build_expression(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    expr_id: ExprId,
    scope: &mut LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpression> {
    let expr = lowered_module.expr(expr_id);
    let span = expr.span();
    let ty = type_env.get_expr_type(expr_id).cloned();
    let kind = match expr {
        ast::Expr::Literal(literal) => CodegenExpressionKind::Literal(literal.clone()),
        ast::Expr::Ident(name) => CodegenExpressionKind::Identifier {
            name: name.as_str().to_string(),
            reference: if scope.contains(name.as_str()) {
                None
            } else {
                resolve_visible_reference(artifact, resolved_module.id, name.as_str())
            },
        },
        ast::Expr::BinaryOp { lhs, op, rhs, .. } => {
            let lhs = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *lhs,
                scope,
                diagnostics,
            )?;
            let rhs = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *rhs,
                scope,
                diagnostics,
            )?;
            CodegenExpressionKind::Binary {
                lhs: Box::new(lhs),
                op: *op,
                rhs: Box::new(rhs),
            }
        }
        ast::Expr::UnaryOp { op, expr, .. } => {
            let expr = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *expr,
                scope,
                diagnostics,
            )?;
            CodegenExpressionKind::Unary {
                op: *op,
                expr: Box::new(expr),
            }
        }
        ast::Expr::Call { func, args, .. } => {
            let callee = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *func,
                scope,
                diagnostics,
            )?;
            let mut mapped_args = Vec::with_capacity(args.len());
            for arg in args {
                mapped_args.push(build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *arg,
                    scope,
                    diagnostics,
                )?);
            }
            CodegenExpressionKind::Call {
                callee: Box::new(callee),
                args: mapped_args,
            }
        }
        ast::Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let condition = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *condition,
                scope,
                diagnostics,
            )?;
            let then_branch = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *then_branch,
                scope,
                diagnostics,
            )?;
            let else_branch = match else_branch {
                Some(expr) => Some(Box::new(build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *expr,
                    scope,
                    diagnostics,
                )?)),
                None => None,
            };
            CodegenExpressionKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            }
        }
        ast::Expr::Match {
            scrutinee,
            arms,
            else_branch,
            ..
        } => {
            let scrutinee = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *scrutinee,
                scope,
                diagnostics,
            )?;
            let mut mapped_arms = Vec::with_capacity(arms.len());
            for arm in arms {
                let mut patterns = Vec::with_capacity(arm.patterns.len());
                for pattern in &arm.patterns {
                    patterns.push(build_expression(
                        artifact,
                        resolved_module,
                        lowered_module,
                        type_env,
                        *pattern,
                        scope,
                        diagnostics,
                    )?);
                }
                let body = build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    arm.body,
                    scope,
                    diagnostics,
                )?;
                mapped_arms.push(CodegenMatchArm { patterns, body });
            }
            let else_branch = match else_branch {
                Some(expr) => Some(Box::new(build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *expr,
                    scope,
                    diagnostics,
                )?)),
                None => None,
            };
            CodegenExpressionKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: mapped_arms,
                else_branch,
            }
        }
        ast::Expr::Let {
            name, value, body, ..
        } => {
            let value = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *value,
                scope,
                diagnostics,
            )?;
            scope.push();
            scope.insert(name.as_str());
            let body = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *body,
                scope,
                diagnostics,
            );
            scope.pop();
            let body = body?;
            CodegenExpressionKind::Let {
                name: name.as_str().to_string(),
                value: Box::new(value),
                body: Box::new(body),
            }
        }
        ast::Expr::Block { stmts, expr, .. } => {
            let mut statements = Vec::with_capacity(stmts.len());
            scope.push();
            for stmt in stmts {
                statements.push(match stmt {
                    ast::Stmt::Let {
                        name, init, span, ..
                    } => {
                        let init = build_expression(
                            artifact,
                            resolved_module,
                            lowered_module,
                            type_env,
                            *init,
                            scope,
                            diagnostics,
                        )?;
                        scope.insert(name.as_str());
                        CodegenStatement::Let {
                            name: name.as_str().to_string(),
                            init,
                            span: *span,
                        }
                    }
                    ast::Stmt::Expr(expr, _) => CodegenStatement::Expr(build_expression(
                        artifact,
                        resolved_module,
                        lowered_module,
                        type_env,
                        *expr,
                        scope,
                        diagnostics,
                    )?),
                });
            }
            let expression = match expr {
                Some(expr) => Some(Box::new(build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *expr,
                    scope,
                    diagnostics,
                )?)),
                None => None,
            };
            scope.pop();
            CodegenExpressionKind::Block {
                statements,
                expression,
            }
        }
        ast::Expr::Array { elements, .. } => {
            let mut mapped_elements = Vec::with_capacity(elements.len());
            for element in elements {
                mapped_elements.push(build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *element,
                    scope,
                    diagnostics,
                )?);
            }
            CodegenExpressionKind::Array(mapped_elements)
        }
        ast::Expr::For {
            item,
            index,
            iterable,
            body,
            ..
        } => {
            let iterable = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *iterable,
                scope,
                diagnostics,
            )?;
            scope.push();
            scope.insert(item.as_str());
            if let Some(index) = index {
                scope.insert(index.as_str());
            }
            let body = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *body,
                scope,
                diagnostics,
            );
            scope.pop();
            let body = body?;
            CodegenExpressionKind::For {
                item: item.as_str().to_string(),
                index: index.as_ref().map(|name| name.as_str().to_string()),
                iterable: Box::new(iterable),
                body: Box::new(body),
            }
        }
        ast::Expr::Index { base, index, .. } => {
            let base = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *base,
                scope,
                diagnostics,
            )?;
            let index = build_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *index,
                scope,
                diagnostics,
            )?;
            CodegenExpressionKind::Index {
                base: Box::new(base),
                index: Box::new(index),
            }
        }
        ast::Expr::Member { base, member, .. } => {
            let combined_reference = qualified_member_reference(
                artifact,
                resolved_module.id,
                lowered_module,
                *base,
                member.as_str(),
                scope,
            );
            match build_union_case_for_member(
                artifact,
                resolved_module.id,
                lowered_module,
                *base,
                member.as_str(),
                scope,
                diagnostics,
            ) {
                UnionCaseLookup::Found {
                    union_reference,
                    case,
                } => {
                    return Some(CodegenExpression {
                        expr_id: expr_id_u32(expr_id),
                        span,
                        ty,
                        kind: CodegenExpressionKind::UnionCase {
                            union_reference,
                            case_name: case.name,
                            fields: case.fields,
                            properties: Vec::new(),
                            content_field: None,
                            content: Vec::new(),
                        },
                    });
                }
                UnionCaseLookup::Failed => {
                    return None;
                }
                UnionCaseLookup::Missing => {}
            }
            if let Some(enum_reference) =
                enum_member_reference(artifact, resolved_module.id, lowered_module, *base, scope)
            {
                CodegenExpressionKind::EnumMember {
                    enum_reference,
                    member: member.as_str().to_string(),
                }
            } else {
                let base = build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    *base,
                    scope,
                    diagnostics,
                )?;
                CodegenExpressionKind::Member {
                    base: Box::new(base),
                    member: member.as_str().to_string(),
                    reference: combined_reference,
                }
            }
        }
        ast::Expr::RecordLiteral {
            record, properties, ..
        } => {
            let mut mapped_properties = Vec::with_capacity(properties.len());
            for property in properties {
                mapped_properties.push(CodegenProperty {
                    name: property.name.as_str().to_string(),
                    value: build_expression(
                        artifact,
                        resolved_module,
                        lowered_module,
                        type_env,
                        property.value,
                        scope,
                        diagnostics,
                    )?,
                    span: property.span,
                });
            }
            let (record_name, fields) =
                record_literal_shape(artifact, resolved_module.id, record.as_str(), diagnostics)?;
            CodegenExpressionKind::Record {
                name: record_name,
                fields,
                properties: mapped_properties,
            }
        }
        ast::Expr::Element { element, .. } => {
            let Some(kind) = build_element_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *element,
                scope,
                diagnostics,
            ) else {
                return None;
            };
            kind
        }
        ast::Expr::ActionHandler { span, .. } => {
            diagnostics.push(unsupported_diagnostic(
                resolved_module,
                *span,
                "action-handler codegen is not supported by this non-reactive executable target",
            ));
            return None;
        }
        ast::Expr::Error(span) => {
            diagnostics.push(unsupported_diagnostic(
                resolved_module,
                *span,
                "malformed expressions cannot be emitted",
            ));
            return None;
        }
    };

    Some(CodegenExpression {
        expr_id: expr_id_u32(expr_id),
        span,
        ty,
        kind,
    })
}

fn build_element_expression(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    element_id: nx_hir::ElementId,
    scope: &mut LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpressionKind> {
    let element = lowered_module.element(element_id);
    let mut mapped = CodegenElement::from_id(element_id, &element.tag);
    for entry in element.property_entries() {
        match entry {
            PropertyEntry::Value(property) => mapped.properties.push(CodegenProperty {
                name: property.key.as_str().to_string(),
                value: build_expression(
                    artifact,
                    resolved_module,
                    lowered_module,
                    type_env,
                    property.value,
                    scope,
                    diagnostics,
                )?,
                span: property.span,
            }),
            PropertyEntry::If { span, .. }
            | PropertyEntry::ConditionList { span, .. }
            | PropertyEntry::Match { span, .. } => {
                diagnostics.push(unsupported_diagnostic(
                    resolved_module,
                    *span,
                    "conditional element property fragments are not supported by executable codegen yet",
                ));
                return None;
            }
        }
    }
    for content in &element.content {
        mapped.content.push(build_expression(
            artifact,
            resolved_module,
            lowered_module,
            type_env,
            *content,
            scope,
            diagnostics,
        )?);
    }
    mapped
        .properties
        .sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
    match build_union_case_for_tag(
        artifact,
        resolved_module.id,
        element.tag.as_str(),
        diagnostics,
    ) {
        UnionCaseLookup::Found {
            union_reference,
            case,
        } => {
            let content_field = case
                .fields
                .iter()
                .find(|field| field.is_content)
                .map(|field| field.name.clone());
            Some(CodegenExpressionKind::UnionCase {
                union_reference,
                case_name: case.name,
                fields: case.fields,
                properties: mapped.properties,
                content_field,
                content: mapped.content,
            })
        }
        UnionCaseLookup::Failed => None,
        UnionCaseLookup::Missing => {
            let Some(reference) =
                resolve_visible_reference(artifact, resolved_module.id, element.tag.as_str())
            else {
                return Some(CodegenExpressionKind::Element(mapped));
            };

            match reference.kind {
                ResolvedItemKind::Function => build_function_element_call(
                    artifact,
                    resolved_module,
                    element.span,
                    reference,
                    mapped.properties,
                    mapped.content,
                    diagnostics,
                ),
                ResolvedItemKind::Component => build_component_descriptor_expression(
                    artifact,
                    resolved_module,
                    reference,
                    mapped.properties,
                    mapped.content,
                    diagnostics,
                ),
                ResolvedItemKind::Record if mapped.content.is_empty() => {
                    let (record_name, fields) = record_literal_shape(
                        artifact,
                        resolved_module.id,
                        element.tag.as_str(),
                        diagnostics,
                    )?;
                    Some(CodegenExpressionKind::Record {
                        name: record_name,
                        fields,
                        properties: mapped.properties,
                    })
                }
                _ => Some(CodegenExpressionKind::Element(mapped)),
            }
        }
    }
}

fn build_function_element_call(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    element_span: TextSpan,
    function_reference: CodegenReference,
    properties: Vec<CodegenProperty>,
    content: Vec<CodegenExpression>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpressionKind> {
    let Some(params) =
        function_params_from_reference(artifact, resolved_module, &function_reference, diagnostics)
    else {
        return None;
    };

    let mut consumed = FxHashSet::default();
    let mut args = Vec::with_capacity(params.len());
    for param in &params {
        if let Some(property) = properties
            .iter()
            .find(|property| property.name == param.name)
        {
            consumed.insert(property.name.clone());
            args.push(property.value.clone());
        } else if param.is_content && !content.is_empty() {
            args.push(content_expression(content.clone(), element_span));
        } else {
            diagnostics.push(unsupported_diagnostic(
                resolved_module,
                element_span,
                format!(
                    "function element call '{}' is missing argument '{}'",
                    function_reference.name, param.name
                ),
            ));
            return None;
        }
    }

    if let Some(property) = properties
        .iter()
        .find(|property| !consumed.contains(&property.name))
    {
        diagnostics.push(unsupported_diagnostic(
            resolved_module,
            property.span,
            format!(
                "function element call '{}' has no parameter '{}'",
                function_reference.name, property.name
            ),
        ));
        return None;
    }

    Some(CodegenExpressionKind::Call {
        callee: Box::new(CodegenExpression {
            expr_id: 0,
            span: element_span,
            ty: None,
            kind: CodegenExpressionKind::Identifier {
                name: function_reference.name.clone(),
                reference: Some(function_reference),
            },
        }),
        args,
    })
}

fn build_component_descriptor_expression(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    component_reference: CodegenReference,
    properties: Vec<CodegenProperty>,
    content: Vec<CodegenExpression>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpressionKind> {
    let Some(target_module) = artifact
        .resolved_program
        .module(component_reference.module_id)
    else {
        diagnostics.push(missing_semantic_data_diagnostic(
            resolved_module,
            "component target module",
            empty_span(),
        ));
        return None;
    };
    let Some(Item::Component(component)) = target_module
        .lowered_module
        .item_by_definition(component_reference.definition_id)
    else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "component declaration",
            empty_span(),
        ));
        return None;
    };
    if component.is_abstract {
        diagnostics.push(unsupported_diagnostic(
            target_module,
            component.span,
            format!(
                "abstract component '{}' cannot be constructed by executable codegen",
                component_reference.name
            ),
        ));
        return None;
    }

    let prepared = prepared_module_for(artifact, target_module);
    let contract = match nx_hir::effective_component_contract(&prepared, component) {
        Ok(contract) => contract,
        Err(error) => {
            diagnostics.push(component_resolution_diagnostic(target_module, &error));
            return None;
        }
    };
    let content_field = contract
        .content_prop()
        .map(|field| field.name.as_str().to_string());

    Some(CodegenExpressionKind::ComponentDescriptor(
        CodegenComponentDescriptor {
            component: component_reference,
            target_kind: if component.is_external {
                CodegenComponentTargetKind::External
            } else {
                CodegenComponentTargetKind::Normal
            },
            properties,
            content_field,
            content,
        },
    ))
}

fn function_params_from_reference(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    reference: &CodegenReference,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenParam>> {
    let Some(target_module) = artifact.resolved_program.module(reference.module_id) else {
        diagnostics.push(missing_semantic_data_diagnostic(
            resolved_module,
            "function target module",
            empty_span(),
        ));
        return None;
    };
    let Some(Item::Function(function)) = target_module
        .lowered_module
        .item_by_definition(reference.definition_id)
    else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "function declaration",
            empty_span(),
        ));
        return None;
    };

    Some(
        function
            .params
            .iter()
            .map(|param| {
                Some(CodegenParam {
                    name: param.name.as_str().to_string(),
                    ty: param.ty.clone(),
                    resolved_ty: build_type_ref(artifact, target_module, &param.ty, diagnostics)?,
                    is_content: param.is_content,
                    span: param.span,
                })
            })
            .collect::<Option<Vec<_>>>()?,
    )
}

fn content_expression(content: Vec<CodegenExpression>, span: TextSpan) -> CodegenExpression {
    if content.len() == 1 {
        return content
            .into_iter()
            .next()
            .expect("content has one expression");
    }

    CodegenExpression {
        expr_id: 0,
        span,
        ty: None,
        kind: CodegenExpressionKind::Array(content),
    }
}

enum UnionCaseLookup {
    Missing,
    Failed,
    Found {
        union_reference: CodegenReference,
        case: CodegenUnionCase,
    },
}

fn build_union_case_for_member(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    lowered_module: &LoweredModule,
    base: ExprId,
    member: &str,
    scope: &LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnionCaseLookup {
    let ast::Expr::Ident(base_name) = lowered_module.expr(base) else {
        return UnionCaseLookup::Missing;
    };
    if scope.contains(base_name.as_str()) {
        return UnionCaseLookup::Missing;
    }
    let Some(reference) = resolve_visible_reference(artifact, module_id, base_name.as_str()) else {
        return UnionCaseLookup::Missing;
    };
    if reference.kind != nx_interpreter::ResolvedItemKind::Union {
        return UnionCaseLookup::Missing;
    }
    build_union_case_from_reference(artifact, reference, member, diagnostics)
}

fn build_union_case_for_tag(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    tag: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnionCaseLookup {
    let Some((union_name, case_name)) = tag.rsplit_once('.') else {
        return UnionCaseLookup::Missing;
    };
    let Some(reference) = resolve_visible_reference(artifact, module_id, union_name) else {
        return UnionCaseLookup::Missing;
    };
    if reference.kind != nx_interpreter::ResolvedItemKind::Union {
        return UnionCaseLookup::Missing;
    }
    build_union_case_from_reference(artifact, reference, case_name, diagnostics)
}

fn build_union_case_from_reference(
    artifact: &ProgramArtifact,
    union_reference: CodegenReference,
    case_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnionCaseLookup {
    let Some(target_module) = artifact.resolved_program.module(union_reference.module_id) else {
        return UnionCaseLookup::Missing;
    };
    let Some(module_artifact) = module_artifact_for(artifact, target_module) else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "module artifact",
            empty_span(),
        ));
        return UnionCaseLookup::Failed;
    };
    let Some(lowered_module) = module_artifact.lowered_module.as_ref() else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "lowered module",
            empty_span(),
        ));
        return UnionCaseLookup::Failed;
    };
    let Some(Item::Union(union_def)) =
        lowered_module.item_by_definition(union_reference.definition_id)
    else {
        return UnionCaseLookup::Missing;
    };
    let Some(case_def) = union_def
        .cases
        .iter()
        .find(|case| case.name.as_str() == case_name)
    else {
        return UnionCaseLookup::Missing;
    };
    let Some(fields) = build_union_case_fields(
        artifact,
        target_module,
        lowered_module.as_ref(),
        &module_artifact.type_env,
        &case_def.fields,
        diagnostics,
    ) else {
        return UnionCaseLookup::Failed;
    };
    UnionCaseLookup::Found {
        union_reference,
        case: CodegenUnionCase {
            name: case_def.name.as_str().to_string(),
            fields,
            span: case_def.span,
        },
    }
}

fn record_literal_shape(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    record_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(String, Vec<CodegenRecordField>)> {
    let Some(reference) = resolve_visible_reference(artifact, module_id, record_name) else {
        return Some((record_name.to_string(), Vec::new()));
    };
    if reference.kind != nx_interpreter::ResolvedItemKind::Record {
        return Some((record_name.to_string(), Vec::new()));
    }

    let Some(target_module) = artifact.resolved_program.module(reference.module_id) else {
        return Some((record_name.to_string(), Vec::new()));
    };
    let Some(module_artifact) = module_artifact_for(artifact, target_module) else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "module artifact",
            empty_span(),
        ));
        return None;
    };
    let Some(lowered_module) = module_artifact.lowered_module.as_ref() else {
        diagnostics.push(missing_semantic_data_diagnostic(
            target_module,
            "lowered module",
            empty_span(),
        ));
        return None;
    };
    let Some(Item::Record(record_def)) = lowered_module.item_by_definition(reference.definition_id)
    else {
        return Some((record_name.to_string(), Vec::new()));
    };
    let fields = build_record_fields(
        artifact,
        target_module,
        lowered_module.as_ref(),
        &module_artifact.type_env,
        &record_def.properties,
        diagnostics,
    )?;
    Some((record_def.name.as_str().to_string(), fields))
}

fn build_type_ref(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    ty: &ast::TypeRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenTypeRef> {
    let prepared = prepared_module_for(artifact, resolved_module);
    build_type_ref_with_prepared(artifact, resolved_module, &prepared, ty, diagnostics)
}

fn build_type_ref_with_prepared(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared: &PreparedModule,
    ty: &ast::TypeRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenTypeRef> {
    match ty {
        ast::TypeRef::Name(name) => {
            if is_builtin_type_name(name.as_str()) {
                return Some(CodegenTypeRef::Primitive {
                    name: name.as_str().to_string(),
                });
            }

            let Some(binding) = prepared.resolve_binding(PreparedNamespace::Type, name) else {
                diagnostics.push(missing_semantic_data_diagnostic(
                    resolved_module,
                    &format!("type binding '{}'", name.as_str()),
                    empty_span(),
                ));
                return None;
            };
            let current_module_identity = resolved_module.prepared_module_identity();
            let target_module_identity = binding.module_identity(&current_module_identity);
            let Some(target_module) = artifact
                .resolved_program
                .module_by_prepared_identity(target_module_identity)
            else {
                diagnostics.push(missing_semantic_data_diagnostic(
                    resolved_module,
                    &format!("type module '{}'", target_module_identity),
                    empty_span(),
                ));
                return None;
            };
            let reference = reference_from_resolved_module(
                artifact,
                target_module.id,
                binding.definition_id(),
                name.as_str(),
                resolved_item_kind_from_prepared(binding.kind),
            );

            Some(CodegenTypeRef::Nominal {
                reference,
                display: name.as_str().to_string(),
            })
        }
        ast::TypeRef::Array(element) => Some(CodegenTypeRef::Array {
            element: Box::new(build_type_ref_with_prepared(
                artifact,
                resolved_module,
                prepared,
                element,
                diagnostics,
            )?),
        }),
        ast::TypeRef::Nullable(inner) => Some(CodegenTypeRef::Nullable {
            inner: Box::new(build_type_ref_with_prepared(
                artifact,
                resolved_module,
                prepared,
                inner,
                diagnostics,
            )?),
        }),
        ast::TypeRef::Function {
            params,
            return_type,
        } => Some(CodegenTypeRef::Function {
            params: params
                .iter()
                .map(|param| {
                    build_type_ref_with_prepared(
                        artifact,
                        resolved_module,
                        prepared,
                        param,
                        diagnostics,
                    )
                })
                .collect::<Option<Vec<_>>>()?,
            return_type: Box::new(build_type_ref_with_prepared(
                artifact,
                resolved_module,
                prepared,
                return_type,
                diagnostics,
            )?),
        }),
    }
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "i32" | "i64" | "int" | "f32" | "f64" | "float" | "string" | "bool" | "void" | "object"
    )
}

fn resolved_item_kind_from_prepared(kind: PreparedItemKind) -> ResolvedItemKind {
    match kind {
        PreparedItemKind::Function => ResolvedItemKind::Function,
        PreparedItemKind::Value => ResolvedItemKind::Value,
        PreparedItemKind::Component => ResolvedItemKind::Component,
        PreparedItemKind::TypeAlias => ResolvedItemKind::TypeAlias,
        PreparedItemKind::Enum => ResolvedItemKind::Enum,
        PreparedItemKind::Union => ResolvedItemKind::Union,
        PreparedItemKind::Record => ResolvedItemKind::Record,
    }
}

fn module_artifact_for<'a>(
    artifact: &'a ProgramArtifact,
    module: &ResolvedModule,
) -> Option<&'a ModuleArtifact> {
    match &module.source {
        ResolvedModuleSource::SourceProvider { identity } => artifact
            .root_modules
            .iter()
            .find(|candidate| candidate.file_name == *identity),
        ResolvedModuleSource::Library { module_path, .. } => artifact
            .libraries
            .iter()
            .find_map(|library| library_module_artifact(library, module_path)),
    }
}

fn library_module_artifact<'a>(
    library: &'a LibraryArtifact,
    module_path: &std::path::Path,
) -> Option<&'a ModuleArtifact> {
    library.modules.iter().find(|candidate| {
        candidate.file_name == module_path.display().to_string()
            || std::path::Path::new(&candidate.file_name) == module_path
    })
}

fn prepared_module_for(artifact: &ProgramArtifact, module: &ResolvedModule) -> PreparedModule {
    let mut prepared = PreparedModule::standalone(
        module.prepared_module_identity(),
        module.lowered_module.as_ref().clone(),
    );

    let Some(visible_items) = artifact.resolved_program.imported_items(module.id) else {
        return prepared;
    };

    for (visible_name, item_ref) in visible_items {
        let Some(target_module) = artifact.resolved_program.module(item_ref.module_id) else {
            continue;
        };
        let kind = prepared_item_kind(item_ref.kind);
        let target_module_identity = target_module.prepared_module_identity();
        prepared.add_peer_module(
            target_module_identity.clone(),
            target_module.lowered_module.clone(),
        );

        for namespace in kind.namespaces() {
            prepared.insert_binding(PreparedBinding {
                visible_name: Name::new(visible_name),
                namespace: *namespace,
                kind,
                origin: PreparedBindingOrigin::Peer {
                    module_identity: target_module_identity.clone(),
                },
                target: PreparedBindingTarget::Peer {
                    module_identity: target_module_identity.clone(),
                    definition_id: item_ref.definition_id,
                },
            });
        }
    }

    prepared
}

fn prepared_item_kind(kind: ResolvedItemKind) -> PreparedItemKind {
    match kind {
        ResolvedItemKind::Function => PreparedItemKind::Function,
        ResolvedItemKind::Value => PreparedItemKind::Value,
        ResolvedItemKind::Component => PreparedItemKind::Component,
        ResolvedItemKind::TypeAlias => PreparedItemKind::TypeAlias,
        ResolvedItemKind::Enum => PreparedItemKind::Enum,
        ResolvedItemKind::Union => PreparedItemKind::Union,
        ResolvedItemKind::Record => PreparedItemKind::Record,
    }
}

fn provenance(source: &ResolvedModuleSource) -> CodegenModuleProvenance {
    match source {
        ResolvedModuleSource::SourceProvider { identity } => {
            CodegenModuleProvenance::SourceProvider {
                identity: identity.clone(),
            }
        }
        ResolvedModuleSource::Library {
            root_path,
            module_path,
        } => CodegenModuleProvenance::Library {
            root_path: root_path.clone(),
            module_path: module_path.clone(),
        },
    }
}

fn resolve_visible_reference(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    visible_name: &str,
) -> Option<CodegenReference> {
    artifact
        .resolved_program
        .local_item(module_id, visible_name)
        .or_else(|| {
            artifact
                .resolved_program
                .imported_item(module_id, visible_name)
        })
        .map(|reference| {
            reference_from_resolved_module(
                artifact,
                reference.module_id,
                reference.definition_id,
                visible_name,
                reference.kind,
            )
        })
}

fn qualified_member_reference(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    lowered_module: &LoweredModule,
    base: ExprId,
    member: &str,
    scope: &LexicalScope,
) -> Option<CodegenReference> {
    let ast::Expr::Ident(base_name) = lowered_module.expr(base) else {
        return None;
    };
    if scope.contains(base_name.as_str()) {
        return None;
    }
    let visible_name = format!("{}.{}", base_name.as_str(), member);
    resolve_visible_reference(artifact, module_id, &visible_name)
}

fn enum_member_reference(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    lowered_module: &LoweredModule,
    base: ExprId,
    scope: &LexicalScope,
) -> Option<CodegenReference> {
    let ast::Expr::Ident(base_name) = lowered_module.expr(base) else {
        return None;
    };
    if scope.contains(base_name.as_str()) {
        return None;
    }
    let reference = resolve_visible_reference(artifact, module_id, base_name.as_str())?;
    (reference.kind == nx_interpreter::ResolvedItemKind::Enum).then_some(reference)
}

fn reference_from_item(
    module_id: RuntimeModuleId,
    definition_id: LocalDefinitionId,
    item: &Item,
) -> CodegenReference {
    CodegenReference {
        module_id,
        definition_id,
        name: item.name().as_str().to_string(),
        kind: resolved_item_kind(item),
    }
}

fn reference_from_resolved_module(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    definition_id: LocalDefinitionId,
    fallback_name: &str,
    kind: nx_interpreter::ResolvedItemKind,
) -> CodegenReference {
    let name = artifact
        .resolved_program
        .module(module_id)
        .and_then(|module| module.lowered_module.item_by_definition(definition_id))
        .map(|item| item.name().as_str().to_string())
        .unwrap_or_else(|| fallback_name.to_string());
    CodegenReference {
        module_id,
        definition_id,
        name,
        kind,
    }
}

fn item_span(item: &Item) -> TextSpan {
    match item {
        Item::Function(item) => item.span,
        Item::Value(item) => item.span,
        Item::Component(item) => item.span,
        Item::TypeAlias(item) => item.span,
        Item::Enum(item) => item.span,
        Item::Union(item) => item.span,
        Item::Record(item) => item.span,
    }
}

fn empty_span() -> TextSpan {
    TextSpan::new(0.into(), 0.into())
}

fn unsupported_diagnostic(
    module: &ResolvedModule,
    span: TextSpan,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error("codegen-unsupported-construct")
        .with_message(message)
        .with_label(Label::primary(module.prepared_module_identity(), span))
        .build()
}

fn missing_semantic_data_diagnostic(
    module: &ResolvedModule,
    missing: &str,
    span: TextSpan,
) -> Diagnostic {
    Diagnostic::error("codegen-missing-semantic-data")
        .with_message(format!(
            "Cannot build codegen program for '{}' because {} is unavailable",
            module.prepared_module_identity(),
            missing
        ))
        .with_label(Label::primary(module.prepared_module_identity(), span))
        .build()
}

fn component_resolution_diagnostic(
    module: &ResolvedModule,
    error: &nx_hir::ComponentResolutionError,
) -> Diagnostic {
    Diagnostic::error("codegen-missing-semantic-data")
        .with_message(format!(
            "Cannot build component codegen metadata: {}",
            error.message()
        ))
        .with_label(Label::primary(
            module.prepared_module_identity(),
            error.span(),
        ))
        .build()
}

fn resolved_item_kind(item: &Item) -> nx_interpreter::ResolvedItemKind {
    match item {
        Item::Function(_) => nx_interpreter::ResolvedItemKind::Function,
        Item::Value(_) => nx_interpreter::ResolvedItemKind::Value,
        Item::Component(_) => nx_interpreter::ResolvedItemKind::Component,
        Item::TypeAlias(_) => nx_interpreter::ResolvedItemKind::TypeAlias,
        Item::Enum(_) => nx_interpreter::ResolvedItemKind::Enum,
        Item::Union(_) => nx_interpreter::ResolvedItemKind::Union,
        Item::Record(_) => nx_interpreter::ResolvedItemKind::Record,
    }
}
