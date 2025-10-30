//! Integration tests for recursive function calls (T055)

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal};
use nx_hir::{Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, ResourceLimits, RuntimeErrorKind, Value};

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// Test factorial function with recursion
#[test]
fn test_factorial_recursion() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: factorial(n) = if n <= 1 { 1 } else { n * factorial(n - 1) }
    let params = vec![Param::new(
        Name::new("n"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Build condition: n <= 1
    let n_expr1 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr1 = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr1,
        op: BinOp::Le,
        rhs: one_expr1,
        span: span(0, 5),
    });

    // Then branch: 1
    let then_branch = module.alloc_expr(Expr::Literal(Literal::Int(1)));

    // Else branch: n * factorial(n - 1)
    // First: n - 1
    let n_expr2 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr2 = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let n_minus_1 = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr2,
        op: BinOp::Sub,
        rhs: one_expr2,
        span: span(0, 5),
    });

    // Recursive call: factorial(n - 1)
    let factorial_ident = module.alloc_expr(Expr::Ident(Name::new("factorial")));
    let recursive_call = module.alloc_expr(Expr::Call {
        func: factorial_ident,
        args: vec![n_minus_1],
        span: span(0, 15),
    });

    // n * factorial(n - 1)
    let n_expr3 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let else_branch = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr3,
        op: BinOp::Mul,
        rhs: recursive_call,
        span: span(0, 20),
    });

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("factorial"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test factorial(5) = 120
    let result = interpreter
        .execute_function(&module, "factorial", vec![Value::Int(5)])
        .unwrap();
    assert_eq!(result, Value::Int(120));

    // Test factorial(0) = 1
    let result = interpreter
        .execute_function(&module, "factorial", vec![Value::Int(0)])
        .unwrap();
    assert_eq!(result, Value::Int(1));

    // Test factorial(1) = 1
    let result = interpreter
        .execute_function(&module, "factorial", vec![Value::Int(1)])
        .unwrap();
    assert_eq!(result, Value::Int(1));
}

/// Test Fibonacci function with recursion
#[test]
fn test_fibonacci_recursion() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: fib(n) = if n <= 1 { n } else { fib(n-1) + fib(n-2) }
    let params = vec![Param::new(
        Name::new("n"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Condition: n <= 1
    let n_expr1 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr1,
        op: BinOp::Le,
        rhs: one_expr,
        span: span(0, 5),
    });

    // Then branch: n
    let then_branch = module.alloc_expr(Expr::Ident(Name::new("n")));

    // Else branch: fib(n-1) + fib(n-2)
    // fib(n-1)
    let n_expr2 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr2 = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let n_minus_1 = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr2,
        op: BinOp::Sub,
        rhs: one_expr2,
        span: span(0, 5),
    });
    let fib_ident1 = module.alloc_expr(Expr::Ident(Name::new("fib")));
    let fib_n_minus_1 = module.alloc_expr(Expr::Call {
        func: fib_ident1,
        args: vec![n_minus_1],
        span: span(0, 10),
    });

    // fib(n-2)
    let n_expr3 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let two_expr = module.alloc_expr(Expr::Literal(Literal::Int(2)));
    let n_minus_2 = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr3,
        op: BinOp::Sub,
        rhs: two_expr,
        span: span(0, 5),
    });
    let fib_ident2 = module.alloc_expr(Expr::Ident(Name::new("fib")));
    let fib_n_minus_2 = module.alloc_expr(Expr::Call {
        func: fib_ident2,
        args: vec![n_minus_2],
        span: span(0, 10),
    });

    // fib(n-1) + fib(n-2)
    let else_branch = module.alloc_expr(Expr::BinaryOp {
        lhs: fib_n_minus_1,
        op: BinOp::Add,
        rhs: fib_n_minus_2,
        span: span(0, 20),
    });

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("fib"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Test fib(6) = 8
    let result = interpreter
        .execute_function(&module, "fib", vec![Value::Int(6)])
        .unwrap();
    assert_eq!(result, Value::Int(8));

    // Test fib(0) = 0
    let result = interpreter
        .execute_function(&module, "fib", vec![Value::Int(0)])
        .unwrap();
    assert_eq!(result, Value::Int(0));

    // Test fib(1) = 1
    let result = interpreter
        .execute_function(&module, "fib", vec![Value::Int(1)])
        .unwrap();
    assert_eq!(result, Value::Int(1));
}

