//! Documentation attached to declarations and members by `///` doc comments.

use crate::Name;
use nx_diagnostics::{TextSize, TextSpan};
use pulldown_cmark::{BrokenLink, Event, LinkType, Options, Parser, Tag};
use smol_str::SmolStr;
use std::ops::Range;
use std::sync::Arc;

/// The documentation of one declaration or member: CommonMark text, with its doc links found.
///
/// <para>Shared rather than copied, because every copy of a declaration — an inherited field, an
/// imported interface item, an update record's mirrored field — carries its documentation.</para>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Doc(Arc<DocData>);

/// The contents of a [`Doc`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DocData {
    /// The documentation text, one line per doc comment line.
    pub text: String,
    /// The doc links in the text, in text order.
    pub links: Vec<DocLink>,
    /// The source offset where each line of `text` begins.
    pub line_starts: Vec<TextSize>,
}

impl std::ops::Deref for Doc {
    type Target = DocData;

    fn deref(&self) -> &DocData {
        &self.0
    }
}

/// A shortcut reference link, `[Name]` or `` [`Type.member`] ``, naming a declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocLink {
    /// The label as written between the brackets.
    pub label: SmolStr,
    /// The identifiers of the dot-separated path the label names.
    pub path: Arc<[Name]>,
    /// The range of `[label]` in the documentation text.
    pub text_range: Range<usize>,
    /// The source range of `[label]`.
    pub span: TextSpan,
}

impl DocLink {
    /// The path as written without backticks, `Type.member`.
    pub fn path_text(&self) -> String {
        self.path
            .iter()
            .map(Name::as_str)
            .collect::<Vec<_>>()
            .join(".")
    }

    /// The path as a Markdown code span, which is how a resolved link is shown.
    pub fn code_span(&self) -> String {
        format!("`{}`", self.path_text())
    }
}

impl Doc {
    /// Builds the documentation of an item from its attached doc comment.
    pub fn from_comment(comment: &nx_syntax::DocComment) -> Self {
        Self::new(&comment.text, &comment.line_starts)
    }

    /// Builds documentation from its text and the source offset where each line begins.
    pub fn new(text: &str, line_starts: &[TextSize]) -> Self {
        let links = doc_links(text)
            .into_iter()
            .map(|(label, path, text_range)| {
                let span = TextSpan::new(
                    nx_syntax::doc_text_source_offset(text, line_starts, text_range.start),
                    nx_syntax::doc_text_source_offset(text, line_starts, text_range.end),
                );
                DocLink {
                    label,
                    path,
                    text_range,
                    span,
                }
            })
            .collect::<Vec<_>>();
        Self(Arc::new(DocData {
            text: text.to_string(),
            links,
            line_starts: line_starts.to_vec(),
        }))
    }

    /// The doc link whose source range contains `offset`, if any.
    pub fn link_at(&self, offset: TextSize) -> Option<&DocLink> {
        self.links
            .iter()
            .find(|link| link.span.start() <= offset && offset < link.span.end())
    }

    /// The text with each doc link for which `replace` returns a string replaced by that string.
    /// A link for which it returns `None` is kept as written.
    pub fn replace_links(&self, mut replace: impl FnMut(&DocLink) -> Option<String>) -> String {
        let mut out = String::with_capacity(self.text.len());
        let mut copied = 0;
        for link in self.links.iter() {
            if let Some(replacement) = replace(link) {
                out.push_str(&self.text[copied..link.text_range.start]);
                out.push_str(&replacement);
                copied = link.text_range.end;
            }
        }
        out.push_str(&self.text[copied..]);
        out
    }

    /// The documentation as Markdown for a reader outside NX: the text with every doc link
    /// replaced by its path in a code span, and nothing else changed.
    ///
    /// <para>Generated host types and exported schemas both carry this form, so neither needs to
    /// know what a doc link is.</para>
    pub fn markdown(&self) -> String {
        self.replace_links(|link| Some(link.code_span()))
    }

