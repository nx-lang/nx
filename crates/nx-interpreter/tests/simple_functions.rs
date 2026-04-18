//! Integration tests for simple function execution (User Story 1)
//!
//! These tests verify end-to-end execution of NX functions with basic operations.

use nx_diagnostics::render_diagnostics_cli;
use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::collections::HashMap;

/// Helper function to execute a function and return the result
fn execute_function(source: &str, function_name: &str, args: Vec<Value>) -> Result<Value, String> {
    // Parse the source code
    let parse_result = parse_str(source, "test.nx");
    if !parse_result.errors.is_empty() {
        let mut sources = HashMap::new();
        sources.insert("test.nx".to_string(), source.to_string());
        let rendered = render_diagnostics_cli(&parse_result.errors, &sources);
        return Err(rendered);
    }

    // Lower to HIR
    let root = parse_result.root().expect("Failed to get root");
    let module = lower(root, SourceId::new(0));

    // Execute the function
    let interpreter = Interpreter::new();
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

// ============================================================================
// T023: Arithmetic Function Tests
// ============================================================================

#[test]
fn test_add_function() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(5), Value::Int(3)])
        .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    assert_eq!(result, Value::Int(8));
}

#[test]
fn test_subtract_function() {
    let source = r#"
        let <sub a:int b:int /> = { a - b }
    "#;

    let result = execute_function(source, "sub", vec![Value::Int(10), Value::Int(3)])
        .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_multiply_function() {
    let source = r#"
        let <mul a:int b:int /> = { a * b }
    "#;

    let result = execute_function(source, "mul", vec![Value::Int(4), Value::Int(5)])
        .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_divide_function() {
    let source = r#"
        let <div a:int b:int /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(15), Value::Int(3)])
        .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_mixed_arithmetic() {
    let source = r#"
        let <calc a:int b:int c:int /> = { a + b * c }
    "#;

    let result = execute_function(
        source,
        "calc",
        vec![Value::Int(2), Value::Int(3), Value::Int(4)],
    )
    .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    // 2 + (3 * 4) = 2 + 12 = 14
    assert_eq!(result, Value::Int(14));
}

// ============================================================================
// T024: String Concatenation Tests
// ============================================================================

#[test]
fn test_string_concat() {
    let source = r#"
        let <concat a:string b:string /> = { a + b }
    "#;

    let result = execute_function(
        source,
        "concat",
        vec![
            Value::String(SmolStr::new("hello")),
            Value::String(SmolStr::new(" world")),
        ],
    )
    .unwrap_or_else(|err| panic!("Function execution failed:\n{}", err));
    assert_eq!(result, Value::String(SmolStr::new("hello world")));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_function_not_found() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "nonexistent", vec![]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Function not found"));
}

#[test]
fn test_parameter_count_mismatch() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(5)]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parameter"));
}

#[test]
fn test_division_by_zero() {
    let source = r#"
        let <div a:int b:int /> = { a / b }
    "#;

    let result = execute_function(source, "div", vec![Value::Int(10), Value::Int(0)]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Division by zero"));
}

// ============================================================================
// Paren Function Tests
// ============================================================================

#[test]
fn test_paren_function_execution() {
    let source = r#"
        let add(a:int, b:int): int = { a + b }
    "#;

    let result = execute_function(source, "add", vec![Value::Int(2), Value::Int(6)])
        .unwrap_or_else(|err| panic!("Paren function execution failed:\n{}", err));
    assert_eq!(result, Value::Int(8));
}

#[test]
fn test_nested_paren_function_calls() {
    let source = r#"
        let add(a:int, b:int): int = { a + b }
        let double(value:int): int = { add(value, value) }
        let compute(n:int): int = { double(add(n, 1)) }
    "#;

    let result = execute_function(source, "compute", vec![Value::Int(3)])
        .unwrap_or_else(|err| panic!("Nested paren call failed:\n{}", err));
    assert_eq!(result, Value::Int(8));
}

#[test]
fn test_paren_function_without_return_annotation() {
    let source = r#"
        let sum(a:int, b:int) = { a + b }
        let apply(n:int) = { sum(n, 1) }
    "#;

    let result = execute_function(source, "apply", vec![Value::Int(9)])
        .unwrap_or_else(|err| panic!("Paren function without annotation failed:\n{}", err));
    assert_eq!(result, Value::Int(10));
}

// ============================================================================
// Enum Support Tests
// ============================================================================

#[test]
fn test_enum_member_return() {
    let source = r#"
        enum Direction = | north | south | east | west
        let <north /> = { Direction.north }
    "#;

    let result = execute_function(source, "north", vec![]).unwrap_or_else(|err| panic!("{}", err));
    assert_eq!(
        result,
        Value::EnumValue {
            type_name: nx_hir::Name::new("Direction"),
            member: SmolStr::new("north")
        }
    );
}

#[test]
fn test_enum_comparison() {
    let source = r#"
        enum Direction = | north | south | east | west
        let isNorth(value:Direction): bool = { value == Direction.north }
    "#;

    let result = execute_function(
        source,
        "isNorth",
        vec![Value::EnumValue {
            type_name: nx_hir::Name::new("Direction"),
            member: SmolStr::new("north"),
        }],
    )
    .unwrap_or_else(|err| panic!("{}", err));
    assert_eq!(result, Value::Boolean(true));
}

// ============================================================================
// Record Support Tests
// ============================================================================

#[test]
fn test_record_field_access() {
    let source = r#"
        type User = { name: string age: int }
        let getName(user:User): string = { user.name }
    "#;

    let mut record = FxHashMap::default();
    record.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));
    record.insert(SmolStr::new("age"), Value::Int(32));

    let result = execute_function(
        source,
        "getName",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: record,
        }],
    )
    .unwrap_or_else(|err| {
        panic!("Record field access failed:\n{}", err);
    });

    assert_eq!(result, Value::String(SmolStr::new("Ada")));
}

