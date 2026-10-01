//! Attaches `///` doc comments to the declarations and members they document.
//!
//! Doc comments are trivia, so tree-sitter hangs each one on whatever node happens to be open
//! where it appears. This pass ignores that placement and works from lines instead:
//!
//! - A *leading* doc comment has only whitespace before it on its line. Leading comments on
//!   consecutive lines form a block, which documents the outermost documentable item that starts
//!   at the first token of the next line.
//! - A *trailing* doc comment follows code on its line. It documents the one outermost documentable
//!   item that both starts and ends on that line.
//!
//! Every later stage reads the resulting [`DocComments`] table rather than rediscovering comments.

use crate::{SyntaxKind, SyntaxNode};
use nx_diagnostics::{Diagnostic, Label};
use std::collections::HashMap;
use text_size::{TextRange, TextSize};

/// The documentation attached to one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocComment {
    /// The documentation text: each comment's text after `///` and one following space, joined by
    /// line breaks in source order.
    pub text: String,
    /// The source offset where each line of `text` begins, so an offset in the text maps back to
    /// the source.
    pub line_starts: Vec<TextSize>,
    /// The source range of the doc comment or block.
    pub span: TextRange,
}

/// Maps an offset in documentation text to the source offset it was read from, given the source
/// offset where each line of the text begins.
///
/// An offset past the end of the text maps to the end of the last line.
pub fn doc_text_source_offset(
    text: &str,
    line_starts: &[TextSize],
    text_offset: usize,
) -> TextSize {
    let mut line_text_start = 0;
    let mut last = TextSize::from(0);
    for (line, line_start) in text.split('\n').zip(line_starts) {
        let line_text_end = line_text_start + line.len();
        let column = text_offset
            .min(line_text_end)
            .saturating_sub(line_text_start);
        last = *line_start
            + TextSize::from(
                u32::try_from(column).expect("doc lines are bounded by the source size"),
            );
        if text_offset <= line_text_end {
            break;
        }
        line_text_start = line_text_end + 1;
    }
    last
}

/// The doc comments of one source file, keyed by the start offset of the item each documents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocComments {
    docs: HashMap<TextSize, DocComment>,
}

impl DocComments {
    /// Returns the documentation of the documentable item whose syntax node starts at `start`.
    ///
    /// Only the outermost documentable item starting at an offset is ever documented, so a caller
    /// asks with the start of the node it is lowering.
    pub fn get(&self, start: TextSize) -> Option<&DocComment> {
        self.docs.get(&start)
    }

    /// Every attached doc comment, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &DocComment> {
        self.docs.values()
    }
}

/// Attaches the doc comments under `root` to the items they document.
///
/// A doc comment that breaks an attachment rule documents nothing here; syntax validation reports
/// it.
pub fn doc_comments(root: &SyntaxNode) -> DocComments {
    analyze(root).docs
}

