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
            ast::Expr::Call { func, args, .. } => self.eval_call(module, ctx, *func, args),
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

    /// Evaluate a binary operation (T017, T036, T038)
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

        // Route to appropriate evaluation module based on operator
        match op {
            // Arithmetic operators
            ast::BinOp::Add
            | ast::BinOp::Sub
            | ast::BinOp::Mul
            | ast::BinOp::Div
            | ast::BinOp::Mod
            | ast::BinOp::Concat => {
                crate::eval::arithmetic::eval_arithmetic_op(lhs_val, op, rhs_val)
            }

            // Comparison operators (T036)
            ast::BinOp::Eq
            | ast::BinOp::Ne
            | ast::BinOp::Lt
            | ast::BinOp::Le
            | ast::BinOp::Gt
            | ast::BinOp::Ge => crate::eval::logical::eval_comparison_op(lhs_val, op, rhs_val),

            // Logical operators (T038)
            ast::BinOp::And | ast::BinOp::Or => {
                crate::eval::logical::eval_logical_op(lhs_val, op, rhs_val)
            }
        }
    }

    /// Evaluate a unary operation (T038)
    fn eval_unary_op(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        op: ast::UnOp,
        expr: ExprId,
    ) -> Result<Value, RuntimeError> {
        let operand = self.eval_expr(module, ctx, expr)?;

        match op {
            ast::UnOp::Not => crate::eval::logical::eval_logical_unary(op, operand),
            ast::UnOp::Neg => {
                // Arithmetic negation
                match operand {
                    Value::Int(n) => Ok(Value::Int(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "number".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "negation".to_string(),
                    })),
                }
            }
        }
    }

    /// Evaluate an if expression (T037)
    fn eval_if(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    ) -> Result<Value, RuntimeError> {
        // Evaluate condition
        let condition_value = self.eval_expr(module, ctx, condition)?;

        // Condition must be a boolean
        let condition_bool = match condition_value {
            Value::Boolean(b) => b,
            v => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "boolean".to_string(),
                    actual: v.type_name().to_string(),
                    operation: "if condition".to_string(),
                }))
            }
        };

        // Execute appropriate branch
        if condition_bool {
            self.eval_expr(module, ctx, then_branch)
        } else if let Some(else_expr) = else_branch {
            self.eval_expr(module, ctx, else_expr)
        } else {
            Ok(Value::Null)
        }
    }

    /// Evaluate a function call expression (T053)
    fn eval_call(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        func_expr: ExprId,
        args: &[ExprId],
    ) -> Result<Value, RuntimeError> {
        // Evaluate the function expression (should be an identifier)
        let func_name = match module.expr(func_expr) {
            ast::Expr::Ident(name) => name.as_str(),
            _ => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "function name".to_string(),
                    actual: "complex expression".to_string(),
                    operation: "function call".to_string(),
                }))
            }
        };

        // Find the function in the module
        let function = self.find_function(module, func_name)?;

        // Validate parameter count
        if function.params.len() != args.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: args.len(),
                    function: SmolStr::new(func_name),
                },
            ));
        }

        // Evaluate arguments
        let mut arg_values = Vec::with_capacity(args.len());
        for arg_expr in args {
            arg_values.push(self.eval_expr(module, ctx, *arg_expr)?);
        }

        // Create call frame (T054)
        let call_frame = crate::error::CallFrame::new(SmolStr::new(func_name), None);
        ctx.push_call_frame(call_frame)?; // This checks recursion depth

        // Create new scope for function parameters
        ctx.push_scope();

        // Bind parameters to argument values
        for (param, arg) in function.params.iter().zip(arg_values.iter()) {
            ctx.define_variable(SmolStr::new(param.name.as_str()), arg.clone());
        }

        // Execute the function body
        let result = self.eval_expr(module, ctx, function.body);

        // Clean up scope and call frame
        ctx.pop_scope();
        ctx.pop_call_frame();

        result
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
