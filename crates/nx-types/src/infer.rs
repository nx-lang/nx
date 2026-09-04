//! Type inference for expressions.

use crate::{
    common_supertype as generic_common_supertype, float_literal_target, is_object_type,
    resolve_type_ref_with, resolve_type_ref_with_seen,
    ty::{DeclaringOrigin, NamedType, Primitive, UnionCaseType, UnionType},
    type_satisfies_expected as generic_type_satisfies_expected, Type, TypeEnvironment,
};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::{
    ast, effective_component_contract_for_name, effective_record_shape_for_name,
    interface_component, interface_function_signature, interface_type_alias, interface_union,
    is_record_subtype, ExprId, InterfaceItemKind, Item, Name, PreparedBindingOrigin,
    PreparedItemKind, PreparedModule, PreparedNamespace, PropertyEntry, ResolvedPreparedItem,
    UnionCaseDef, UnionDef,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// One component's lineage, nearest first: the component itself, then each ancestor.
fn component_lineage(
    contract: &nx_hir::EffectiveComponentContract,
) -> Vec<nx_hir::ComponentAncestor> {
    let mut lineage = vec![nx_hir::ComponentAncestor {
        name: contract.component.name.clone(),
        origin: contract.origin.clone(),
    }];
    lineage.extend(contract.ancestors.iter().cloned());
    lineage
}

/// One record's lineage, nearest first: the record itself, then each ancestor.
fn record_lineage(shape: &nx_hir::EffectiveRecordShape) -> Vec<nx_hir::RecordAncestor> {
    let mut lineage = vec![nx_hir::RecordAncestor {
        name: shape.record.name.clone(),
        origin: shape.origin.clone(),
    }];
    lineage.extend(shape.ancestors.iter().cloned());
    lineage
}

struct TypeAliasInfo {
    target: ast::TypeRef,
    span: TextSpan,
}

/// One discriminated union definition together with the declaration it came from.
///
/// A union is registered under whatever name this module reaches it by, which is not necessarily
/// the name it was declared with. The origin is what says which declaration that name reached.
#[derive(Clone)]
struct UnionEntry {
    def: UnionDef,
    origin: Option<DeclaringOrigin>,
}

impl UnionEntry {
    /// The `UnionType` shape of this definition, carrying its declaring origin.
    fn shape(&self) -> UnionType {
        UnionType::new(
            self.def.name.clone(),
            self.def
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
            self.def.base.clone(),
            self.origin.clone(),
        )
    }

    /// The case type for one of this union's cases, carrying the union's declaring origin.
    fn case_type(&self, case: Name) -> Type {
        Type::union_case_type(self.def.name.clone(), case, self.origin.clone())
    }
}

struct ElementBindingSpec {
    content_property: Option<Name>,
    properties: FxHashMap<Name, ElementPropertySpec>,
    handler_properties: FxHashSet<Name>,
}

struct ElementPropertySpec {
    ty: Type,
    is_required: bool,
}

/// One resolved contextual name, and everything needed to rewrite it to a reference.
#[derive(Clone, Debug)]
pub struct ContextualResolution {
    /// The union that the bare name resolved against, as the declaring module displays it.
    pub type_name: Name,
    /// The case it named.
    pub member: Name,
    /// Where that union is declared.
    ///
    /// This is what the rewrite carries in place of a name, so the case reaches code generation and
    /// evaluation without the union being nameable at the use site.
    pub origin: Option<DeclaringOrigin>,
}

#[derive(Clone)]
struct PropertyPath {
    properties: Vec<PropertyPathBinding>,
}

#[derive(Clone)]
struct PropertyPathBinding {
    key: Name,
    ty: Type,
    span: TextSpan,
    /// The expression the type was inferred from, so a contextual name can be recorded once it
    /// resolves against this binding's expected type.
    value: ExprId,
}

fn handler_prop_name(emit_name: &str) -> String {
    format!("on{}", emit_name)
}

/// Type inference context.
///
/// Manages type inference state and provides methods for inferring types
/// of expressions within a module.
pub struct InferenceContext<'a> {
    /// The module being type-checked
    module: &'a PreparedModule,
    /// Original caller-provided file name for diagnostics.
    file_name: String,
    /// Type environment (name → type, expr → type)
    env: TypeEnvironment,
    /// Type errors collected during inference
    diagnostics: Vec<Diagnostic>,
    /// Next type variable ID for inference
    next_var_id: u32,
    /// Placeholder return types for functions without explicit annotations
    function_return_placeholders: FxHashMap<Name, Type>,
    /// Registered type aliases
    type_aliases: FxHashMap<Name, TypeAliasInfo>,
    /// Registered discriminated union definitions.
    union_defs: FxHashMap<Name, UnionEntry>,
    /// The declaration each visible record name reaches.
    ///
    /// A record type is a `Type::Named`, so without this the type carries only a spelling and an
    /// unrelated same-named record in another module satisfies it.
    record_origins: FxHashMap<Name, DeclaringOrigin>,
    /// The declaration each visible component name reaches, for the same reason.
    ///
    /// Components live in the element namespace rather than the type namespace, but a property may
    /// still be typed by one, so they are reached the same way.
    component_origins: FxHashMap<Name, DeclaringOrigin>,
    /// Union definitions reached through a foreign declaration's own signature.
    ///
    /// <para>Keyed by the declaration each entry is, not by the name it was reached under. Two
    /// foreign unions a module receives under one spelling are two declarations, and a map keyed
    /// by that spelling would keep only whichever arrived last — dropping the other exactly where
    /// a post-resolution lookup needs it. Consulted only for a type that came from another module,
    /// so it can neither shadow a local union nor make a foreign name spellable in source.</para>
    foreign_union_defs: FxHashMap<DeclaringOrigin, UnionEntry>,
    /// Foreign type aliases currently being followed, so a cycle among them terminates.
    ///
    /// <para>A foreign alias is resolved by resolving what it names, which may be another alias in
    /// the same module. The declaring module reports its own cycle; this only stops the consumer
    /// from following one forever.</para>
    foreign_alias_stack: FxHashSet<DeclaringOrigin>,
    /// Contextual names resolved at binding sites, as `expr → (declaring type, member)`.
    ///
    /// Consumed after analysis to rewrite each `Expr::ContextualName` into the qualified member
    /// access it resolved to, so nothing downstream of type checking can observe the bare spelling.
    resolved_contextual_names: FxHashMap<ExprId, ContextualResolution>,
    /// Integer literals that took a floating-point type from their binding site.
    ///
    /// Consumed after analysis to rewrite each one into a float literal, on the same terms and for
    /// the same reason as `resolved_contextual_names`: nothing downstream of type checking should
    /// have to know that the author wrote `24` where `24.0` was expected, or be able to tell.
    converted_int_literals: FxHashMap<ExprId, Primitive>,
}

impl<'a> InferenceContext<'a> {
    /// Creates a new inference context for a module.
    pub fn new(module: &'a PreparedModule) -> Self {
        Self::with_file_name(module, "")
    }

    /// Creates a new inference context for a module with a diagnostic file name.
    pub fn with_file_name(module: &'a PreparedModule, file_name: impl Into<String>) -> Self {
        let mut ctx = Self {
            module,
            file_name: file_name.into(),
            env: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            next_var_id: 0,
            function_return_placeholders: FxHashMap::default(),
            type_aliases: FxHashMap::default(),
            foreign_union_defs: FxHashMap::default(),
            foreign_alias_stack: FxHashSet::default(),
            union_defs: FxHashMap::default(),
            record_origins: FxHashMap::default(),
            component_origins: FxHashMap::default(),
            resolved_contextual_names: FxHashMap::default(),
            converted_int_literals: FxHashMap::default(),
        };
        ctx.register_type_definitions();
        ctx.register_function_signatures();
        ctx.register_value_bindings();
        ctx.validate_local_record_defaults();
        ctx.validate_local_union_defaults();
        ctx
    }

