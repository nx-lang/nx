//! Type inference for expressions.

use crate::{ty::EnumType, Type, TypeEnvironment};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::{ast, ExprId, Module, Name};
use rustc_hash::{FxHashMap, FxHashSet};

struct TypeAliasInfo {
    target: ast::TypeRef,
    span: TextSpan,
}

/// Type inference context.
///
/// Manages type inference state and provides methods for inferring types
/// of expressions within a module.
pub struct InferenceContext<'a> {
    /// The module being type-checked
    module: &'a Module,
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
    /// Registered enum definitions
    enum_defs: FxHashMap<Name, EnumType>,
}

impl<'a> InferenceContext<'a> {
    /// Creates a new inference context for a module.
    pub fn new(module: &'a Module) -> Self {
        let mut ctx = Self {
            module,
            env: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            next_var_id: 0,
            function_return_placeholders: FxHashMap::default(),
            type_aliases: FxHashMap::default(),
            enum_defs: FxHashMap::default(),
        };
        ctx.register_type_definitions();
        ctx.register_function_signatures();
        ctx
    }

    /// Generates a fresh type variable for inference.
    fn fresh_var(&mut self) -> Type {
        let id = self.next_var_id;
        self.next_var_id += 1;
        Type::var(id)
    }

    /// Infers the type of an expression.
    pub fn infer_expr(&mut self, expr_id: ExprId) -> Type {
        let expr = self.module.expr(expr_id);

        let ty = match expr {
            // Literals have known types
            ast::Expr::Literal(lit) => self.infer_literal(lit),

            // Identifiers look up in environment
            ast::Expr::Ident(name) => {
                if let Some(ty) = self.env.lookup(name) {
                    ty.clone()
                } else {
                    // Undefined identifier - record error
                    self.error(
                        "undefined-identifier",
                        format!("Undefined identifier '{}'", name),
                        expr.span(),
                    );
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

                self.infer_call(&func_ty, &arg_tys, *span)
            }

            // If expressions
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(*condition);

                // Condition must be bool
                if !cond_ty.is_compatible_with(&Type::bool()) && !cond_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("If condition must be bool, found {}", cond_ty),
                        *span,
                    );
                }

                let then_ty = self.infer_expr(*then_branch);

                if let Some(else_id) = else_branch {
                    let else_ty = self.infer_expr(*else_id);

                    // Both branches must have compatible types
                    if !then_ty.is_compatible_with(&else_ty)
                        && !then_ty.is_error()
                        && !else_ty.is_error()
                    {
                        self.error(
                            "type-mismatch",
                            format!(
                                "If branches have incompatible types: {} vs {}",
                                then_ty, else_ty
                            ),
                            *span,
                        );
                        Type::Error
                    } else {
                        then_ty
                    }
                } else {
                    // No else branch - type is void
                    Type::void()
                }
            }

            // Arrays
            ast::Expr::Array { elements, .. } => {
                if elements.is_empty() {
                    // Empty array - need more context to infer element type
                    Type::array(self.fresh_var())
                } else {
                    // Infer element types
                    let elem_tys: Vec<_> = elements.iter().map(|e| self.infer_expr(*e)).collect();

                    // All elements should have the same type
                    let first_ty = &elem_tys[0];
                    for (i, ty) in elem_tys.iter().enumerate().skip(1) {
                        if !ty.is_compatible_with(first_ty)
                            && !ty.is_error()
                            && !first_ty.is_error()
                        {
                            self.error(
                                "type-mismatch",
                                format!(
                                    "Array element {} has type {}, expected {}",
                                    i, ty, first_ty
                                ),
                                expr.span(),
                            );
                        }
                    }

                    Type::array(first_ty.clone())
                }
            }

