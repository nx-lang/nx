//! Comprehensive parser tests for NX syntax.

mod tree_helpers;

use nx_diagnostics::render_diagnostics_cli;
use nx_syntax::{parse_file, parse_str, SyntaxKind};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tree_helpers::{collect_kinds, contains_kind, contains_missing, count_kind, find_first_kind};

/// Helper to resolve test fixture paths (works from both crate and workspace root)
fn fixture_path(relative: &str) -> PathBuf {
    let from_crate = PathBuf::from("tests/fixtures").join(relative);
    let from_workspace = PathBuf::from("crates/nx-syntax/tests/fixtures").join(relative);

    if from_crate.exists() {
        from_crate
    } else {
        from_workspace
    }
}

// ============================================================================
// Valid Syntax Tests (T050)
// ============================================================================

#[test]
fn test_parse_simple_element() {
    let path = fixture_path("valid/simple-element.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse valid simple element without errors"
    );
    assert!(result.tree.is_some(), "Should produce a syntax tree");

    let root = result.root().expect("Should have root node");
    assert_eq!(root.kind(), SyntaxKind::MODULE_DEFINITION);
}

#[test]
fn test_parse_function_definition() {
    let path = fixture_path("valid/function.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse function definition without errors"
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_paren_function_definition() {
    let source = "let add(a:int, b:int): int = { a + b }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Paren-style function definition should parse"
    );
    let root = result.root().expect("Should have root node");

    let func_node = root
        .children()
        .find(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .expect("Should find function_definition node");

    let name = func_node
        .child_by_field("name")
        .expect("Function should have name field");
    assert_eq!(name.kind(), SyntaxKind::IDENTIFIER);
    assert_eq!(name.text(), "add");

    let param_count = func_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .count();
    assert_eq!(param_count, 2, "Should parse two parameters");

    assert!(
        func_node.child_by_field("return_type").is_some(),
        "Function should capture return type annotation"
    );
}

#[test]
fn test_parse_content_modifier_is_contextual_in_property_positions() {
    let source = r#"
        type Note = { content body:string title:string }
        let Wrap(title:string, content body:Element) = <section>{body}</section>
        component <Panel title:string content body:Element emits { Submitted { content payload:string } } /> = {
            state { content current:Element }
            <section>{body}</section>
        }
    "#;
    let result = parse_str(source, "content-modifier.nx");

    assert!(
        result.is_ok(),
        "Content modifiers should parse across declaration surfaces"
    );
    let root = result.root().expect("Should have root node");

    let record = root
        .children()
        .find(|child| child.kind() == SyntaxKind::RECORD_DEFINITION)
        .expect("Expected record definition");
    let record_props: Vec<_> = record
        .children()
        .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(record_props.len(), 2);
    assert_eq!(
        record_props[0]
            .child_by_field("modifier")
            .expect("Record content property should expose modifier")
            .text(),
        "content"
    );

    let function = root
        .children()
        .find(|child| {
            child.kind() == SyntaxKind::FUNCTION_DEFINITION
                && child
                    .child_by_field("name")
                    .map(|name| name.text() == "Wrap")
                    .unwrap_or(false)
        })
        .expect("Expected paren-style function definition");
    let function_params: Vec<_> = function
        .children()
        .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(function_params.len(), 2);
    assert_eq!(
        function_params[1]
            .child_by_field("modifier")
            .expect("Content parameter should expose modifier")
            .text(),
        "content"
    );

    let component = root
        .children()
        .find(|child| child.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Expected component definition");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature");
    let component_props: Vec<_> = signature
        .children()
        .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(component_props.len(), 2);
    assert_eq!(
        component_props[1]
            .child_by_field("modifier")
            .expect("Component content prop should expose modifier")
            .text(),
        "content"
    );

    let emits = signature
        .child_by_field("emits")
        .expect("Component should expose emits");
    let emit_definition = emits
        .children()
        .find(|child| child.kind() == SyntaxKind::EMIT_DEFINITION)
        .expect("Expected inline emit definition");
    let emit_payload = emit_definition
        .children()
        .find(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .expect("Expected emit payload property");
    assert_eq!(
        emit_payload
            .child_by_field("modifier")
            .expect("Emit payload content property should expose modifier")
            .text(),
        "content"
    );

    let component_body = component
        .child_by_field("body")
        .expect("Component should expose body");
    let state = component_body
        .child_by_field("state")
        .expect("Component body should expose state");
    let state_field = state
        .children()
        .find(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .expect("Expected state property");
    assert_eq!(
        state_field
            .child_by_field("modifier")
            .expect("State content property should expose modifier")
            .text(),
        "content"
    );
}

#[test]
fn test_parse_content_remains_identifier_outside_modifier_position() {
    let source = r#"
        type Note = { content:string }
        let render(content:string) = { content }
    "#;
    let result = parse_str(source, "content-contextual.nx");

    assert!(
        result.is_ok(),
        "Content should remain a valid identifier outside modifier position"
    );
    let root = result.root().expect("Should have root node");

    let record = root
        .children()
        .find(|child| child.kind() == SyntaxKind::RECORD_DEFINITION)
        .expect("Expected record definition");
    let property = record
        .children()
        .find(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .expect("Expected property definition");
    assert!(
        property.child_by_field("modifier").is_none(),
        "Property named content should not be treated as a modifier"
    );
    assert_eq!(
        property
            .child_by_field("name")
            .expect("Property should expose name")
            .text(),
        "content"
    );

    let function = root
        .children()
        .find(|child| child.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .expect("Expected function definition");
    let param = function
        .children()
        .find(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .expect("Expected parameter");
    assert!(
        param.child_by_field("modifier").is_none(),
        "Parameter named content should not be treated as a modifier"
    );
    assert_eq!(
        param
            .child_by_field("name")
            .expect("Parameter should expose name")
            .text(),
        "content"
    );
}

#[test]
fn test_parse_regular_element_mixed_content_preserves_text_runs() {
    let source = r#"
        let value = 1
        let root() = <Panel>label text<Badge />{value}</Panel>
    "#;
    let result = parse_str(source, "mixed-content.nx");

    assert!(result.is_ok(), "Regular element mixed content should parse");
    let root = result.root().expect("Should have root node");
    let function = root
        .children()
        .find(|child| {
            child.kind() == SyntaxKind::FUNCTION_DEFINITION
                && child
                    .child_by_field("name")
                    .map(|name| name.text() == "root")
                    .unwrap_or(false)
        })
        .expect("Expected root function");
    let body = function
        .child_by_field("body")
        .expect("Function should expose body");
    let element = body
        .children()
        .find(|child| child.kind() == SyntaxKind::ELEMENT)
        .expect("Function body should contain element");
    let mixed_content = element
        .child_by_field("content")
        .expect("Element should expose mixed content");
    assert_eq!(mixed_content.kind(), SyntaxKind::MIXED_CONTENT);

    let child_kinds: Vec<_> = mixed_content.children().map(|child| child.kind()).collect();
    assert_eq!(
        child_kinds,
        vec![
            SyntaxKind::TEXT_RUN,
            SyntaxKind::ELEMENT,
            SyntaxKind::VALUES_BRACED_EXPRESSION
        ]
    );
}

#[test]
fn test_parse_element_function_with_return_type() {
    let source = r#"let <Button text:string />: Element = <button>{text}</button>"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Element-style function with return type should parse"
    );
    let root = result.root().expect("Should have root node");

    let func_node = root
        .children()
        .find(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .expect("Should find function_definition node");

    let name = func_node
        .child_by_field("name")
        .expect("Function should have name field");
    assert_eq!(name.text(), "Button");

    let return_type = func_node
        .child_by_field("return_type")
        .expect("Function should capture return type annotation");
    assert_eq!(return_type.kind(), SyntaxKind::TYPE);
    assert_eq!(return_type.text(), "Element");
}

#[test]
fn test_parse_paren_function_without_return_type() {
    let source = "let subtract(a:int, b:int) = { a - b }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Paren function without return type should parse"
    );
    let root = result.root().expect("Should have root node");

    let func_node = root
        .children()
        .find(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .expect("Should find function_definition node");

    assert!(
        func_node.child_by_field("return_type").is_none(),
        "Return type should be optional for paren functions"
    );
}

#[test]
fn test_parse_component_definition() {
    let path = fixture_path("valid/component-minimal.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Component fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");

    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    assert_eq!(signature.kind(), SyntaxKind::COMPONENT_SIGNATURE);

    let name = signature
        .child_by_field("name")
        .expect("Component signature should expose name");
    assert_eq!(name.text(), "Button");

    let prop_count = signature
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .count();
    assert_eq!(prop_count, 1, "Component should parse one prop");

    let body = component
        .child_by_field("body")
        .expect("Component should expose body field");
    assert_eq!(body.kind(), SyntaxKind::COMPONENT_BODY);

    let body_expr = body
        .child_by_field("body")
        .expect("Component body should expose rendered expression");
    assert!(
        contains_kind(&body_expr, SyntaxKind::ELEMENT),
        "Component body should contain the rendered element"
    );
}

#[test]
fn test_parse_abstract_component_definition() {
    let path = fixture_path("valid/component-abstract.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Abstract component fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");

    assert!(
        component.child_by_field("abstract").is_some(),
        "Abstract component should expose abstract modifier"
    );
    assert!(
        component.child_by_field("external").is_none(),
        "Abstract-only component should omit external modifier"
    );
    assert!(
        component.child_by_field("body").is_none(),
        "Abstract component should be bodyless"
    );
}

#[test]
fn test_parse_external_component_definition() {
    let path = fixture_path("valid/component-external.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "External component fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");

    assert!(
        component.child_by_field("external").is_some(),
        "External component should expose external modifier"
    );
    assert!(
        component.child_by_field("body").is_none(),
        "External component should be bodyless"
    );
}

#[test]
fn test_parse_external_component_with_state_only_body() {
    let path = fixture_path("valid/component-external-state.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "External component state-only fixture should parse"
    );
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let body = component
        .child_by_field("body")
        .expect("External component state-only body should be preserved");
    let state = body
        .child_by_field("state")
        .expect("State-only external body should expose state");

    assert_eq!(state.kind(), SyntaxKind::STATE_GROUP);
    assert!(
        body.child_by_field("body").is_none(),
        "State-only external body should omit a rendered body expression"
    );
}

#[test]
fn test_parse_abstract_external_component_definition() {
    let path = fixture_path("valid/component-abstract-external.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Abstract external component fixture should parse"
    );
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");

    assert!(
        component.child_by_field("abstract").is_some(),
        "Abstract external component should expose abstract modifier"
    );
    assert!(
        component.child_by_field("external").is_some(),
        "Abstract external component should expose external modifier"
    );
    assert!(
        component.child_by_field("body").is_none(),
        "Abstract external component should be bodyless"
    );
}

#[test]
fn test_parse_component_definition_with_base() {
    let path = fixture_path("valid/component-extends.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Derived component fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");

    assert_eq!(
        signature
            .child_by_field("base")
            .expect("Derived component should expose base")
            .text(),
        "SearchBase"
    );
    assert!(
        component.child_by_field("body").is_some(),
        "Concrete derived component should preserve its body"
    );
}

#[test]
fn test_parse_component_with_emits() {
    let path = fixture_path("valid/component-emits.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Component emits fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");

    let emits = signature
        .child_by_field("emits")
        .expect("Component signature should expose emits group");
    assert_eq!(emits.kind(), SyntaxKind::EMITS_GROUP);

    let emit_defs: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_DEFINITION)
        .collect();
    assert_eq!(
        emit_defs.len(),
        2,
        "Expected two emitted action definitions"
    );

    let first = emit_defs[0];
    assert_eq!(
        first
            .child_by_field("name")
            .expect("Emit definition should expose name")
            .text(),
        "ValueChanged"
    );
    let first_fields: Vec<_> = first
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(first_fields.len(), 2, "Expected two payload fields");

    assert!(
        root.children().any(|c| c.kind() == SyntaxKind::ELEMENT),
        "Fixture should retain a component invocation after the definition"
    );
}

#[test]
fn test_parse_component_with_emits_inherited_action() {
    let path = fixture_path("valid/component-emits-inherited-action.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Component emits inheritance fixture should parse"
    );
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    let emits = signature
        .child_by_field("emits")
        .expect("Component signature should expose emits group");

    let emit_defs: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_DEFINITION)
        .collect();
    assert_eq!(emit_defs.len(), 1, "Expected one emitted action definition");

    let emit_def = emit_defs[0];
    assert_eq!(
        emit_def
            .child_by_field("name")
            .expect("Emit definition should expose name")
            .text(),
        "ValueChanged"
    );
    assert_eq!(
        emit_def
            .child_by_field("base")
            .expect("Emit definition should expose base")
            .text(),
        "InputAction"
    );
}

#[test]
fn test_parse_component_with_emits_reference() {
    let path = fixture_path("valid/component-emits-reference.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Component emits reference fixture should parse"
    );
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    let emits = signature
        .child_by_field("emits")
        .expect("Component signature should expose emits group");

    let emit_references: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_REFERENCE)
        .collect();
    assert_eq!(
        emit_references.len(),
        1,
        "Expected one emitted action reference"
    );
    assert_eq!(
        emit_references[0]
            .child_by_field("name")
            .expect("Emit reference should expose name")
            .text(),
        "ActionSharedWithMultipleComponents"
    );
}

#[test]
fn test_parse_component_with_qualified_emits_reference() {
    let path = fixture_path("valid/component-emits-qualified-reference.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Qualified component emits reference fixture should parse"
    );
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    let emits = signature
        .child_by_field("emits")
        .expect("Component signature should expose emits group");

    let emit_references: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_REFERENCE)
        .collect();
    assert_eq!(
        emit_references.len(),
        1,
        "Expected one qualified emitted action reference"
    );
    assert_eq!(
        emit_references[0]
            .child_by_field("name")
            .expect("Emit reference should expose name")
            .text(),
        "SharedActions.SearchSubmitted"
    );
}

#[test]
fn test_parse_component_with_mixed_emits_entries() {
    let path = fixture_path("valid/component-emits-mixed.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Mixed emits fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    let emits = signature
        .child_by_field("emits")
        .expect("Component signature should expose emits group");

    let emit_defs: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_DEFINITION)
        .collect();
    let emit_refs: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_REFERENCE)
        .collect();

    assert_eq!(emit_defs.len(), 1, "Expected one inline emitted action");
    assert_eq!(emit_refs.len(), 1, "Expected one emitted action reference");
    assert_eq!(
        emit_defs[0]
            .child_by_field("name")
            .expect("Emit definition should expose name")
            .text(),
        "MyAction"
    );
    assert_eq!(
        emit_refs[0]
            .child_by_field("name")
            .expect("Emit reference should expose name")
            .text(),
        "ActionSharedWithMultipleComponents"
    );
}

#[test]
fn test_parse_component_with_state() {
    let path = fixture_path("valid/component-state.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Component state fixture should parse");
    let root = result.root().expect("Should have root node");

    let top_level_kinds: Vec<_> = root.children().map(|child| child.kind()).collect();
    assert!(
        top_level_kinds.contains(&SyntaxKind::IMPORT_STATEMENT),
        "Fixture should retain top-level imports"
    );
    assert!(
        top_level_kinds.contains(&SyntaxKind::COMPONENT_DEFINITION),
        "Fixture should contain a component definition"
    );
    assert!(
        top_level_kinds.contains(&SyntaxKind::ELEMENT),
        "Fixture should retain a trailing root element"
    );

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let body = component
        .child_by_field("body")
        .expect("Component should expose body field");

    let state = body
        .child_by_field("state")
        .expect("Component body should expose state group");
    assert_eq!(state.kind(), SyntaxKind::STATE_GROUP);

    let state_fields: Vec<_> = state
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(state_fields.len(), 1, "Expected one state field");
    assert_eq!(
        state_fields[0]
            .child_by_field("name")
            .expect("State property should expose name")
            .text(),
        "query"
    );

    let rendered = body
        .child_by_field("body")
        .expect("Component body should expose rendered expression");
    assert!(
        contains_kind(&rendered, SyntaxKind::VALUE_IF_SIMPLE_EXPRESSION),
        "Component body should preserve the conditional render expression"
    );
}

#[test]
fn test_parse_component_full_syntax_with_default_prop() {
    let path = fixture_path("valid/component-full.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Full component fixture should parse");
    let root = result.root().expect("Should have root node");

    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    let body = component
        .child_by_field("body")
        .expect("Component should expose body field");

    let props: Vec<_> = signature
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(props.len(), 1, "Expected one prop definition");
    assert!(
        props[0].child_by_field("default").is_some(),
        "Component prop default should be preserved"
    );

    let emits = signature
        .child_by_field("emits")
        .expect("Component should expose emits group");
    let emit_defs: Vec<_> = emits
        .children()
        .filter(|c| c.kind() == SyntaxKind::EMIT_DEFINITION)
        .collect();
    assert_eq!(emit_defs.len(), 2, "Expected two emit definitions");

    let state = body
        .child_by_field("state")
        .expect("Component should expose state group");
    let state_fields: Vec<_> = state
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(state_fields.len(), 1, "Expected one state field");
}

#[test]
fn test_parse_nested_elements() {
    let path = fixture_path("valid/nested-elements.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse nested elements without errors"
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_type_annotations() {
    let path = fixture_path("valid/type-annotations.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse type annotations without errors"
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_occurrence_type_suffixes() {
    let source = r#"
        type Maybe = string?
        type Names = string+
        type Tags = string*

        type SearchState = {
          queries?: string+
          aliases: string*
          grouped?: string
        }

        let loadUsers(users: User+): User* = {users}
        let maybeUsers(): User? = {}
    "#;
    let result = parse_str(source, "occurrence-types.nx");

    assert!(
        result.is_ok(),
        "Occurrence suffixes should parse. Errors: {:?}",
        result.errors
    );
    let root = result.root().expect("Should have syntax tree root");

    let aliases: Vec<_> = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::TYPE_DEFINITION)
        .map(|alias| {
            alias
                .child_by_field("type")
                .expect("alias type")
                .text()
                .to_string()
        })
        .collect();
    assert_eq!(aliases, vec!["string?", "string+", "string*"]);

    let record = root
        .children()
        .find(|c| c.kind() == SyntaxKind::RECORD_DEFINITION)
        .expect("Should find record definition");
    let fields: Vec<_> = record
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .map(|field| {
            (
                field.child_by_field("optional").is_some(),
                field
                    .child_by_field("type")
                    .expect("Field should have type")
                    .text()
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        fields,
        vec![
            (true, "string+".to_string()),
            (false, "string*".to_string()),
            (true, "string".to_string()),
        ]
    );

    let maybe_users = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .find(|func| {
            func.child_by_field("name")
                .is_some_and(|name| name.text() == "maybeUsers")
        })
        .expect("Should find maybeUsers function_definition node");
    let return_type = maybe_users
        .child_by_field("return_type")
        .expect("Function should capture return type annotation");
    assert_eq!(return_type.text(), "User?");
}

#[test]
fn test_parse_record_definition() {
    let path = fixture_path("valid/record-definition.nx");
    let result = parse_file(&path).expect("record fixture should load");

    assert!(result.is_ok(), "Record definition should parse");
    let root = result.root().expect("Should have syntax tree root");
    assert!(
        contains_kind(&root, SyntaxKind::RECORD_DEFINITION),
        "Should contain record_definition node"
    );

    let record_node = root
        .children()
        .find(|c| c.kind() == SyntaxKind::RECORD_DEFINITION)
        .expect("Should find record_definition");
    let prop_count = record_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .count();
    assert_eq!(prop_count, 3, "Should parse three record fields");
}

#[test]
fn test_parse_record_inheritance_definition() {
    let path = fixture_path("valid/record-inheritance.nx");
    let result = parse_file(&path).expect("record inheritance fixture should load");

    assert!(result.is_ok(), "Record inheritance fixture should parse");
    let root = result.root().expect("Should have syntax tree root");

    let records: Vec<_> = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::RECORD_DEFINITION)
        .collect();
    assert_eq!(records.len(), 3, "Expected three record definitions");

    let entity = records[0];
    assert!(entity.child_by_field("abstract").is_some());
    assert!(entity.child_by_field("base").is_none());

    let user_base = records[1];
    assert!(user_base.child_by_field("abstract").is_some());
    assert_eq!(
        user_base
            .child_by_field("base")
            .expect("Expected base field")
            .text(),
        "Entity"
    );

    let user = records[2];
    assert!(user.child_by_field("abstract").is_none());
    assert_eq!(
        user.child_by_field("base")
            .expect("Expected concrete base field")
            .text(),
        "UserBase"
    );
}

#[test]
fn test_parse_action_definition() {
    let path = fixture_path("valid/action-definition.nx");
    let result = parse_file(&path).expect("action fixture should load");

    assert!(result.is_ok(), "Action definition should parse");
    let root = result.root().expect("Should have syntax tree root");
    assert!(
        contains_kind(&root, SyntaxKind::ACTION_DEFINITION),
        "Should contain action_definition node"
    );

    let action_node = root
        .children()
        .find(|c| c.kind() == SyntaxKind::ACTION_DEFINITION)
        .expect("Should find action_definition");
    let prop_count = action_node
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .count();
    assert_eq!(prop_count, 2, "Should parse two action fields");
}

#[test]
fn test_parse_action_inheritance_definition() {
    let path = fixture_path("valid/action-inheritance.nx");
    let result = parse_file(&path).expect("action inheritance fixture should load");

    assert!(result.is_ok(), "Action inheritance fixture should parse");
    let root = result.root().expect("Should have syntax tree root");

    let actions: Vec<_> = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::ACTION_DEFINITION)
        .collect();
    assert_eq!(actions.len(), 3, "Expected three action definitions");

    let input_action = actions[0];
    assert!(input_action.child_by_field("abstract").is_some());
    assert!(input_action.child_by_field("base").is_none());

    let search_action = actions[1];
    assert!(search_action.child_by_field("abstract").is_some());
    assert_eq!(
        search_action
            .child_by_field("base")
            .expect("Expected base field")
            .text(),
        "InputAction"
    );

    let submitted = actions[2];
    assert!(submitted.child_by_field("abstract").is_none());
    assert_eq!(
        submitted
            .child_by_field("base")
            .expect("Expected concrete base field")
            .text(),
        "SearchAction"
    );
}

#[test]
fn test_parse_expressions() {
    let path = fixture_path("valid/expressions.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse various expressions without errors"
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_conditionals() {
    let path = fixture_path("valid/conditionals.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse conditional expressions without errors"
    );
    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::ELEMENTS_BRACED_EXPRESSION),
        "Element conditionals should use elements braced expression nodes"
    );
}

#[test]
fn test_parse_markup_interpolation_items() {
    let path = fixture_path("valid/markup-interpolation.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should allow interpolation items between markup children"
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION),
        "Values braced expression should appear in the markup tree"
    );
}

#[test]
fn test_parse_text_elements_and_embed_interpolation() {
    let path = fixture_path("valid/text-elements.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Typed and raw text element variants should parse"
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION),
        "Expected embed braced expression parsed from @{{...}}"
    );
    assert!(
        contains_kind(&root, SyntaxKind::RAW_TEXT_RUN),
        "Expected raw text run inside raw text elements"
    );
}

#[test]
fn test_parse_braced_value_sequence_fixture() {
    let path = fixture_path("valid/braced-value-sequences.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Braced value sequence fixture should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION),
        "Expected plain braced value sequences in the fixture"
    );
    assert!(
        contains_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION),
        "Expected embed braced value sequences in the fixture"
    );
    assert!(
        contains_kind(&root, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION),
        "Expected value list item nodes in the fixture"
    );
}

#[test]
fn test_parse_space_delimited_braced_lists() {
    let source = r#"
let single = {item}
let items = {first second third}
"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Space-delimited braced value lists should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced_count = count_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION);
    assert_eq!(
        braced_count, 2,
        "Expected both singleton and list braced expressions"
    );

    let list_item_count = count_kind(&root, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION);
    assert!(
        list_item_count >= 4,
        "Expected singleton and list brace items to be represented, found {}",
        list_item_count
    );
}

#[test]
fn test_parse_text_and_embed_braced_lists() {
    let source = r#"
<Root>
  <message:>
    Hello {first second}
  </message>

  <markdown:text>
    Welcome @{user title}
  </markdown>
</Root>
"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Text/embed braced value lists should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION),
        "Text content should include values braced expressions"
    );
    assert!(
        contains_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION),
        "Embed text content should include embed braced expressions"
    );
}

#[test]
fn test_parse_empty_braced_value_at_annotated_let() {
    let source = "let value:string* = {}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Empty braced value should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    assert_eq!(
        count_kind(&braced, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION),
        0,
        "Empty braces should have no list items"
    );
    // The real assertion: before the empty form was admitted this source still produced a
    // `values_braced_expression`, but only because recovery inserted a zero-width MISSING
    // identifier inside it. Absence of that node is what distinguishes parsing from recovering.
    assert!(
        !contains_missing(&braced),
        "Empty braces should parse outright, not by inserting a missing node"
    );
}

