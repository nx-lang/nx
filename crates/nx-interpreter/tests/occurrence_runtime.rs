//! The runtime face of the occurrence model: the empty value, optional fields that are not
//! stored, coercion at suffixed sites, and the three presence operators.
//!
//! One case per runtime scenario in `occurrence-types`, `optional-properties` and
//! `presence-operators`. Every source is type checked first, so these are programs a host would
//! actually run.

use nx_hir::LoweredModule;
use nx_interpreter::{Interpreter, ResolvedProgram, Value};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::sync::Arc;

struct Runtime {
    module: Arc<LoweredModule>,
    interpreter: Interpreter,
}

impl Runtime {
    fn new(source: &str) -> Self {
        let result = nx_types::check_str(source, "occurrence.nx");
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
            "occurrence.nx",
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

fn record(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Record {
        type_name: nx_hir::Name::new(type_name),
        fields: fields
            .iter()
            .map(|(name, value)| (SmolStr::new(name), value.clone()))
            .collect::<FxHashMap<_, _>>(),
    }
}

const BOOK: &str = "type Person = { name:string }\ntype Book = { title:string author?:Person }\n";

// ---------------------------------------------------------------------------
// The empty value and the lattice at runtime
// ---------------------------------------------------------------------------

#[test]
fn exactly_one_binds_as_an_item_at_an_optional_site_and_as_a_sequence_at_a_plus_site() {
    let runtime =
        Runtime::new("let a(): int? = { 1 }\nlet b(): int+ = { 1 }\nlet c(): int* = { 1 }");
    assert_eq!(runtime.call("a", vec![]), Value::Int(1));
    assert_eq!(runtime.call("b", vec![]), Value::Array(vec![Value::Int(1)]));
    assert_eq!(runtime.call("c", vec![]), Value::Array(vec![Value::Int(1)]));
}

#[test]
fn every_source_of_emptiness_is_the_same_value() {
    let runtime = Runtime::new(
        "let f(c:boolean): int? = { if c { 1 } }\nlet g(): int? = {}\nlet h(ns?:int+): int* = { for n in ns {} }",
    );
    let f = runtime.call("f", vec![Value::Boolean(false)]);
    let g = runtime.call("g", vec![]);
    let h = runtime.call("h", vec![Value::Array(vec![Value::Int(1)])]);
    assert_eq!(f, Value::empty());
    assert_eq!(g, Value::empty());
    assert_eq!(h, Value::empty());
    assert_eq!(f, Value::Array(vec![]));
}

#[test]
fn an_optional_beside_an_item_makes_one_or_more() {
    let runtime = Runtime::new(
        "let o:int? = {}\nlet p:int? = 3\nlet xs(): int+ = {o 2}\nlet ys(): int* = {o p}",
    );
    assert_eq!(
        runtime.call("xs", vec![]),
        Value::Array(vec![Value::Int(2)])
    );
    assert_eq!(
        runtime.call("ys", vec![]),
        Value::Array(vec![Value::Int(3)])
    );
}

#[test]
fn an_empty_array_is_rejected_where_at_least_one_is_required() {
    let runtime = Runtime::new(
        "type Box = { items:string+ }\nlet make(items:string+): Box = { <Box items={items} /> }",
    );
    let error = runtime.call_err("make", vec![Value::Array(vec![])]);
    assert!(error.contains("items"), "{error}");
}

#[test]
fn a_for_over_an_optional_yields_its_item() {
    let runtime = Runtime::new(
        "type Person = { name:string }\nlet name(p?:Person): string? = { for x in p { x.name } }",
    );
    let ada = record("Person", &[("name", Value::String(SmolStr::new("Ada")))]);
    assert_eq!(
        runtime.call("name", vec![ada]),
        Value::String(SmolStr::new("Ada"))
    );
    assert_eq!(runtime.call("name", vec![Value::empty()]), Value::empty());
}

#[test]
fn a_for_over_an_optional_yields_its_item_without_an_annotation() {
    let runtime = Runtime::new(
        "type Person = { name:string }\nlet name(p?:Person) = { for x in p { x.name } }\nlet same(p?:Person) = { name(p) == \"Ada\" }",
    );
    let ada = record("Person", &[("name", Value::String(SmolStr::new("Ada")))]);
    assert_eq!(
        runtime.call("name", vec![ada.clone()]),
        Value::String(SmolStr::new("Ada"))
    );
    assert_eq!(runtime.call("same", vec![ada]), Value::Boolean(true));
}

#[test]
fn an_item_compares_as_a_sequence_of_one() {
    let runtime = Runtime::new(
        "let one:int? = 1\nlet ones:int* = { 1 }\nlet two:int+ = { 1 2 }\nlet same() = { one == ones }\nlet other() = { one == two }\nlet differs() = { one != two }",
    );
    assert_eq!(runtime.call("same", vec![]), Value::Boolean(true));
    assert_eq!(runtime.call("other", vec![]), Value::Boolean(false));
    assert_eq!(runtime.call("differs", vec![]), Value::Boolean(true));
}

#[test]
fn a_literal_compared_with_an_optional_float32_takes_its_item_type() {
    let runtime = Runtime::new(
        "let h:float32? = {0.1}\nlet k:float32+ = {0.1}\nlet a() = { h == 0.1 }\nlet b() = { k == 0.1 }",
    );
    assert_eq!(runtime.call("a", vec![]), Value::Boolean(true));
    assert_eq!(runtime.call("b", vec![]), Value::Boolean(true));
}

#[test]
fn an_unstored_optional_field_of_a_union_case_reads_as_empty() {
    let runtime = Runtime::new(
        "type Shape =\n  | circle { r:int label?:string }\n  | square { s:int }\nlet f(s:Shape): string = { if s is { Shape.circle => s.label ?? \"nolabel\" Shape.square => \"sq\" } }\nlet root(): string = { f(<Shape.circle r={1} />) }",
    );
    assert_eq!(
        runtime.call("root", vec![]),
        Value::String(SmolStr::new("nolabel"))
    );
}

// ---------------------------------------------------------------------------
// Optional fields are not stored
// ---------------------------------------------------------------------------

#[test]
fn an_absent_optional_field_is_an_omitted_key_and_reads_as_empty() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let a(): Book = {{ <Book title=\"A\" author=<Person name=\"X\" /> /> }}\nlet b(): Book = {{ <Book title=\"B\" /> }}\nlet author(b:Book): Person? = {{ b.author }}\nlet c(): Book = {{ <Book title=\"C\" author={{}} /> }}"
    ));
    let a = runtime.call("a", vec![]);
    let b = runtime.call("b", vec![]);
    let c = runtime.call("c", vec![]);
    let Value::Record {
        fields: present, ..
    } = &a
    else {
        panic!("a record");
    };
    assert!(matches!(present.get("author"), Some(Value::Record { .. })));
    let Value::Record { fields, .. } = &b else {
        panic!("a record");
    };
    assert!(!fields.contains_key("author"));
    let Value::Record {
        fields: written, ..
    } = &c
    else {
        panic!("a record");
    };
    assert!(
        !written.contains_key("author"),
        "omitted and written empty are one value"
    );
    assert_eq!(runtime.call("author", vec![b]), Value::empty());
}

#[test]
fn a_cleared_update_record_field_is_stored_as_the_empty_value() {
    let runtime = Runtime::new(
        "type User = { name:string email?:string }\nlet clear(): User.Update = { <User.Update email={} /> }\nlet rename(): User.Update = { <User.Update name=\"Bo\" /> }",
    );
    assert_eq!(
        runtime.call("clear", vec![]),
        record("User.Update", &[("email", Value::empty())])
    );
    assert_eq!(
        runtime.call("rename", vec![]),
        record(
            "User.Update",
            &[("name", Value::String(SmolStr::new("Bo")))]
        )
    );
}

// ---------------------------------------------------------------------------
// Presence operators
// ---------------------------------------------------------------------------

#[test]
fn a_presence_test_is_a_boolean() {
    let runtime = Runtime::new(&format!("{BOOK}let has(b:Book): boolean = {{ b.author? }}"));
    let without = record("Book", &[("title", Value::String(SmolStr::new("A")))]);
    let with = record(
        "Book",
        &[
            ("title", Value::String(SmolStr::new("A"))),
            (
                "author",
                record("Person", &[("name", Value::String(SmolStr::new("P")))]),
            ),
        ],
    );
    assert_eq!(runtime.call("has", vec![without]), Value::Boolean(false));
    assert_eq!(runtime.call("has", vec![with]), Value::Boolean(true));
}

#[test]
fn a_test_on_zero_or_more_is_non_emptiness() {
    let runtime = Runtime::new("let any(xs?:int+): boolean = { xs? }");
    assert_eq!(
        runtime.call("any", vec![Value::empty()]),
        Value::Boolean(false)
    );
    assert_eq!(
        runtime.call("any", vec![Value::Array(vec![Value::Int(1)])]),
        Value::Boolean(true)
    );
}

#[test]
fn a_step_through_an_absent_receiver_is_empty() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let name(b:Book): string? = {{ b.author?.name }}"
    ));
    let without = record("Book", &[("title", Value::String(SmolStr::new("A")))]);
    let with = record(
        "Book",
        &[
            ("title", Value::String(SmolStr::new("A"))),
            (
                "author",
                record("Person", &[("name", Value::String(SmolStr::new("P")))]),
            ),
        ],
    );
    assert_eq!(runtime.call("name", vec![without]), Value::empty());
    assert_eq!(
        runtime.call("name", vec![with]),
        Value::String(SmolStr::new("P"))
    );
}