    /// Generates a fresh type variable for inference.
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::var(id)
    }

    fn flattened_expr_name(&self, expr_id: ExprId) -> Option<Name> {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Ident(name) => Some(name.clone()),
            ast::Expr::Member { base, member, .. } => {
                let mut name = self.flattened_expr_name(*base)?.as_str().to_string();
                name.push('.');
                name.push_str(member.as_str());
                Some(Name::new(&name))
            }
            _ => None,
        }
    }

    /// Infers the type of an expression.
    pub fn infer_expr(&mut self, expr_id: ExprId) -> Type {
        let expr = self.module.raw_module().expr(expr_id);

        let ty = match expr {
            // Literals have known types
            ast::Expr::Literal(lit) => self.infer_literal(lit),

            // A bare name has no context-free type. It carries its own name forward as a pending
            // marker and is resolved at the binding site, which is the only place that knows the
            // expected type.
            ast::Expr::ContextualName { name, .. } => Type::ContextualName(name.clone()),

            // Already resolved, and typed by the union it names rather than by anything visible
            // here. Re-inference reaches this only when a caller types an expression twice.
            ast::Expr::ResolvedUnionCase {
                union,
                case,
                module_identity,
                definition_id,
                ..
            } => Type::union_case_type(
                union.clone(),
                case.clone(),
                Some(DeclaringOrigin::new(module_identity, *definition_id)),
            ),

            // Identifiers look up in environment
            ast::Expr::Ident(name) => {
                if let Some(ty) = self.env.lookup(name) {
                    ty.clone()
                } else {
                    Type::Error
                }
            }

            // Binary operations
            ast::Expr::BinaryOp { lhs, op, rhs, span } => {
                let lhs_ty = self.infer_expr(*lhs);
                let rhs_ty = self.infer_expr(*rhs);

                self.infer_binop(*op, &lhs_ty, &rhs_ty, *span)
            }

            // Unary operations
            ast::Expr::UnaryOp { op, expr, span } => {
                let expr_ty = self.infer_expr(*expr);
                self.infer_unop(*op, &expr_ty, *span)
            }

            // Function calls
            ast::Expr::Call { func, args, span } => {
                let func_ty = self.infer_expr(*func);

                // Infer argument types
                let arg_tys: Vec<_> = args.iter().map(|arg| self.infer_expr(*arg)).collect();

                self.infer_call(&func_ty, args, &arg_tys, *span)
            }

            // If expressions
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(*condition);

                // Condition must be boolean
                if !cond_ty.is_compatible_with(&Type::boolean()) && !cond_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("If condition must be boolean, found {}", cond_ty),
                        *span,
                    );
                }

                let then_ty = self.infer_expr(*then_branch);

                if let Some(else_id) = else_branch {
                    let else_ty = self.infer_expr(*else_id);

                    self.common_supertype(&then_ty, &else_ty)
                } else {
                    // No else branch - type is void
                    Type::void()
                }
            }

            ast::Expr::Match {
                scrutinee,
                arms,
                else_branch,
                span,
            } => self.infer_match_expr(*scrutinee, arms, *else_branch, *span),

            // Arrays
            ast::Expr::Array { elements, span } => {
                if elements.is_empty() {
                    // Empty array - need more context to infer element type
                    Type::array(self.fresh_var())
                } else {
                    let elem_tys: Vec<_> = elements.iter().map(|e| self.infer_expr(*e)).collect();
                    let item_ty = self.common_sequence_item_type(&elem_tys, *span);
                    Type::array(item_ty)
                }
            }

            // Index operation
            ast::Expr::Index { base, index, span } => {
                let base_ty = self.infer_expr(*base);
                let index_ty = self.infer_expr(*index);

                // Index must be an integer of any width
                if !index_ty.is_compatible_with(&Type::int()) && !index_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("Array index must be an integer, found {}", index_ty),
                        *span,
                    );
                }

                // Base must be array
                match base_ty {
                    Type::Array(elem_ty) => *elem_ty,
                    Type::Error => Type::Error,
                    _ => {
                        self.error(
                            "type-mismatch",
                            format!("Cannot index into non-array type {}", base_ty),
                            *span,
                        );
                        Type::Error
                    }
                }
            }

            // Member access
            ast::Expr::Member { base, member, span } => {
                if let Some(name) = self.flattened_expr_name(expr_id) {
                    if let Some(ty) = self.env.lookup(&name) {
                        ty.clone()
                    } else if let Some((entry, case)) = self.union_case_from_qualified_name(&name) {
                        let union_name = entry.def.name.clone();
                        let case_name = case.name.clone();
                        let is_fieldless = case.fields.is_empty();
                        let case_ty = entry.case_type(case_name.clone());
                        if is_fieldless {
                            case_ty
                        } else {
                            self.error(
                                "payload-union-case-requires-constructor",
                                format!(
                                    "Union case '{}.{}' requires element-style payload construction",
                                    union_name, case_name
                                ),
                                *span,
                            );
                            Type::Error
                        }
                    } else if let Some(union_info) = self.union_info_for_expr(*base) {
                        self.union_case_by_member(&union_info, member, *span)
                    } else {
                        let base_ty = self.infer_expr(*base);
                        self.infer_member_access(&base_ty, member, *span)
                    }
                } else if let Some(union_info) = self.union_info_for_expr(*base) {
                    self.union_case_by_member(&union_info, member, *span)
                } else {
                    let base_ty = self.infer_expr(*base);
                    self.infer_member_access(&base_ty, member, *span)
                }
            }

            ast::Expr::Element { element, span } => {
                let element_ref = self.module.raw_module().element(*element).clone();
                self.infer_element_expression(&element_ref, *span)
            }

            ast::Expr::RecordLiteral {
                record,
                properties,
                span,
            } => self.infer_record_literal(record, properties, *span),
            // TODO: Action handlers are lowered as lazy runtime callbacks. Wire them into
            // expression-level type inference once the language has a first-class handler type.
            ast::Expr::ActionHandler { .. } => Type::Error,

            // Block expressions
            ast::Expr::Block { stmts: _, expr, .. } => {
                // TODO: Process statements
                if let Some(expr_id) = expr {
                    self.infer_expr(*expr_id)
                } else {
                    Type::void()
                }
            }

            // For loop expressions
            ast::Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => {
                // Infer iterable type (should be array)
                let iterable_ty = self.infer_expr(*iterable);
                let item_ty = match iterable_ty.clone() {
                    Type::Array(inner) => *inner,
                    Type::Error => Type::Error,
                    other => {
                        self.error(
                            "type-mismatch",
                            format!("For iterable must be an array, found {}", other),
                            expr.span(),
                        );
                        Type::Error
                    }
                };

                self.env.push_scope();
                self.env.bind(item.clone(), item_ty);
                if let Some(index_name) = index {
                    self.env.bind(index_name.clone(), Type::int());
                }
                let body_ty = self.infer_expr(*body);
                self.env.pop_scope();

                Type::array(body_ty)
            }

            // Let expressions (used for match lowering)
            ast::Expr::Let {
                name, value, body, ..
            } => {
                // Infer the type of the value
                let value_ty = self.infer_expr(*value);

                // Create a new scope for the let binding
                self.env.push_scope();

                // Bind the name to the value type in this scope
                self.env.bind(name.clone(), value_ty);

                // Infer the body with the binding in scope
                let body_ty = self.infer_expr(*body);

                // Pop the scope to remove the binding
                self.env.pop_scope();

                body_ty
            }

            // Error expressions
            ast::Expr::Error(_) => Type::Error,
        };

        // Record the inferred type
        self.env.set_expr_type(expr_id, ty.clone());
        ty
    }

    /// Infers all types within a function, binding parameters while visiting the body.
    pub fn infer_function(&mut self, func: &nx_hir::Function) {
        let mut bound_names = Vec::new();

        for param in &func.params {
            let param_ty = self.type_from_type_ref(&param.ty);
            self.env.bind(param.name.clone(), param_ty);
            bound_names.push(param.name.clone());
        }

        let body_ty = self.infer_expr(func.body);

        for name in bound_names {
            self.env.remove(&name);
        }

        let return_ty = if let Some(ty) = func.return_type.as_ref() {
            let expected = self.type_from_type_ref(ty);
            self.check_typed_binding_for(
                Some(func.body),
                &body_ty,
                &expected,
                func.span,
                "return-type-mismatch",
                format!("Return value for function '{}'", func.name),
            );
            expected
        } else {
            body_ty.clone()
        };

        self.bind_function_signature(func, return_ty.clone());
        if func.return_type.is_none() {
            self.function_return_placeholders.remove(&func.name);
        }
    }

    /// Infers a component's prop and state defaults and its body.
    ///
    /// <para>A component body is markup written against the same binding sites an element in a
    /// function body has, so it is inferred the same way. Props and state are bound by name, because
    /// the body reads them, and they are bound in a pushed scope so a prop cannot outlive the
    /// component that declared it.</para>
    ///
    /// <para>The props bound are the component's *effective* ones, so a prop reached through the
    /// base chain reads at its declared type the way a directly declared one does. Scope building
    /// already resolves an inherited name, so leaving it unbound would not fail the read — it would
    /// make it infer vacuously, which is the one outcome worse than either. Each inherited type is
    /// resolved in the module that declared the field, since that is the module whose names it was
    /// written against.</para>
    ///
    /// <para>Fields are bound in the order both runtimes materialize them — the effective props,
    /// then the state — and each default is checked *before* its own field is bound. That is what
    /// makes a default's environment here the same one it will have when it runs: it sees the
    /// fields materialized before it and nothing else. A name from later in the declaration is
    /// reported as undefined by scope checking, so nothing is silently accepted; it simply is not
    /// this pass's diagnostic to give.</para>
    ///
    /// <para>A defaulted prop is checked against its own declared type, which is also what lets a
    /// contextual literal be written as a default: the default's binding site is the declaration
    /// itself.</para>
    pub fn infer_component(&mut self, component: &nx_hir::Component) {
        self.env.push_scope();

        let effective_props = self
            .effective_component_contract(&component.name)
            .ok()
            .flatten()
            .map(|contract| contract.props);

        match effective_props {
            Some(props) => {
                for field in &props {
                    let field_ty =
                        self.type_from_type_ref_in(Some(&field.module_identity), &field.ty);
                    self.check_component_field_default(component, &field.name, &field_ty);
                    self.env.bind(field.name.clone(), field_ty);
                }
            }
            // Without a contract there is no base chain to read, and the component's own props are
            // the whole of what its body and its defaults can see.
            None => {
                for field in &component.props {
                    let field_ty = self.type_from_type_ref(&field.ty);
                    self.check_component_field_default(component, &field.name, &field_ty);
                    self.env.bind(field.name.clone(), field_ty);
                }
            }
        }

        for field in &component.state {
            let field_ty = self.type_from_type_ref(&field.ty);
            self.check_component_field_default(component, &field.name, &field_ty);
            self.env.bind(field.name.clone(), field_ty);
        }

        // A body is absent exactly when the component is abstract or external, and there is then
        // nothing below the declaration to infer.
        if let Some(body) = component.body {
            self.infer_expr(body);
        }

        self.env.pop_scope();
    }

    /// Checks the default this component declares for one of its fields, where it declares one.
    ///
    /// An inherited field's default belongs to the module that declared it and is checked there;
    /// what is checked here is only what this declaration wrote.
    fn check_component_field_default(
        &mut self,
        component: &nx_hir::Component,
        name: &Name,
        field_ty: &Type,
    ) {
        let declared = component
            .props
            .iter()
            .chain(component.state.iter())
            .find(|field| &field.name == name);
        let Some(field) = declared else {
            return;
        };
        let Some(default) = field.default else {
            return;
        };

        let actual = self.infer_expr(default);
        self.check_typed_binding_for(
            Some(default),
            &actual,
            field_ty,
            field.span,
            "component-default-type-mismatch",
            format!("Default value for '{}.{}'", component.name, field.name),
        );
    }

    /// Infers the type of a literal.
    fn infer_literal(&mut self, lit: &ast::Literal) -> Type {
        match lit {
            ast::Literal::String(_) => Type::string(),
            ast::Literal::Int(_) => Type::int(),
            ast::Literal::Float(_) => Type::float64(),
            ast::Literal::Boolean(_) => Type::boolean(),
            ast::Literal::Null => Type::nullable(self.fresh_var()),
        }
    }

    fn infer_match_expr(
        &mut self,
        scrutinee: ExprId,
        arms: &[ast::MatchArm],
        else_branch: Option<ExprId>,
        span: TextSpan,
    ) -> Type {
        let scrutinee_ty = self.infer_expr(scrutinee);
        let scrutinee_name = match self.module.raw_module().expr(scrutinee) {
            ast::Expr::Ident(name) => Some(name.clone()),
            _ => None,
        };
        let union_ty = match &scrutinee_ty {
            Type::Union(union_ty) => Some(union_ty.clone()),
            _ => None,
        };

        let mut covered_cases = FxHashSet::default();
        let mut result_tys = Vec::new();

        for arm in arms {
            let pattern_tys = arm
                .patterns
                .iter()
                .map(|pattern| {
                    let pattern_ty = self.infer_match_pattern(*pattern, &scrutinee_ty);
                    self.check_match_pattern(
                        &scrutinee_ty,
                        union_ty.as_ref(),
                        &pattern_ty,
                        &mut covered_cases,
                        self.module.raw_module().expr(*pattern).span(),
                    );
                    pattern_ty
                })
                .collect::<Vec<_>>();

            let narrowed_case = self.match_arm_narrowed_case(union_ty.as_ref(), &pattern_tys);
            let body_ty =
                if let (Some(name), Some(case_ty)) = (scrutinee_name.as_ref(), narrowed_case) {
                    self.env.push_scope();
                    self.env.bind(name.clone(), Type::UnionCase(case_ty));
                    let ty = self.infer_expr(arm.body);
                    self.env.pop_scope();
                    ty
                } else {
                    self.infer_expr(arm.body)
                };

            result_tys.push(body_ty);
        }

        let is_exhaustive = union_ty.as_ref().is_some_and(|union_ty| {
            union_ty
                .cases
                .iter()
                .all(|case| covered_cases.contains(case))
        });

        if let Some(else_id) = else_branch {
            result_tys.push(self.infer_expr(else_id));
        } else if let Some(union_ty) = union_ty.as_ref() {
            if !is_exhaustive {
                let missing = union_ty
                    .cases
                    .iter()
                    .filter(|case| !covered_cases.contains(*case))
                    .map(|case| case.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error(
                    "non-exhaustive-union-match",
                    format!(
                        "Union match on '{}' is missing cases: {}",
                        union_ty.name, missing
                    ),
                    span,
                );
                result_tys.push(Type::void());
            }
        } else {
            result_tys.push(Type::void());
        }

        self.common_result_type(&result_tys)
    }

    fn infer_match_pattern(&mut self, pattern: ExprId, scrutinee_ty: &Type) -> Type {
        // A bare pattern resolves against the scrutinee's type in preference to any lexically
        // visible binding of the same name. The preference is reported so a pattern that used to
        // compare against a variable never changes meaning silently.
        if let ast::Expr::ContextualName { name, span } = self.module.raw_module().expr(pattern) {
            let name = name.clone();
            let span = *span;
            if self.env.lookup(&name).is_some() {
                self.error(
                    "contextual-name-displaces-binding",
                    format!(
                        "Pattern '{}' resolves as a case of '{}', not as the binding named '{}' \
                         that is in scope here",
                        name, scrutinee_ty, name
                    ),
                    span,
                );
            }
            let context = format!("Pattern '{}'", name);
            let resolved =
                self.resolve_contextual_name_in(pattern, &name, scrutinee_ty, span, &context, true);
            let ty = resolved.unwrap_or(Type::Error);
            self.env.set_expr_type(pattern, ty.clone());
            return ty;
        }

        if let Some(name) = self.flattened_expr_name(pattern) {
            if let Some((entry, case)) = self.union_case_from_qualified_name(&name) {
                let ty = entry.case_type(case.name.clone());
                self.env.set_expr_type(pattern, ty.clone());
                return ty;
            }
        }

        self.infer_expr(pattern)
    }

    fn check_match_pattern(
        &mut self,
        scrutinee_ty: &Type,
        union_ty: Option<&UnionType>,
        pattern_ty: &Type,
        covered_cases: &mut FxHashSet<Name>,
        span: TextSpan,
    ) {
        if pattern_ty.is_error() || scrutinee_ty.is_error() {
            return;
        }

        if let Some(union_ty) = union_ty {
            match pattern_ty {
                Type::UnionCase(case_ty) if case_ty.is_same_union_as(union_ty) => {
                    covered_cases.insert(case_ty.case.clone());
                }
                Type::UnionCase(case_ty) => {
                    self.error(
                        "wrong-union-pattern",
                        format!(
                            "Pattern '{}.{}' is not a case of union '{}'",
                            case_ty.union, case_ty.case, union_ty.name
                        ),
                        span,
                    );
                }
                _ => {
                    self.error(
                        "invalid-union-case-pattern",
                        format!(
                            "Union match on '{}' requires union case patterns",
                            union_ty.name
                        ),
                        span,
                    );
                }
            }
            return;
        }

        if !self.type_satisfies_expected(scrutinee_ty, pattern_ty)
            && !self.type_satisfies_expected(pattern_ty, scrutinee_ty)
        {
            self.error(
                "type-mismatch",
                format!("Cannot compare types {} and {}", scrutinee_ty, pattern_ty),
                span,
            );
        }
    }

    fn match_arm_narrowed_case(
        &self,
        union_ty: Option<&UnionType>,
        pattern_tys: &[Type],
    ) -> Option<UnionCaseType> {
        if pattern_tys.len() != 1 {
            return None;
        }

        let union_ty = union_ty?;
        match &pattern_tys[0] {
            Type::UnionCase(case_ty) if case_ty.is_same_union_as(union_ty) => Some(case_ty.clone()),
            _ => None,
        }
    }

    fn common_result_type(&self, result_tys: &[Type]) -> Type {
        let mut current = result_tys.first().cloned().unwrap_or_else(Type::void);

        for ty in result_tys.iter().skip(1) {
            current = self.common_supertype(&current, ty);
        }

        current
    }

    /// Infers the result type of a binary operation.
    fn infer_binop(
        &mut self,
        op: ast::BinOp,
        lhs: &Type,
        rhs: &Type,
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        use ast::BinOp::*;

        // Skip error checking if either operand is error
        if lhs.is_error() || rhs.is_error() {
            return Type::Error;
        }

        match op {
            // Arithmetic: same numeric category with promotion
            Add | Sub | Mul | Div | Mod => {
                if let (Type::Primitive(a), Type::Primitive(b)) = (lhs, rhs) {
                    if a.is_numeric() && b.is_numeric() {
                        if let Some(promoted) = crate::ty::Primitive::numeric_promotion(*a, *b) {
                            return Type::Primitive(promoted);
                        } else {
                            self.error(
                                "type-mismatch",
                                format!("Cannot mix integer and float types: {} and {}", lhs, rhs),
                                span,
                            );
                            return Type::Error;
                        }
                    }
                }
                if lhs == &Type::string() && rhs == &Type::string() && op == Add {
                    Type::string()
                } else {
                    self.error(
                        "type-mismatch",
                        format!(
                            "Binary operator {:?} cannot be applied to types {} and {}",
                            op, lhs, rhs
                        ),
                        span,
                    );
                    Type::Error
                }
            }

            // Comparison: T × T → bool (where T supports comparison)
            Eq | Ne | Lt | Le | Gt | Ge => {
                if self.type_satisfies_expected(lhs, rhs) || self.type_satisfies_expected(rhs, lhs)
                {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Cannot compare types {} and {}", lhs, rhs),
                        span,
                    );
                    Type::Error
                }
            }

            // Logical: boolean × boolean → boolean
            And | Or => {
                if lhs == &Type::boolean() && rhs == &Type::boolean() {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!(
                            "Logical operator {:?} requires boolean operands, found {} and {}",
                            op, lhs, rhs
                        ),
                        span,
                    );
                    Type::Error
                }
            }

            Concat => {
                // String concatenation
                if lhs == &Type::string() && rhs == &Type::string() {
                    Type::string()
                } else {
                    self.error(
                        "type-mismatch",
                        format!(
                            "String concatenation requires string operands, found {} and {}",
                            lhs, rhs
                        ),
                        span,
                    );
                    Type::Error
                }
            }
        }
    }

    /// Infers the result type of a unary operation.
    fn infer_unop(
        &mut self,
        op: ast::UnOp,
        operand: &Type,
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        if operand.is_error() {
            return Type::Error;
        }

        match op {
            ast::UnOp::Neg => {
                if let Type::Primitive(p) = operand {
                    if p.is_numeric() {
                        return operand.clone();
                    }
                }
                self.error(
                    "type-mismatch",
                    format!("Negation requires a numeric type, found {}", operand),
                    span,
                );
                Type::Error
            }
            ast::UnOp::Not => {
                if operand == &Type::boolean() {
                    Type::boolean()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Logical NOT requires boolean, found {}", operand),
                        span,
                    );
                    Type::Error
                }
            }
        }
    }

    /// Infers the result type of a function call.
    fn infer_call(
        &mut self,
        func_ty: &Type,
        args: &[ExprId],
        arg_tys: &[Type],
        span: nx_diagnostics::TextSpan,
    ) -> Type {
        if func_ty.is_error() {
            return Type::Error;
        }

        match func_ty {
            Type::Function { params, ret } => {
                // Check argument count
                if params.len() != arg_tys.len() {
                    self.error(
                        "arg-count-mismatch",
                        format!(
                            "Function expects {} arguments, got {}",
                            params.len(),
                            arg_tys.len()
                        ),
                        span,
                    );
                    return Type::Error;
                }

                // Check argument types. The argument expression is passed so a literal written
                // there can take the parameter's type.
                for (i, (param_ty, arg_ty)) in params.iter().zip(arg_tys.iter()).enumerate() {
                    self.check_typed_binding_for(
                        args.get(i).copied(),
                        arg_ty,
                        param_ty,
                        span,
                        "type-mismatch",
                        format!("Argument {}", i),
                    );
                }

                (**ret).clone()
            }
            _ => {
                self.error(
                    "not-a-function",
                    format!("Cannot call non-function type {}", func_ty),
                    span,
                );
                Type::Error
            }
        }
    }

    fn infer_member_access(&mut self, base_ty: &Type, member: &Name, span: TextSpan) -> Type {
        // A nullable base reads its field like a non-nullable one. NX has no narrowing construct to
        // discharge the null with, and the catalogs that drive most markup declare every property
        // optional, so requiring one would make a nullable record or union prop unreadable rather
        // than safer. The field's own declared type is returned, not a nullable of it, for the same
        // reason: a `string?` here would fail at every `string` site downstream.
        match base_ty.strip_nullable() {
            Type::Union(union_ty) => {
                if let Some(ty) =
                    self.union_shared_field_type(&union_ty.name, union_ty.origin(), member)
                {
                    return ty;
                }

                if self.union_has_case_field(&union_ty.name, union_ty.origin(), member) {
                    self.error(
                        "union-case-field-requires-narrowing",
                        format!(
                            "Field '{}' is case-specific on union '{}' and requires narrowing",
                            member, union_ty.name
                        ),
                        span,
                    );
                    Type::Error
                } else {
                    self.error(
                        "unknown-union-field",
                        format!("Union '{}' has no shared field '{}'", union_ty.name, member),
                        span,
                    );
                    Type::Error
                }
            }
            Type::UnionCase(case_ty) => self
                .union_case_field_type(&case_ty.union, case_ty.origin(), &case_ty.case, member)
                .unwrap_or_else(|| {
                    self.error(
                        "unknown-union-case-field",
                        format!(
                            "Union case '{}.{}' has no field '{}'",
                            case_ty.union, case_ty.case, member
                        ),
                        span,
                    );
                    Type::Error
                }),
            // A record's field, reached through the effective shape so an inherited field is
            // found as readily as a declared one. Each field's type is resolved in the module that
            // declared *that field*, which is not always the module the record itself came from.
            Type::Named(named) => {
                let Ok(Some(shape)) = self.record_shape_of(named) else {
                    return self.member_access_unsupported(member, span);
                };
                let Some(field) = shape.fields.iter().find(|field| field.name == *member) else {
                    let known = shape
                        .fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.error(
                        "unknown-record-field",
                        format!(
                            "Record '{}' has no field '{}'; it has: {}",
                            named.name, member, known
                        ),
                        span,
                    );
                    return Type::Error;
                };
                let declaring_module = field.module_identity.clone();
                let field_ty = field.ty.clone();
                self.type_from_type_ref_in(Some(&declaring_module), &field_ty)
            }
            Type::Error => Type::Error,
            _ => self.member_access_unsupported(member, span),
        }
    }

    /// Reports a member access on a base that has no fields to reach.
    fn member_access_unsupported(&mut self, member: &Name, span: TextSpan) -> Type {
        self.error(
            "not-implemented",
            format!("Member access not yet implemented: .{}", member),
            span,
        );
        Type::Error
    }

    /// Returns the union declaration a resolved union type denotes.
    ///
    /// <para>A resolved `Type::Union` or `Type::UnionCase` already names one declaration, so the
    /// entry is selected by that declaration. Looking it up by spelling alone would let a
    /// same-named union visible here answer for a foreign one — the capture that carrying an
    /// origin exists to prevent. A foreign entry is addressed by its origin outright. The local
    /// map is keyed by the name this module reaches a union under, so there the name is tried
    /// first, because it is the common case and reaches the same entry, and a scan covers a
    /// collision.</para>
    fn union_entry_for(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
    ) -> Option<&UnionEntry> {
        let denotes = |entry: &&UnionEntry| {
            nx_hir::same_declaration(
                entry.origin.as_ref(),
                &entry.def.name,
                union_origin,
                union_name,
            )
        };

        self.union_defs
            .get(union_name)
            .filter(denotes)
            .or_else(|| {
                union_origin
                    .and_then(|origin| self.foreign_union_defs.get(origin))
                    .filter(denotes)
            })
            .or_else(|| self.union_defs.values().find(denotes))
    }

    /// The type of a field every case of a union shares, through the union's abstract base.
    ///
    /// <para>The base is a name the union's own module wrote, so it is resolved there, and each
    /// inherited field's type is resolved in the module that declared that field. Resolving either
    /// here would let an unrelated local record supply the fields of a foreign union.</para>
    fn union_shared_field_type(
        &mut self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        member: &Name,
    ) -> Option<Type> {
        let entry = self.union_entry_for(union_name, union_origin)?;
        let base_name = entry.def.base.clone()?;
        let base_origin = entry
            .origin
            .as_ref()
            .and_then(|origin| self.record_origin_in(origin.module_identity(), &base_name));

        let shape = match base_origin.as_ref() {
            Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
            None => effective_record_shape_for_name(self.module, &base_name),
        }
        .ok()
        .flatten()?;

        let field = shape.fields.iter().find(|field| field.name == *member)?;
        let field_module = field.module_identity.clone();
        let field_ty = field.ty.clone();
        Some(self.type_from_type_ref_in(Some(&field_module), &field_ty))
    }

    fn union_has_case_field(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        member: &Name,
    ) -> bool {
        self.union_entry_for(union_name, union_origin)
            .map(|entry| {
                entry
                    .def
                    .cases
                    .iter()
                    .any(|case| case.fields.iter().any(|field| field.name == *member))
            })
            .unwrap_or(false)
    }

    /// The type of a field on one case of a union, shared fields included.
    ///
    /// <para>A case's own field types are written in the union's module, so they are resolved
    /// there rather than here.</para>
    fn union_case_field_type(
        &mut self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        case_name: &Name,
        member: &Name,
    ) -> Option<Type> {
        if let Some(ty) = self.union_shared_field_type(union_name, union_origin, member) {
            return Some(ty);
        }

        let entry = self.union_entry_for(union_name, union_origin)?;
        let declaring_module = entry
            .origin
            .as_ref()
            .map(|origin| origin.module_identity().to_string());
        let union_def = entry.def.clone();
        let case = union_def
            .cases
            .iter()
            .find(|case| case.name == *case_name)?;
        let field = case.fields.iter().find(|field| field.name == *member)?;
        let field_ty = field.ty.clone();
        Some(self.type_from_type_ref_in(declaring_module.as_deref(), &field_ty))
    }

    fn infer_record_literal(
        &mut self,
        record: &Name,
        properties: &[ast::RecordLiteralProperty],
        span: TextSpan,
    ) -> Type {
        if let Some(record_def) = self.resolve_record_definition(record) {
            if record_def.is_abstract {
                self.error(
                    "abstract-record-instantiation",
                    format!("Cannot instantiate abstract record '{}'", record_def.name),
                    span,
                );
            }

            let effective_shape = self.effective_record_shape(record).ok().flatten();
            for property in properties {
                match self.record_field_type_ref(
                    &record_def,
                    effective_shape.as_ref(),
                    &property.name,
                ) {
                    Some(field_ty) => {
                        let actual = self.infer_expr(property.value);
                        let expected = self.type_from_type_ref(&field_ty);
                        self.check_typed_binding_for(
                            Some(property.value),
                            &actual,
                            &expected,
                            property.span,
                            "record-field-type-mismatch",
                            format!("Record field '{}' on '{}'", property.name, record),
                        );
                    }
                    None => {
                        self.error(
                            "unknown-record-field",
                            format!("Record '{}' has no field '{}'", record, property.name),
                            property.span,
                        );
                    }
                }
            }

            self.nominal_named_type(record)
        } else {
            self.nominal_named_type(record)
        }
    }

    fn record_field_type_ref(
        &self,
        record_def: &nx_hir::RecordDef,
        effective_shape: Option<&nx_hir::EffectiveRecordShape>,
        name: &Name,
    ) -> Option<ast::TypeRef> {
        if let Some(shape) = effective_shape {
            return shape
                .fields
                .iter()
                .find(|field| field.name == *name)
                .map(|field| field.ty.clone());
        }

        record_def
            .properties
            .iter()
            .find(|field| field.name == *name)
            .map(|field| field.ty.clone())
    }

    fn infer_element_expression(&mut self, element: &nx_hir::Element, span: TextSpan) -> Type {
        if let Some(function) = self.resolve_function_definition(&element.tag) {
            let declaring_module = function.module_identity().to_string();
            match function {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(function),
                    ..
                } => {
                    self.check_element_bindings_against_function(
                        element,
                        &function,
                        span,
                        Some(declaring_module.as_str()),
                    );
                    if let Some(func_ty) = self.env.lookup(&element.tag) {
                        if let Type::Function { ret, .. } = func_ty {
                            return (**ret).clone();
                        }
                    }
                    return function
                        .return_type
                        .as_ref()
                        .map(|ty| self.type_from_type_ref(ty))
                        .unwrap_or_else(|| self.nominal_named_type(&element.tag));
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some((_name, _visibility, params, return_type, _span)) =
                        interface_function_signature(&item)
                    {
                        let declaring_module = item.module_identity.clone();
                        let spec = self.build_element_binding_spec_in(
                            Some(declaring_module.as_str()),
                            params
                                .iter()
                                .map(|param| (&param.name, &param.ty, param.is_content, true)),
                        );
                        self.check_element_bindings(element, span, &spec);
                        return self.type_from_type_ref(&return_type);
                    }
                }
                _ => {}
            }
        }

        if let Some(component) = self.resolve_component_definition(&element.tag) {
            let declaring_module = component.module_identity().to_string();
            match component {
                ResolvedPreparedItem::Raw {
                    item: Item::Component(component),
                    ..
                } => {
                    if component.is_abstract {
                        self.error(
                            "abstract-component-instantiation",
                            format!("Cannot instantiate abstract component '{}'", component.name),
                            span,
                        );
                    }
                    self.check_element_bindings_against_component(
                        element,
                        &component,
                        span,
                        Some(declaring_module.as_str()),
                    );
                    return self.nominal_named_type(&element.tag);
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some(component) = interface_component(&item) {
                        if component.is_abstract {
                            self.error(
                                "abstract-component-instantiation",
                                format!(
                                    "Cannot instantiate abstract component '{}'",
                                    component.name
                                ),
                                span,
                            );
                        }
                        self.check_element_bindings_against_component(
                            element,
                            &component,
                            span,
                            Some(declaring_module.as_str()),
                        );
                        return self.nominal_named_type(&element.tag);
                    }
                }
                _ => {}
            }
        }

        if let Some((declaring_module, record_def)) =
            self.resolve_record_definition_with_origin(&element.tag)
        {
            if record_def.is_abstract {
                self.error(
                    "abstract-record-instantiation",
                    format!("Cannot instantiate abstract record '{}'", record_def.name),
                    span,
                );
            }
            self.check_element_bindings_against_record(
                element,
                &record_def,
                span,
                Some(declaring_module.as_str()),
            );
            return self.nominal_named_type(&element.tag);
        }

        if let Some((entry, case)) = self.union_case_from_qualified_name(&element.tag) {
            let entry = entry.clone();
            let case = case.clone();
            let declaring_module = entry
                .origin
                .as_ref()
                .map(|origin| origin.module_identity().to_string());
            self.check_element_bindings_against_union_case(
                element,
                &entry.def,
                &case,
                span,
                declaring_module.as_deref(),
            );
            return entry.case_type(case.name);
        }

        self.nominal_named_type(&element.tag)
    }

    fn check_element_bindings_against_function(
        &mut self,
        element: &nx_hir::Element,
        function: &nx_hir::Function,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let spec = self.build_element_binding_spec_in(
            declaring_module,
            function
                .params
                .iter()
                .map(|param| (&param.name, &param.ty, param.is_content, true)),
        );
        self.check_element_bindings(element, span, &spec);
    }

    fn check_element_bindings_against_component(
        &mut self,
        element: &nx_hir::Element,
        component: &nx_hir::Component,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let effective_contract = self
            .effective_component_contract(&component.name)
            .ok()
            .flatten();
        let mut spec = if let Some(contract) = effective_contract.as_ref() {
            self.build_element_binding_spec_in(
                declaring_module,
                contract
                    .props
                    .iter()
                    .map(|field| (&field.name, &field.ty, field.is_content, field.is_required)),
            )
        } else {
            self.build_element_binding_spec_in(
                declaring_module,
                component.props.iter().map(|field| {
                    (
                        &field.name,
                        &field.ty,
                        field.is_content,
                        field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_)),
                    )
                }),
            )
        };
        let emit_names: Vec<&Name> = match effective_contract.as_ref() {
            Some(contract) => contract.emits.iter().map(|emit| &emit.emit.name).collect(),
            None => component.emits.iter().map(|emit| &emit.name).collect(),
        };
        spec.handler_properties.extend(
            emit_names
                .into_iter()
                .map(|name| Name::new(&handler_prop_name(name.as_str()))),
        );
        self.check_element_bindings(element, span, &spec);
    }

    fn check_element_bindings_against_record(
        &mut self,
        element: &nx_hir::Element,
        record_def: &nx_hir::RecordDef,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let effective_shape = self.effective_record_shape(&record_def.name).ok().flatten();
        let spec = if let Some(shape) = effective_shape.as_ref() {
            self.build_element_binding_spec_in(
                declaring_module,
                shape
                    .fields
                    .iter()
                    .map(|field| (&field.name, &field.ty, field.is_content, field.is_required)),
            )
        } else {
            self.build_element_binding_spec_in(
                declaring_module,
                record_def.properties.iter().map(|field| {
                    (
                        &field.name,
                        &field.ty,
                        field.is_content,
                        field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_)),
                    )
                }),
            )
        };
        self.check_record_element_bindings(element, span, &spec);
    }

    fn check_record_element_bindings(
        &mut self,
        element: &nx_hir::Element,
        span: TextSpan,
        spec: &ElementBindingSpec,
    ) {
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths.iter().any(|path| {
                    path.properties
                        .iter()
                        .any(|property| property.key == *content_name)
                }) {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Record '{}' passes content for '{}' both as a property and as body content",
                            element.tag, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        &element.content,
                        &expected.ty,
                        span,
                        format!("Content for '{}' binds to '{}'", element.tag, content_name),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Record '{}' passes body content, but '{}' does not declare a content field",
                        element.tag, element.tag
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            spec,
            &element.tag,
            content_from_body,
            span,
            "record-field-type-mismatch",
            "unknown-record-field",
            "missing-property",
        );
    }

    /// Checks `<Union.case ... />` against the case's declared fields.
    ///
    /// <para>`declaring_module` is the module that wrote the union. Its base and its field types
    /// are names that module wrote, so both are resolved there — an imported union's `n:Size` means
    /// its module's `Size`, not a same-named record visible here.</para>
    fn check_element_bindings_against_union_case(
        &mut self,
        element: &nx_hir::Element,
        union_def: &UnionDef,
        case: &UnionCaseDef,
        span: TextSpan,
        declaring_module: Option<&str>,
    ) {
        let mut content_property: Option<Name> = None;
        let mut properties = FxHashMap::<Name, ElementPropertySpec>::default();

        if let Some(base_name) = union_def.base.as_ref() {
            let base_origin = declaring_module
                .and_then(|module_identity| self.record_origin_in(module_identity, base_name));
            let shape = match base_origin.as_ref() {
                Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
                None => effective_record_shape_for_name(self.module, base_name),
            };
            if let Ok(Some(shape)) = shape {
                for field in shape.fields {
                    let ty = self.type_from_type_ref_in(Some(&field.module_identity), &field.ty);
                    if field.is_content {
                        content_property = Some(field.name.clone());
                    }
                    properties.insert(
                        field.name,
                        ElementPropertySpec {
                            ty,
                            is_required: field.is_required,
                        },
                    );
                }
            }
        }

        for field in &case.fields {
            let ty = self.type_from_type_ref_in(declaring_module, &field.ty);
            let is_required =
                field.default.is_none() && !matches!(field.ty, ast::TypeRef::Nullable(_));
            if field.is_content {
                content_property = Some(field.name.clone());
            }
            properties.insert(field.name.clone(), ElementPropertySpec { ty, is_required });
        }

        let spec = ElementBindingSpec {
            content_property,
            properties,
            handler_properties: FxHashSet::default(),
        };
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths
                    .iter()
                    .any(|path| path.properties.iter().any(|prop| prop.key == *content_name))
                {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Union case '{}.{}' passes content for '{}' both as a property and as body content",
                            union_def.name, case.name, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        &element.content,
                        &expected.ty,
                        span,
                        format!(
                            "Content for '{}.{}' binds to '{}'",
                            union_def.name, case.name, content_name
                        ),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Union case '{}.{}' receives body content but does not declare a content field",
                        union_def.name, case.name
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            &spec,
            &element.tag,
            content_from_body,
            span,
            "union-case-field-type-mismatch",
            "unknown-union-case-field",
            "missing-union-case-field",
        );
    }

    /// Resolves a nominal name written by a declaration in `module_identity`.
    ///
    /// A declaration's type references are written in its own namespace: `fit: Fit` in a library
    /// means that library's `Fit`. Resolving it in the consumer's scope is what leaves the type as
    /// an unresolved `Type::Named`, and what lets an unrelated local `Fit` stand in for it.
    fn nominal_type_in_module(&mut self, module_identity: &str, name: &Name) -> Option<Type> {
        if module_identity == self.module.module_identity() {
            return None;
        }

        // The declaring module's own namespace, which includes what it imported. A type it named
        // but did not declare is reached here and nowhere else. Records and unions live in the
        // type namespace, components in the element namespace, and a property may be typed by any
        // of the three.
        let peer_entry = self
            .module
            .peer_entry(PreparedNamespace::Type, module_identity, name)
            .or_else(|| {
                self.module
                    .peer_entry(PreparedNamespace::Element, module_identity, name)
            });
        if let Some(entry) = peer_entry {
            let origin = entry.clone();
            if let Some(item) = self
                .module
                .peer_module(origin.module_identity())
                .and_then(|module| module.item_by_definition(origin.definition_id()))
                .cloned()
            {
                if let Some(ty) = self.foreign_nominal_type(name, &item, origin) {
                    return Some(ty);
                }
            }
        }

        // A workspace peer keeps its whole lowered module, so its own definitions are readable
        // directly, whether or not the consumer imported them. This is what a module analyzed
        // outside a graph has, where no peer namespace was registered.
        if let Some((definition_id, item)) = self
            .module
            .peer_module(module_identity)
            .and_then(|peer| peer.find_item_with_definition(name.as_str()))
            .map(|(definition_id, item)| (definition_id, item.clone()))
        {
            let origin = DeclaringOrigin::new(module_identity, definition_id);
            if let Some(ty) = self.foreign_nominal_type(name, &item, origin) {
                return Some(ty);
            }
        }

        None
    }

    /// Builds the nominal type one foreign declaration denotes, under the name the reference used.
    ///
    /// The name is the contract's spelling and need not be the name the declaration was given —
    /// the declaring module may have imported it under an alias — so it is display information
    /// only. The origin decides what type this is.
    fn foreign_nominal_type(
        &mut self,
        name: &Name,
        item: &Item,
        origin: DeclaringOrigin,
    ) -> Option<Type> {
        match item {
            Item::Union(union_def) => {
                let mut union_def = union_def.clone();
                union_def.name = name.clone();
                let entry = UnionEntry {
                    def: union_def,
                    origin: Some(origin.clone()),
                };
                let ty = Type::Union(entry.shape());
                self.foreign_union_defs.insert(origin, entry);
                Some(ty)
            }
            Item::Record(_) | Item::Component(_) => {
                Some(Type::named_at(name.clone(), Some(origin)))
            }
            // An alias is a type reference the declaring module wrote, so what it names is
            // resolved there too. Stopping at the alias would hand the reference back to the
            // consumer's namespace, where an unrelated same-named declaration answers for it.
            Item::TypeAlias(alias) => {
                if !self.foreign_alias_stack.insert(origin.clone()) {
                    return Some(Type::Error);
                }
                let target = alias.ty.clone();
                let ty = self.type_from_type_ref_in(Some(origin.module_identity()), &target);
                self.foreign_alias_stack.remove(&origin);
                Some(ty)
            }
            _ => None,
        }
    }

    /// Converts a type reference written by a declaration owned by `declaring_module`.
    ///
    /// The declaring module is tried first, so an unrelated local type that merely shares the
    /// spelling cannot stand in for the one the declaration actually named.
    fn type_from_type_ref_in(
        &mut self,
        declaring_module: Option<&str>,
        type_ref: &ast::TypeRef,
    ) -> Type {
        let Some(module_identity) = declaring_module else {
            return self.type_from_type_ref(type_ref);
        };
        let module_identity = module_identity.to_string();
        resolve_type_ref_with(type_ref, &mut |name, seen| {
            if let Some(ty) = self.nominal_type_in_module(&module_identity, name) {
                return ty;
            }
            self.resolve_named_type(name, seen)
        })
    }

    fn build_element_binding_spec_in<'b, I>(
        &mut self,
        declaring_module: Option<&str>,
        bindings: I,
    ) -> ElementBindingSpec
    where
        I: IntoIterator<Item = (&'b Name, &'b ast::TypeRef, bool, bool)>,
    {
        let mut content_property = None;
        let mut properties = FxHashMap::default();

        for (name, ty_ref, is_content, is_required) in bindings {
            let ty = self.type_from_type_ref_in(declaring_module, ty_ref);
            if is_content {
                content_property = Some(name.clone());
            }
            properties.insert(name.clone(), ElementPropertySpec { ty, is_required });
        }

        ElementBindingSpec {
            content_property,
            properties,
            handler_properties: FxHashSet::default(),
        }
    }

    fn check_element_bindings(
        &mut self,
        element: &nx_hir::Element,
        span: TextSpan,
        spec: &ElementBindingSpec,
    ) {
        let property_paths = self.property_paths_for_entries(element.property_entries());
        self.report_duplicate_property_paths(&property_paths, &element.tag);

        let content_from_body = if !element.content.is_empty() {
            if let Some(content_name) = spec.content_property.as_ref() {
                if property_paths.iter().any(|path| {
                    path.properties
                        .iter()
                        .any(|property| property.key == *content_name)
                }) {
                    self.error(
                        "content-binding-conflict",
                        format!(
                            "Element '{}' passes content for '{}' both as a property and as body content",
                            element.tag, content_name
                        ),
                        span,
                    );
                    false
                } else if let Some(expected) = spec.properties.get(content_name) {
                    self.check_content_binding(
                        &element.content,
                        &expected.ty,
                        span,
                        format!("Content for '{}' binds to '{}'", element.tag, content_name),
                    );
                    true
                } else {
                    false
                }
            } else {
                self.error(
                    "missing-content-property",
                    format!(
                        "Element '{}' passes body content, but '{}' does not declare a content property",
                        element.tag, element.tag
                    ),
                    span,
                );
                false
            }
        } else {
            false
        };

        self.check_property_path_bindings(
            &property_paths,
            spec,
            &element.tag,
            content_from_body,
            span,
            "property-type-mismatch",
            "unknown-property",
            "missing-property",
        );
    }

    fn property_paths_for_entries(&mut self, entries: &[PropertyEntry]) -> Vec<PropertyPath> {
        let mut paths = vec![PropertyPath {
            properties: Vec::new(),
        }];

        for entry in entries {
            let alternatives = self.property_paths_for_entry(entry);
            let mut next_paths = Vec::new();
            for path in &paths {
                for alternative in &alternatives {
                    let mut properties = path.properties.clone();
                    properties.extend(alternative.properties.clone());
                    next_paths.push(PropertyPath { properties });
                }
            }
            paths = next_paths;
        }

        paths
    }

    fn property_paths_for_entry(&mut self, entry: &PropertyEntry) -> Vec<PropertyPath> {
        match entry {
            PropertyEntry::Value(property) => vec![PropertyPath {
                properties: vec![PropertyPathBinding {
                    key: property.key.clone(),
                    ty: self.infer_expr(property.value),
                    span: property.span,
                    value: property.value,
                }],
            }],
            PropertyEntry::If {
                condition,
                then_entries,
                else_entries,
                span,
            } => {
                self.check_boolean_condition(*condition, *span, "property-list if condition");
                let mut paths = self.property_paths_for_entries(then_entries);
                paths.extend(self.property_paths_for_entries(else_entries));
                paths
            }
            PropertyEntry::ConditionList {
                arms, else_entries, ..
            } => {
                let mut paths = Vec::new();
                for arm in arms {
                    self.check_boolean_condition(
                        arm.condition,
                        arm.span,
                        "property-list condition arm",
                    );
                    paths.extend(self.property_paths_for_entries(&arm.entries));
                }
                paths.extend(self.property_paths_for_entries(else_entries));
                paths
            }
            PropertyEntry::Match {
                scrutinee,
                arms,
                else_entries,
                span,
            } => self.property_paths_for_match(*scrutinee, arms, else_entries, *span),
        }
    }

    fn property_paths_for_match(
        &mut self,
        scrutinee: ExprId,
        arms: &[nx_hir::PropertyMatchArm],
        else_entries: &[PropertyEntry],
        span: TextSpan,
    ) -> Vec<PropertyPath> {
        let scrutinee_ty = self.infer_expr(scrutinee);
        let scrutinee_name = match self.module.raw_module().expr(scrutinee) {
            ast::Expr::Ident(name) => Some(name.clone()),
            _ => None,
        };
        let union_ty = match &scrutinee_ty {
            Type::Union(union_ty) => Some(union_ty.clone()),
            _ => None,
        };

        let mut covered_cases = FxHashSet::default();
        let mut paths = Vec::new();

        for arm in arms {
            let pattern_tys = arm
                .patterns
                .iter()
                .map(|pattern| {
                    let pattern_ty = self.infer_match_pattern(*pattern, &scrutinee_ty);
                    self.check_match_pattern(
                        &scrutinee_ty,
                        union_ty.as_ref(),
                        &pattern_ty,
                        &mut covered_cases,
                        self.module.raw_module().expr(*pattern).span(),
                    );
                    pattern_ty
                })
                .collect::<Vec<_>>();

            let narrowed_case = self.match_arm_narrowed_case(union_ty.as_ref(), &pattern_tys);
            if let (Some(name), Some(case_ty)) = (scrutinee_name.as_ref(), narrowed_case) {
                self.env.push_scope();
                self.env.bind(name.clone(), Type::UnionCase(case_ty));
                paths.extend(self.property_paths_for_entries(&arm.entries));
                self.env.pop_scope();
            } else {
                paths.extend(self.property_paths_for_entries(&arm.entries));
            }
        }

        let is_exhaustive = union_ty.as_ref().is_some_and(|union_ty| {
            union_ty
                .cases
                .iter()
                .all(|case| covered_cases.contains(case))
        });

        if !else_entries.is_empty() {
            paths.extend(self.property_paths_for_entries(else_entries));
        } else if let Some(union_ty) = union_ty.as_ref() {
            if !is_exhaustive {
                let missing = union_ty
                    .cases
                    .iter()
                    .filter(|case| !covered_cases.contains(*case))
                    .map(|case| case.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error(
                    "non-exhaustive-union-match",
                    format!(
                        "Union match on '{}' is missing cases: {}",
                        union_ty.name, missing
                    ),
                    span,
                );
                paths.push(PropertyPath {
                    properties: Vec::new(),
                });
            }
        } else {
            paths.push(PropertyPath {
                properties: Vec::new(),
            });
        }

        paths
    }

    fn check_boolean_condition(&mut self, condition: ExprId, span: TextSpan, context: &str) {
        let condition_ty = self.infer_expr(condition);
        if !condition_ty.is_error()
            && !self.type_satisfies_expected(&condition_ty, &Type::boolean())
        {
            self.error(
                "type-mismatch",
                format!("{} expects boolean, found {}", context, condition_ty),
                span,
            );
        }
    }

    fn report_duplicate_property_paths(&mut self, paths: &[PropertyPath], element_name: &Name) {
        let mut reported = FxHashSet::<(Name, usize, usize)>::default();
        for path in paths {
            let mut seen = FxHashSet::<Name>::default();
            for property in &path.properties {
                if !seen.insert(property.key.clone()) {
                    let start: usize = property.span.start().into();
                    let end: usize = property.span.end().into();
                    if reported.insert((property.key.clone(), start, end)) {
                        self.error(
                            "duplicate-property",
                            format!(
                                "Property '{}' on '{}' can be supplied more than once on the same path",
                                property.key, element_name
                            ),
                            property.span,
                        );
                    }
                }
            }
        }
    }

    fn check_property_path_bindings(
        &mut self,
        paths: &[PropertyPath],
        spec: &ElementBindingSpec,
        element_name: &Name,
        content_from_body: bool,
        span: TextSpan,
        type_mismatch_code: &str,
        unknown_property_code: &str,
        missing_property_code: &str,
    ) {
        let mut reported_unknown = FxHashSet::<(Name, usize, usize)>::default();

        for path in paths {
            for property in &path.properties {
                if let Some(expected) = spec.properties.get(&property.key) {
                    self.check_typed_binding_for(
                        Some(property.value),
                        &property.ty,
                        &expected.ty,
                        property.span,
                        type_mismatch_code,
                        format!("Property '{}' on '{}'", property.key, element_name),
                    );
                } else if spec.handler_properties.contains(&property.key) {
                    continue;
                } else {
                    let start: usize = property.span.start().into();
                    let end: usize = property.span.end().into();
                    if reported_unknown.insert((property.key.clone(), start, end)) {
                        self.error(
                            unknown_property_code,
                            format!(
                                "Element '{}' has no property '{}'",
                                element_name, property.key
                            ),
                            property.span,
                        );
                    }
                }
            }
        }

        for (name, expected) in &spec.properties {
            if !expected.is_required {
                continue;
            }

            let supplied_by_body = content_from_body
                && spec
                    .content_property
                    .as_ref()
                    .is_some_and(|prop| prop == name);
            if supplied_by_body {
                continue;
            }

            if paths
                .iter()
                .any(|path| !path.properties.iter().any(|property| property.key == *name))
            {
                self.error(
                    missing_property_code,
                    format!("Element '{}' requires property '{}'", element_name, name),
                    span,
                );
            }
        }
    }

    fn normalized_sequence_type(&mut self, exprs: &[ExprId], span: TextSpan) -> Type {
        if exprs.is_empty() {
            return Type::array(self.fresh_var());
        }

        if exprs.len() == 1 {
            return self.infer_expr(exprs[0]);
        }

        let item_types: Vec<_> = exprs
            .iter()
            .map(|expr_id| match self.infer_expr(*expr_id) {
                Type::Array(inner) => *inner,
                other => other,
            })
            .collect();

        Type::array(self.common_sequence_item_type(&item_types, span))
    }

    fn common_sequence_item_type(&self, item_types: &[Type], _span: TextSpan) -> Type {
        let mut current = item_types
            .first()
            .cloned()
            .unwrap_or_else(|| Type::named("object"));

        for ty in item_types.iter().skip(1) {
            current = self.common_supertype(&current, ty);
        }

        current
    }

    /// Strips the wrappers a contextual name is allowed to resolve through.
    ///
    /// Nullability first, then one list level, so `Fit?` and `Fit[]` both resolve against `Fit` and
    /// the existing scalar-to-list coercion applies to the resolved value.
    fn contextual_target<'ty>(expected: &'ty Type) -> &'ty Type {
        let expected = match expected {
            Type::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        match expected {
            Type::Array(inner) => match inner.as_ref() {
                Type::Nullable(inner) => inner.as_ref(),
                other => other,
            },
            other => other,
        }
    }

    /// Suggests the closest candidate to `name`, for a did-you-mean on an unresolved bare name.
    fn closest_candidate(name: &Name, candidates: &[Name]) -> Option<Name> {
        candidates
            .iter()
            .map(|candidate| {
                (
                    Self::edit_distance(name.as_str(), candidate.as_str()),
                    candidate,
                )
            })
            .filter(|(distance, candidate)| *distance <= 2.max(candidate.as_str().len() / 3))
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, candidate)| candidate.clone())
    }

    fn edit_distance(a: &str, b: &str) -> usize {
        let b_chars: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
        let mut current = vec![0usize; b_chars.len() + 1];
        for (i, a_char) in a.chars().enumerate() {
            current[0] = i + 1;
            for (j, b_char) in b_chars.iter().enumerate() {
                let cost = usize::from(a_char != *b_char);
                current[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(current[j] + 1);
            }
            std::mem::swap(&mut prev, &mut current);
        }
        prev[b_chars.len()]
    }

    fn candidate_list(candidates: &[Name]) -> String {
        candidates
            .iter()
            .map(|c| c.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Resolves a bare name against the expected type of its binding site.
    ///
    /// The single entry point for contextual literal resolution, shared by every binding site so
    /// the rule cannot drift between them. Resolves only against the closed nominal set — the
    /// constant cases of a union — and never falls back to treating the name as a string.
    fn resolve_contextual_name(
        &mut self,
        expr: ExprId,
        name: &Name,
        expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<Type> {
        self.resolve_contextual_name_in(expr, name, expected, span, context, false)
    }

    /// Resolves a bare name, optionally admitting union cases that carry a payload.
    ///
    /// A pattern matches on the discriminator, so `failed` is a valid pattern for a payload case
    /// even though it is not a valid way to construct one.
    fn resolve_contextual_name_in(
        &mut self,
        expr: ExprId,
        name: &Name,
        expected: &Type,
        span: TextSpan,
        context: &str,
        allow_payload_cases: bool,
    ) -> Option<Type> {
        let target = Self::contextual_target(expected).clone();

        // One nominal kind. What `enum` declared is a union whose cases are all constant, so a
        // bare name resolves against one closed set rather than two.
        //
        // Only a resolved `Type::Union` counts. A `Type::Named` reaching here is a name resolution
        // did not find a declaration for, and looking it up again in *this* module's namespace is
        // what let a same-named local declaration stand in for a foreign one.
        let union_info = match &target {
            Type::Union(info) => Some(info.clone()),
            _ => None,
        };
        if let Some(info) = union_info {
            let union_name = info.name.clone();

            // The resolved type's own cases are authoritative. Looking the name up again here
            // would find whatever declaration *this* module binds to that spelling, which need
            // not be the same type — that is how a same-named local declaration used to stand in
            // for a foreign one.
            if !info.cases.iter().any(|case| case == name) {
                let suggestion = Self::closest_candidate(name, &info.cases)
                    .map(|s| format!("; did you mean `{}`?", s))
                    .unwrap_or_default();
                self.error(
                    "unresolved-contextual-name",
                    format!(
                        "'{}' is not a case of union '{}'{} Cases: {}",
                        name,
                        self.display_union_name(&info),
                        suggestion,
                        Self::candidate_list(&info.cases)
                    ),
                    span,
                );
                return Some(Type::Error);
            }

            // The declaration reached under that name, and only when it really is that type.
            // Whether this module can *name* the union does not matter: the case it resolved to
            // carries the union's declaring origin from here on, so nothing below type checking
            // has to find the declaration again by a spelling that is visible here.
            let entry = self.union_entry_for(&union_name, info.origin()).cloned();

            let Some(entry) = entry else {
                // The expected type resolved to this union, so its definition was reached to build
                // it. Not finding it again is an internal inconsistency, not an authoring error.
                self.error(
                    "unresolved-contextual-name",
                    format!(
                        "'{}' resolves to '{}.{}', but the definition of '{}' could not be reached",
                        name, union_name, name, union_name
                    ),
                    span,
                );
                return Some(Type::Error);
            };
            let case = entry
                .def
                .cases
                .iter()
                .find(|case| case.name == *name)
                .expect("the resolved type lists this case");
            let case_name = case.name.clone();
            // A fieldless case of a union with an abstract base still constructs — it takes the
            // base's fields and their defaults — so nameability, not constant-ness, is what the
            // bare form needs here.
            let is_fieldless = case.fields.is_empty();

            if is_fieldless || allow_payload_cases {
                self.resolved_contextual_names.insert(
                    expr,
                    ContextualResolution {
                        type_name: union_name.clone(),
                        member: case_name.clone(),
                        origin: entry.origin.clone(),
                    },
                );
                let resolved = entry.case_type(case_name);
                self.env.set_expr_type(expr, resolved.clone());
                return Some(resolved);
            }

            self.error(
                "payload-union-case-requires-constructor",
                format!(
                    "Union case '{}.{}' requires element-style payload construction; write \
                     `<{}.{} ... />`",
                    union_name, case_name, union_name, case_name
                ),
                span,
            );
            return Some(Type::Error);
        }

        // Not a nominal type: a bare name never falls back to being a string.
        self.error(
            "contextual-name-requires-nominal-type",
            format!(
                "{} expects {}, and a bare name resolves only against a union's cases; \
                 for a string value write \"{}\"",
                context, expected, name
            ),
            span,
        );
        Some(Type::Error)
    }

    /// Suggests the bare form when a string was written at a site that wants a nominal value.
    ///
    /// A quoted string never resolves to a union case, so the fix is the bare
    /// spelling rather than a different string.
    fn bare_form_hint(&self, actual: &Type, expected: &Type) -> String {
        if !matches!(actual, Type::Primitive(crate::ty::Primitive::String)) {
            return String::new();
        }
        let target = Self::contextual_target(expected);
        let candidates: Vec<Name> = match target {
            Type::Union(info) => info.cases.clone(),
            _ => Vec::new(),
        };
        if candidates.is_empty() {
            return String::new();
        }
        format!(
            "; a quoted string is never a member of {}, so write the bare form, one of: {}",
            expected,
            Self::candidate_list(&candidates)
        )
    }

    /// Checks body content against the type its content property declares.
    ///
    /// <para>Content is a sequence of expressions with no expression of its own, so a rule that
    /// attaches to an expression — a contextual name, an integer literal at a float site — cannot
    /// reach it the way it reaches a property binding. A single content expression is checked as
    /// itself; several are checked as the elements of the declared list.</para>
    fn check_content_binding(
        &mut self,
        content: &[ExprId],
        expected: &Type,
        span: TextSpan,
        context: String,
    ) -> bool {
        if content.len() == 1 {
            let actual = self.infer_expr(content[0]);
            return self.check_typed_binding_for(
                Some(content[0]),
                &actual,
                expected,
                span,
                "content-type-mismatch",
                context,
            );
        }

        let actual = self.normalized_sequence_type(content, span);
        if self.type_satisfies_expected_with_coercion(&actual, expected) {
            return true;
        }

        if let Type::Array(element_expected) = expected.strip_nullable() {
            match self.convert_int_literals_in(content, element_expected, span, &context) {
                Some(true) => return true,
                // The inexactness diagnostic is already reported.
                Some(false) => return false,
                None => {}
            }
        }

        self.check_typed_binding(&actual, expected, span, "content-type-mismatch", context)
    }

    fn check_typed_binding(
        &mut self,
        actual: &Type,
        expected: &Type,
        span: TextSpan,
        code: &str,
        context: String,
    ) -> bool {
        self.check_typed_binding_for(None, actual, expected, span, code, context)
    }

    /// Checks a binding, recording the resolution when `expr` is a contextual name.
    ///
    /// Sites that know which expression they are checking pass it, so a resolved contextual name
    /// can be rewritten to its qualified form after analysis. Sites that do not pass `None`, and a
    /// contextual name reaching one of those is reported rather than silently accepted.
    fn check_typed_binding_for(
        &mut self,
        expr: Option<ExprId>,
        actual: &Type,
        expected: &Type,
        span: TextSpan,
        code: &str,
        context: String,
    ) -> bool {
        // A pending contextual name resolves here, where the expected type is finally known.
        if let Type::ContextualName(name) = actual {
            let name = name.clone();
            let Some(expr) = expr else {
                self.error(
                    "contextual-name-without-expected-type",
                    format!(
                        "{}: a bare name is only allowed where the declared type is known;                          write the qualified form in braces instead",
                        context
                    ),
                    span,
                );
                return false;
            };
            let resolved = self.resolve_contextual_name(expr, &name, expected, span, &context);
            return match resolved {
                Some(Type::Error) => false,
                Some(resolved) => {
                    self.check_typed_binding_for(None, &resolved, expected, span, code, context)
                }
                None => false,
            };
        }

        if self.type_satisfies_expected_with_coercion(actual, expected) {
            return true;
        }

        // An integer literal written where a float is declared takes the declared type. Tried only
        // after ordinary satisfaction, so a site that already accepts the value — `object`, an
        // undecided type variable — keeps the literal an integer.
        if let Some(expr) = expr {
            if let Some(converted) = self.convert_int_literals(expr, expected, span, &context) {
                return converted;
            }
        }

        // Two same-named types are told apart by their declaring modules; one nominal type in
        // a message is left unqualified.
        let (expected_display, mut actual_display) = crate::display_type_pair(expected, actual);
        if Self::is_null_literal_type(actual) {
            actual_display = "null".to_string();
        }
        let message = if matches!(actual, Type::Array(_)) && !matches!(expected, Type::Array(_)) {
            format!(
                "{} expects {}, found list {}",
                context, expected_display, actual_display
            )
        } else {
            let hint = self.bare_form_hint(actual, expected);
            format!(
                "{} expects {}, found {}{}",
                context, expected_display, actual_display, hint
            )
        };
        self.error(code, message, span);
        false
    }

    /// Types the integer literals `expr` is made of by the floating-point type expected of them.
    ///
    /// <para>Returns `None` when the rule does not reach this expression, so the caller reports its
    /// own mismatch; `Some(true)` when every literal converted, and `Some(false)` when one could not
    /// be represented exactly and the diagnostic has already been reported.</para>
    ///
    /// <para>A list is walked because its elements are each written at the element type, and the
    /// binding site names only the list. A single literal at a list-typed site is reached the same
    /// way, since a scalar binds there by coercion.</para>
    fn convert_int_literals(
        &mut self,
        expr: ExprId,
        expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<bool> {
        match self.module.raw_module().expr(expr).clone() {
            ast::Expr::Literal(ast::Literal::Int(value)) => {
                let target = float_literal_target(expected)?;
                if !target.represents_integer_exactly(value) {
                    self.error(
                        "float-literal-not-exact",
                        format!(
                            "{}: {} is not exactly representable as {}; write the value you mean as \
                             a {} literal",
                            context, value, target, target
                        ),
                        span,
                    );
                    return Some(false);
                }
                // The recorded type moves with the value. Leaving it `int` would put a float
                // literal in the IR under an integer type annotation, which is the inconsistency
                // the conversion exists to prevent rather than a cosmetic mismatch.
                //
                // It becomes `float64` rather than the target, because that is the type a written
                // real literal takes at the same site — `infer_literal` gives every float literal
                // `float64`, and a `float32` site narrows it no further. Recording the target here
                // instead would make the converted `24` more precisely typed than the `24.0` it is
                // supposed to be indistinguishable from. Which type a float literal should take at
                // a `float32` site is a real question, but it is the same question for both
                // spellings and not one this change answers.
                self.env.set_expr_type(expr, Type::float64());
                self.converted_int_literals.insert(expr, target);
                Some(true)
            }
            ast::Expr::Array { elements, .. } => {
                let Type::Array(element_expected) = expected.strip_nullable() else {
                    return None;
                };
                if !self.convert_int_literals_in(&elements, element_expected, span, context)? {
                    return Some(false);
                }
                // The list's own recorded type was inferred from elements that were still
                // integers, so it says `int[]` over elements that are now floats. Recomputing it
                // the way inference would have is what keeps the list indistinguishable from one
                // whose elements were written as real literals.
                let item_types: Vec<_> = elements
                    .iter()
                    .map(|element| {
                        self.env
                            .get_expr_type(*element)
                            .cloned()
                            .unwrap_or(Type::Error)
                    })
                    .collect();
                let item_ty = self.common_sequence_item_type(&item_types, span);
                self.env.set_expr_type(expr, Type::array(item_ty));
                Some(true)
            }
            _ => None,
        }
    }

    /// Types the integer literals in a sequence of expressions by the type expected of each one.
    ///
    /// <para>Every element has to end up satisfying the element type. One that is not a convertible
    /// literal must already do so on its own, or the sequence as a whole does not bind and the
    /// caller's mismatch is the right diagnostic.</para>
    fn convert_int_literals_in(
        &mut self,
        elements: &[ExprId],
        element_expected: &Type,
        span: TextSpan,
        context: &str,
    ) -> Option<bool> {
        let mut converted_any = false;
        for element in elements {
            match self.convert_int_literals(*element, element_expected, span, context) {
                Some(true) => converted_any = true,
                Some(false) => return Some(false),
                None => {
                    let actual = self.env.get_expr_type(*element)?.clone();
                    if !self.type_satisfies_expected_with_coercion(&actual, element_expected) {
                        return None;
                    }
                }
            }
        }
        if converted_any {
            Some(true)
        } else {
            None
        }
    }

    /// Records a type error.
    fn error(&mut self, code: &str, message: String, span: nx_diagnostics::TextSpan) {
        let diag = Diagnostic::error(code)
            .with_message(message)
            .with_label(Label::primary(self.file_name.clone(), span))
            .build();
        self.diagnostics.push(diag);
    }

    /// Returns the contextual names resolved during analysis, as `expr → (type, member)`.
    pub fn resolved_contextual_names(&self) -> &FxHashMap<ExprId, ContextualResolution> {
        &self.resolved_contextual_names
    }

    /// Returns the integer literals that took a floating-point type from their binding site.
    pub fn converted_int_literals(&self) -> &FxHashMap<ExprId, Primitive> {
        &self.converted_int_literals
    }

    /// Returns the collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns the type environment.
    pub fn env(&self) -> &TypeEnvironment {
        &self.env
    }

    /// Consumes the context and returns the environment and diagnostics.
    pub fn finish(self) -> (TypeEnvironment, Vec<Diagnostic>) {
        (self.env, self.diagnostics)
    }

    fn register_type_definitions(&mut self) {
        let bindings = self
            .module
            .bindings(PreparedNamespace::Type)
            .cloned()
            .collect::<Vec<_>>();

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };
            // The declaration this name reaches, which is what makes the type it denotes the same
            // type wherever else it is reached — under another name, or from another module.
            let origin = DeclaringOrigin::new(resolved.module_identity(), resolved.definition_id());

            match resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::TypeAlias(ref alias),
                    ..
                } => {
                    self.type_aliases.insert(
                        binding.visible_name.clone(),
                        TypeAliasInfo {
                            target: alias.ty.clone(),
                            span: alias.span,
                        },
                    );
                }
                ResolvedPreparedItem::Imported { ref item, .. } => {
                    if let Some(alias) = interface_type_alias(item) {
                        self.type_aliases.insert(
                            binding.visible_name.clone(),
                            TypeAliasInfo {
                                target: alias.ty,
                                span: alias.span,
                            },
                        );
                    } else if let Some(mut union_def) = interface_union(item) {
                        union_def.name = binding.visible_name.clone();
                        self.union_defs.insert(
                            binding.visible_name.clone(),
                            UnionEntry {
                                def: union_def,
                                origin: Some(origin),
                            },
                        );
                    } else if matches!(item.item, InterfaceItemKind::Record { .. }) {
                        self.record_origins
                            .insert(binding.visible_name.clone(), origin);
                    }
                }
                ResolvedPreparedItem::Raw {
                    item: Item::Record(_),
                    ..
                } => {
                    self.record_origins
                        .insert(binding.visible_name.clone(), origin);
                }
                ResolvedPreparedItem::Raw {
                    item: Item::Union(ref union_def),
                    ..
                } => {
                    let mut union_def = union_def.clone();
                    union_def.name = binding.visible_name.clone();
                    self.union_defs.insert(
                        binding.visible_name.clone(),
                        UnionEntry {
                            def: union_def,
                            origin: Some(origin),
                        },
                    );
                }
                _ => {}
            }
        }

        let element_bindings = self
            .module
            .bindings(PreparedNamespace::Element)
            .cloned()
            .collect::<Vec<_>>();

        for binding in element_bindings {
            if binding.kind != PreparedItemKind::Component {
                continue;
            }
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };
            self.component_origins
                .insert(binding.visible_name.clone(), resolved.declaring_origin());
        }
    }

    fn validate_local_record_defaults(&mut self) {
        let local_items = self.module.raw_module().items().to_vec();
        for item in local_items {
            if let Item::Record(record_def) = item {
                for prop in &record_def.properties {
                    if let Some(default_expr) = prop.default {
                        let expected = self.type_from_type_ref(&prop.ty);
                        let actual = self.infer_expr(default_expr);
                        self.check_typed_binding_for(
                            Some(default_expr),
                            &actual,
                            &expected,
                            prop.span,
                            "record-default-type-mismatch",
                            format!("Default value for record property '{}'", prop.name),
                        );
                    }
                }
            }
        }
    }

    fn validate_local_union_defaults(&mut self) {
        let local_items = self.module.raw_module().items().to_vec();
        for item in local_items {
            if let Item::Union(union_def) = item {
                for case in &union_def.cases {
                    for field in &case.fields {
                        if let Some(default_expr) = field.default {
                            let expected = self.type_from_type_ref(&field.ty);
                            let actual = self.infer_expr(default_expr);
                            self.check_typed_binding_for(
                                Some(default_expr),
                                &actual,
                                &expected,
                                field.span,
                                "union-case-default-type-mismatch",
                                format!(
                                    "Default value for union case field '{}.{}.{}'",
                                    union_def.name, case.name, field.name
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    fn register_function_signatures(&mut self) {
        let bindings = self
            .module
            .bindings(PreparedNamespace::Value)
            .cloned()
            .collect::<Vec<_>>();

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };

            // A function's parameter and return annotations are names the *declaring* module
            // wrote, so they are resolved there. Resolving them here would let an unrelated local
            // declaration sharing the spelling become an imported function's signature.
            let declaring_module = resolved.module_identity().to_string();

            match resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(func),
                    origin,
                    ..
                } => {
                    let return_type = if let Some(ty) = func.return_type.as_ref() {
                        self.type_from_type_ref_in(Some(&declaring_module), ty)
                    } else {
                        let placeholder = self.fresh_var();
                        if matches!(origin, PreparedBindingOrigin::Local) {
                            self.function_return_placeholders
                                .insert(binding.visible_name.clone(), placeholder.clone());
                        }
                        placeholder
                    };

                    self.bind_function_signature_from_parts(
                        binding.visible_name.clone(),
                        &func.params,
                        return_type,
                        Some(&declaring_module),
                    );
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let Some((_name, _visibility, params, return_type, _span)) =
                        interface_function_signature(&item)
                    {
                        let param_types = params
                            .iter()
                            .map(|param| {
                                self.type_from_type_ref_in(Some(&declaring_module), &param.ty)
                            })
                            .collect::<Vec<_>>();
                        let return_type =
                            self.type_from_type_ref_in(Some(&declaring_module), &return_type);
                        self.env.bind(
                            binding.visible_name.clone(),
                            Type::function(param_types, return_type),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn register_value_bindings(&mut self) {
        let bindings = self
            .module
            .bindings(PreparedNamespace::Value)
            .cloned()
            .collect::<Vec<_>>();

        for binding in bindings {
            let Some(resolved) = self.module.resolve_prepared_item(&binding) else {
                continue;
            };

            // An imported value's annotation is a name the *declaring* module wrote, so it is
            // resolved there. Resolving it here would let an unrelated local declaration sharing
            // the spelling become the imported value's type.
            let declaring_module = resolved.module_identity().to_string();

            match resolved {
                ResolvedPreparedItem::Raw {
                    module_identity,
                    item: Item::Value(value),
                    ..
                } => {
                    let binding_ty = if module_identity == self.module.module_identity() {
                        let actual = self.infer_expr(value.value);
                        if let Some(ty_ref) = value.ty.as_ref() {
                            let expected = self.type_from_type_ref(ty_ref);
                            self.check_typed_binding_for(
                                Some(value.value),
                                &actual,
                                &expected,
                                value.span,
                                "value-type-mismatch",
                                format!("Initializer for value '{}'", value.name),
                            );
                            expected
                        } else {
                            actual
                        }
                    } else {
                        value
                            .ty
                            .as_ref()
                            .map(|ty_ref| {
                                self.type_from_type_ref_in(Some(&declaring_module), ty_ref)
                            })
                            .unwrap_or(Type::Error)
                    };

                    self.env.bind(binding.visible_name.clone(), binding_ty);
                }
                ResolvedPreparedItem::Imported { item, .. } => {
                    if let InterfaceItemKind::Value { ty, .. } = &item.item {
                        let ty = ty.clone();
                        let binding_ty = self.type_from_type_ref_in(Some(&declaring_module), &ty);
                        self.env.bind(binding.visible_name.clone(), binding_ty);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_function_definition(&self, name: &Name) -> Option<ResolvedPreparedItem> {
        self.module
            .resolve_binding(PreparedNamespace::Element, name)
            .and_then(|binding| self.module.resolve_prepared_item(binding))
            .and_then(|resolved| match &resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Function(_),
                    ..
                } => Some(resolved),
                ResolvedPreparedItem::Imported { item, .. }
                    if matches!(item.item, InterfaceItemKind::Function { .. }) =>
                {
                    Some(resolved)
                }
                _ => None,
            })
    }

    fn resolve_component_definition(&self, name: &Name) -> Option<ResolvedPreparedItem> {
        self.module
            .resolve_binding(PreparedNamespace::Element, name)
            .and_then(|binding| self.module.resolve_prepared_item(binding))
            .and_then(|resolved| match &resolved {
                ResolvedPreparedItem::Raw {
                    item: Item::Component(_),
                    ..
                } => Some(resolved),
                ResolvedPreparedItem::Imported { item, .. }
                    if matches!(item.item, InterfaceItemKind::Component { .. }) =>
                {
                    Some(resolved)
                }
                _ => None,
            })
    }

    fn resolve_record_definition(&self, name: &Name) -> Option<nx_hir::RecordDef> {
        nx_hir::resolve_record_definition(self.module, name)
    }

    fn resolve_record_definition_with_origin(
        &self,
        name: &Name,
    ) -> Option<(String, nx_hir::RecordDef)> {
        nx_hir::resolve_record_definition_with_module(self.module, name)
    }

    fn union_info_for_expr(&self, expr_id: ExprId) -> Option<UnionType> {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Ident(name) => {
                let mut seen = FxHashSet::default();
                self.union_info_from_name(name, &mut seen)
            }
            _ => None,
        }
    }

    /// The union a type name denotes here, following type aliases.
    fn union_info_from_name(&self, name: &Name, seen: &mut FxHashSet<Name>) -> Option<UnionType> {
        if let Some(entry) = self.union_defs.get(name) {
            return Some(entry.shape());
        }

        if let Some(alias) = self.type_aliases.get(name) {
            if !seen.insert(name.clone()) {
                return None;
            }
            if let ast::TypeRef::Name(target) = &alias.target {
                let target_info = self.union_info_from_name(target, seen);
                seen.remove(name);
                return target_info;
            }
            seen.remove(name);
        }

        None
    }

    /// Types a member access against a union whose type is already resolved.
    fn union_case_by_member(
        &mut self,
        union_info: &UnionType,
        member: &Name,
        span: TextSpan,
    ) -> Type {
        if union_info.cases.iter().any(|case| case == member) {
            return Type::union_case_type(
                union_info.name.clone(),
                member.clone(),
                union_info.origin().cloned(),
            );
        }
        self.report_unknown_union_case(union_info, member, span);
        Type::Error
    }

    /// Reports a member access that names something the union does not declare.
    fn report_unknown_union_case(&mut self, union_info: &UnionType, case: &Name, span: TextSpan) {
        let suggestion = Self::closest_candidate(case, &union_info.cases)
            .map(|s| format!("; did you mean `{}`?", s))
            .unwrap_or_default();
        self.error(
            "undefined-union-case",
            format!(
                "Union '{}' has no case named '{}'{} Cases: {}",
                self.display_union_name(union_info),
                case,
                suggestion,
                Self::candidate_list(&union_info.cases)
            ),
            span,
        );
    }

    /// Renders a union's name for a diagnostic, qualified when another union here shares it.
    ///
    /// <para>A message that says `union 'Fit'` while a different `Fit` is declared in the module
    /// the author is reading describes the wrong declaration to them: theirs does have the case the
    /// message says does not exist. Naming the declaring module is what tells the two apart, and it
    /// is added only when there are two to tell apart.</para>
    fn display_union_name(&self, union_info: &UnionType) -> String {
        let Some(origin) = union_info.origin() else {
            return union_info.name.to_string();
        };
        let shares_the_name_here = self
            .union_defs
            .get(&union_info.name)
            .is_some_and(|entry| entry.origin.as_ref() != Some(origin));
        if shares_the_name_here {
            format!("{}:{}", origin.module_identity(), union_info.name)
        } else {
            union_info.name.to_string()
        }
    }

    fn union_case_from_qualified_name<'info>(
        &'info self,
        name: &Name,
    ) -> Option<(&'info UnionEntry, &'info UnionCaseDef)> {
        let (union_name, case_name) = name.as_str().rsplit_once('.')?;
        let union_name = Name::new(union_name);
        let case_name = Name::new(case_name);
        let entry = self.union_defs.get(&union_name)?;
        let case = entry.def.cases.iter().find(|case| case.name == case_name)?;
        Some((entry, case))
    }

    fn type_from_type_ref(&mut self, type_ref: &ast::TypeRef) -> Type {
        resolve_type_ref_with(type_ref, &mut |name, seen| {
            self.resolve_named_type(name, seen)
        })
    }

    fn resolve_named_type(&mut self, name: &Name, seen: &mut FxHashSet<Name>) -> Type {
        if let Some(alias) = self.type_aliases.get(name) {
            if !seen.insert(name.clone()) {
                self.error(
                    "type-alias-cycle",
                    format!("Type alias '{}' forms a cycle", name),
                    alias.span,
                );
                return Type::Error;
            }

            let target = alias.target.clone();
            let ty = resolve_type_ref_with_seen(&target, seen, &mut |nested_name, nested_seen| {
                self.resolve_named_type(nested_name, nested_seen)
            });
            seen.remove(name);
            return ty;
        }

        if let Some(entry) = self.union_defs.get(name) {
            return Type::Union(entry.shape());
        }

        self.nominal_named_type(name)
    }

    /// The nominal type one visible name denotes, carrying the declaration it reaches.
    ///
    /// A record or component name reaches a declaration; `Element`, `object`, and a name that
    /// reaches nothing do not, and stay origin-less.
    fn nominal_named_type(&self, name: &Name) -> Type {
        let origin = self
            .record_origins
            .get(name)
            .or_else(|| self.component_origins.get(name))
            .cloned();
        Type::named_at(name.clone(), origin)
    }

    fn bind_function_signature(&mut self, func: &nx_hir::Function, return_type: Type) {
        self.bind_function_signature_from_parts(func.name.clone(), &func.params, return_type, None);
    }

    /// Binds a function's type from its parts, resolving each parameter annotation in
    /// `declaring_module` — the module that wrote the signature, or `None` for this one.
    fn bind_function_signature_from_parts(
        &mut self,
        name: Name,
        params: &[nx_hir::Param],
        return_type: Type,
        declaring_module: Option<&str>,
    ) {
        let param_types = params
            .iter()
            .map(|param| self.type_from_type_ref_in(declaring_module, &param.ty))
            .collect::<Vec<_>>();
        self.env
            .bind(name, Type::function(param_types, return_type));
    }

    fn effective_record_shape(
        &self,
        name: &Name,
    ) -> Result<Option<nx_hir::EffectiveRecordShape>, nx_hir::RecordResolutionError> {
        effective_record_shape_for_name(self.module, name)
    }

    fn effective_component_contract(
        &self,
        name: &Name,
    ) -> Result<Option<nx_hir::EffectiveComponentContract>, nx_hir::ComponentResolutionError> {
        effective_component_contract_for_name(self.module, name)
    }

    fn record_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        is_record_subtype(
            self.module,
            &actual.name,
            actual.origin(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    fn component_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        nx_hir::is_component_subtype(
            self.module,
            &actual.name,
            actual.origin(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    fn named_type_satisfies_expected(&self, actual: &NamedType, expected: &NamedType) -> bool {
        self.record_type_satisfies_expected(actual, expected)
            || self.component_type_satisfies_expected(actual, expected)
    }

    /// Decides whether a union satisfies an expected record type through its abstract base.
    ///
    /// The base is a name the *union's* module wrote, so it is resolved there. Resolving it here
    /// would let an unrelated local record of that name make the union satisfy a foreign base.
    fn union_type_satisfies_record(
        &self,
        union_name: &Name,
        union_origin: Option<&DeclaringOrigin>,
        expected: &NamedType,
    ) -> bool {
        let Some(entry) = self.union_entry_for(union_name, union_origin) else {
            return false;
        };
        let Some(base) = entry.def.base.as_ref() else {
            return false;
        };

        let base_origin = entry
            .origin
            .as_ref()
            .and_then(|origin| self.record_origin_in(origin.module_identity(), base));

        is_record_subtype(
            self.module,
            base,
            base_origin.as_ref(),
            &expected.name,
            expected.origin(),
        )
        .unwrap_or(false)
    }

    /// Returns the declaration a record name reaches in `module_identity`'s own namespace.
    fn record_origin_in(&self, module_identity: &str, name: &Name) -> Option<DeclaringOrigin> {
        if module_identity == self.module.module_identity() {
            return self.record_origins.get(name).cloned();
        }

        self.module
            .resolve_in_module(PreparedNamespace::Type, module_identity, name)
            .filter(|resolved| resolved.kind() == PreparedItemKind::Record)
            .map(|resolved| resolved.declaring_origin())
    }

    fn named_type_is_element_like(&self, name: &Name) -> bool {
        if name.as_str() == "Element" {
            return true;
        }

        if self
            .module
            .resolve_binding(PreparedNamespace::Element, name)
            .is_some()
        {
            true
        } else {
            self.module
                .resolve_binding(PreparedNamespace::Type, name)
                .or_else(|| self.module.resolve_binding(PreparedNamespace::Value, name))
                .is_none()
        }
    }

    fn type_satisfies_expected(&self, actual: &Type, expected: &Type) -> bool {
        if generic_type_satisfies_expected(actual, expected) {
            return true;
        }

        match (actual, expected) {
            (_, Type::Nullable(_)) if Self::is_null_literal_type(actual) => true,
            (Type::Named(actual_name), Type::Named(expected_name))
                if expected_name.name.as_str() == "Element" =>
            {
                self.named_type_is_element_like(&actual_name.name)
            }
            (Type::Named(actual_name), Type::Named(expected_name)) => {
                self.named_type_satisfies_expected(actual_name, expected_name)
            }
            // The case must be a case of *that* union — the one declared at the same origin.
            // Matching on the name alone, or on the name and the case list, would let a same-named
            // local declaration's case stand in for a foreign union's.
            (Type::UnionCase(case), Type::Union(union)) => case.is_case_of(union),
            (Type::UnionCase(case), Type::Named(expected_name)) => {
                self.union_type_satisfies_record(&case.union, case.origin(), expected_name)
            }
            (Type::Union(union), Type::Named(expected_name)) => {
                self.union_type_satisfies_record(&union.name, union.origin(), expected_name)
            }
            (_, Type::Nullable(expected_inner)) => {
                self.type_satisfies_expected(actual, expected_inner)
            }
            (Type::Array(actual_inner), Type::Array(expected_inner)) => {
                self.type_satisfies_expected(actual_inner, expected_inner)
            }
            _ => false,
        }
    }

    fn is_null_literal_type(ty: &Type) -> bool {
        matches!(ty, Type::Nullable(inner) if matches!(inner.as_ref(), Type::Variable(_)))
    }

    fn type_satisfies_expected_with_coercion(&self, actual: &Type, expected: &Type) -> bool {
        if self.type_satisfies_expected(actual, expected) {
            return true;
        }

        let coercion_target = expected.strip_nullable();

        match (actual, coercion_target) {
            (Type::Array(actual_inner), Type::Array(expected_inner)) => {
                self.type_satisfies_expected(actual_inner, expected_inner)
            }
            (Type::Array(_), _) if is_object_type(coercion_target) => true,
            (Type::Array(_), _) => false,
            (_, Type::Array(expected_inner)) => {
                self.type_satisfies_expected(actual, expected_inner)
            }
            _ => false,
        }
    }

    fn common_supertype(&self, lhs: &Type, rhs: &Type) -> Type {
        match (lhs, rhs) {
            (Type::Array(lhs_inner), Type::Array(rhs_inner)) => {
                Type::array(self.common_supertype(lhs_inner, rhs_inner))
            }
            (Type::Nullable(lhs_inner), Type::Nullable(rhs_inner)) => {
                Type::nullable(self.common_supertype(lhs_inner, rhs_inner))
            }
            (Type::UnionCase(lhs_case), Type::UnionCase(rhs_case))
                if lhs_case.shares_union_with(rhs_case) =>
            {
                self.union_entry_for(&lhs_case.union, lhs_case.origin())
                    .map(|entry| Type::Union(entry.shape()))
                    .unwrap_or_else(|| generic_common_supertype(lhs, rhs))
            }
            (Type::UnionCase(case), Type::Union(union))
            | (Type::Union(union), Type::UnionCase(case))
                if case.is_same_union_as(union) =>
            {
                Type::Union(union.clone())
            }
            (Type::Named(lhs_name), Type::Named(rhs_name)) => self
                .common_record_supertype(lhs_name, rhs_name)
                .or_else(|| self.common_component_supertype(lhs_name, rhs_name))
                .unwrap_or_else(|| generic_common_supertype(lhs, rhs)),
            _ => generic_common_supertype(lhs, rhs),
        }
    }

    /// The nearest record both lineages share, compared by declaration rather than by spelling.
    fn common_record_supertype(&self, lhs: &NamedType, rhs: &NamedType) -> Option<Type> {
        let lhs_shape = self.record_shape_of(lhs).ok().flatten()?;
        let rhs_shape = self.record_shape_of(rhs).ok().flatten()?;

        let lhs_lineage = record_lineage(&lhs_shape);
        let rhs_lineage = record_lineage(&rhs_shape);

        lhs_lineage
            .into_iter()
            .find(|candidate| {
                rhs_lineage.iter().any(|other| {
                    nx_hir::same_declaration(
                        candidate.origin.as_ref(),
                        &candidate.name,
                        other.origin.as_ref(),
                        &other.name,
                    )
                })
            })
            .map(|ancestor| Type::named_at(ancestor.name, ancestor.origin))
    }

    /// Returns the shape of the record a nominal type denotes.
    ///
    /// A type that carries an origin is read from the declaration it names, so a foreign record
    /// resolves whether or not the asking module can spell it.
    fn record_shape_of(
        &self,
        named: &NamedType,
    ) -> Result<Option<nx_hir::EffectiveRecordShape>, nx_hir::RecordResolutionError> {
        match named.origin() {
            Some(origin) => nx_hir::effective_record_shape_at(self.module, origin),
            None => self.effective_record_shape(&named.name),
        }
    }

    /// The nearest component both lineages share, compared by declaration rather than by spelling.
    fn common_component_supertype(&self, lhs: &NamedType, rhs: &NamedType) -> Option<Type> {
        let lhs_contract = self.component_contract_of(lhs).ok().flatten()?;
        let rhs_contract = self.component_contract_of(rhs).ok().flatten()?;

        let lhs_lineage = component_lineage(&lhs_contract);
        let rhs_lineage = component_lineage(&rhs_contract);

        lhs_lineage
            .into_iter()
            .find(|candidate| {
                rhs_lineage.iter().any(|other| {
                    nx_hir::same_declaration(
                        candidate.origin.as_ref(),
                        &candidate.name,
                        other.origin.as_ref(),
                        &other.name,
                    )
                })
            })
            .map(|ancestor| Type::named_at(ancestor.name, ancestor.origin))
    }

    /// Returns the contract of the component a nominal type denotes.
    fn component_contract_of(
        &self,
        named: &NamedType,
    ) -> Result<Option<nx_hir::EffectiveComponentContract>, nx_hir::ComponentResolutionError> {
        match named.origin() {
            Some(origin) => nx_hir::effective_component_contract_at(self.module, origin),
            None => self.effective_component_contract(&named.name),
        }
    }
}

/// High-level type inference entry point.
pub struct TypeInference;

impl TypeInference {
    /// Infers types for all expressions in a module.
    pub fn infer_module(module: &PreparedModule) -> (TypeEnvironment, Vec<Diagnostic>) {
        let ctx = InferenceContext::new(module);

        // TODO: Process all items and their expressions
        // For now, just return empty results

        ctx.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::{TextSize, TextSpan};
    use nx_hir::{
        ast::BinOp, ast::Expr, ast::Literal, ast::TypeRef, Function, Item, LoweredModule, Name,
        Param, PreparedModule, SourceId, TypeAlias,
    };

    fn prepared(module: &LoweredModule) -> PreparedModule {
        PreparedModule::standalone("test.nx", module.clone())
    }

    #[test]
    fn test_infer_int_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Int(42)));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::int());
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_infer_string_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::String("hello".into())));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::string());
    }

    #[test]
    fn test_infer_bool_literal() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Boolean(true)));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::boolean());
    }

    #[test]
    fn test_element_supertype_requires_exact_case() {
        let module = LoweredModule::new(SourceId::new(0));
        let prepared = prepared(&module);
        let ctx = InferenceContext::new(&prepared);

        assert!(ctx.type_satisfies_expected(
            &Type::named(Name::new("div")),
            &Type::named(Name::new("Element"))
        ));
        assert!(!ctx.type_satisfies_expected(
            &Type::named(Name::new("div")),
            &Type::named(Name::new("element"))
        ));
    }

    #[test]
    fn test_converted_int_literals_records_only_the_literal_that_took_a_float_type() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        // One literal at a declared float return type, one with nothing expecting anything.
        let converted_body = module.alloc_expr(Expr::Literal(Literal::Int(42)));
        let untouched_body = module.alloc_expr(Expr::Literal(Literal::Int(7)));

        module.add_item(Item::Function(Function {
            name: Name::new("declared"),
            visibility: nx_hir::Visibility::Export,
            params: vec![],
            return_type: Some(TypeRef::name("float64")),
            body: converted_body,
            span,
        }));
        module.add_item(Item::Function(Function {
            name: Name::new("inferred"),
            visibility: nx_hir::Visibility::Export,
            params: vec![],
            return_type: None,
            body: untouched_body,
            span,
        }));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        for item in module.items() {
            if let Item::Function(func) = item {
                ctx.infer_function(func);
            }
        }

        assert_eq!(
            ctx.converted_int_literals().get(&converted_body),
            Some(&Primitive::Float64),
            "the literal at the declared float type should be recorded"
        );
        assert!(
            !ctx.converted_int_literals().contains_key(&untouched_body),
            "a literal with no float expectation should not be recorded"
        );
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_infer_function_parameter_reference() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("text")));
        let param = Param::new(Name::new("text"), TypeRef::name("string"), span);

        let function = Function {
            name: Name::new("Button"),
            visibility: nx_hir::Visibility::Export,
            params: vec![param],
            return_type: None,
            body,
            span,
        };

        module.add_item(Item::Function(function));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);

        if let Item::Function(func) = &module.items()[0] {
            ctx.infer_function(func);
        } else {
            panic!("Expected function item");
        }

        let (env, diagnostics) = ctx.finish();
        assert!(diagnostics.is_empty());
        let name = Name::new("text");
        assert!(env.lookup(&name).is_none());
    }

    #[test]
    fn test_infers_return_type_for_unannotated_function() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("value")));
        let function = Function {
            name: Name::new("identity"),
            visibility: nx_hir::Visibility::Export,
            params: vec![Param::new(Name::new("value"), TypeRef::name("int"), span)],
            return_type: None,
            body,
            span,
        };
        module.add_item(Item::Function(function));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        if let Item::Function(func) = &module.items()[0] {
            ctx.infer_function(func);
        }

        let (env, diagnostics) = ctx.finish();
        assert!(
            diagnostics.is_empty(),
            "Unexpected diagnostics: {:?}",
            diagnostics
        );

        let func_ty = env
            .lookup(&Name::new("identity"))
            .expect("Function binding should exist");
        match func_ty {
            Type::Function { params, ret } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], Type::int());
                assert_eq!(**ret, Type::int());
            }
            other => panic!("Expected function type, got {:?}", other),
        }
    }

    #[test]
    fn test_infer_paren_function_call() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        // add(a:int, b:int): int = a + b
        let add_lhs = module.alloc_expr(Expr::Ident(Name::new("a")));
        let add_rhs = module.alloc_expr(Expr::Ident(Name::new("b")));
        let add_body = module.alloc_expr(Expr::BinaryOp {
            lhs: add_lhs,
            op: BinOp::Add,
            rhs: add_rhs,
            span,
        });
        let add_fn = Function {
            name: Name::new("add"),
            visibility: nx_hir::Visibility::Export,
            params: vec![
                Param::new(Name::new("a"), TypeRef::name("int"), span),
                Param::new(Name::new("b"), TypeRef::name("int"), span),
            ],
            return_type: Some(TypeRef::name("int")),
            body: add_body,
            span,
        };
        module.add_item(Item::Function(add_fn));

        // double(value:int): int = add(value, value)
        let double_callee = module.alloc_expr(Expr::Ident(Name::new("add")));
        let double_arg1 = module.alloc_expr(Expr::Ident(Name::new("value")));
        let double_arg2 = module.alloc_expr(Expr::Ident(Name::new("value")));
        let double_body = module.alloc_expr(Expr::Call {
            func: double_callee,
            args: vec![double_arg1, double_arg2],
            span,
        });
        let double_fn = Function {
            name: Name::new("double"),
            visibility: nx_hir::Visibility::Export,
            params: vec![Param::new(Name::new("value"), TypeRef::name("int"), span)],
            return_type: Some(TypeRef::name("int")),
            body: double_body,
            span,
        };
        module.add_item(Item::Function(double_fn));

        // compute(n:int): int = double(add(n, 1))
        let inner_add_callee = module.alloc_expr(Expr::Ident(Name::new("add")));
        let inner_arg_n = module.alloc_expr(Expr::Ident(Name::new("n")));
        let inner_arg_one = module.alloc_expr(Expr::Literal(Literal::Int(1)));
        let inner_call = module.alloc_expr(Expr::Call {
            func: inner_add_callee,
            args: vec![inner_arg_n, inner_arg_one],
            span,
        });
        let outer_callee = module.alloc_expr(Expr::Ident(Name::new("double")));
        let compute_body = module.alloc_expr(Expr::Call {
            func: outer_callee,
            args: vec![inner_call],
            span,
        });
        let compute_fn = Function {
            name: Name::new("compute"),
            visibility: nx_hir::Visibility::Export,
            params: vec![Param::new(Name::new("n"), TypeRef::name("int"), span)],
            return_type: Some(TypeRef::name("int")),
            body: compute_body,
            span,
        };
        module.add_item(Item::Function(compute_fn));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        for item in module.items() {
            if let Item::Function(func) = item {
                ctx.infer_function(func);
            }
        }

        let (env, diagnostics) = ctx.finish();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got {:?}",
            diagnostics
        );

        let add_ty = env.lookup(&Name::new("add")).expect("add type binding");
        match add_ty {
            Type::Function { params, ret } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], Type::int());
                assert_eq!(params[1], Type::int());
                assert_eq!(**ret, Type::int());
            }
            _ => panic!("expected function type"),
        }
    }

    #[test]
    fn test_infer_union_case_access() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Direction"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![
                UnionCaseDef {
                    name: Name::new("north"),
                    fields: Vec::new(),
                    span,
                },
                UnionCaseDef {
                    name: Name::new("south"),
                    fields: Vec::new(),
                    span,
                },
            ],
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("north"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::UnionCase(case_ty) => {
                assert_eq!(case_ty.union.as_str(), "Direction");
                assert_eq!(case_ty.case.as_str(), "north");
            }
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(
            ctx.diagnostics().is_empty(),
            "Enum member access should not emit diagnostics"
        );
    }

    #[test]
    fn test_infer_invalid_union_case() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Status"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("active"),
                fields: Vec::new(),
                span,
            }],
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Status")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("pending_review"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        assert!(ty.is_error());
        assert_eq!(ctx.diagnostics().len(), 1);
    }

    #[test]
    fn test_union_case_access_via_alias() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Status"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("active"),
                fields: Vec::new(),
                span,
            }],
            span,
        };
        module.add_item(Item::Union(union_def));
        let alias = TypeAlias {
            name: Name::new("State"),
            visibility: nx_hir::Visibility::Export,
            ty: ast::TypeRef::name("Status"),
            span,
        };
        module.add_item(Item::TypeAlias(alias));

        let base = module.alloc_expr(Expr::Ident(Name::new("State")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("active"),
            span,
        });

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::UnionCase(case_ty) => assert_eq!(case_ty.union.as_str(), "Status"),
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_function_signature_uses_union_type() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let union_def = UnionDef {
            name: Name::new("Direction"),
            visibility: nx_hir::Visibility::Export,
            base: None,
            cases: vec![UnionCaseDef {
                name: Name::new("north"),
                fields: Vec::new(),
                span,
            }],
            span,
        };
        module.add_item(Item::Union(union_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let member = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("north"),
            span,
        });
        let func = Function {
            name: Name::new("north"),
            visibility: nx_hir::Visibility::Export,
            params: vec![],
            return_type: None,
            body: member,
            span,
        };
        module.add_item(Item::Function(func));

        let prepared = prepared(&module);
        let mut ctx = InferenceContext::new(&prepared);
        if let Item::Function(func) = &module.items()[1] {
            ctx.infer_function(func);
        }
        let (env, diagnostics) = ctx.finish();
        assert!(diagnostics.is_empty());

        let func_ty = env.lookup(&Name::new("north")).expect("function type");
        match func_ty {
            Type::Function { ret, .. } => match ret.as_ref() {
                Type::UnionCase(case_ty) => assert_eq!(case_ty.union.as_str(), "Direction"),
                other => panic!("Expected enum return type, got {:?}", other),
            },
            other => panic!("Expected function type, got {:?}", other),
        }
    }
}