    /// The summary of [`Doc::markdown`]: its first Markdown block.
    pub fn markdown_summary(&self) -> String {
        markdown_summary(&self.markdown()).to_string()
    }
}

/// The summary of CommonMark `text`: its first block, as written, without surrounding whitespace.
///
/// <para>For ordinary text that is the first paragraph. A block that can interrupt a paragraph,
/// such as a list, ends the summary without a blank line.</para>
pub fn markdown_summary(text: &str) -> &str {
    let mut depth = 0usize;
    for (event, range) in Parser::new_ext(text, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(_) => {
                if depth == 0 {
                    return text[range].trim();
                }
                depth += 1;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            _ if depth == 0 => return text[range].trim(),
            _ => {}
        }
    }
    text.trim()
}

/// Whether `offset` in CommonMark `text` is inside a code span or a code block, where brackets form
/// no doc link. A code span or fence that is still open is not code yet, by CommonMark's rules.
pub fn in_code(text: &str, offset: usize) -> bool {
    Parser::new_ext(text, Options::empty())
        .into_offset_iter()
        .any(|(event, range)| {
            matches!(event, Event::Code(_) | Event::Start(Tag::CodeBlock(_)))
                && range.start < offset
                && offset < range.end
        })
}

/// Finds the doc links in CommonMark `text`: shortcut reference links with no matching link
/// reference definition whose label is an identifier path, optionally in one code span.
///
/// The parser's broken-link callback reports exactly those references. It never looks inside a code
/// span or code block, and never sees a link with an explicit destination or one whose reference
/// is defined in the text.
fn doc_links(text: &str) -> Vec<(SmolStr, Arc<[Name]>, Range<usize>)> {
    let mut links = Vec::new();
    let callback = |link: BrokenLink<'_>| {
        if link.link_type == LinkType::Shortcut {
            let label = link.reference.as_ref();
            if let Some(path) = identifier_path(label) {
                links.push((SmolStr::new(label), path, link.span.clone()));
            }
        }
        None
    };
    Parser::new_with_broken_link_callback(text, Options::empty(), Some(callback)).for_each(drop);

    links.sort_by_key(|(_, _, range)| range.start);
    links.dedup_by_key(|(_, _, range)| range.start);
    links
}

