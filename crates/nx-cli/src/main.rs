//! NX CLI - Command-line tools for parsing, checking, and running NX code.
//!
//! Provides commands like:
//! - `nxlang run <file>` - Run an NX file and output the result
//! - `nxlang generate <path> --language <csharp|typescript>` - Generate language-specific type definitions
//! - `nxlang parse <file>` - Parse and display AST (future)
//! - `nxlang check <file>` - Type check and report errors (future)
//! - `nxlang format <file>` - Format NX source code (future)

mod codegen;
mod format;
mod json;

use clap::{Parser, Subcommand};
use nx_api::{
    build_program_artifact_from_source, LibraryRegistry, NxDiagnostic, ProgramArtifact,
    ProgramBuildContext,
};
use nx_diagnostics::{render_diagnostics_cli, Severity};
use nx_hir::{lower_source_module, Item, LoweredModule};
use nx_interpreter::{Interpreter, Value};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
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

        /// Output format for the evaluation result
        #[arg(long, default_value_t = OutputFormat::Nx)]
        format: OutputFormat,

        /// Write output to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate language-specific type definitions from an NX file or library directory
    ///
    /// Outputs exported NX type declarations. File input generates one file. Directory input
    /// analyzes the full library and writes one generated file per contributing module.
    Generate {
        /// Path to an NX source file or NX library directory
        file: PathBuf,

        /// Target language for generated code
        #[arg(long, value_enum)]
        language: GenLanguage,

        /// Write output to a file for single-file generation or a directory for library generation
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Optional .editorconfig file to control formatting of generated output
        #[arg(long)]
        editorconfig: Option<PathBuf>,

        /// C# namespace for generated types (only used for --language csharp)
        #[arg(long, default_value = "Nx.Generated")]
        csharp_namespace: String,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Nx,
    Json,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum GenLanguage {
    Csharp,
    Typescript,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Nx => write!(f, "nx"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            format,
            output,
        } => run_file(&file, format, output.as_ref()),
        Commands::Generate {
            file,
            language,
            output,
            editorconfig,
            csharp_namespace,
        } => generate_types(
            &file,
            language,
            output.as_ref(),
            editorconfig.as_ref(),
            &csharp_namespace,
        ),
    }
}

