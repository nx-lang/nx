//! Type checking inside a component body, and the record field reads it depends on.
//!
//! One case per scenario in the `component-syntax` and `record-type-inheritance` additions made by
//! the `add-drawnui-fiddle` change. Component bodies used to be skipped by the item loop in
//! `check_file`, which left every binding site inside one unchecked and every contextual literal
//! there unresolved.

use nx_types::check_str;

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn assert_reports(source: &str, needle: &str) {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
}

const PAINT: &str = "type Hue = Red | Green\n\
                     external component <Paint colour:Hue? />\n\
                     abstract external component <Node />\n";

#[test]
fn contextual_literal_in_a_component_body_resolves() {
    assert_clean(&format!(
        "{PAINT}component <A extends Node /> = {{ <Paint colour=Red /> }}"
    ));
}

#[test]
fn prop_default_accepts_a_contextual_literal() {
    assert_clean(&format!(
        "{PAINT}component <A extends Node hue:Hue = Red /> = {{ <Paint colour={{hue}} /> }}"
    ));
}

#[test]
fn state_default_accepts_a_contextual_literal() {
    assert_clean(&format!(
        "{PAINT}component <A extends Node /> = {{ state {{ tint:Hue = Red }} <Paint colour={{tint}} /> }}"
    ));
}

#[test]
fn property_type_mismatch_in_a_component_body_is_reported() {
    assert_reports(
        "type Alpha = Red | Green\n\
         type Beta = Red | Blue\n\
         external component <Paint colour:Alpha? />\n\
         abstract external component <Node />\n\
         component <Wrapper extends Node /> = { <Paint colour={Beta.Red} /> }",
        "colour",
    );
}

#[test]
fn a_component_body_reports_what_the_top_level_reports() {
    let preamble = "type Alpha = Red | Green\n\
                    type Beta = Red | Blue\n\
                    external component <Paint colour:Alpha? />\n\
                    abstract external component <Node />\n";
    let in_body = errors(&format!(
        "{preamble}component <Wrapper extends Node /> = {{ <Paint colour={{Beta.Red}} /> }}"
    ));
    let at_top = errors(&format!(
        "{preamble}let root() = {{ <Paint colour={{Beta.Red}} /> }}"
    ));
    assert_eq!(in_body, at_top);
}

#[test]
fn prop_default_that_does_not_match_its_declared_type_is_reported() {
    assert_reports(
        &format!("{PAINT}component <Bad extends Node hue:Hue = \"Green\" /> = {{ <Node /> }}"),
        "Default value for 'Bad.hue'",
    );
}

#[test]
fn component_props_are_not_visible_outside_the_component() {
    assert_reports(
        &format!(
            "{PAINT}component <A extends Node hue:Hue = Red /> = {{ <Paint colour={{hue}} /> }}\n\
             let root() = {{ <Paint colour={{hue}} /> }}"
        ),
        "hue",
    );
}

#[test]
fn a_declared_record_field_reads_at_its_declared_type() {
    assert_reports(
        "type User = { name:string score:int }\n\
         external component <TextInput value:string />\n\
         let show(u:User) = { <TextInput value={u.score} /> }",
        "expects string",
    );
    assert_clean(
        "type User = { name:string score:int }\n\
         external component <TextInput value:string />\n\
         let show(u:User) = { <TextInput value={u.name} /> }",
    );
}

#[test]
fn an_inherited_record_field_reads_like_a_declared_one() {
    assert_clean(
        "abstract type UserBase = { name:string }\n\
         type User extends UserBase = { role:string }\n\
         external component <TextInput value:string />\n\
         abstract external component <Node />\n\
         component <Row extends Node u:User /> = { <TextInput value={u.name} /> }",
    );
}

#[test]
fn reading_a_name_that_is_not_a_field_names_the_fields_that_exist() {
    assert_reports(
        "type User = { name:string }\n\
         external component <TextInput value:string />\n\
         let show(u:User) = { <TextInput value={u.nombre} /> }",
        "Record 'User' has no field 'nombre'; it has: name",
    );
}

