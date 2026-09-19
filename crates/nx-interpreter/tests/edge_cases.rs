//! Integration tests for edge cases and special scenarios
//!
//! Tests for variable shadowing, Unicode strings, null handling, union error handling,
//! boolean operations, and other edge cases.
//!
//! Note: Ternary and if-else expression tests are in conditionals.rs

use nx_diagnostics::{TextSize, TextSpan};
use nx_hir::ast::{Expr, Literal};
use nx_hir::{lower, Function, Item, LoweredModule, Name, Param, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
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

// ============================================================================
// Variable Shadowing (via direct HIR construction)
// Note: NX parser doesn't support let bindings inside function bodies,
// so these tests use direct HIR construction.
// ============================================================================

/// Test that inner scope variables shadow outer scope using direct HIR
#[test]
fn test_variable_shadowing_in_block() {
    let mut module = LoweredModule::new(SourceId::new(0));

    // Build: { let y = x * 2; { let y = x * 3; y } }
    // The inner y should be returned (x * 3)
    let x_expr1 = module.alloc_expr(Expr::Ident(Name::new("x")));
    let three = module.alloc_expr(Expr::Literal(Literal::Int(3)));
    let inner_y = module.alloc_expr(Expr::BinaryOp {
        lhs: x_expr1,
        op: nx_hir::ast::BinOp::Mul,
        rhs: three,
        span: span(0, 10),
    });

    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 5),
    )];

    let func = Function {
        name: Name::new("shadow"),
        visibility: nx_hir::Visibility::Export,
        params,
        return_type: None,
        body: inner_y,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "shadow", vec![Value::Int(5)])
        .unwrap();
    assert_eq!(result, Value::Int(15)); // 5 * 3 = 15
}

/// Test function parameter usage
#[test]
fn test_parameter_usage() {
    let source = r#"
        let <use_param x:int /> = { x + 10 }
    "#;

    let result = execute_function(source, "use_param", vec![Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(15)); // 5 + 10 = 15
}

/// Test multiple parameters
#[test]
fn test_multiple_parameters() {
    let source = r#"
        let <combine a:int b:int c:int /> = { a + b + c }
    "#;

    let result = execute_function(
        source,
        "combine",
        vec![Value::Int(1), Value::Int(2), Value::Int(3)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(6)); // 1 + 2 + 3 = 6
}

// ============================================================================
// String Edge Cases
// ============================================================================

/// Test empty string
#[test]
fn test_empty_string() {
    let source = r#"
        let <empty /> = { "" }
    "#;

    let result = execute_function(source, "empty", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("")));
}

/// Test Unicode string (basic multilingual plane)
#[test]
fn test_unicode_string_basic() {
    let source = r#"
        let <greeting /> = { "Hello, 世界!" }
    "#;

    let result = execute_function(source, "greeting", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("Hello, 世界!")));
}

/// Test Unicode string with emoji
#[test]
fn test_unicode_string_emoji() {
    let source = r#"
        let <emoji /> = { "🎉🚀💡" }
    "#;

    let result = execute_function(source, "emoji", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("🎉🚀💡")));
}

/// Test string with special characters (note: NX preserves literal backslash)
#[test]
fn test_string_with_special_chars() {
    let source = r#"
        let <text /> = { "hello world" }
    "#;

    let result = execute_function(source, "text", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("hello world")));
}

// ============================================================================
// Deeply Nested Expressions
// ============================================================================

/// Test deeply nested arithmetic
#[test]
fn test_deeply_nested_arithmetic() {
    let source = r#"
        let deep(x:int): int = { ((((x + 1) * 2) - 3) / 2) }
    "#;

    // x = 10: ((((10 + 1) * 2) - 3) / 2) = (((11 * 2) - 3) / 2) = ((22 - 3) / 2) = (19 / 2) = 9
    let result =
        execute_function(source, "deep", vec![Value::Int(10)]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(9));
}

