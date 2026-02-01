//! Minimal `.editorconfig` support for code generation formatting.
//!
//! This module intentionally implements only the subset of EditorConfig settings that are useful
//! for generated source formatting in this repo. It is not a full EditorConfig implementation.
//!
//! # Public API
//!
//! - [`EditorConfig::parse_file`] parses a `.editorconfig` file from disk.
//! - [`EditorConfig::apply_to`] applies supported settings to [`FormatOptions`] for a given target
//!   language and output file name (used for section matching like `[*.cs]` / `[*.ts]`).
//!
//! # Supported settings
//!
//! This module currently supports:
//! - `indent_style = space|tab`
//! - `indent_size = <n>|tab` (with `tab_width` as a fallback when `indent_size = tab`)
//! - `tab_width = <n>`
//! - `end_of_line = lf|crlf`
//! - C# only: `csharp_new_line_before_open_brace = all|none`
//!
//! Notes:
//! - Section matching is implemented with a simple `*` / `?` wildcard matcher and brace
//!   expansion (`*.{cs,csx}` style). It is intentionally minimal.
//! - For TypeScript brace style is intentionally not configurable; generation uses K&R by default.

use crate::codegen::options::{BraceStyle, FormatOptions, IndentStyle, NewlineStyle};
use crate::codegen::TargetLanguage;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug)]
struct Section {
    pattern: String,
    props: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct EditorConfig {
    global: HashMap<String, String>,
    sections: Vec<Section>,
}

impl EditorConfig {
    /// Parse a `.editorconfig` file from disk.
    ///
    /// This returns a lightweight representation of the file which can later be applied to
    /// [`FormatOptions`] via [`EditorConfig::apply_to`].
    pub fn parse_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read .editorconfig '{}': {}", path.display(), e))?;
        Ok(Self::parse_str(&text))
    }

    fn parse_str(text: &str) -> Self {
        let mut global = HashMap::new();
        let mut sections = Vec::new();

        let mut current_pattern: Option<String> = None;
        let mut current_props: HashMap<String, String> = HashMap::new();

        for raw_line in text.lines() {
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }

            if let Some(pattern) = parse_section_header(line) {
                if let Some(prev_pattern) = current_pattern.take() {
                    sections.push(Section {
                        pattern: prev_pattern,
                        props: std::mem::take(&mut current_props),
                    });
                } else {
                    global = std::mem::take(&mut current_props);
                }

                current_pattern = Some(pattern);
                continue;
            }

            if let Some((k, v)) = parse_key_value(line) {
                current_props.insert(k, v);
            }
        }

        if let Some(pattern) = current_pattern {
            sections.push(Section {
                pattern,
                props: current_props,
            });
        } else {
            global = current_props;
        }

        Self { global, sections }
    }

    /// Apply supported `.editorconfig` settings to the given [`FormatOptions`].
    ///
    /// `target_file_name` is used to determine which sections apply (e.g. `[*.cs]`).
    pub fn apply_to(
        &self,
        opts: &mut FormatOptions,
        language: TargetLanguage,
        target_file_name: &str,
    ) {
        let effective = self.effective_properties(target_file_name);
        apply_indent_settings(opts, &effective);
        apply_newline_settings(opts, &effective);
        apply_brace_settings(opts, language, &effective);
    }

    fn effective_properties(&self, target_file_name: &str) -> HashMap<String, String> {
        let mut out = self.global.clone();
        for section in &self.sections {
            if pattern_matches(&section.pattern, target_file_name) {
                for (k, v) in &section.props {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        out
    }
}

fn parse_section_header(line: &str) -> Option<String> {
    if !line.starts_with('[') || !line.ends_with(']') {
        return None;
    }
    Some(line[1..line.len() - 1].trim().to_string())
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

fn strip_comment(line: &str) -> &str {
    let mut chars = line.char_indices();
    while let Some((idx, ch)) = chars.next() {
        if ch == '#' || ch == ';' {
            return &line[..idx];
        }
    }
    line
}

fn pattern_matches(pattern: &str, text: &str) -> bool {
    for expanded in expand_braces(pattern) {
        if wildcard_match(&expanded, text) {
            return true;
        }
    }
    false
}

fn expand_braces(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_string()];
    };
    let Some(close) = pattern[open + 1..].find('}') else {
        return vec![pattern.to_string()];
    };
    let close = open + 1 + close;

    let prefix = &pattern[..open];
    let suffix = &pattern[close + 1..];
    let inner = &pattern[open + 1..close];

    let mut out = Vec::new();
    for part in inner.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        out.push(format!("{prefix}{part}{suffix}"));
    }

    if out.is_empty() {
        vec![pattern.to_string()]
    } else {
        out
    }
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    wildcard_match_inner(pattern.as_bytes(), text.as_bytes())
}

