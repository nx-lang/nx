//! Diagnostic error reporting for the NX language.
//!
//! This crate provides beautiful, user-friendly error messages using the Ariadne library.
//! It includes diagnostic types, severity levels, and rendering functionality.

mod diagnostic;
mod render;

pub use diagnostic::{Diagnostic, DiagnosticBuilder, Label, Severity};
pub use render::{render_diagnostic, render_diagnostics, render_diagnostics_cli};

// Re-export text-size types with NX-specific names
pub use text_size::TextRange as TextSpan;
pub use text_size::TextSize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("test error")
            .with_message("This is a test error message")
            .build();

        assert_eq!(diag.severity(), Severity::Error);
        assert_eq!(diag.code(), Some("test error"));
    }
}