/// Splits `Name`, `Type.member`, or either one in backticks into its names. A name may hold `-`
/// after its first character, as a markup-style member name such as `aria-label` does.
fn identifier_path(label: &str) -> Option<Arc<[Name]>> {
    let label = label
        .strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .unwrap_or(label);
    let parts = label.split('.').collect::<Vec<_>>();
    let is_identifier = |part: &&str| {
        let mut chars = part.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    };
    if !parts.iter().all(is_identifier) {
        return None;
    }
    Some(parts.into_iter().map(Name::new).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Doc {
        // One line per text line, each starting 4 bytes after the previous line's end, as if every
        // line were written `/// ` at the start of the file.
        let mut starts = Vec::new();
        let mut offset = 4u32;
        for line in text.split('\n') {
            starts.push(TextSize::from(offset));
            offset += line.len() as u32 + 1 + 4;
        }
        Doc::new(text, &starts)
    }

    fn paths(text: &str) -> Vec<String> {
        doc(text).links.iter().map(DocLink::path_text).collect()
    }

    #[test]
    fn a_bare_name_is_a_doc_link() {
        let doc = doc("See [renderNotice].");
        assert_eq!(doc.links.len(), 1);
        let link = &doc.links[0];
        assert_eq!(link.label, "renderNotice");
        assert_eq!(&doc.text[link.text_range.clone()], "[renderNotice]");
        assert_eq!(usize::from(link.span.start()), 4 + "See ".len());
        assert_eq!(usize::from(link.span.end()), 4 + "See [renderNotice]".len());
    }

    #[test]
    fn a_name_in_backticks_is_a_doc_link() {
        let doc = doc("See [`renderNotice`].");
        assert_eq!(paths(&doc.text), vec!["renderNotice"]);
        assert_eq!(doc.links[0].label, "`renderNotice`");
        assert_eq!(doc.links[0].code_span(), "`renderNotice`");
    }

    #[test]
    fn a_member_path_is_a_doc_link() {
        let doc = doc("Starts as [LoadState.idle].");
        assert_eq!(doc.links[0].path.len(), 2);
        assert_eq!(paths(&doc.text), vec!["LoadState.idle"]);
    }

    #[test]
    fn brackets_in_code_are_not_doc_links() {
        assert!(paths("Read `items[index]` first.").is_empty());
        assert!(paths("Example:\n\n```\nitems[index]\n```").is_empty());
        assert!(paths("Example:\n\n    items[index]").is_empty());
    }

    #[test]
    fn an_explicit_link_is_not_a_doc_link() {
        assert!(paths("Read [the spec](https://nxlang.org/spec).").is_empty());
    }

    #[test]
    fn a_label_with_a_reference_definition_is_not_a_doc_link() {
        assert!(paths("Read [spec].\n\n[spec]: https://nxlang.org/spec").is_empty());
    }

    #[test]
    fn a_label_that_is_not_an_identifier_path_is_not_a_doc_link() {
        assert!(paths("A [plain phrase] and [0-9] and [a.].").is_empty());
    }

    #[test]
    fn a_hyphenated_member_name_is_a_doc_link() {
        // A property may be named in markup style, and a link names it as written.
        assert_eq!(
            paths("Read [aria-label] and [Box.aria-label]."),
            vec!["aria-label", "Box.aria-label"]
        );
        assert!(paths("A [-x] is not one.").is_empty());
    }

    #[test]
    fn a_full_reference_is_not_a_doc_link() {
        assert!(paths("See [text][Name].").is_empty());
    }

    #[test]
    fn a_link_on_a_later_line_maps_to_its_source_line() {
        let doc = doc("Summary.\n\nSee [Theme].");
        let link = &doc.links[0];
        let expected = doc.line_starts[2] + TextSize::from("See ".len() as u32);
        assert_eq!(link.span.start(), expected);
        assert_eq!(doc.link_at(expected + TextSize::from(2)), Some(link));
        assert_eq!(doc.link_at(doc.line_starts[0]), None);
    }

    #[test]
    fn code_is_found_by_commonmark_rules() {
        let text = "Use ``a ` b`` and [x].\n\n    code [y]\n\n```\n[z]\n```";
        assert!(in_code(text, text.find("a ` b").unwrap()));
        assert!(!in_code(text, text.find("[x]").unwrap() + 1));
        assert!(in_code(text, text.find("[y]").unwrap() + 1));
        assert!(in_code(text, text.find("[z]").unwrap() + 1));
        assert!(!in_code("Unclosed `a [b", 11));
    }

    #[test]
    fn links_can_be_replaced_by_code_spans() {
        let doc = doc("Use [Grid] with [Missing] and [`Row`].");
        let replaced =
            doc.replace_links(|link| (link.path_text() != "Missing").then(|| link.code_span()));
        assert_eq!(replaced, "Use `Grid` with [Missing] and `Row`.");
    }

    #[test]
    fn markdown_writes_every_link_as_code() {
        assert_eq!(
            doc("Ignored when [maxMonthlyPrice] is set.").markdown(),
            "Ignored when `maxMonthlyPrice` is set."
        );
    }

    #[test]
    fn the_summary_is_the_first_block() {
        assert_eq!(
            doc("Summary line.\n\nMore detail.").markdown_summary(),
            "Summary line."
        );
        assert_eq!(doc("Modes:\n- fill\n- cover").markdown_summary(), "Modes:");
        assert_eq!(
            doc("Two lines\nof summary.\n\nRest.").markdown_summary(),
            "Two lines\nof summary."
        );
        assert_eq!(doc("See [Theme].").markdown_summary(), "See `Theme`.");
    }
}