#[test]
fn a_nullable_record_base_reads_its_field() {
    assert_clean(
        "type User = { name:string }\n\
         external component <TextInput value:string />\n\
         abstract external component <Node />\n\
         component <Row extends Node u:User? /> = { <TextInput value={u.name} /> }",
    );
}

#[test]
fn a_nullable_union_base_reads_its_shared_field() {
    assert_clean(
        "abstract type EventBase = { source:string }\n\
         type UiEvent extends EventBase = | clicked { x:int }\n\
         external component <TextInput value:string />\n\
         let show(e:UiEvent?) = { <TextInput value={e.source} /> }",
    );
}

#[test]
fn a_nullable_base_still_rejects_a_name_that_is_not_a_field() {
    assert_reports(
        "type User = { name:string }\n\
         external component <TextInput value:string />\n\
         let show(u:User?) = { <TextInput value={u.nombre} /> }",
        "Record 'User' has no field 'nombre'",
    );
}

const INHERIT: &str = "abstract external component <Node />\n\
                       abstract external component <Base extends Node n:int />\n\
                       external component <Txt v:string />\n\
                       external component <Leaf extends Node />\n";

#[test]
fn an_inherited_prop_reads_at_its_declared_type() {
    assert_reports(
        &format!("{INHERIT}component <A extends Base /> = {{ <Txt v={{n}} /> }}"),
        "v",
    );
}

#[test]
fn an_inherited_prop_reports_what_a_declared_one_reports() {
    let inherited = errors(&format!(
        "{INHERIT}component <A extends Base /> = {{ <Txt v={{n}} /> }}"
    ));
    let declared = errors(&format!(
        "{INHERIT}component <A extends Node n:int /> = {{ <Txt v={{n}} /> }}"
    ));
    assert_eq!(inherited, declared);
}

#[test]
fn an_inherited_prop_at_a_matching_site_is_accepted() {
    assert_clean(&format!(
        "{INHERIT}external component <Count c:int />\n\
         component <A extends Base /> = {{ <Count c={{n}} /> }}"
    ));
}

#[test]
fn a_default_naming_an_earlier_prop_is_checked_against_its_declared_type() {
    assert_reports(
        &format!("{INHERIT}component <A extends Node b:int = 1 a:string = {{b}} /> = {{ <Leaf /> }}"),
        "Default value for 'A.a'",
    );
}

#[test]
fn a_default_may_name_a_prop_declared_before_it() {
    assert_clean(&format!(
        "{INHERIT}component <A extends Node b:int = 1 a:int = {{b}} /> = {{ <Leaf /> }}"
    ));
}

#[test]
fn a_default_may_name_an_inherited_prop() {
    assert_clean(&format!(
        "{INHERIT}component <A extends Base a:int = {{n}} /> = {{ <Leaf /> }}"
    ));
}

#[test]
fn a_default_naming_a_later_prop_is_reported() {
    // A default is built where its own field is materialized, so a later field has no value to read
    // yet. Accepting the name here is what produced IR carrying an unresolved slot, which failed
    // only when the program ran.
    assert_reports(
        &format!("{INHERIT}component <A extends Node a:int = {{b}} b:int = 1 /> = {{ <Leaf /> }}"),
        "Undefined identifier 'b'",
    );
}

#[test]
fn a_default_naming_itself_is_reported() {
    assert_reports(
        &format!("{INHERIT}component <A extends Node a:int = {{a}} /> = {{ <Leaf /> }}"),
        "Undefined identifier 'a'",
    );
}

#[test]
fn a_prop_default_naming_a_state_field_is_reported() {
    // State materializes after every prop, so a prop default naming one is a forward reference
    // however the declaration is arranged.
    assert_reports(
        &format!(
            "{INHERIT}component <A extends Node a:int = {{s}} /> = \
             {{ state {{ s:int = 1 }} <Leaf /> }}"
        ),
        "Undefined identifier 's'",
    );
}

#[test]
fn a_state_default_may_name_a_prop() {
    assert_clean(&format!(
        "{INHERIT}component <A extends Node a:int = 1 /> = {{ state {{ s:int = {{a}} }} <Leaf /> }}"
    ));
}