#[test]
fn test_parse_empty_braced_value_in_property_position() {
    let source = "let c = <Img fits={} />";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Empty braced property value should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    assert_eq!(
        count_kind(&braced, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION),
        0,
        "Empty braces should have no list items"
    );
    assert!(!contains_missing(&braced), "Expected no recovery node");
}

#[test]
fn test_parse_empty_braced_value_in_markup_child_position() {
    let source = "component <N /> = { <List>{}</List> }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Empty braced child content should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION),
        "Markup child content should be a values braced expression"
    );
    assert!(!contains_missing(&root), "Expected no recovery node");
}

#[test]
fn test_parse_empty_element_body_is_still_rejected() {
    // `elements_braced_expression` is a separate rule over a `repeat1`, so admitting the empty
    // values brace deliberately does not reach element-position bodies.
    let source = "component <N r:boolean /> = { <div>if r {} else { <B/> }</div> }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.has_errors(),
        "An empty element-position `if` body should still be a parse error"
    );
}

#[test]
fn test_parse_empty_for_body_is_still_rejected() {
    let source = "component <N /> = { <div>for x in xs {}</div> }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.has_errors(),
        "An empty element-position `for` body should still be a parse error"
    );
}

/// The empty form reaches every rule that references `values_braced_expression`, not only the
/// positions the design set out to change. Value-position `if` branches, a value-position `for`
/// body, and match and condition arm bodies are all that rule, so `{}` parses in each -- a parse
/// error in all of them before this change. They are pinned here because what parses is what the
/// type checker then has to have an answer for.
#[test]
fn test_parse_empty_braced_value_in_value_position_control_flow() {
    let sources = [
        "let pick(c:boolean): string* = {if c {\"a\" \"b\"} else {}}",
        "let pick(c:boolean): string* = {if { c => {} else => {\"a\" \"b\"} }}",
        "let xs:string* = {for y in ys {}}",
    ];

    for source in sources {
        let result = parse_str(source, "test.nx");
        assert!(
            result.is_ok(),
            "`{{}}` should parse in a value-position body: {source}. Errors: {:?}",
            result.errors
        );

        let root = result.root().expect("Should have root node");
        assert!(
            !contains_missing(&root),
            "Expected no recovery node in: {source}"
        );
    }
}

/// `{}` is an item and an operand, not only a whole braced value. Each form below failed with
/// "Unclosed brace" or a syntax error before the empty brace was admitted as an item.
#[test]
fn test_parse_empty_braced_value_as_an_item_and_operand() {
    // (source, braced values in total, of which empty)
    for (source, total, empty) in [
        ("let xs:string* = { \"a\" {} }", 2, 1),
        ("let e = {none == {}}", 2, 1),
        ("let f(c:boolean): int? = { if c { {} } else { 1 } }", 4, 1),
        ("let g:int* = { {} {} }", 3, 2),
        ("let h:int? = { {} }", 2, 1),
        ("let k(x:int?): int = { {} ?? x ?? 1 }", 2, 1),
        ("let r = <div>{ {} }</div>", 2, 1),
        ("let items = {{} b}", 2, 1),
        ("let v = <Box items={{}} />", 2, 1),
    ] {
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{source}: {:?}", result.errors);

        let root = result.root().expect("Should have root node");
        assert!(
            !contains_missing(&root),
            "Expected no recovery node in: {source}"
        );

        let mut braces = Vec::new();
        collect_kinds(&root, SyntaxKind::VALUES_BRACED_EXPRESSION, &mut braces);
        assert_eq!(braces.len(), total, "{source}");
        let empties = braces
            .iter()
            .filter(|brace| brace.children().next().is_none())
            .count();
        assert_eq!(empties, empty, "{source}");
    }
}

