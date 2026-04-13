//! Runtime error types and handling for the NX interpreter.

use ariadne::{sources, Color, Label, Report, ReportKind};
use smol_str::SmolStr;
use std::fmt;
use text_size::TextRange;

/// Runtime error kinds that can occur during interpretation
///
/// Represents all possible runtime errors that can be detected during
/// expression evaluation. Each variant includes context-specific information
/// to aid in debugging and error reporting.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrorKind {
    /// Division by zero
    ///
    /// Triggered when attempting to divide or modulo by zero
    DivisionByZero,

    /// Operation on null value
    ///
    /// Triggered when attempting arithmetic or logical operations on null
    NullOperation { operation: String },

    /// Type mismatch in operation
    ///
    /// Triggered when operand types don't match operation requirements
    /// (e.g., adding string to int, comparing incompatible types)
    TypeMismatch {
        expected: String,
        actual: String,
        operation: String,
    },

    /// Undefined variable reference
    ///
    /// Triggered when referencing a variable that doesn't exist in scope
    UndefinedVariable { name: SmolStr },

    /// Parameter count mismatch in function call
    ///
    /// Triggered when function is called with wrong number of arguments
    ParameterCountMismatch {
        expected: usize,
        actual: usize,
        function: SmolStr,
    },

    /// Function not found
    ///
    /// Triggered when attempting to call a non-existent function
    FunctionNotFound { name: SmolStr },

    /// Component not found
    ///
    /// Triggered when attempting to initialize or dispatch a missing component
    ComponentNotFound { name: SmolStr },

    /// Operation limit exceeded (infinite loop protection)
    ///
    /// Triggered when execution exceeds the configured operation count limit
    OperationLimitExceeded { limit: usize },

    /// Stack overflow (recursion depth exceeded)
    ///
    /// Triggered when recursion depth exceeds the configured limit
    StackOverflow { depth: usize },

    /// Enum type referenced at runtime could not be found
    EnumNotFound { name: SmolStr },

    /// Enum member not defined on the referenced enum type
    EnumMemberNotFound { enum_name: SmolStr, member: SmolStr },

    /// Record field not found on the given record value
    RecordFieldNotFound { record: SmolStr, field: SmolStr },

    /// Record type referenced at runtime could not be found
    RecordTypeNotFound { name: SmolStr },

    /// Required record field omitted from an externally supplied typed record
    MissingRequiredRecordField {
        record: SmolStr,
        field: SmolStr,
        operation: String,
    },

    /// Attempted to instantiate an abstract record
    AbstractRecordInstantiation { record: SmolStr, operation: String },

    /// Attempted to instantiate an abstract component
    AbstractComponentInstantiation {
        component: SmolStr,
        operation: String,
    },

    /// Required component prop or state field was not provided and has no default
    MissingRequiredComponentField {
        component: SmolStr,
        field: SmolStr,
        phase: String,
    },

    /// Operation requires a resolved program runtime to provide stable module identity
    ResolvedProgramRequired { operation: String },

    /// Runtime could not locate one lowered module required for evaluation
    ModuleNotFound {
        module_identity: String,
        operation: String,
    },

    /// Malformed or incompatible component state snapshot
    InvalidComponentStateSnapshot { reason: String },

    /// Dispatched action is not declared by the target component
    UnsupportedComponentAction { component: SmolStr, action: SmolStr },
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
            RuntimeErrorKind::ComponentNotFound { name } => {
                write!(f, "Component not found: {}", name)
            }
            RuntimeErrorKind::OperationLimitExceeded { limit } => {
                write!(f, "Operation limit exceeded: {} operations", limit)
            }
            RuntimeErrorKind::StackOverflow { depth } => {
                write!(f, "Stack overflow: recursion depth {} exceeded", depth)
            }
            RuntimeErrorKind::EnumNotFound { name } => {
                write!(f, "Enum not found: {}", name)
            }
            RuntimeErrorKind::EnumMemberNotFound { enum_name, member } => {
                write!(f, "Enum '{}' has no member named '{}'", enum_name, member)
            }
            RuntimeErrorKind::RecordFieldNotFound { record, field } => {
                write!(f, "Record '{}' has no field named '{}'", record, field)
            }
            RuntimeErrorKind::RecordTypeNotFound { name } => {
                write!(f, "Record type not found: {}", name)
            }
            RuntimeErrorKind::MissingRequiredRecordField {
                record,
                field,
                operation,
            } => write!(
                f,
                "Missing required field '{}' on record '{}' in {}",
                field, record, operation
            ),
            RuntimeErrorKind::AbstractRecordInstantiation { record, operation } => write!(
                f,
                "Cannot instantiate abstract record '{}' in {}",
                record, operation
            ),
            RuntimeErrorKind::AbstractComponentInstantiation {
                component,
                operation,
            } => write!(
                f,
                "Cannot instantiate abstract component '{}' in {}",
                component, operation
            ),
            RuntimeErrorKind::MissingRequiredComponentField {
                component,
                field,
                phase,
            } => write!(
                f,
                "Missing required component field '{}' on '{}' during {}",
                field, component, phase
            ),
            RuntimeErrorKind::ResolvedProgramRequired { operation } => {
                write!(f, "Resolved program runtime required for {}", operation)
            }
            RuntimeErrorKind::ModuleNotFound {
                module_identity,
                operation,
            } => write!(
                f,
                "Cannot perform {}: module '{}' is not available",
                operation, module_identity
            ),
            RuntimeErrorKind::InvalidComponentStateSnapshot { reason } => {
                write!(f, "Invalid component state snapshot: {}", reason)
            }
            RuntimeErrorKind::UnsupportedComponentAction { component, action } => write!(
                f,
                "Component '{}' does not declare emitted action '{}'",
                component, action
            ),
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
///
/// Wraps a [`RuntimeErrorKind`] with additional context including
/// source location and call stack for detailed error reporting.
/// Can be formatted with Ariadne for beautiful terminal output.
///
/// # Examples
/// ```ignore
/// use nx_interpreter::{RuntimeError, RuntimeErrorKind};
///
/// let error = RuntimeError::new(RuntimeErrorKind::DivisionByZero);
/// println!("{}", error.format("example.nx", "let x = 1 / 0"));
/// ```
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
    ///
    /// Generates a formatted error message with source context,
    /// highlighting the error location and including call stack if available.
    ///
    /// # Arguments
    /// * `filename` - Name of the source file for display
    /// * `source` - Full source code text for context
    ///
    /// # Returns
    /// Formatted error string with ANSI colors and source highlighting
    pub fn format(&self, filename: &str, source: &str) -> String {
        let mut output = Vec::new();

        if let Some(location) = self.location {
            let file_id = filename.to_string();
            let span = location.start().into()..location.end().into();
            let report = Report::build(ReportKind::Error, (file_id.clone(), span.clone()))
                .with_message(format!("Runtime Error: {}", self.kind))
                .with_label(
                    Label::new((file_id.clone(), span))
                        .with_message(self.kind.to_string())
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

            let cache = sources([(file_id, source.to_string())]);
            let _ = report.finish().write(cache, &mut output);
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

        let err3 = RuntimeErrorKind::RecordTypeNotFound {
            name: SmolStr::new("User"),
        };
        assert!(err3.to_string().contains("Record type not found"));

        let err4 = RuntimeErrorKind::MissingRequiredRecordField {
            record: SmolStr::new("User"),
            field: SmolStr::new("name"),
            operation: "function call parameter 'user'".to_string(),
        };
        assert!(err4.to_string().contains("Missing required field"));
    }
}