#[test]
fn a_fallback_makes_an_optional_exactly_one() {
    let runtime = Runtime::new(
        "type Book = { subtitle?:string }\nlet sub(b:Book): string = { b.subtitle ?? \"none\" }",
    );
    assert_eq!(
        runtime.call("sub", vec![record("Book", &[])]),
        Value::String(SmolStr::new("none"))
    );
    assert_eq!(
        runtime.call(
            "sub",
            vec![record(
                "Book",
                &[("subtitle", Value::String(SmolStr::new("S")))]
            )]
        ),
        Value::String(SmolStr::new("S"))
    );
}

#[test]
fn a_fallback_binds_tighter_than_concatenation() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let byline(b:Book): string = {{ \"Author: \" + b.author?.name ?? \"Anonymous\" }}"
    ));
    assert_eq!(
        runtime.call(
            "byline",
            vec![record(
                "Book",
                &[("title", Value::String(SmolStr::new("A")))]
            )]
        ),
        Value::String(SmolStr::new("Author: Anonymous"))
    );
}

#[test]
fn the_right_operand_is_evaluated_only_when_needed() {
    let runtime = Runtime::new("let fail(): int = { 1 / 0 }\nlet f(o?:int): int = { o ?? fail() }");
    assert_eq!(runtime.call("f", vec![Value::Int(1)]), Value::Int(1));
    let error = runtime.call_err("f", vec![Value::empty()]);
    assert!(error.contains("Division by zero"), "{error}");
}

