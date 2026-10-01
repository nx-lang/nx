//! Writes NX documentation, which is CommonMark, as a TypeScript `/** */` comment.
//!
//! <para>TSDoc and JSDoc bodies are Markdown, and TypeScript tools render them, so the Markdown is
//! kept as written. Two things in it would otherwise be misread: a `*/`, which would end the
//! comment, and an `@` that begins a word, which JSDoc reads as a tag and would drop the text after
//! it from the description.</para>

use pulldown_cmark::{Event, Options, Parser, Tag};
use std::ops::Range;

/// The lines of the `/** */` comment for `markdown`: `/**`, one ` * ` line per line of the text,
/// and ` */`.
pub(crate) fn typescript_doc_lines(markdown: &str) -> Vec<String> {
    let markdown = markdown.trim_end();
    if markdown.is_empty() {
        return Vec::new();
    }
    let escaped = escape_tags(markdown).replace("*/", "*\\/");

    let mut lines = vec!["/**".to_string()];
    lines.extend(escaped.split('\n').map(|line| {
        if line.trim().is_empty() {
            " *".to_string()
        } else {
            format!(" * {}", line.trim_end())
        }
    }));
    lines.push(" */".to_string());
    lines
}

/// `markdown` with each `@` that begins a word outside code written `\@`.
fn escape_tags(markdown: &str) -> String {
    let code = code_ranges(markdown);
    let mut out = String::with_capacity(markdown.len());
    let mut previous: Option<char> = None;
    let mut chars = markdown.char_indices().peekable();
    while let Some((offset, c)) = chars.next() {
        let begins_word = previous.is_none_or(|p| !p.is_alphanumeric() && p != '_' && p != '\\');
        let before_letter = chars.peek().is_some_and(|(_, next)| next.is_alphabetic());
        let in_code = code.iter().any(|range| range.contains(&offset));
        if c == '@' && begins_word && before_letter && !in_code {
            out.push('\\');
        }
        out.push(c);
        previous = Some(c);
    }
    out
}

/// The source ranges of the code spans and code blocks in `markdown`.
fn code_ranges(markdown: &str) -> Vec<Range<usize>> {
    Parser::new_ext(markdown, Options::empty())
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Code(_) | Event::Start(Tag::CodeBlock(_)) => Some(range),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::typescript_doc_lines;

    fn doc(markdown: &str) -> String {
        typescript_doc_lines(markdown).join("\n")
    }

    #[test]
    fn one_line_is_one_prefixed_line() {
        assert_eq!(doc("Hint text."), "/**\n * Hint text.\n */");
    }

    #[test]
    fn nothing_writes_no_comment() {
        assert!(typescript_doc_lines("").is_empty());
    }

    #[test]
    fn markdown_is_kept_and_blank_lines_are_bare_stars() {
        assert_eq!(
            doc("Use **only** with `Grid`.\n\n- fill\n- cover"),
            "/**\n * Use **only** with `Grid`.\n *\n * - fill\n * - cover\n */"
        );
    }

    #[test]
    fn a_comment_terminator_does_not_end_the_comment() {
        assert_eq!(
            doc("Ends with */ here."),
            "/**\n * Ends with *\\/ here.\n */"
        );
    }

    #[test]
    fn a_word_initial_at_sign_is_escaped() {
        assert_eq!(
            doc("@nx/prelude provides it; see (@types)."),
            "/**\n * \\@nx/prelude provides it; see (\\@types).\n */"
        );
    }

    #[test]
    fn an_at_sign_in_code_or_mid_word_is_kept() {
        assert_eq!(
            doc("Import `@nx/prelude`; mail ada@example.com; @ alone."),
            "/**\n * Import `@nx/prelude`; mail ada@example.com; @ alone.\n */"
        );
        assert_eq!(
            doc("Example:\n\n```\n@decorator\n```"),
            "/**\n * Example:\n *\n * ```\n * @decorator\n * ```\n */"
        );
    }
}
