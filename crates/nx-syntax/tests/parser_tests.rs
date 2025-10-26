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
    let path = fixture_path("valid/simple_element.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse valid simple element without errors");
    assert!(result.tree.is_some(), "Should produce a syntax tree");

    let root = result.root().expect("Should have root node");
    assert_eq!(root.kind(), SyntaxKind::MODULE_DEFINITION);
}

#[test]
fn test_parse_function_definition() {
    let path = fixture_path("valid/function_definition.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse function definition without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_nested_elements() {
    let path = fixture_path("valid/nested_elements.nx");
    let result = parse_file(&path).unwrap();

    assert!(result.is_ok(), "Should parse nested elements without errors");
    assert!(result.tree.is_some());
}

#[test]
fn test_parse_type_annotations() {
    let path = fixture_path("valid/type_annotations.nx");
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
    let path = fixture_path("valid/complex_example.nx");
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
    let path = fixture_path("invalid/incomplete_expression.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect incomplete expression");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_unclosed_brace() {
    let path = fixture_path("invalid/unclosed_brace.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect unclosed brace");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_mismatched_tags() {
    let path = fixture_path("invalid/mismatched_tags.nx");
    let result = parse_file(&path).unwrap();

    // May have parse errors or validation errors depending on grammar
    assert!(!result.is_ok() || !result.errors.is_empty(), "Should detect tag mismatch");
}

#[test]
fn test_parse_missing_parenthesis() {
    let path = fixture_path("invalid/missing_parenthesis.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect missing parenthesis");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_invalid_element() {
    let path = fixture_path("invalid/invalid_element.nx");
    let result = parse_file(&path).unwrap();

    assert!(!result.is_ok(), "Should detect invalid element syntax");
    assert!(!result.errors.is_empty(), "Should have parse errors");
}

#[test]
fn test_parse_multiple_errors() {
    let path = fixture_path("invalid/multiple_errors.nx");
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
