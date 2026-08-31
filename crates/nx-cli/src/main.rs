//! NX CLI - Command-line tools for parsing, checking, and running NX code.
//!
//! Provides commands like:
//! - `nxlang run <file>` - Run an NX file and output the result
//! - `nxlang typegen <path> --language <csharp|typescript>` - Generate language-specific type definitions
//! - `nxlang codegen <path> --target <javascript|typescript|nx-ir>` - Generate executable output
//! - `nxlang parse <file>` - Parse and display AST (future)
//! - `nxlang check <file>` - Type check and report errors (future)
//! - `nxlang format <file>` - Format NX source code (future)

mod format;
mod json;
mod typegen;

use clap::{Parser, Subcommand};
use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact, LibraryRegistry,
    NxDiagnostic, NxWorkspace, ProgramArtifact, ProgramBuildContext,
};
use nx_codegen::{
    emit_js_program_module, emit_nx_ir, emit_program, CodegenOptions, JsProgramModuleOptions,
};
use nx_diagnostics::{render_diagnostics_cli, Diagnostic, Severity};
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
    Typegen {
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

        /// Package prefix for TypeScript dependency-library imports (only used for --language typescript)
        #[arg(long = "typescript-package-prefix")]
        typescript_package_prefix: Option<String>,
    },

    /// Generate executable TypeScript, JavaScript, or NX IR from an NX file or workspace directory
    Codegen {
        /// Path to an NX source file or workspace directory
        file: PathBuf,

        /// Executable output target
        #[arg(long, value_enum)]
        target: ExecutableTarget,

        /// Output directory for generated program and runtime files
        #[arg(short, long)]
        output: PathBuf,

        /// Executable output format
        #[arg(long, default_value_t = ExecutableOutputFormat::Files)]
        format: ExecutableOutputFormat,

        /// Entry module identity when generating from a workspace directory
        #[arg(long)]
        entry: Option<String>,
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

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutableTarget {
    Javascript,
    Typescript,
    #[value(name = "nx-ir")]
    NxIr,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ExecutableOutputFormat {
    Files,
    #[value(name = "program-module")]
    JsProgramModule,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Nx => write!(f, "nx"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

impl std::fmt::Display for ExecutableOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutableOutputFormat::Files => write!(f, "files"),
            ExecutableOutputFormat::JsProgramModule => write!(f, "program-module"),
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
        Commands::Typegen {
            file,
            language,
            output,
            editorconfig,
            csharp_namespace,
            typescript_package_prefix,
        } => generate_types(
            &file,
            language,
            output.as_ref(),
            editorconfig.as_ref(),
            &csharp_namespace,
            typescript_package_prefix.as_deref(),
        ),
        Commands::Codegen {
            file,
            target,
            output,
            format,
            entry,
        } => generate_executable_source(&file, target, format, &output, entry.as_deref()),
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
    typescript_package_prefix: Option<&str>,
) -> ExitCode {
    let input_kind = match classify_generate_input(path) {
        Ok(kind) => kind,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };

    let target_language = match language {
        GenLanguage::Typescript => typegen::TargetLanguage::TypeScript,
        GenLanguage::Csharp => typegen::TargetLanguage::CSharp,
    };
    let csharp_namespace = match language {
        GenLanguage::Typescript => None,
        GenLanguage::Csharp => Some(csharp_namespace.to_string()),
    };
    let typescript_package_prefix = match language {
        GenLanguage::Typescript => typescript_package_prefix.map(str::to_string),
        GenLanguage::Csharp => None,
    };

    let format_target_name = match input_kind {
        GenerateInputKind::SourceFile => output
            .and_then(|output_path| output_path.file_name().and_then(|name| name.to_str()))
            .unwrap_or(typegen::default_single_file_name(target_language)),
        GenerateInputKind::LibraryDirectory => {
            typegen::default_library_target_name(target_language)
        }
    };
    let format = match resolve_format_options(target_language, editorconfig, format_target_name) {
        Ok(format) => format,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };
    let opts = typegen::GenerateTypesOptions {
        language: target_language,
        csharp_namespace,
        typescript_package_prefix,
        format,
    };

    match input_kind {
        GenerateInputKind::SourceFile => generate_types_from_file(path, output, &opts),
        GenerateInputKind::LibraryDirectory => generate_types_from_library(path, output, &opts),
    }
}

fn generate_executable_source(
    path: &PathBuf,
    target: ExecutableTarget,
    format: ExecutableOutputFormat,
    output_root: &Path,
    entry: Option<&str>,
) -> ExitCode {
    let input_kind = match classify_generate_input(path) {
        Ok(kind) => kind,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };

    if output_root.exists() && !output_root.is_dir() {
        eprintln!("Error: Executable codegen requires --output to be a directory root");
        return ExitCode::from(1);
    }

    let artifact = match input_kind {
        GenerateInputKind::SourceFile => load_source_program_for_codegen(path),
        GenerateInputKind::LibraryDirectory => load_workspace_program_for_codegen(path, entry),
    };
    let artifact = match artifact {
        Ok(artifact) => artifact,
        Err(exit_code) => return exit_code,
    };

    if target == ExecutableTarget::NxIr {
        if format != ExecutableOutputFormat::Files {
            eprintln!("Error: NX IR codegen does not use --format; use --target nx-ir");
            return ExitCode::from(1);
        }
        return generate_executable_nx_ir(&artifact, output_root);
    }

    if format == ExecutableOutputFormat::JsProgramModule {
        return generate_executable_js_program_module(&artifact, target, output_root);
    }

    let options = match target {
        ExecutableTarget::Typescript => CodegenOptions::typescript(),
        ExecutableTarget::Javascript => CodegenOptions::javascript(),
        ExecutableTarget::NxIr => unreachable!("NX IR target is handled before source codegen"),
    };
    let generated = match emit_program(&artifact, &options) {
        Ok(output) => output,
        Err(error) => return render_codegen_diagnostics(&artifact, &error.diagnostics),
    };

    for warning in &generated.warnings {
        eprintln!("Warning: {}", warning.message);
    }

    if let Err(error) = std::fs::create_dir_all(output_root) {
        eprintln!(
            "Error creating output directory '{}': {}",
            output_root.display(),
            error
        );
        return ExitCode::from(1);
    }

    for file in generated.files {
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

fn generate_executable_nx_ir(artifact: &ProgramArtifact, output_root: &Path) -> ExitCode {
    let generated = match emit_nx_ir(artifact) {
        Ok(output) => output,
        Err(error) => return render_codegen_diagnostics(artifact, &error.diagnostics),
    };

    if let Err(error) = std::fs::create_dir_all(output_root) {
        eprintln!(
            "Error creating output directory '{}': {}",
            output_root.display(),
            error
        );
        return ExitCode::from(1);
    }

    let target_path =
        match resolve_generated_output_path(output_root, &nx_ir_relative_path(artifact)) {
            Ok(path) => path,
            Err(message) => {
                eprintln!("Error: {}", message);
                return ExitCode::from(1);
            }
        };
    if let Err(error) = std::fs::write(&target_path, generated.json) {
        eprintln!(
            "Error writing output to '{}': {}",
            target_path.display(),
            error
        );
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn nx_ir_relative_path(artifact: &ProgramArtifact) -> PathBuf {
    let stem = Path::new(&artifact.entry_identity)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("program");
    PathBuf::from(format!("{stem}.nxir.json"))
}

fn generate_executable_js_program_module(
    artifact: &ProgramArtifact,
    target: ExecutableTarget,
    output_root: &Path,
) -> ExitCode {
    if target != ExecutableTarget::Javascript {
        eprintln!("Error: Program-module codegen is only supported with --target javascript");
        return ExitCode::from(1);
    }

    let generated = match emit_js_program_module(artifact, &JsProgramModuleOptions::javascript()) {
        Ok(output) => output,
        Err(error) => return render_codegen_diagnostics(artifact, &error.diagnostics),
    };

    if let Err(error) = std::fs::create_dir_all(output_root) {
        eprintln!(
            "Error creating output directory '{}': {}",
            output_root.display(),
            error
        );
        return ExitCode::from(1);
    }

    let target_path = match resolve_generated_output_path(output_root, Path::new("program.js")) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };
    if let Err(error) = std::fs::write(&target_path, generated.source_text) {
        eprintln!(
            "Error writing output to '{}': {}",
            target_path.display(),
            error
        );
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
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
    language: typegen::TargetLanguage,
    editorconfig: Option<&PathBuf>,
    target_file_name: &str,
) -> Result<typegen::options::FormatOptions, String> {
    match editorconfig {
        Some(path) => typegen::format_options_from_editorconfig(language, path, target_file_name),
        None => Ok(typegen::options::FormatOptions::defaults_for(language)),
    }
}

fn generate_types_from_file(
    path: &Path,
    output: Option<&PathBuf>,
    opts: &typegen::GenerateTypesOptions,
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

    let generated = match typegen::generate_types_with_warnings(&module, path, opts) {
        Ok(output) => output,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };
    render_generate_warnings(&generated.warnings);

    if let Some(output_path) = output {
        if let Err(error) = std::fs::write(output_path, generated.value) {
            eprintln!(
                "Error writing output to '{}': {}",
                output_path.display(),
                error
            );
            return ExitCode::from(1);
        }
    } else {
        print!("{}", generated.value);
    }

    ExitCode::SUCCESS
}

fn generate_types_from_library(
    path: &Path,
    output: Option<&PathBuf>,
    opts: &typegen::GenerateTypesOptions,
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

    let generated = match typegen::generate_library_types_with_warnings(library.as_ref(), opts) {
        Ok(output) => output,
        Err(message) => {
            eprintln!("Error: {}", message);
            return ExitCode::from(1);
        }
    };
    render_generate_warnings(&generated.warnings);

    if let Err(error) = std::fs::create_dir_all(output_root) {
        eprintln!(
            "Error creating output directory '{}': {}",
            output_root.display(),
            error
        );
        return ExitCode::from(1);
    }

    for file in generated.value {
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

fn load_source_program_for_codegen(path: &Path) -> Result<ProgramArtifact, ExitCode> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error reading file: {}", error);
            return Err(ExitCode::from(1));
        }
    };

    load_source_program_for_run(&source, path)
}

fn load_workspace_program_for_codegen(
    root_path: &Path,
    entry: Option<&str>,
) -> Result<ProgramArtifact, ExitCode> {
    let workspace = match NxWorkspace::from_directory(root_path) {
        Ok(workspace) => workspace,
        Err(error) => {
            eprintln!("Error: {}", error);
            return Err(ExitCode::from(1));
        }
    };

    if workspace.modules().is_empty() {
        eprintln!(
            "Error: '{}' is not a valid NX workspace because it contains no .nx source files",
            root_path.display()
        );
        return Err(ExitCode::from(1));
    }

    let entry_identity = match entry {
        Some(entry) => entry.to_string(),
        None => match default_workspace_entry(&workspace) {
            Ok(identity) => identity,
            Err(message) => {
                eprintln!("Error: {}", message);
                return Err(ExitCode::from(1));
            }
        },
    };

    build_workspace_program_artifact(&workspace, &entry_identity, &ProgramBuildContext::empty())
        .map_err(|diagnostics| render_api_diagnostics(&diagnostics))
}

fn default_workspace_entry(workspace: &NxWorkspace) -> Result<String, String> {
    let modules = workspace.modules();
    if modules.len() == 1 {
        return Ok(modules[0].identity().to_string());
    }
    if modules.iter().any(|module| module.identity() == "main.nx") {
        return Ok("main.nx".to_string());
    }
    Err("Workspace executable codegen requires --entry when the directory contains multiple .nx files".to_string())
}

fn render_generate_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("Warning: {}", warning);
    }
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

fn render_codegen_diagnostics(program: &ProgramArtifact, diagnostics: &[Diagnostic]) -> ExitCode {
    let mut sources = HashMap::new();
    for entry in program.source_entries() {
        sources.insert(entry.identity.to_string(), entry.source.to_string());
    }
    let rendered = render_diagnostics_cli(diagnostics, &sources);
    eprint!("{}", rendered);
    ExitCode::from(1)
}

fn format_output(value: &Value, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Nx => format::format_value(value),
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
    for diagnostic in diagnostics {
        for label in diagnostic.labels() {
            sources
                .entry(label.file.clone())
                .or_insert_with(|| source.to_string());
        }
    }
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
        // Numbers are emitted unquoted so the output reads back at an int-typed site.
        assert!(output.contains("age=30"));
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
    fn test_cli_typegen_file_infers_single_file_generation() {
        let source = r#"
            type Hidden = string
            export type Theme = string
            export action SearchRequested = { query:string }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
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
        assert!(stdout
            .contains("export interface SearchRequested extends NxRecord<\"SearchRequested\">"));
        assert!(!stdout.contains("Hidden"));
    }

    #[test]
    fn test_cli_generate_command_is_removed() {
        let (_dir, path) = create_temp_nx_file("export type Theme = string");

        let output = run_cli(&[
            "generate",
            path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            !output.status.success(),
            "old generate command should not be accepted"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unrecognized subcommand"));
    }

    #[test]
    fn test_cli_types_command_is_removed() {
        let (_dir, path) = create_temp_nx_file("export type Theme = string");

        let output = run_cli(&["types", path.to_str().unwrap(), "--language", "typescript"]);

        assert!(
            !output.status.success(),
            "old types command should not be accepted"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unrecognized subcommand"));
    }

    #[test]
    fn test_cli_codegen_file_writes_executable_javascript_output() {
        let (dir, path) = create_temp_nx_file("let root() = { 1 + 2 }");
        let output_path = dir.path().join("codegen-js");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "javascript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write executable JavaScript output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let module = fs::read_to_string(output_path.join("m0_test.js")).unwrap();
        let runtime = fs::read_to_string(output_path.join("nx-runtime.js")).unwrap();
        let index = fs::read_to_string(output_path.join("index.js")).unwrap();

        assert!(module.contains("export function root()"));
        assert!(module.contains("return (1 + 2);"));
        assert!(runtime.contains("export function nxElement"));
        assert!(runtime.contains("export function nxRecordSchema"));
        assert!(index.contains("export { root } from \"./m0_test.js\";"));
    }

    #[test]
    fn test_cli_codegen_js_program_module_writes_single_host_neutral_javascript_module() {
        let (dir, path) = create_temp_nx_file(r#"let root() = { <div class="test" /> }"#);
        let output_path = dir.path().join("codegen-program-module-js");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "javascript",
            "--format",
            "program-module",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write program-module JavaScript output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let module = fs::read_to_string(output_path.join("program.js")).unwrap();

        assert!(module.contains("import { nxElement } from \"nx:runtime\";"));
        assert!(module.contains("export function root()"));
        assert!(module.contains("export const nxProgramModuleManifest"));
        assert!(!output_path.join("nx-runtime.js").exists());
        assert!(!output_path.join("index.js").exists());
        assert!(!output_path.join("m0_test.js").exists());
    }

    #[test]
    fn test_cli_codegen_js_program_module_rejects_typescript_target() {
        let (dir, path) = create_temp_nx_file("let root() = { 1 + 2 }");
        let output_path = dir.path().join("codegen-program-module-ts");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "typescript",
            "--format",
            "program-module",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Program-module codegen is only supported with --target javascript")
        );
    }

    #[test]
    fn test_cli_codegen_source_file_writes_single_nx_ir_artifact() {
        let (dir, path) = create_temp_nx_file("let root() = { 1 + 2 }");
        let output_path = dir.path().join("codegen-nx-ir");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write NX IR output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let ir_path = output_path.join("test.nxir.json");
        let document: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(ir_path).unwrap()).unwrap();

        assert_eq!(document["format"], "nx-ir-json");
        assert_eq!(document["functionEntrypoints"][0]["name"], "root");
        assert!(!output_path.join("nx-runtime.js").exists());
        assert!(!output_path.join("index.js").exists());
        assert!(!output_path.join("m0_test.js").exists());
    }

    #[test]
    fn test_cli_codegen_workspace_nx_ir_uses_selected_entry() {
        let (dir, workspace_path) = create_temp_library(&[
            ("main.nx", "let root() = { 1 }"),
            ("other.nx", "let root() = { 2 }"),
        ]);
        let output_path = dir.path().join("codegen-workspace-nx-ir");

        let output = run_cli(&[
            "codegen",
            workspace_path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--entry",
            "other.nx",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write workspace NX IR output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let document: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output_path.join("other.nxir.json")).unwrap())
                .unwrap();

        assert_eq!(document["functionEntrypoints"][0]["name"], "root");
        assert!(document["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["identity"] == "other.nx"));
    }

    #[test]
    fn test_cli_codegen_nx_ir_static_diagnostics_prevent_output() {
        let (dir, path) = create_temp_nx_file("let root(): int = { \"oops\" }");
        let output_path = dir.path().join("codegen-invalid-nx-ir");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(!output.status.success());
        assert!(!output_path.join("test.nxir.json").exists());
    }

    #[test]
    fn test_cli_codegen_nx_ir_rejects_output_format_override() {
        let (dir, path) = create_temp_nx_file("let root() = { 1 + 2 }");
        let output_path = dir.path().join("codegen-nx-ir-format");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--format",
            "program-module",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("NX IR codegen does not use --format"));
        assert!(!output_path.join("test.nxir.json").exists());
    }

    #[test]
    fn test_cli_codegen_nx_ir_is_not_source_output_format() {
        let (dir, path) = create_temp_nx_file("let root() = { 1 + 2 }");
        let output_path = dir.path().join("codegen-old-nx-ir-format");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "javascript",
            "--format",
            "nx-ir",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("invalid value 'nx-ir'"));
        assert!(!output_path.join("test.nxir.json").exists());
    }

    #[test]
    fn test_cli_codegen_workspace_uses_entry_and_resolved_imports() {
        let (dir, workspace_path) = create_temp_library(&[
            (
                "main.nx",
                r#"import { answer } from "./value.nx"
let root(): int = { answer }"#,
            ),
            ("value.nx", "export let answer: int = 42"),
        ]);
        let output_path = dir.path().join("codegen-workspace-js");

        let output = run_cli(&[
            "codegen",
            workspace_path.to_str().unwrap(),
            "--target",
            "javascript",
            "--entry",
            "main.nx",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write workspace executable output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let module = fs::read_to_string(output_path.join("m0_main.js")).unwrap();

        assert!(module.contains("import { answer as m1_answer } from \"./m1_value.js\";"));
        assert!(module.contains("return m1_answer;"));
    }

    #[test]
    fn test_cli_codegen_file_writes_component_capable_javascript_output() {
        let (dir, path) = create_temp_nx_file(
            r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox /> }
"#,
        );
        let output_path = dir.path().join("codegen-component-js");

        let output = run_cli(&[
            "codegen",
            path.to_str().unwrap(),
            "--target",
            "javascript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write component-capable JavaScript output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let module = fs::read_to_string(output_path.join("m0_test.js")).unwrap();
        let index = fs::read_to_string(output_path.join("index.js")).unwrap();

        assert!(module.contains("export function SearchBox("));
        assert!(module
            .contains("return { $type: \"SearchBox\", placeholder: resolvedProps.placeholder };"));
        assert!(module.contains("export function initialSearchBoxState("));
        assert!(module.contains("export function renderSearchBox("));
        assert!(module.contains("export const SearchBoxSchema"));
        assert!(!module.contains("export class SearchBox"));
        assert!(!module.contains("static initialize("));
        assert!(index.contains(
            "export { SearchBox, SearchBoxSchema, initialSearchBoxState, renderSearchBox } from \"./m0_test.js\";"
        ));
    }

    #[test]
    fn test_cli_codegen_workspace_writes_component_capable_typescript_output() {
        let (dir, workspace_path) = create_temp_library(&[
            (
                "main.nx",
                r#"import { Question } from "./ui.nx"
external component <ShortTextQuestion extends Question placeholder:string? />
let root() = { <ShortTextQuestion /> }"#,
            ),
            (
                "ui.nx",
                r#"export abstract external component <Question label:string = "Untitled" />"#,
            ),
        ]);
        let output_path = dir.path().join("codegen-component-ts");

        let output = run_cli(&[
            "codegen",
            workspace_path.to_str().unwrap(),
            "--target",
            "typescript",
            "--entry",
            "main.nx",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write component-capable TypeScript output: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let module = fs::read_to_string(output_path.join("m0_main.ts")).unwrap();
        let index = fs::read_to_string(output_path.join("index.ts")).unwrap();

        assert!(!module.contains("Question as m1_Question"));
        assert!(module.contains("export type ShortTextQuestionProps"));
        assert!(module.contains("export type ShortTextQuestionElement"));
        assert!(module.contains("export function ShortTextQuestion("));
        assert!(module.contains("export const ShortTextQuestionSchema"));
        assert!(!module.contains("export class ShortTextQuestion"));
        assert!(index.contains(
            "export { ShortTextQuestion, ShortTextQuestionSchema } from \"./m0_main.js\";"
        ));
    }

    #[test]
    fn test_cli_typegen_file_emits_external_component_state_contract() {
        let source = r#"
            export external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
            path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            output.status.success(),
            "CLI should generate external component state contract for .nx file input"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("export interface SearchBox_state"));
        assert!(stdout.contains("query: string;"));
        let state_block = stdout
            .split("export interface SearchBox_state")
            .nth(1)
            .and_then(|tail| tail.split("}").next())
            .expect("SearchBox_state block");
        assert!(!state_block.contains("$type"));
    }

    #[test]
    fn test_cli_typegen_file_warns_and_skips_conflicting_external_component_state_contract() {
        let source = r#"
            export type SearchBox_state = string
            export external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
            path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            output.status.success(),
            "CLI should succeed when generated external component state conflicts with an explicit export"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(stdout.contains("export type SearchBox_state = string;"));
        assert!(!stdout.contains("export interface SearchBox_state"));
        assert!(stderr.contains("Warning:"));
        assert!(stderr.contains("SearchBox_state"));
    }

    #[test]
    fn test_cli_typegen_file_preserves_composed_typescript_list_suffixes() {
        let source = r#"
            export type Matrix = string[][]
            export type MaybeNames = string[]?
            export type Payload = {
              aliases:string?[]
              maybeNames:string[]?
              matrix:string[][]
            }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
            path.to_str().unwrap(),
            "--language",
            "typescript",
        ]);

        assert!(
            output.status.success(),
            "CLI should generate composed TypeScript list suffixes for .nx file input"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("export type Matrix = string[][];"));
        assert!(stdout.contains("export type MaybeNames = string[] | null;"));
        assert!(stdout.contains("aliases: (string | null)[];"));
        assert!(stdout.contains("maybeNames: string[] | null;"));
        assert!(stdout.contains("matrix: string[][];"));
    }

    #[test]
    fn test_cli_typegen_file_preserves_composed_csharp_list_suffixes() {
        let source = r#"
            export type Payload = {
              matrix:string[][]
              maybeNames:string[]?
              aliases:string?[]
            }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
            path.to_str().unwrap(),
            "--language",
            "csharp",
            "--csharp-namespace",
            "MyApp.Models",
        ]);

        assert!(
            output.status.success(),
            "CLI should generate composed C# list suffixes for .nx file input"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("public string[][] Matrix { get; set; } = default!;"));
        assert!(stdout.contains("public string[]? MaybeNames { get; set; }"));
        assert!(stdout.contains("public string?[] Aliases { get; set; } = default!;"));
    }

    #[test]
    fn test_cli_typegen_file_warns_for_csharp_abstract_root_without_concrete_descendants() {
        let source = r#"
            export abstract type Question = { label:string }
        "#;
        let (_dir, path) = create_temp_nx_file(source);

        let output = run_cli(&[
            "typegen",
            path.to_str().unwrap(),
            "--language",
            "csharp",
            "--csharp-namespace",
            "MyApp.Models",
        ]);

        assert!(
            output.status.success(),
            "CLI should still generate C# output for abstract roots without concrete descendants"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(stdout.contains("public abstract class Question"));
        assert!(!stdout.contains("[JsonPolymorphic("));
        assert!(!stdout.contains("[JsonDerivedType("));
        assert!(stdout.contains(
            "// No polymorphism metadata (JSON or MessagePack) was generated because this abstract type had"
        ));
        assert!(stdout.contains("// no concrete exported descendants at code-generation time."));
        assert!(stderr.contains("Warning:"));
        assert!(stderr.contains("Question"));
        assert!(stderr.contains("no concrete exported descendants"));
    }

    #[test]
    fn test_cli_typegen_rejects_non_nx_files() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("README.md");
        fs::write(&file_path, "# Not NX").unwrap();

        let output = run_cli(&[
            "typegen",
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
    fn test_cli_typegen_library_requires_output_directory() {
        let (_dir, library_path) =
            create_temp_library(&[("theme.nx", "export type ThemeMode = light | dark")]);

        let output = run_cli(&[
            "typegen",
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
    fn test_cli_typegen_rejects_empty_library_directory() {
        let dir = TempDir::new().unwrap();
        let library_path = dir.path().join("empty-library");
        let output_path = dir.path().join("generated");
        fs::create_dir_all(&library_path).unwrap();

        let output = run_cli(&[
            "typegen",
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
    fn test_cli_typegen_surfaces_library_diagnostics() {
        let (_dir, library_path) =
            create_temp_library(&[("broken.nx", r#"export let answer(): int = { "oops" }"#)]);
        let output_path = library_path.parent().unwrap().join("generated");

        let output = run_cli(&[
            "typegen",
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
    fn test_cli_typegen_library_writes_typescript_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export type ThemeMode = light | dark"),
            (
                "forms.nx",
                "export type FormState = { theme: ThemeMode }\nexport type FormTheme = ThemeMode",
            ),
        ]);
        let output_path = dir.path().join("generated-ts");

        let output = run_cli(&[
            "typegen",
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
        assert!(forms.contains("export interface FormState extends NxRecord<\"FormState\">"));
        assert!(forms.contains("export type FormTheme = ThemeMode;"));
        assert!(theme.contains("export type ThemeMode = \"light\" | \"dark\";"));
        assert!(index.contains("export * from \"./forms\";"));
        assert!(index.contains("export * from \"./theme\";"));
    }

    #[test]
    fn test_cli_typegen_library_writes_typescript_dependency_package_import() {
        let dir = TempDir::new().unwrap();
        let question_flow_path = dir.path().join("question-flow");
        let chat_link_path = dir.path().join("chat-link");
        fs::create_dir_all(&question_flow_path).unwrap();
        fs::create_dir_all(&chat_link_path).unwrap();
        fs::write(
            question_flow_path.join("QuestionFlow.nx"),
            "export type QuestionFlow = { id:string }",
        )
        .unwrap();
        fs::write(
            chat_link_path.join("QuestionFlowInitialExperience.nx"),
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
        )
        .unwrap();
        let output_path = dir.path().join("generated-ts");

        let output = run_cli(&[
            "typegen",
            chat_link_path.to_str().unwrap(),
            "--language",
            "typescript",
            "--typescript-package-prefix",
            "@org/nx-",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write TypeScript library output with dependency imports"
        );
        let generated =
            fs::read_to_string(output_path.join("QuestionFlowInitialExperience.ts")).unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(generated.contains("import type { QuestionFlow } from \"@org/nx-question-flow\";"));
        assert!(generated.contains("questionFlow: QuestionFlow;"));
        assert!(stderr.contains("Warning:"));
        assert!(stderr.contains("@org/nx-question-flow"));
    }

    #[test]
    fn test_cli_typegen_library_writes_external_component_state_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export type ThemeMode = light | dark"),
            (
                "search-box.nx",
                r#"export external component <SearchBox /> = {
  state { theme:ThemeMode }
}"#,
            ),
        ]);
        let output_path = dir.path().join("generated-ts");

        let output = run_cli(&[
            "typegen",
            library_path.to_str().unwrap(),
            "--language",
            "typescript",
            "--output",
            output_path.to_str().unwrap(),
        ]);

        assert!(
            output.status.success(),
            "CLI should write TypeScript external component state output"
        );
        let search_box = fs::read_to_string(output_path.join("search-box.ts")).unwrap();
        let index = fs::read_to_string(output_path.join("index.ts")).unwrap();

        assert!(search_box.contains("import type { ThemeMode } from \"./theme\";"));
        assert!(search_box.contains("export interface SearchBox_state"));
        assert!(search_box.contains("theme: ThemeMode;"));
        assert!(!search_box.contains("$type"));
        assert!(index.contains("export * from \"./search-box\";"));
    }

    #[test]
    fn test_cli_typegen_library_writes_csharp_external_component_state_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export type ThemeMode = light | dark"),
            (
                "search-box.nx",
                r#"export external component <SearchBox /> = {
  state { theme:ThemeMode }
}"#,
            ),
        ]);
        let output_path = dir.path().join("generated-cs");

        let output = run_cli(&[
            "typegen",
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
            "CLI should write C# external component state output"
        );
        let search_box = fs::read_to_string(output_path.join("search-box.g.cs")).unwrap();
        let theme = fs::read_to_string(output_path.join("theme.g.cs")).unwrap();

        assert!(search_box.contains("namespace MyApp.Models"));
        assert!(search_box.contains("public sealed class SearchBox_state"));
        assert!(search_box.contains("public ThemeMode Theme { get; set; }"));
        let state_block = search_box
            .split("public sealed class SearchBox_state")
            .nth(1)
            .and_then(|tail| tail.split("}").next())
            .expect("SearchBox_state block");
        assert!(!state_block.contains("__NxType"));
        assert!(theme.contains("namespace MyApp.Models"));
        assert!(theme.contains("public enum ThemeMode"));
    }

    #[test]
    fn test_cli_typegen_library_writes_csharp_output() {
        let (dir, library_path) = create_temp_library(&[
            ("theme.nx", "export type ThemeMode = light | dark"),
            ("forms.nx", "export type FormState = { theme: ThemeMode }"),
        ]);
        let output_path = dir.path().join("generated-cs");

        let output = run_cli(&[
            "typegen",
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
        assert!(!forms.contains("__NxType"));
        assert!(!forms.contains("[Key(\"$type\")]"));
        assert!(!forms.contains("[JsonPropertyName(\"$type\")]"));
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

    /// A contextual literal and the qualified form must be indistinguishable after type checking.
    #[test]
    fn test_cli_contextual_literal_matches_qualified_form() {
        let bare = r#"
            type Fit = fill | contain | cover
            type LoadState = idle | loading
            type Box = { fit: Fit  state: LoadState }
            let root() = { <Box fit=cover state=loading /> }
        "#;
        let qualified = r#"
            type Fit = fill | contain | cover
            type LoadState = idle | loading
            type Box = { fit: Fit  state: LoadState }
            let root() = { <Box fit={Fit.cover} state={LoadState.loading} /> }
        "#;

        let (_bare_dir, bare_path) = create_temp_nx_file(bare);
        let (_qual_dir, qual_path) = create_temp_nx_file(qualified);

        let bare_out = run_cli(&["run", bare_path.to_str().unwrap()]);
        let qual_out = run_cli(&["run", qual_path.to_str().unwrap()]);

        assert!(bare_out.status.success(), "bare form should run");
        assert!(qual_out.status.success(), "qualified form should run");
        assert_eq!(
            String::from_utf8_lossy(&bare_out.stdout),
            String::from_utf8_lossy(&qual_out.stdout),
            "the source spelling must not be observable downstream of type checking"
        );
    }

    /// The same equivalence must hold in the generated IR, not only in interpreted output.
    #[test]
    fn test_cli_contextual_literal_codegen_matches_qualified_form() {
        let bare = r#"
            type Fit = fill | contain | cover
            type Box = { fit: Fit }
            let root() = { <Box fit=cover /> }
        "#;
        let qualified = r#"
            type Fit = fill | contain | cover
            type Box = { fit: Fit }
            let root() = { <Box fit={Fit.cover} /> }
        "#;

        let (_bare_dir, bare_path) = create_temp_nx_file(bare);
        let (_qual_dir, qual_path) = create_temp_nx_file(qualified);

        let bare_dest = _bare_dir.path().join("bare-out");
        let qual_dest = _qual_dir.path().join("qual-out");

        let bare_out = run_cli(&[
            "codegen",
            bare_path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--output",
            bare_dest.to_str().unwrap(),
        ]);
        let qual_out = run_cli(&[
            "codegen",
            qual_path.to_str().unwrap(),
            "--target",
            "nx-ir",
            "--output",
            qual_dest.to_str().unwrap(),
        ]);

        assert!(bare_out.status.success(), "bare form should generate");
        assert!(qual_out.status.success(), "qualified form should generate");

        // The IR embeds spans, node ids, and the source text, all of which legitimately differ
        // between two different spellings. Everything else must match.
        fn strip_volatile(value: &mut serde_json::Value) {
            const VOLATILE: &[&str] = &[
                "start",
                "end",
                "id",
                "slot",
                "programFingerprint",
                "source",
                "identity",
            ];
            match value {
                serde_json::Value::Object(map) => {
                    map.retain(|key, _| !VOLATILE.contains(&key.as_str()));
                    for entry in map.values_mut() {
                        strip_volatile(entry);
                    }
                }
                serde_json::Value::Array(items) => {
                    for item in items {
                        strip_volatile(item);
                    }
                }
                _ => {}
            }
        }

        let read_ir = |dir: &std::path::Path| -> serde_json::Value {
            let path = std::fs::read_dir(dir)
                .expect("generated output directory")
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .find(|path| path.extension().is_some_and(|ext| ext == "json"))
                .expect("generated IR file");
            let mut value: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).expect("read IR"))
                    .expect("parse IR");
            strip_volatile(&mut value);
            value
        };

        assert_eq!(
            read_ir(&bare_dest),
            read_ir(&qual_dest),
            "generated IR must not depend on the source spelling"
        );
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
        // Numbers are emitted unquoted so the output reads back at an int-typed site.
        assert!(stdout.contains("age=30"));
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

        // The root executes through the resolved-program runtime — JSON output proves it.
        let json = run_cli(&["run", path.to_str().unwrap(), "--format", "json"]);
        assert!(
            json.status.success(),
            "CLI should execute handler-producing roots"
        );
        let json_stdout = String::from_utf8_lossy(&json.stdout);
        assert!(json_stdout.contains("SearchBox"), "got: {json_stdout}");
        assert!(
            json_stdout.contains("SearchSubmitted"),
            "got: {json_stdout}"
        );

        // Rendering it as NX source does not: an action handler has no source spelling, and the
        // old output put the property name in element-tag position, which never read back.
        let nx = run_cli(&["run", path.to_str().unwrap()]);
        assert!(
            !nx.status.success(),
            "a value with no NX spelling must not be printed as if it had one"
        );
        let stderr = String::from_utf8_lossy(&nx.stderr);
        assert!(stderr.contains("action handler"), "got: {stderr}");
    }

    #[test]
    fn test_cli_help() {
        let output = run_cli(&["--help"]);

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("NX Language CLI"));
        assert!(stdout.contains("run"));
        assert!(stdout.contains("typegen"));
        assert!(stdout.contains("codegen"));
        assert!(!stdout.contains("  types"));
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

    // --- Value round-tripping (OpenSpec change `replace-enums-with-unions`, task group 1) ---

    /// Builds a program that imports one sibling library, reporting analysis errors instead of
    /// panicking on them, so a round-trip that fails to read back can be asserted on.
    fn build_program_reporting_errors(path: &Path) -> Result<ProgramArtifact, String> {
        let source = fs::read_to_string(path).expect("source file should load");
        let file_name = path.display().to_string();
        let module = lower_source_module(&source, &file_name)
            .map_err(|diagnostics| format!("{diagnostics:?}"))?;
        let registry = LibraryRegistry::new();
        for import in &module.imports {
            let parent = path.parent().expect("main file has a parent");
            registry
                .load_library_from_directory(&parent.join(&import.library_path))
                .map_err(|diagnostics| format!("{diagnostics:?}"))?;
        }
        let artifact =
            build_program_artifact_from_source(&source, &file_name, &registry.build_context())
                .map_err(|diagnostics| format!("{diagnostics:?}"))?;
        let errors = artifact
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .map(|diagnostic| {
                format!(
                    "{}: {}",
                    diagnostic.code().unwrap_or("?"),
                    diagnostic.message()
                )
            })
            .collect::<Vec<_>>();
        if errors.is_empty() {
            Ok(artifact)
        } else {
            Err(errors.join("; "))
        }
    }

    struct RoundTrip {
        value: Value,
        formatted: String,
        read_back: Result<Value, String>,
    }

    /// Evaluates `root`, formats the value it produces, then reads that formatting back at the
    /// same typed site. A value with a correct NX spelling round-trips: `read_back` succeeds and
    /// equals `value`.
    fn round_trip_root(library: &[(&str, &str)], preamble: &str, root_body: &str) -> RoundTrip {
        let dir = TempDir::new().expect("temp dir");
        let app_dir = dir.path().join("app");
        let library_dir = dir.path().join("lib");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&library_dir).expect("library dir");
        for (name, content) in library {
            fs::write(library_dir.join(name), content).expect("library file");
        }

        let main = app_dir.join("main.nx");
        let evaluate = |source: &str| -> Result<Value, String> {
            fs::write(&main, source).expect("main file");
            let program = build_program_reporting_errors(&main)?;
            Interpreter::from_resolved_program(program.resolved_program.clone())
                .execute_resolved_program_function("root", vec![])
                .map_err(|error| format!("{error:?}"))
        };

        let value = evaluate(&format!("{preamble}\nlet root() = {root_body}\n"))
            .expect("the original program should analyze and evaluate");
        let formatted = format::format_value(&value).expect("the value should have an NX spelling");
        let read_back = evaluate(&format!("{preamble}\nlet root() = {formatted}\n"));

        RoundTrip {
            value,
            formatted,
            read_back,
        }
    }

    /// Evaluates `root` in a single-file program and returns the value it produces.
    fn evaluate_root(source: &str) -> Value {
        let (_dir, path) = create_temp_nx_file(source);
        let program =
            build_program_reporting_errors(&path).expect("the program should analyze cleanly");
        Interpreter::from_resolved_program(program.resolved_program.clone())
            .execute_resolved_program_function("root", vec![])
            .expect("root should evaluate")
    }

    /// A fieldless case of a base-less union is a constant case, and carries no more information
    /// than its name — so it serializes as a bare string, exactly as an enum member does, rather
    /// than as a `$type` map.
    #[test]
    fn constant_union_case_serializes_as_a_bare_string() {
        let value = evaluate_root(
            r#"type Fit = fill | contain | cover
type Shape =
  | point
  | circle { radius: float64 }
type Box = { fit: Fit  shape: Shape }
let root() = <Box fit=cover shape={<Shape.point />} />
"#,
        );
        let nx_value = nx_api::to_nx_value(&value);
        let json: serde_json::Value =
            serde_json::from_str(&nx_value.to_json_string_pretty().expect("json")).expect("parse");

        assert_eq!(
            json["shape"],
            serde_json::Value::String("point".to_string()),
            "a constant case must serialize like an enum member ({}), got {}",
            json["fit"],
            json["shape"]
        );

        let bytes = nx_value.to_msgpack_vec().expect("messagepack");
        let decoded =
            nx_value::NxValue::from_msgpack_slice(&bytes).expect("messagepack round-trip");
        let nx_value::NxValue::Record { properties, .. } = &decoded else {
            panic!("expected a record, got {decoded:?}");
        };
        assert_eq!(
            properties["shape"],
            nx_value::NxValue::String("point".to_string()),
            "MessagePack must carry the same bare string, got {:?}",
            properties["shape"]
        );
    }

    /// A record-valued property must keep its name. Today the value is emitted as element body
    /// content, which discards the property name. This is RF5 in `contextual-literal-binding`'s
    /// `review.md`.
    #[test]
    fn record_valued_property_round_trips() {
        let trip = round_trip_root(
            &[],
            "type Address = { city: string }\nexternal component <div home:Address />",
            "<div home={<Address city=\"Boston\" />} />",
        );

        assert_eq!(
            trip.read_back.as_ref().ok(),
            Some(&trip.value),
            "a record-valued property must read back as itself, but formatted as `{}` \
             and read back as {:?}",
            trip.formatted.trim(),
            trip.read_back
        );
    }

    /// Two properties of the same record type must stay distinguishable. Emitting both as body
    /// content renders them as identical siblings, so which field each belongs to is lost.
    #[test]
    fn two_properties_of_the_same_record_type_stay_distinguishable() {
        let trip = round_trip_root(
            &[],
            "type Address = { city: string }\nexternal component <div home:Address work:Address />",
            "<div home={<Address city=\"Boston\" />} work={<Address city=\"Denver\" />} />",
        );

        assert!(
            trip.formatted.contains("home=") && trip.formatted.contains("work="),
            "both property names must survive formatting, got `{}`",
            trip.formatted.trim()
        );
        assert_eq!(
            trip.read_back.as_ref().ok(),
            Some(&trip.value),
            "read back as {:?}",
            trip.read_back
        );
    }

    /// A list-valued property must be emitted as a braced sequence, not as an element named after
    /// the property — NX has no property-element syntax, so `<items>` does not read back.
    #[test]
    fn list_valued_property_round_trips() {
        let trip = round_trip_root(
            &[],
            "type Item = { n: int }\nexternal component <div items:Item[] />",
            "<div items={<Item n=1 /> <Item n=2 />} />",
        );

        assert!(
            !trip.formatted.contains("<items>"),
            "the property name must not become an element tag, got `{}`",
            trip.formatted.trim()
        );
        assert_eq!(
            trip.read_back.as_ref().ok(),
            Some(&trip.value),
            "a list-valued property must read back as itself, but formatted as `{}` \
             and read back as {:?}",
            trip.formatted.trim(),
            trip.read_back
        );
    }
}