            // Index operation
            ast::Expr::Index { base, index, span } => {
                let base_ty = self.infer_expr(*base);
                let index_ty = self.infer_expr(*index);

                // Index must be int
                if !index_ty.is_compatible_with(&Type::int()) && !index_ty.is_error() {
                    self.error(
                        "type-mismatch",
                        format!("Array index must be int, found {}", index_ty),
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
                if let Some(enum_info) = self.enum_info_for_expr(*base) {
                    if enum_info.members.iter().any(|m| m == member) {
                        Type::Enum(enum_info.clone())
                    } else {
                        self.error(
                            "undefined-enum-member",
                            format!("Enum '{}' has no member named '{}'", enum_info.name, member),
                            *span,
                        );
                        Type::Error
                    }
                } else {
                    let _base_ty = self.infer_expr(*base);
                    self.error(
                        "not-implemented",
                        format!("Member access not yet implemented: .{}", member),
                        *span,
                    );
                    Type::Error
                }
            }

            ast::Expr::Element { element, .. } => {
                let element_ref = self.module.element(*element);
                Type::named(element_ref.tag.clone())
            }

            ast::Expr::RecordLiteral { record, .. } => Type::named(record.clone()),

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
                item: _,
                index: _,
                iterable,
                body,
                ..
            } => {
                // Infer iterable type (should be array)
                let iterable_ty = self.infer_expr(*iterable);

                // TODO: Add item and index to environment with proper types
                // For now, just infer the body type
                let _body_ty = self.infer_expr(*body);

                // For loops return arrays of the body type
                // For simplicity, return the iterable type for now
                iterable_ty
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
            self.type_from_type_ref(ty)
        } else {
            body_ty.clone()
        };

        self.bind_function_signature(func, return_ty.clone());
        if func.return_type.is_none() {
            self.function_return_placeholders.remove(&func.name);
        }
    }

