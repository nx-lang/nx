//! Core interpreter implementation for executing NX HIR.

use crate::context::{ExecutionContext, ResourceLimits};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::{ast, ElementId, ExprId, Function, Item, Module, Name};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
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
    /// Uses default resource limits (recursion: 1000, operations: 1M).
    /// For custom limits, use [`execute_function_with_limits`](Self::execute_function_with_limits).
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function (must match parameter count)
    ///
    /// # Returns
    /// The result value or a runtime error
    ///
    /// # Errors
    /// Returns `RuntimeError` if:
    /// - Function not found in module
    /// - Parameter count mismatch
    /// - Runtime error occurs during execution (division by zero, type mismatch, etc.)
    /// - Resource limits exceeded (recursion depth or operation count)
    ///
    /// # Example
    /// ```ignore
    /// use nx_interpreter::{Interpreter, Value};
    ///
    /// let interpreter = Interpreter::new();
    /// let result = interpreter.execute_function(
    ///     &module,
    ///     "add",
    ///     vec![Value::Int(5), Value::Int(3)],
    /// )?;
    /// assert_eq!(result, Value::Int(8));
    /// ```
    pub fn execute_function(
        &self,
        module: &Module,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        self.execute_function_with_limits(module, function_name, args, ResourceLimits::default())
    }

    /// Execute a function with custom resource limits
    ///
    /// # Arguments
    /// * `module` - The HIR module containing the function
    /// * `function_name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    /// * `limits` - Custom resource limits (recursion depth, operation count)
    ///
    /// # Returns
    /// The result value or a runtime error
    ///
    /// # Errors
    /// Returns `RuntimeError` if:
    /// - Function not found in module
    /// - Parameter count mismatch
    /// - Runtime error occurs during execution
    /// - Resource limits exceeded
    ///
    /// # Example
    /// ```ignore
    /// let limits = ResourceLimits {
    ///     recursion_limit: 100,
    ///     operation_limit: 10_000,
    /// };
    /// let result = interpreter.execute_function_with_limits(
    ///     &module,
    ///     "factorial",
    ///     vec![Value::Int(5)],
    ///     limits,
    /// )?;
    /// ```
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
            ast::Expr::Let {
                name, value, body, ..
            } => self.eval_let(module, ctx, name, *value, *body),
            ast::Expr::Call { func, args, .. } => self.eval_call(module, ctx, *func, args),
            ast::Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => self.eval_for(module, ctx, item, index.as_ref(), *iterable, *body),
            ast::Expr::Array { elements, .. } => {
                let mut values = Vec::with_capacity(elements.len());
                for elem_expr in elements {
                    values.push(self.eval_expr(module, ctx, *elem_expr)?);
                }
                Ok(Value::Array(values))
            }
            ast::Expr::Element { element, .. } => self.eval_element_expr(module, ctx, *element),
            ast::Expr::RecordLiteral {
                record, properties, ..
            } => self.eval_record_literal(module, ctx, record, properties),
            ast::Expr::Member { base, member, .. } => self.eval_member(module, ctx, *base, member),
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
            ast::Literal::Boolean(b) => Value::Boolean(*b),
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
        // Handle short-circuit operators specially - don't evaluate rhs eagerly
        match op {
            ast::BinOp::And => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                match lhs_val {
                    Value::Boolean(false) => Ok(Value::Boolean(false)),
                    Value::Boolean(true) => {
                        let rhs_val = self.eval_expr(module, ctx, rhs)?;
                        match rhs_val {
                            Value::Boolean(b) => Ok(Value::Boolean(b)),
                            v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                                expected: "boolean".to_string(),
                                actual: v.type_name().to_string(),
                                operation: "logical and".to_string(),
                            })),
                        }
                    }
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "boolean".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "logical and".to_string(),
                    })),
                }
            }
            ast::BinOp::Or => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                match lhs_val {
                    Value::Boolean(true) => Ok(Value::Boolean(true)),
                    Value::Boolean(false) => {
                        let rhs_val = self.eval_expr(module, ctx, rhs)?;
                        match rhs_val {
                            Value::Boolean(b) => Ok(Value::Boolean(b)),
                            v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                                expected: "boolean".to_string(),
                                actual: v.type_name().to_string(),
                                operation: "logical or".to_string(),
                            })),
                        }
                    }
                    v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                        expected: "boolean".to_string(),
                        actual: v.type_name().to_string(),
                        operation: "logical or".to_string(),
                    })),
                }
            }
            // All other operators evaluate both sides eagerly
            _ => {
                let lhs_val = self.eval_expr(module, ctx, lhs)?;
                let rhs_val = self.eval_expr(module, ctx, rhs)?;

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
                    | ast::BinOp::Ge => {
                        crate::eval::logical::eval_comparison_op(lhs_val, op, rhs_val)
                    }

                    // And/Or already handled above
                    ast::BinOp::And | ast::BinOp::Or => unreachable!(),
                }
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

    /// Evaluate a let binding expression
    ///
    /// Evaluates the value expression once, binds it to the name in a new scope,
    /// then evaluates the body expression with that binding.
    fn eval_let(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        name: &Name,
        value: ExprId,
        body: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Evaluate the value expression once
        let val = self.eval_expr(module, ctx, value)?;

        // Create a new scope with the binding
        ctx.push_scope();
        ctx.define_variable(name.as_str().into(), val);

        // Evaluate the body
        let result = self.eval_expr(module, ctx, body);

        // Pop the scope
        ctx.pop_scope();

        result
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

        let mut arg_values = Vec::with_capacity(args.len());
        for arg_expr in args {
            arg_values.push(self.eval_expr(module, ctx, *arg_expr)?);
        }

        match module.find_item(func_name) {
            Some(Item::Function(function)) => {
                self.eval_function_call(module, ctx, func_name, function, arg_values)
            }
            Some(Item::Record(record_def)) => {
                self.eval_record_constructor_call(module, ctx, func_name, record_def, arg_values)
            }
            Some(Item::TypeAlias(_)) => {
                if let Some(record_def) = self.resolve_record_definition(module, func_name) {
                    self.eval_record_constructor_call(module, ctx, func_name, record_def, arg_values)
                } else {
                    Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                        name: SmolStr::new(func_name),
                    }))
                }
            }
            _ => Err(RuntimeError::new(RuntimeErrorKind::FunctionNotFound {
                name: SmolStr::new(func_name),
            })),
        }
    }

    fn eval_function_call(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        func_name: &str,
        function: &Function,
        arg_values: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if function.params.len() != arg_values.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: arg_values.len(),
                    function: SmolStr::new(func_name),
                },
            ));
        }

        let call_frame = crate::error::CallFrame::new(SmolStr::new(func_name), None);
        ctx.push_call_frame(call_frame)?;

        ctx.push_scope();
        for (param, arg) in function.params.iter().zip(arg_values.iter()) {
            ctx.define_variable(SmolStr::new(param.name.as_str()), arg.clone());
        }

        let result = self.eval_expr(module, ctx, function.body);

        ctx.pop_scope();
        ctx.pop_call_frame();

        result
    }

    fn eval_record_constructor_call(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        func_name: &str,
        record_def: &nx_hir::RecordDef,
        arg_values: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if arg_values.len() > record_def.properties.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ParameterCountMismatch {
                    expected: record_def.properties.len(),
                    actual: arg_values.len(),
                    function: SmolStr::new(func_name),
                },
            ));
        }

        let mut overrides = FxHashMap::default();
        for (field, value) in record_def.properties.iter().zip(arg_values.into_iter()) {
            overrides.insert(SmolStr::new(field.name.as_str()), value);
        }

        self.build_record_value(module, ctx, record_def.name.as_str(), overrides)
    }

    fn eval_element_expr(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        element_id: ElementId,
    ) -> Result<Value, RuntimeError> {
        let element = module.element(element_id);
        let tag_name = element.tag.as_str();

        let mut fields = FxHashMap::default();
        for prop in &element.properties {
            let value = self.eval_expr(module, ctx, prop.value)?;
            fields.insert(SmolStr::new(prop.key.as_str()), value);
        }

        let mut child_values = Vec::with_capacity(element.children.len());
        for child_id in &element.children {
            child_values.push(self.eval_element_expr(module, ctx, *child_id)?);
        }

        if let Some(Item::Function(function)) = module.find_item(tag_name) {
            let has_children_param = function
                .params
                .iter()
                .any(|p| p.name.as_str() == "children");

            if fields.contains_key("children") && !child_values.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "children passed either as a named property or as element body"
                        .to_string(),
                    actual: "both a children property and element body".to_string(),
                    operation: "element function call".to_string(),
                }));
            }

            if !has_children_param && !child_values.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "function without 'children' parameter".to_string(),
                    actual: "element body content".to_string(),
                    operation: "element function call".to_string(),
                }));
            }

            if has_children_param && !fields.contains_key("children") {
                fields.insert(SmolStr::new("children"), Value::Array(child_values));
            }

            if fields.len() != function.params.len() {
                return Err(RuntimeError::new(RuntimeErrorKind::ParameterCountMismatch {
                    expected: function.params.len(),
                    actual: fields.len(),
                    function: SmolStr::new(tag_name),
                }));
            }

            let mut arg_values = Vec::with_capacity(function.params.len());
            for param in &function.params {
                match fields.remove(param.name.as_str()) {
                    Some(value) => {
                        arg_values.push(value);
                    }
                    None => {
                        return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                            expected: format!("argument '{}'", param.name.as_str()),
                            actual: "missing".to_string(),
                            operation: "element function call".to_string(),
                        }))
                    }
                }
            }

            if !fields.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "known parameters".to_string(),
                    actual: "unknown property".to_string(),
                    operation: "element function call".to_string(),
                }));
            }

            return self.eval_function_call(module, ctx, tag_name, function, arg_values);
        }

        if let Some(record_def) = self.resolve_record_definition(module, tag_name) {
            let record_has_children_field = record_def
                .properties
                .iter()
                .any(|p| p.name.as_str() == "children");

            if fields.contains_key("children") && !child_values.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "children passed either as a named property or as element body"
                        .to_string(),
                    actual: "both a children property and element body".to_string(),
                    operation: "element record call".to_string(),
                }));
            }

            if !record_has_children_field && !child_values.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "record without 'children' field".to_string(),
                    actual: "element body content".to_string(),
                    operation: "element record call".to_string(),
                }));
            }

            if record_has_children_field && !fields.contains_key("children") && !child_values.is_empty()
            {
                fields.insert(SmolStr::new("children"), Value::Array(child_values));
            }

            return self.build_record_value(module, ctx, record_def.name.as_str(), fields);
        }

        if !child_values.is_empty() && !fields.contains_key("children") {
            fields.insert(SmolStr::new("children"), Value::Array(child_values));
        }

        Ok(Value::Record {
            type_name: element.tag.clone(),
            fields,
        })
    }

    fn eval_member(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        base_expr: ExprId,
        member: &Name,
    ) -> Result<Value, RuntimeError> {
        if let ast::Expr::Ident(base_name) = module.expr(base_expr) {
            // Prefer runtime value if variable exists
            if let Some(var_value) = ctx.try_lookup_variable(base_name.as_str()) {
                return self.project_member(var_value, member, Some(base_name.as_str()));
            }

            if let Some(enum_def) = self.resolve_enum_definition(module, base_name) {
                if enum_def
                    .members
                    .iter()
                    .any(|m| m.name.as_str() == member.as_str())
                {
                    return Ok(Value::EnumVariant {
                        type_name: enum_def.name.clone(),
                        variant: SmolStr::new(member.as_str()),
                    });
                } else {
                    return Err(RuntimeError::new(RuntimeErrorKind::EnumMemberNotFound {
                        enum_name: SmolStr::new(enum_def.name.as_str()),
                        member: SmolStr::new(member.as_str()),
                    }));
                }
            } else {
                return Err(RuntimeError::new(RuntimeErrorKind::EnumNotFound {
                    name: SmolStr::new(base_name.as_str()),
                }));
            }
        }

        let base_value = self.eval_expr(module, ctx, base_expr)?;
        self.project_member(base_value, member, None)
    }

    fn project_member(
        &self,
        base_value: Value,
        member: &Name,
        record_label: Option<&str>,
    ) -> Result<Value, RuntimeError> {
        match base_value {
            Value::Record { fields, .. } => {
                if let Some(value) = fields.get(member.as_str()) {
                    Ok(value.clone())
                } else {
                    let name = record_label.unwrap_or("record");
                    Err(RuntimeError::new(RuntimeErrorKind::RecordFieldNotFound {
                        record: SmolStr::new(name),
                        field: SmolStr::new(member.as_str()),
                    }))
                }
            }
            Value::EnumVariant { .. } => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "record".to_string(),
                actual: "enum".to_string(),
                operation: format!("member access .{}", member.as_str()),
            })),
            other => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "record".to_string(),
                actual: other.type_name().to_string(),
                operation: format!("member access .{}", member.as_str()),
            })),
        }
    }

    /// Instantiate a record value from its definition, applying default values.
    pub fn instantiate_record_defaults(
        &self,
        module: &Module,
        record_name: &str,
    ) -> Result<Value, RuntimeError> {
        let mut ctx = ExecutionContext::new();
        self.build_record_value(module, &mut ctx, record_name, FxHashMap::default())
    }

    /// Evaluate a for loop expression
    fn eval_for(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        item: &Name,
        index: Option<&Name>,
        iterable_expr: ExprId,
        body_expr: ExprId,
    ) -> Result<Value, RuntimeError> {
        // Evaluate the iterable expression
        let iterable_value = self.eval_expr(module, ctx, iterable_expr)?;

        // Extract array elements
        let elements = match iterable_value {
            Value::Array(ref arr) => arr.clone(),
            _ => {
                return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                    expected: "array".to_string(),
                    actual: iterable_value.type_name().to_string(),
                    operation: "for loop iteration".to_string(),
                }))
            }
        };

        // Collect results from each iteration
        let mut results = Vec::with_capacity(elements.len());

        // Iterate over elements
        for (idx, element) in elements.iter().enumerate() {
            // Create new scope for loop variables
            ctx.push_scope();

            // Bind item variable
            ctx.define_variable(SmolStr::new(item.as_str()), element.clone());

            // Bind index variable if present
            if let Some(index_name) = index {
                ctx.define_variable(SmolStr::new(index_name.as_str()), Value::Int(idx as i64));
            }

            // Evaluate body
            let result = self.eval_expr(module, ctx, body_expr)?;
            results.push(result);

            // Pop scope
            ctx.pop_scope();
        }

        // Return array of results
        Ok(Value::Array(results))
    }

    fn resolve_enum_definition<'a>(
        &self,
        module: &'a Module,
        name: &Name,
    ) -> Option<&'a nx_hir::EnumDef> {
        self.resolve_enum_definition_inner(module, name, &mut FxHashSet::default())
    }

    fn resolve_record_definition<'a>(
        &self,
        module: &'a Module,
        name: &str,
    ) -> Option<&'a nx_hir::RecordDef> {
        self.resolve_record_definition_inner(module, name, &mut FxHashSet::default())
    }

    fn resolve_enum_definition_inner<'a>(
        &self,
        module: &'a Module,
        name: &Name,
        seen: &mut FxHashSet<SmolStr>,
    ) -> Option<&'a nx_hir::EnumDef> {
        let key = SmolStr::new(name.as_str());
        if !seen.insert(key.clone()) {
            return None;
        }

        let result = match module.find_item(name.as_str()) {
            Some(nx_hir::Item::Enum(enum_def)) => Some(enum_def),
            Some(nx_hir::Item::TypeAlias(alias)) => match &alias.ty {
                ast::TypeRef::Name(target) => {
                    self.resolve_enum_definition_inner(module, target, seen)
                }
                _ => None,
            },
            _ => None,
        };

        seen.remove(&key);
        result
    }

    fn resolve_record_definition_inner<'a>(
        &self,
        module: &'a Module,
        name: &str,
        seen: &mut FxHashSet<SmolStr>,
    ) -> Option<&'a nx_hir::RecordDef> {
        let key = SmolStr::new(name);
        if !seen.insert(key.clone()) {
            return None;
        }

        let result = match module.find_item(name) {
            Some(Item::Record(def)) => Some(def),
            Some(Item::TypeAlias(alias)) => match &alias.ty {
                ast::TypeRef::Name(target) => {
                    self.resolve_record_definition_inner(module, target.as_str(), seen)
                }
                _ => None,
            },
            _ => None,
        };

        seen.remove(&key);
        result
    }
}

