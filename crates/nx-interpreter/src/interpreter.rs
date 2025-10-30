//! Core interpreter implementation for executing NX HIR.

use crate::context::{ExecutionContext, ResourceLimits};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::{ast, ExprId, Function, Module, Name};
use smol_str::SmolStr;

/// Tree-walking interpreter for NX HIR
#[derive(Debug)]
pub struct Interpreter {
    // The interpreter is stateless; all state is in ExecutionContext
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Self {}
    }

    /// Execute a function by name with the given arguments
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    ///
    /// # Returns
    /// The result value or a runtime error
    pub fn execute_function(
        &self,
        module: &Module,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.execute_function_with_limits(module, function_name, args, ResourceLimits::default())
    }

    /// Execute a function with custom resource limits
    pub fn execute_function_with_limits(
        &self,
        module: &Module,
        function_name: &str,
        args: Vec<Value>,
        limits: ResourceLimits,
    ) -> Result<Value, RuntimeError> {
        // T011: Find function in module
        let function = self.find_function(module, function_name)?;

        // T011: Validate parameter count
        if function.params.len() != args.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: args.len(),
                    function: SmolStr::new(function_name),
                },
            ));
        }

        // T011: Create execution context
        let mut ctx = ExecutionContext::with_limits(limits);

        // T012: Bind parameters to argument values
        for (param, arg) in function.params.iter().zip(args.iter()) {
            ctx.define_variable(SmolStr::new(param.name.as_str()), arg.clone());
        }

        // Execute the function body
        self.eval_expr(module, &mut ctx, function.body)
    }

    /// Find a function by name in the module
    fn find_function<'a>(
        &self,
        module: &'a Module,
        name: &str,
    ) -> Result<&'a Function, RuntimeError> {
        match module.find_item(name) {
            Some(nx_hir::Item::Function(func)) => Ok(func),
            _ => Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(name),
            })),
        }
    }

    /// Evaluate an expression (T013 - skeleton)
    fn eval_expr(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        expr_id: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Check operation limit
        ctx.check_operation_limit()?;

        let expr = module.expr(expr_id);
        match expr {
            ast::Expr::Literal(lit) => self.eval_literal(lit),
            ast::Expr::Ident(name) => self.eval_ident(ctx, name),
            ast::Expr::Block { stmts, expr, .. } => {
                self.eval_block(module, ctx, stmts, expr.as_ref())
            }
            ast::Expr::BinaryOp { lhs, op, rhs, .. } => {
                self.eval_binary_op(module, ctx, *lhs, *op, *rhs)
            }
            ast::Expr::UnaryOp { op, expr, .. } => self.eval_unary_op(module, ctx, *op, *expr),
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.eval_if(module, ctx, *condition, *then_branch, *else_branch),
            _ => {
                // Other expression types not yet implemented
                Ok(Value::Null)
            }
        }
    }

    /// Evaluate a literal expression (T015 - placeholder)
    fn eval_literal(&self, lit: &ast::Literal) -> Result<Value, RuntimeError> {
        let value = match lit {
            ast::Literal::Int(n) => Value::Int(*n),
            ast::Literal::Float(f) => Value::Float(f.0),
            ast::Literal::String(s) => Value::String(s.clone()),
            ast::Literal::Bool(b) => Value::Boolean(*b),
            ast::Literal::Null => Value::Null,
        };
        Ok(value)
    }

    /// Evaluate an identifier (T016 - placeholder)
    fn eval_ident(&self, ctx: &ExecutionContext, name: &Name) -> Result<Value, RuntimeError> {
        ctx.lookup_variable(name.as_str())
    }

    /// Evaluate a block expression (T014 - placeholder)
    fn eval_block(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        stmts: &[ast::Stmt],
        final_expr: Option<&ExprId>,
    ) -> Result<Value, RuntimeError> {
        ctx.push_scope();

        // Execute statements
        for stmt in stmts {
            self.eval_stmt(module, ctx, stmt)?;
        }

        // Evaluate final expression or return null
        let result = if let Some(expr_id) = final_expr {
            self.eval_expr(module, ctx, *expr_id)?
        } else {
            Value::Null
        };

        ctx.pop_scope();
        Ok(result)
    }

    /// Evaluate a statement (T014 - placeholder)
    fn eval_stmt(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        stmt: &ast::Stmt,
    ) -> Result<(), RuntimeError> {
        match stmt {
            ast::Stmt::Let { name, init, .. } => {
                let value = self.eval_expr(module, ctx, *init)?;
                ctx.define_variable(SmolStr::new(name.as_str()), value);
                Ok(())
            }
            ast::Stmt::Expr(expr_id, _) => {
                self.eval_expr(module, ctx, *expr_id)?;
                Ok(())
            }
        }
    }

    /// Evaluate a binary operation (T017)
    fn eval_binary_op(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        lhs: ExprId,
        op: ast::BinOp,
        rhs: ExprId,
    ) -> Result<Value, RuntimeError> {
        let lhs_val = self.eval_expr(module, ctx, lhs)?;
        let rhs_val = self.eval_expr(module, ctx, rhs)?;
        
        // Delegate to arithmetic module
        crate::eval::arithmetic::eval_arithmetic_op(lhs_val, op, rhs_val)
    }

    /// Evaluate a unary operation (placeholder)
    fn eval_unary_op(
        &self,
        _module: &Module,
        _ctx: &mut ExecutionContext,
        _op: ast::UnOp,
        _expr: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Will be implemented later
        Ok(Value::Null)
    }

    /// Evaluate an if expression (placeholder)
    fn eval_if(
        &self,
        _module: &Module,
        _ctx: &mut ExecutionContext,
        _condition: ExprId,
        _then_branch: ExprId,
        _else_branch: Option<ExprId>,
    ) -> Result<Value, RuntimeError> {
        // Will be implemented in Phase 5
        Ok(Value::Null)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_hir::SourceId;

    #[test]
    fn test_interpreter_creation() {
        let _interpreter = Interpreter::new();
        // Successfully created
    }

    #[test]
    fn test_function_not_found() {
        let interpreter = Interpreter::new();
        let module = Module::new(SourceId::new(0));
        let result = interpreter.execute_function(&module, "nonexistent", vec![]);
        assert!(result.is_err());
    }
}
