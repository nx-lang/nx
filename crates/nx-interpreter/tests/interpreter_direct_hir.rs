//! Unit tests for interpreter using manually-constructed HIR
//!
//! These tests bypass the parser and create HIR modules directly to test
//! interpreter functionality.

use nx_hir::ast::{BinOp, Expr, Stmt};
use nx_hir::{Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_diagnostics::{TextSize, TextSpan};
use smol_str::SmolStr;

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// Test simple addition function
#[test]
fn test_add_function_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    // Create parameters a:int and b:int
    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
    ];

    // Create expression: a + b
    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let add_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Add,
        rhs: b_expr,
        span: span(0, 5),
    });

    // Create function
    let func = Function {
        name: Name::new("add"),
        params,
        return_type: None,
        body: add_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    // Execute function
    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "add", vec![Value::Int(5), Value::Int(3)])
        .unwrap();

    assert_eq!(result, Value::Int(8));
}

/// Test subtraction
#[test]
fn test_subtract_function_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
    ];

    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let sub_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Sub,
        rhs: b_expr,
        span: span(0, 5),
    });

    let func = Function {
        name: Name::new("sub"),
        params,
        return_type: None,
        body: sub_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "sub", vec![Value::Int(10), Value::Int(3)])
        .unwrap();

    assert_eq!(result, Value::Int(7));
}

/// Test multiplication
#[test]
fn test_multiply_function_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
    ];

    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let mul_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Mul,
        rhs: b_expr,
        span: span(0, 5),
    });

    let func = Function {
        name: Name::new("mul"),
        params,
        return_type: None,
        body: mul_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "mul", vec![Value::Int(4), Value::Int(5)])
        .unwrap();

    assert_eq!(result, Value::Int(20));
}

/// Test division
#[test]
fn test_divide_function_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
    ];

    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let div_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Div,
        rhs: b_expr,
        span: span(0, 5),
    });

    let func = Function {
        name: Name::new("div"),
        params,
        return_type: None,
        body: div_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "div", vec![Value::Int(15), Value::Int(3)])
        .unwrap();

    assert_eq!(result, Value::Int(5));
}

/// Test division by zero error
#[test]
fn test_division_by_zero_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
    ];

    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let div_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Div,
        rhs: b_expr,
        span: span(0, 5),
    });

    let func = Function {
        name: Name::new("div"),
        params,
        return_type: None,
        body: div_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter.execute_function(&module, "div", vec![Value::Int(10), Value::Int(0)]);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        *err.kind(),
        nx_interpreter::RuntimeErrorKind::DivisionByZero
    );
}

/// Test string concatenation
#[test]
fn test_string_concat_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("string"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("string"),
            span(2, 3),
        ),
    ];

    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let concat_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Concat,
        rhs: b_expr,
        span: span(0, 5),
    });

    let func = Function {
        name: Name::new("concat"),
        params,
        return_type: None,
        body: concat_expr,
        span: span(0, 10),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(
            &module,
            "concat",
            vec![
                Value::String(SmolStr::new("hello")),
                Value::String(SmolStr::new(" world")),
            ],
        )
        .unwrap();

    assert_eq!(result, Value::String(SmolStr::new("hello world")));
}

/// Test function with block and local variables
#[test]
fn test_block_with_variables_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Create: let double = x + x
    let x_expr1 = module.alloc_expr(Expr::Ident(Name::new("x")));
    let x_expr2 = module.alloc_expr(Expr::Ident(Name::new("x")));
    let add_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: x_expr1,
        op: BinOp::Add,
        rhs: x_expr2,
        span: span(0, 5),
    });

    let let_stmt = Stmt::Let {
        name: Name::new("double"),
        ty: None,
        init: add_expr,
        span: span(0, 10),
    };

    // Final expression: double
    let double_expr = module.alloc_expr(Expr::Ident(Name::new("double")));

    // Create block
    let block_expr = module.alloc_expr(Expr::Block {
        stmts: vec![let_stmt],
        expr: Some(double_expr),
        span: span(0, 15),
    });

    let func = Function {
        name: Name::new("compute"),
        params,
        return_type: None,
        body: block_expr,
        span: span(0, 20),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "compute", vec![Value::Int(5)])
        .unwrap();

    assert_eq!(result, Value::Int(10));
}

/// Test complex arithmetic with multiple operations
#[test]
fn test_complex_arithmetic_direct_hir() {
    let mut module = Module::new(SourceId::new(0));

    let params = vec![
        Param::new(
            Name::new("a"),
            nx_hir::ast::TypeRef::name("int"),
            span(0, 1),
        ),
        Param::new(
            Name::new("b"),
            nx_hir::ast::TypeRef::name("int"),
            span(2, 3),
        ),
        Param::new(
            Name::new("c"),
            nx_hir::ast::TypeRef::name("int"),
            span(4, 5),
        ),
    ];

    // Create: a + b * c
    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let c_expr = module.alloc_expr(Expr::Ident(Name::new("c")));

    let mul_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: b_expr,
        op: BinOp::Mul,
        rhs: c_expr,
        span: span(5, 10),
    });

    let add_expr = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Add,
        rhs: mul_expr,
        span: span(0, 10),
    });

    let func = Function {
        name: Name::new("calc"),
        params,
        return_type: None,
        body: add_expr,
        span: span(0, 15),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(
            &module,
            "calc",
            vec![Value::Int(2), Value::Int(3), Value::Int(4)],
        )
        .unwrap();

    // 2 + (3 * 4) = 2 + 12 = 14
    assert_eq!(result, Value::Int(14));
}
