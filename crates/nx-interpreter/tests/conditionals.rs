//! Integration tests for conditional expressions (if/else)
//!
//! Tests T039-T042: Conditional execution

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal};
use nx_hir::{Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// T039: Test if/else true branch
#[test]
fn test_if_else_true_branch() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: max(a, b) = if a > b { a } else { b }
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

    // Build condition: a > b
    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Gt,
        rhs: b_expr,
        span: span(0, 5),
    });

    // Then branch: a
    let then_branch = module.alloc_expr(Expr::Ident(Name::new("a")));

    // Else branch: b
    let else_branch = module.alloc_expr(Expr::Ident(Name::new("b")));

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("max"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    // Test with a > b (true branch)
    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "max", vec![Value::Int(10), Value::Int(5)])
        .unwrap();

    assert_eq!(result, Value::Int(10));
}

/// T040: Test if/else false branch
#[test]
fn test_if_else_false_branch() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: max(a, b) = if a > b { a } else { b }
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

    // Build condition: a > b
    let a_expr = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr = module.alloc_expr(Expr::Ident(Name::new("b")));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr,
        op: BinOp::Gt,
        rhs: b_expr,
        span: span(0, 5),
    });

    // Then branch: a
    let then_branch = module.alloc_expr(Expr::Ident(Name::new("a")));

    // Else branch: b
    let else_branch = module.alloc_expr(Expr::Ident(Name::new("b")));

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("max"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    // Test with a < b (false branch)
    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "max", vec![Value::Int(3), Value::Int(7)])
        .unwrap();

    assert_eq!(result, Value::Int(7));
}

/// T041: Test nested conditionals
#[test]
fn test_nested_conditionals() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: classify(n) = if n > 0 { if n > 10 { 2 } else { 1 } } else { 0 }
    let params = vec![Param::new(
        Name::new("n"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Outer condition: n > 0
    let n_expr1 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let zero_expr = module.alloc_expr(Expr::Literal(Literal::Int(0)));
    let outer_condition = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr1,
        op: BinOp::Gt,
        rhs: zero_expr,
        span: span(0, 5),
    });

    // Inner condition: n > 10
    let n_expr2 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let ten_expr = module.alloc_expr(Expr::Literal(Literal::Int(10)));
    let inner_condition = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr2,
        op: BinOp::Gt,
        rhs: ten_expr,
        span: span(0, 5),
    });

    // Inner then: 2
    let two_expr = module.alloc_expr(Expr::Literal(Literal::Int(2)));

    // Inner else: 1
    let one_expr = module.alloc_expr(Expr::Literal(Literal::Int(1)));

    // Inner if
    let inner_if = module.alloc_expr(Expr::If {
        condition: inner_condition,
        then_branch: two_expr,
        else_branch: Some(one_expr),
        span: span(0, 10),
    });

    // Outer else: 0
    let zero_expr2 = module.alloc_expr(Expr::Literal(Literal::Int(0)));

    // Outer if
    let outer_if = module.alloc_expr(Expr::If {
        condition: outer_condition,
        then_branch: inner_if,
        else_branch: Some(zero_expr2),
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("classify"),
        params,
        return_type: None,
        body: outer_if,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test n > 10 (returns 2)
    let result = interpreter
        .execute_function(&module, "classify", vec![Value::Int(15)])
        .unwrap();
    assert_eq!(result, Value::Int(2));

    // Test 0 < n <= 10 (returns 1)
    let result = interpreter
        .execute_function(&module, "classify", vec![Value::Int(5)])
        .unwrap();
    assert_eq!(result, Value::Int(1));

    // Test n <= 0 (returns 0)
    let result = interpreter
        .execute_function(&module, "classify", vec![Value::Int(-5)])
        .unwrap();
    assert_eq!(result, Value::Int(0));
}

/// T042: Test conditionals with complex expressions
#[test]
fn test_conditionals_with_complex_expressions() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: f(a, b, c) = if (a > b) && (b > c) { a + b + c } else { 0 }
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

    // Build condition: (a > b) && (b > c)
    let a_expr1 = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr1 = module.alloc_expr(Expr::Ident(Name::new("b")));
    let left_cond = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr1,
        op: BinOp::Gt,
        rhs: b_expr1,
        span: span(0, 5),
    });

    let b_expr2 = module.alloc_expr(Expr::Ident(Name::new("b")));
    let c_expr1 = module.alloc_expr(Expr::Ident(Name::new("c")));
    let right_cond = module.alloc_expr(Expr::BinaryOp {
        lhs: b_expr2,
        op: BinOp::Gt,
        rhs: c_expr1,
        span: span(0, 5),
    });

    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: left_cond,
        op: BinOp::And,
        rhs: right_cond,
        span: span(0, 15),
    });

    // Then branch: a + b + c
    let a_expr2 = module.alloc_expr(Expr::Ident(Name::new("a")));
    let b_expr3 = module.alloc_expr(Expr::Ident(Name::new("b")));
    let sum_ab = module.alloc_expr(Expr::BinaryOp {
        lhs: a_expr2,
        op: BinOp::Add,
        rhs: b_expr3,
        span: span(0, 5),
    });

    let c_expr2 = module.alloc_expr(Expr::Ident(Name::new("c")));
    let sum_abc = module.alloc_expr(Expr::BinaryOp {
        lhs: sum_ab,
        op: BinOp::Add,
        rhs: c_expr2,
        span: span(0, 10),
    });

    // Else branch: 0
    let zero_expr = module.alloc_expr(Expr::Literal(Literal::Int(0)));

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch: sum_abc,
        else_branch: Some(zero_expr),
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("f"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test with condition true: a=10, b=5, c=2 (10 > 5 && 5 > 2)
    let result = interpreter
        .execute_function(
            &module,
            "f",
            vec![Value::Int(10), Value::Int(5), Value::Int(2)],
        )
        .unwrap();
    assert_eq!(result, Value::Int(17)); // 10 + 5 + 2

    // Test with condition false: a=10, b=5, c=8 (10 > 5 but 5 < 8)
    let result = interpreter
        .execute_function(
            &module,
            "f",
            vec![Value::Int(10), Value::Int(5), Value::Int(8)],
        )
        .unwrap();
    assert_eq!(result, Value::Int(0));
}