#[test]
fn the_empty_pattern_matches_absence() {
    let runtime = Runtime::new(&format!(
        "{BOOK}let f(b:Book): string = {{ if b.author is {{ {{}} => \"anonymous\" else => b.author.name }} }}"
    ));
    assert_eq!(
        runtime.call(
            "f",
            vec![record(
                "Book",
                &[("title", Value::String(SmolStr::new("A")))]
            )]
        ),
        Value::String(SmolStr::new("anonymous"))
    );
    let with = record(
        "Book",
        &[
            ("title", Value::String(SmolStr::new("A"))),
            (
                "author",
                record("Person", &[("name", Value::String(SmolStr::new("P")))]),
            ),
        ],
    );
    assert_eq!(
        runtime.call("f", vec![with]),
        Value::String(SmolStr::new("P"))
    );
}

#[test]
fn the_empty_pattern_completes_a_match_over_an_optional_union() {
    let runtime = Runtime::new(
        "type State = idle | busy\nlet f(s?:State): string = { if s is { {} => \"none\" idle => \"idle\" busy => \"busy\" } }",
    );
    assert_eq!(
        runtime.call("f", vec![Value::empty()]),
        Value::String(SmolStr::new("none"))
    );
    assert_eq!(
        runtime.call(
            "f",
            vec![Value::UnionCase {
                union: nx_hir::Name::new("State"),
                case: SmolStr::new("busy"),
            }]
        ),
        Value::String(SmolStr::new("busy"))
    );
}

#[test]
fn a_bare_pattern_naming_a_payload_case_matches_its_records() {
    // The checker resolves a bare case name; one with fields must still match every record of
    // its case rather than be constructed without them.
    let runtime = Runtime::new(
        "type Load = idle | loading | failed { message:string }\nlet f(l:Load): string = { if l is { idle => \"idle\" failed => l.message else => \"other\" } }",
    );
    assert_eq!(
        runtime.call(
            "f",
            vec![record(
                "Load.failed",
                &[("message", Value::String(SmolStr::new("Offline")))]
            )]
        ),
        Value::String(SmolStr::new("Offline"))
    );
    assert_eq!(
        runtime.call(
            "f",
            vec![Value::UnionCase {
                union: nx_hir::Name::new("Load"),
                case: SmolStr::new("idle"),
            }]
        ),
        Value::String(SmolStr::new("idle"))
    );
    assert_eq!(
        runtime.call(
            "f",
            vec![Value::UnionCase {
                union: nx_hir::Name::new("Load"),
                case: SmolStr::new("loading"),
            }]
        ),
        Value::String(SmolStr::new("other"))
    );
}

#[test]
fn optional_state_starts_empty() {
    let source = "external component <Label text:string />\ncomponent <Search /> = { state { query?:string } <Label text={query ?? \"\"} /> }";
    let runtime = Runtime::new(source);
    let result = runtime
        .interpreter
        .initialize_component(runtime.module.as_ref(), "Search", Value::empty())
        .expect("initialization");
    assert_eq!(
        result.rendered,
        record("Label", &[("text", Value::String(SmolStr::new("")))])
    );
}
