//! NX CLI - Command-line tools for parsing, checking, and running NX code.
//!
//! Provides commands like:
//! - `nxlang run <file>` - Run an NX file and output the result
//! - `nxlang parse <file>` - Parse and display AST (future)
//! - `nxlang check <file>` - Type check and report errors (future)
//! - `nxlang format <file>` - Format NX source code (future)

mod format;

use clap::{Parser, Subcommand};
use nx_diagnostics::{render_diagnostics_cli, Severity};
use nx_hir::{lower, Item, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "nxlang")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "NX Language CLI - Tools for NX development", long_about = None)]
#[command(disable_version_flag = true)]
struct Cli {
    /// Print version
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    version: (),

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

    // Read the source file once
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::from(1);
        }
    };

    // Get the file name for error messages
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Parse the source (using the already-read content to ensure consistency)
    let parse_result = parse_str(&source, file_name);

    // Check for parse errors
    let errors: Vec<_> = parse_result
        .errors
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .cloned()
        .collect();

    if !errors.is_empty() {
        // Build a sources map for error rendering
        let mut sources = HashMap::new();
        sources.insert(file_name.to_string(), source);

        // Render errors with line numbers and context
        let rendered = render_diagnostics_cli(&errors, &sources);
        eprint!("{}", rendered);
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

    // Lower to HIR using the same source_id from parsing for consistency
    let source_id = SourceId::new(parse_result.source_id.as_u32());
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
    use nx_syntax::parse_file;
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

    // ===== CLI Integration Tests =====
    // These tests run the actual CLI binary and verify exit codes and output

    /// Helper to run the CLI binary with arguments and capture output
    fn run_cli(args: &[&str]) -> std::process::Output {
        use std::process::Command;

        // Build the path to the test binary
        // In tests, CARGO_MANIFEST_DIR points to the crate's directory
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_dir = manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target")
            .join("debug")
            .join("nxlang");

        Command::new(&target_dir)
            .args(args)
            .output()
            .expect("Failed to execute CLI - ensure 'cargo build' was run first")
    }

    #[test]
    fn test_cli_run_success() {
        let (_dir, path) = create_temp_nx_file("let root() = { 42 }");

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(output.status.success(), "CLI should exit with success");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "42");
    }

    #[test]
    fn test_cli_run_string_output() {
        let (_dir, path) = create_temp_nx_file("let root() = { \"Hello, World!\" }");

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "Hello, World!");
    }

    #[test]
    fn test_cli_run_file_not_found() {
        let output = run_cli(&["run", "/nonexistent/path/to/file.nx"]);

        assert!(!output.status.success(), "CLI should fail for missing file");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("File not found"));
    }

    #[test]
    fn test_cli_run_no_root_error() {
        let (_dir, path) =
            create_temp_nx_file("let <Button text:string /> = <button>{text}</button>");

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(!output.status.success(), "CLI should fail when no root");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("No root element found"));
        assert!(stderr.contains("Hint:"));
    }

    #[test]
    fn test_cli_run_typed_record_preserves_name() {
        let source = r#"
            type User = {
              name: string
              age: int = 30
            }

            let root() = { <User name="Bob" /> }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should use "User" as the tag name, not generic "result"
        assert!(stdout.contains("<User"));
        assert!(stdout.contains("name=\"Bob\""));
        assert!(stdout.contains("age=\"30\""));
    }

    #[test]
    fn test_cli_help() {
        let output = run_cli(&["--help"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("NX Language CLI"));
        assert!(stdout.contains("run"));
    }

    #[test]
    fn test_cli_version() {
        let output = run_cli(&["--version"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_cli_run_missing_argument() {
        let output = run_cli(&["run"]);

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        // clap reports missing required argument
        assert!(stderr.contains("FILE") || stderr.contains("required"));
    }

    #[test]
    fn test_cli_run_parse_error_shows_line_numbers() {
        // Create a file with a syntax error
        let (_dir, path) = create_temp_nx_file("let x = {");

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(!output.status.success(), "CLI should fail on parse error");
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should show error with line number (format: "error file.nx:1:1: ...")
        assert!(
            stderr.contains(":1:"),
            "Error should include line number. Got: {}",
            stderr
        );
        // Should show the source line
        assert!(
            stderr.contains("let x = {"),
            "Error should include source line. Got: {}",
            stderr
        );
        // Should include caret indicators
        assert!(
            stderr.contains("^"),
            "Error should include caret indicator. Got: {}",
            stderr
        );
    }

    #[test]
    fn test_cli_run_parse_error_multiline_shows_correct_line() {
        // Create a file with a syntax error on line 3
        let source = r#"let x = 42
let y = 100
let z = {
"#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(!output.status.success(), "CLI should fail on parse error");
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should show error on line 3 (format: "error file.nx:3:...")
        assert!(
            stderr.contains(":3:"),
            "Error should be on line 3. Got: {}",
            stderr
        );
        // Should show the problematic source line
        assert!(
            stderr.contains("let z = {"),
            "Error should include the problematic source line. Got: {}",
            stderr
        );
    }
}