/// Where a full braced value is admitted anyway (a match arm body, a call argument), `{}` stays
/// that braced value rather than a braced value wrapped in an item.
#[test]
fn test_parse_empty_braced_value_prefers_the_full_brace_where_both_fit() {
    let source = "let p(x:int?): int? = { if x is { {} => {} else => 1 } }";
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "{:?}", result.errors);

    let root = result.root().expect("Should have root node");
    let arm = find_first_kind(&root, SyntaxKind::VALUE_IF_MATCH_ARM).expect("Expected an arm");
    let body = arm.child_by_field("body").expect("Expected an arm body");
    assert_eq!(body.kind(), SyntaxKind::VALUES_BRACED_EXPRESSION);
}

#[test]
fn test_parse_empty_interpolation_is_still_rejected() {
    let source = "component <N /> = { <p:html>Hi @{}</p> }";
    let result = parse_str(source, "test.nx");

    assert!(
        result.has_errors(),
        "An empty interpolation `@{{}}` should still be a parse error"
    );
}

#[test]
fn test_parse_empty_braced_value_as_a_call_argument() {
    // A call argument takes a braced value on its own rule, so a function is passed a list the
    // same way a property is bound one.
    let source = "let n = {count({})}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "An empty braced call argument should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let call = find_first_kind(&root, SyntaxKind::CALL_EXPRESSION).expect("Expected a call");
    let argument = find_first_kind(&call, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected the argument to be a values braced expression");
    assert_eq!(
        count_kind(&argument, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION),
        0,
        "Empty braces should have no list items"
    );
    assert!(!contains_missing(&argument), "Expected no recovery node");
}

#[test]
fn test_parse_braced_list_as_a_call_argument() {
    let source = "let n = {count({\"a\" \"b\"})}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "A braced list call argument should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let call = find_first_kind(&root, SyntaxKind::CALL_EXPRESSION).expect("Expected a call");
    let argument = find_first_kind(&call, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected the argument to be a values braced expression");
    assert_eq!(
        count_kind(&argument, SyntaxKind::VALUE_LIST_ITEM_EXPRESSION),
        2,
        "The braced argument should keep both items"
    );
}

#[test]
fn test_parse_braced_values_in_every_argument_position() {
    // Each argument is admitted independently, so a braced value is not restricted to the first
    // one and mixes freely with ordinary expressions.
    let source = "let n = {pick({}, x, {\"a\" \"b\"})}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Braced values should parse in any argument position. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let call = find_first_kind(&root, SyntaxKind::CALL_EXPRESSION).expect("Expected a call");
    assert_eq!(
        count_kind(&call, SyntaxKind::VALUES_BRACED_EXPRESSION),
        2,
        "Expected both braced arguments"
    );
}

#[test]
fn test_parse_braced_value_is_still_not_a_list_item() {
    // Admitting the brace as an argument deliberately does not admit it as a list item: a
    // non-empty list is still not an item of a list, at any arity. The empty brace `{}` is the
    // exception (`test_parse_empty_braced_value_as_an_item_and_operand`): a sequence is flat, so
    // `{{} b}` is `{b}` and `{{}}` is `{}`.
    for source in [
        "let items = {{\"a\"} b}",
        "let v = <Box items={{\"a\" \"b\"}} />",
        "let v = <Box items={{\"a\"}} />",
    ] {
        let result = parse_str(source, "test.nx");
        assert!(
            result.has_errors(),
            "A braced value nested in a braced value should still be a parse error: {}",
            source
        );
    }
}

#[test]
fn test_parse_braced_value_is_still_not_a_list_item_inside_an_argument() {
    // The argument rule admits one brace, not a brace whose items are braces.
    let source = "let n = {count({{\"a\"} \"b\"})}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.has_errors(),
        "A braced list item inside a braced argument should still be a parse error"
    );
}

#[test]
fn test_parse_singleton_binary_braced_expression() {
    let source = "let arithmetic = {a - b}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Singleton binary braced expression should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    assert!(
        contains_kind(&braced, SyntaxKind::BINARY_EXPRESSION),
        "Singleton braces should still allow binary expressions without forcing list syntax"
    );
}

#[test]
fn test_parse_singleton_call_expression_braced_expression() {
    let source = "let value = {double(add(n, 1))}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Singleton call expression in braces should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    assert!(
        count_kind(&braced, SyntaxKind::CALL_EXPRESSION) >= 2,
        "Singleton call braces should preserve both the outer and inner call expressions"
    );
}

#[test]
fn test_parse_parenthesized_binary_list_item() {
    let source = "let items = {(a - b) c}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Parenthesized binary list items should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    let list_item_count = braced
        .children()
        .filter(|child| child.kind() == SyntaxKind::VALUE_LIST_ITEM_EXPRESSION)
        .count();
    assert_eq!(
        list_item_count, 2,
        "Expected two list items in the parenthesized binary list"
    );
}

#[test]
fn test_parse_parenthesized_prefix_unary_list_item() {
    let source = "let items = {(-x) y}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Parenthesized prefix-unary list items should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    let list_item_count = braced
        .children()
        .filter(|child| child.kind() == SyntaxKind::VALUE_LIST_ITEM_EXPRESSION)
        .count();
    assert_eq!(
        list_item_count, 2,
        "Expected two list items in the parenthesized prefix-unary list"
    );
    assert!(
        contains_kind(&braced, SyntaxKind::PREFIX_UNARY_EXPRESSION),
        "Expected prefix unary expression to remain inside the parenthesized list item"
    );
}

#[test]
fn test_parse_rejects_unparenthesized_binary_list_item() {
    let path = fixture_path("invalid/braced-value-sequence-requires-parens.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Non-parenthesized binary list items should fail to parse"
    );
    assert!(
        !result.errors.is_empty(),
        "Expected parse errors for non-parenthesized binary list items"
    );
}

#[test]
fn test_parse_rejects_unparenthesized_prefix_unary_list_item() {
    let source = "let items = {-x y}";
    let result = parse_str(source, "test.nx");

    assert!(
        !result.is_ok(),
        "Non-parenthesized prefix-unary list items should fail to parse"
    );
    assert!(
        !result.errors.is_empty(),
        "Expected parse errors for non-parenthesized prefix-unary list items"
    );
}

#[test]
fn test_parse_embed_braced_element_list() {
    let source = r#"
<Root>
  <markdown:text>@{<A/> <B/>}</markdown>
</Root>
"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Embed braced element lists should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let embed_braced = find_first_kind(&root, SyntaxKind::EMBED_BRACED_EXPRESSION)
        .expect("Expected embed braced expression");
    let list_item_count = embed_braced
        .children()
        .filter(|child| child.kind() == SyntaxKind::VALUE_LIST_ITEM_EXPRESSION)
        .count();
    assert_eq!(
        list_item_count, 2,
        "Expected two list items in the embed braced element list"
    );
}

#[test]
fn test_parse_rejects_nested_braced_value_sequences() {
    // Not pending: at arity one the brace is a scalar, so a nested brace would collapse and a
    // one-row list of lists would have no spelling while a two-row one did.
    let source = "let items = {{a b} {c d}}";
    let result = parse_str(source, "test.nx");

    assert!(
        !result.is_ok(),
        "A braced value is not an item of a braced value: a list is not an item of a list"
    );
    assert!(
        !result.errors.is_empty(),
        "Expected parse errors for nested braced value sequences"
    );
}

#[test]
fn test_parse_accepts_empty_braced_expression() {
    // Superseded `test_parse_rejects_empty_braced_expression`: `{}` is now the spelling of the
    // empty list. Typing it still requires an expected type, but that is a type-checking rule.
    let source = "let items = {}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Empty braced expressions should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION)
        .expect("Expected values braced expression");
    assert!(!contains_missing(&braced), "Expected no recovery node");
}

#[test]
fn test_parse_text_child_elements() {
    let path = fixture_path("valid/text-child-elements.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Text child elements should parse without errors. Errors:\n{}",
        {
            let mut sources = HashMap::new();
            sources.insert(
                "test.nx".to_string(),
                std::fs::read_to_string(&path).unwrap_or_default(),
            );
            render_diagnostics_cli(&result.errors, &sources)
        }
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::TEXT_CHILD_ELEMENT),
        "Expected text_child_element nodes in the tree"
    );
    assert!(
        contains_kind(&root, SyntaxKind::TEXT_CONTENT),
        "Expected text_content nodes in the tree"
    );
}

#[test]
fn test_text_child_element_simple() {
    let source = "<p:>Hello <b>world</b>!</p>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Simple text child element should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::TEXT_CHILD_ELEMENT),
        "Should contain text_child_element node for <b>world</b>"
    );
}

#[test]
fn test_text_child_element_self_closing() {
    let source = "<p:>Line<br />break</p>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Self-closing text child element should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::TEXT_CHILD_ELEMENT),
        "Should contain text_child_element node for <br />"
    );
}

#[test]
fn test_text_child_element_nested() {
    let source = "<p:>Start <b>bold <i>italic</i> bold</b> end</p>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Nested text child elements should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");

    let text_child_count = count_kind(&root, SyntaxKind::TEXT_CHILD_ELEMENT);
    assert!(
        text_child_count >= 2,
        "Should have at least 2 text_child_element nodes, found {}",
        text_child_count
    );
}

#[test]
fn test_text_child_element_with_properties() {
    let source = r#"<p:>Click <a href="link">here</a></p>"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Text child element with properties should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    assert!(
        contains_kind(&root, SyntaxKind::TEXT_CHILD_ELEMENT),
        "Should contain text_child_element node"
    );
    assert!(
        contains_kind(&root, SyntaxKind::PROPERTY_VALUE),
        "Should contain property_value node for href attribute"
    );
}