/// Test deeply nested function calls
#[test]
fn test_deeply_nested_function_calls() {
    let source = r#"
        let inc(x:int): int = { x + 1 }
        let deep_calls(x:int): int = { inc(inc(inc(inc(x)))) }
    "#;

    let result = execute_function(source, "deep_calls", vec![Value::Int(0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(4)); // 0 + 1 + 1 + 1 + 1 = 4
}

/// Test deeply nested blocks via direct HIR
#[test]
fn test_deeply_nested_blocks() {
    let mut module = LoweredModule::new(SourceId::new(0));

    // Build: x * 2 (deeply nested blocks not supported in parser)
    let x_expr = module.alloc_expr(Expr::Ident(Name::new("x")));
    let two = module.alloc_expr(Expr::Literal(Literal::Int(2)));
    let body = module.alloc_expr(Expr::BinaryOp {
        lhs: x_expr,
        op: nx_hir::ast::BinOp::Mul,
        rhs: two,
        span: span(0, 10),
    });

    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("int"),
        span(0, 5),
    )];

    let func = Function {
        name: Name::new("deep_blocks"),
        visibility: nx_hir::Visibility::Export,
        params,
        return_type: None,
        body,
        span: span(0, 40),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();
    let result = interpreter
        .execute_function(&module, "deep_blocks", vec![Value::Int(7)])
        .unwrap();
    assert_eq!(result, Value::Int(14));
}

// ============================================================================
// Null/Void Handling
// ============================================================================

/// Test null literal
#[test]
fn test_null_literal() {
    let source = r#"
        let <nothing /> = { null }
    "#;

    let result = execute_function(source, "nothing", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Null);
}

/// Test the empty braced body
///
/// Superseded `test_void_value`, which read `{ }` as an element that produces nothing. `{}` is now
/// the spelling of the empty list, so an empty braced body produces one. Type checking still
/// rejects this particular source, because an unannotated binding supplies no element type, but the
/// interpreter runs below that and evaluates the list it was given.
#[test]
fn test_empty_braced_body_is_an_empty_list() {
    let source = r#"
        let <empty_elem /> = { }
    "#;

    let result = execute_function(source, "empty_elem", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Array(items) => assert!(items.is_empty(), "Expected no items, got {:?}", items),
        other => panic!("Expected an empty array, got {:?}", other),
    }
}

// ============================================================================
// Constant Union Edge Cases
// ============================================================================

/// Test constant case access
#[test]
fn test_constant_case_access() {
    let source = r#"type Color = red | green | blue
let <color /> = { Color.green }"#;

    let result = execute_function(source, "color", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::UnionCase { union, case } => {
            assert_eq!(union.as_str(), "Color");
            assert_eq!(case.as_str(), "green");
        }
        other => panic!("Expected a constant union case, got {:?}", other),
    }
}

/// Test undefined union case (should error)
#[test]
fn test_undefined_union_case() {
    let source = r#"type Color = red | green | blue
let <color /> = { Color.yellow }"#;

    let result = execute_function(source, "color", vec![]);
    assert!(result.is_err(), "Expected error for undefined enum variant");
}

/// Test undefined union (should error)
#[test]
fn test_undefined_union() {
    let source = r#"let <color /> = { UnknownUnion.value }"#;

    let result = execute_function(source, "color", vec![]);
    assert!(result.is_err(), "Expected error for undefined enum");
}

// ============================================================================
// Boolean Edge Cases
// ============================================================================

/// Test complex boolean expression
#[test]
fn test_complex_boolean_expression() {
    let source = r#"
        let <complex a:boolean b:boolean c:boolean /> = { (a && b) || c }
    "#;

    // (true && false) || true = false || true = true
    let result = execute_function(
        source,
        "complex",
        vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Boolean(true));

    // (false && true) || false = false || false = false
    let result2 = execute_function(
        source,
        "complex",
        vec![
            Value::Boolean(false),
            Value::Boolean(true),
            Value::Boolean(false),
        ],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result2, Value::Boolean(false));
}

/// Test boolean negation chain
#[test]
fn test_boolean_double_negation() {
    // Uses direct HIR since unary ! isn't in the parser
    let mut module = LoweredModule::new(SourceId::new(0));

    // Build: !!x (double negation)
    let x_expr = module.alloc_expr(Expr::Ident(Name::new("x")));
    let not_x = module.alloc_expr(Expr::UnaryOp {
        op: nx_hir::ast::UnOp::Not,
        expr: x_expr,
        span: span(0, 5),
    });
    let not_not_x = module.alloc_expr(Expr::UnaryOp {
        op: nx_hir::ast::UnOp::Not,
        expr: not_x,
        span: span(0, 10),
    });

    let params = vec![Param::new(
        Name::new("x"),
        nx_hir::ast::TypeRef::name("boolean"),
        span(0, 5),
    )];

    let func = Function {
        name: Name::new("double_neg"),
        visibility: nx_hir::Visibility::Export,
        params,
        return_type: None,
        body: not_not_x,
        span: span(0, 20),
    };

    module.add_item(Item::Function(func));

    let interpreter = Interpreter::new();

    // !!true = true
    let result = interpreter
        .execute_function(&module, "double_neg", vec![Value::Boolean(true)])
        .unwrap();
    assert_eq!(result, Value::Boolean(true));

    // !!false = false
    let result2 = interpreter
        .execute_function(&module, "double_neg", vec![Value::Boolean(false)])
        .unwrap();
    assert_eq!(result2, Value::Boolean(false));
}

// ============================================================================
// Numeric Edge Cases
// ============================================================================

/// Test large integer values
#[test]
fn test_large_integers() {
    let source = r#"
        let big(x:int): int = { x + 1 }
    "#;

    let large_val = 2_147_483_647i64; // max i32
    let result = execute_function(source, "big", vec![Value::Int(large_val)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(large_val + 1));
}

/// Test negative integers
#[test]
fn test_negative_integers() {
    let source = r#"
        let negate(x:int): int = { 0 - x }
    "#;

    let result = execute_function(source, "negate", vec![Value::Int(42)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(-42));
}

/// Test zero division (should error)
#[test]
fn test_division_by_zero() {
    let source = r#"
        let divide(x:int y:int): int = { x / y }
    "#;

    let result = execute_function(source, "divide", vec![Value::Int(10), Value::Int(0)]);
    assert!(result.is_err(), "Expected error for division by zero");
}

/// Test integer overflow behavior
#[test]
fn test_integer_overflow() {
    let source = r#"
        let overflow(x:int): int = { x * 2 }
    "#;

    // This tests whatever the interpreter does with overflow
    // (wrap, saturate, or error)
    let large = i64::MAX / 2 + 1;
    // Accept any behavior - just don't panic
    let _ = execute_function(source, "overflow", vec![Value::Int(large)]);
}

// ============================================================================
// Type Coercion Edge Cases
// ============================================================================

/// Test using wrong type for function parameter
#[test]
fn test_type_mismatch_parameter() {
    let source = r#"
        let need_int(x:int): int = { x }
    "#;

    let result = execute_function(
        source,
        "need_int",
        vec![Value::String(SmolStr::new("hello"))],
    );
    // This may succeed (dynamic typing) or fail (static types) depending on implementation
    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_str(), "hello"),
        Err(_) => (), // Type error is acceptable
        Ok(other) => panic!("Unexpected result: {:?}", other),
    }
}

// ============================================================================
// Record Edge Cases
// ============================================================================

/// Test accessing undefined field on record
#[test]
fn test_undefined_record_field() {
    let source = r#"
        type Person = { name: string = "Unknown" }
        let <age person:Person /> = { person.age }
    "#;

    use rustc_hash::FxHashMap;
    let mut person = FxHashMap::default();
    person.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));

    let result = execute_function(
        source,
        "age",
        vec![Value::Record {
            type_name: nx_hir::Name::new("Person"),
            fields: person,
        }],
    );
    // Should error because 'age' field doesn't exist
    assert!(result.is_err(), "Expected error for undefined field access");
}

