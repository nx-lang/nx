//! The host boundary of the occurrence model: `null`, `[]` and a missing key all decode to the
//! empty value, an empty optional field is an omitted key, an entry call's empty standalone `T?`
//! result and a cleared update-record field encode as `null`.

use nx_api::{eval_source, from_nx_value, to_nx_value, EvalResult, ProgramBuildContext};
use nx_hir::LoweredModule;
use nx_interpreter::{Interpreter, ResolvedProgram, Value};
use nx_value::NxValue;
use std::sync::Arc;

struct Runtime {
    module: Arc<LoweredModule>,
    interpreter: Interpreter,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, "boundary.nx");
        assert!(
            result.errors().is_empty(),
            "Expected source to pass analysis, got {:?}",
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message().to_string())
                .collect::<Vec<_>>()
        );
        let module = result.lowered_module.expect("lowered module");
        let interpreter = Interpreter::from_resolved_program(ResolvedProgram::single_root_module(
            source.len() as u64,
            "boundary.nx",
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn call(&self, function: &str, args: Vec<Value>) -> Value {
        self.interpreter
            .execute_function(self.module.as_ref(), function, args)
            .unwrap_or_else(|error| panic!("{function} failed: {error}"))
    }

    fn call_err(&self, function: &str, args: Vec<Value>) -> String {
        self.interpreter
            .execute_function(self.module.as_ref(), function, args)
            .expect_err("expected a runtime error")
            .to_string()
    }
}

fn json(value: &Value) -> String {
    to_nx_value(value).to_json_string().expect("JSON")
}

fn decode(json: &str) -> Value {
    from_nx_value(&NxValue::from_json_str(json).expect("valid JSON")).expect("a decodable value")
}

const BOOK: &str =
    "type Person = { name:string }\ntype Book = { title:string author?:Person tags?:string+ }\n";

#[test]
fn a_host_null_an_empty_array_and_a_missing_key_all_decode_to_the_empty_value() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let same(x:Book, y:Book): boolean = {{ x == y }}\nlet b(): Book = {{ <Book title=\"A\" /> }}\nlet has(b:Book): boolean = {{ b.author? }}"
    ));
    let b = runtime.call("b", vec![]);
    for source in [
        r#"{"$type":"Book","title":"A","author":null}"#,
        r#"{"$type":"Book","title":"A","author":[]}"#,
        r#"{"$type":"Book","title":"A"}"#,
        r#"{"$type":"Book","title":"A","tags":null}"#,
    ] {
        let decoded = decode(source);
        assert_eq!(
            runtime.call("same", vec![decoded.clone(), b.clone()]),
            Value::Boolean(true),
            "{source}"
        );
        assert_eq!(
            runtime.call("has", vec![decoded]),
            Value::Boolean(false),
            "{source}"
        );
    }
}

#[test]
fn a_host_empty_value_is_rejected_where_at_least_one_is_required() {
    // `name` never reads `items`, so the error comes from the boundary, not a later read.
    let runtime = Runtime::new(
        "type Box = { name:string items:string+ }\nlet name(b:Box): string = { b.name }",
    );
    for source in [
        r#"{"$type":"Box","name":"A","items":null}"#,
        r#"{"$type":"Box","name":"A","items":[]}"#,
    ] {
        let error = runtime.call_err("name", vec![decode(source)]);
        assert!(error.contains("items"), "{source}: {error}");
    }
}

