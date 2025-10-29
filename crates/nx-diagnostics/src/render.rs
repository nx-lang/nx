//! Rendering functionality for displaying diagnostics with beautiful formatting.

use crate::{Diagnostic, Severity};
use ariadne::{Color, Fmt, Label as AriadneLabel, Report, ReportKind, Source};
use std::collections::HashMap;
use std::fmt::Write as _;
//

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
#[cfg_attr(not(test), allow(dead_code))]
pub fn render_diagnostics(
    diagnostics: &[Diagnostic],
    sources: &HashMap<String, String>,
) -> String {
    diagnostics
        .iter()
        .map(|d| render_diagnostic(d, sources))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Renders diagnostics in a compact CLI style similar to common compilers.
///
/// Format example:
///   error complex-example.nx:12:34: Syntax error
///    12 | let <Foo x: string =
///       |                                  ^^^^^ unexpected syntax here
#[cfg_attr(not(test), allow(dead_code))]
pub fn render_diagnostics_cli(
    diagnostics: &[Diagnostic],
    sources: &HashMap<String, String>,
) -> String {
    let mut out = String::new();

    for (idx, d) in diagnostics.iter().enumerate() {
        // Pick primary label, or fall back to the first label if none is primary.
        let label = d
            .labels()
            .iter()
            .find(|l| l.primary)
            .or_else(|| d.labels().first())
            .cloned();

        let (file, start, end, label_msg) = if let Some(l) = label {
            let s: usize = l.range.start().into();
            let e: usize = l.range.end().into();
            (l.file, s, e, l.message)
        } else {
            (String::from("<unknown>"), 0usize, 0usize, None)
        };

        // Resolve source and compute line/col
        let src = sources.get(&file).map(String::as_str).unwrap_or("");
        let (line_num, col_num, line_text, col_in_line, highlight_len) = locate(src, start, end);

        // Header: severity file:line:col: message
        let severity = match d.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        let _ = writeln!(out, "{} {}:{}:{}: {}", severity, file, line_num, col_num, d.message());

        // Source line with caret underline
        if !line_text.is_empty() {
            let _ = writeln!(out, " {:>4} | {}", line_num, line_text);
            let caret_padding: String = " ".repeat(col_in_line.saturating_sub(1));
            let carets: String = "^".repeat(highlight_len.max(1));
            let caret_msg = label_msg.unwrap_or_default();
            if caret_msg.is_empty() {
                let _ = writeln!(out, "      | {}{}", caret_padding, carets);
            } else {
                let _ = writeln!(out, "      | {}{} {}", caret_padding, carets, caret_msg);
            }
        }

        if let Some(help) = d.help() {
            let _ = writeln!(out, "help: {}", help);
        }
        if let Some(note) = d.note() {
            let _ = writeln!(out, "note: {}", note);
        }

        if idx + 1 < diagnostics.len() {
            let _ = writeln!(out);
        }
    }

    out
}

// Compute 1-based line/col and a single-line highlight presentation
#[cfg_attr(not(test), allow(dead_code))]
fn locate<'a>(src: &'a str, start: usize, end: usize) -> (usize, usize, &'a str, usize, usize) {
    // Clamp indices to source length
    let len = src.len();
    let s = start.min(len);
    let e = end.min(len);

    // Find line boundaries
    let mut line_start = 0usize;
    let mut line_idx = 1usize; // 1-based
    for (i, ch) in src.char_indices() {
        if i >= s { break; }
        if ch == '\n' { line_idx += 1; line_start = i + ch.len_utf8(); }
    }
    let line_end = src[line_start..].find('\n').map(|o| line_start + o).unwrap_or(len);

    let line_text = &src[line_start..line_end];

    // Column (1-based) computed by counting chars from line_start to s
    let col_by_chars = src[line_start..s].chars().count() + 1;

    // Highlight length limited to the rest of the line, at least 1
    let range_len_chars = src[s..e].chars().count().max(1);
    let remaining_chars = line_text.chars().count().saturating_sub(col_by_chars - 1);
    let hl = range_len_chars.min(remaining_chars).max(1);

    (line_idx, col_by_chars, line_text, col_by_chars, hl)
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
