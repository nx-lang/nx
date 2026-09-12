//! Evaluation of the update intrinsics `apply`, `merge`, `diff`, and `changed` through the public
//! `Interpreter` API, and of property union values. Every source is type checked first, so these
//! are the programs a host would actually run. Covers the evaluation scenarios of the
//! update-records, property-references, and value-equality capabilities.

use nx_hir::{LoweredModule, Name};
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
        let result = nx_types::check_str(source, "update-intrinsics.nx");
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
            "update-intrinsics.nx",
            module.clone(),
        ));
        Self {
            module,
            interpreter,
        }
    }

    fn call(&self, function: &str) -> Value {
        self.interpreter
            .execute_function(self.module.as_ref(), function, vec![])
            .expect("Expected function to evaluate")
    }
}

fn record(type_name: &str, fields: &[(&str, Value)]) -> Value {
    Value::Record {
        type_name: Name::new(type_name),
        fields: fields
            .iter()
            .map(|(name, value)| (SmolStr::new(*name), value.clone()))
            .collect::<FxHashMap<_, _>>(),
    }
}

fn string(value: &str) -> Value {
    Value::String(SmolStr::new(value))
}

fn case(union: &str, case: &str) -> Value {
    Value::UnionCase {
        union: Name::new(union),
        case: SmolStr::new(case),
    }
}

const USER: &str = r#"
type User = { name:string email:string? age:int? }
"#;

// ============================================================================
// apply
// ============================================================================

#[test]
fn apply_replaces_present_fields_and_keeps_the_rest() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let u = <User name="Ada" email="ada@example.com" age=30 />
        let v() = {{apply(u, <User.Update email={{null}} />)}}
        "#
    ));
    assert_eq!(
        runtime.call("v"),
        record(
            "User",
            &[
                ("name", string("Ada")),
                ("email", Value::Null),
                ("age", Value::Int(30)),
            ]
        )
    );
}

#[test]
fn apply_with_an_empty_update_returns_an_equal_record() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string = "anon" }
        let v() = {apply(<User name="Ada" />, <User.Update />)}
        "#,
    );
    assert_eq!(
        runtime.call("v"),
        record("User", &[("name", string("Ada"))])
    );
}

// ============================================================================
// merge
// ============================================================================

#[test]
fn merge_lets_the_later_update_win_and_keeps_absence() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let m() = {{merge(<User.Update name="Ada" email="x@y" />, <User.Update email={{null}} />)}}
        "#
    ));
    assert_eq!(
        runtime.call("m"),
        record(
            "User.Update",
            &[("name", string("Ada")), ("email", Value::Null)]
        )
    );
}

#[test]
fn applying_a_merged_update_equals_applying_both_in_turn() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let u = <User name="Ada" email="x@y" />
        let a = <User.Update name="Bo" />
        let b = <User.Update email={{null}} age=1 />
        let merged() = {{apply(u, merge(a, b))}}
        let stepped() = {{apply(apply(u, a), b)}}
        "#
    ));
    assert_eq!(runtime.call("merged"), runtime.call("stepped"));
}

// ============================================================================
// diff
// ============================================================================

#[test]
fn diff_lists_only_differing_fields_with_a_present_null() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let d() = {{diff(<User name="Ada" email="x@y" />, <User name="Ada" email={{null}} />)}}
        "#
    ));
    assert_eq!(
        runtime.call("d"),
        record("User.Update", &[("email", Value::Null)])
    );
}

#[test]
fn diff_of_equal_records_is_an_empty_update() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string }
        let d() = {diff(<User name="Ada" />, <User name="Ada" />)}
        "#,
    );
    assert_eq!(runtime.call("d"), record("User.Update", &[]));
}