#[test]
fn a_host_empty_value_is_rejected_where_exactly_one_is_required_even_with_a_default() {
    let runtime = Runtime::new(
        "type Card = { name:string label:string = \"none\" }\nlet name(c:Card): string = { c.name }\nlet label(c:Card): string = { c.label }",
    );
    assert_eq!(
        runtime.call("label", vec![decode(r#"{"$type":"Card","name":"A"}"#)]),
        Value::String("none".into()),
        "A missing key takes the default"
    );
    for source in [
        r#"{"$type":"Card","name":"A","label":null}"#,
        r#"{"$type":"Card","name":"A","label":[]}"#,
    ] {
        let error = runtime.call_err("name", vec![decode(source)]);
        assert!(error.contains("label"), "{source}: {error}");
    }
}

#[test]
fn a_host_array_at_an_optional_field_is_normalized_at_the_boundary() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let same(x:Book, y:Book): boolean = {{ x == y }}\nlet title(b:Book): string = {{ b.title }}\nlet a(): Book = {{ <Book title=\"A\" author=<Person name=\"X\" /> /> }}"
    ));
    let decoded =
        decode(r#"{"$type":"Book","title":"A","author":[{"$type":"Person","name":"X"}]}"#);
    let a = runtime.call("a", vec![]);
    assert_eq!(runtime.call("same", vec![decoded, a]), Value::Boolean(true));

    let error = runtime.call_err(
        "title",
        vec![decode(
            r#"{"$type":"Book","title":"A","author":[{"$type":"Person","name":"X"},{"$type":"Person","name":"Y"}]}"#,
        )],
    );
    assert!(error.contains("author"), "{error}");
}

#[test]
fn a_host_record_with_an_unknown_field_is_rejected_at_an_entry_call() {
    let runtime = Runtime::new(&format!("{BOOK}let title(b:Book): string = {{ b.title }}"));
    let error = runtime.call_err(
        "title",
        vec![decode(r#"{"$type":"Book","title":"A","extra":1}"#)],
    );
    assert!(error.contains("extra"), "{error}");
}

#[test]
fn an_empty_optional_field_encodes_as_an_omitted_key() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let a(): Book = {{ <Book title=\"A\" author=<Person name=\"X\" /> /> }}\nlet b(): Book = {{ <Book title=\"B\" /> }}\nlet c(): Book = {{ <Book title=\"B\" author={{}} tags={{}} /> }}"
    ));
    let a = json(&runtime.call("a", vec![]));
    assert!(a.contains(r#""author":{"#), "{a}");
    let b = json(&runtime.call("b", vec![]));
    assert_eq!(b, r#"{"$type":"Book","title":"B"}"#);
    assert_eq!(
        json(&runtime.call("c", vec![])),
        b,
        "omitted and written empty are one value"
    );
}

/// An entry call returns an empty result whose type is a standalone `T?`, declared or inferred, as
/// the host's `null`; every other result keeps its encoding.
#[test]
fn an_entry_calls_empty_optional_result_is_null() {
    let root = |source: &str| match eval_source(source, "entry.nx", &ProgramBuildContext::empty()) {
        EvalResult::Ok(value) => value.to_json_string().expect("JSON"),
        EvalResult::Err(diagnostics) => panic!("{source}: {diagnostics:?}"),
    };
    assert_eq!(root("let root(): string? = { if false { \"a\" } }"), "null");
    assert_eq!(root("let root() = { if false { 1 } }"), "null");
    assert_eq!(root("let root(): string? = { \"a\" }"), r#""a""#);
    assert_eq!(root("let root(): int* = { if false { 1 } }"), "[]");
    assert_eq!(
        root(&format!(
            "{BOOK}let root() = {{ <Book title=\"A\" />.author?.name }}"
        )),
        "null"
    );
}

/// Inside the program, and wherever the empty value is not an entry call's `T?` result, it is the
/// empty array.
#[test]
fn every_source_of_emptiness_encodes_as_an_empty_array() {
    let runtime = Runtime::new(
        "let f(c:boolean): int? = { if c { 1 } }\nlet g(): int? = {}\nlet h(ns?:int+): int* = { for n in ns {} }",
    );
    assert_eq!(json(&runtime.call("f", vec![Value::Boolean(false)])), "[]");
    assert_eq!(json(&runtime.call("g", vec![])), "[]");
    assert_eq!(
        json(&runtime.call("h", vec![Value::Array(vec![Value::Int(1)])])),
        "[]"
    );
}

#[test]
fn a_cleared_update_record_field_encodes_as_null() {
    let runtime = Runtime::new(
        "type User = { name:string email?:string }\nlet clear(): User.Update = { <User.Update email={} /> }\nlet rename(): User.Update = { <User.Update name=\"Bo\" /> }",
    );
    assert_eq!(
        json(&runtime.call("clear", vec![])),
        r#"{"$type":"User.Update","email":null}"#
    );
    assert_eq!(
        json(&runtime.call("rename", vec![])),
        r#"{"$type":"User.Update","name":"Bo"}"#
    );
}

#[test]
fn a_host_update_record_with_a_null_field_clears_it() {
    let runtime = Runtime::new(
        "type User = { name:string email?:string }\nlet patched(u:User, p:User.Update): User = { apply(u, p) }\nlet has(u:User): boolean = { u.email? }",
    );
    let user = decode(r#"{"$type":"User","name":"Ada","email":"a@x"}"#);
    let patch = decode(r#"{"$type":"User.Update","email":null}"#);
    let patched = runtime.call("patched", vec![user, patch]);
    assert_eq!(json(&patched), r#"{"$type":"User","name":"Ada"}"#);
    assert_eq!(runtime.call("has", vec![patched]), Value::Boolean(false));
}
