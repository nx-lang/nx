//! A written type reference must resolve to a visible type.
//!
//! One case per scenario in the `symbol-resolution-model` requirement that a written type
//! reference resolves to a visible type, plus one per declaration and annotation position and one
//! per kind of visible name. Library, import and prelude (`Range`) cases live with the workspace
//! tests in `nx-api`, where a prelude and other modules exist.

use nx_types::check_str;

fn unresolved(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some("unresolved-type"))
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

/// Asserts `source` reports exactly one `unresolved-type`, naming `name`.
fn assert_unresolved(source: &str, name: &str) -> String {
    let messages = unresolved(source);
    assert_eq!(
        messages.len(),
        1,
        "expected one unresolved-type for {name}, got: {messages:?}"
    );
    assert!(
        messages[0].contains(&format!("`{name}` is not a visible type")),
        "expected the message to name {name}, got: {messages:?}"
    );
    messages[0].clone()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

// ---------------------------------------------------------------------------------------------
// Spec scenarios
// ---------------------------------------------------------------------------------------------

/// The source text under the primary label of each `code` diagnostic, in report order.
fn labelled(source: &str, code: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some(code))
        .map(|diagnostic| {
            let label = diagnostic
                .labels()
                .iter()
                .find(|label| label.primary)
                .expect("a primary label");
            source[usize::from(label.range.start())..usize::from(label.range.end())].to_string()
        })
        .collect()
}

#[test]
fn an_unresolved_field_type_is_rejected() {
    let source = "type Broken = { id: MissingType }";
    assert_unresolved(source, "MissingType");
    assert_eq!(labelled(source, "unresolved-type"), ["MissingType"]);
}

#[test]
fn an_unresolved_parameter_type_is_rejected() {
    assert_unresolved("let f(x: MissingType) = { x }", "MissingType");
}

#[test]
fn a_misspelled_type_suggests_the_visible_name() {
    let message = assert_unresolved(
        "type Contact = { name:string }\nlet people: Contatc+ = {}",
        "Contatc",
    );
    assert!(
        message.contains("did you mean `Contact`?"),
        "expected a suggestion, got: {message}"
    );
}

#[test]
fn visible_names_of_every_kind_resolve() {
    assert_clean(
        "type Contact = { name:string }\n\
         type Status = | active | archived\n\
         type Names = string+\n\
         type Box = { T:type value:T }\n\
         type Uses = {\n\
           s:string i:int b:boolean o:object\n\
           c:Contact st:Status n:Names\n\
           p:Contact.Property\n\
           u?:Contact.Update\n\
           e?:Element\n\
         }",
    );
}

// ---------------------------------------------------------------------------------------------
// Every declaration and annotation position
// ---------------------------------------------------------------------------------------------

#[test]
fn a_union_case_field_is_checked() {
    assert_unresolved("type Shape = | circle { r:Radius } | dot", "Radius");
}

#[test]
fn an_action_field_is_checked() {
    assert_unresolved("action Rename = { name:Nme }", "Nme");
}

#[test]
fn a_component_prop_is_checked() {
    assert_unresolved("component <Card title:Txt /> = { <div /> }", "Txt");
}

#[test]
fn a_component_state_field_is_checked() {
    assert_unresolved(
        "component <Counter /> = { state { count:Count = 0 } <div /> }",
        "Count",
    );
}

#[test]
fn a_shared_emit_is_checked() {
    assert_unresolved("external component <Button emits { Tapped } />", "Tapped");
}

#[test]
fn an_inline_emit_payload_is_checked() {
    assert_unresolved(
        "external component <Button emits { Tapped { at:Instant } } />",
        "Instant",
    );
}

#[test]
fn a_return_type_is_checked() {
    assert_unresolved("let f(): Result = { 1 }", "Result");
}

#[test]
fn a_let_annotation_is_checked() {
    assert_unresolved("let x: Count = 1", "Count");
}

#[test]
fn an_alias_target_is_checked() {
    assert_unresolved("type Ids = Id+", "Id");
}

#[test]
fn a_function_type_is_checked() {
    assert_unresolved(
        "type Handlers = { onPick?:<function item:Item />: string }",
        "Item",
    );
    assert_unresolved(
        "type Handlers = { onPick?:<function item:string />: Result }",
        "Result",
    );
}

#[test]
fn an_applied_type_argument_keeps_its_own_diagnostic() {
    let source = "type Box = { T:type value:T }\ntype Bad = { b:<Box T=Contatc/> }";
    assert!(unresolved(source).is_empty(), "{:?}", unresolved(source));
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains("Contatc")),
        "{errors:?}"
    );
}

