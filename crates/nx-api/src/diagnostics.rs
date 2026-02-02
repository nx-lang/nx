use nx_diagnostics::{Diagnostic, Severity};
use serde::{Deserialize, Serialize};
use text_size::TextRange;

/// The severity level of a diagnostic message.
///
/// Serializes to lowercase strings: `"error"`, `"warning"`, `"info"`, `"hint"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NxSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<Severity> for NxSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
            Severity::Info => Self::Info,
            Severity::Hint => Self::Hint,
        }
    }
}

/// A half-open span of text in a source file.
///
/// Byte offsets form a half-open range `[start_byte, end_byte)`.
/// Line and column numbers are 1-based; columns count Unicode scalar values, not bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NxTextSpan {
    /// Byte offset of the first byte in the span (inclusive).
    pub start_byte: u32,
    /// Byte offset one past the last byte in the span (exclusive).
    pub end_byte: u32,

    /// 1-based line number of the span start.
    pub start_line: u32,
    /// 1-based column of the span start, counted in Unicode scalar values.
    pub start_column: u32,

    /// 1-based line number of the span end.
    pub end_line: u32,
    /// 1-based column of the span end, counted in Unicode scalar values.
    pub end_column: u32,
}

/// A label pointing to a specific source location related to a diagnostic.
///
/// A diagnostic may carry multiple labels. The primary label marks the main site of the issue;
/// secondary labels provide additional context (e.g. "this was expected because...").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NxDiagnosticLabel {
    /// The file name originally passed to [`eval_source`](crate::eval_source).
    pub file: String,
    /// The text span indicating the location in the source.
    pub span: NxTextSpan,
    /// An optional message specific to this label location.
    pub message: Option<String>,
    /// `true` for the primary label (the main site of the issue), `false` for secondary context.
    pub primary: bool,
}

/// A diagnostic message from the NX language runtime.
///
/// This is a stable, serde-friendly representation suitable for serialization over FFI
/// boundaries (MessagePack, JSON). Every diagnostic has at least a [`severity`](Self::severity)
/// and [`message`](Self::message); all other fields are optional or may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NxDiagnostic {
    /// The severity level of the diagnostic.
    pub severity: NxSeverity,
    /// An optional diagnostic code identifying the specific type of issue (e.g. `"no-root"`,
    /// `"runtime-error"`).
    pub code: Option<String>,
    /// The main human-readable diagnostic message.
    pub message: String,
    /// Labels pointing to specific locations in the source code related to this diagnostic.
    /// May be empty for diagnostics that have no meaningful source location (e.g. runtime errors).
    pub labels: Vec<NxDiagnosticLabel>,
    /// An optional help message with suggestions for resolving the issue.
    pub help: Option<String>,
    /// An optional note providing additional context.
    pub note: Option<String>,
}

/// Converts internal [`Diagnostic`] values into the stable [`NxDiagnostic`] representation,
/// resolving byte offsets to 1-based line/column positions.
///
/// `source` must be the same source text that was parsed to produce `diagnostics`; otherwise
/// the computed line/column positions will be incorrect.
pub fn diagnostics_to_api(diagnostics: &[Diagnostic], source: &str) -> Vec<NxDiagnostic> {
    let index = LineIndex::new(source);
    diagnostics
        .iter()
        .map(|d| diagnostic_to_api(d, source, &index))
        .collect()
}

fn diagnostic_to_api(diagnostic: &Diagnostic, source: &str, index: &LineIndex) -> NxDiagnostic {
    let mut labels = Vec::with_capacity(diagnostic.labels().len());
    for label in diagnostic.labels() {
        labels.push(NxDiagnosticLabel {
            file: label.file.clone(),
            span: text_range_to_span(label.range, source, index),
            message: label.message.clone(),
            primary: label.primary,
        });
    }

    NxDiagnostic {
        severity: diagnostic.severity().into(),
        code: diagnostic.code().map(ToString::to_string),
        message: diagnostic.message().to_string(),
        labels,
        help: diagnostic.help().map(ToString::to_string),
        note: diagnostic.note().map(ToString::to_string),
    }
}

fn text_range_to_span(range: TextRange, source: &str, index: &LineIndex) -> NxTextSpan {
    let start: usize = range.start().into();
    let end: usize = range.end().into();
    let (start_line, start_col) = index.byte_offset_to_line_col(source, start);
    let (end_line, end_col) = index.byte_offset_to_line_col(source, end);

    NxTextSpan {
        start_byte: start as u32,
        end_byte: end as u32,
        start_line,
        start_column: start_col,
        end_line,
        end_column: end_col,
    }
}

struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(idx + 1);
            }
        }

        Self { line_starts }
    }

    fn byte_offset_to_line_col(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(text.len());

        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert.saturating_sub(1),
        };

        let line_start = self.line_starts[line_idx];
        let slice = &text[line_start..offset];
        let col = slice.chars().count() + 1;

        ((line_idx as u32) + 1, col as u32)
    }
}