/// Test recursion depth limit enforcement (T054)
#[test]
fn test_recursion_depth_limit() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: infinite(n) = infinite(n + 1)
    let params = vec![Param::new(
        Name::new("n"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // n + 1
    let n_expr = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let n_plus_1 = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr,
        op: BinOp::Add,
        rhs: one_expr,
        span: span(0, 5),
    });

    // infinite(n + 1)
    let infinite_ident = module.alloc_expr(Expr::Ident(Name::new("infinite")));
    let recursive_call = module.alloc_expr(Expr::Call {
        func: infinite_ident,
        args: vec![n_plus_1],
        span: span(0, 15),
    });

    let func = Function {
        name: Name::new("infinite"),
        params,
        return_type: None,
        body: recursive_call,
        span: span(0, 25),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Use small recursion limit for testing
    let limits = ResourceLimits {
        max_operations: 1_000_000,
        max_recursion_depth: 10,
    };

    let result =
        interpreter.execute_function_with_limits(&module, "infinite", vec![Value::Int(0)], limits);

    // Should fail with stack overflow
    assert!(result.is_err());
    match result.unwrap_err().kind() {
        RuntimeErrorKind::StackOverflow { .. } => (),
        other => panic!("Expected StackOverflow, got {:?}", other),
    }
}

/// Test deep recursion within limit
#[test]
fn test_deep_recursion_within_limit() {
    let mut module = Module::new(SourceId::new(0));

    // Create function: countdown(n) = if n <= 0 { 0 } else { countdown(n - 1) }
    let params = vec![Param::new(
        Name::new("n"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 1),
    )];

    // Condition: n <= 0
    let n_expr1 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let zero_expr = module.alloc_expr(Expr::Literal(Literal::Int(0)));
    let condition = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr1,
        op: BinOp::Le,
        rhs: zero_expr,
        span: span(0, 5),
    });

    // Then branch: 0
    let then_branch = module.alloc_expr(Expr::Literal(Literal::Int(0)));

    // Else branch: countdown(n - 1)
    let n_expr2 = module.alloc_expr(Expr::Ident(Name::new("n")));
    let one_expr = module.alloc_expr(Expr::Literal(Literal::Int(1)));
    let n_minus_1 = module.alloc_expr(Expr::BinaryOp {
        lhs: n_expr2,
        op: BinOp::Sub,
        rhs: one_expr,
        span: span(0, 5),
    });
    let countdown_ident = module.alloc_expr(Expr::Ident(Name::new("countdown")));
    let else_branch = module.alloc_expr(Expr::Call {
        func: countdown_ident,
        args: vec![n_minus_1],
        span: span(0, 15),
    });

    // If expression
    let if_expr = module.alloc_expr(Expr::If {
        condition,
        then_branch,
        else_branch: Some(else_branch),
        span: span(0, 25),
    });

    let func = Function {
        name: Name::new("countdown"),
        params,
        return_type: None,
        body: if_expr,
        span: span(0, 35),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // Use recursion limit of 100
    let limits = ResourceLimits {
        max_operations: 1_000_000,
        max_recursion_depth: 100,
    };

    // Test with 50 (within limit)
    let result = interpreter
        .execute_function_with_limits(&module, "countdown", vec![Value::Int(50)], limits.clone())
        .unwrap();
    assert_eq!(result, Value::Int(0));

    // Test with 101 (exceeds limit)
    let result = interpreter.execute_function_with_limits(
        &module,
        "countdown",
        vec![Value::Int(101)],
        limits,
    );
    assert!(result.is_err());
}