#[test]
fn each_reference_is_reported_once() {
    // An alias reached from several references is resolved, and reported, once, at the alias.
    let messages = unresolved(
        "type Ids = Id+\ntype A = { ids:Ids }\ntype B = { ids:Ids }\nlet f(ids:Ids): Ids = { ids }",
    );
    assert_eq!(messages.len(), 1, "{messages:?}");
}

// ---------------------------------------------------------------------------------------------
// Every kind of visible name
// ---------------------------------------------------------------------------------------------

#[test]
fn a_record_type_parameter_is_visible_in_its_fields() {
    assert_clean("type Page = { T:type items:T+ first?:T }");
}

#[test]
fn a_component_type_parameter_is_visible_in_its_props_and_state() {
    assert_clean(
        "component <List TItem:type items?:TItem+ /> = { state { picked?:TItem } <div /> }",
    );
}

#[test]
fn a_type_parameter_is_not_visible_outside_its_declaration() {
    assert_unresolved(
        "type Page = { T:type items:T+ }\ntype Other = { item:T }",
        "T",
    );
}

#[test]
fn component_names_and_actions_are_types() {
    assert_clean(
        "component <Card title:string /> = { <div /> }\n\
         action Rename = { name:string }\n\
         type Uses = { card:Card rename:Rename }\n\
         external component <Button emits { Rename } />",
    );
}

#[test]
fn derived_companions_resolve() {
    assert_clean(
        "type Contact = { name:string }\n\
         component <Counter /> = { state { count:int = 0 } <div /> }\n\
         type Uses = { p:Contact.Property u:Contact.Update cu?:Counter.Update cp:Counter.Property }",
    );
}

#[test]
fn a_type_declared_after_its_use_resolves() {
    assert_clean("type A = { b:B }\ntype B = { name:string }");
}

