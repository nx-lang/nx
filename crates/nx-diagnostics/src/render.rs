//! Rendering functionality for displaying diagnostics with beautiful formatting.

use crate::{Diagnostic, Severity};
use ariadne::{Color, Fmt, Label as AriadneLabel, Report, ReportKind, Source};
use std::collections::HashMap;

/// Renders a diagnostic to a string with beautiful formatting.
///
/// # Arguments
///
/// * `diagnostic` - The diagnostic to render
/// * `sources` - Map of file names to their source code
///
/// # Returns
///
/// A formatted string containing the rendered diagnostic with syntax highlighting,
/// source snippets, and helpful annotations.
pub fn render_diagnostic(diagnostic: &Diagnostic, sources: &HashMap<String, String>) -> String {
    let mut output = Vec::new();

    let report_kind = match diagnostic.severity() {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Info => ReportKind::Advice,
        Severity::Hint => ReportKind::Advice,
    };

    let mut report = Report::build(report_kind, (), 0);

    // Set the main message
    report = report.with_message(diagnostic.message());

    // Add labels
    for label in diagnostic.labels() {
        let color = if label.primary {
            Color::Red
        } else {
            Color::Cyan
        };

        let start: usize = label.range.start().into();
        let end: usize = label.range.end().into();

        let ariadne_label = if let Some(msg) = &label.message {
            AriadneLabel::new(start..end)
                .with_message(msg.fg(color))
                .with_color(color)
        } else {
            AriadneLabel::new(start..end).with_color(color)
        };

        report = report.with_label(ariadne_label);
    }

    // Add help text if present
    if let Some(help) = diagnostic.help() {
        report = report.with_help(help);
    }

    // Add note if present
    if let Some(note) = diagnostic.note() {
        report = report.with_note(note);
    }

    let report = report.finish();

    // Find the primary source file
    let primary_file = diagnostic
        .labels()
        .iter()
        .find(|l| l.primary)
        .map(|l| &l.file)
        .or_else(|| diagnostic.labels().first().map(|l| &l.file));

    if let Some(file) = primary_file {
        if let Some(source) = sources.get(file) {
            let _ = report.write(Source::from(source), &mut output);
        } else {
            let _ = report.write(Source::from(""), &mut output);
        }
    } else {
        let _ = report.write(Source::from(""), &mut output);
    }

    String::from_utf8_lossy(&output).to_string()
}

/// Renders multiple diagnostics to a string.
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &HashMap<String, String>) -> String {
    diagnostics
        .iter()
        .map(|d| render_diagnostic(d, sources))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostic, Label};
    use text_size::{TextRange, TextSize};

    #[test]
    fn test_render_simple_diagnostic() {
        let source = "let x = 42;";
        let mut sources = HashMap::new();
        sources.insert("test.nx".to_string(), source.to_string());

        let range = TextRange::new(TextSize::from(4), TextSize::from(5));
        let label = Label::primary("test.nx", range).with_message("undefined variable");

        let diag = Diagnostic::error("E001")
            .with_message("Variable not defined")
            .with_label(label)
            .with_help("Declare the variable before using it")
            .build();

        let rendered = render_diagnostic(&diag, &sources);

        // Basic sanity checks
        assert!(rendered.contains("Variable not defined"));
        assert!(rendered.contains("undefined variable"));
    }

    #[test]
    fn test_render_multiple_diagnostics() {
        let source = "let x = 42;\nlet y = 100;";
        let mut sources = HashMap::new();
        sources.insert("test.nx".to_string(), source.to_string());

        let diag1 = Diagnostic::error("E001")
            .with_message("First error")
            .with_label(Label::primary(
                "test.nx",
                TextRange::new(TextSize::from(4), TextSize::from(5)),
            ))
            .build();

        let diag2 = Diagnostic::warning("W001")
            .with_message("First warning")
            .with_label(Label::primary(
                "test.nx",
                TextRange::new(TextSize::from(16), TextSize::from(17)),
            ))
            .build();

        let rendered = render_diagnostics(&[diag1, diag2], &sources);

        assert!(rendered.contains("First error"));
        assert!(rendered.contains("First warning"));
    }
}
