//! Execution context for managing runtime state during interpretation.

use crate::error::{CallFrame, RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Resource limits for execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum number of operations allowed per execution
    pub max_operations: usize,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_operations: 1_000_000,
            max_recursion_depth: 1000,
        }
    }
}

/// Variable scope for local variables
#[derive(Debug, Clone)]
struct Scope {
    variables: FxHashMap<SmolStr, Value>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
        }
    }

    fn define(&mut self, name: SmolStr, value: Value) {
        self.variables.insert(name, value);
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    fn update(&mut self, name: &str, value: Value) -> bool {
        if let Some(v) = self.variables.get_mut(name) {
            *v = value;
            true
        } else {
            false
        }
    }
}

/// Execution context maintaining runtime state
#[derive(Debug)]
pub struct ExecutionContext {
    /// Stack of scopes (innermost scope is last)
    scopes: Vec<Scope>,
    /// Call stack for function calls
    call_stack: Vec<CallFrame>,
    /// Operation counter
    operation_count: usize,
    /// Resource limits
    limits: ResourceLimits,
}

impl ExecutionContext {
    /// Create a new execution context with default limits
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }

    /// Create a new execution context with custom limits
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            scopes: vec![Scope::new()],
            call_stack: Vec::new(),
            operation_count: 0,
            limits,
        }
    }

    /// Push a new scope onto the scope stack
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Pop the current scope from the scope stack
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: SmolStr, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, value);
        }
    }

    /// Look up a variable in the scope stack (innermost to outermost)
    pub fn lookup_variable(&self, name: &str) -> Result<Value, RuntimeError> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.lookup(name) {
                return Ok(value.clone());
            }
        }

        Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable {
            name: SmolStr::new(name),
        }))
    }

    /// Update a variable in the scope stack
    pub fn update_variable(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter_mut().rev() {
            if scope.update(name, value.clone()) {
                return Ok(());
            }
        }

        Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable {
            name: SmolStr::new(name),
        }))
    }

    /// Push a call frame onto the call stack
    pub fn push_call_frame(&mut self, frame: CallFrame) -> Result<(), RuntimeError> {
        if self.call_stack.len() >= self.limits.max_recursion_depth {
            return Err(RuntimeError::new(RuntimeErrorKind::StackOverflow {
                depth: self.limits.max_recursion_depth,
            })
            .with_call_stack(self.call_stack.clone()));
        }
        self.call_stack.push(frame);
        Ok(())
    }

    /// Pop a call frame from the call stack
    pub fn pop_call_frame(&mut self) {
        self.call_stack.pop();
    }

    /// Get a copy of the current call stack
    pub fn get_call_stack(&self) -> Vec<CallFrame> {
        self.call_stack.clone()
    }

    /// Increment the operation counter and check limits
    pub fn check_operation_limit(&mut self) -> Result<(), RuntimeError> {
        self.operation_count += 1;
        if self.operation_count > self.limits.max_operations {
            return Err(RuntimeError::new(
                RuntimeErrorKind::OperationLimitExceeded {
                    limit: self.limits.max_operations,
                },
            )
            .with_call_stack(self.call_stack.clone()));
        }
        Ok(())
    }

    /// Get the current operation count
    pub fn operation_count(&self) -> usize {
        self.operation_count
    }

    /// Get the current call stack depth
    pub fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = ExecutionContext::new();
        assert_eq!(ctx.operation_count(), 0);
        assert_eq!(ctx.call_stack_depth(), 0);
    }

    #[test]
    fn test_variable_define_and_lookup() {
        let mut ctx = ExecutionContext::new();
        ctx.define_variable(SmolStr::new("x"), Value::Int(42));

        let result = ctx.lookup_variable("x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_undefined_variable() {
        let ctx = ExecutionContext::new();
        let result = ctx.lookup_variable("x");
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_stack() {
        let mut ctx = ExecutionContext::new();
        ctx.define_variable(SmolStr::new("x"), Value::Int(1));

        ctx.push_scope();
        ctx.define_variable(SmolStr::new("x"), Value::Int(2));
        assert_eq!(ctx.lookup_variable("x").unwrap(), Value::Int(2));

        ctx.pop_scope();
        assert_eq!(ctx.lookup_variable("x").unwrap(), Value::Int(1));
    }

    #[test]
    fn test_operation_limit() {
        let mut ctx = ExecutionContext::with_limits(ResourceLimits {
            max_operations: 5,
            max_recursion_depth: 10,
        });

        for _ in 0..5 {
            assert!(ctx.check_operation_limit().is_ok());
        }
        assert!(ctx.check_operation_limit().is_err());
    }

    #[test]
    fn test_variable_update() {
        let mut ctx = ExecutionContext::new();
        ctx.define_variable(SmolStr::new("x"), Value::Int(1));
        assert!(ctx.update_variable("x", Value::Int(2)).is_ok());
        assert_eq!(ctx.lookup_variable("x").unwrap(), Value::Int(2));
    }
}