/// Reports every doc comment that breaks an attachment rule.
pub(crate) fn validate_doc_comments(
    root: &SyntaxNode,
    file_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // A half-typed declaration is an error-recovery node, not a documentable item, so the doc
    // comment above it would also be reported as dangling. The parse error already says what is
    // wrong.
    let report_dangling = !root.has_error();
    for problem in analyze(root).problems {
        let diagnostic = match problem.kind {
            ProblemKind::Dangling if !report_dangling => continue,
            ProblemKind::Dangling => Diagnostic::error("dangling-doc-comment")
                .with_message(
                    "This doc comment documents nothing: a `///` block must be directly above a \
                     declaration or member, and a trailing `///` must follow one that starts and \
                     ends on its line",
                )
                .with_label(Label::primary(file_name, problem.span))
                .with_help(
                    "Move it directly above what it documents, or write `//` for an ordinary \
                     comment",
                ),
            ProblemKind::Ambiguous => Diagnostic::error("ambiguous-trailing-doc-comment")
                .with_message(
                    "This trailing doc comment follows more than one item on its line, so it is \
                     not clear which it documents",
                )
                .with_label(Label::primary(file_name, problem.span))
                .with_help("Put each item on its own line, or document them in leading blocks"),
            ProblemKind::Misaligned => Diagnostic::error("misaligned-doc-comment-continuation")
                .with_message(
                    "This `///` line is directly under a trailing doc comment but not aligned with \
                     it; a trailing doc comment continues only on lines whose `///` starts at its \
                     own column",
                )
                .with_label(Label::primary(file_name, problem.span))
                .with_help(
                    "Align the `///` under the one above to continue that comment, or put a blank \
                     line before it to document the item below",
                ),
            ProblemKind::Duplicate => Diagnostic::error("duplicate-doc-comment")
                .with_message(
                    "This item is already documented by the `///` block above it; an item is \
                     documented once",
                )
                .with_label(Label::primary(file_name, problem.span))
                .with_help("Keep either the leading block or the trailing doc comment"),
        };
        diagnostics.push(diagnostic.build());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProblemKind {
    Dangling,
    Ambiguous,
    Misaligned,
    Duplicate,
}

#[derive(Debug)]
struct Problem {
    kind: ProblemKind,
    span: TextRange,
}

struct Analysis {
    docs: DocComments,
    problems: Vec<Problem>,
}

/// A `///` token and where it is.
struct Comment {
    span: TextRange,
    line: usize,
    /// The column of the `///`, in characters from the start of its line.
    column: usize,
    leading: bool,
}

/// A documentable syntax node and the lines it spans.
struct Item {
    span: TextRange,
    start_line: usize,
    end_line: usize,
}

fn analyze(root: &SyntaxNode) -> Analysis {
    let source = root.source_text();
    let lines = LineIndex::new(source);

    let mut comments = Vec::new();
    let mut items = Vec::new();
    collect(root, false, source, &lines, &mut comments, &mut items);
    comments.sort_by_key(|comment| comment.span.start());

    let mut docs = HashMap::new();
    let mut problems = Vec::new();

    // Leading blocks first, so that a trailing comment on an item a block already documents is the
    // one reported, and so that each trailing comment's continuation lines are known.
    let trailing: HashMap<usize, &Comment> = comments
        .iter()
        .filter(|comment| !comment.leading)
        .map(|comment| (comment.line, comment))
        .collect();
    let mut continuations: HashMap<usize, Vec<&Comment>> = HashMap::new();
    let leading: Vec<&Comment> = comments.iter().filter(|comment| comment.leading).collect();
    for block in leading.chunk_by(|a, b| b.line == a.line + 1) {
        let first = block[0];
        let last = block[block.len() - 1];
        let span = TextRange::new(first.span.start(), last.span.end());

        // A block directly under a trailing doc comment continues it, as far as its lines are
        // aligned with it. A line that is not ends the continuation, and it and the rest of the
        // block document nothing: a leading block for the next item needs a blank line first.
        let above = first
            .line
            .checked_sub(1)
            .and_then(|line| trailing.get(&line));
        if let Some(trailing) = above {
            let aligned = block
                .iter()
                .take_while(|comment| comment.column == trailing.column)
                .count();
            continuations.insert(trailing.line, block[..aligned].to_vec());
            if let Some(misaligned) = block.get(aligned) {
                problems.push(Problem {
                    kind: ProblemKind::Misaligned,
                    span: TextRange::new(misaligned.span.start(), last.span.end()),
                });
            }
            continue;
        }

        let target = lines
            .first_non_whitespace(source, last.line + 1)
            .and_then(|offset| outermost_starting_at(&items, offset));
        match target {
            Some(item) => {
                docs.insert(item.span.start(), doc_comment(source, block, span));
            }
            None => problems.push(Problem {
                kind: ProblemKind::Dangling,
                span,
            }),
        }
    }

    for comment in comments.iter().filter(|comment| !comment.leading) {
        let mut lines_of_comment = vec![comment];
        lines_of_comment.extend(continuations.remove(&comment.line).unwrap_or_default());
        let span = TextRange::new(
            comment.span.start(),
            lines_of_comment[lines_of_comment.len() - 1].span.end(),
        );
        let on_line: Vec<&Item> = items
            .iter()
            .filter(|item| item.start_line == comment.line && item.end_line == comment.line)
            .collect();
        let mut outermost: Vec<&Item> = on_line
            .iter()
            .copied()
            .filter(|item| {
                !on_line
                    .iter()
                    .any(|other| other.span != item.span && other.span.contains_range(item.span))
            })
            .collect();
        outermost.dedup_by_key(|item| item.span);

        let kind = match outermost.as_slice() {
            [] => ProblemKind::Dangling,
            [item] if docs.contains_key(&item.span.start()) => ProblemKind::Duplicate,
            [item] => {
                docs.insert(
                    item.span.start(),
                    doc_comment(source, &lines_of_comment, span),
                );
                continue;
            }
            _ => ProblemKind::Ambiguous,
        };
        problems.push(Problem { kind, span });
    }

    problems.sort_by_key(|problem| problem.span.start());
    Analysis {
        docs: DocComments { docs },
        problems,
    }
}

fn collect(
    node: &SyntaxNode,
    in_function_type: bool,
    source: &str,
    lines: &LineIndex,
    comments: &mut Vec<Comment>,
    items: &mut Vec<Item>,
) {
    let kind = node.kind();
    if kind == SyntaxKind::DOC_COMMENT {
        let span = node.span();
        let line = lines.line_of(span.start());
        let line_start = usize::from(lines.starts[line]);
        let before = &source[line_start..usize::from(span.start())];
        comments.push(Comment {
            span,
            line,
            column: before.chars().count(),
            leading: before.chars().all(char::is_whitespace),
        });
        return;
    }

    if is_documentable(node, in_function_type) && !node.is_error() {
        let span = node.span();
        items.push(Item {
            span,
            start_line: node.start_position().0,
            end_line: node.end_position().0,
        });
    }

    // A function type's parameters are part of a type, not declarations.
    let in_function_type = in_function_type || kind == SyntaxKind::FUNCTION_TYPE;
    for child in node.children_with_tokens() {
        collect(&child, in_function_type, source, lines, comments, items);
    }
}

/// Whether `node` is one of the items the `doc-comments` spec lists as documentable.
fn is_documentable(node: &SyntaxNode, in_function_type: bool) -> bool {
    match node.kind() {
        SyntaxKind::RECORD_DEFINITION
        | SyntaxKind::ACTION_DEFINITION
        | SyntaxKind::UNION_DEFINITION
        | SyntaxKind::TYPE_DEFINITION
        | SyntaxKind::VALUE_DEFINITION
        | SyntaxKind::FUNCTION_DEFINITION
        | SyntaxKind::COMPONENT_DEFINITION
        | SyntaxKind::UNION_CASE
        | SyntaxKind::EMIT_DEFINITION
        | SyntaxKind::EMIT_REFERENCE => true,
        // A type parameter, `T:type`, is written like a property but is not a member.
        SyntaxKind::PROPERTY_DEFINITION => {
            !in_function_type && !crate::ast::property_definition_is_type_parameter(node)
        }
        _ => false,
    }
}

fn outermost_starting_at(items: &[Item], offset: TextSize) -> Option<&Item> {
    items
        .iter()
        .filter(|item| item.span.start() == offset)
        .max_by_key(|item| item.span.len())
}

fn doc_comment(source: &str, comments: &[&Comment], span: TextRange) -> DocComment {
    let mut text = String::new();
    let mut line_starts = Vec::with_capacity(comments.len());
    for (index, comment) in comments.iter().enumerate() {
        let raw = &source[comment.span];
        let body = &raw["///".len()..];
        let (body, skipped) = match body.strip_prefix(' ') {
            Some(rest) => (rest, "/// ".len()),
            None => (body, "///".len()),
        };
        let body = body.strip_suffix('\r').unwrap_or(body);
        if index > 0 {
            text.push('\n');
        }
        text.push_str(body);
        line_starts.push(
            comment.span.start()
                + TextSize::from(u32::try_from(skipped).expect("marker length fits in u32")),
        );
    }
    DocComment {
        text,
        line_starts,
        span,
    }
}

/// The start offset of every line in a source.
struct LineIndex {
    starts: Vec<TextSize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![TextSize::from(0)];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(TextSize::from(
                    u32::try_from(offset + 1).expect("NX source size is validated before parsing"),
                ));
            }
        }
        Self { starts }
    }

    fn line_of(&self, offset: TextSize) -> usize {
        self.starts.partition_point(|&start| start <= offset) - 1
    }

    /// The offset of the first non-whitespace character on `line`, if the line exists and is not
    /// blank.
    fn first_non_whitespace(&self, source: &str, line: usize) -> Option<TextSize> {
        let start = usize::from(*self.starts.get(line)?);
        let end = self
            .starts
            .get(line + 1)
            .map_or(source.len(), |&next| usize::from(next));
        let text = &source[start..end];
        let indent = text.len() - text.trim_start().len();
        if start + indent >= end || text.trim().is_empty() {
            return None;
        }
        Some(TextSize::from(
            u32::try_from(start + indent).expect("NX source size is validated before parsing"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    /// Parses `source` and returns the documentation of the item whose source starts with `item`,
    /// searching from the first occurrence of `item`.
    fn doc_of(source: &str, item: &str) -> Option<String> {
        let result = parse_str(source, "test.nx");
        let root = result.root().expect("root");
        let docs = doc_comments(&root);
        let offset = source.find(item).expect("item text is in the source");
        docs.get(TextSize::from(offset as u32))
            .map(|doc| doc.text.clone())
    }

    fn codes(source: &str) -> Vec<String> {
        parse_str(source, "test.nx")
            .errors
            .iter()
            .filter_map(|diagnostic| diagnostic.code().map(str::to_string))
            .collect()
    }

    const SEARCH_BOX: &str = "\
/// A text box.
component <SearchBox
  /// Hint text.
  placeholder:string
  value:string   /// The current text.
  emits {
    /// Fired on every keystroke.
    ValueChanged { value:string }
  }
/> = { <span /> }
";

    #[test]
    fn a_leading_block_documents_a_component() {
        assert_eq!(
            doc_of(SEARCH_BOX, "component <SearchBox").as_deref(),
            Some("A text box.")
        );
        assert!(codes(SEARCH_BOX).is_empty(), "{:?}", codes(SEARCH_BOX));
    }

    #[test]
    fn a_leading_block_documents_a_property_not_the_component() {
        assert_eq!(
            doc_of(SEARCH_BOX, "placeholder:string").as_deref(),
            Some("Hint text.")
        );
    }

    #[test]
    fn a_leading_block_documents_a_union_case() {
        let source = "\
type LoadState =
  /// No search has run yet.
  | idle
  | loading
";
        assert_eq!(
            doc_of(source, "| idle").as_deref(),
            Some("No search has run yet.")
        );
        assert!(codes(source).is_empty(), "{:?}", codes(source));
    }

    #[test]
    fn the_outermost_item_starting_on_the_line_is_documented() {
        assert_eq!(
            doc_of(SEARCH_BOX, "ValueChanged").as_deref(),
            Some("Fired on every keystroke.")
        );
        assert_eq!(doc_of(SEARCH_BOX, "value:string }"), None);
    }

    #[test]
    fn a_visibility_modifier_is_part_of_the_declaration() {
        let source = "/// Shared theme.\nexport type Theme = string\n";
        assert_eq!(
            doc_of(source, "export type Theme").as_deref(),
            Some("Shared theme.")
        );
    }

    #[test]
    fn a_trailing_doc_comment_documents_a_property() {
        assert_eq!(
            doc_of(SEARCH_BOX, "value:string   ").as_deref(),
            Some("The current text.")
        );
    }

    #[test]
    fn a_trailing_doc_comment_documents_a_union_case_with_a_payload() {
        let source = "\
type LoadState =
  | idle
  | failed { message:string }   /// The last search failed.
";
        assert_eq!(
            doc_of(source, "| failed").as_deref(),
            Some("The last search failed.")
        );
        assert_eq!(doc_of(source, "message:string"), None);
        assert!(codes(source).is_empty(), "{:?}", codes(source));
    }

    #[test]
    fn a_trailing_doc_comment_documents_a_one_line_declaration() {
        let source = "type Size = int   /// Size in device pixels.\n";
        assert_eq!(
            doc_of(source, "type Size").as_deref(),
            Some("Size in device pixels.")
        );
    }

    #[test]
    fn four_slashes_document_nothing_and_report_nothing() {
        let source = "//// Section\ntype Theme = string\n";
        assert_eq!(doc_of(source, "type Theme"), None);
        assert!(codes(source).is_empty(), "{:?}", codes(source));
    }

    #[test]
    fn a_block_comment_is_never_documentation() {
        let source = "/** Summary. */\ntype Theme = string\n";
        assert_eq!(doc_of(source, "type Theme"), None);
    }

    #[test]
    fn a_doc_block_separated_by_a_blank_line_is_dangling() {
        let source = "/// Theme.\n\ntype Theme = string\n";
        assert_eq!(doc_of(source, "type Theme"), None);
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);
    }

    #[test]
    fn a_doc_block_above_an_ordinary_comment_is_dangling() {
        let source = "/// Theme.\n// note\ntype Theme = string\n";
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);
    }

    #[test]
    fn a_doc_comment_at_the_end_of_a_file_is_dangling() {
        assert_eq!(
            codes("type Theme = string\n/// Nothing follows."),
            vec!["dangling-doc-comment"]
        );
        assert_eq!(
            codes("type Theme = string\n/// Nothing follows.\n"),
            vec!["dangling-doc-comment"]
        );
    }

    #[test]
    fn a_doc_comment_above_an_expression_is_dangling() {
        let source = "\
component <Search /> = {
  /// The input.
  <TextInput value={query} />
}
";
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);
    }

    #[test]
    fn a_doc_comment_on_a_type_parameter_is_dangling() {
        let source = "\
type Range = {
  /// The element type.
  T:type
  start:T
}
";
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);

        let source = "\
component <List
  TItem:type   /// The item type.
  items:TItem+