fn run_file(path: &PathBuf, format: OutputFormat, output: Option<&PathBuf>) -> ExitCode {
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

    let program = match load_source_program_for_run(&source, path.as_path()) {
        Ok(program) => program,
        Err(exit_code) => return exit_code,
    };
    let Some(module) = program
        .root_modules
        .first()
        .and_then(|artifact| artifact.lowered_module.as_ref())
    else {
        eprintln!("Error: No root module available for '{}'", path.display());
        return ExitCode::from(1);
    };

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
    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    match interpreter.execute_resolved_program_function("root", vec![]) {
        Ok(value) => {
            let output_text = match format_output(&value, format) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return ExitCode::from(1);
                }
            };

            if let Some(output_path) = output {
                if let Err(e) = std::fs::write(output_path, format!("{}\n", output_text)) {
                    eprintln!("Error writing output to '{}': {}", output_path.display(), e);
                    return ExitCode::from(1);
                }
            } else {
                println!("{}", output_text);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn generate_types(
    path: &PathBuf,
    language: GenLanguage,
    output: Option<&PathBuf>,
    editorconfig: Option<&PathBuf>,
    csharp_namespace: &str,
) -> ExitCode {
    let input_kind = match classify_generate_input(path) {
        Ok(kind) => kind,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };

    let target_language = match language {
        GenLanguage::Typescript => codegen::TargetLanguage::TypeScript,
        GenLanguage::Csharp => codegen::TargetLanguage::CSharp,
    };
    let csharp_namespace = match language {
        GenLanguage::Typescript => None,
        GenLanguage::Csharp => Some(csharp_namespace.to_string()),
    };

    let format_target_name = match input_kind {
        GenerateInputKind::SourceFile => output
            .and_then(|output_path| output_path.file_name().and_then(|name| name.to_str()))
            .unwrap_or(codegen::default_single_file_name(target_language)),
        GenerateInputKind::LibraryDirectory => {
            codegen::default_library_target_name(target_language)
        }
    };
    let format = match resolve_format_options(target_language, editorconfig, format_target_name) {
        Ok(format) => format,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };
    let opts = codegen::GenerateTypesOptions {
        language: target_language,
        csharp_namespace,
        format,
    };

    match input_kind {
        GenerateInputKind::SourceFile => generate_types_from_file(path, output, &opts),
        GenerateInputKind::LibraryDirectory => generate_types_from_library(path, output, &opts),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GenerateInputKind {
    SourceFile,
    LibraryDirectory,
}

fn classify_generate_input(path: &Path) -> Result<GenerateInputKind, String> {
    let metadata =
        std::fs::metadata(path).map_err(|_| format!("Input not found: {}", path.display()))?;

    if metadata.is_dir() {
        return Ok(GenerateInputKind::LibraryDirectory);
    }

    if metadata.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("nx") {
            return Ok(GenerateInputKind::SourceFile);
        }

        return Err(format!(
            "Unsupported input '{}': expected a .nx file or a directory",
            path.display()
        ));
    }

    Err(format!(
        "Unsupported input '{}': expected a .nx file or a directory",
        path.display()
    ))
}

fn resolve_format_options(
    language: codegen::TargetLanguage,
    editorconfig: Option<&PathBuf>,
    target_file_name: &str,
) -> Result<codegen::options::FormatOptions, String> {
    match editorconfig {
        Some(path) => codegen::format_options_from_editorconfig(language, path, target_file_name),
        None => Ok(codegen::options::FormatOptions::defaults_for(language)),
    }
}

fn generate_types_from_file(
    path: &Path,
    output: Option<&PathBuf>,
    opts: &codegen::GenerateTypesOptions,
) -> ExitCode {
    if output.is_some_and(|output_path| output_path.is_dir()) {
        eprintln!(
            "Error: Single-file generation requires --output to be a file path, not a directory"
        );
        return ExitCode::from(1);
    }

    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error reading file: {}", error);
            return ExitCode::from(1);
        }
    };

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    let module = match load_source_module(&source, file_name, path) {
        Ok(module) => module,
        Err(exit_code) => return exit_code,
    };

    let output_text = match codegen::generate_types(&module, path, opts) {
        Ok(text) => text,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };

    if let Some(output_path) = output {
        if let Err(error) = std::fs::write(output_path, output_text) {
            eprintln!(
                "Error writing output to '{}': {}",
                output_path.display(),
                error
            );
            return ExitCode::from(1);
        }
    } else {
        print!("{}", output_text);
    }

    ExitCode::SUCCESS
}

fn generate_types_from_library(
    path: &Path,
    output: Option<&PathBuf>,
    opts: &codegen::GenerateTypesOptions,
) -> ExitCode {
    let Some(output_root) = output else {
        eprintln!("Error: Library generation requires an output directory");
        return ExitCode::from(1);
    };

    if output_root.exists() && !output_root.is_dir() {
        eprintln!("Error: Library generation requires --output to be a directory root");
        return ExitCode::from(1);
    }

    let registry = LibraryRegistry::new();
    let library = match registry.load_library_from_directory(path) {
        Ok(library) => library,
        Err(diagnostics) => return render_api_diagnostics(&diagnostics),
    };

    if library.modules.is_empty() {
        eprintln!(
            "Error: '{}' is not a valid NX library root because it contains no .nx source files",
            path.display()
        );
        return ExitCode::from(1);
    }

    let generated_files = match codegen::generate_library_types(library.as_ref(), opts) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };

    if let Err(error) = std::fs::create_dir_all(output_root) {
        eprintln!(
            "Error creating output directory '{}': {}",
            output_root.display(),
            error
        );
        return ExitCode::from(1);
    }

    for file in generated_files {
        let target_path = match resolve_generated_output_path(output_root, &file.relative_path) {
            Ok(path) => path,
            Err(message) => {
                eprintln!("Error: {}", message);
                return ExitCode::from(1);
            }
        };
        if let Some(parent) = target_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                eprintln!(
                    "Error creating output directory '{}': {}",
                    parent.display(),
                    error
                );
                return ExitCode::from(1);
            }
        }

        if let Err(error) = std::fs::write(&target_path, file.content) {
            eprintln!(
                "Error writing output to '{}': {}",
                target_path.display(),
                error
            );
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}

fn resolve_generated_output_path(
    output_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, String> {
    if relative_path.as_os_str().is_empty() {
        return Err("Generated output path must not be empty".to_string());
    }

    for component in relative_path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Generated output path '{}' escapes the output directory",
                    relative_path.display()
                ));
            }
        }
    }

    Ok(output_root.join(relative_path))
}

