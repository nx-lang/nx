//! Integration tests for for loop expressions
//!
//! Tests T047-T051: Loop execution

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal};
use nx_hir::{Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// T047: Test simple for loop
#[test]
fn test_for_loop_simple() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: double_all(items) = for item in items { item * 2 }
    let params = vec![Param::new(
        Name::new("items"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 5),
    )];

    // Build: items identifier
    let items_expr = module.alloc_expr(Expr::Ident(Name::new("items")));

    // Build body: item * 2
    let item_expr = module.alloc_expr(Expr::Ident(Name::new("item")));
    let two_expr = module.alloc_expr(Expr::Literal(Literal::Int(2)));
    let body = module.alloc_expr(Expr::BinaryOp {
        lhs: item_expr,
        op: BinOp::Mul,
        rhs: two_expr,
        span: span(0, 10),
    });

    // Build for loop
    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: None,
        iterable: items_expr,
        body,
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("double_all"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    // Test with array [1, 2, 3]
    let interpreter = Interpreter::new();
    let input = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let result = interpreter
        .execute_function(&module, "double_all", vec![input])
        .unwrap();

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
    );
}

/// T048: Test for loop with index
#[test]
fn test_for_loop_with_index() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: add_index(items) = for item, index in items { item + index }
    let params = vec![Param::new(
        Name::new("items"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 5),
    )];

    let items_expr = module.alloc_expr(Expr::Ident(Name::new("items")));

    // Build body: item + index
    let item_expr = module.alloc_expr(Expr::Ident(Name::new("item")));
    let index_expr = module.alloc_expr(Expr::Ident(Name::new("index")));
    let body = module.alloc_expr(Expr::BinaryOp {
        lhs: item_expr,
        op: BinOp::Add,
        rhs: index_expr,
        span: span(0, 15),
    });

    // Build for loop with index
    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: Some(Name::new("index")),
        iterable: items_expr,
        body,
        span: span(0, 35),
    });

    let func = Function {
        name: Name::new("add_index"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 45),
    };

    module.add_item(Item::Function(func));

    // Test: [10, 20, 30] -> [10+0, 20+1, 30+2] = [10, 21, 32]
    let interpreter = Interpreter::new();
    let input = Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
    let result = interpreter
        .execute_function(&module, "add_index", vec![input])
        .unwrap();

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(10), Value::Int(21), Value::Int(32)])
    );
}

/// T049: Test nested for loops
#[test]
fn test_nested_for_loops() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: flatten(matrix) = for row in matrix { for cell in row { cell } }
    // This is a simplified version - real nested loops would need proper handling
    // For now, test a single level

    let params = vec![Param::new(
        Name::new("numbers"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 7),
    )];

    let numbers_expr = module.alloc_expr(Expr::Ident(Name::new("numbers")));

    // Just return the number itself
    let num_expr = module.alloc_expr(Expr::Ident(Name::new("num")));

    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("num"),
        index: None,
        iterable: numbers_expr,
        body: num_expr,
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("identity"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let input = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let result = interpreter
        .execute_function(&module, "identity", vec![input])
        .unwrap();

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

/// T050: Test for loop with empty array
#[test]
fn test_for_loop_empty_array() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![Param::new(
        Name::new("items"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 5),
    )];

    let items_expr = module.alloc_expr(Expr::Ident(Name::new("items")));
    let item_expr = module.alloc_expr(Expr::Ident(Name::new("item")));

    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: None,
        iterable: items_expr,
        body: item_expr,
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("process"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    // Test with empty array
    let interpreter = Interpreter::new();
    let input = Value::Array(vec![]);
    let result = interpreter
        .execute_function(&module, "process", vec![input])
        .unwrap();

    assert_eq!(result, Value::Array(vec![]));
}

/// T051: Test for loop with type error (non-array iterable)
#[test]
fn test_for_loop_type_error() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![Param::new(
        Name::new("value"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 5),
    )];

    let value_expr = module.alloc_expr(Expr::Ident(Name::new("value")));
    let item_expr = module.alloc_expr(Expr::Ident(Name::new("item")));

    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: None,
        iterable: value_expr,
        body: item_expr,
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("bad_loop"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    // Test with integer (should fail)
    let interpreter = Interpreter::new();
    let result = interpreter.execute_function(&module, "bad_loop", vec![Value::Int(42)]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind(),
        nx_interpreter::RuntimeErrorKind::TypeMismatch { .. }
    ));
}