#[test]
fn test_derived_record_argument_satisfies_abstract_ancestor_parameter() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        abstract type UserBase extends Entity = {
          name: string
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }

        let consume(entity:Entity): int = { 1 }
    "#;

    let mut record = FxHashMap::default();
    record.insert(SmolStr::new("id"), Value::Int(1));
    record.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));

    let result = execute_function(
        source,
        "consume",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: record,
        }],
    )
    .unwrap_or_else(|err| panic!("derived record argument failed:\n{}", err));

    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_derived_record_return_satisfies_abstract_ancestor_return_type() {
    let source = r#"
        abstract type Entity = {
          id: int
        }

        abstract type UserBase extends Entity = {
          name: string
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }

        let make(): UserBase = { <User id={1} name={"Ada"} /> }
    "#;

    let result = execute_function(source, "make", vec![])
        .unwrap_or_else(|err| panic!("derived record return failed:\n{}", err));

    match result {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "User");
            assert_eq!(fields.get("id"), Some(&Value::Int(1)));
            assert_eq!(
                fields.get("name"),
                Some(&Value::String(SmolStr::new("Ada")))
            );
        }
        other => panic!("Expected derived record return, got {:?}", other),
    }
}

#[test]
fn test_record_missing_field_errors() {
    let source = r#"
        type User = { name: string }
        let missing(user:User) = { user.email }
    "#;

    let mut record = FxHashMap::default();
    record.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));

    let result = execute_function(
        source,
        "missing",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: record,
        }],
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("no field"),
        "Expected missing field error"
    );
}

#[test]
fn test_host_supplied_record_defaults_are_not_applied_during_argument_coercion() {
    let source = r#"
        type User = { name: string = "Anon" age: int }
        let echo(user:User) = { user }
    "#;

    let mut record = FxHashMap::default();
    record.insert(SmolStr::new("age"), Value::Int(32));

    let result = execute_function(
        source,
        "echo",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: record,
        }],
    )
    .unwrap_or_else(|err| panic!("host-supplied record should pass through:\n{}", err));

    match result {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "User");
            assert_eq!(fields.get("age"), Some(&Value::Int(32)));
            assert!(!fields.contains_key("name"));
        }
        other => panic!("expected Record, got {:?}", other),
    }
}

#[test]
fn test_host_supplied_nullable_record_field_remains_absent() {
    let source = r#"
        type User = { email: string? }
        let getEmail(user:User): string? = { user.email }
    "#;

    let result = execute_function(
        source,
        "getEmail",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: FxHashMap::default(),
        }],
    );

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Record 'user' has no field named 'email'"),
        "Expected missing optional field to remain absent on external records"
    );
}

