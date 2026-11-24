//! Checking `with`: what it applies to, the fields a field list sets and the paths through them,
//! and the names it reserves, from the `add-with-expressions` change.

use nx_types::check_str;

fn codes(source: &str) -> Vec<(String, String)> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code().unwrap_or("").to_string(),
                diagnostic.message().to_string(),
            )
        })
        .collect()
}

fn assert_clean(source: &str) {
    let errors = codes(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

/// The message of the first error with `code`.
fn message_of(source: &str, code: &str) -> String {
    let errors = codes(source);
    errors
        .iter()
        .find(|(found, _)| found == code)
        .map(|(_, message)| message.clone())
        .unwrap_or_else(|| panic!("expected {code}, got: {errors:?}"))
}

const TYPES: &str = "type Address = { city:string zip?:string }\n\
                     type User = { name:string age:int address:Address home?:Address }\n\
                     let ada(): User = { <User name=\"Ada\" age=36 address=<Address city=\"London\" /> /> }\n";

#[test]
fn a_record_takes_a_field_list_a_path_and_an_update_record() {
    assert_clean(&format!(
        "{TYPES}let a() = {{ ada() with {{ name=\"Bo\" address.city=\"Paris\" }} }}\n\
         let b() = {{ ada() with <User.Update age=37 /> }}\n\
         let c() = {{ ada() with {{ age=1 }} with {{ age=2 }} }}"
    ));
}

#[test]
fn an_update_record_takes_an_update_record() {
    assert_clean(&format!(
        "{TYPES}let patch(): User.Update = {{ <User.Update name=\"Bo\" /> }}\n\
         let merged(): User.Update = {{ patch() with {{ age=40 }} }}"
    ));
}

#[test]
fn with_on_a_primitive_is_with_target() {
    let message = message_of("let n() = { 3 with { x=1 } }", "with-target");
    assert!(message.contains("int"), "{message}");
}

#[test]
fn with_on_an_optional_suggests_unwrapping_it() {
    let message = message_of(
        &format!("{TYPES}let h(u: User) = {{ u.home with {{ city=\"Rome\" }} }}"),
        "with-target",
    );
    assert!(
        message.contains('?') || message.contains("optional"),
        "{message}"
    );
}

#[test]
fn with_on_a_sequence_is_with_target() {
    message_of(
        &format!(
            "{TYPES}let all(): User+ = {{ ada() ada() }}\nlet s() = {{ all() with {{ age=1 }} }}"
        ),
        "with-target",
    );
}

#[test]
fn a_path_through_an_optional_field_is_with_path() {
    message_of(
        &format!("{TYPES}let a() = {{ ada() with {{ home.city=\"Rome\" }} }}"),
        "with-path",
    );
}

#[test]
fn a_path_on_an_update_record_is_with_path() {
    message_of(
        &format!(
            "{TYPES}let patch(): User.Update = {{ <User.Update name=\"Bo\" /> }}\n\
             let p() = {{ patch() with {{ address.city=\"Rome\" }} }}"
        ),
        "with-path",
    );
}

#[test]
fn a_field_list_checks_names_duplicates_and_values() {
    message_of(
        &format!("{TYPES}let a() = {{ ada() with {{ missing=1 }} }}"),
        "unknown-record-field",
    );
    message_of(
        &format!("{TYPES}let a() = {{ ada() with {{ age=1 age=2 }} }}"),
        "duplicate-property",
    );
    message_of(
        &format!("{TYPES}let a() = {{ ada() with {{ age=\"old\" }} }}"),
        "record-field-type-mismatch",
    );
}

#[test]
fn an_operand_of_another_type_is_rejected() {
    message_of(
        &format!("{TYPES}let a() = {{ ada() with <Address.Update city=\"Rome\" /> }}"),
        "value-type-mismatch",
    );
}

#[test]
fn with_is_reserved_as_a_name() {
    message_of("let with = 1", "reserved-name");
    message_of("let f(with: int) = { with }", "reserved-name");
    message_of(
        "let f(xs: int+) = { for with in xs { 1 } }",
        "reserved-name",
    );
}

#[test]
fn with_stays_a_field_name() {
    assert_clean("type Options = { with:string }");
}

#[test]
fn env_with_outside_provide_is_reserved() {
    let message = message_of(
        "environment space: int = 4\nlet b() = { env with { space=2 } }",
        "reserved-name",
    );
    assert!(message.contains("provide"), "{message}");
}

#[test]
fn a_bundle_takes_steps_and_a_field_list_with_paths() {
    assert_clean(
        "type Size = { width:int height:int }\n\
         environment size: Size = <Size width=1 height=2 />\n\
         environment space: int = 4\n\
         let theme: env.Update = <env.Update space=8 />\n\
         let dense: env.Update = { theme with { size.width=3 } }\n\
         let page() = { provide env with dense with { space={env.space * 2} } { env.size } }",
    );
}
