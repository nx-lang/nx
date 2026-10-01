//! Diagnostics: what every API of this crate returns in place of a value when it fails.

use std::fmt;

/// A byte span in the source text of the module `identity` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub identity: String,
    pub start: u32,
    pub end: u32,
}

/// One thing that went wrong.
///
/// <para>`code` is one of the `nx-ir-*` strings the TypeScript runtime reports for the same
/// failure, so a host that handles one runtime's diagnostics handles the other's. The message is
/// for people and its wording is not part of the contract.</para>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    /// The declaration the failing expression belongs to, as `identity::name`, when it is known.
    pub declaration: Option<String>,
    /// The expression's span, when the image carries its debug section.
    pub source: Option<SourceSpan>,
}

impl Diagnostic {
    /// A diagnostic that names no declaration.
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            declaration: None,
            source: None,
        }
    }
}

/// The error of every API of this crate: one or more diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxIrRuntimeError {
    pub diagnostics: Vec<Diagnostic>,
}

impl NxIrRuntimeError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            diagnostics: vec![Diagnostic::new(code, message)],
        }
    }

    /// The code of the first diagnostic, which is the failure that stopped the operation.
    pub fn code(&self) -> &'static str {
        self.diagnostics
            .first()
            .map(|diagnostic| diagnostic.code)
            .unwrap_or("")
    }
}

impl fmt::Display for NxIrRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{}: {}", diagnostic.code, diagnostic.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for NxIrRuntimeError {}

pub type Result<T> = std::result::Result<T, NxIrRuntimeError>;

/// Fails with one diagnostic that names no declaration.
pub(crate) fn fail<T>(code: &'static str, message: impl Into<String>) -> Result<T> {
    Err(NxIrRuntimeError::new(code, message))
}