#[test]
fn test_host_supplied_record_missing_required_field_does_not_fail_before_body() {
    let source = r#"
        type User = { name: string age: int }
        let noop(user:User): int = { 0 }
    "#;

    let mut record = FxHashMap::default();
    record.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));

    let result = execute_function(
        source,
        "noop",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: record,
        }],
    );
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_record_defaults_instantiation() {
    let source = r#"
        type User = { name: string = "Anon" age: int = 30 }
        let greet(user:User): string = { user.name }
    "#;

    // Build module
    let parse_result = parse_str(source, "record-defaults.nx");
    assert!(
        parse_result.errors.is_empty(),
        "Parse errors: {:?}",
        parse_result.errors
    );
    let root = parse_result.root().expect("root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    let record = interpreter
        .instantiate_record_defaults(&module, "User")
        .expect("instantiate record");

    let result = interpreter
        .execute_function(&module, "greet", vec![record])
        .unwrap_or_else(|err| panic!("Execution failed: {}", err));

    assert_eq!(result, Value::String(SmolStr::new("Anon")));
}

#[test]
fn test_record_inheritance_applies_inherited_defaults() {
    let source = r#"
        abstract type UserBase = {
          name: string = "Anon"
          age: int = 18
        }

        type User extends UserBase = {
          isAdmin: bool = false
        }
    "#;

    let parse_result = parse_str(source, "record-inheritance-defaults.nx");
    assert!(
        parse_result.errors.is_empty(),
        "Parse errors: {:?}",
        parse_result.errors
    );
    let root = parse_result.root().expect("root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    let record = interpreter
        .instantiate_record_defaults(&module, "User")
        .expect("instantiate inherited defaults");

    match record {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "User");
            assert_eq!(
                fields.get("name"),
                Some(&Value::String(SmolStr::new("Anon")))
            );
            assert_eq!(fields.get("age"), Some(&Value::Int(18)));
            assert_eq!(fields.get("isAdmin"), Some(&Value::Boolean(false)));
        }
        other => panic!(
            "Expected inherited defaults to produce a record, got {:?}",
            other
        ),
    }
}

#[test]
fn test_action_inheritance_applies_inherited_defaults() {
    let source = r#"
        abstract action InputAction = {
          source: string = "ui"
        }

        action SearchSubmitted extends InputAction = {
          query: string = "search"
        }
    "#;

    let parse_result = parse_str(source, "action-inheritance-defaults.nx");
    assert!(
        parse_result.errors.is_empty(),
        "Parse errors: {:?}",
        parse_result.errors
    );
    let root = parse_result.root().expect("root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    let action = interpreter
        .instantiate_record_defaults(&module, "SearchSubmitted")
        .expect("instantiate inherited action defaults");

    match action {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "SearchSubmitted");
            assert_eq!(
                fields.get("source"),
                Some(&Value::String(SmolStr::new("ui")))
            );
        }
        other => panic!(
            "Expected inherited defaults to produce an action record, got {:?}",
            other
        ),
    }
}

#[test]
fn test_record_literal_defaults_and_overrides() {
    let source = r#"
        type User = { name: string = "Anon" age: int = 30 }
        let getName(): string = { <User name="Bob" />.name }
        let getAge(): int = { <User name="Bob" />.age }
    "#;

    let result = execute_function(source, "getName", vec![])
        .unwrap_or_else(|err| panic!("record literal name failed: {}", err));
    assert_eq!(result, Value::String(SmolStr::new("Bob")));

    let result = execute_function(source, "getAge", vec![])
        .unwrap_or_else(|err| panic!("record literal age failed: {}", err));
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_action_record_literal_uses_defaults() {
    let source = r#"
        action SaveRequested = { value: string = "Anon" source: string? }
        let getValue(): string = { <SaveRequested />.value }
    "#;

    let result = execute_function(source, "getValue", vec![])
        .unwrap_or_else(|err| panic!("action record literal failed: {}", err));
    assert_eq!(result, Value::String(SmolStr::new("Anon")));
}

#[test]
fn test_action_record_defaults_instantiation() {
    let source = r#"
        action SaveRequested = { value: string = "Anon" source: string? }
        let getValue(): string = { <SaveRequested />.value }
    "#;

    let parse_result = parse_str(source, "action-records.nx");
    assert!(
        parse_result.errors.is_empty(),
        "Parse errors: {:?}",
        parse_result.errors
    );
    let root = parse_result.root().expect("root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    let action = interpreter
        .instantiate_record_defaults(&module, "SaveRequested")
        .expect("instantiate action defaults");

    match action {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "SaveRequested");
            assert_eq!(
                fields.get("value"),
                Some(&Value::String(SmolStr::new("Anon")))
            );
        }
        other => panic!(
            "Expected action defaults to produce a record, got {:?}",
            other
        ),
    }
}

