use nx_diagnostics::Diagnostic;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Executable source target emitted by `nx-codegen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    TypeScript,
    JavaScript,
}

impl CodegenTarget {
    pub fn extension(self) -> &'static str {
        match self {
            Self::TypeScript => "ts",
            Self::JavaScript => "js",
        }
    }

    pub fn is_typescript(self) -> bool {
        matches!(self, Self::TypeScript)
    }
}

/// Layout style for generated executable output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenOutputFormat {
    Files,
}

/// Options shared by TypeScript and JavaScript executable generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenOptions {
    pub target: CodegenTarget,
    pub output_format: CodegenOutputFormat,
}

impl CodegenOptions {
    pub fn typescript() -> Self {
        Self {
            target: CodegenTarget::TypeScript,
            output_format: CodegenOutputFormat::Files,
        }
    }

    pub fn javascript() -> Self {
        Self {
            target: CodegenTarget::JavaScript,
            output_format: CodegenOutputFormat::Files,
        }
    }
}

/// One generated output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
}

/// Non-fatal code generation warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenWarning {
    pub message: String,
}

/// Complete generated executable output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenOutput {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<CodegenWarning>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Fatal code generation failure. No executable output should be emitted when this is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    pub diagnostics: Vec<Diagnostic>,
}

impl CodegenError {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn single(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.diagnostics.as_slice() {
            [] => write!(formatter, "code generation failed"),
            [diagnostic] => write!(formatter, "{}", diagnostic.message()),
            diagnostics => write!(
                formatter,
                "code generation failed with {} diagnostics",
                diagnostics.len()
            ),
        }
    }
}

impl Error for CodegenError {}
