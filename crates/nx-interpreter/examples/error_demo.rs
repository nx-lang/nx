//! Demo of beautiful error messages with Ariadne formatting

use nx_interpreter::{RuntimeError, RuntimeErrorKind};
use text_size::{TextRange, TextSize};

fn main() {
    println!("=== NX Interpreter Error Display Demo ===\n");

    // Example 1: Division by zero with source location
    demo_division_by_zero();

    println!("\n{}\n", "=".repeat(70));

    // Example 2: Undefined variable with source location
    demo_undefined_variable();

    println!("\n{}\n", "=".repeat(70));

    // Example 3: Type mismatch with source location
    demo_type_mismatch();
}

fn demo_division_by_zero() {
    println!("Example 1: Division by Zero Error\n");

    let source = "let <bad_divide /> = { 10 / 0 }";

    // Create error with location information
    let error = RuntimeError::new(RuntimeErrorKind::DivisionByZero).with_location(TextRange::new(
        TextSize::from(27), // position of '/'
        TextSize::from(28),
    ));

    println!("{}", error.format("example.nx", source));
}

fn demo_undefined_variable() {
    println!("Example 2: Undefined Variable Error\n");

    let source = "let <use_undefined /> = { unknown_var }";

    let error = RuntimeError::new(RuntimeErrorKind::UndefinedVariable {
        name: "unknown_var".into(),
    })
    .with_location(TextRange::new(
        TextSize::from(26), // start of 'unknown_var'
        TextSize::from(37), // end of 'unknown_var'
    ));

    println!("{}", error.format("example.nx", source));
}

fn demo_type_mismatch() {
    println!("Example 3: Type Mismatch Error\n");

    let source = r#"let <bad_add /> = { 5 + "hello" }"#;

    let error = RuntimeError::new(RuntimeErrorKind::TypeMismatch {
        expected: "number".to_string(),
        actual: "int and string".to_string(),
        operation: "addition".to_string(),
    })
    .with_location(TextRange::new(
        TextSize::from(20), // start of expression
        TextSize::from(32), // end of expression
    ));

    println!("{}", error.format("example.nx", source));
}
