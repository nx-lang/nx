//! Checking environment values: declarations, `env.` reads, `env.` attributes and `provide`
//! blocks, from the `add-environment-values` change.

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

fn assert_code(source: &str, code: &str) {
    let errors = codes(source);
    assert!(
        errors.iter().any(|(found, _)| found == code),
        "expected {code}, got: {errors:?}"
    );
}

const MODES: &str = "export type ColorMode = light | dark\n\
                     export environment colorMode: ColorMode = light\n\
                     environment locale: string = \"en-US\"\n\
                     type SemanticColors = { accent:string surface:string }\n\
                     environment color: SemanticColors = <SemanticColors accent=\"blue\" surface=\"white\" />\n";

#[test]
fn a_declaration_with_a_type_and_a_default_is_clean() {
    assert_clean(MODES);
}

#[test]
fn a_declaration_without_a_default_is_rejected() {
    assert_code("environment locale: string", "environment-default-required");
}

#[test]
fn a_default_of_the_wrong_type_is_rejected() {
    assert_code(
        "environment scale: float64 = \"large\"",
        "value-type-mismatch",
    );
}

#[test]
fn environment_stays_a_field_name() {
    assert_clean("type Deployment = { environment:string }");
}

#[test]
fn a_read_has_the_declared_type() {
    assert_clean(&format!("{MODES}let greeting(): string = {{ env.locale }}"));
    assert_code(
        &format!("{MODES}let greeting(): int = {{ env.locale }}"),
        "return-type-mismatch",
    );
}

#[test]
fn a_field_read_goes_through_the_record() {
    assert_clean(&format!(
        "{MODES}let accent(): string = {{ env.color.accent }}"
    ));
}

#[test]
fn an_unknown_read_is_rejected() {
    assert_code("let x() = { env.missing }", "unknown-environment-value");
}

#[test]
fn env_cannot_be_bound_or_read_alone() {
    assert_code("let describe(env: string) = { 1 }", "reserved-name");
    assert_code("let describe() = { env }", "reserved-name");
    assert_code(
        "let xs = { 1 2 }\nlet each() = { for env in xs { env } }",
        "reserved-name",
    );
    assert_code("let env = 1", "reserved-name");
}

/// Review RF4: `provide` begins a block at the start of an expression, so a name `provide`
/// could never be read and is refused where it is declared.
#[test]
fn provide_cannot_be_declared_as_a_name() {
    assert_code("let provide(a: int) = { a }", "reserved-name");
    assert_code("let f(provide: int) = { 1 }", "reserved-name");
    assert_clean("type Feed = { provide: string }\nlet read(f: Feed) = { f.provide }");
}

#[test]
fn a_provide_block_checks_its_overrides() {
    assert_clean(&format!(
        "{MODES}let label(): string = {{ provide env.locale=\"fr-FR\" {{ env.locale }} }}"
    ));
    assert_clean(&format!(
        "{MODES}let mode() = {{ provide env.colorMode=dark {{ env.colorMode }} }}"
    ));
    assert_code(
        &format!("{MODES}let label() = {{ provide env.locale=3 {{ env.locale }} }}"),
        "value-type-mismatch",
    );
}

#[test]
fn a_field_override_checks_against_the_field() {
    assert_clean(&format!(
        "{MODES}let a() = {{ provide env.color.accent=\"pink\" {{ env.color }} }}"
    ));
    assert_code(
        &format!("{MODES}let a() = {{ provide env.locale.region=\"FR\" {{ env.locale }} }}"),
        "environment-path",
    );
}

#[test]
fn an_element_attribute_overrides_and_is_not_a_property() {
    let source = format!(
        "{MODES}component <Card title:string /> = {{ <Text value={{title}} /> }}\n\
         external component <Text value:string />\n\
         component <Page /> = {{ <Card title=\"x\" env.colorMode=dark /> }}"
    );
    assert_clean(&source);
    let missing = format!(
        "{MODES}component <Card title:string /> = {{ <Text value={{title}} /> }}\n\
         external component <Text value:string />\n\
         component <Page /> = {{ <Card env.colorMode=dark /> }}"
    );
    assert!(!codes(&missing).is_empty());
}

#[test]
fn a_top_level_provide_is_checked() {
    assert_clean(&format!("{MODES}provide env.locale = \"fr-FR\""));
    assert_code(
        &format!("{MODES}provide env.locale = 3"),
        "value-type-mismatch",
    );
}

#[test]
fn a_bundle_checks_its_entries() {
    assert_clean(&format!(
        "{MODES}let theme: env.Update = <env.Update locale=\"fr-FR\" color.accent=\"red\" />\n\
         provide env with theme"
    ));
    assert_code(
        &format!("{MODES}let theme: env.Update = <env.Update locale=3 />"),
        "value-type-mismatch",
    );
    assert_code(
        &format!("{MODES}let theme: env.Update = <env.Update missing=3 />"),
        "unknown-environment-value",
    );
}

#[test]
fn a_bundle_takes_an_update_record_for_a_record_value() {
    assert_clean(&format!(
        "{MODES}let theme: env.Update = <env.Update color=<SemanticColors.Update accent=\"red\" /> />\n\
         provide env with theme"
    ));
}

#[test]
fn a_provide_of_something_else_is_rejected() {
    assert_code(
        &format!("{MODES}let n = 3\nprovide env with n"),
        "value-type-mismatch",
    );
}

#[test]
fn an_environment_value_named_update_is_rejected() {
    assert_code("environment Update: int = 1", "reserved-name");
}

#[test]
fn a_duplicate_environment_value_is_rejected() {
    assert_code(
        "environment space: int = 1\nenvironment space: int = 2",
        "duplicate-environment",
    );
}
