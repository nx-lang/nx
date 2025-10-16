//! Core diagnostic types for representing errors, warnings, and information messages.

use text_size::TextRange;

/// Severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A fatal error that prevents compilation.
    Error,
    /// A warning that should be addressed but doesn't prevent compilation.
    Warning,
    /// An informational message.
    Info,
    /// A hint or suggestion for improvement.
    Hint,
}

impl Severity {
    /// Returns the display name for this severity level.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        }
    }
}

/// A label pointing to a specific location in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The source file this label refers to.
    pub file: String,
    /// The text range in the source file.
    pub range: TextRange,
    /// Optional message associated with this label.
    pub message: Option<String>,
    /// Whether this is the primary label for the diagnostic.
    pub primary: bool,
}

impl Label {
    /// Creates a new primary label.
    pub fn primary(file: impl Into<String>, range: TextRange) -> Self {
        Self {
            file: file.into(),
            range,
            message: None,
            primary: true,
        }
    }

    /// Creates a new secondary label.
    pub fn secondary(file: impl Into<String>, range: TextRange) -> Self {
        Self {
            file: file.into(),
            range,
            message: None,
            primary: false,
        }
    }

    /// Adds a message to this label.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// A diagnostic message (error, warning, info, or hint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity level of this diagnostic.
    severity: Severity,
    /// Optional error code (e.g., "E0001" or "type-mismatch").
    code: Option<String>,
    /// Main diagnostic message.
    message: String,
    /// Labels pointing to relevant source locations.
    labels: Vec<Label>,
    /// Optional help text suggesting how to fix the issue.
    help: Option<String>,
    /// Optional note with additional context.
    note: Option<String>,
}

impl Diagnostic {
    /// Creates a builder for an error diagnostic.
    pub fn error(code: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Error, code)
    }

    /// Creates a builder for a warning diagnostic.
    pub fn warning(code: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Warning, code)
    }

    /// Creates a builder for an info diagnostic.
    pub fn info(code: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Info, code)
    }

    /// Creates a builder for a hint diagnostic.
    pub fn hint(code: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Hint, code)
    }

    /// Returns the severity of this diagnostic.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the error code, if any.
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the main message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the labels for this diagnostic.
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Returns the help text, if any.
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// Returns the note, if any.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// Builder for constructing diagnostic messages.
#[derive(Debug)]
pub struct DiagnosticBuilder {
    severity: Severity,
    code: Option<String>,
    message: Option<String>,
    labels: Vec<Label>,
    help: Option<String>,
    note: Option<String>,
}

impl DiagnosticBuilder {
    /// Creates a new diagnostic builder with the given severity and code.
    pub fn new(severity: Severity, code: impl Into<String>) -> Self {
        Self {
            severity,
            code: Some(code.into()),
            message: None,
            labels: Vec::new(),
            help: None,
            note: None,
        }
    }

    /// Sets the main diagnostic message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds a label to this diagnostic.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Adds multiple labels to this diagnostic.
    pub fn with_labels(mut self, labels: impl IntoIterator<Item = Label>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Adds help text to this diagnostic.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Adds a note to this diagnostic.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Builds the diagnostic.
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            code: self.code,
            message: self.message.unwrap_or_default(),
            labels: self.labels,
            help: self.help,
            note: self.note,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_size::{TextRange, TextSize};

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Hint.as_str(), "hint");
    }

    #[test]
    fn test_label_creation() {
        let range = TextRange::new(TextSize::from(0), TextSize::from(5));
        let label = Label::primary("test.nx", range).with_message("test message");

        assert_eq!(label.file, "test.nx");
        assert_eq!(label.range, range);
        assert_eq!(label.message.as_deref(), Some("test message"));
        assert!(label.primary);
    }

    #[test]
    fn test_diagnostic_builder() {
        let range = TextRange::new(TextSize::from(0), TextSize::from(5));
        let label = Label::primary("test.nx", range);

        let diag = Diagnostic::error("E001")
            .with_message("Type mismatch")
            .with_label(label)
            .with_help("Try converting the value to the expected type")
            .with_note("Expected type: string, found: number")
            .build();

        assert_eq!(diag.severity(), Severity::Error);
        assert_eq!(diag.code(), Some("E001"));
        assert_eq!(diag.message(), "Type mismatch");
        assert_eq!(diag.labels().len(), 1);
        assert_eq!(
            diag.help(),
            Some("Try converting the value to the expected type")
        );
        assert_eq!(diag.note(), Some("Expected type: string, found: number"));
    }
}