#[test]
fn test_abstract_record_instantiation_fails_at_runtime() {
    let source = r#"
        abstract type UserBase = {
          name: string
        }

        let make(): UserBase = { <UserBase name={"Ada"} /> }
    "#;

    let result = execute_function(source, "make", vec![]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Cannot instantiate abstract record 'UserBase'"),
        "Expected abstract instantiation runtime error"
    );
}

#[test]
fn test_nested_record_access() {
    let source = r#"
        type Address = { city: string = "SF" }
        type User = { address: Address }
        let city(user:User): string = { user.address.city }
    "#;

    let mut addr = FxHashMap::default();
    addr.insert(SmolStr::new("city"), Value::String(SmolStr::new("Paris")));
    let mut user = FxHashMap::default();
    user.insert(
        SmolStr::new("address"),
        Value::Record {
            type_name: nx_hir::Name::new("Address"),
            fields: addr,
        },
    );

    let result = execute_function(
        source,
        "city",
        vec![Value::Record {
            type_name: nx_hir::Name::new("User"),
            fields: user,
        }],
    )
    .unwrap_or_else(|err| panic!("Nested record access failed: {}", err));
    assert_eq!(result, Value::String(SmolStr::new("Paris")));
}

#[test]
fn test_record_return_type_and_collection() {
    let source = r#"
        type User = { name: string = "Anon" }
        let make(): User = { <User /> }
    "#;

    let interpreter = Interpreter::new();
    let parse = parse_str(source, "record-return.nx");
    assert!(parse.errors.is_empty(), "parse errors: {:?}", parse.errors);
    let module = lower(parse.root().unwrap(), SourceId::new(0));

    let made = interpreter
        .execute_function(&module, "make", vec![])
        .unwrap_or_else(|e| panic!("{}", e));
    match made {
        Value::Record { fields, type_name } => {
            assert_eq!(type_name.as_str(), "User");
            assert_eq!(
                fields.get("name"),
                Some(&Value::String(SmolStr::new("Anon")))
            );
        }
        other => panic!("expected Record, got {:?}", other),
    }
}

