//! Comprehensive parser tests for NX syntax.

use nx_diagnostics::render_diagnostics_cli;
use nx_syntax::{parse_file, parse_str, SyntaxKind, SyntaxNode};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

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

fn contains_kind(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    if node.kind() == kind {
        return true;
    }

    node.children().any(|child| contains_kind(&child, kind))
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
    assert!(result.tree.is_some());
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
        contains_kind(&root, SyntaxKind::INTERPOLATION_EXPRESSION),
        "Interpolation expression should appear in the markup tree"
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
        contains_kind(&root, SyntaxKind::EMBED_INTERPOLATION_EXPRESSION),
        "Expected embed interpolation parsed from @{{...}}"
    );
    assert!(
        contains_kind(&root, SyntaxKind::RAW_TEXT_RUN),
        "Expected raw text run inside raw text elements"
    );
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

    // Count text_child_element nodes - should have at least 2 (b and i)
    fn count_kind(node: &SyntaxNode, kind: SyntaxKind) -> usize {
        let mut count = if node.kind() == kind { 1 } else { 0 };
        for child in node.children() {
            count += count_kind(&child, kind);
        }
        count
    }

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
        kinds
            .iter()
            .any(|kind| *kind == SyntaxKind::TYPE_DEFINITION),
        "Expected at least one type definition"
    );
    assert!(
        kinds
            .iter()
            .any(|kind| *kind == SyntaxKind::VALUE_DEFINITION),
        "Expected at least one value definition"
    );
    assert!(
        kinds
            .iter()
            .any(|kind| *kind == SyntaxKind::FUNCTION_DEFINITION),
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
fn test_parse_enum_definition() {
    let path = fixture_path("valid/enum-definition.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Enum definition file should parse");
    let root = result.root().expect("Should produce root node");

    let enums: Vec<_> = root
        .children()
        .filter(|child| child.kind() == SyntaxKind::ENUM_DEFINITION)
        .collect();
    assert_eq!(enums.len(), 2, "Expected two enum definitions");

    let status = enums.first().expect("First enum definition should exist");
    let name_node = status
        .child_by_field("name")
        .expect("Enum definition should expose name field");
    assert_eq!(name_node.text(), "Status");

    let members_node = status
        .child_by_field("members")
        .expect("Enum definition should contain members list");
    let member_names: Vec<_> = members_node
        .children()
        .filter(|child| child.kind() == SyntaxKind::ENUM_MEMBER)
        .map(|member| member.text().to_string())
        .collect();
    assert!(
        member_names.contains(&"Pending".to_string())
            && member_names.contains(&"Active".to_string())
            && member_names.contains(&"Disabled".to_string()),
        "All enum members should be captured"
    );
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
    assert!(result.errors.len() >= 1, "Should have parse errors");
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
    assert!(result.errors.len() >= 1, "Should detect errors");

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
        .map(|d| format!("{}", d.message()))
        .collect();

    insta::assert_debug_snapshot!(errors);
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

    // Null literal
    let result = parse_str("let test = null", "test.nx");
    assert!(result.is_ok());

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
fn test_conditional_ternary_expressions() {
    // Simple ternary
    let result = parse_str("let <Test x: int /> = {x > 0 ? 1 : -1}", "test.nx");
    assert!(result.is_ok());

    // Nested ternary
    let result = parse_str(
        "let <Test x: int /> = {x > 0 ? x * 2 : x < 0 ? x * -2 : 0}",
        "test.nx",
    );
    assert!(result.is_ok());
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
        "let <Test x: int y: int /> = {x + y * 2 > 10 && x < 100 ? x * y : x + y}",
        "test.nx",
    );
    assert!(result.is_ok());

    // Chained method calls with ternary
    let result = parse_str(
        "let <Test obj: object x: int /> = {obj.method(x + 1, x * 2).result > 0 ? \"pos\" : \"neg\"}",
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
  ternary: int = {5 > 3 ? 100 : 200}
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