fn render_api_diagnostics(diagnostics: &[NxDiagnostic]) -> ExitCode {
    for diagnostic in diagnostics {
        eprintln!("error: {}", diagnostic.message);

        for label in &diagnostic.labels {
            if label.file.is_empty() {
                continue;
            }

            eprintln!(
                "  --> {}:{}:{}",
                label.file, label.span.start_line, label.span.start_column
            );
            if let Some(message) = &label.message {
                eprintln!("   | {}", message);
            }
        }

        if let Some(help) = &diagnostic.help {
            eprintln!("help: {}", help);
        }

        if let Some(note) = &diagnostic.note {
            eprintln!("note: {}", note);
        }
    }

    ExitCode::from(1)
}

fn format_output(value: &Value, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Nx => Ok(format::format_value(value)),
        OutputFormat::Json => json::format_value_json_pretty(value),
    }
}

fn load_source_module(
    source: &str,
    file_name: &str,
    _path: &Path,
) -> Result<LoweredModule, ExitCode> {
    match lower_source_module(source, file_name) {
        Ok(module) => Ok(module),
        Err(diagnostics) => Err(render_source_diagnostics(file_name, source, &diagnostics)),
    }
}

fn load_source_program_for_run(source: &str, path: &Path) -> Result<ProgramArtifact, ExitCode> {
    let file_name = path.display().to_string();
    let build_context = ProgramBuildContext::empty();
    let program = match build_program_artifact_from_source(source, &file_name, &build_context) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("Error: Failed to build program artifact: {}", error);
            return Err(ExitCode::from(1));
        }
    };

    if program
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
    {
        return Err(render_source_diagnostics(
            file_name.as_str(),
            source,
            &program.diagnostics,
        ));
    }

    Ok(program)
}