#[test]
fn test_optional_record_field_defaults_to_null() {
    let source = r#"
        type User = { email: string? }
        let getEmail(user:User) = { user.email }
    "#;

    let interpreter = Interpreter::new();
    let parse = parse_str(source, "optional.nx");
    assert!(parse.errors.is_empty());
    let module = lower(parse.root().unwrap(), SourceId::new(0));

    // instantiate with defaults (null for optional field)
    let record = interpreter
        .instantiate_record_defaults(&module, "User")
        .unwrap();
    let result = interpreter
        .execute_function(&module, "getEmail", vec![record])
        .unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_record_all_fields_have_defaults() {
    let source = r#"
        type Config = { host: string = "localhost" port: int = 80 }
        let make(): Config = { <Config /> }
    "#;

    let interpreter = Interpreter::new();
    let parse = parse_str(source, "config.nx");
    assert!(parse.errors.is_empty());
    let module = lower(parse.root().unwrap(), SourceId::new(0));

    let result = interpreter
        .execute_function(&module, "make", vec![])
        .unwrap();
    match result {
        Value::Record { fields, type_name } => {
            assert_eq!(type_name.as_str(), "Config");
            assert_eq!(
                fields.get("host"),
                Some(&Value::String(SmolStr::new("localhost")))
            );
            assert_eq!(fields.get("port"), Some(&Value::Int(80)));
        }
        other => panic!("expected Record, got {:?}", other),
    }
}

#[test]
fn test_external_record_argument_does_not_evaluate_definition_defaults() {
    let source = r#"
        let defaultTimeout = 30
        type Config = { timeout:int = { defaultTimeout } }
        let echo(config:Config) = { config }
    "#;

    let result = execute_function(
        source,
        "echo",
        vec![Value::Record {
            type_name: nx_hir::Name::new("Config"),
            fields: FxHashMap::default(),
        }],
    )
    .unwrap_or_else(|err| panic!("External record argument should pass through:\n{}", err));

    match result {
        Value::Record { type_name, fields } => {
            assert_eq!(type_name.as_str(), "Config");
            assert!(
                fields.is_empty(),
                "Expected external record fields to be preserved"
            );
        }
        other => panic!("expected Record, got {:?}", other),
    }
}

// ============================================================================
// Element and Paren Call Interop Tests
// ============================================================================

#[test]
fn test_element_call_invokes_paren_defined_function() {
    let source = r#"
        let add(a:int, b:int): int = { a + b }
        let compute(): int = { <add b=2 a=1 /> }
    "#;

    let result = execute_function(source, "compute", vec![])
        .unwrap_or_else(|err| panic!("Element call failed:\n{}", err));
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_element_call_invokes_element_defined_function() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
        let compute(): int = { <add a=1 b=2 /> }
    "#;

    let result = execute_function(source, "compute", vec![])
        .unwrap_or_else(|err| panic!("Element call failed:\n{}", err));
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_paren_call_constructs_record_type_positionally() {
    let source = r#"
        type User = { name:string age:int = 30 }
        let getName(): string = { User("Bob", 42).name }
        let getAge(): int = { User("Bob").age }
    "#;

    let result = execute_function(source, "getName", vec![])
        .unwrap_or_else(|err| panic!("Record constructor call failed:\n{}", err));
    assert_eq!(result, Value::String(SmolStr::new("Bob")));

    let result = execute_function(source, "getAge", vec![])
        .unwrap_or_else(|err| panic!("Record constructor call failed:\n{}", err));
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_element_call_passes_content_to_paren_function() {
    let source = r#"
        let collect(content items:object[]): object[] = { items }
        let root(): object[] = { <collect><div /><span /></collect> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element call with content failed:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("span"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_paren_call_invokes_element_defined_function() {
    let source = r#"
        let <add a:int b:int /> = { a + b }
        let compute(): int = { add(1, 2) }
    "#;

    let result = execute_function(source, "compute", vec![])
        .unwrap_or_else(|err| panic!("Paren call to element-defined function failed:\n{}", err));
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_element_call_constructs_record_via_type_alias() {
    let source = r#"
        type User = { name: string age: int = 30 }
        type Person = User
        let getName(): string = { <Person name="Bob" />.name }
        let getAge(): int = { <Person name="Bob" />.age }
    "#;

    let result = execute_function(source, "getName", vec![])
        .unwrap_or_else(|err| panic!("Element call record alias failed:\n{}", err));
    assert_eq!(result, Value::String(SmolStr::new("Bob")));

    let result = execute_function(source, "getAge", vec![])
        .unwrap_or_else(|err| panic!("Element call record alias failed:\n{}", err));
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_element_call_content_is_injected_for_element_defined_function() {
    let source = r#"
        let <collect content items: object[] />: object[] = { items }
        let root(): object[] = { <collect><div /><span /></collect> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element call with content failed:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("span"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_element_call_single_child_coerces_to_scalar_content_parameter() {
    let source = r#"
        let <collect content item: div />: div = { item }
        let root(): div = { <collect><div /></collect> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element call with scalar content failed:\n{}", err));

    assert_eq!(
        result,
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        }
    );
}

#[test]
fn test_element_call_scalar_value_child_passes_to_scalar_content_parameter() {
    let source = r#"
        let <collect content item: int />: int = { item }
        let root(): int = { <collect>{1}</collect> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element call with scalar value content failed:\n{}", err));

    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_element_call_scalar_value_child_coerces_to_list_content_parameter() {
    let source = r#"
        let <collect content items: int[] />: int[] = { items }
        let root(): int[] = { <collect>{1}</collect> }
    "#;

    let result = execute_function(source, "root", vec![]).unwrap_or_else(|err| {
        panic!(
            "Element call with scalar value content list failed:\n{}",
            err
        )
    });

    assert_eq!(result, Value::Array(vec![Value::Int(1)]));
}

#[test]
fn test_element_call_multi_children_rejected_for_scalar_content_parameter() {
    let source = r#"
        let <collect content item: div />: div = { item }
        let root(): div = { <collect><div /><span /></collect> }
    "#;

    let result = execute_function(source, "root", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("return value") || err.contains("parameter") || err.contains("content"),
        "Error should mention the scalar/list mismatch:\n{}",
        err
    );
}

#[test]
fn test_intrinsic_element_named_and_body_content_conflict_is_rejected() {
    let source = r#"
        let root() = <div content="named">body</div>
    "#;

    let result = execute_function(source, "root", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("both a 'content' property and element body content"),
        "Error should mention the intrinsic content conflict:\n{}",
        err
    );
}

#[test]
fn test_element_call_braced_child_list_flattens_content_array() {
    let source = r#"
        let <collect content items: object[] />: object[] = { items }
        let root(): object[] = { <collect>{<div /> <span />}</collect> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element call with braced child list failed:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("span"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_element_call_conditional_content_expression_preserves_selected_element() {
    let source = r#"
        let <collect content items: object[] />: object[] = { items }
        let root(flag: bool): object[] = { <collect>if flag { <A /> } else { <B /> }</collect> }
    "#;

    let true_result = execute_function(source, "root", vec![Value::Boolean(true)])
        .unwrap_or_else(|err| panic!("Element call with conditional content failed:\n{}", err));
    assert_eq!(
        true_result,
        Value::Array(vec![Value::Record {
            type_name: nx_hir::Name::new("A"),
            fields: FxHashMap::default(),
        }])
    );

    let false_result = execute_function(source, "root", vec![Value::Boolean(false)])
        .unwrap_or_else(|err| panic!("Element call with conditional content failed:\n{}", err));
    assert_eq!(
        false_result,
        Value::Array(vec![Value::Record {
            type_name: nx_hir::Name::new("B"),
            fields: FxHashMap::default(),
        }])
    );
}

#[test]
fn test_element_call_for_content_expression_flattens_element_results() {
    let source = r#"
        let <collect content items: object[] />: object[] = { items }
        let root(items: object[]): object[] = { <collect>for item in items { <Row /> }</collect> }
    "#;

    let items = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let result = execute_function(source, "root", vec![items])
        .unwrap_or_else(|err| panic!("Element call with for content failed:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("Row"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("Row"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("Row"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_multi_value_element_brace_list_returns_array_runtime() {
    let source = r#"
        let root() = { <div /> <span /> }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Element-valued brace list failed at runtime:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("span"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_scalar_brace_return_is_wrapped_for_list_annotation_runtime() {
    let source = r#"
        let values(): int[] = { 1 }
    "#;

    let result = execute_function(source, "values", vec![])
        .unwrap_or_else(|err| panic!("Scalar-to-list return coercion failed:\n{}", err));

    assert_eq!(result, Value::Array(vec![Value::Int(1)]));
}

#[test]
fn test_multi_value_brace_return_is_rejected_for_scalar_annotation_runtime() {
    let source = r#"
        let values(): int = { 1 2 }
    "#;

    let result = execute_function(source, "values", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("return value"),
        "Error should mention the return value mismatch:\n{}",
        err
    );
}

#[test]
fn test_element_call_content_conflict_is_error_for_function() {
    let source = r#"
        let collect(content items: object[]): object[] = { items }
        let root(): object[] = { <collect items={null}><div /></collect> }
    "#;

    let result = execute_function(source, "root", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("content"),
        "Error should mention content:\n{}",
        err
    );
    assert!(err.contains("both"), "Error should mention both:\n{}", err);
}

#[test]
fn test_element_call_content_conflict_is_error_for_record() {
    let source = r#"
        type Container = { content items: object[] }
        type Box = Container
        let root(): object = { <Box items={null}><div /></Box> }
    "#;

    let result = execute_function(source, "root", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("content"),
        "Error should mention content:\n{}",
        err
    );
    assert!(err.contains("both"), "Error should mention both:\n{}", err);
}

#[test]
fn test_element_call_body_is_error_for_record_without_content_field() {
    let source = r#"
        type User = { name: string }
        type Person = User
        let root(): object = { <Person><div /></Person> }
    "#;

    let result = execute_function(source, "root", vec![]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("content"),
        "Error should mention content:\n{}",
        err
    );
}

#[test]
fn test_element_call_body_populates_record_content_field() {
    let source = r#"
        type Container = { content items: object[] }
        type Box = Container
        let root(): object[] = { <Box><div /><span /></Box>.items }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Record content injection failed:\n{}", err));

    let expected = Value::Array(vec![
        Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields: FxHashMap::default(),
        },
        Value::Record {
            type_name: nx_hir::Name::new("span"),
            fields: FxHashMap::default(),
        },
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_element_call_text_body_populates_scalar_content_field() {
    let source = r#"
        type Label = { content text:string }
        let root(): string = { <Label>label text</Label>.text }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Record text content injection failed:\n{}", err));

    assert_eq!(result, Value::String(SmolStr::new("label text")));
}

#[test]
fn test_component_invocation_binds_body_to_declared_content_prop() {
    let source = r#"
        component <Panel title:string content body:object /> = {
            <section title={title}>{body}</section>
        }
        let root() = { <Panel title="Docs"><Badge /></Panel>.body }
    "#;

    let result = execute_function(source, "root", vec![])
        .unwrap_or_else(|err| panic!("Component content invocation failed:\n{}", err));

    assert_eq!(
        result,
        Value::Record {
            type_name: nx_hir::Name::new("Badge"),
            fields: FxHashMap::default(),
        }
    );
}
