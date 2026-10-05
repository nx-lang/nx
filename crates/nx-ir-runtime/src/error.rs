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
    /// The limit that was reached. Every `nx-ir-resource-limit` diagnostic carries one, and no
    /// other does.
    pub limit: Option<Limit>,
    /// The parameter whose argument the failure is in, by its declared name. It is present when
    /// `call_function` or `evaluate_function` refuses a value the host passed for one parameter
    /// (a value that does not fit the parameter's type, at any depth, or a `Function` record in
    /// it that names no function) and when a required parameter was given nothing. Nothing else
    /// carries it: not a failure a default raises, a resource limit, a failure in the function's
    /// body or result, or a failure of another entry point. The TypeScript runtime names the
    /// same argument for the same call.
    pub argument: Option<String>,
}

/// A limit an evaluation reached, as data a host can act on without reading the message.
///
/// <para>A limit the TypeScript runtime also has carries the name of its option there:
/// `maxOperations`, `maxCallDepth`, `maxRangeLength` and `maxExpressionNesting`. The two only this
/// runtime has are `maxStackBytes`, the native stack an evaluation may use, and `maxValueNesting`,
/// how deeply a value may nest at the host boundary or in component state.</para>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit {
    pub name: &'static str,
    pub value: Option<u64>,
}

impl Diagnostic {
    /// A diagnostic that names no declaration.
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            declaration: None,
            source: None,
            limit: None,
            argument: None,
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

/// Fails with one `nx-ir-resource-limit` diagnostic that names no declaration and carries `limit`.
pub(crate) fn fail_limit<T>(limit: Limit, message: impl Into<String>) -> Result<T> {
    let mut diagnostic = Diagnostic::new(RESOURCE_LIMIT, message);
    diagnostic.limit = Some(limit);
    Err(NxIrRuntimeError {
        diagnostics: vec![diagnostic],
    })
}

/// The code of every diagnostic that reports a limit.
pub(crate) const RESOURCE_LIMIT: &str = "nx-ir-resource-limit";