    /// Infers the type of a literal.
    fn infer_literal(&mut self, lit: &ast::Literal) -> Type {
        match lit {
            ast::Literal::String(_) => Type::string(),
            ast::Literal::Int(_) => Type::int(),
            ast::Literal::Float(_) => Type::float(),
            ast::Literal::Bool(_) => Type::bool(),
            ast::Literal::Null => Type::nullable(self.fresh_var()),
        }
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
            // Arithmetic: int × int → int, float × float → float
            Add | Sub | Mul | Div | Mod => {
                if lhs == &Type::int() && rhs == &Type::int() {
                    Type::int()
                } else if lhs == &Type::float() && rhs == &Type::float() {
                    Type::float()
                } else if lhs == &Type::string() && rhs == &Type::string() && op == Add {
                    // String concatenation
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
                if lhs.is_compatible_with(rhs) {
                    Type::bool()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Cannot compare types {} and {}", lhs, rhs),
                        span,
                    );
                    Type::Error
                }
            }

            // Logical: bool × bool → bool
            And | Or => {
                if lhs == &Type::bool() && rhs == &Type::bool() {
                    Type::bool()
                } else {
                    self.error(
                        "type-mismatch",
                        format!(
                            "Logical operator {:?} requires bool operands, found {} and {}",
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
                if operand == &Type::int() || operand == &Type::float() {
                    operand.clone()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Negation requires int or float, found {}", operand),
                        span,
                    );
                    Type::Error
                }
            }
            ast::UnOp::Not => {
                if operand == &Type::bool() {
                    Type::bool()
                } else {
                    self.error(
                        "type-mismatch",
                        format!("Logical NOT requires bool, found {}", operand),
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

                // Check argument types
                for (i, (param_ty, arg_ty)) in params.iter().zip(arg_tys.iter()).enumerate() {
                    if !arg_ty.is_compatible_with(param_ty) && !arg_ty.is_error() {
                        self.error(
                            "type-mismatch",
                            format!("Argument {} has type {}, expected {}", i, arg_ty, param_ty),
                            span,
                        );
                    }
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

    /// Records a type error.
    fn error(&mut self, code: &str, message: String, span: nx_diagnostics::TextSpan) {
        let diag = Diagnostic::error(code)
            .with_message(message)
            .with_label(Label::primary("", span))
            .build();
        self.diagnostics.push(diag);
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
        for item in self.module.items() {
            match item {
                nx_hir::Item::TypeAlias(alias) => {
                    self.type_aliases.insert(
                        alias.name.clone(),
                        TypeAliasInfo {
                            target: alias.ty.clone(),
                            span: alias.span,
                        },
                    );
                }
                nx_hir::Item::Enum(enum_def) => {
                    let members = enum_def
                        .members
                        .iter()
                        .map(|member| member.name.clone())
                        .collect();
                    self.enum_defs.insert(
                        enum_def.name.clone(),
                        EnumType::new(enum_def.name.clone(), members),
                    );
                }
                nx_hir::Item::Record(record_def) => {
                    for prop in &record_def.properties {
                        if let Some(default_expr) = prop.default {
                            let expected = self.type_from_type_ref(&prop.ty);
                            let actual = self.infer_expr(default_expr);

                            if !actual.is_compatible_with(&expected)
                                && !actual.is_error()
                                && !expected.is_error()
                            {
                                self.error(
                                    "record-default-type-mismatch",
                                    format!(
                                        "Default value for record property '{}' expects {}, found {}",
                                        prop.name, expected, actual
                                    ),
                                    prop.span,
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn register_function_signatures(&mut self) {
        for item in self.module.items() {
            if let nx_hir::Item::Function(func) = item {
                let return_type = if let Some(ty) = func.return_type.as_ref() {
                    self.type_from_type_ref(ty)
                } else {
                    let placeholder = self.fresh_var();
                    self.function_return_placeholders
                        .insert(func.name.clone(), placeholder.clone());
                    placeholder
                };

                self.bind_function_signature(func, return_type);
            }
        }
    }

    fn enum_info_for_expr(&self, expr_id: ExprId) -> Option<&EnumType> {
        match self.module.expr(expr_id) {
            ast::Expr::Ident(name) => {
                let mut seen = FxHashSet::default();
                self.enum_info_from_name(name, &mut seen)
            }
            _ => None,
        }
    }

    fn enum_info_from_name<'info>(
        &'info self,
        name: &Name,
        seen: &mut FxHashSet<Name>,
    ) -> Option<&'info EnumType> {
        if let Some(info) = self.enum_defs.get(name) {
            return Some(info);
        }

        if let Some(alias) = self.type_aliases.get(name) {
            if !seen.insert(name.clone()) {
                return None;
            }
            if let ast::TypeRef::Name(target) = &alias.target {
                let target_info = self.enum_info_from_name(target, seen);
                seen.remove(name);
                return target_info;
            }
            seen.remove(name);
        }

        None
    }

    fn type_from_type_ref(&mut self, type_ref: &ast::TypeRef) -> Type {
        let mut seen = FxHashSet::default();
        self.resolve_type_ref(type_ref, &mut seen)
    }

    fn resolve_type_ref(&mut self, type_ref: &ast::TypeRef, seen: &mut FxHashSet<Name>) -> Type {
        match type_ref {
            ast::TypeRef::Name(name) => self.resolve_named_type(name, seen),
            ast::TypeRef::Array(inner) => Type::array(self.resolve_type_ref(inner, seen)),
            ast::TypeRef::Nullable(inner) => Type::nullable(self.resolve_type_ref(inner, seen)),
            ast::TypeRef::Function {
                params,
                return_type,
            } => {
                let param_types = params
                    .iter()
                    .map(|p| self.resolve_type_ref(p, seen))
                    .collect();
                let ret = self.resolve_type_ref(return_type, seen);
                Type::function(param_types, ret)
            }
        }
    }

    fn resolve_named_type(&mut self, name: &Name, seen: &mut FxHashSet<Name>) -> Type {
        let lower = name.as_str().to_ascii_lowercase();
        match lower.as_str() {
            "string" => Type::string(),
            "int" | "long" => Type::int(),
            "float" | "double" => Type::float(),
            "boolean" | "bool" => Type::bool(),
            "void" => Type::void(),
            _ => {
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
                    let ty = self.resolve_type_ref(&target, seen);
                    seen.remove(name);
                    return ty;
                }

                if let Some(enum_ty) = self.enum_defs.get(name) {
                    return Type::Enum(enum_ty.clone());
                }

                Type::named(name.clone())
            }
        }
    }

    fn function_param_types(&mut self, func: &nx_hir::Function) -> Vec<Type> {
        func.params
            .iter()
            .map(|p| self.type_from_type_ref(&p.ty))
            .collect()
    }

    fn bind_function_signature(&mut self, func: &nx_hir::Function, return_type: Type) {
        let param_types = self.function_param_types(func);
        self.env
            .bind(func.name.clone(), Type::function(param_types, return_type));
    }
}

/// High-level type inference entry point.
pub struct TypeInference;

impl TypeInference {
    /// Infers types for all expressions in a module.
    pub fn infer_module(module: &Module) -> (TypeEnvironment, Vec<Diagnostic>) {
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
        ast::BinOp, ast::Expr, ast::Literal, ast::TypeRef, EnumDef, EnumMember, Function, Item,
        Name, Param, SourceId, TypeAlias,
    };

    #[test]
    fn test_infer_int_literal() {
        let mut module = Module::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Int(42)));

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::int());
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_infer_string_literal() {
        let mut module = Module::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::String("hello".into())));

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::string());
    }

    #[test]
    fn test_infer_bool_literal() {
        let mut module = Module::new(SourceId::new(0));
        let expr_id = module.alloc_expr(Expr::Literal(Literal::Bool(true)));

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        assert_eq!(ty, Type::bool());
    }

    #[test]
    fn test_infer_function_parameter_reference() {
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("text")));
        let param = Param::new(Name::new("text"), TypeRef::name("string"), span);

        let function = Function {
            name: Name::new("Button"),
            params: vec![param],
            return_type: None,
            body,
            span,
        };

        module.add_item(Item::Function(function));

        let mut ctx = InferenceContext::new(&module);

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
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));

        let body = module.alloc_expr(Expr::Ident(Name::new("value")));
        let function = Function {
            name: Name::new("identity"),
            params: vec![Param::new(Name::new("value"), TypeRef::name("int"), span)],
            return_type: None,
            body,
            span,
        };
        module.add_item(Item::Function(function));

        let mut ctx = InferenceContext::new(&module);
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
        let mut module = Module::new(SourceId::new(0));
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
            params: vec![Param::new(Name::new("n"), TypeRef::name("int"), span)],
            return_type: Some(TypeRef::name("int")),
            body: compute_body,
            span,
        };
        module.add_item(Item::Function(compute_fn));

        let mut ctx = InferenceContext::new(&module);
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
    fn test_infer_enum_member_access() {
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let enum_def = EnumDef {
            name: Name::new("Direction"),
            members: vec![
                EnumMember {
                    name: Name::new("North"),
                    span,
                },
                EnumMember {
                    name: Name::new("South"),
                    span,
                },
            ],
            span,
        };
        module.add_item(Item::Enum(enum_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("North"),
            span,
        });

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::Enum(enum_ty) => assert_eq!(enum_ty.name.as_str(), "Direction"),
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(
            ctx.diagnostics().is_empty(),
            "Enum member access should not emit diagnostics"
        );
    }

    #[test]
    fn test_infer_enum_invalid_member() {
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let enum_def = EnumDef {
            name: Name::new("Status"),
            members: vec![EnumMember {
                name: Name::new("Active"),
                span,
            }],
            span,
        };
        module.add_item(Item::Enum(enum_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Status")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("Pending"),
            span,
        });

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        assert!(ty.is_error());
        assert_eq!(ctx.diagnostics().len(), 1);
    }

    #[test]
    fn test_enum_member_access_via_alias() {
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let enum_def = EnumDef {
            name: Name::new("Status"),
            members: vec![EnumMember {
                name: Name::new("Active"),
                span,
            }],
            span,
        };
        module.add_item(Item::Enum(enum_def));
        let alias = TypeAlias {
            name: Name::new("State"),
            ty: ast::TypeRef::name("Status"),
            span,
        };
        module.add_item(Item::TypeAlias(alias));

        let base = module.alloc_expr(Expr::Ident(Name::new("State")));
        let expr_id = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("Active"),
            span,
        });

        let mut ctx = InferenceContext::new(&module);
        let ty = ctx.infer_expr(expr_id);

        match ty {
            Type::Enum(enum_ty) => assert_eq!(enum_ty.name.as_str(), "Status"),
            other => panic!("Expected enum type, got {:?}", other),
        }
        assert!(ctx.diagnostics().is_empty());
    }

    #[test]
    fn test_function_signature_uses_enum_type() {
        let mut module = Module::new(SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(0));
        let enum_def = EnumDef {
            name: Name::new("Direction"),
            members: vec![EnumMember {
                name: Name::new("North"),
                span,
            }],
            span,
        };
        module.add_item(Item::Enum(enum_def));

        let base = module.alloc_expr(Expr::Ident(Name::new("Direction")));
        let member = module.alloc_expr(Expr::Member {
            base,
            member: Name::new("North"),
            span,
        });
        let func = Function {
            name: Name::new("north"),
            params: vec![],
            return_type: None,
            body: member,
            span,
        };
        module.add_item(Item::Function(func));

        let mut ctx = InferenceContext::new(&module);
        if let Item::Function(func) = &module.items()[1] {
            ctx.infer_function(func);
        }
        let (env, diagnostics) = ctx.finish();
        assert!(diagnostics.is_empty());

        let func_ty = env.lookup(&Name::new("north")).expect("function type");
        match func_ty {
            Type::Function { ret, .. } => match ret.as_ref() {
                Type::Enum(enum_ty) => assert_eq!(enum_ty.name.as_str(), "Direction"),
                other => panic!("Expected enum return type, got {:?}", other),
            },
            other => panic!("Expected function type, got {:?}", other),
        }
    }
}
