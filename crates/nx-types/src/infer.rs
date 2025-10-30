//! Type inference for expressions.

use crate::{Type, TypeEnvironment};
use nx_diagnostics::{Diagnostic, Label};
use nx_hir::{ast, ExprId, Module};

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
}

impl<'a> InferenceContext<'a> {
    /// Creates a new inference context for a module.
    pub fn new(module: &'a Module) -> Self {
        Self {
            module,
            env: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            next_var_id: 0,
        }
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
                let _base_ty = self.infer_expr(*base);
                // TODO: Implement struct/object types and member lookup
                self.error(
                    "not-implemented",
                    format!("Member access not yet implemented: .{}", member),
                    *span,
                );
                Type::Error
            }

            ast::Expr::Element { element, .. } => {
                let element_ref = self.module.element(*element);
                Type::named(element_ref.tag.clone())
            }

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
            ast::Expr::For { item: _, index: _, iterable, body, .. } => {
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
            self.env.bind(param.name.clone(), Type::Unknown);
            bound_names.push(param.name.clone());
        }

        let _ = self.infer_expr(func.body);

        for name in bound_names {
            self.env.remove(&name);
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
    use nx_hir::{ast::Expr, ast::Literal, ast::TypeRef, Function, Item, Name, Param, SourceId};

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
}