#[test]
fn test_parse_complex_example() {
    let path = fixture_path("valid/complex-example.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse complex example without errors"
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_module_with_definitions_and_element() {
    let path = fixture_path("valid/module-with-definitions-and-element.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse module that mixes declarations and a root element"
    );
    let root = result.root().expect("Should have root node");

    let kinds: Vec<SyntaxKind> = root.children().map(|child| child.kind()).collect();
    assert!(
        kinds.contains(&SyntaxKind::TYPE_DEFINITION),
        "Expected at least one type definition"
    );
    assert!(
        kinds.contains(&SyntaxKind::VALUE_DEFINITION),
        "Expected at least one value definition"
    );
    assert!(
        kinds.contains(&SyntaxKind::FUNCTION_DEFINITION),
        "Expected at least one function definition"
    );

    let last = *kinds.last().expect("Module should have at least one child");
    assert!(
        matches!(last, SyntaxKind::ELEMENT | SyntaxKind::SELF_CLOSING_ELEMENT),
        "Expected trailing root element, found {:?}",
        last
    );
}

#[test]
fn test_parse_wildcard_and_namespace_imports() {
    let source = r#"import "./tokens"
import "../ui" as UI"#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Wildcard/namespace imports should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let imports: Vec<_> = root
        .children()
        .filter(|child| child.kind() == SyntaxKind::IMPORT_STATEMENT)
        .collect();
    assert_eq!(imports.len(), 2);

    let wildcard_kind = imports[0]
        .child_by_field("kind")
        .expect("Wildcard import should expose kind");
    assert_eq!(wildcard_kind.kind(), SyntaxKind::WILDCARD_IMPORT);
    assert!(
        wildcard_kind.child_by_field("alias").is_none(),
        "Bare wildcard import should not have alias"
    );
    assert_eq!(
        wildcard_kind
            .child_by_field("path")
            .expect("Wildcard import should expose library path")
            .child_by_field("value")
            .expect("library_path should expose value")
            .text(),
        r#""./tokens""#
    );

    let namespace_kind = imports[1]
        .child_by_field("kind")
        .expect("Namespace import should expose kind");
    assert_eq!(namespace_kind.kind(), SyntaxKind::WILDCARD_IMPORT);
    assert_eq!(
        namespace_kind
            .child_by_field("alias")
            .expect("Namespace import should expose alias")
            .text(),
        "UI"
    );
}

#[test]
fn test_parse_selective_imports_with_aliases() {
    let source = r#"import { Button, Stack as Layout.Stack } from "https://example.com/ui.zip""#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Selective imports should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let import = root
        .children()
        .find(|child| child.kind() == SyntaxKind::IMPORT_STATEMENT)
        .expect("Expected import statement");

    let kind = import
        .child_by_field("kind")
        .expect("Import should expose kind");
    assert_eq!(kind.kind(), SyntaxKind::SELECTIVE_IMPORT_LIST);

    let selective: Vec<_> = kind
        .children()
        .filter(|child| child.kind() == SyntaxKind::SELECTIVE_IMPORT)
        .collect();
    assert_eq!(selective.len(), 2);
    assert_eq!(
        selective[0]
            .child_by_field("name")
            .expect("Selective import should expose name")
            .text(),
        "Button"
    );
    assert!(
        selective[0].child_by_field("alias").is_none(),
        "Button import should not have alias"
    );
    assert_eq!(
        selective[1]
            .child_by_field("name")
            .expect("Selective import should expose name")
            .text(),
        "Stack"
    );
    assert_eq!(
        selective[1]
            .child_by_field("alias")
            .expect("Aliased selective import should expose alias")
            .text(),
        "Layout.Stack"
    );

    let library_path = import
        .child_by_field("path")
        .expect("Import should expose library path");
    assert_eq!(
        library_path
            .child_by_field("value")
            .expect("library_path should expose value")
            .text(),
        r#""https://example.com/ui.zip""#
    );
}

#[test]
fn test_parse_visibility_modifiers() {
    let source = r#"private let title = "NX"
export component <Button/> = { <button/> }
let subtitle = "Runtime""#;
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Visibility modifiers should parse. Errors: {:?}",
        result.errors
    );

    let root = result.root().expect("Should have root node");
    let children: Vec<_> = root.children().collect();
    assert_eq!(children[0].kind(), SyntaxKind::VALUE_DEFINITION);
    assert_eq!(
        children[0]
            .child_by_field("visibility")
            .expect("private value should expose visibility")
            .text(),
        "private"
    );
    assert_eq!(
        children[1]
            .child_by_field("visibility")
            .expect("export component should expose visibility")
            .text(),
        "export"
    );
    assert!(
        children[2].child_by_field("visibility").is_none(),
        "default-internal declaration should omit visibility field"
    );
    assert_eq!(children[2].kind(), SyntaxKind::VALUE_DEFINITION);
}

#[test]
fn test_parse_import_without_from_is_error() {
    let source = "import ui.components";
    let result = parse_str(source, "test.nx");

    assert!(!result.is_ok(), "Import without from should fail");
    assert!(
        !result.errors.is_empty(),
        "Import without from should produce parse errors"
    );
}

#[test]
fn test_parse_removed_contenttype_is_error() {
    let source = r#"contenttype "./prelude""#;
    let result = parse_str(source, "test.nx");

    assert!(!result.is_ok(), "Removed contenttype should fail");
    assert!(
        !result.errors.is_empty(),
        "Removed contenttype should produce parse errors"
    );
}

#[test]
fn test_parse_invalid_component_emits_is_error() {
    let path = fixture_path("invalid/component-invalid-emits.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Malformed emits syntax should fail");
    assert!(
        !result.errors.is_empty(),
        "Malformed emits syntax should produce parse errors"
    );

    if let Some(root) = result.root() {
        assert!(
            contains_kind(&root, SyntaxKind::COMPONENT_DEFINITION)
                || contains_kind(&root, SyntaxKind::ERROR),
            "Parser should either recover a component node or surface an error node"
        );
    }
}

#[test]
fn test_parse_invalid_component_state_is_error() {
    let path = fixture_path("invalid/component-invalid-state.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Malformed state syntax should fail");
    assert!(
        !result.errors.is_empty(),
        "Malformed state syntax should produce parse errors"
    );

    if let Some(root) = result.root() {
        assert!(
            contains_kind(&root, SyntaxKind::COMPONENT_DEFINITION)
                || contains_kind(&root, SyntaxKind::ERROR),
            "Parser should either recover a component node or surface an error node"
        );
    }
}

#[test]
fn test_parse_bodyless_concrete_component_is_error() {
    let path = fixture_path("invalid/component-concrete-bodyless.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Bodyless concrete component should fail");
    assert!(
        !result.errors.is_empty(),
        "Bodyless concrete component should produce diagnostics"
    );
}

#[test]
fn test_parse_abstract_component_with_body_is_error() {
    let path = fixture_path("invalid/component-abstract-body.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Abstract component with body should fail");
    assert!(
        !result.errors.is_empty(),
        "Abstract component with body should produce diagnostics"
    );
}

#[test]
fn test_parse_external_component_with_body_is_error() {
    let path = fixture_path("invalid/component-external-body.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "External component with body should fail");
    assert!(
        !result.errors.is_empty(),
        "External component with body should produce diagnostics"
    );
}

#[test]
fn test_parse_external_component_with_empty_body_is_error() {
    let path = fixture_path("invalid/component-external-empty-body.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "External component with empty body should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "External component with empty body should produce diagnostics"
    );
}

#[test]
fn test_parse_external_component_with_duplicate_state_groups_is_error() {
    let path = fixture_path("invalid/component-external-duplicate-state.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "External component with duplicate state groups should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "External component with duplicate state groups should produce diagnostics"
    );
}

#[test]
fn test_parse_multiple_component_bases_is_error() {
    let path = fixture_path("invalid/component-inheritance-multiple-bases.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Malformed multiple-base component syntax should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "Malformed multiple-base component syntax should produce parse errors"
    );
}

#[test]
fn test_parse_invalid_action_definition_is_error() {
    let path = fixture_path("invalid/action-invalid-declaration.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Malformed action syntax should fail");
    assert!(
        !result.errors.is_empty(),
        "Malformed action syntax should produce parse errors"
    );

    if let Some(root) = result.root() {
        assert!(
            contains_kind(&root, SyntaxKind::ACTION_DEFINITION)
                || contains_kind(&root, SyntaxKind::ERROR),
            "Parser should either recover an action node or surface an error node"
        );
    }
}

#[test]
fn test_parse_multiple_record_bases_is_error() {
    let path = fixture_path("invalid/record-inheritance-multiple-bases.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Malformed multiple-base record syntax should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "Malformed multiple-base record syntax should produce parse errors"
    );
}

#[test]
fn test_parse_multiple_action_bases_is_error() {
    let path = fixture_path("invalid/action-inheritance-multiple-bases.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Malformed multiple-base action syntax should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "Malformed multiple-base action syntax should produce parse errors"
    );
}

#[test]
fn test_parse_invalid_component_emits_reference_is_error() {
    let path = fixture_path("invalid/component-invalid-emits-reference-qualifier.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Malformed emits reference syntax should fail"
    );
    assert!(
        !result.errors.is_empty(),
        "Malformed emits reference syntax should produce parse errors"
    );

    if let Some(root) = result.root() {
        assert!(
            contains_kind(&root, SyntaxKind::COMPONENT_DEFINITION)
                || contains_kind(&root, SyntaxKind::ERROR),
            "Parser should either recover a component node or surface an error node"
        );
    }
}

#[test]
fn test_parse_constant_union_definition() {
    let path = fixture_path("valid/constant-union-definition.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Constant union file should parse");
    let root = result.root().expect("Should produce root node");

    let unions: Vec<_> = root
        .children()
        .filter(|child| child.kind() == SyntaxKind::UNION_DEFINITION)
        .collect();
    assert_eq!(unions.len(), 2, "Expected two union definitions");

    let status = unions.first().expect("First union definition should exist");
    let name_node = status
        .child_by_field("name")
        .expect("Union definition should expose name field");
    assert_eq!(name_node.text(), "Status");

    let cases_node = status
        .child_by_field("cases")
        .expect("Union definition should contain a case list");
    let case_names: Vec<_> = cases_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::UNION_CASE)
        .map(|case| {
            case.child_by_field("name")
                .expect("Union case should expose name")
                .text()
                .to_string()
        })
        .collect();
    assert_eq!(case_names, vec!["pending_review", "active", "disabled"]);
}

#[test]
fn test_parse_union_definition() {
    let path = fixture_path("valid/union-definition.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Union definition file should parse. Errors: {:?}",
        result.errors
    );
    let root = result.root().expect("Should produce root node");

    let unions: Vec<_> = root
        .children()
        .filter(|child| child.kind() == SyntaxKind::UNION_DEFINITION)
        .collect();
    // `CardSortMode` is a constant union: the fixture's former `enum` declaration.
    assert_eq!(unions.len(), 3, "Expected three union definitions");
    assert_eq!(
        unions[0]
            .child_by_field("name")
            .expect("Union should expose name field")
            .text(),
        "CardSortMode"
    );

    let load_state = unions.get(1).expect("LoadState union should exist");
    assert_eq!(
        load_state
            .child_by_field("name")
            .expect("Union should expose name field")
            .text(),
        "LoadState"
    );

    let cases = load_state
        .child_by_field("cases")
        .expect("Union should expose case list");
    let case_names: Vec<_> = cases
        .children()
        .filter(|child| child.kind() == SyntaxKind::UNION_CASE)
        .map(|case| {
            case.child_by_field("name")
                .expect("Union case should expose name")
                .text()
                .to_string()
        })
        .collect();
    assert_eq!(case_names, vec!["idle", "loading", "failed", "loaded"]);

    let failed = cases
        .children()
        .find(|case| {
            case.child_by_field("name")
                .is_some_and(|name| name.text() == "failed")
        })
        .expect("Expected failed case");
    let failed_fields: Vec<_> = failed
        .children()
        .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .collect();
    assert_eq!(failed_fields.len(), 2, "Expected two failed-case fields");
    assert!(
        failed_fields[1].child_by_field("default").is_some(),
        "Case field default should be preserved"
    );

    let ui_event = unions.get(2).expect("UiEvent union should exist");
    assert_eq!(
        ui_event
            .child_by_field("base")
            .expect("Union should expose inherited abstract base")
            .text(),
        "EventBase"
    );
}

#[test]
fn test_parse_union_definition_leading_pipe_is_optional_for_multiple_cases() {
    let path = fixture_path("valid/union-without-leading-pipe.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "A union of two or more cases needs no leading pipe. Errors: {:?}",
        result.errors
    );
    let root = result.root().expect("Should produce root node");

    let unions: Vec<_> = root
        .children()
        .filter(|child| child.kind() == SyntaxKind::UNION_DEFINITION)
        .collect();
    assert_eq!(unions.len(), 3, "Expected three union definitions");

    let case_names = |union: &nx_syntax::SyntaxNode| -> Vec<String> {
        union
            .child_by_field("cases")
            .expect("Union should expose case list")
            .children()
            .filter(|child| child.kind() == SyntaxKind::UNION_CASE)
            .map(|case| {
                case.child_by_field("name")
                    .expect("Union case should expose name")
                    .text()
                    .to_string()
            })
            .collect()
    };

    assert_eq!(case_names(&unions[0]), vec!["idle", "loading"]);
    assert_eq!(case_names(&unions[1]), vec!["point", "circle"]);
    assert_eq!(case_names(&unions[2]), vec!["solo"]);
}

/// A single bare case would be ambiguous with the alias form, so `type A = B` stays an alias.
#[test]
fn test_parse_single_bare_name_is_a_type_alias_not_a_union() {
    let result = parse_str("type Alias = Other", "alias.nx");

    assert!(
        result.is_ok(),
        "Expected a clean parse. Errors: {:?}",
        result.errors
    );
    let root = result.root().expect("Should produce root node");
    let kinds: Vec<_> = root.children().map(|child| child.kind()).collect();

    assert!(
        kinds.contains(&SyntaxKind::TYPE_DEFINITION),
        "Expected a type_definition, got {kinds:?}"
    );
    assert!(
        !kinds.contains(&SyntaxKind::UNION_DEFINITION),
        "A single bare name must not parse as a union, got {kinds:?}"
    );
}

/// The removed `enum` keyword is reported by name rather than as an unhelpful parse error.
#[test]
fn test_parse_removed_enum_keyword_names_the_replacement() {
    let path = fixture_path("invalid/removed-enum-keyword.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "`enum` is no longer an NX declaration");

    let codes: Vec<_> = result
        .errors
        .iter()
        .filter_map(|diagnostic| diagnostic.code())
        .collect();
    assert_eq!(
        codes,
        vec!["removed-enum-keyword"],
        "the targeted diagnostic replaces the generic parse error rather than joining it"
    );

    let notes = result
        .errors
        .iter()
        .filter_map(|diagnostic| diagnostic.note())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        notes.contains("Write `type Fit = fill | contain | cover` instead"),
        "expected the concrete replacement form, got: {notes}"
    );
}

#[test]
fn test_parse_union_definition_rejects_duplicate_cases() {
    let path = fixture_path("invalid/union-duplicate-cases.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        !result.is_ok(),
        "Duplicate union case should fail validation"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|diagnostic| diagnostic.code() == Some("duplicate-union-case")),
        "Duplicate union case should produce a dedicated diagnostic"
    );
}

#[test]
fn test_parse_union_definition_rejects_malformed_cases() {
    for fixture in [
        "invalid/union-empty-case-list.nx",
        "invalid/union-malformed-case-payload.nx",
        "invalid/union-invalid-extends-clause.nx",
    ] {
        let path = fixture_path(fixture);
        let result = parse_file(&path).unwrap();

        assert!(
            !result.is_ok(),
            "{fixture} should fail parsing or validation"
        );
        assert!(
            !result.errors.is_empty(),
            "{fixture} should produce diagnostics"
        );
    }
}

#[test]
fn test_parse_all_valid_fixtures() {
    let valid_dir = fixture_path("valid");

    for entry in fs::read_dir(&valid_dir).expect("Should read valid fixtures directory") {
        let entry = entry.expect("Should read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("nx") {
            let result = parse_file(&path).expect("Should parse file");

            assert!(
                result.is_ok(),
                "File {:?} should parse without errors, but got:\n{}",
                path.file_name(),
                {
                    let mut sources = HashMap::new();
                    let file_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    let src = std::fs::read_to_string(&path).unwrap_or_default();
                    sources.insert(file_name.clone(), src);
                    render_diagnostics_cli(&result.errors, &sources)
                }
            );
        }
    }
}

// (reserved) typed raw embed tests to be added after parser accepts typed raw embeds

// ============================================================================
// Syntax Error Tests (T051)
// ============================================================================

