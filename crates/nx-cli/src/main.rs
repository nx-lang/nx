//! NX CLI - Command-line tools for parsing, checking, and running NX code.
//!
//! Provides commands like:
//! - `nxlang run <file>` - Run an NX file and output the result
//! - `nxlang parse <file>` - Parse and display AST (future)
//! - `nxlang check <file>` - Type check and report errors (future)
//! - `nxlang format <file>` - Format NX source code (future)

mod format;

use clap::{Parser, Subcommand};
use nx_diagnostics::Severity;
use nx_hir::{lower, Item, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_file;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "nxlang")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "NX Language CLI - Tools for NX development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an NX file and output the result
    ///
    /// Executes the root function in the NX file and prints the result.
    /// If the file has no root element/function, an error is reported.
    Run {
        /// Path to the NX file to run
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => run_file(&file),
    }
}

fn run_file(path: &PathBuf) -> ExitCode {
    // Check if file exists
    if !path.exists() {
        eprintln!("Error: File not found: {}", path.display());
        return ExitCode::from(1);
    }

    // Check if it's an .nx file
    if path.extension().and_then(|e| e.to_str()) != Some("nx") {
        eprintln!(
            "Warning: File '{}' does not have .nx extension",
            path.display()
        );
    }

    // Parse the file
    let parse_result = match parse_file(path) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::from(1);
        }
    };

    // Check for parse errors
    let has_errors = parse_result
        .errors
        .iter()
        .any(|d| d.severity() == Severity::Error);

    if has_errors {
        eprintln!("Parse errors in '{}':", path.display());
        for error in &parse_result.errors {
            if error.severity() == Severity::Error {
                eprintln!("  {}", error.message());
            }
        }
        return ExitCode::from(1);
    }

    // Get the syntax tree
    let tree = match parse_result.tree {
        Some(t) => t,
        None => {
            eprintln!("Error: Failed to parse file");
            return ExitCode::from(1);
        }
    };

    // Lower to HIR
    let source_id = SourceId::new(
        path.to_string_lossy()
            .as_bytes()
            .iter()
            .fold(0u32, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u32)),
    );
    let module = lower(tree.root(), source_id);

    // Check if there's a root function
    let has_root = module
        .items()
        .iter()
        .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == "root"));

    if !has_root {
        eprintln!("Error: No root element found in '{}'", path.display());
        eprintln!("Hint: Add a top-level element to create an implicit root function.");
        return ExitCode::from(1);
    }

    // Execute the root function
    let interpreter = Interpreter::new();
    match interpreter.execute_function(&module, "root", vec![]) {
        Ok(value) => {
            let output = format_output(&value);
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn format_output(value: &Value) -> String {
    format::format_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_nx_file(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.nx");
        fs::write(&file_path, content).unwrap();
        (dir, file_path)
    }

    #[test]
    fn test_run_simple_element() {
        let (_dir, path) = create_temp_nx_file("<div class=\"test\" />");

        // Parse and run
        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_run_no_root() {
        // A file with only a function definition, no top-level element
        let (_dir, path) =
            create_temp_nx_file("let <Button text:string /> = <button>{text}</button>");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        // Should have Button function but no root
        let has_root = module
            .items()
            .iter()
            .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == "root"));

        assert!(!has_root);
    }

    #[test]
    fn test_run_explicit_root_with_int() {
        let (_dir, path) = create_temp_nx_file("let root() = { 42 }");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(format_output(&value), "42");
    }

    #[test]
    fn test_run_explicit_root_with_string() {
        let (_dir, path) = create_temp_nx_file("let root() = { \"Hello, World!\" }");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(format_output(&value), "Hello, World!");
    }

    #[test]
    fn test_run_explicit_root_with_arithmetic() {
        let (_dir, path) = create_temp_nx_file("let root() = { 2 + 3 * 4 }");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(format_output(&value), "14");
    }

    #[test]
    fn test_run_record_literal() {
        let source = r#"
            type User = {
              name: string
              age: int = 30
            }

            let root() = { <User name="Alice" /> }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let output = format_output(&result.unwrap());
        assert!(output.contains("name=\"Alice\""));
        assert!(output.contains("age=\"30\""));
    }

    #[test]
    fn test_run_boolean_result() {
        let (_dir, path) = create_temp_nx_file("let root() = { true }");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(format_output(&value), "true");
    }

    #[test]
    fn test_run_null_result() {
        let (_dir, path) = create_temp_nx_file("let root() = { null }");

        let parse_result = parse_file(&path).unwrap();
        assert!(parse_result.is_ok());

        let tree = parse_result.tree.unwrap();
        let module = lower(tree.root(), SourceId::new(0));

        let interpreter = Interpreter::new();
        let result = interpreter.execute_function(&module, "root", vec![]);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(format_output(&value), "null");
    }
}