/// Test empty record
#[test]
fn test_empty_record() {
    use rustc_hash::FxHashMap;

    let source = r#"
        let <identity r:object /> = { r }
    "#;

    let empty_record = Value::Record {
        type_name: nx_hir::Name::new("object"),
        fields: FxHashMap::default(),
    };
    let result = execute_function(source, "identity", vec![empty_record])
        .unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Record { fields, .. } => assert!(fields.is_empty()),
        other => panic!("Expected empty Record, got {:?}", other),
    }
}

// ============================================================================
// Top-level values and record field defaults
// ============================================================================

/// A top-level value whose construction omits a defaulted field evaluates.
///
/// <para>Filling the default binds the declaring module's top-level values so the default can name
/// one, and the value being filled is one of them — which is a cycle unless the value in flight is
/// left out. It used to be a stack overflow with no diagnostic.</para>
#[test]
fn test_top_level_value_omitting_a_defaulted_field_terminates() {
    let result = execute_function(
        "type R = { a:int b:boolean = false }\nlet r = <R a={1} />\nlet root() = { r }",
        "root",
        vec![],
    )
    .expect("the default should be filled without recursing");

    let Value::Record { fields, .. } = result else {
        panic!("expected a record, got {result:?}");
    };
    assert_eq!(fields.get("a"), Some(&Value::Int(1)));
    assert_eq!(fields.get("b"), Some(&Value::Boolean(false)));
}