fn wildcard_match_inner(pattern: &[u8], text: &[u8]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    match pattern[0] {
        b'*' => {
            if wildcard_match_inner(&pattern[1..], text) {
                return true;
            }
            if !text.is_empty() {
                return wildcard_match_inner(pattern, &text[1..]);
            }
            false
        }
        b'?' => {
            if text.is_empty() {
                false
            } else {
                wildcard_match_inner(&pattern[1..], &text[1..])
            }
        }
        ch => {
            if text.first().copied() == Some(ch) {
                wildcard_match_inner(&pattern[1..], &text[1..])
            } else {
                false
            }
        }
    }
}

fn apply_indent_settings(opts: &mut FormatOptions, props: &HashMap<String, String>) {
    let indent_style = props
        .get("indent_style")
        .map(|s| s.trim().to_ascii_lowercase());
    if let Some(style) = indent_style.as_deref() {
        match style {
            "tab" => opts.indent_style = IndentStyle::Tabs,
            "space" => opts.indent_style = IndentStyle::Spaces,
            _ => {}
        }
    }

    if opts.indent_style == IndentStyle::Spaces {
        let indent_size = props
            .get("indent_size")
            .map(|s| s.trim().to_ascii_lowercase());
        if let Some(value) = indent_size.as_deref() {
            if value == "tab" {
                if let Some(width) = parse_usize(props.get("tab_width")) {
                    opts.indent_size = width;
                }
            } else if let Ok(size) = value.parse::<usize>() {
                opts.indent_size = size;
            }
        } else if let Some(width) = parse_usize(props.get("tab_width")) {
            opts.indent_size = width;
        }
    }
}

fn apply_newline_settings(opts: &mut FormatOptions, props: &HashMap<String, String>) {
    let eol = props
        .get("end_of_line")
        .map(|s| s.trim().to_ascii_lowercase());
    if let Some(value) = eol.as_deref() {
        match value {
            "lf" => opts.newline_style = NewlineStyle::Lf,
            "crlf" => opts.newline_style = NewlineStyle::CrLf,
            _ => {}
        }
    }
}

fn apply_brace_settings(
    opts: &mut FormatOptions,
    language: TargetLanguage,
    props: &HashMap<String, String>,
) {
    // For TypeScript we intentionally hardcode K&R (same-line braces) by default and do not
    // read any brace-style settings from .editorconfig.
    if language != TargetLanguage::CSharp {
        return;
    }

    if let Some(value) = props
        .get("csharp_new_line_before_open_brace")
        .map(|s| s.trim().to_ascii_lowercase())
    {
        if value == "all" {
            opts.brace_style = BraceStyle::Allman;
        } else if value == "none" {
            opts.brace_style = BraceStyle::KAndR;
        }
    }
}

fn parse_usize(value: Option<&String>) -> Option<usize> {
    let s = value?.trim();
    s.parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indent_and_braces_for_csharp() {
        let cfg = EditorConfig::parse_str(
            r#"
                root = true

                [*]
                indent_style = space
                indent_size = 4

                [*.cs]
                csharp_new_line_before_open_brace = none
            "#,
        );

        let mut opts = FormatOptions::defaults_for(TargetLanguage::CSharp);
        cfg.apply_to(&mut opts, TargetLanguage::CSharp, "Types.g.cs");

        assert_eq!(opts.indent_style, IndentStyle::Spaces);
        assert_eq!(opts.indent_size, 4);
        assert_eq!(opts.brace_style, BraceStyle::KAndR);
    }

    #[test]
    fn respects_section_matching() {
        let cfg = EditorConfig::parse_str(
            r#"
                [*]
                indent_style = space
                indent_size = 2

                [*.ts]
                indent_size = 4
            "#,
        );

        let mut ts = FormatOptions::defaults_for(TargetLanguage::TypeScript);
        cfg.apply_to(&mut ts, TargetLanguage::TypeScript, "types.ts");
        assert_eq!(ts.indent_size, 4);

        let mut cs = FormatOptions::defaults_for(TargetLanguage::CSharp);
        cfg.apply_to(&mut cs, TargetLanguage::CSharp, "types.cs");
        assert_eq!(cs.indent_size, 2);
    }
}
