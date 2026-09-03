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
use rustc_hash::{FxHashMap, FxHashSet};

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
    let mut prepared_cache = PreparedModuleCache::default();
    for module in artifact.resolved_program.modules() {
        match build_module(artifact, module, &mut prepared_cache, &mut diagnostics) {
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
    prepared_cache: &mut PreparedModuleCache,
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
            prepared_cache,
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

#[derive(Debug, Default)]
struct PreparedModuleCache {
    modules: FxHashMap<u32, PreparedModule>,
}

impl PreparedModuleCache {
    fn get<'a>(
        &'a mut self,
        artifact: &ProgramArtifact,
        module: &ResolvedModule,
    ) -> &'a PreparedModule {
        let key = module.id.as_u32();
        if !self.modules.contains_key(&key) {
            let prepared = build_prepared_module_for(artifact, module);
            self.modules.insert(key, prepared);
        }

        self.modules
            .get(&key)
            .expect("prepared module should be cached")
    }
}

fn build_declaration(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
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
                prepared_cache,
                lowered_module,
                type_env,
                function.body,
                &mut scope,
                diagnostics,
            ) else {
                return None;
            };
            CodegenDeclarationKind::Function {
                params: build_params(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    &function.params,
                    diagnostics,
                )?,
                body,
                return_type: type_env.get_expr_type(function.body).cloned(),
            }
        }
        Item::Value(value) => {
            let mut scope = LexicalScope::new();
            let Some(expr) = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
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
        Item::Record(record) => {
            let shape = effective_record_shape_of(
                artifact,
                resolved_module,
                prepared_cache,
                record,
                diagnostics,
            )?;
            CodegenDeclarationKind::Record {
                fields: build_effective_record_fields(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    &shape.fields,
                    diagnostics,
                )?,
                bases: record_ancestor_references(
                    artifact,
                    resolved_module,
                    &shape.ancestors,
                    record.span,
                    diagnostics,
                )?,
                is_abstract: record.is_abstract,
            }
        }
        Item::Union(union_def) => {
            let mut cases = Vec::with_capacity(union_def.cases.len());
            for case in &union_def.cases {
                let fields = effective_union_case_fields(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    union_def,
                    case,
                    diagnostics,
                )?;
                cases.push(CodegenUnionCase {
                    name: case.name.as_str().to_string(),
                    fields: build_effective_record_fields(
                        artifact,
                        resolved_module,
                        prepared_cache,
                        &fields,
                        diagnostics,
                    )?,
                    is_constant: union_def.is_constant_case(case),
                    span: case.span,
                });
            }
            CodegenDeclarationKind::Union {
                cases,
                bases: union_base_references(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    union_def,
                    diagnostics,
                )?,
            }
        }
        Item::TypeAlias(_) => CodegenDeclarationKind::TypeAlias,
        Item::Component(component) => CodegenDeclarationKind::Component(build_component(
            artifact,
            resolved_module,
            prepared_cache,
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
    prepared_cache: &mut PreparedModuleCache,
    lowered_module: &LoweredModule,
    type_env: &TypeEnvironment,
    component: &nx_hir::Component,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenComponent> {
    let prepared = prepared_cache.get(artifact, resolved_module);
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
        prepared_cache,
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
        prepared_cache,
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
                prepared_cache,
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
    prepared_cache: &mut PreparedModuleCache,
    params: &[Param],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenParam>> {
    params
        .iter()
        .map(|param| {
            Some(CodegenParam {
                name: param.name.as_str().to_string(),
                ty: param.ty.clone(),
                resolved_ty: build_type_ref(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    &param.ty,
                    diagnostics,
                )?,
                is_content: param.is_content,
                span: param.span,
            })
        })
        .collect()
}

fn build_effective_component_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
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
                prepared_cache,
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
            resolved_ty: build_type_ref(
                artifact,
                owner_module,
                prepared_cache,
                &field.ty,
                diagnostics,
            )?,
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
    prepared_cache: &mut PreparedModuleCache,
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
                prepared_cache,
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
            resolved_ty: build_type_ref(
                artifact,
                resolved_module,
                prepared_cache,
                &field.ty,
                diagnostics,
            )?,
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
    prepared_cache: &mut PreparedModuleCache,
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
        prepared_cache,
        lowered_module.as_ref(),
        &module_artifact.type_env,
        expr_id,
        scope,
        diagnostics,
    )
}

/// The fields a record carries, its base's included.
///
/// <para>An inherited field is a field of the record: the interpreter materializes it, and a value
/// constructed from this declaration carries it. Emitting only the declared ones is what left the IR
/// runtime rejecting `name` on a `User` that extends a base declaring it.</para>
fn effective_record_shape_of(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    record: &nx_hir::RecordDef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<nx_hir::EffectiveRecordShape> {
    let prepared = prepared_cache.get(artifact, resolved_module);
    match nx_hir::effective_record_shape(prepared, record) {
        Ok(shape) => Some(shape),
        Err(error) => {
            diagnostics.push(record_resolution_diagnostic(resolved_module, &error));
            None
        }
    }
}

/// Maps a record's inheritance chain onto references a runtime can compare.
///
/// Fields are flattened before they reach the IR, so this chain answers only the question
/// flattening cannot: whether a value stamped with one record's name is acceptable where another
/// record is expected. Without it a runtime cannot tell a subtype from a foreign type, and has to
/// reject both.
fn record_ancestor_references(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    ancestors: &[nx_hir::RecordAncestor],
    span: TextSpan,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenReference>> {
    let mut mapped = Vec::with_capacity(ancestors.len());
    for ancestor in ancestors {
        // A base whose spelling reached no declaration is analysis's diagnostic to report, not
        // this pass's. Leaving it out keeps the chain to the bases a runtime can actually name.
        let Some(origin) = ancestor.origin.as_ref() else {
            continue;
        };
        let Some(base_module) = artifact
            .resolved_program
            .module_by_prepared_identity(origin.module_identity())
        else {
            diagnostics.push(missing_semantic_data_diagnostic(
                resolved_module,
                &format!("record base module '{}'", origin.module_identity()),
                span,
            ));
            return None;
        };
        mapped.push(CodegenReference {
            module_id: base_module.id,
            definition_id: origin.definition_id(),
            name: ancestor.name.as_str().to_string(),
            kind: ResolvedItemKind::Record,
        });
    }
    Some(mapped)
}

/// The fields one union case carries: its union's abstract base's first, then its own.
///
/// This is the order the interpreter materializes them in, and a base field a case overrides must
/// therefore come first here too.
fn effective_union_case_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    union_def: &nx_hir::UnionDef,
    case: &nx_hir::UnionCaseDef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<EffectiveField>> {
    let module_identity = resolved_module.prepared_module_identity();
    let mut fields = Vec::new();

    if let Some(base) = union_def.base.as_ref() {
        let prepared = prepared_cache.get(artifact, resolved_module);
        match nx_hir::effective_record_shape_for_name(prepared, base) {
            Ok(Some(shape)) => fields.extend(shape.fields),
            // A base that names nothing resolvable is analysis's diagnostic to report, not this
            // pass's; the case still emits the fields it declares itself.
            Ok(None) => {}
            Err(error) => {
                diagnostics.push(record_resolution_diagnostic(resolved_module, &error));
                return None;
            }
        }
    }

    fields.extend(case.fields.iter().map(|field| {
        EffectiveField::from_record_field(
            nx_hir::RecordField {
                name: field.name.clone(),
                ty: field.ty.clone(),
                is_content: field.is_content,
                default: field.default,
                span: field.span,
            },
            module_identity.clone(),
        )
    }));

    Some(fields)
}

/// The bases every case of a union inherits, nearest first.
///
/// A union extends at most one abstract record, so the chain is that base followed by the base's
/// own. Cases carry no separate chain: a case is acceptable wherever the union's base is.
fn union_base_references(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    union_def: &nx_hir::UnionDef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenReference>> {
    let Some(base) = union_def.base.as_ref() else {
        return Some(Vec::new());
    };

    let prepared = prepared_cache.get(artifact, resolved_module);
    let shape = match nx_hir::effective_record_shape_for_name(prepared, base) {
        Ok(Some(shape)) => shape,
        // A base that names nothing resolvable is analysis's diagnostic to report, the same way
        // `effective_union_case_fields` leaves it alone.
        Ok(None) => return Some(Vec::new()),
        Err(error) => {
            diagnostics.push(record_resolution_diagnostic(resolved_module, &error));
            return None;
        }
    };

    let mut ancestors = vec![nx_hir::RecordAncestor {
        name: base.clone(),
        origin: shape.origin.clone(),
    }];
    ancestors.extend(shape.ancestors.iter().cloned());
    record_ancestor_references(
        artifact,
        resolved_module,
        &ancestors,
        union_def.span,
        diagnostics,
    )
}

/// Builds IR fields from a record's effective ones, each default built in the module that wrote it.
///
/// A default may name a field declared before it, so the scope grows as the fields are walked —
/// the same order the interpreter materializes them in.
fn build_effective_record_fields(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    fields: &[EffectiveField],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CodegenRecordField>> {
    let mut scope = LexicalScope::new();
    let mut mapped = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(owner_module) = artifact
            .resolved_program
            .module_by_prepared_identity(&field.module_identity)
        else {
            diagnostics.push(missing_semantic_data_diagnostic(
                resolved_module,
                &format!("record field owner module '{}'", field.module_identity),
                field.span,
            ));
            return None;
        };
        let default = match field.default.as_ref() {
            Some(default) => Some(build_expression_for_module_identity(
                artifact,
                resolved_module,
                prepared_cache,
                &default.module_identity,
                default.expr_id,
                &mut scope,
                diagnostics,
            )?),
            None => None,
        };
        mapped.push(CodegenRecordField {
            name: field.name.as_str().to_string(),
            ty: field.ty.clone(),
            resolved_ty: build_type_ref(
                artifact,
                owner_module,
                prepared_cache,
                &field.ty,
                diagnostics,
            )?,
            is_content: field.is_content,
            is_required: field.is_required,
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
    prepared_cache: &mut PreparedModuleCache,
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
        // A case already resolved during analysis. The declaration is reached by the origin the
        // node carries, not by looking its union's name back up in the module using it — that name
        // need not be visible here at all.
        ast::Expr::ResolvedUnionCase {
            union,
            case,
            module_identity,
            definition_id,
            ..
        } => {
            match build_union_case_from_origin(
                artifact,
                prepared_cache,
                module_identity,
                *definition_id,
                union.as_str(),
                case.as_str(),
                diagnostics,
            ) {
                UnionCaseLookup::Found {
                    union_reference,
                    case,
                    union_is_constant,
                } => CodegenExpressionKind::UnionCase {
                    union_reference,
                    case_name: case.name,
                    is_constant: case.is_constant,
                    union_is_constant,
                    fields: case.fields,
                    properties: Vec::new(),
                    content_field: None,
                    content: Vec::new(),
                },
                UnionCaseLookup::Failed | UnionCaseLookup::Missing => {
                    diagnostics.push(
                        Diagnostic::error("unresolved-union-case-origin")
                            .with_message(format!(
                                "Union case '{}.{}' resolved during analysis but its declaration \
                                 in '{}' could not be reached during code generation",
                                union, case, module_identity
                            ))
                            .with_label(Label::primary(
                                resolved_module.prepared_module_identity(),
                                span,
                            ))
                            .build(),
                    );
                    return None;
                }
            }
        }
        ast::Expr::Ident(name) => {
            let in_scope = scope.contains(name.as_str());
            let reference = if in_scope {
                None
            } else {
                resolve_visible_reference(artifact, resolved_module.id, name.as_str())
            };
            if !in_scope && reference.is_none() {
                // A name that is neither a local nor a declaration used to emit a slot spelled
                // `unresolved:<name>`, which no runtime can bind — the program built, and failed
                // when it ran. Analysis reports such a name, so reaching here means analysis missed
                // it, and saying so beats emitting IR that cannot run.
                diagnostics.push(
                    Diagnostic::error("codegen-unresolved-name")
                        .with_message(format!(
                            "Name '{}' reaches no binding and no declaration, so it cannot be \
                             emitted",
                            name.as_str()
                        ))
                        .with_label(Label::primary(
                            resolved_module.prepared_module_identity(),
                            span,
                        ))
                        .build(),
                );
                return None;
            }
            CodegenExpressionKind::Identifier {
                name: name.as_str().to_string(),
                reference,
            }
        }
        ast::Expr::BinaryOp { lhs, op, rhs, .. } => {
            let lhs = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
                lowered_module,
                type_env,
                *lhs,
                scope,
                diagnostics,
            )?;
            let rhs = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
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
                prepared_cache,
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
                prepared_cache,
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
                    prepared_cache,
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
                prepared_cache,
                lowered_module,
                type_env,
                *condition,
                scope,
                diagnostics,
            )?;
            let then_branch = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
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
                    prepared_cache,
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
                prepared_cache,
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
                        prepared_cache,
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
                    prepared_cache,
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
                    prepared_cache,
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
                prepared_cache,
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
                prepared_cache,
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
                            prepared_cache,
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
                        prepared_cache,
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
                    prepared_cache,
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
                    prepared_cache,
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
                prepared_cache,
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
                prepared_cache,
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
                prepared_cache,
                lowered_module,
                type_env,
                *base,
                scope,
                diagnostics,
            )?;
            let index = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
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
                prepared_cache,
                lowered_module,
                *base,
                member.as_str(),
                scope,
                diagnostics,
            ) {
                UnionCaseLookup::Found {
                    union_reference,
                    case,
                    union_is_constant,
                } => {
                    return Some(CodegenExpression {
                        expr_id: expr_id_u32(expr_id),
                        span,
                        ty,
                        kind: CodegenExpressionKind::UnionCase {
                            union_reference,
                            case_name: case.name,
                            is_constant: case.is_constant,
                            union_is_constant,
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
            let mut base_diagnostics = Vec::new();
            let built = build_expression(
                artifact,
                resolved_module,
                prepared_cache,
                lowered_module,
                type_env,
                *base,
                scope,
                &mut base_diagnostics,
            );
            let base = match built {
                Some(base) => {
                    diagnostics.append(&mut base_diagnostics);
                    base
                }
                // An import alias binds one visible name, dots and all: `value as One.value` binds
                // `One.value`, and there is no `One` to take a member of. The whole name resolved
                // above, so the base here is a spelling rather than a value, and it is emitted as
                // the name it is.
                None if combined_reference.is_some() => {
                    match unbound_name_expression(lowered_module, *base) {
                        Some(base) => base,
                        None => {
                            diagnostics.append(&mut base_diagnostics);
                            return None;
                        }
                    }
                }
                None => {
                    diagnostics.append(&mut base_diagnostics);
                    return None;
                }
            };
            CodegenExpressionKind::Member {
                base: Box::new(base),
                member: member.as_str().to_string(),
                reference: combined_reference,
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
                        prepared_cache,
                        lowered_module,
                        type_env,
                        property.value,
                        scope,
                        diagnostics,
                    )?,
                    span: property.span,
                });
            }
            let (record_name, fields) = record_literal_shape(
                artifact,
                resolved_module.id,
                prepared_cache,
                record.as_str(),
                diagnostics,
            )?;
            CodegenExpressionKind::Record {
                name: record_name,
                fields,
                properties: mapped_properties,
                content_field: None,
                content: Vec::new(),
            }
        }
        ast::Expr::Element { element, .. } => {
            let Some(kind) = build_element_expression(
                artifact,
                resolved_module,
                lowered_module,
                type_env,
                *element,
                prepared_cache,
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
        // Type analysis resolves every contextual name before codegen runs, so one reaching here
        // means analysis was skipped rather than that the source is ambiguous.
        ast::Expr::ContextualName { span, .. } => {
            diagnostics.push(unsupported_diagnostic(
                resolved_module,
                *span,
                "unresolved contextual name cannot be emitted",
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
    prepared_cache: &mut PreparedModuleCache,
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
                    prepared_cache,
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
            prepared_cache,
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
        prepared_cache,
        element.tag.as_str(),
        diagnostics,
    ) {
        UnionCaseLookup::Found {
            union_reference,
            case,
            union_is_constant,
        } => {
            let content_field = case
                .fields
                .iter()
                .find(|field| field.is_content)
                .map(|field| field.name.clone());
            Some(CodegenExpressionKind::UnionCase {
                union_reference,
                case_name: case.name,
                is_constant: case.is_constant,
                union_is_constant,
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
                    prepared_cache,
                    element.span,
                    reference,
                    mapped.properties,
                    mapped.content,
                    diagnostics,
                ),
                ResolvedItemKind::Component => build_component_descriptor_expression(
                    artifact,
                    resolved_module,
                    prepared_cache,
                    reference,
                    mapped.properties,
                    mapped.content,
                    diagnostics,
                ),
                ResolvedItemKind::Record => {
                    let (record_name, fields) = record_literal_shape(
                        artifact,
                        resolved_module.id,
                        prepared_cache,
                        element.tag.as_str(),
                        diagnostics,
                    )?;
                    let content_field = fields
                        .iter()
                        .find(|field| field.is_content)
                        .map(|field| field.name.clone());
                    Some(CodegenExpressionKind::Record {
                        name: record_name,
                        fields,
                        properties: mapped.properties,
                        content_field,
                        content: mapped.content,
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
    prepared_cache: &mut PreparedModuleCache,
    element_span: TextSpan,
    function_reference: CodegenReference,
    properties: Vec<CodegenProperty>,
    content: Vec<CodegenExpression>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenExpressionKind> {
    let Some(params) = function_params_from_reference(
        artifact,
        resolved_module,
        prepared_cache,
        &function_reference,
        diagnostics,
    ) else {
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
    prepared_cache: &mut PreparedModuleCache,
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

    let prepared = prepared_cache.get(artifact, target_module);
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
    prepared_cache: &mut PreparedModuleCache,
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
                    resolved_ty: build_type_ref(
                        artifact,
                        target_module,
                        prepared_cache,
                        &param.ty,
                        diagnostics,
                    )?,
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
        /// Whether every case of the declaring union is constant.
        union_is_constant: bool,
    },
}

fn build_union_case_for_member(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    prepared_cache: &mut PreparedModuleCache,
    lowered_module: &LoweredModule,
    base: ExprId,
    member: &str,
    scope: &LexicalScope,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnionCaseLookup {
    let Some(base_name) = flattened_visible_name(lowered_module, base, scope) else {
        return UnionCaseLookup::Missing;
    };
    let Some(reference) = resolve_visible_reference(artifact, module_id, &base_name) else {
        return UnionCaseLookup::Missing;
    };
    if reference.kind != nx_interpreter::ResolvedItemKind::Union {
        return UnionCaseLookup::Missing;
    }
    build_union_case_from_reference(artifact, prepared_cache, reference, member, diagnostics)
}

/// Builds the name segments of a member chain as an expression, without resolving them.
///
/// This is for a base that is part of a name rather than a value of its own — the `One` of a
/// `One.value` import alias. Only plain names are spelled this way; anything else is a real
/// expression and is built as one.
fn unbound_name_expression(
    lowered_module: &LoweredModule,
    expr: ExprId,
) -> Option<CodegenExpression> {
    match lowered_module.expr(expr) {
        ast::Expr::Ident(name) => Some(CodegenExpression {
            expr_id: expr_id_u32(expr),
            span: lowered_module.expr(expr).span(),
            ty: None,
            kind: CodegenExpressionKind::Identifier {
                name: name.as_str().to_string(),
                reference: None,
            },
        }),
        ast::Expr::Member { base, member, .. } => Some(CodegenExpression {
            expr_id: expr_id_u32(expr),
            span: lowered_module.expr(expr).span(),
            ty: None,
            kind: CodegenExpressionKind::Member {
                base: Box::new(unbound_name_expression(lowered_module, *base)?),
                member: member.as_str().to_string(),
                reference: None,
            },
        }),
        _ => None,
    }
}

/// The whole name a member chain spells, when every segment is a plain name.
///
/// <para>An import alias binds one visible name, dots and all: `Fit as ui.Fit` binds `ui.Fit`, and
/// there is no `ui` to take a member of. Reading only a single-segment base leaves `ui.Fit.cover`
/// lowered as a member access on a name that reaches nothing, which is what put an `unresolved:`
/// slot and a null case reference into otherwise clean output.</para>
fn flattened_visible_name(
    lowered_module: &LoweredModule,
    expr: ExprId,
    scope: &LexicalScope,
) -> Option<String> {
    match lowered_module.expr(expr) {
        ast::Expr::Ident(name) => {
            if scope.contains(name.as_str()) {
                return None;
            }
            Some(name.as_str().to_string())
        }
        ast::Expr::Member { base, member, .. } => {
            let base = flattened_visible_name(lowered_module, *base, scope)?;
            Some(format!("{}.{}", base, member.as_str()))
        }
        _ => None,
    }
}

fn build_union_case_for_tag(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    prepared_cache: &mut PreparedModuleCache,
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
    build_union_case_from_reference(artifact, prepared_cache, reference, case_name, diagnostics)
}

/// Builds a union case from the declaring origin an analysis-resolved reference carries.
fn build_union_case_from_origin(
    artifact: &ProgramArtifact,
    prepared_cache: &mut PreparedModuleCache,
    module_identity: &str,
    definition_id: LocalDefinitionId,
    union_name: &str,
    case_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnionCaseLookup {
    let Some(target_module) = artifact
        .resolved_program
        .module_by_prepared_identity(module_identity)
    else {
        return UnionCaseLookup::Missing;
    };
    let reference = reference_from_resolved_module(
        artifact,
        target_module.id,
        definition_id,
        union_name,
        nx_interpreter::ResolvedItemKind::Union,
    );
    build_union_case_from_reference(artifact, prepared_cache, reference, case_name, diagnostics)
}

fn build_union_case_from_reference(
    artifact: &ProgramArtifact,
    prepared_cache: &mut PreparedModuleCache,
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
    let Some(effective) = effective_union_case_fields(
        artifact,
        target_module,
        prepared_cache,
        &union_def,
        case_def,
        diagnostics,
    ) else {
        return UnionCaseLookup::Failed;
    };
    let Some(fields) = build_effective_record_fields(
        artifact,
        target_module,
        prepared_cache,
        &effective,
        diagnostics,
    ) else {
        return UnionCaseLookup::Failed;
    };
    UnionCaseLookup::Found {
        union_reference,
        case: CodegenUnionCase {
            name: case_def.name.as_str().to_string(),
            fields,
            is_constant: union_def.is_constant_case(case_def),
            span: case_def.span,
        },
        union_is_constant: union_def.is_constant_union(),
    }
}

fn record_literal_shape(
    artifact: &ProgramArtifact,
    module_id: RuntimeModuleId,
    prepared_cache: &mut PreparedModuleCache,
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
    let shape = effective_record_shape_of(
        artifact,
        target_module,
        prepared_cache,
        &record_def,
        diagnostics,
    )?;
    let fields = build_effective_record_fields(
        artifact,
        target_module,
        prepared_cache,
        &shape.fields,
        diagnostics,
    )?;
    Some((record_def.name.as_str().to_string(), fields))
}

fn build_type_ref(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    ty: &ast::TypeRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenTypeRef> {
    let mut aliases = Vec::new();
    build_type_ref_resolving_aliases(
        artifact,
        resolved_module,
        prepared_cache,
        ty,
        &mut aliases,
        diagnostics,
    )
}

/// Builds one IR type reference, resolving any type alias it names down to what the alias stands
/// for.
///
/// <para>A type alias is transparent in NX — `type Ints = int[]` *is* a list — and the runtime that
/// reads this IR decides how to normalize a value from the shape of its type. An alias emitted as a
/// nominal reference hides that shape, so `xs:Ints` given one value stayed one value where `xs:int[]`
/// became a list of one. Resolving here keeps the two spellings the same program.</para>
///
/// <para>`aliases` carries the aliases already being resolved on this path, so a cyclic alias stops
/// at the repeat rather than recursing forever. The cycle itself is a diagnostic analysis already
/// reports; this only declines to follow it.</para>
fn build_type_ref_resolving_aliases(
    artifact: &ProgramArtifact,
    resolved_module: &ResolvedModule,
    prepared_cache: &mut PreparedModuleCache,
    ty: &ast::TypeRef,
    aliases: &mut Vec<(String, LocalDefinitionId)>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CodegenTypeRef> {
    match ty {
        ast::TypeRef::Name(name) => {
            if is_builtin_type_name(name.as_str()) {
                return Some(CodegenTypeRef::Primitive {
                    name: name.as_str().to_string(),
                });
            }

            let current_module_identity = resolved_module.prepared_module_identity();
            let resolved = {
                let prepared = prepared_cache.get(artifact, resolved_module);
                let binding = prepared
                    .resolve_binding(PreparedNamespace::Type, name)
                    .or_else(|| prepared.resolve_binding(PreparedNamespace::Element, name));
                binding.map(|binding| {
                    (
                        binding.module_identity(&current_module_identity).to_string(),
                        binding.definition_id(),
                        binding.kind,
                    )
                })
            };
            let Some((target_module_identity, definition_id, kind)) = resolved else {
                if name.as_str() == "Element" {
                    return Some(CodegenTypeRef::Primitive {
                        name: "object".to_string(),
                    });
                }

                diagnostics.push(missing_semantic_data_diagnostic(
                    resolved_module,
                    &format!("type binding '{}'", name.as_str()),
                    empty_span(),
                ));
                return None;
            };
            let Some(target_module) = artifact
                .resolved_program
                .module_by_prepared_identity(&target_module_identity)
            else {
                diagnostics.push(missing_semantic_data_diagnostic(
                    resolved_module,
                    &format!("type module '{}'", target_module_identity),
                    empty_span(),
                ));
                return None;
            };

            if kind == PreparedItemKind::TypeAlias {
                let alias = (target_module_identity.clone(), definition_id);
                if !aliases.contains(&alias) {
                    // An alias declared by a module compiled to an interface alone has no target to
                    // read here, and stays the nominal reference it was before.
                    if let Some(target) = type_alias_target(artifact, target_module, definition_id) {
                        aliases.push(alias);
                        let resolved = build_type_ref_resolving_aliases(
                            artifact,
                            target_module,
                            prepared_cache,
                            &target,
                            aliases,
                            diagnostics,
                        );
                        aliases.pop();
                        return resolved;
                    }
                }
            }

            let reference = reference_from_resolved_module(
                artifact,
                target_module.id,
                definition_id,
                name.as_str(),
                resolved_item_kind_from_prepared(kind),
            );

            Some(CodegenTypeRef::Nominal {
                reference,
                display: name.as_str().to_string(),
            })
        }
        ast::TypeRef::Array(element) => Some(CodegenTypeRef::Array {
            element: Box::new(build_type_ref_resolving_aliases(
                artifact,
                resolved_module,
                prepared_cache,
                element,
                aliases,
                diagnostics,
            )?),
        }),
        ast::TypeRef::Nullable(inner) => Some(CodegenTypeRef::Nullable {
            inner: Box::new(build_type_ref_resolving_aliases(
                artifact,
                resolved_module,
                prepared_cache,
                inner,
                aliases,
                diagnostics,
            )?),
        }),
        ast::TypeRef::Function {
            params,
            return_type,
        } => Some(CodegenTypeRef::Function {
            params: {
                let mut mapped = Vec::with_capacity(params.len());
                for param in params {
                    mapped.push(build_type_ref_resolving_aliases(
                        artifact,
                        resolved_module,
                        prepared_cache,
                        param,
                        aliases,
                        diagnostics,
                    )?);
                }
                mapped
            },
            return_type: Box::new(build_type_ref_resolving_aliases(
                artifact,
                resolved_module,
                prepared_cache,
                return_type,
                aliases,
                diagnostics,
            )?),
        }),
    }
}

/// Returns what a type alias declaration stands for, where the declaring module was compiled from
/// source rather than reached as an interface.
fn type_alias_target(
    artifact: &ProgramArtifact,
    module: &ResolvedModule,
    definition_id: LocalDefinitionId,
) -> Option<ast::TypeRef> {
    let module_artifact = module_artifact_for(artifact, module)?;
    let lowered_module = module_artifact.lowered_module.as_ref()?;
    match lowered_module.item_by_definition(definition_id) {
        Some(Item::TypeAlias(alias)) => Some(alias.ty.clone()),
        _ => None,
    }
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "int"
            | "int32"
            | "int64"
            | "float32"
            | "float64"
            | "string"
            | "boolean"
            | "void"
            | "object"
    )
}

fn resolved_item_kind_from_prepared(kind: PreparedItemKind) -> ResolvedItemKind {
    match kind {
        PreparedItemKind::Function => ResolvedItemKind::Function,
        PreparedItemKind::Value => ResolvedItemKind::Value,
        PreparedItemKind::Component => ResolvedItemKind::Component,
        PreparedItemKind::TypeAlias => ResolvedItemKind::TypeAlias,
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

fn build_prepared_module_for(
    artifact: &ProgramArtifact,
    module: &ResolvedModule,
) -> PreparedModule {
    let mut prepared = PreparedModule::standalone(
        module.prepared_module_identity(),
        module.lowered_module.as_ref().clone(),
    );

    for peer_module in artifact.resolved_program.modules() {
        if peer_module.id == module.id {
            continue;
        }

        prepared.add_peer_module(
            peer_module.prepared_module_identity(),
            peer_module.lowered_module.clone(),
        );
    }

    if let Some(visible_items) = artifact.resolved_program.imported_items(module.id) {
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
    }

    if let Some(module_artifact) = module_artifact_for(artifact, module) {
        for binding in &module_artifact.prepared_bindings {
            prepared.insert_binding(binding.clone());
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
    let base_name = flattened_visible_name(lowered_module, base, scope)?;
    let visible_name = format!("{}.{}", base_name, member);
    resolve_visible_reference(artifact, module_id, &visible_name)
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

fn record_resolution_diagnostic(
    module: &ResolvedModule,
    error: &nx_hir::RecordResolutionError,
) -> Diagnostic {
    Diagnostic::error("codegen-missing-semantic-data")
        .with_message(format!(
            "Cannot build record codegen metadata: {}",
            error.message()
        ))
        .with_label(Label::primary(
            module.prepared_module_identity(),
            error.span(),
        ))
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
        Item::Union(_) => nx_interpreter::ResolvedItemKind::Union,
        Item::Record(_) => nx_interpreter::ResolvedItemKind::Record,
    }
}