fn render_source_diagnostics(
    file_name: &str,
    source: &str,
    diagnostics: &[nx_diagnostics::Diagnostic],
) -> ExitCode {
    let mut sources = HashMap::new();
    sources.insert(file_name.to_string(), source.to_string());
    let rendered = render_diagnostics_cli(diagnostics, &sources);
    eprint!("{}", rendered);
    ExitCode::from(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_api::LibraryRegistry;
    use nx_hir::{lower, SourceId};
    use nx_syntax::parse_file;
    use nx_value::NxValue;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_temp_nx_file(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.nx");
        fs::write(&file_path, content).unwrap();
        (dir, file_path)
    }

    fn create_temp_library(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let library_path = dir.path().join("library");
        fs::create_dir_all(&library_path).unwrap();

        for (relative_path, content) in files {
            let file_path = library_path.join(relative_path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, content).unwrap();
        }

        (dir, library_path)
    }

    fn build_import_resolved_program(path: &Path) -> ProgramArtifact {
        let source = fs::read_to_string(path).expect("source file should load");
        let file_name = path.display().to_string();
        let module = lower_source_module(&source, &file_name).unwrap_or_else(|diagnostics| {
            panic!("Expected {file_name} to lower, got {:?}", diagnostics)
        });
        let registry = LibraryRegistry::new();

        for import in &module.imports {
            if import.library_path.contains("://") || import.library_path.starts_with("git+") {
                continue;
            }

            let Some(parent) = path.parent() else {
                continue;
            };
            let library_root = parent.join(&import.library_path);
            registry
                .load_library_from_directory(&library_root)
                .unwrap_or_else(|diagnostics| {
                    panic!(
                        "Expected {} to load, got {:?}",
                        library_root.display(),
                        diagnostics
                    )
                });
        }

        let build_context = registry.build_context();
        let artifact = build_program_artifact_from_source(&source, &file_name, &build_context)
            .expect("program artifact should build");
        assert!(
            !artifact
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity() == Severity::Error),
            "Expected import-resolved program to analyze without errors, got {:?}",
            artifact.diagnostics
        );
        artifact
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
    fn test_run_namespace_imported_function() {
        let dir = TempDir::new().expect("temp dir");
        let app_dir = dir.path().join("app");
        let math_dir = dir.path().join("math");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&math_dir).expect("math dir");

        fs::write(
            math_dir.join("add.nx"),
            r#"export let addOne(n:int) = { n + 1 }"#,
        )
        .expect("math library");
        fs::write(
            app_dir.join("main.nx"),
            r#"import "../math" as Math
let root() = { Math.addOne(41) }"#,
        )
        .expect("root file");

        let program = build_import_resolved_program(&app_dir.join("main.nx"));
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let result = interpreter
            .execute_resolved_program_function("root", vec![])
            .expect("qualified imported function should execute");

        assert_eq!(format_output(&result, OutputFormat::Nx).unwrap(), "42");
    }

    #[test]
    fn test_run_qualified_selective_imported_function() {
        let dir = TempDir::new().expect("temp dir");
        let app_dir = dir.path().join("app");
        let ui_dir = dir.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("theme.nx"),
            r#"export let title() = { "Hello" }"#,
        )
        .expect("ui library");
        fs::write(
            app_dir.join("main.nx"),
            r#"import { title as Ui.title } from "../ui"
let root() = { Ui.title() }"#,
        )
        .expect("root file");

        let program = build_import_resolved_program(&app_dir.join("main.nx"));
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let result = interpreter
            .execute_resolved_program_function("root", vec![])
            .expect("qualified imported function should execute");

        assert_eq!(format_output(&result, OutputFormat::Nx).unwrap(), "Hello");
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
        assert_eq!(format_output(&value, OutputFormat::Nx).unwrap(), "42");
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
        assert_eq!(
            format_output(&value, OutputFormat::Nx).unwrap(),
            "Hello, World!"
        );
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
        assert_eq!(format_output(&value, OutputFormat::Nx).unwrap(), "14");
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
        let output = format_output(&result.unwrap(), OutputFormat::Nx).unwrap();
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
        assert_eq!(format_output(&value, OutputFormat::Nx).unwrap(), "true");
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
        assert_eq!(format_output(&value, OutputFormat::Nx).unwrap(), "null");
    }

    // ===== CLI Integration Tests =====
    // These tests run the actual CLI binary and verify exit codes and output

    /// Helper to run the CLI binary with arguments and capture output
    fn run_cli(args: &[&str]) -> std::process::Output {
        use std::process::Command;
        use std::sync::Once;

        static BUILD: Once = Once::new();

        BUILD.call_once(|| {
            let status = Command::new("cargo")
                .args(["build", "-p", "nx-cli", "--bin", "nxlang"])
                .status()
                .expect("Failed to build nxlang binary");

            assert!(status.success(), "Failed to build nxlang binary");
        });

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
    fn test_cli_run_json_string_output() {
        let (_dir, path) = create_temp_nx_file("let root() = { \"Hello, World!\" }");

        let output = run_cli(&["run", path.to_str().unwrap(), "--format", "json"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value = NxValue::from_json_str(stdout.trim()).unwrap();
        assert_eq!(value, NxValue::String("Hello, World!".to_string()));
    }

    #[test]
    fn test_cli_run_json_typed_record_output() {
        let source = r#"
            type User = {
              name: string
              age: int = 30
            }

            let root() = { <User name="Bob" /> }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&["run", path.to_str().unwrap(), "--format", "json"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value = NxValue::from_json_str(stdout.trim()).unwrap();

        let NxValue::Record {
            type_name,
            properties,
        } = value
        else {
            panic!("Expected JSON record. Got: {:?}", value);
        };

        assert_eq!(type_name.as_deref(), Some("User"));
        assert_eq!(
            properties.get("name"),
            Some(&NxValue::String("Bob".to_string()))
        );
        assert_eq!(properties.get("age"), Some(&NxValue::Int(30)));
    }

    #[test]
    fn test_cli_run_json_output_to_file() {
        let (dir, file_path) = create_temp_nx_file("let root() = { 42 }");
        let output_path = dir.path().join("out.json");

        let output = run_cli(&[
            "run",
            file_path.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "");

        let written = fs::read_to_string(&output_path).unwrap();
        let value = NxValue::from_json_str(written.trim()).unwrap();
        assert_eq!(value, NxValue::Int(42));
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
    fn test_cli_generate_file_infers_single_file_generation() {
        let source = r#"
            type Hidden = string
            export type Theme = string
            export action SearchRequested = { query:string }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "generate",
            path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            output.status.success(),
            "CLI should generate for .nx file input"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("export type Theme = string;"));
        assert!(stdout.contains("export interface SearchRequested {"));
        assert!(!stdout.contains("Hidden"));
    }

    #[test]
    fn test_cli_generate_rejects_non_nx_files() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("README.md");
        fs::write(&file_path, "# Not NX").unwrap();

        let output = run_cli(&[
            "generate",
            file_path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(!output.status.success(), "CLI should reject non-NX files");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Unsupported input"));
        assert!(stderr.contains(".nx file or a directory"));
    }

    #[test]
    fn test_cli_generate_library_requires_output_directory() {
        let (_dir, library_path) =
            create_temp_library(&[("theme.nx", "export enum ThemeMode = | light | dark")]);

        let output = run_cli(&[
            "generate",
            library_path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            !output.status.success(),
            "CLI should require --output for library generation"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Library generation requires an output directory"));
    }

    #[test]
    fn test_cli_generate_rejects_empty_library_directory() {
        let dir = TempDir::new().unwrap();
        let library_path = dir.path().join("empty-library");
        let output_path = dir.path().join("generated");
        fs::create_dir_all(&library_path).unwrap();

        let output = run_cli(&[
            "generate",
            library_path.to_str().unwrap(),
            "--language",
            "typescript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            !output.status.success(),
            "CLI should reject empty library directories"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("contains no .nx source files"));
    }

    #[test]
    fn test_cli_generate_surfaces_library_diagnostics() {
        let (_dir, library_path) =
            create_temp_library(&[("broken.nx", r#"export let answer(): int = { "oops" }"#)]);
        let output_path = library_path.parent().unwrap().join("generated");

        let output = run_cli(&[
            "generate",
            library_path.to_str().unwrap(),
            "--language",
            "typescript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            !output.status.success(),
            "CLI should fail when library analysis reports errors"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error:"));
        assert!(stderr.contains("broken.nx"));
    }

    #[test]
    fn test_cli_generate_library_writes_typescript_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export enum ThemeMode = | light | dark"),
            (
                "forms.nx",
                "export type FormState = { theme: ThemeMode }\nexport type FormTheme = ThemeMode",
            ),
        ]);
        let output_path = dir.path().join("generated-ts");

        let output = run_cli(&[
            "generate",
            library_path.to_str().unwrap(),
            "--language",
            "typescript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write TypeScript library output"
        );
        let forms = fs::read_to_string(output_path.join("forms.ts")).unwrap();
        let theme = fs::read_to_string(output_path.join("theme.ts")).unwrap();
        let index = fs::read_to_string(output_path.join("index.ts")).unwrap();

        assert!(forms.contains("import type { ThemeMode } from \"./theme\";"));
        assert!(forms.contains("export interface FormState {"));
        assert!(forms.contains("export type FormTheme = ThemeMode;"));
        assert!(theme.contains("export type ThemeMode = \"light\" | \"dark\";"));
        assert!(index.contains("export * from \"./forms\";"));
        assert!(index.contains("export * from \"./theme\";"));
    }

    #[test]
    fn test_cli_generate_library_writes_csharp_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export enum ThemeMode = | light | dark"),
            ("forms.nx", "export type FormState = { theme: ThemeMode }"),
        ]);
        let output_path = dir.path().join("generated-cs");

        let output = run_cli(&[
            "generate",
            library_path.to_str().unwrap(),
            "--language",
            "csharp",
            "--csharp-namespace",
            "MyApp.Models",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write C# library output"
        );
        let forms = fs::read_to_string(output_path.join("forms.g.cs")).unwrap();
        let theme = fs::read_to_string(output_path.join("theme.g.cs")).unwrap();

        assert!(forms.contains("namespace MyApp.Models"));
        assert!(forms.contains("public sealed class FormState"));
        assert!(forms.contains("public ThemeMode Theme { get; set; }"));
        assert!(theme.contains("namespace MyApp.Models"));
        assert!(theme.contains("public enum ThemeMode"));
    }

    #[test]
    fn test_resolve_generated_output_path_rejects_parent_dir_escape() {
        let output_root = Path::new("/tmp/generated");
        let error = resolve_generated_output_path(output_root, Path::new("../escape.ts"))
            .expect_err("parent-dir output path should be rejected");

        assert!(error.contains("escapes the output directory"));
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
    fn test_cli_run_component_with_action_handler_uses_resolved_program_runtime() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let root() = { <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> /> }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&["run", path.to_str().unwrap()]);

        assert!(
            output.status.success(),
            "CLI should execute handler-producing roots"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("<onSearchSubmitted"));
        assert!(stdout.contains("component=\"SearchBox\""));
        assert!(stdout.contains("action=\"SearchSubmitted\""));
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
