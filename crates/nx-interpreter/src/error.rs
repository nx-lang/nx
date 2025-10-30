//! Runtime error types and handling for the NX interpreter.

use ariadne::{Color, Label, Report, ReportKind, Source};
use smol_str::SmolStr;
use std::fmt;
use text_size::TextRange;

/// Runtime error kinds that can occur during interpretation
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrorKind {
    /// Division by zero
    DivisionByZero,
    /// Operation on null value
    NullOperation { operation: String },
    /// Type mismatch in operation
    TypeMismatch {
        expected: String,
        actual: String,
        operation: String,
    },
    /// Undefined variable reference
    UndefinedVariable { name: SmolStr },
    /// Parameter count mismatch in function call
    ParameterCountMismatch {
        expected: usize,
        actual: usize,
        function: SmolStr,
    },
    /// Function not found
    FunctionNotFound { name: SmolStr },
    /// Operation limit exceeded (infinite loop protection)
    OperationLimitExceeded { limit: usize },
    /// Stack overflow (recursion depth exceeded)
    StackOverflow { depth: usize },
}

impl fmt::Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeErrorKind::DivisionByZero => write!(f, "Division by zero"),
            RuntimeErrorKind::NullOperation { operation } => {
                write!(f, "Cannot perform {} on null value", operation)
            }
            RuntimeErrorKind::TypeMismatch {
                expected,
                actual,
                operation,
            } => write!(
                f,
                "Type mismatch in {}: expected {}, got {}",
                operation, expected, actual
            ),
            RuntimeErrorKind::UndefinedVariable { name } => {
                write!(f, "Undefined variable: {}", name)
            }
            RuntimeErrorKind::ParameterCountMismatch {
                expected,
                actual,
                function,
            } => write!(
                f,
                "Function {} expects {} parameter(s), got {}",
                function, expected, actual
            ),
            RuntimeErrorKind::FunctionNotFound { name } => {
                write!(f, "Function not found: {}", name)
            }
            RuntimeErrorKind::OperationLimitExceeded { limit } => {
                write!(f, "Operation limit exceeded: {} operations", limit)
            }
            RuntimeErrorKind::StackOverflow { depth } => {
                write!(f, "Stack overflow: recursion depth {} exceeded", depth)
            }
        }
    }
}

/// Call stack frame for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct CallFrame {
    /// Function name
    pub function_name: SmolStr,
    /// Source location (optional)
    pub location: Option<TextRange>,
}

impl CallFrame {
    /// Create a new call frame
    pub fn new(function_name: SmolStr, location: Option<TextRange>) -> Self {
        Self {
            function_name,
            location,
        }
    }
}

/// Runtime error with context and source location
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// Error kind
    kind: RuntimeErrorKind,
    /// Source location where the error occurred
    location: Option<TextRange>,
    /// Call stack at the time of error
    call_stack: Vec<CallFrame>,
}

impl RuntimeError {
    /// Create a new runtime error
    pub fn new(kind: RuntimeErrorKind) -> Self {
        Self {
            kind,
            location: None,
            call_stack: Vec::new(),
        }
    }

    /// Set the source location of the error
    pub fn with_location(mut self, location: TextRange) -> Self {
        self.location = Some(location);
        self
    }

    /// Set the call stack
    pub fn with_call_stack(mut self, call_stack: Vec<CallFrame>) -> Self {
        self.call_stack = call_stack;
        self
    }

    /// Get the error kind
    pub fn kind(&self) -> &RuntimeErrorKind {
        &self.kind
    }

    /// Get the source location
    pub fn location(&self) -> Option<TextRange> {
        self.location
    }

    /// Get the call stack
    pub fn call_stack(&self) -> &[CallFrame] {
        &self.call_stack
    }

    /// Format the error with Ariadne for beautiful error output
    pub fn format(&self, filename: &str, source: &str) -> String {
        let mut output = Vec::new();

        if let Some(location) = self.location {
            let report = Report::build(ReportKind::Error, filename, location.start().into())
                .with_message(format!("Runtime Error: {}", self.kind))
                .with_label(
                    Label::new((filename, location.start().into()..location.end().into()))
                        .with_message(&self.kind.to_string())
                        .with_color(Color::Red),
                );

            let report = if !self.call_stack.is_empty() {
                let mut note = String::from("Call stack:\n");
                for (i, frame) in self.call_stack.iter().enumerate() {
                    note.push_str(&format!("  {}: {}\n", i + 1, frame.function_name));
                }
                report.with_note(note)
            } else {
                report
            };

            let _ = report.finish().write((filename, Source::from(source)), &mut output);
        } else {
            // No source location, format as simple message
            output.extend_from_slice(format!("Runtime Error: {}", self.kind).as_bytes());
        }

        String::from_utf8_lossy(&output).to_string()
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = RuntimeError::new(RuntimeErrorKind::DivisionByZero);
        assert_eq!(error.kind(), &RuntimeErrorKind::DivisionByZero);
        assert!(error.location().is_none());
        assert!(error.call_stack().is_empty());
    }

    #[test]
    fn test_error_with_location() {
        let error = RuntimeError::new(RuntimeErrorKind::DivisionByZero)
            .with_location(TextRange::new(0.into(), 5.into()));
        assert!(error.location().is_some());
    }

    #[test]
    fn test_error_kinds() {
        let err1 = RuntimeErrorKind::DivisionByZero;
        assert!(err1.to_string().contains("Division by zero"));

        let err2 = RuntimeErrorKind::UndefinedVariable {
            name: SmolStr::new("x"),
        };
        assert!(err2.to_string().contains("Undefined variable"));
    }
}
