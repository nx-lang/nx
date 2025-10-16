//! Integration tests for diagnostic rendering.

use nx_diagnostics::{render_diagnostic, Diagnostic, Label};
use std::collections::HashMap;
use text_size::{TextRange, TextSize};

#[test]
fn test_render_type_error() {
    let source = r#"let greeting: string = 42;"#;
    let mut sources = HashMap::new();
    sources.insert("example.nx".to_string(), source.to_string());

    let range = TextRange::new(TextSize::from(23), TextSize::from(25));
    let label = Label::primary("example.nx", range).with_message("expected string, found number");

    let diag = Diagnostic::error("type-mismatch")
        .with_message("Type mismatch in assignment")
        .with_label(label)
        .with_help("Convert the number to a string using `.toString()`")
        .with_note("Variable 'greeting' is declared as type 'string'")
        .build();

    let rendered = render_diagnostic(&diag, &sources);

    // Verify the output contains key elements
    assert!(rendered.contains("Type mismatch in assignment"));
    assert!(rendered.contains("expected string, found number"));
    assert!(rendered.contains("Convert the number to a string"));
}

#[test]
fn test_render_with_multiple_labels() {
    let source = r#"let x = 10;
let y = x + "hello";"#;
    let mut sources = HashMap::new();
    sources.insert("example.nx".to_string(), source.to_string());

    let primary_range = TextRange::new(TextSize::from(20), TextSize::from(21));
    let secondary_range = TextRange::new(TextSize::from(24), TextSize::from(31));

    let diag = Diagnostic::error("type-error")
        .with_message("Cannot add number and string")
        .with_label(Label::primary("example.nx", primary_range).with_message("this is a number"))
        .with_label(
            Label::secondary("example.nx", secondary_range).with_message("this is a string"),
        )
        .with_help("Use explicit type conversion")
        .build();

    let rendered = render_diagnostic(&diag, &sources);

    assert!(rendered.contains("Cannot add number and string"));
    assert!(rendered.contains("this is a number"));
    assert!(rendered.contains("this is a string"));
}

#[test]
fn test_render_warning() {
    let source = "let unused = 42;";
    let mut sources = HashMap::new();
    sources.insert("test.nx".to_string(), source.to_string());

    let range = TextRange::new(TextSize::from(4), TextSize::from(10));

    let diag = Diagnostic::warning("unused-variable")
        .with_message("Variable 'unused' is never used")
        .with_label(Label::primary("test.nx", range))
        .with_help("Remove the variable or prefix it with '_' to suppress this warning")
        .build();

    let rendered = render_diagnostic(&diag, &sources);

    assert!(rendered.contains("Variable 'unused' is never used"));
}
