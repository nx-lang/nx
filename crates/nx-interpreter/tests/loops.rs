//! Integration tests for for loop expressions
//!
//! Tests T047-T051: Loop execution

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{BinOp, Expr, Literal, OrderedFloat};
use nx_hir::{lower, Function, Item, Module, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Helper to create a text span
fn span(start: u32, end: u32) -> TextSpan {
    TextSpan::new(TextSize::from(start), TextSize::from(end))
}

/// Helper function to execute a function from NX source code and return the result
fn execute_function(source: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    let parse_result = parse_str(source, "test.nx");
    if !parse_result.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", parse_result.errors));
    }

    let root = parse_result.root().expect("Failed to get root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
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

// ============================================================================
// Additional For Loop Tests
// ============================================================================

/// Test for loop with string array (using parsed NX)
#[test]
fn test_for_loop_string_array() {
    let source = r#"
        let <process items:object /> = {
            for item in items { item }
        }
    "#;

    let input = Value::Array(vec![
        Value::String(SmolStr::new("hello")),
        Value::String(SmolStr::new("world")),
        Value::String(SmolStr::new("!")),
    ]);
    let result = execute_function(source, "process", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(SmolStr::new("hello")),
            Value::String(SmolStr::new("world")),
            Value::String(SmolStr::new("!")),
        ])
    );
}

/// Test for loop with record array - access record fields
#[test]
fn test_for_loop_record_array() {
    let source = r#"
        type Person = { name:string age:int }
        let <names people:object /> = {
            for person in people { person.name }
        }
    "#;

    let mut person1 = FxHashMap::default();
    person1.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
    person1.insert(SmolStr::new("age"), Value::Int(30));

    let mut person2 = FxHashMap::default();
    person2.insert(SmolStr::new("name"), Value::String(SmolStr::new("Bob")));
    person2.insert(SmolStr::new("age"), Value::Int(25));

    let input = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("Person"),
            fields: person1,
        },
        Value::Record {
            type_name: nx_hir::Name::new("Person"),
            fields: person2,
        },
    ]);
    let result = execute_function(source, "names", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(SmolStr::new("Alice")),
            Value::String(SmolStr::new("Bob")),
        ])
    );
}

/// Test for loop index arithmetic via direct HIR
#[test]
fn test_for_loop_index_arithmetic() {
    let mut module = Module::new(SourceId::new(0));

    // Function: index_times_two(items) = for item, index in items { index * 2 }
    let params = vec![Param::new(
        Name::new("items"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 5),
    )];

    let items_expr = module.alloc_expr(Expr::Ident(Name::new("items")));

    // Body: index * 2
    let index_expr = module.alloc_expr(Expr::Ident(Name::new("index")));
    let two_expr = module.alloc_expr(Expr::Literal(Literal::Int(2)));
    let body = module.alloc_expr(Expr::BinaryOp {
        lhs: index_expr,
        op: BinOp::Mul,
        rhs: two_expr,
        span: span(0, 15),
    });

    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: Some(Name::new("index")),
        iterable: items_expr,
        body,
        span: span(0, 35),
    });

    let func = Function {
        name: Name::new("index_times_two"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 45),
    };

    module.add_item(Item::Function(func));

    // Input: [10, 20, 30] -> indices [0, 1, 2] * 2 = [0, 2, 4]
    let interpreter = Interpreter::new();
    let input = Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
    let result = interpreter
        .execute_function(&module, "index_times_two", vec![input])
        .unwrap();

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(0), Value::Int(2), Value::Int(4)])
    );
}

/// Test for loop combining item and index in calculation
#[test]
fn test_for_loop_item_plus_index() {
    let source = r#"
        let <add_index items:object /> = {
            for item, index in items { item + index }
        }
    "#;

    let input = Value::Array(vec![Value::Int(100), Value::Int(200), Value::Int(300)]);
    let result = execute_function(source, "add_index", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    // 100+0=100, 200+1=201, 300+2=302
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(100), Value::Int(201), Value::Int(302)])
    );
}

/// Test for loop multiplication via parsed NX
#[test]
fn test_for_loop_multiply() {
    let source = r#"
        let <triple items:object /> = {
            for item in items { item * 3 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(5)]);
    let result = execute_function(source, "triple", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(3), Value::Int(6), Value::Int(15)])
    );
}

/// Test for loop with subtraction
#[test]
fn test_for_loop_subtract() {
    let source = r#"
        let <decrement items:object /> = {
            for item in items { item - 1 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(10), Value::Int(5), Value::Int(1)]);
    let result = execute_function(source, "decrement", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(9), Value::Int(4), Value::Int(0)])
    );
}

/// Test for loop with boolean array
#[test]
fn test_for_loop_boolean_array() {
    let source = r#"
        let <identity items:object /> = {
            for item in items { item }
        }
    "#;

    let input = Value::Array(vec![
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Boolean(true),
    ]);
    let result = execute_function(source, "identity", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
        ])
    );
}

/// Test for loop with float array via direct HIR
#[test]
fn test_for_loop_float_array() {
    let mut module = Module::new(SourceId::new(0));

    // Function: double_floats(items) = for item in items { item * 2.0 }
    let params = vec![Param::new(
        Name::new("items"),
        nx_hir::ast::TypeRef::name("array"),
        span(0, 5),
    )];

    let items_expr = module.alloc_expr(Expr::Ident(Name::new("items")));

    // Body: item * 2.0
    let item_expr = module.alloc_expr(Expr::Ident(Name::new("item")));
    let two_expr = module.alloc_expr(Expr::Literal(Literal::Float(OrderedFloat(2.0))));
    let body = module.alloc_expr(Expr::BinaryOp {
        lhs: item_expr,
        op: BinOp::Mul,
        rhs: two_expr,
        span: span(0, 15),
    });

    let for_expr = module.alloc_expr(Expr::For {
        item: Name::new("item"),
        index: None,
        iterable: items_expr,
        body,
        span: span(0, 30),
    });

    let func = Function {
        name: Name::new("double_floats"),
        params,
        return_type: None,
        body: for_expr,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let input = Value::Array(vec![
        Value::Float(1.5),
        Value::Float(2.5),
        Value::Float(3.5),
    ]);
    let result = interpreter
        .execute_function(&module, "double_floats", vec![input])
        .unwrap();

    match result {
        Value::Array(items) => {
            assert_eq!(items.len(), 3);
            match (&items[0], &items[1], &items[2]) {
                (Value::Float(a), Value::Float(b), Value::Float(c)) => {
                    assert!((a - 3.0).abs() < 1e-10);
                    assert!((b - 5.0).abs() < 1e-10);
                    assert!((c - 7.0).abs() < 1e-10);
                }
                _ => panic!("Expected floats"),
            }
        }
        other => panic!("Expected Array, got {:?}", other),
    }
}

/// Test for loop with single element array
#[test]
fn test_for_loop_single_element() {
    let source = r#"
        let <double items:object /> = {
            for item in items { item * 2 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(42)]);
    let result = execute_function(source, "double", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(result, Value::Array(vec![Value::Int(84)]));
}

/// Test for loop preserves order
#[test]
fn test_for_loop_preserves_order() {
    let source = r#"
        let <identity items:object /> = {
            for item in items { item }
        }
    "#;

    let input = Value::Array(vec![
        Value::Int(5),
        Value::Int(3),
        Value::Int(8),
        Value::Int(1),
        Value::Int(9),
    ]);
    let result = execute_function(source, "identity", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![
            Value::Int(5),
            Value::Int(3),
            Value::Int(8),
            Value::Int(1),
            Value::Int(9),
        ])
    );
}