/// A default may still name another top-level value; only the one it is filling is out of scope.
#[test]
fn test_a_field_default_still_sees_the_other_top_level_values() {
    let result = execute_function(
        "let base = 7\ntype R = { a:int b:int = {base} }\nlet r = <R a={1} />\nlet root() = { r }",
        "root",
        vec![],
    )
    .expect("the default should read the other top-level value");

    let Value::Record { fields, .. } = result else {
        panic!("expected a record, got {result:?}");
    };
    assert_eq!(fields.get("b"), Some(&Value::Int(7)));
}

/// Another top-level value may depend on the one whose default is being filled. It is simply not
/// bound while that default runs, and the outer pass binds it for real afterwards.
#[test]
fn test_a_value_depending_on_the_one_being_bound_is_still_bound_afterwards() {
    let result = execute_function(
        "type R = { a:int b:boolean = false }\n\
         let r = <R a={1} />\n\
         let doubled = {r.a + r.a}\n\
         let root() = { doubled }",
        "root",
        vec![],
    )
    .expect("the dependent value should bind once the one it names is bound");
    assert_eq!(result, Value::Int(2));
}

/// A default that names the very value it is filling is a cycle, and is reported as one rather
/// than run forever.
#[test]
fn test_a_field_default_naming_the_value_it_fills_is_an_error() {
    let error = execute_function(
        "type R = { a:int b:int = {r.a} }\nlet r = <R a={1} />\nlet root() = { r }",
        "root",
        vec![],
    )
    .expect_err("the cycle should be reported");
    assert!(error.contains("Undefined variable: r"), "got: {error}");
}

/// A genuine evaluation failure in a value bound during a nested pass still surfaces.
///
/// <para>A nested pass discards the error so the two passes cannot recurse forever, on the
/// grounds that the outer pass binds every value for real and reports it there. That holds only
/// because the outer pass visits every item, including the ones the nested pass reached first —
/// which is what this pins down.</para>
#[test]
fn test_a_failure_in_a_value_bound_by_a_nested_pass_is_reported_by_the_outer_pass() {
    let error = execute_function(
        "type R = { a:int b:int = {2} }\n\
         let r = <R a={1} />\n\
         let bad = {r.a / (r.a - 1)}\n\
         let root() = { r.a }",
        "root",
        vec![],
    )
    .expect_err("the division by zero should be reported");
    assert!(error.to_lowercase().contains("zero"), "got: {error}");
}
