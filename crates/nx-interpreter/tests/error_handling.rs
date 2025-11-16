//! Integration tests for runtime error handling
//!
//! This test suite verifies that all RuntimeErrorKind variants are properly
//! detected and reported with appropriate error messages.

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal};
use nx_hir::{lower, Function, Item, Module, Name, SourceId};
use nx_interpreter::{Interpreter, RuntimeErrorKind, Value};
use nx_syntax::parse_str;

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// Parses NX source and lowers it into a module for interpreter tests.
fn module_from_source(source: &str) -> Module {
    let parse_result = parse_str(source, "error-handling.nx");
    assert!(
        parse_result.is_ok(),
        "Parser diagnostics: {:?}",
        parse_result.errors
    );
    let root = parse_result.root().expect("Should have root node");
    lower(root, SourceId::new(0))
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

#[test]
fn test_paren_function_parameter_count_mismatch() {
    let source = r#"
        let double(value:int): int = { value + value }
    "#;
    let module = module_from_source(source);
    let interpreter = Interpreter::new();

    let err = interpreter
        .execute_function(&module, "double", vec![Value::Int(2), Value::Int(8)])
        .expect_err("Calling with the wrong number of args should fail");

    match err.kind() {
        RuntimeErrorKind::ParameterCountMismatch {
            expected,
            actual,
            function,
        } => {
            assert_eq!(*expected, 1);
            assert_eq!(*actual, 2);
            assert_eq!(function.as_str(), "double");
        }
        other => panic!("Expected ParameterCountMismatch, got {:?}", other),
    }
}

#[test]
fn test_paren_function_argument_type_mismatch() {
    let source = r#"
        let add(a:int, b:int): int = { a + b }
    "#;
    let module = module_from_source(source);
    let interpreter = Interpreter::new();

    let err = interpreter
        .execute_function(
            &module,
            "add",
            vec![Value::Boolean(true), Value::Int(3)],
        )
        .expect_err("Adding incompatible argument types should fail");

    match err.kind() {
        RuntimeErrorKind::TypeMismatch { operation, .. } => {
            assert_eq!(operation, "addition");
        }
        other => panic!("Expected TypeMismatch during addition, got {:?}", other),
    }
}

#[test]
fn test_paren_function_invalid_return_type_usage() {
    let source = r#"
        let flag(): string = { "yes" }
        let select(value:int): int = {
            if flag() {
                value
            } else {
                value
            }
        }
    "#;
    let module = module_from_source(source);
    let interpreter = Interpreter::new();

    let err = interpreter
        .execute_function(&module, "select", vec![Value::Int(1)])
        .expect_err("Function should fail when condition isn't boolean");

    match err.kind() {
        RuntimeErrorKind::TypeMismatch { operation, .. } => {
            assert_eq!(operation, "if condition");
        }
        other => panic!("Expected TypeMismatch for invalid return type, got {:?}", other),
    }
}
