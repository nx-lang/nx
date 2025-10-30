//! NX Interpreter - Runtime execution engine for NX HIR
//!
//! This crate provides a tree-walking interpreter for executing NX functions
//! represented in High-level Intermediate Representation (HIR). It supports
//! arithmetic, logical, and control flow operations with comprehensive error
//! reporting and resource limits for safe execution.

mod context;
mod error;
mod interpreter;
mod value;

pub mod eval;

pub use context::{ExecutionContext, ResourceLimits};
pub use error::{RuntimeError, RuntimeErrorKind};
pub use interpreter::Interpreter;
pub use value::Value;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_creation() {
        let _interpreter = Interpreter::new();
    }
}
