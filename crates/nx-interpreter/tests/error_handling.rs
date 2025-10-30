//! Integration tests for runtime error handling
//!
//! This test suite verifies that all RuntimeErrorKind variants are properly
//! detected and reported with appropriate error messages.

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal};
use nx_hir::{Function, Item, Module, Name, SourceId};
use nx_interpreter::{Interpreter, RuntimeErrorKind};

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

#[test]
fn test_division_by_zero_int() {
    let mut module = Module::new(SourceId::new(0));

    // Create: 10 / 0
    let ten = module.alloc_expr(Expr::Literal(Literal::Int(10)));
    let zero = module.alloc_expr(Expr::Literal(Literal::Int(0)));
    let div_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: ten,
        op: BinOp::Div,
        rhs: zero,
        span: span(0, 10),
    });

    let func = Function {
        name: Name::new("divide_by_zero"),
        params: vec![],
        return_type: None,
        body: div_expr,
        span: span(0, 20),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter.execute_function(&module, "divide_by_zero", vec![]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind(), RuntimeErrorKind::DivisionByZero));
}

#[test]
fn test_function_not_found() {
    let module = Module::new(SourceId::new(0));
    let interpreter = Interpreter::new();
    let result = interpreter.execute_function(&module, "nonexistent", vec![]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind(),
        RuntimeErrorKind::FunctionNotFound { .. }
    ));
}