impl Interpreter {
    fn eval_record_literal(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        record: &Name,
        properties: &[(Name, ExprId)],
    ) -> Result<Value, RuntimeError> {
        let mut overrides = FxHashMap::default();
        for (name, expr_id) in properties {
            let value = self.eval_expr(module, ctx, *expr_id)?;
            overrides.insert(SmolStr::new(name.as_str()), value);
        }

        self.build_record_value(module, ctx, record.as_str(), overrides)
    }

    fn build_record_value(
        &self,
        module: &Module,
        ctx: &mut ExecutionContext,
        record_name: &str,
        mut overrides: FxHashMap<SmolStr, Value>,
    ) -> Result<Value, RuntimeError> {
        let record_def = module.find_item(record_name);
        let record_def = match record_def {
            Some(nx_hir::Item::Record(def)) => def,
            _ => {
                return Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable {
                    name: SmolStr::new(record_name),
                }))
            }
        };

        for prop in &record_def.properties {
            if overrides.contains_key(prop.name.as_str()) {
                continue;
            }

            let value = if let Some(default_expr) = prop.default {
                self.eval_expr(module, ctx, default_expr)?
            } else {
                Value::Null
            };
            overrides.insert(SmolStr::new(prop.name.as_str()), value);
        }

        Ok(Value::Record {
            type_name: Name::new(record_name),
            fields: overrides,
        })
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
