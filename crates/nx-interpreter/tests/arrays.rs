//! Integration tests for array operations
//!
//! Tests for array literals, empty arrays, nested arrays, and arrays of different types.
//!
//! Note: Array literal syntax [1, 2, 3] is not yet supported in the parser.
//! Tests using array literals are marked #[ignore] until parser support is added.
//!
//! Note: NX does not currently support array indexing (arr[0]), only iteration.

use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

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
// Array Literals (ignored until parser support is added)
// ============================================================================

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_literal_integers() {
    let source = r#"
        let <arr /> = { [1, 2, 3] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_literal_single_element() {
    let source = r#"
        let <arr /> = { [42] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Array(vec![Value::Int(42)]));
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_literal_strings() {
    let source = r#"
        let <arr /> = { ["hello", "world"] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(SmolStr::new("hello")),
            Value::String(SmolStr::new("world"))
        ])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_literal_booleans() {
    let source = r#"
        let <arr /> = { [true, false, true] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true)
        ])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_literal_floats() {
    let source = r#"
        let <arr /> = { [1.5, 2.5, 3.5] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    match result {
        Value::Array(items) => {
            assert_eq!(items.len(), 3);
            match (&items[0], &items[1], &items[2]) {
                (Value::Float(a), Value::Float(b), Value::Float(c)) => {
                    assert!((a - 1.5).abs() < 1e-10);
                    assert!((b - 2.5).abs() < 1e-10);
                    assert!((c - 3.5).abs() < 1e-10);
                }
                _ => panic!("Expected floats, got {:?}", items),
            }
        }
        other => panic!("Expected Array, got {:?}", other),
    }
}

// ============================================================================
// Empty Arrays (ignored until parser support is added)
// ============================================================================

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_empty_array() {
    let source = r#"
        let <arr /> = { [] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Array(vec![]));
}

// ============================================================================
// Braced Call Arguments
// ============================================================================

/// A braced argument reaches the parameter as a list, at each arity.
///
/// The one-item case is the interesting one: `{"only"}` is a scalar, and it becomes a one-element
/// list by the same parameter coercion a property binding uses, not by anything in the brace.
#[test]
fn test_braced_call_arguments_arrive_as_lists() {
    let source = r#"
        let echo(xs:string[]): string[] = {xs}
        let <none /> = { echo({}) }
        let <one /> = { echo({"only"}) }
        let <two /> = { echo({"a" "b"}) }
    "#;

    let none = execute_function(source, "none", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(none, Value::Array(vec![]));

    let one = execute_function(source, "one", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(one, Value::Array(vec![Value::String(SmolStr::new("only"))]));

    let two = execute_function(source, "two", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        two,
        Value::Array(vec![
            Value::String(SmolStr::new("a")),
            Value::String(SmolStr::new("b")),
        ])
    );
}

// ============================================================================
// Empty Body Content
// ============================================================================

/// Returns the value bound to `field` in the record a test function returns.
fn record_field(source: &str, function_name: &str, field: &str) -> Value {
    let value = execute_function(source, function_name, vec![]).unwrap_or_else(|e| panic!("{}", e));
    match value {
        Value::Record { fields, .. } => fields
            .get(&SmolStr::new(field))
            .unwrap_or_else(|| panic!("no field {field:?} in {fields:?}"))
            .clone(),
        other => panic!("expected a record, got {other:?}"),
    }
}

/// A body that was written and produced nothing binds the empty list.
///
/// The distinction is between a body and no body: `<Box />` leaves the content property to its
/// default, while `<Box>{}</Box>` says the content is empty. Collapsing the two loses the value --
/// the property falls through to null and then fails to coerce at a non-nullable list field.
#[test]
fn test_empty_body_content_binds_the_empty_list() {
    let source = r#"
        type Box = { content items: string[] }
        let <empty /> = { <Box>{}</Box> }
    "#;

    assert_eq!(record_field(source, "empty", "items"), Value::Array(vec![]));
}

/// At a nullable list field the empty body is still the empty list, not the absence of one.
///
/// This is the `T[]?` versus `T?[]` rule the typing requirement states for property position,
/// which body content must not contradict: it would type check and then mean something else.
#[test]
fn test_empty_body_content_at_a_nullable_list_field_is_not_null() {
    let source = r#"
        type Box = { content items: string[]? }
        let <empty /> = { <Box>{}</Box> }
    "#;

    assert_eq!(record_field(source, "empty", "items"), Value::Array(vec![]));
}

/// An element with no body at all still leaves the content property alone.
#[test]
fn test_absent_body_content_leaves_the_content_property_to_its_default() {
    let source = r#"
        type Box = { content items: string[] = {"d"} }
        let <bare /> = { <Box /> }
    "#;

    assert_eq!(
        record_field(source, "bare", "items"),
        Value::Array(vec![Value::String(SmolStr::new("d"))])
    );
}

/// The rule is about the body, not about `{}`.
///
/// A `for` that iterates zero times produces no values just as `{}` does, and binds no children
/// rather than falling back to the declared default. This source contains no `{}` at all, so it is
/// the case that pins what the distinction is drawn on -- and the one program shape in this change
/// that means something different than it did.
#[test]
fn test_a_body_that_produced_nothing_binds_the_empty_list_over_a_default() {
    let source = r#"
        type A = { n: int = 1 }
        type Box = { content items: object[] = {<A n=9 />} }
        let empty:string[] = {}
        let <ran /> = { <Box>for x in empty { <A n=2 /> }</Box> }
    "#;

    assert_eq!(record_field(source, "ran", "items"), Value::Array(vec![]));
}

// ============================================================================
// Nested Arrays (ignored until parser support is added)
// ============================================================================

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_nested_array_simple() {
    let source = r#"
        let <arr /> = { [[1, 2], [3, 4]] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
            Value::Array(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_nested_array_mixed_sizes() {
    let source = r#"
        let <arr /> = { [[1], [2, 3], [4, 5, 6]] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::Array(vec![Value::Int(1)]),
            Value::Array(vec![Value::Int(2), Value::Int(3)]),
            Value::Array(vec![Value::Int(4), Value::Int(5), Value::Int(6)])
        ])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_deeply_nested_array() {
    let source = r#"
        let <arr /> = { [[[1]]] }
    "#;

    let result = execute_function(source, "arr", vec![]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![Value::Array(vec![Value::Array(vec![Value::Int(1)])])])
    );
}

// ============================================================================
// Arrays with Expressions (ignored until parser support is added)
// ============================================================================

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_with_expressions() {
    let source = r#"
        let <arr a:int b:int /> = { [a, b, a + b] }
    "#;

    let result = execute_function(source, "arr", vec![Value::Int(3), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(3), Value::Int(5), Value::Int(8)])
    );
}

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_array_with_function_calls() {
    let source = r#"
        let double(x:int): int = { x * 2 }
        let <arr n:int /> = { [n, double(n), double(double(n))] }
    "#;

    let result =
        execute_function(source, "arr", vec![Value::Int(2)]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(2), Value::Int(4), Value::Int(8)])
    );
}

// ============================================================================
// Arrays as Function Arguments
// ============================================================================

#[test]
fn test_array_as_argument() {
    let source = r#"
        let <first arr:object /> = { 
            for item in arr { item }
        }
    "#;

    let arr = Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
    let result = execute_function(source, "first", vec![arr]).unwrap_or_else(|e| panic!("{}", e));
    // For loop returns an array of results
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

#[test]
fn test_array_of_strings_as_argument() {
    let source = r#"
        let <identity arr:object /> = { 
            for item in arr { item }
        }
    "#;

    let arr = Value::Array(vec![
        Value::String(SmolStr::new("a")),
        Value::String(SmolStr::new("b")),
        Value::String(SmolStr::new("c")),
    ]);
    let result =
        execute_function(source, "identity", vec![arr]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(SmolStr::new("a")),
            Value::String(SmolStr::new("b")),
            Value::String(SmolStr::new("c"))
        ])
    );
}

// ============================================================================
// Arrays of Records
// ============================================================================

#[test]
fn test_array_of_records() {
    let source = r#"
        type User = { name: string = "Anon" }
        let <names users:object /> = { 
            for user in users { user.name }
        }
    "#;

    let mut user1 = FxHashMap::default();
    user1.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
    let mut user2 = FxHashMap::default();
    user2.insert(SmolStr::new("name"), Value::String(SmolStr::new("Bob")));

    let users = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: user1,
        },
        Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: user2,
        },
    ]);
    let result = execute_function(source, "names", vec![users]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![
            Value::String(SmolStr::new("Alice")),
            Value::String(SmolStr::new("Bob"))
        ])
    );
}

// ============================================================================
// Array Concatenation (via for loops)
// ============================================================================

#[test]
fn test_transform_array() {
    let source = r#"
        let <doubled arr:object /> = { 
            for x in arr { x * 2 }
        }
    "#;

    let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let result = execute_function(source, "doubled", vec![arr]).unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(
        result,
        Value::Array(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
    );
}

// ============================================================================
// Mixed Type Arrays (ignored until parser support is added)
// ============================================================================

#[test]
#[ignore = "array literal syntax not yet supported in parser"]
fn test_mixed_type_array() {
    // NX might allow or disallow mixed types - test current behavior
    let source = r#"
        let <arr /> = { [1, "two", true] }
    "#;

    let result = execute_function(source, "arr", vec![]);
    // Accept either success with mixed types or a type error
    match result {
        Ok(Value::Array(items)) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[1], Value::String(SmolStr::new("two")));
            assert_eq!(items[2], Value::Boolean(true));
        }
        Err(_) => {
            // Type error is acceptable if NX enforces homogeneous arrays
        }
        Ok(other) => panic!("Unexpected result: {:?}", other),
    }
}