#[test]
fn test_parse_incomplete_expression() {
    let path = fixture_path("invalid/incomplete-expression.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect incomplete expression");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_unclosed_brace() {
    let path = fixture_path("invalid/unclosed-brace.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect unclosed brace");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_mismatched_tags() {
    let path = fixture_path("invalid/mismatched-tags.nx");
    let result = parse_file(&path).unwrap();

    // May have parse errors or validation errors depending on grammar
    assert!(
        !result.is_ok() || !result.errors.is_empty(),
        "Should detect tag mismatch"
    );
}

#[test]
fn test_parse_missing_parenthesis() {
    let path = fixture_path("invalid/missing-parenthesis.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect missing parenthesis");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_invalid_element() {
    let path = fixture_path("invalid/invalid-element.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect invalid element syntax");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_multiple_errors() {
    let path = fixture_path("invalid/multiple-errors.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect multiple errors");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_all_invalid_fixtures() {
    let invalid_dir = fixture_path("invalid");

    for entry in fs::read_dir(&invalid_dir).expect("Should read invalid fixtures directory") {
        let entry = entry.expect("Should read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("nx") {
            let result = parse_file(&path).expect("Should parse file");

            assert!(
                !result.is_ok() || !result.errors.is_empty(),
                "File {:?} should have errors",
                path.file_name()
            );
        }
    }
}

// ============================================================================
// Error Recovery Tests (T055)
// ============================================================================

#[test]
fn test_error_recovery_within_scope() {
    let source = r#"
        let x = {;
        let y = };
        let z = 42
    "#;

    let result = parse_str(source, "test.nx");

    // Should collect all errors within the scope
    assert!(!result.errors.is_empty(), "Should detect errors");

    // Should still produce a tree (best-effort recovery)
    assert!(result.tree.is_some(), "Should produce tree with errors");
}

#[test]
fn test_error_recovery_continues_parsing() {
    let source = r#"
        let valid1 = 42
        let invalid =
        let valid2 = 99
    "#;

    let result = parse_str(source, "test.nx");

    // Should detect the error but continue parsing
    assert!(
        !result.errors.is_empty(),
        "Should detect error in invalid statement"
    );
    assert!(result.tree.is_some(), "Should continue parsing after error");
}

// ============================================================================
// UTF-8 Validation Tests (T053)
// ============================================================================

#[test]
fn test_utf8_valid_unicode() {
    let source = "let emoji = \"😀🎉\"";
    let result = parse_str(source, "test.nx");

    assert!(result.is_ok(), "Should handle valid UTF-8 unicode");
}

#[test]
fn test_utf8_valid_chinese() {
    let source = "let greeting = \"你好世界\"";
    let result = parse_str(source, "test.nx");

    assert!(result.is_ok(), "Should handle Chinese characters");
}

#[test]
fn test_utf8_valid_arabic() {
    let source = "let text = \"مرحبا\"";
    let result = parse_str(source, "test.nx");

    assert!(result.is_ok(), "Should handle Arabic characters");
}

#[test]
fn test_utf8_valid_mixed() {
    let source = r#"
        let mixed = "Hello 世界 مرحبا 😀"
        let name = "José García"
    "#;
    let result = parse_str(source, "test.nx");

    assert!(result.is_ok(), "Should handle mixed UTF-8 characters");
}

// ============================================================================
// Concurrent Parsing Tests (T054)
// ============================================================================

#[test]
fn test_concurrent_parsing_different_files() {
    let sources = vec![
        ("let x = 42", "test1.nx"),
        ("let <Foo /> = <div />", "test2.nx"),
        ("let <Button /> = <button />", "test3.nx"),
    ];

    let handles: Vec<_> = sources
        .into_iter()
        .map(|(source, name)| {
            thread::spawn(move || {
                let result = parse_str(source, name);
                assert!(result.is_ok(), "Concurrent parsing should succeed");
                result
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
}

#[test]
fn test_concurrent_parsing_same_source() {
    let source = Arc::new(String::from("let x = 42"));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let src = Arc::clone(&source);
            thread::spawn(move || {
                let result = parse_str(&src, &format!("test{}.nx", i));
                assert!(
                    result.is_ok(),
                    "Concurrent parsing of same source should succeed"
                );
                result
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
}

#[test]
fn test_concurrent_parsing_stress() {
    let source = r#"
        let <Card
            title:string
            content:string
        /> =
            <div class="card">
                <h2>{title}</h2>
                <p>{content}</p>
            </div>
    "#;

    let handles: Vec<_> = (0..100)
        .map(|i| {
            let src = source.to_string();
            thread::spawn(move || {
                let result = parse_str(&src, &format!("card{}.nx", i));
                assert!(result.is_ok(), "Stress test parsing should succeed");
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Stress test thread should complete");
    }
}

// ============================================================================
// Snapshot Tests (T052)
// ============================================================================

#[test]
fn test_snapshot_simple_element() {
    let result = parse_str("let <Button /> = <button />", "test.nx");
    let root = result.root().expect("Should have root");

    // Snapshot the CST structure
    let debug_repr = format!("{:#?}", root.kind());
    insta::assert_snapshot!(debug_repr);
}

#[test]
fn test_snapshot_function_definition() {
    let result = parse_str("fn greet(name: string) { name }", "test.nx");
    let root = result.root().expect("Should have root");

    let debug_repr = format!("{:#?}", root.kind());
    insta::assert_snapshot!(debug_repr);
}

#[test]
fn test_snapshot_error_diagnostics() {
    let result = parse_str("let x = ", "test.nx");

    // Snapshot the error messages
    let errors: Vec<_> = result
        .errors
        .iter()
        .map(|d| d.message().to_string())
        .collect();

    insta::assert_debug_snapshot!(errors);
}

#[test]
fn test_snapshot_union_definition() {
    let result = parse_str(
        "type LoadState = idle | failed { message:string retryable:boolean = true }",
        "test.nx",
    );
    let root = result.root().expect("Should have root");
    let union = root
        .children()
        .find(|child| child.kind() == SyntaxKind::UNION_DEFINITION)
        .expect("Should find union definition");
    let case_list = union
        .child_by_field("cases")
        .expect("Union should expose case list");
    let case_summaries: Vec<_> = case_list
        .children()
        .map(|case| {
            let name = case
                .child_by_field("name")
                .expect("Union case should expose name")
                .text()
                .to_string();
            let field_count = case
                .children()
                .filter(|child| child.kind() == SyntaxKind::PROPERTY_DEFINITION)
                .count();
            (name, field_count)
        })
        .collect();

    insta::assert_debug_snapshot!(case_summaries);
}

// ============================================================================
// Performance Tests (T056)
// ============================================================================

#[test]
fn test_performance_large_file() {
    // Generate a file with ~1000 lines
    let mut large_source = String::new();
    for i in 0..1000 {
        large_source.push_str(&format!("let var{} = {}\n", i, i));
    }

    let start = std::time::Instant::now();
    let result = parse_str(&large_source, "large.nx");
    let duration = start.elapsed();

    assert!(result.tree.is_some(), "Should parse large file");

    // Should parse ~1000 lines in reasonable time
    // Target: >10,000 lines/second means ~100ms for 1000 lines
    assert!(
        duration.as_millis() < 200,
        "Should parse 1000 lines in <200ms, took {:?}",
        duration
    );
}

#[test]
fn test_performance_many_small_parses() {
    let source = "let x = 42";

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok());
    }
    let duration = start.elapsed();

    // Should be fast for repeated small parses
    assert!(
        duration.as_millis() < 1000,
        "Should parse 1000 times in <1s, took {:?}",
        duration
    );
}

// ============================================================================
// Comprehensive Expression Tests (T050)
// ============================================================================

#[test]
fn test_all_expression_types() {
    let path = fixture_path("valid/all-expressions.nx");
    let result = parse_file(&path).unwrap();

    assert!(
        result.is_ok(),
        "Should parse all expression types without errors. Errors:\n{}",
        {
            let mut sources = HashMap::new();
            sources.insert(
                "test.nx".to_string(),
                std::fs::read_to_string(path).unwrap_or_default(),
            );
            render_diagnostics_cli(&result.errors, &sources)
        }
    );
    assert!(result.tree.is_some());
}

#[test]
fn test_literal_expressions() {
    // Integer literal
    let result = parse_str("let test = 42", "test.nx");
    assert!(result.is_ok());

    // Real literal
    let result = parse_str("let test = 3.14", "test.nx");
    assert!(result.is_ok());

    // Hex literal
    let result = parse_str("let test = 0xFF", "test.nx");
    assert!(result.is_ok());

    // Boolean literals
    let result = parse_str("let test = true", "test.nx");
    assert!(result.is_ok());
    let result = parse_str("let test = false", "test.nx");
    assert!(result.is_ok());

    // `null` is not a literal; it parses as a contextual name and is rejected by name resolution.
    let result = parse_str("let test = null", "test.nx");
    assert!(result.is_ok());
    assert!(
        contains_kind(&result.root().unwrap(), SyntaxKind::CONTEXTUAL_NAME),
        "`null` should be an ordinary name"
    );

    // String literal
    let result = parse_str("let test = \"hello\"", "test.nx");
    assert!(result.is_ok());

    // Unit literal in interpolation
    let result = parse_str("let test = {()}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_binary_expressions_arithmetic() {
    // Multiplication
    let result = parse_str("let <Test x: int y: int /> = {x * y}", "test.nx");
    assert!(result.is_ok());

    // Division
    let result = parse_str("let <Test x: int y: int /> = {x / y}", "test.nx");
    assert!(result.is_ok());

    // Remainder
    let result = parse_str("let <Test x: int y: int /> = {x % y}", "test.nx");
    assert!(result.is_ok());

    // Addition
    let result = parse_str("let <Test x: int y: int /> = {x + y}", "test.nx");
    assert!(result.is_ok());

    // Subtraction
    let result = parse_str("let <Test x: int y: int /> = {x - y}", "test.nx");
    assert!(result.is_ok());

    // Complex: precedence (multiplication before addition)
    let result = parse_str("let <Test x: int y: int z: int /> = {x + y * z}", "test.nx");
    assert!(result.is_ok());
}

/// The range operators: one binary expression per operator, between the additive and relational
/// levels, with the tokens lexing as written.
mod range_operators {
    use super::*;
    use nx_syntax::SyntaxNode;

    /// The one binary expression a source's value holds, with its operator and operands.
    fn binary<'tree>(root: &SyntaxNode<'tree>) -> SyntaxNode<'tree> {
        find_first_kind(root, SyntaxKind::BINARY_EXPRESSION).expect("a binary expression")
    }

    fn operator<'tree>(node: &SyntaxNode<'tree>) -> &'tree str {
        node.child_by_field("operator")
            .expect("an operator field")
            .text()
    }

    /// An operand, past the `value_expression` wrapper the grammar puts around one.
    fn operand<'tree>(node: &SyntaxNode<'tree>, field: &str) -> SyntaxNode<'tree> {
        let operand = node
            .child_by_field(field)
            .unwrap_or_else(|| panic!("a {field} field"));
        if operand.kind() == SyntaxKind::VALUE_EXPRESSION {
            operand.children().next().unwrap_or(operand)
        } else {
            operand
        }
    }

    /// A binary expression is not a `let`'s right-hand side on its own, so every case is braced,
    /// as `{x + y}` is.
    fn parse_value(expression: &str) -> nx_syntax::ParseResult {
        let source = format!("let r = {{{expression}}}\n");
        let result = parse_str(&source, "test.nx");
        assert!(
            result.is_ok(),
            "expected {expression:?} to parse: {:?}",
            result.errors
        );
        result
    }

    #[test]
    fn a_half_open_range_holds_two_integer_literals() {
        let result = parse_value("1..5");
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
        assert_eq!(operand(&range, "left").text(), "1");
        assert_eq!(operand(&range, "right").text(), "5");
        assert_eq!(
            count_kind(&range, SyntaxKind::INT_LITERAL),
            2,
            "an integer literal before `..` stays an integer literal"
        );
        assert_eq!(count_kind(&range, SyntaxKind::REAL_LITERAL), 0);
    }

    #[test]
    fn the_inclusive_operator_is_one_token() {
        let result = parse_value("1..=5");
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..=");
        assert_eq!(count_kind(&range, SyntaxKind::INT_LITERAL), 2);
    }

    #[test]
    fn arithmetic_binds_tighter_than_a_range() {
        let source = "let n = 4\nlet r = {0..n + 1}\n";
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{:?}", result.errors);
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
        assert_eq!(operand(&range, "left").text(), "0");
        let right = operand(&range, "right");
        assert_eq!(right.kind(), SyntaxKind::BINARY_EXPRESSION);
        assert_eq!(operator(&right), "+", "`0..n + 1` is `0..(n + 1)`");
    }

    /// The other half of the precedence claim both grammar documents make: a range binds tighter
    /// than a comparison, so `a..b < c` compares the range rather than ranging over `a..(b < c)`.
    #[test]
    fn a_range_binds_tighter_than_a_comparison() {
        let result = parse_value("0..5 < 9");
        let root = result.root().expect("root");
        let comparison = binary(&root);

        assert_eq!(operator(&comparison), "<");
        let left = operand(&comparison, "left");
        assert_eq!(left.kind(), SyntaxKind::BINARY_EXPRESSION);
        assert_eq!(operator(&left), "..", "`0..5 < 9` is `(0..5) < 9`");
        assert_eq!(operand(&comparison, "right").text(), "9");
    }

    /// Equality is looser still, so a comparison of two ranges reads left to right.
    #[test]
    fn a_range_binds_tighter_than_equality() {
        let result = parse_value("1..5 == 1..=4");
        let root = result.root().expect("root");
        let equality = binary(&root);

        assert_eq!(operator(&equality), "==");
        assert_eq!(operator(&operand(&equality, "left")), "..");
        assert_eq!(operator(&operand(&equality, "right")), "..=");
    }

    #[test]
    fn a_prefix_minus_is_an_operand() {
        let result = parse_value("-5..-1");
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
        assert_eq!(operand(&range, "left").text(), "-5");
        assert_eq!(operand(&range, "right").text(), "-1");
    }

    #[test]
    fn a_member_access_binds_before_the_operator() {
        let source = "type Page = { first:int last:int }\n\
                      let p = <Page first={1} last={5} />\n\
                      let r = {p.first..=p.last}\n";
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{:?}", result.errors);
        let root = result.root().expect("root");
        let range = find_first_kind(&root, SyntaxKind::BINARY_EXPRESSION).expect("a range");

        assert_eq!(operator(&range), "..=");
        assert_eq!(operand(&range, "left").text(), "p.first");
        assert_eq!(operand(&range, "right").text(), "p.last");
    }

    #[test]
    fn real_literals_are_operands() {
        let result = parse_value("1.5..2.5");
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
        assert_eq!(count_kind(&range, SyntaxKind::REAL_LITERAL), 2);
    }

    /// Left associativity means `1..5..9` parses, and the checker gives the better message.
    #[test]
    fn a_range_of_a_range_parses_left_associatively() {
        let result = parse_value("1..5..9");
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
        let left = operand(&range, "left");
        assert_eq!(left.kind(), SyntaxKind::BINARY_EXPRESSION);
        assert_eq!(operator(&left), "..");
        assert_eq!(operand(&range, "right").text(), "9");
    }

    #[test]
    fn a_range_is_a_for_iterable_in_both_forms() {
        let source = "let values = { for i in 0..4 { i } }\n\
                      let <Stars count:int /> = <div>for i in 1..=count { <Star /> }</div>\n";
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{:?}", result.errors);
        let root = result.root().expect("root");

        let value_for =
            find_first_kind(&root, SyntaxKind::VALUE_FOR_EXPRESSION).expect("a value for");
        let value_range = binary(&value_for);
        assert_eq!(operator(&value_range), "..");

        let elements_for =
            find_first_kind(&root, SyntaxKind::ELEMENTS_FOR_EXPRESSION).expect("an elements for");
        let elements_range = binary(&elements_for);
        assert_eq!(operator(&elements_range), "..=");
    }

    #[test]
    fn a_range_is_a_property_value() {
        let source = "let s = <Slider range={0..1} />\n";
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{:?}", result.errors);
        let root = result.root().expect("root");
        let range = binary(&root);

        assert_eq!(operator(&range), "..");
    }

    /// A range is a binary expression, so a braced value list takes it only in parentheses.
    #[test]
    fn a_range_in_a_braced_list_is_parenthesized() {
        let result = parse_str("let rs = { (0..5) (5..=9) }\n", "test.nx");
        assert!(result.is_ok(), "{:?}", result.errors);
        let root = result.root().expect("root");
        let mut ranges = Vec::new();
        collect_kinds(&root, SyntaxKind::BINARY_EXPRESSION, &mut ranges);
        assert_eq!(ranges.len(), 2);
        assert_eq!(operator(&ranges[0]), "..");
        assert_eq!(operator(&ranges[1]), "..=");
    }

    /// An unbraced initializer is a literal, never an expression, so a range is rejected there for
    /// the same reason a sum is. Pinned because the shorthand `let r = 1..5` reads so naturally
    /// that it keeps being written, in specs and in examples alike.
    #[test]
    fn an_unbraced_range_is_rejected_like_any_other_expression() {
        let range = parse_str("let r = 1..5\n", "test.nx");
        let sum = parse_str("let n = 1 + 2\n", "test.nx");

        assert!(!range.is_ok(), "an unbraced range should not parse");
        assert!(
            !range.errors.is_empty(),
            "the rejection should carry a diagnostic"
        );
        assert!(
            !sum.is_ok(),
            "an unbraced sum should not parse either, which is the rule the range follows"
        );
    }
}

#[test]
fn test_binary_expressions_comparison() {
    let result = parse_str("let <Test x: int y: int /> = {x < y}", "test.nx");
    assert!(result.is_ok());

    let result = parse_str("let <Test x: int y: int /> = {x > y}", "test.nx");
    assert!(result.is_ok());

    let result = parse_str("let <Test x: int y: int /> = {x <= y}", "test.nx");
    assert!(result.is_ok());

    let result = parse_str("let <Test x: int y: int /> = {x >= y}", "test.nx");
    assert!(result.is_ok());

    let result = parse_str("let <Test x: int y: int /> = {x == y}", "test.nx");
    assert!(result.is_ok());

    let result = parse_str("let <Test x: int y: int /> = {x != y}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_binary_expressions_logical() {
    // Logical AND
    let result = parse_str("let <Test x: boolean y: boolean /> = {x && y}", "test.nx");
    assert!(result.is_ok());

    // Logical OR
    let result = parse_str("let <Test x: boolean y: boolean /> = {x || y}", "test.nx");
    assert!(result.is_ok());

    // Complex: precedence (AND before OR)
    let result = parse_str(
        "let <Test x: boolean y: boolean z: boolean /> = {x && y || z}",
        "test.nx",
    );
    assert!(result.is_ok());
}

#[test]
fn test_unary_expressions() {
    // Prefix negation
    let result = parse_str("let <Test x: int /> = {-x}", "test.nx");
    assert!(result.is_ok());

    // Double negation
    let result = parse_str("let <Test x: int /> = {--x}", "test.nx");
    assert!(result.is_ok());

    // Logical not
    let result = parse_str("let <Test flag: boolean /> = {!flag}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_conditional_operator_is_rejected_with_the_if_form() {
    for (source, replacement) in [
        // An unbraced `let` value needs the `if` in braces.
        (
            "let ratio = ready ? 1 : 2",
            "`{ if ready { 1 } else { 2 } }`",
        ),
        (
            "let <Test x: int /> = {x > 0 ? x * 2 : -1}",
            "if x > 0 { x * 2 } else { -1 }",
        ),
        (
            "let c = {\"a?\" + (b ? \"x\" : \"y\")}",
            "if b { \"x\" } else { \"y\" }",
        ),
        // The alternative stops at the next call argument.
        ("let r = { g(c ? 1 : 2, 3) }", "`if c { 1 } else { 2 }`"),
        // A trailing comment is not part of the alternative.
        ("let r = { c ? 1 : 2 // pick\n}", "`if c { 1 } else { 2 }`"),
        (
            "let r = { c ? 1 : 2 /* pick */ }",
            "`if c { 1 } else { 2 }`",
        ),
        // A colon inside a string literal does not end the consequent early.
        (
            "let r = { c ? \"a:b\" : \"c\" }",
            "`if c { \"a:b\" } else { \"c\" }`",
        ),
        // A later item in the sequence follows the conditional rather than joining the `else`.
        (
            "let r = { c ? \"a\" : \"b\" \"z\" }",
            "`if c { \"a\" } else { \"b\" }`",
        ),
        (
            "let r = { c ? 1 : 2 undefinedAfter }",
            "`if c { 1 } else { 2 }`",
        ),
        // One operand per line.
        (
            "let r = {\n  c\n    ? 1\n    : 2\n}",
            "`if c { 1 } else { 2 }`",
        ),
    ] {
        let result = parse_str(source, "test.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
        assert_eq!(
            codes,
            vec!["removed-conditional-operator"],
            "{source}: {:?}",
            result.errors
        );
        let note = result.errors[0].note().unwrap_or_default();
        assert!(note.contains(replacement), "{source}: {note}");
    }
}

/// The `?` of a parameter's optional mark is not a ternary's `?`, so an unrelated error after it
/// is reported as itself rather than hidden behind a conditional-operator rewrite.
#[test]
fn test_optional_mark_is_not_taken_for_a_conditional_operator() {
    for source in [
        "let f(a?:int): int = { a ) : 3 }",
        "component <C a?:int /> = { <div x={a ) : 3} /> }",
    ] {
        let result = parse_str(source, "test.nx");
        assert!(result.has_errors(), "{source}");
        assert!(
            result
                .errors
                .iter()
                .all(|d| d.code() != Some("removed-conditional-operator")),
            "{source}: {:?}",
            result.errors
        );
    }
}

/// A rewrite that would not parse — an alternative cut at a line break, or an operand that was
/// already malformed — is not suggested, so the note names the `if` form without filling it in.
#[test]
fn test_conditional_operator_rewrite_that_does_not_parse_gets_no_filled_in_fix_it() {
    for source in [
        "let r = {\n  c ? 1 : n +\n    2\n}",
        "let f(a?:int): boolean = { a? ) : 3 }",
        "let f(a?:int): boolean = { a? + ) : 3 }",
    ] {
        let result = parse_str(source, "test.nx");
        let diagnostic = result
            .errors
            .iter()
            .find(|d| d.code() == Some("removed-conditional-operator"))
            .unwrap_or_else(|| panic!("{source}: {:?}", result.errors));
        let note = diagnostic.note().unwrap_or_default();
        assert!(
            note.contains("`if condition { a } else { b }`"),
            "{source}: {note}"
        );
    }
}

/// A nested conditional cannot be split at one `?` and one `:` into the author's operands, so the
/// note names the `if` form without filling it in rather than suggesting a wrong rewrite.
#[test]
fn test_nested_conditional_operator_gets_no_filled_in_fix_it() {
    // A parenthesized inner conditional (`c ? (d ? 1 : 2) : 3`) is its own error region and gets
    // its own, correct fix-it, so only the unparenthesized chain is pinned here.
    for source in ["let r = { c ? 1 : d ? 2 : 3 }", "let r = c ? 1 : d ? 2 : 3"] {
        let result = parse_str(source, "test.nx");
        let conditional: Vec<_> = result
            .errors
            .iter()
            .filter(|d| d.code() == Some("removed-conditional-operator"))
            .collect();
        assert!(!conditional.is_empty(), "{source}: {:?}", result.errors);
        for diagnostic in conditional {
            let note = diagnostic.note().unwrap_or_default();
            assert!(
                note.contains("`if condition { a } else { b }`"),
                "{source}: {note}"
            );
        }
    }
}

#[test]
fn test_presence_operators_parse() {
    let source = r#"
        let has(b:Book): boolean = { b.author? }
        let name(b:Book): string? = { b.author?.name }
        let byline(b:Book): string = { "Author: " + b.author?.name ?? "Anonymous" }
        let pick(c:int, a?:int, b?:int): int = { a ?? b ?? c }
        let f(a?:int, b?:int): boolean = { !a? || (a? && b?) }
        let g(s?:State): string = { if s is { {} => "none" idle => "idle" else => "busy" } }
    "#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "{:?}", result.errors);
    let root = result.root().unwrap();
    assert_eq!(count_kind(&root, SyntaxKind::EXISTS_EXPRESSION), 4);
    assert_eq!(count_kind(&root, SyntaxKind::OPTIONAL_MEMBER_EXPRESSION), 2);
    let mut binaries = Vec::new();
    collect_kinds(&root, SyntaxKind::BINARY_EXPRESSION, &mut binaries);
    let coalesces = binaries
        .iter()
        .filter(|binary| {
            binary
                .child_by_field("operator")
                .is_some_and(|operator| operator.text() == "??")
        })
        .count();
    assert_eq!(coalesces, 3);

    // `??` associates to the right: `a ?? (b ?? c)`.
    let pick = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .nth(3)
        .unwrap();
    let outer = find_first_kind(&pick, SyntaxKind::BINARY_EXPRESSION).unwrap();
    let right = outer.child_by_field("right").unwrap();
    assert!(
        contains_kind(&right, SyntaxKind::BINARY_EXPRESSION),
        "{}",
        outer.text()
    );

    // `??` binds above `+`: `"Author: " + (b.author?.name ?? "Anonymous")`.
    let byline = root
        .children()
        .filter(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION)
        .nth(2)
        .unwrap();
    let outer = find_first_kind(&byline, SyntaxKind::BINARY_EXPRESSION).unwrap();
    assert_eq!(
        outer.child_by_field("operator").unwrap().text(),
        "+",
        "{}",
        outer.text()
    );
}

/// The value of the body brace of `let r = { ... }`, below its `value_expression` wrapper.
fn body_expression(source: &str) -> String {
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "{source}: {:?}", result.errors);
    let root = result.root().unwrap();
    let braced = find_first_kind(&root, SyntaxKind::VALUES_BRACED_EXPRESSION).unwrap();
    let shape = shape_of(&braced.children().next().unwrap());
    shape
}

/// A bracketed outline of an expression's operator structure: `(?? a (* b c))`.
fn shape_of(node: &nx_syntax::SyntaxNode) -> String {
    match node.kind() {
        SyntaxKind::VALUE_EXPRESSION | SyntaxKind::VALUE_LIST_ITEM_EXPRESSION => {
            shape_of(&node.children().next().unwrap())
        }
        SyntaxKind::BINARY_EXPRESSION => format!(
            "({} {} {})",
            node.child_by_field("operator").unwrap().text(),
            shape_of(&node.child_by_field("left").unwrap()),
            shape_of(&node.child_by_field("right").unwrap())
        ),
        SyntaxKind::PREFIX_UNARY_EXPRESSION => format!(
            "({} {})",
            node.child_by_field("operator").unwrap().text(),
            shape_of(&node.child_by_field("operand").unwrap())
        ),
        SyntaxKind::EXISTS_EXPRESSION => {
            format!("(? {})", shape_of(&node.child_by_field("operand").unwrap()))
        }
        SyntaxKind::MEMBER_ACCESS_EXPRESSION => format!(
            "(. {} {})",
            shape_of(&node.child_by_field("target").unwrap()),
            node.child_by_field("member").unwrap().text()
        ),
        SyntaxKind::OPTIONAL_MEMBER_EXPRESSION => format!(
            "(?. {} {})",
            shape_of(&node.child_by_field("target").unwrap()),
            node.child_by_field("member").unwrap().text()
        ),
        _ => node.text().to_string(),
    }
}

/// Pins how the presence operators group against their neighbours: `??` above the binary
/// arithmetic operators and below the prefix ones, postfix `?` above everything else.
#[test]
fn test_presence_operator_precedence_shapes() {
    for (source, expected) in [
        ("let r = { a ?? b * c }", "(* (?? a b) c)"),
        ("let r = { a * b ?? c }", "(* a (?? b c))"),
        ("let r = { a + b ?? c }", "(+ a (?? b c))"),
        ("let r = { a ?? b == c }", "(== (?? a b) c)"),
        ("let r = { !a? }", "(! (? a))"),
        ("let r = { a? && b? }", "(&& (? a) (? b))"),
        ("let r = { -x ?? 1 }", "(?? (- x) 1)"),
        ("let r = { !x ?? y }", "(?? (! x) y)"),
        ("let r = { a?.b?.c }", "(?. (?. a b) c)"),
        ("let r = { a.b? }", "(? (. a b))"),
        // A space splits `?.`: `x? .m` is a member access on the presence test, which the
        // checker then rejects, rather than an optional member access.
        ("let r = { x? .m }", "(. (? x) m)"),
    ] {
        assert_eq!(body_expression(source), expected, "{source}");
    }
}

#[test]
fn test_parenthesized_expressions() {
    // Simple parentheses
    let result = parse_str("let <Test x: int y: int /> = {(x + y) * 2}", "test.nx");
    assert!(result.is_ok());

    // Nested parentheses
    let result = parse_str("let <Test x: int /> = {((x + 1) * 2)}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_member_access_expressions() {
    // Simple member access
    let result = parse_str("let <Test obj: object /> = {obj.field}", "test.nx");
    assert!(result.is_ok());

    // Chained member access
    let result = parse_str("let <Test obj: object /> = {obj.first.second}", "test.nx");
    assert!(result.is_ok());

    // Member access on method result
    let result = parse_str("let <Test obj: object /> = {obj.field.method}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_call_expressions() {
    // No arguments
    let result = parse_str("let <Test func: object /> = {func()}", "test.nx");
    assert!(result.is_ok());

    // One argument
    let result = parse_str("let <Test func: object x: int /> = {func(x)}", "test.nx");
    assert!(result.is_ok());

    // Multiple arguments
    let result = parse_str(
        "let <Test func: object x: int y: int /> = {func(x, y)}",
        "test.nx",
    );
    assert!(result.is_ok());

    // Chained calls
    let result = parse_str("let <Test func: object /> = {func()()}", "test.nx");
    assert!(result.is_ok());

    // Method call
    let result = parse_str("let <Test obj: object /> = {obj.method(42)}", "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_if_expressions_simple() {
    // If-else
    let result = parse_str(
        "let <Test x: int /> = {if x > 0 { 1 } else { -1 }}",
        "test.nx",
    );
    assert!(result.is_ok());

    // If without else
    let result = parse_str("let <Test x: int /> = {if x > 0 { x }}", "test.nx");
    assert!(result.is_ok());

    // Nested if
    let result = parse_str(
        "let <Test x: int /> = {if x > 0 { if x > 10 { 2 } else { 1 } } else { 0 }}",
        "test.nx",
    );
    assert!(result.is_ok());
}

#[test]
fn test_if_expressions_condition_list() {
    let source = r#"let <Test x: int /> = {if {
  x > 100 => 3
  x > 10 => 2
  x > 0 => 1
  else => 0
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(
        result.is_ok(),
        "Condition list if expression should parse. Errors: {:?}",
        result.errors
    );
}

#[test]
fn test_if_expressions_match() {
    // With scrutinee
    let source = r#"let <Test x: int /> = {if x is {
  0 => "zero"
  1 => "one"
  else => "other"
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(
        result.is_ok(),
        "Match if expression should parse. Errors: {:?}",
        result.errors
    );

    // Without scrutinee should now be an error
    let source = r#"let <Test /> = {if is {
  true => "yes"
  false => "no"
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(
        !result.is_ok(),
        "Match if expression without scrutinee should now fail to parse"
    );
}

#[test]
fn test_for_expressions() {
    // Simple for
    let result = parse_str(
        "let <Test items: object /> = {for item in items { item * 2 }}",
        "test.nx",
    );
    assert!(result.is_ok());

    // For with index
    let result = parse_str(
        "let <Test items: object /> = {for item, index in items { item + index }}",
        "test.nx",
    );
    assert!(result.is_ok());

    // Nested for
    let result = parse_str(
        "let <Test matrix: object /> = {for row in matrix { for cell in row { cell } }}",
        "test.nx",
    );
    assert!(result.is_ok());
}

#[test]
fn test_complex_expression_combinations() {
    // For with if inside
    let source = r#"let <Test x: int items: object /> = {
  for item in items {
    if item > 0 {
      item + x
    } else {
      -item
    }
  }
}"#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok());

    // Mixed operators with precedence
    let result = parse_str(
        "let <Test x: int y: int /> = {if x + y * 2 > 10 && x < 100 { x * y } else { x + y }}",
        "test.nx",
    );
    assert!(result.is_ok());

    // Chained method calls under an if
    let result = parse_str(
        "let <Test obj: object x: int /> = {if obj.method(x + 1, x * 2).result > 0 { \"pos\" } else { \"neg\" }}",
        "test.nx"
    );
    assert!(result.is_ok());
}

#[test]
fn test_property_defaults_with_expressions() {
    let source = r#"let <Test
  sum: int = {1 + 2 + 3}
  product: int = {4 * 5}
  comparison: boolean = {10 > 5}
  logical: boolean = {true && false}
  chosen: int = {if 5 > 3 { 100 } else { 200 }}
  nested: int = {(1 + 2) * (3 + 4)}
/> = {sum + product}"#;
    let result = parse_str(source, "test.nx");
    assert!(
        result.is_ok(),
        "Property defaults with expressions should parse. Errors: {:?}",
        result.errors
    );
}

#[test]
fn test_expression_operator_precedence() {
    // Verify operator precedence is correct
    let source = "let test = {1 + 2 * 3}"; // Should parse as 1 + (2 * 3)
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok());

    let source = "let test = {1 * 2 + 3}"; // Should parse as (1 * 2) + 3
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok());

    let source = "let test = {true && false || true}"; // Should parse as (true && false) || true
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok());
}

#[test]
fn test_value_definitions() {
    // Simple value definition without type
    let result = parse_str("let x = 42", "test.nx");
    assert!(
        result.is_ok(),
        "Simple value definition should parse. Errors: {:?}",
        result.errors
    );

    // Value definition with type annotation
    let result = parse_str("let x: int = 42", "test.nx");
    assert!(result.is_ok(), "Value definition with type should parse");

    // Value definition with expression
    let result = parse_str("let sum = {1 + 2 + 3}", "test.nx");
    assert!(
        result.is_ok(),
        "Value definition with expression should parse"
    );

    // Value definition with type and expression
    let result = parse_str("let sum: int = {1 + 2 + 3}", "test.nx");
    assert!(
        result.is_ok(),
        "Value definition with type and expression should parse"
    );

    // Multiple value definitions
    let source = r#"let x = 42
let y = 10
let sum = {x + y}"#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "Multiple value definitions should parse");
}

#[test]
fn test_value_definition_vs_function_definition() {
    // Value definition (no parameters)
    let result = parse_str("let x = 42", "test.nx");
    assert!(result.is_ok());
    let root = result.root().unwrap();
    // Should find a value_definition child
    let has_value_def = root
        .children()
        .any(|c| c.kind() == SyntaxKind::VALUE_DEFINITION);
    assert!(has_value_def, "Should have value_definition node");

    // Function definition (with parameters)
    let result = parse_str("let <Add x: int y: int /> = {x + y}", "test.nx");
    assert!(result.is_ok());
    let root = result.root().unwrap();
    // Should find a function_definition child
    let has_func_def = root
        .children()
        .any(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION);
    assert!(has_func_def, "Should have function_definition node");
}

#[test]
fn test_parse_element_with_empty_body() {
    // An element whose open and close tags have nothing between them is well-formed, and carries
    // no content field — the same shape a self-closing tag produces.
    let source = "<App></App>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Element with an empty body should parse: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );

    let element = find_first_kind(
        &result.root().expect("Should have root node"),
        SyntaxKind::ELEMENT,
    )
    .expect("Expected element");
    assert!(
        element.child_by_field("content").is_none(),
        "An empty body should expose no content field"
    );
    assert_eq!(
        element
            .child_by_field("close_name")
            .expect("Element should expose closing tag name")
            .text(),
        "App"
    );
}

#[test]
fn test_parse_element_with_empty_body_across_lines() {
    // The whitespace-and-comment-only body is the shape authors actually write.
    let source = "let root() = {\n  <App VerticalOptions=Fill>\n    // nothing yet\n  </App>\n}";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "Element with a whitespace-only body should parse: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );

    let element = find_first_kind(
        &result.root().expect("Should have root node"),
        SyntaxKind::ELEMENT,
    )
    .expect("Expected element");
    assert!(
        element.child_by_field("content").is_none(),
        "A whitespace-only body should expose no content field"
    );
    assert!(
        element.child_by_field("properties").is_some(),
        "Properties should still be parsed alongside an empty body"
    );
}

#[test]
fn test_parse_top_level_element_with_empty_body() {
    // A source file may end in a single element expression, and that element may be empty.
    let source = "external component <App content Children?:Element+ />\n\n<App>\n</App>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "A top-level element with an empty body should parse: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );
    assert!(
        contains_kind(
            &result.root().expect("Should have root node"),
            SyntaxKind::ELEMENT
        ),
        "Expected a top-level element"
    );
}

// ============================================================================
// External scanner termination
// ============================================================================

/// Parses on a worker thread, so a scanner that fails to terminate fails this test rather than
/// hanging the whole suite. Returns whether the parse was clean, or `None` if it never finished.
fn parse_within(source: &'static str, limit: Duration) -> Option<bool> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = parse_str(source, "test.nx");
        let _ = sender.send(result.is_ok());
    });
    receiver.recv_timeout(limit).ok()
}

#[test]
fn test_scan_delimiter_at_end_of_file_terminates() {
    // The external scanner used to decide these cases by copying `TSLexer` and restoring it, which
    // rewinds the lookahead character but not the position. At end of input, where `advance` has
    // nothing left to consume, the restored character came back forever and the parse never
    // returned. Each of these is a syntax error; the point is that saying so takes finite time.
    for source in [
        "@",
        "let x = @",
        "<Doc:string>text@",
        "<Doc:string>text&",
        "<Doc>text&",
    ] {
        let outcome = parse_within(source, Duration::from_secs(10));
        assert_eq!(
            outcome,
            Some(false),
            "Parsing {source:?} should report a syntax error rather than run forever"
        );
    }
}

#[test]
fn test_scan_lone_at_in_embedded_text_is_literal() {
    // Only "@{" opens a braced value inside typed text content; a bare '@' is ordinary text, and
    // the character after it must not be swallowed along with it.
    let source = "<Doc:string>a@b</Doc>";
    let result = parse_str(source, "test.nx");

    assert!(
        result.is_ok(),
        "A lone '@' in embedded text should parse: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );

    let element = find_first_kind(
        &result.root().expect("Should have root node"),
        SyntaxKind::ELEMENT,
    )
    .expect("Expected element");
    let content = element
        .child_by_field("content")
        .expect("Expected embedded text content");
    assert_eq!(
        content.text(),
        "a@b",
        "The '@' and the character after it are both text"
    );
}

// ============================================================================
// Component type parameters
// ============================================================================

/// Collects the PROPERTY_DEFINITION nodes of the first component signature in `source`.
fn component_property_definitions(source: &str) -> Vec<(String, String, bool)> {
    let result = parse_str(source, "test.nx");
    assert!(
        result.is_ok(),
        "expected a clean parse, got: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );
    let root = result.root().expect("Should have root node");
    let component = root
        .children()
        .find(|c| c.kind() == SyntaxKind::COMPONENT_DEFINITION)
        .expect("Should find component_definition node");
    let signature = component
        .child_by_field("signature")
        .expect("Component should expose signature field");
    signature
        .children()
        .filter(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .map(|prop| {
            let name = prop
                .child_by_field("name")
                .expect("prop name")
                .text()
                .to_string();
            let ty = prop.child_by_field("type").expect("prop type");
            (name, ty.text().to_string(), ty.raw().is_named())
        })
        .collect()
}

#[test]
fn test_parse_component_prop_named_type() {
    let props = component_property_definitions("component <X type:string /> = { <a /> }");
    assert_eq!(
        props,
        vec![("type".to_string(), "string".to_string(), true)],
        "a prop named `type` should parse as an ordinary prop with a type reference"
    );
}

#[test]
fn test_parse_record_field_named_type() {
    let result = parse_str("type X = { type:string }", "test.nx");
    assert!(
        result.is_ok(),
        "a record field named `type` should parse, got: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );
    let root = result.root().expect("Should have root node");
    let record = root
        .children()
        .find(|c| c.kind() == SyntaxKind::RECORD_DEFINITION)
        .expect("Should find record_definition node");
    let prop = record
        .children()
        .find(|c| c.kind() == SyntaxKind::PROPERTY_DEFINITION)
        .expect("record should declare one property");
    assert_eq!(prop.child_by_field("name").expect("name").text(), "type");
    let ty = prop.child_by_field("type").expect("type");
    assert_eq!(ty.kind(), SyntaxKind::TYPE);
    assert!(
        ty.raw().is_named(),
        "the field type should be a type reference"
    );
    assert_eq!(ty.text(), "string");
}

#[test]
fn test_parse_leading_component_type_parameter() {
    let props = component_property_definitions(
        "external component <SkiaLayout extends SkiaControl TItem:type layoutType?:string itemsSource?:TItem+ />",
    );
    assert_eq!(
        props,
        vec![
            ("TItem".to_string(), "type".to_string(), false),
            ("layoutType".to_string(), "string".to_string(), true),
            ("itemsSource".to_string(), "TItem+".to_string(), true),
        ],
        "the type parameter should carry the `type` keyword token; the props ordinary type references"
    );
}

#[test]
fn test_parse_type_parameter_rejects_suffix() {
    for source in [
        "component <Bad TItem:type? /> = { <Label /> }",
        "component <Bad TItem:type+ /> = { <Label /> }",
        "component <Bad TItem?:type /> = { <Label /> }",
    ] {
        let result = parse_str(source, "test.nx");
        assert!(!result.is_ok(), "{source} should fail to parse");
    }
}

/// Parses `source` and returns the messages of every `invalid-type-parameter` diagnostic.
fn type_parameter_errors(source: &str) -> Vec<String> {
    let result = parse_str(source, "test.nx");
    result
        .errors
        .iter()
        .filter(|diagnostic| diagnostic.code() == Some("invalid-type-parameter"))
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

#[test]
fn test_validate_type_parameter_after_prop_is_rejected() {
    let errors =
        type_parameter_errors("component <Bad items:object+ TItem:type /> = { <Label /> }");
    assert_eq!(
        errors,
        vec!["Type parameter 'TItem' must be declared before every prop".to_string()]
    );

    let errors = type_parameter_errors(
        "component <Ok TKey:type TValue:type key:TKey value:TValue /> = { <Label /> }",
    );
    assert!(
        errors.is_empty(),
        "leading type parameters are valid: {errors:?}"
    );
}

#[test]
fn test_validate_type_parameter_with_default_is_rejected() {
    let result = parse_str(
        "component <Bad TItem:type = object /> = { <Label /> }",
        "test.nx",
    );
    assert!(
        !result.is_ok(),
        "a type parameter default should be rejected"
    );

    let errors = type_parameter_errors("component <Bad TItem:type = 1 /> = { <Label /> }");
    assert_eq!(
        errors,
        vec!["Type parameter 'TItem' cannot have a default value".to_string()]
    );
}

#[test]
fn test_validate_type_parameter_named_after_a_primitive_is_rejected() {
    let errors = type_parameter_errors("external component <List string:type items?:string+ />");
    assert_eq!(
        errors,
        vec!["Type parameter 'string' cannot take the name of a primitive type".to_string()]
    );

    // Every primitive name is refused; a declared type's name is not validation's concern.
    for primitive in nx_syntax::PRIMITIVE_TYPE_NAMES {
        let errors = type_parameter_errors(&format!(
            "external component <List {primitive}:type items?:{primitive}+ />"
        ));
        assert_eq!(errors.len(), 1, "{primitive}: {errors:?}");
    }
    let errors = type_parameter_errors(
        "type Contact = { name:string }\nexternal component <List Contact:type items?:Contact+ />",
    );
    assert!(errors.is_empty(), "{errors:?}");

    // `Element` is a built-in, refused on the same terms as a primitive.
    let errors =
        type_parameter_errors("component <List Element:type slot?:Element /> = { <Label /> }");
    assert_eq!(
        errors,
        vec![
            "Type parameter 'Element' cannot take the name of the built-in type 'Element'"
                .to_string()
        ]
    );
}

#[test]
fn test_validate_type_parameter_with_modifier_is_rejected() {
    let errors = type_parameter_errors("component <Worse content TItem:type /> = { <Label /> }");
    assert_eq!(
        errors,
        vec!["Type parameter 'TItem' cannot have the 'content' modifier".to_string()]
    );
}

#[test]
fn test_validate_type_parameter_outside_component_signature_is_rejected() {
    for (source, expected) in [
        (
            "action Select = { T:type }",
            "Type parameter 'T' is not supported in an action",
        ),
        (
            "component <C emits { click { T:type } } /> = { <Label /> }",
            "Type parameter 'T' is not supported in an emitted action",
        ),
        (
            "component <C /> = { state { T:type } <Label /> }",
            "Type parameter 'T' is not supported in a state group",
        ),
        (
            "let f(T:type) = 1",
            "Type parameter 'T' is not supported in a function parameter list",
        ),
        (
            "type F = <function T:type />: string",
            "Type parameter 'T' is not supported in a function type",
        ),
    ] {
        let errors = type_parameter_errors(source);
        assert_eq!(errors, vec![expected.to_string()], "for source: {source}");
    }
}

#[test]
fn test_validate_record_type_parameter_is_accepted() {
    for source in [
        "type Range = { T:type start:T end:T endInclusive:boolean }",
        "type Pair = { TKey:type TValue:type key:TKey value:TValue }",
        "type Page = { T:type items:T+ next?:T render?:<function item:T />: string }",
    ] {
        let errors = type_parameter_errors(source);
        assert!(errors.is_empty(), "for source: {source}: {errors:?}");
    }
}

#[test]
fn test_validate_record_type_parameter_misuse_is_rejected() {
    for (source, expected) in [
        (
            "type Bad = { start:int T:type }",
            "Type parameter 'T' must be declared before every field",
        ),
        (
            "type A = { T:type = int }",
            "Type parameter 'T' cannot have a default value",
        ),
        (
            "type B = { content T:type }",
            "Type parameter 'T' cannot have the 'content' modifier",
        ),
        (
            "type C = { string:type }",
            "Type parameter 'string' cannot take the name of a primitive type",
        ),
        (
            "type D = { Element:type }",
            "Type parameter 'Element' cannot take the name of the built-in type 'Element'",
        ),
    ] {
        let errors = type_parameter_errors(source);
        assert_eq!(errors, vec![expected.to_string()], "for source: {source}");
    }
}

// ============================================================================
// Applied types
// ============================================================================

/// Every `applied_type` in `source`, in document order, as its tag and its `name=type` arguments.
fn applied_types(source: &str) -> Vec<(String, Vec<(String, String)>)> {
    let result = parse_str(source, "test.nx");
    assert!(
        result.is_ok(),
        "expected a clean parse, got: {}",
        render_diagnostics_cli(&result.errors, &HashMap::new())
    );
    let root = result.root().expect("Should have root node");
    let mut nodes = Vec::new();
    collect_kinds(&root, SyntaxKind::APPLIED_TYPE, &mut nodes);
    nodes
        .iter()
        .map(|applied| {
            let name = applied
                .child_by_field("name")
                .expect("applied type exposes a name field")
                .text()
                .to_string();
            let arguments = applied
                .children()
                .filter(|child| child.kind() == SyntaxKind::TYPE_ARGUMENT)
                .map(|argument| {
                    (
                        argument
                            .child_by_field("name")
                            .expect("argument name")
                            .text()
                            .to_string(),
                        argument
                            .child_by_field("type")
                            .expect("argument type")
                            .text()
                            .to_string(),
                    )
                })
                .collect();
            (name, arguments)
        })
        .collect()
}

/// One applied type with one `T=int` argument, the shape most of these cases expect.
fn int_range() -> Vec<(String, Vec<(String, String)>)> {
    vec![(
        "Range".to_string(),
        vec![("T".to_string(), "int".to_string())],
    )]
}

#[test]
fn test_parse_applied_type_as_a_field_type() {
    assert_eq!(
        applied_types("type Slider = { range:<Range T=int/> }"),
        int_range()
    );
}

#[test]
fn test_parse_applied_type_as_a_let_annotation() {
    assert_eq!(
        applied_types("let r:<Range T=int/> = <Range T=int start={1} end={5} />"),
        int_range(),
        "only the type annotation is an applied type; the construction is an element"
    );
}

#[test]
fn test_parse_applied_type_as_an_alias_target() {
    assert_eq!(applied_types("type IntRange = <Range T=int/>"), int_range());
}

#[test]
fn test_parse_applied_type_under_suffixes() {
    // The suffixes belong to the enclosing `type`, so the applied type itself is unchanged.
    assert_eq!(
        applied_types("type Schedule = { slots:<Range T=int/>+ override?:<Range T=int/> }"),
        [int_range(), int_range()].concat()
    );
    let result = parse_str("type S = <Range T=int/>*", "test.nx");
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn test_parse_applied_type_nests() {
    assert_eq!(
        applied_types("type Nested = { b:<Box T=<Box T=int/>/> }"),
        vec![
            (
                "Box".to_string(),
                vec![("T".to_string(), "<Box T=int/>".to_string())]
            ),
            (
                "Box".to_string(),
                vec![("T".to_string(), "int".to_string())]
            ),
        ]
    );
}

#[test]
fn test_parse_applied_type_with_a_suffixed_argument() {
    assert_eq!(
        applied_types("type Ints = { b:<Box T=int+/> }"),
        vec![(
            "Box".to_string(),
            vec![("T".to_string(), "int+".to_string())]
        )]
    );
}

#[test]
fn test_parse_applied_type_with_zero_arguments() {
    // `<Range/>` parses so the missing argument is named by a diagnostic, not a parse error.
    assert_eq!(
        applied_types("type Bad = { r:<Range/> }"),
        vec![("Range".to_string(), Vec::new())]
    );
}

#[test]
fn test_parse_applied_type_with_a_qualified_tag() {
    assert_eq!(
        applied_types("let u:<Range.Update T=int/> = <Range.Update T=int end={9} />"),
        vec![(
            "Range.Update".to_string(),
            vec![("T".to_string(), "int".to_string())]
        )]
    );
}

#[test]
fn test_parse_applied_type_beside_a_function_type() {
    // `function` is a keyword only in the name slot, so the two element-shaped types never collide.
    let source = "type Page = { T:type render:(<function item:<Box T=int/> />: T)? }";
    assert_eq!(
        applied_types(source),
        vec![(
            "Box".to_string(),
            vec![("T".to_string(), "int".to_string())]
        )]
    );
    let result = parse_str(source, "test.nx");
    let root = result.root().expect("Should have root node");
    assert_eq!(count_kind(&root, SyntaxKind::FUNCTION_TYPE), 1);
}

// ============================================================================
// Function types
// ============================================================================

fn invalid_fixture_diagnostics(relative: &str) -> (nx_syntax::ParseResult, String) {
    let path = fixture_path(relative);
    let result = parse_file(&path).expect("Should parse file");
    let mut sources = HashMap::new();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    sources.insert(file_name, fs::read_to_string(&path).unwrap_or_default());
    let rendered = render_diagnostics_cli(&result.errors, &sources);
    (result, rendered)
}

#[test]
fn test_snapshot_function_types_fixture() {
    let path = fixture_path("valid/function-types.nx");
    let result = parse_file(&path).unwrap();
    assert!(
        result.is_ok(),
        "function-types.nx should parse: {:?}",
        result.errors
    );
    let root = result.root().expect("Should have root");
    assert!(
        !root.has_error(),
        "function-types.nx should contain no error nodes"
    );
    assert!(
        !contains_missing(&root),
        "function-types.nx should contain no MISSING nodes"
    );
    assert_eq!(count_kind(&root, SyntaxKind::FUNCTION_TYPE), 11);
    assert_eq!(count_kind(&root, SyntaxKind::PARENTHESIZED_TYPE), 4);

    insta::assert_snapshot!(root.raw().to_sexp());
}

#[test]
fn test_suffix_after_function_result_binds_to_the_result() {
    let result = parse_str("type Maybe = <function Count:int />: string?", "test.nx");
    assert!(result.is_ok(), "{:?}", result.errors);
    let root = result.root().unwrap();
    let alias_type = find_first_kind(&root, SyntaxKind::TYPE).unwrap();
    // The outer type has the function type as its base and no suffix of its own.
    let outer_tokens: Vec<_> = alias_type
        .children_with_tokens()
        .map(|child| child.kind())
        .collect();
    assert_eq!(outer_tokens, vec![SyntaxKind::FUNCTION_TYPE]);
    let function_type = find_first_kind(&root, SyntaxKind::FUNCTION_TYPE).unwrap();
    let result_type = function_type.child_by_field("result").unwrap();
    let result_tokens: Vec<_> = result_type
        .children_with_tokens()
        .map(|child| child.kind())
        .collect();
    assert_eq!(
        result_tokens,
        vec![SyntaxKind::PRIMITIVE_TYPE, SyntaxKind::QUESTION]
    );
}

#[test]
fn test_function_is_an_identifier_outside_a_function_type() {
    let path = fixture_path("valid/function-identifier.nx");
    let result = parse_file(&path).unwrap();
    assert!(result.is_ok(), "{:?}", result.errors);
    let root = result.root().unwrap();
    assert_eq!(count_kind(&root, SyntaxKind::FUNCTION_KW), 0);
    assert_eq!(count_kind(&root, SyntaxKind::FUNCTION_TYPE), 0);
    assert_eq!(count_kind(&root, SyntaxKind::VALUE_DEFINITION), 2);
    assert_eq!(count_kind(&root, SyntaxKind::FUNCTION_DEFINITION), 1);
}

#[test]
fn test_function_type_default_is_rejected() {
    let (result, rendered) = invalid_fixture_diagnostics("invalid/function-type-default.nx");
    assert!(!result.is_ok());
    let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(codes, vec!["function-type-default"], "{rendered}");
    assert!(
        rendered.contains("cannot carry a default value"),
        "{rendered}"
    );
}

#[test]
fn test_function_type_type_parameter_is_rejected() {
    let (result, rendered) = invalid_fixture_diagnostics("invalid/function-type-type-parameter.nx");
    assert!(!result.is_ok());
    let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(codes, vec!["invalid-type-parameter"], "{rendered}");
    assert!(
        rendered.contains("Type parameter 'TItem' is not supported in a function type"),
        "{rendered}"
    );
}

#[test]
fn test_function_type_second_content_parameter_is_rejected() {
    let (result, rendered) =
        invalid_fixture_diagnostics("invalid/function-type-duplicate-content.nx");
    assert!(!result.is_ok());
    let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(codes, vec!["function-type-duplicate-content"], "{rendered}");
    assert!(rendered.contains("'More'"), "{rendered}");
}

#[test]
fn test_second_occurrence_suffix_across_a_parenthesis_is_rejected() {
    let (result, rendered) =
        invalid_fixture_diagnostics("invalid/parenthesized-second-occurrence.nx");
    assert!(!result.is_ok());
    let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(codes, vec!["second-occurrence-suffix"], "{rendered}");
    assert!(
        rendered.contains("already carries an occurrence"),
        "{rendered}"
    );
    let primary = result.errors[0]
        .labels()
        .iter()
        .find(|label| label.primary)
        .unwrap();
    let source =
        fs::read_to_string(fixture_path("invalid/parenthesized-second-occurrence.nx")).unwrap();
    assert_eq!(&source[primary.range], "*");
    assert_eq!(
        usize::from(primary.range.start()),
        source.trim_end().len() - 1,
        "the outer `*` is the one reported"
    );

    // Parentheses add no layer however many of them there are.
    let nested = parse_str("type Twice = ((string?))?", "test.nx");
    assert!(
        !nested.is_ok(),
        "a nested parenthesis is still the same chain"
    );
    let nested_codes: Vec<_> = nested.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(
        nested_codes,
        vec!["second-occurrence-suffix"],
        "{:?}",
        nested.errors
    );
}

#[test]
fn test_second_occurrence_suffix_is_rejected_wherever_it_is_written() {
    // One suffix per chain, so the second one is the one reported, wherever the first was.
    let second = [
        ("type E = string??", "string??"),
        ("type F = string?+", "string?+"),
        ("type G = (string+)*", "(string+)*"),
        ("type H = <function />: int*?", "int*?"),
        ("type Deep = ((string+))?", "((string+))?"),
    ];

    for (source, described) in second {
        let result = parse_str(source, "test.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
        assert_eq!(
            codes,
            vec!["second-occurrence-suffix"],
            "{described} should report one occurrence error: {:?}",
            result.errors
        );
        let primary = result.errors[0]
            .labels()
            .iter()
            .find(|label| label.primary)
            .expect("the occurrence error should have a primary label");
        assert_eq!(
            usize::from(primary.range.start()),
            source.len() - 1,
            "the trailing suffix is the one reported in {described}"
        );
    }

    // Parentheses add no layer at all, and a function type's result is a chain of its own.
    for source in [
        "type Maybe = string?",
        "type Names = string+",
        "type Tags = string*",
        "type Paren = (string)+",
        "type Loaders = (<function />: string+)+",
        "type Loader = <function />: string+",
    ] {
        let result = parse_str(source, "test.nx");
        assert!(result.is_ok(), "{source}: {:?}", result.errors);
    }
}

#[test]
fn test_list_suffix_is_rejected_naming_the_occurrence_spellings() {
    for (source, star, plus) in [
        ("type Names = string[]", "`string*`", "`string+`"),
        (
            "type Rows = <Range T=int/>[]",
            "`<Range T=int/>*`",
            "`<Range T=int/>+`",
        ),
        ("type Box = { items:Item[] }", "`Item*`", "`Item+`"),
    ] {
        let result = parse_str(source, "test.nx");
        let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
        assert_eq!(
            codes,
            vec!["removed-list-suffix"],
            "{source}: {:?}",
            result.errors
        );
        let note = result.errors[0].note().unwrap_or_default();
        assert!(
            note.contains(star) && note.contains(plus),
            "{source}: {note}"
        );
        let primary = result.errors[0]
            .labels()
            .iter()
            .find(|label| label.primary)
            .unwrap();
        assert_eq!(&source[primary.range], "[]", "{source}");
    }
}

#[test]
fn test_misspelled_function_keyword_parses_as_an_applied_type() {
    // `<Name .../>` in a type position is an applied type, so a misspelled `function` keyword is a
    // well-formed parse that names a record; the rejection belongs to name resolution.
    let result = parse_str("external component <List Name: <functon /> />", "test.nx");
    assert!(result.is_ok(), "{:?}", result.errors);
    let root = result.root().unwrap();
    assert_eq!(count_kind(&root, SyntaxKind::APPLIED_TYPE), 1);
}

#[test]
fn test_unclosed_function_type_yields_one_error_node() {
    let (result, rendered) = invalid_fixture_diagnostics("invalid/function-type-unclosed.nx");
    assert!(!result.is_ok());
    let root = result.root().unwrap();
    assert_eq!(
        count_kind(&root, SyntaxKind::ERROR),
        1,
        "{}",
        root.raw().to_sexp()
    );
    assert_eq!(result.errors.len(), 1, "{rendered}");
}

#[test]
fn test_validate_optional_property_with_default_quotes_both_forms() {
    let result = parse_str("type Book = { subtitle?:string = \"none\" }", "test.nx");
    let codes: Vec<_> = result.errors.iter().filter_map(|d| d.code()).collect();
    assert_eq!(
        codes,
        vec!["optional-property-with-default"],
        "{:?}",
        result.errors
    );
    assert!(
        result.errors[0]
            .message()
            .contains("write `subtitle:string = \"none\"` or `subtitle?:string`"),
        "{:?}",
        result.errors
    );

    // A function type's parameter and a type parameter reject any default by their own rules,
    // so an optional one is not reported twice.
    for (source, code) in [
        (
            "type Render = <function Index?:int = 1 />: string",
            "function-type-default",
        ),
        (
            "component <Bad TItem?:type = object /> = { <Label /> }",
            "type-parameter-definition",
        ),
    ] {
        let result = parse_str(source, "test.nx");
        assert!(
            result
                .errors
                .iter()
                .all(|d| d.code() != Some("optional-property-with-default")),
            "{source}: {:?}",
            result.errors
        );
        assert!(
            !result.errors.is_empty(),
            "{source}: expected the {code} rule to fire"
        );
    }
}