/> = { <span /> }
";
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);
    }

    #[test]
    fn a_trailing_doc_comment_on_a_closing_brace_is_dangling() {
        let source = "type Contact = {\n  name:string\n}   /// End of the record.\n";
        assert_eq!(doc_of(source, "type Contact"), None);
        assert_eq!(codes(source), vec!["dangling-doc-comment"]);
    }

    #[test]
    fn a_trailing_doc_comment_after_two_properties_is_ambiguous() {
        let source = "\
component <Card
  tone:string density:string   /// Style.
/> = { <span /> }
";
        assert_eq!(doc_of(source, "tone:string"), None);
        assert_eq!(doc_of(source, "density:string"), None);
        assert_eq!(codes(source), vec!["ambiguous-trailing-doc-comment"]);
    }

    #[test]
    fn an_aligned_continuation_extends_a_trailing_doc_comment() {
        let source = "\
component <SearchBox
  placeholder:string   /// Hint text shown while
                       /// the box is empty.
  tone:string
/> = { <span /> }
";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        assert_eq!(
            doc_of(source, "placeholder:string").as_deref(),
            Some("Hint text shown while\nthe box is empty.")
        );
        assert_eq!(doc_of(source, "tone:string"), None);
    }

    #[test]
    fn a_blank_line_separates_a_leading_block_from_a_trailing_doc_comment() {
        let source = "\
component <Shelf
  label:string   /// The label.

  /// Which way it faces.
  facing:string
/> = { <span /> }
";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        assert_eq!(
            doc_of(source, "label:string").as_deref(),
            Some("The label.")
        );
        assert_eq!(
            doc_of(source, "facing:string").as_deref(),
            Some("Which way it faces.")
        );
    }

    #[test]
    fn a_misaligned_continuation_is_an_error() {
        let source = "\
component <Shelf
  label:string   /// The label.
  /// Which way it faces.
  facing:string
/> = { <span /> }
";
        let result = parse_str(source, "test.nx");
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        let diagnostic = &result.errors[0];
        assert_eq!(
            diagnostic.code(),
            Some("misaligned-doc-comment-continuation")
        );
        assert!(
            diagnostic.message().contains("not aligned"),
            "{diagnostic:?}"
        );
        assert!(
            diagnostic
                .help()
                .is_some_and(|help| help.contains("blank line")),
            "{diagnostic:?}"
        );
        let start = source.find("/// Which").unwrap();
        assert_eq!(usize::from(diagnostic.labels()[0].range.start()), start);

        assert_eq!(
            doc_of(source, "label:string").as_deref(),
            Some("The label.")
        );
        assert_eq!(doc_of(source, "facing:string"), None);
    }

    #[test]
    fn a_partly_aligned_continuation_keeps_its_aligned_lines() {
        let source = "\
component <SearchBox
  placeholder:string   /// Hint text shown while
                       /// the box is empty.
  /// The tone.
  tone:string
/> = { <span /> }
";
        let result = parse_str(source, "test.nx");
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        let diagnostic = &result.errors[0];
        assert_eq!(
            diagnostic.code(),
            Some("misaligned-doc-comment-continuation")
        );
        let start = source.find("/// The tone.").unwrap();
        assert_eq!(usize::from(diagnostic.labels()[0].range.start()), start);
        assert_eq!(
            doc_of(source, "placeholder:string").as_deref(),
            Some("Hint text shown while\nthe box is empty.")
        );
        assert_eq!(doc_of(source, "tone:string"), None);
    }

    #[test]
    fn alignment_counts_characters_not_bytes() {
        let source = "\
component <Label
  text:string = \"é\"   /// Shown as written,
                      /// accents included.
/> = { <span /> }
";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        assert_eq!(
            doc_of(source, "text:string").as_deref(),
            Some("Shown as written,\naccents included.")
        );
    }

    #[test]
    fn an_item_documented_both_ways_reports_the_trailing_comment() {
        let source = "\
component <SearchBox
  /// Hint.
  placeholder:string   /// Placeholder.
/> = { <span /> }
";
        let result = parse_str(source, "test.nx");
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        let diagnostic = &result.errors[0];
        assert_eq!(diagnostic.code(), Some("duplicate-doc-comment"));
        let start = source.find("/// Placeholder.").unwrap();
        assert_eq!(usize::from(diagnostic.labels()[0].range.start()), start);
        assert_eq!(
            doc_of(source, "placeholder:string").as_deref(),
            Some("Hint.")
        );
    }

    #[test]
    fn a_dangling_doc_comment_is_not_reported_when_the_file_has_parse_errors() {
        let source = "/// Half typed.\ntype Contact = {\n";
        let codes = codes(source);
        assert!(!codes.is_empty());
        assert!(
            !codes.iter().any(|code| code == "dangling-doc-comment"),
            "{codes:?}"
        );
    }

    #[test]
    fn an_empty_doc_line_is_a_blank_line_in_the_text() {
        let source = "/// Summary.\n///\n/// Details.\ntype Theme = string\n";
        assert_eq!(
            doc_of(source, "type Theme").as_deref(),
            Some("Summary.\n\nDetails.")
        );
    }

    #[test]
    fn only_one_space_after_the_marker_is_stripped() {
        let source = "/// Items:\n///   - one\ntype Theme = string\n";
        assert_eq!(
            doc_of(source, "type Theme").as_deref(),
            Some("Items:\n  - one")
        );
    }

    #[test]
    fn a_text_offset_maps_back_to_its_source_offset() {
        let source = "/// Summary.\n///\n///See [Theme] here.\ntype Theme = string\n";
        let result = parse_str(source, "test.nx");
        let docs = doc_comments(&result.root().unwrap());
        let doc = docs
            .get(TextSize::from(source.find("type Theme").unwrap() as u32))
            .unwrap();
        assert_eq!(doc.text, "Summary.\n\nSee [Theme] here.");

        let source_offset =
            |doc: &DocComment, offset| doc_text_source_offset(&doc.text, &doc.line_starts, offset);
        let in_text = doc.text.find("[Theme]").unwrap();
        let in_source = source.find("[Theme]").unwrap();
        assert_eq!(usize::from(source_offset(doc, in_text)), in_source);
        assert_eq!(usize::from(source_offset(doc, 0)), "/// ".len());
        assert_eq!(
            usize::from(source_offset(doc, doc.text.find("Summary.").unwrap() + 3)),
            "/// Sum".len()
        );
    }

    #[test]
    fn crlf_line_endings_are_not_part_of_the_text() {
        let source = "/// Summary.\r\n/// Details.\r\ntype Theme = string\r\n";
        assert_eq!(
            doc_of(source, "type Theme").as_deref(),
            Some("Summary.\nDetails.")
        );
    }

    #[test]
    fn an_empty_doc_line_in_a_crlf_file_is_part_of_the_block() {
        let source = "/// Summary.\r\n///\r\n/// Details.\r\ntype Theme = string\r\n";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        assert_eq!(
            doc_of(source, "type Theme").as_deref(),
            Some("Summary.\n\nDetails.")
        );

        let source = "\
type Shelf = {\r
  label:string   /// The label.\r
                 ///\r
                 /// Shown on the front.\r
}\r
";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        assert_eq!(
            doc_of(source, "label:string").as_deref(),
            Some("The label.\n\nShown on the front.")
        );
    }

    #[test]
    fn documentable_members_of_every_kind_are_found() {
        let source = "\
/// A record.
type Contact = {
  /// A field.
  name:string
}

/// An action.
action Saved = {
  id:int   /// The saved id.
}

/// A value.
let answer = 42

/// A function.
let add(
  /// The count.
  count:int
): int = {count + 1}

/// An element function.
let <Badge
  /// The label.
  label:string
/> = <span>{label}</span>

component <Search
  emits {
    /// A reference.
    Saved
  }
/> = {
  state {
    /// The query.
    query:string
  }
  <span />
}
";
        assert!(codes(source).is_empty(), "{:?}", codes(source));
        for (item, doc) in [
            ("type Contact", "A record."),
            ("name:string", "A field."),
            ("action Saved", "An action."),
            ("id:int", "The saved id."),
            ("let answer", "A value."),
            ("let add", "A function."),
            ("count:int", "The count."),
            ("let <Badge", "An element function."),
            ("label:string", "The label."),
            ("Saved\n  }", "A reference."),
            ("query:string", "The query."),
        ] {
            assert_eq!(doc_of(source, item).as_deref(), Some(doc), "{item}");
        }
    }
}
