//! Integration tests for for loop expressions
//!
//! Tests T047-T051: Loop execution

use crate::support::source::{IrEvaluator, Value};
use nx_hir::{lower, SourceId};
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

    let interpreter = IrEvaluator::new(source);
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

// ============================================================================
// Additional For Loop Tests
// ============================================================================

/// Test for loop with string array (using parsed NX)
#[test]
fn test_for_loop_string_array() {
    let source = r#"
        let <process items:object+ /> = {
            for item in items { item }
        }
    "#;

    let input = Value::Array(vec![
        Value::String(SmolStr::new("hello")),
        Value::String(SmolStr::new("world")),
        Value::String(SmolStr::new("!")),
    ]);
    let result =
        execute_function(source, "process", vec![input]).unwrap_or_else(|e| panic!("{}", e));

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
        let <names people:Person+ /> = {
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

/// Test for loop combining item and index in calculation
#[test]
fn test_for_loop_item_plus_index() {
    let source = r#"
        let <add_index items:int+ /> = {
            for item, index in items { item + index }
        }
    "#;

    let input = Value::Array(vec![Value::Int(100), Value::Int(200), Value::Int(300)]);
    let result =
        execute_function(source, "add_index", vec![input]).unwrap_or_else(|e| panic!("{}", e));

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
        let <triple items:int+ /> = {
            for item in items { item * 3 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(5)]);
    let result =
        execute_function(source, "triple", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(3), Value::Int(6), Value::Int(15)])
    );
}

/// Test for loop with subtraction
#[test]
fn test_for_loop_subtract() {
    let source = r#"
        let <decrement items:int+ /> = {
            for item in items { item - 1 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(10), Value::Int(5), Value::Int(1)]);
    let result =
        execute_function(source, "decrement", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![Value::Int(9), Value::Int(4), Value::Int(0)])
    );
}

/// Test for loop with boolean array
#[test]
fn test_for_loop_boolean_array() {
    let source = r#"
        let <identity items:object+ /> = {
            for item in items { item }
        }
    "#;

    let input = Value::Array(vec![
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Boolean(true),
    ]);
    let result =
        execute_function(source, "identity", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(
        result,
        Value::Array(vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::Boolean(true),
        ])
    );
}

/// Test for loop with single element array
#[test]
fn test_for_loop_single_element() {
    let source = r#"
        let <double items:int+ /> = {
            for item in items { item * 2 }
        }
    "#;

    let input = Value::Array(vec![Value::Int(42)]);
    let result =
        execute_function(source, "double", vec![input]).unwrap_or_else(|e| panic!("{}", e));

    assert_eq!(result, Value::Array(vec![Value::Int(84)]));
}

/// Test for loop preserves order
#[test]
fn test_for_loop_preserves_order() {
    let source = r#"
        let <identity items:object+ /> = {
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
    let result =
        execute_function(source, "identity", vec![input]).unwrap_or_else(|e| panic!("{}", e));

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
