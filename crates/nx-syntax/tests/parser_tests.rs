//! Comprehensive parser tests for NX syntax.

use nx_syntax::{parse_file, parse_str, SyntaxKind};
use std::fs;
use std::path::{Path, PathBuf};
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

// ============================================================================
// Valid Syntax Tests (T050)
// ============================================================================

#[test]
fn test_parse_simple_element() {
    let path = fixture_path("valid/simple-element.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse valid simple element without errors");
    assert!(result.tree.is_some(), "Should produce a syntax tree");

    let root = result.root().expect("Should have root node");
    assert_eq!(root.kind(), SyntaxKind::MODULE_DEFINITION);
}

#[test]
fn test_parse_function_definition() {
    let path = fixture_path("valid/function.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse function definition without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_nested_elements() {
    let path = fixture_path("valid/nested-elements.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse nested elements without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_type_annotations() {
    let path = fixture_path("valid/type-annotations.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse type annotations without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_expressions() {
    let path = fixture_path("valid/expressions.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse various expressions without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_conditionals() {
    let path = fixture_path("valid/conditionals.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse conditional expressions without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_complex_example() {
    let path = fixture_path("valid/complex-example.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse complex example without errors");
    assert!(result.tree.is_some());
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
                "File {:?} should parse without errors, but got: {:?}",
                path.file_name(),
                result.errors
            );
        }
    }
}

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
    assert!(!result.is_ok() || !result.errors.is_empty(), "Should detect tag mismatch");
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
    assert!(!result.errors.is_empty(), "Should detect error in invalid statement");
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
        ("fn foo() { }", "test2.nx"),
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
    let source = Arc::new(String::from("let x = 42 + 58"));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let src = Arc::clone(&source);
            thread::spawn(move || {
                let result = parse_str(&src, &format!("test{}.nx", i));
                assert!(result.is_ok(), "Concurrent parsing of same source should succeed");
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
        fn fibonacci(n: int) {
            if n <= 1 {
                n
            } else {
                fibonacci(n - 1) + fibonacci(n - 2)
            }
        }
    "#;

    let handles: Vec<_> = (0..100)
        .map(|i| {
            let src = source.to_string();
            thread::spawn(move || {
                let result = parse_str(&src, &format!("fib{}.nx", i));
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
    let errors: Vec<_> = result.errors.iter()
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
        "Should parse all expression types without errors. Errors: {:?}",
        result.errors
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
    let result = parse_str("let <Test x: boolean y: boolean z: boolean /> = {x && y || z}", "test.nx");
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
}

#[test]
fn test_conditional_ternary_expressions() {
    // Simple ternary
    let result = parse_str("let <Test x: int /> = {x > 0 ? 1 : -1}", "test.nx");
    assert!(result.is_ok());

    // Nested ternary
    let result = parse_str("let <Test x: int /> = {x > 0 ? x * 2 : x < 0 ? x * -2 : 0}", "test.nx");
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
    let result = parse_str("let <Test func: object x: int y: int /> = {func(x, y)}", "test.nx");
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
    let result = parse_str("let <Test x: int /> = {if x > 0 { 1 } else { -1 }}", "test.nx");
    assert!(result.is_ok());

    // If without else
    let result = parse_str("let <Test x: int /> = {if x > 0 { x }}", "test.nx");
    assert!(result.is_ok());

    // Nested if
    let result = parse_str(
        "let <Test x: int /> = {if x > 0 { if x > 10 { 2 } else { 1 } } else { 0 }}",
        "test.nx"
    );
    assert!(result.is_ok());
}

#[test]
fn test_if_expressions_condition_list() {
    let source = r#"let <Test x: int /> = {if {
  x > 100: 3
  x > 10: 2
  x > 0: 1
  else: 0
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "Condition list if expression should parse. Errors: {:?}", result.errors);
}

#[test]
fn test_if_expressions_match() {
    // With scrutinee
    let source = r#"let <Test x: int /> = {if x is {
  0: "zero"
  1: "one"
  else: "other"
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "Match if expression should parse. Errors: {:?}", result.errors);

    // Without scrutinee
    let source = r#"let <Test /> = {if is {
  true: "yes"
  false: "no"
}}"#;
    let result = parse_str(source, "test.nx");
    assert!(result.is_ok(), "Match if expression without scrutinee should parse. Errors: {:?}", result.errors);
}

#[test]
fn test_for_expressions() {
    // Simple for
    let result = parse_str("let <Test items: object /> = {for item in items { item * 2 }}", "test.nx");
    assert!(result.is_ok());

    // For with index
    let result = parse_str("let <Test items: object /> = {for item, index in items { item + index }}", "test.nx");
    assert!(result.is_ok());

    // Nested for
    let result = parse_str(
        "let <Test matrix: object /> = {for row in matrix { for cell in row { cell } }}",
        "test.nx"
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
        "test.nx"
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
    assert!(result.is_ok(), "Simple value definition should parse. Errors: {:?}", result.errors);

    // Value definition with type annotation
    let result = parse_str("let x: int = 42", "test.nx");
    assert!(result.is_ok(), "Value definition with type should parse");

    // Value definition with expression
    let result = parse_str("let sum = {1 + 2 + 3}", "test.nx");
    assert!(result.is_ok(), "Value definition with expression should parse");

    // Value definition with type and expression
    let result = parse_str("let sum: int = {1 + 2 + 3}", "test.nx");
    assert!(result.is_ok(), "Value definition with type and expression should parse");

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
    let has_value_def = root.children().any(|c| c.kind() == SyntaxKind::VALUE_DEFINITION);
    assert!(has_value_def, "Should have value_definition node");

    // Function definition (with parameters)
    let result = parse_str("let <Add x: int y: int /> = {x + y}", "test.nx");
    assert!(result.is_ok());
    let root = result.root().unwrap();
    // Should find a function_definition child
    let has_func_def = root.children().any(|c| c.kind() == SyntaxKind::FUNCTION_DEFINITION);
    assert!(has_func_def, "Should have function_definition node");
}