/// Test if without else (returns null on false)
#[test]
fn test_if_without_else() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: f(x) = if x > 0 { x }
    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Condition: x > 0
    let x_expr = module.alloc_expr(Expr::Ident(Name::new("x")));
    let zero_expr = module.alloc_expr(Expr::Literal(Literal::Int(0)));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: x_expr,
        op: BinOp::Gt,
        rhs: zero_expr,
        span: span(0, 5),
    });

    // Then branch: x
    let then_branch = module.alloc_expr(Expr::Ident(Name::new("x")));

    // If expression (no else)
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: None,
        span: span(0, 10),
    });

    let func = Function {
        name: Name::new("f"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 20),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test with true condition
    let result = interpreter
        .execute_function(&module, "f", vec![Value::Int(5)])
        .unwrap();
    assert_eq!(result, Value::Int(5));

    // Test with false condition (should return null)
    let result = interpreter
        .execute_function(&module, "f", vec![Value::Int(-5)])
        .unwrap();
    assert_eq!(result, Value::Null);
}

/// Test logical NOT operator with if
#[test]
fn test_if_with_not_operator() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: is_zero(x) = if !(x > 0) { 1 } else { 0 }
    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Inner condition: x > 0
    let x_expr = module.alloc_expr(Expr::Ident(Name::new("x")));
    let zero_expr = module.alloc_expr(Expr::Literal(Literal::Int(0)));
    let inner_cond = module.alloc_expr(Expr::BinaryOp {
        lhs: x_expr,
        op: BinOp::Gt,
        rhs: zero_expr,
        span: span(0, 5),
    });

    // Condition: !(x > 0)
    let condition = module.alloc_expr(Expr::UnaryOp {
        op: nx_hir::ast::UnOp::Not,
        expr: inner_cond,
        span: span(0, 7),
    });

    // Then: 1
    let one_expr = module.alloc_expr(Expr::Literal(Literal::Int(1)));

    // Else: 0
    let zero_expr2 = module.alloc_expr(Expr::Literal(Literal::Int(0)));

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch: one_expr,
        else_branch: Some(zero_expr2),
        span: span(0, 20),
    });

    let func = Function {
        name: Name::new("is_zero_or_negative"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 30),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test with x <= 0 (returns 1)
    let result = interpreter
        .execute_function(&module, "is_zero_or_negative", vec![Value::Int(0)])
        .unwrap();
    assert_eq!(result, Value::Int(1));

    // Test with x > 0 (returns 0)
    let result = interpreter
        .execute_function(&module, "is_zero_or_negative", vec![Value::Int(5)])
        .unwrap();
    assert_eq!(result, Value::Int(0));
}