#[test]
fn diff_compares_nested_records_and_lists_structurally_and_apply_round_trips() {
    let runtime = Runtime::new(
        r#"
        type Address = { city:string }
        type User = { name:string tags:string[] home:Address }
        let a = <User name="Ada" tags={ "x" "y" } home=<Address city="Paris" /> />
        let same() = {diff(a, <User name="Ada" tags={ "x" "y" } home=<Address city="Paris" /> />)}
        let b = <User name="Ada" tags={ "x" } home=<Address city="Rome" /> />
        let changed_fields() = {diff(a, b)}
        let round_trip() = {apply(a, diff(a, b)) == b}
        "#,
    );
    assert_eq!(runtime.call("same"), record("User.Update", &[]));
    assert_eq!(
        runtime.call("changed_fields"),
        record(
            "User.Update",
            &[
                ("tags", Value::Array(vec![string("x")])),
                ("home", record("Address", &[("city", string("Rome"))])),
            ]
        )
    );
    assert_eq!(runtime.call("round_trip"), Value::Boolean(true));
}

// ============================================================================
// changed
// ============================================================================

#[test]
fn changed_lists_present_fields_in_declaration_order() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let keys() = {{changed(<User.Update age={{null}} name="Ada" />)}}
        let none() = {{changed(<User.Update />)}}
        "#
    ));
    assert_eq!(
        runtime.call("keys"),
        Value::Array(vec![
            case("User.Property", "name"),
            case("User.Property", "age"),
        ])
    );
    assert_eq!(runtime.call("none"), Value::Array(Vec::new()));
}

#[test]
fn changed_of_a_component_update_names_state_fields() {
    let runtime = Runtime::new(
        r#"
        component <Counter step:int = 1 /> = { state { count:int = 0 label:string = "" } <Label /> }
        let keys() = {changed(<Counter.Update label="x" count=1 />)}
        "#,
    );
    assert_eq!(
        runtime.call("keys"),
        Value::Array(vec![
            case("Counter.Property", "count"),
            case("Counter.Property", "label"),
        ])
    );
}

#[test]
fn changed_and_diff_agree() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let keys() = {{changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))}}
        "#
    ));
    assert_eq!(
        runtime.call("keys"),
        Value::Array(vec![case("User.Property", "name")])
    );
}

// ============================================================================
// property union values
// ============================================================================

#[test]
fn property_union_cases_evaluate_and_compare() {
    let runtime = Runtime::new(&format!(
        r#"{USER}
        let key() = {{User.Property.email}}
        let same() = {{User.Property.name == User.Property.name}}
        let other() = {{User.Property.name == User.Property.email}}
        "#
    ));
    assert_eq!(runtime.call("key"), case("User.Property", "email"));
    assert_eq!(runtime.call("same"), Value::Boolean(true));
    assert_eq!(runtime.call("other"), Value::Boolean(false));
}

#[test]
fn a_bare_case_at_a_property_typed_site_evaluates_as_the_qualified_form() {
    let runtime = Runtime::new(
        r#"
        type Contact = { title:string subtitle:string }
        let bare(): Contact.Property = subtitle
        let qualified() = {Contact.Property.subtitle}
        "#,
    );
    assert_eq!(runtime.call("bare"), runtime.call("qualified"));
    assert_eq!(runtime.call("bare"), case("Contact.Property", "subtitle"));
}

#[test]
fn a_match_over_a_property_union_selects_the_arm() {
    let runtime = Runtime::new(
        r#"
        type User = { name:string email:string? }
        let label(key:User.Property) = {if key is { name => "Name" email => "Email" }}
        let e() = {label(User.Property.email)}
        "#,
    );
    assert_eq!(runtime.call("e"), string("Email"));
}

#[test]
fn equality_compares_records_and_lists_structurally() {
    let runtime = Runtime::new(
        r#"
        type Address = { city:string }
        let rome = <Address city="Rome" />
        let romeAgain = <Address city="Rome" />
        let paris = <Address city="Paris" />
        let ones = { 1 2 }
        let onesAgain = { 1 2 }
        let twos = { 2 1 }
        let sameRecord() = {rome == romeAgain}
        let otherRecord() = {rome == paris}
        let sameList() = {ones == onesAgain}
        let otherList() = {ones == twos}
        let notEqual() = {rome != paris}
        "#,
    );
    assert_eq!(runtime.call("sameRecord"), Value::Boolean(true));
    assert_eq!(runtime.call("otherRecord"), Value::Boolean(false));
    assert_eq!(runtime.call("sameList"), Value::Boolean(true));
    assert_eq!(runtime.call("otherList"), Value::Boolean(false));
    assert_eq!(runtime.call("notEqual"), Value::Boolean(true));
}