#[test]
fn an_unresolved_extends_clause_keeps_its_own_diagnostic() {
    // Lowering already rejects a base that resolves to nothing, naming the declaration that
    // extends it; that is the one report, with no `unresolved-type` on top of it.
    for (source, kind) in [
        ("type User extends Bse = { name:string }", "Record 'User'"),
        (
            "action Rename extends Bse = { name:string }",
            "Action 'Rename'",
        ),
        (
            "component <Card extends Bse /> = { <div /> }",
            "Component 'Card'",
        ),
        ("type Shape extends Bse = | circle | dot", "Union 'Shape'"),
    ] {
        assert!(
            unresolved(source).is_empty(),
            "{source}: {:?}",
            unresolved(source)
        );
        let errors = errors(source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert!(
            errors[0].contains(&format!(
                "{kind} extends 'Bse', but 'Bse' could not be resolved"
            )),
            "{source}: {errors:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Where the report goes, and what it does not pile onto
// ---------------------------------------------------------------------------------------------

#[test]
fn the_label_is_the_written_name_in_every_position() {
    for (source, name) in [
        (
            "let f(first:string, x: MissingParam): string = {\n  x\n}",
            "MissingParam",
        ),
        // A return type is underlined where it is written, not through the body after it.
        ("let f(x:string): MissingRet = {\n  x\n}", "MissingRet"),
        (
            "type Contact = { name:string }\nlet people: Contatc+ = {}",
            "Contatc",
        ),
        ("type Ids = Id+", "Id"),
        (
            "type H = { onPick?:<function item:Item />: string }",
            "Item",
        ),
        ("type Shape = | circle { r:Radius } | dot", "Radius"),
        ("component <Card title:Txt /> = { <div /> }", "Txt"),
    ] {
        assert_eq!(labelled(source, "unresolved-type"), [name], "{source}");
    }
}

#[test]
fn a_name_written_as_a_parameter_and_a_return_type_is_reported_at_each() {
    let source = "let g(x: Missing): Missing = { x }";
    let result = check_str(source, "test.nx");
    let starts: Vec<usize> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some("unresolved-type"))
        .map(|diagnostic| usize::from(diagnostic.labels()[0].range.start()))
        .collect();
    assert_eq!(starts, [9, 19], "{:?}", result.diagnostics);
}

#[test]
fn a_type_argument_is_labelled_at_the_argument() {
    let source = "type Box = { T:type value:T }\ntype Contact = { n:string }\ntype Bad = { b:<Box T=Contatc/> }";
    assert_eq!(labelled(source, "unresolved-type-argument"), ["Contatc"]);
}

#[test]
fn a_type_that_did_not_parse_is_reported_only_as_a_syntax_error() {
    // Lowering stands in for a type it could not lower; the syntax or validation error already
    // says what is wrong there, so the stand-in is not reported as a type nobody declared.
    for source in [
        "type A = { n: }",
        "let f(x: ) = { 1 }",
        "type A = ( )",
        "type A = { n:<function x: />: int }",
        "type LoadState =",
        "type T = <function TItem:type />: string",
    ] {
        assert!(
            unresolved(source).is_empty(),
            "{source}: {:?}",
            unresolved(source)
        );
        let errors = errors(source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
    }
}

#[test]
fn an_inline_payload_rejected_for_a_type_parameter_still_reports_other_names() {
    // The payload field typed by `T` is component resolution's to reject, once; every other name
    // the component's payloads write is still checked.
    for source in [
        "component <List T:type emits { Picked { item:T at:Instant } } /> = { <div /> }",
        "component <List T:type emits { Picked { item:T } Other { at:Instant } } /> = { <div /> }",
    ] {
        assert_unresolved(source, "Instant");
        let errors = errors(source);
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        assert!(
            errors
                .iter()
                .any(|message| message.contains("cannot type its payload field 'item'")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn a_union_case_is_not_a_type() {
    let message = assert_unresolved(
        "type Shape = | circle { r:int } | dot\nlet f(s: Shape.circle): int = { 1 }",
        "Shape.circle",
    );
    assert!(
        message.contains("it is a case of the union `Shape`"),
        "{message}"
    );
}

#[test]
fn a_bare_update_is_not_a_type_name() {
    // Only `<Name>.Update` names a derived update record.
    assert_unresolved(
        "component <Counter /> = { state { count:int = 0 u?:Update } <div /> }",
        "Update",
    );
}

#[test]
fn a_suggestion_is_offered_only_for_a_likely_misspelling() {
    let source = "type Contact = { n:string }\ntype Text = { n:string }\n\
                  type Rec = { a:Txt b:Paint c:Zed d:T e:Strin f:contact g:Contatc }";
    let messages = unresolved(source);
    let suggestion = |name: &str| {
        let message = messages
            .iter()
            .find(|message| message.starts_with(&format!("`{name}`")))
            .unwrap_or_else(|| panic!("{name} is reported: {messages:?}"));
        message
            .split_once("did you mean `")
            .map(|(_, rest)| rest.trim_end_matches("`?").to_string())
    };
    assert_eq!(suggestion("Txt").as_deref(), Some("Text"));
    assert_eq!(suggestion("Paint"), None, "not `int`");
    assert_eq!(suggestion("Zed"), None);
    assert_eq!(suggestion("T"), None);
    assert_eq!(suggestion("Strin").as_deref(), Some("string"));
    assert_eq!(suggestion("contact").as_deref(), Some("Contact"));
    assert_eq!(suggestion("Contatc").as_deref(), Some("Contact"));
}

#[test]
fn a_rejected_function_type_parameter_is_reported_only_by_validation() {
    // `T:type` declares nothing in a function type, so its uses there are not reported again.
    let source = "type H = { f?:<function T:type item:T />: T }";
    assert!(unresolved(source).is_empty(), "{:?}", unresolved(source));
    let errors = errors(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("Type parameter 'T' is not supported in a function type"),
        "{errors:?}"
    );
    // Outside that function type the name is an ordinary one again.
    assert_unresolved(
        "type H = { f?:<function T:type item:T />: string }\ntype K = { t:T }",
        "T",
    );
}

/// The byte offset of each `code` diagnostic's primary label, in report order.
fn label_starts(source: &str, code: &str) -> Vec<usize> {
    check_str(source, "test.nx")
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some(code))
        .map(|diagnostic| usize::from(diagnostic.labels()[0].range.start()))
        .collect()
}

#[test]
fn a_name_written_twice_in_one_reference_is_labelled_at_each_occurrence() {
    let source = "type Twice = <function a:Dup />: Dup";
    let first = source.find("Dup").unwrap();
    let second = source.rfind("Dup").unwrap();
    assert_eq!(
        label_starts(source, "unresolved-type"),
        [first, second],
        "{source}"
    );

    let source = "type Pair = { A:type B:type a:A b:B }\ntype M = { m:<Pair A=Dup B=Dup/> }";
    let first = source.find("A=Dup").unwrap() + 2;
    let second = source.find("B=Dup").unwrap() + 2;
    assert_eq!(
        label_starts(source, "unresolved-type-argument"),
        [first, second],
        "{source}"
    );

    // An argument the walk rejects before resolving still keeps the later occurrence in place.
    let source = "type Q = <function a:<Nope A=Dup/> />: Dup";
    assert_eq!(
        label_starts(source, "unresolved-type"),
        [source.rfind("Dup").unwrap()],
        "{source}"
    );
}
