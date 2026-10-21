//! Source edits: what a reader meant, by node, turned into text edits.
//!
//! <para>A viewer that edits a document says which node it means and what it wants there: a
//! string, a number, a boolean or a union case for a value, a line of text for a run of content, or
//! the removal of a property. This module turns that into the smallest text edit that says the same
//! thing, so the rules of NX syntax (quoting, escapes, entities, braces, a property's own line) live
//! beside the parser and not in every viewer.</para>
//!
//! <para>The shape is unstable: it is published for the viewer to build on, and a later change
//! commits to it.</para>

use crate::source_tree::{SourceFlag, SourceNode, SourceRole, SourceTree};
use crate::{DocumentUri, EditorRange, LineIndex, SnapshotError, WorkspaceSnapshot};
use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};
use serde::{Deserialize, Serialize};
use text_size::{TextRange as ByteTextRange, TextSize};

/// One edit a reader asks for, aimed at one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceEdit {
    /// Set a literal or a bare union case that is a whole value.
    SetValue {
        /// The new value.
        value: SourceEditValue,
    },
    /// Set a run of text content written on one line.
    SetText {
        /// The new text, one line, as it should read once parsed.
        text: String,
    },
    /// Remove an attribute.
    Remove,
}

/// A value to write, by kind; the service decides its spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceEditValue {
    /// A string, written as a quoted string literal.
    String {
        /// The string's content, unescaped.
        value: String,
    },
    /// A number, written as given when it is an NX numeric literal.
    Number {
        /// The number as the reader typed it, such as `12.5` or `-3`.
        text: String,
    },
    /// A boolean.
    Boolean {
        /// The value.
        value: bool,
    },
    /// A union case, written as its bare name.
    Case {
        /// The case's name.
        name: String,
    },
}

/// One replacement in a document's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    /// The range replaced, in the document as the request carried it.
    pub range: EditorRange,
    /// What replaces it.
    pub new_text: String,
}

/// Why an edit was not made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceEditRefusal {
    /// The document's source tree holds no node with the key.
    UnknownNode,
    /// The node is not a target the edit takes.
    NotEditable,
    /// The value cannot be spelled.
    InvalidValue,
}

/// The answer to a source edit: the text edits, or why there are none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEditAnswer {
    /// The edits that make the change, applied one after another to the document as sent.
    pub edits: Vec<TextEdit>,
    /// Why the edit was not made, when it was not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<SourceEditRefusal>,
}

impl SourceEditAnswer {
    fn refused(reason: SourceEditRefusal) -> Self {
        Self {
            edits: Vec::new(),
            refused: Some(reason),
        }
    }
}

impl WorkspaceSnapshot {
    /// Answers the text edits that make `edit` at the node `key` of the document at `uri`.
    ///
    /// <para>The key is resolved against the source tree of the document as the snapshot holds it,
    /// so a key from an older tree still finds its node when nothing around it moved. What cannot
    /// be done is refused in the answer rather than failed.</para>
    pub fn source_edit(
        &self,
        uri: &DocumentUri,
        key: &str,
        edit: &SourceEdit,
    ) -> Result<SourceEditAnswer, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let tree = self.source_tree(uri)?;
        Ok(edit_tree(document.source(), &tree, key, edit))
    }
}

/// Answers `edit` at `key` over a document's text and its source tree.
fn edit_tree(source: &str, tree: &SourceTree, key: &str, edit: &SourceEdit) -> SourceEditAnswer {
    let Some(at) = tree.nodes.iter().position(|node| node.key == key) else {
        return SourceEditAnswer::refused(SourceEditRefusal::UnknownNode);
    };
    let result = match edit {
        SourceEdit::SetValue { value } => set_value(source, tree, at, value),
        SourceEdit::SetText { text } => set_text(source, tree, at, text),
        SourceEdit::Remove => remove(source, tree, at),
    };
    match result {
        Ok(Some((start, end, new_text))) => {
            if source.get(start..end) == Some(new_text.as_str()) {
                return SourceEditAnswer {
                    edits: Vec::new(),
                    refused: None,
                };
            }
            let index = LineIndex::new(source);
            let range = index.range_from_bytes(ByteTextRange::new(
                TextSize::from(start as u32),
                TextSize::from(end as u32),
            ));
            SourceEditAnswer {
                edits: vec![TextEdit { range, new_text }],
                refused: None,
            }
        }
        Ok(None) => SourceEditAnswer {
            edits: Vec::new(),
            refused: None,
        },
        Err(reason) => SourceEditAnswer::refused(reason),
    }
}

/// A replacement of the bytes `start..end`, or nothing to do.
type Replacement = Result<Option<(usize, usize, String)>, SourceEditRefusal>;

fn byte_span(node: &SourceNode) -> (usize, usize) {
    (node.range.start_byte as usize, node.range.end_byte as usize)
}

fn parent<'t>(tree: &'t SourceTree, node: &SourceNode) -> Option<&'t SourceNode> {
    node.parent
        .and_then(|parent| tree.nodes.get(parent as usize))
}

/// Whether the node at `at` is a whole value: an attribute's value, the value of a `let` without
/// parameters, or an item of a braced sequence that is itself a whole value.
pub(crate) fn is_whole_value(tree: &SourceTree, at: usize) -> bool {
    let node = &tree.nodes[at];
    let Some(owner) = parent(tree, node) else {
        return false;
    };
    match owner.role {
        SourceRole::Attribute => node.key == format!("{}.value", owner.key),
        SourceRole::Declaration => {
            let owner_index = node.parent.expect("a node with a parent") as usize;
            node.key == format!("{}.value", owner.key)
                && !tree.nodes.iter().any(|sibling| {
                    sibling.parent == Some(owner_index as u32)
                        && sibling.role == SourceRole::Parameter
                })
        }
        SourceRole::Sequence => {
            let owner_index = node.parent.expect("a node with a parent") as usize;
            node.key.starts_with(&format!("{}[", owner.key)) && is_whole_value(tree, owner_index)
        }
        _ => false,
    }
}

fn set_value(source: &str, tree: &SourceTree, at: usize, value: &SourceEditValue) -> Replacement {
    let node = &tree.nodes[at];
    if !matches!(node.role, SourceRole::Literal | SourceRole::Case) || !is_whole_value(tree, at) {
        return Err(SourceEditRefusal::NotEditable);
    }
    let (start, end) = byte_span(node);
    // A literal's range is its own token; braces written around it stay where they are. A string
    // literal's span is checked so that nothing but the token is replaced.
    if source.get(start..end).is_none() {
        return Err(SourceEditRefusal::NotEditable);
    }
    let text = spell_value(value)?;
    // A case written qualified, `{ Status.open }`, keeps its qualifier.
    if node.role == SourceRole::Case {
        if let Some(dot) = source[start..end].rfind('.') {
            if !matches!(value, SourceEditValue::Case { .. }) {
                return Err(SourceEditRefusal::InvalidValue);
            }
            return Ok(Some((start + dot + 1, end, text)));
        }
    }
    Ok(Some((start, end, text)))
}

/// Spells a value as NX source, or refuses one that cannot be spelled.
pub(crate) fn spell_value(value: &SourceEditValue) -> Result<String, SourceEditRefusal> {
    match value {
        SourceEditValue::String { value } => Ok(quote_string(value)),
        SourceEditValue::Number { text } => {
            if is_numeric_literal(text) {
                Ok(text.clone())
            } else {
                Err(SourceEditRefusal::InvalidValue)
            }
        }
        SourceEditValue::Boolean { value } => Ok(if *value { "true" } else { "false" }.to_string()),
        SourceEditValue::Case { name } => {
            if is_identifier(name) && name != "true" && name != "false" {
                Ok(name.clone())
            } else {
                Err(SourceEditRefusal::InvalidValue)
            }
        }
    }
}

/// A double-quoted string literal holding `value`, its quotes and backslashes escaped. Every other
/// character, a line break included, is written as it is, as a string literal may hold it.
fn quote_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Whether `text` is an NX numeric literal: an integer, a real or a hex literal, with an optional
/// leading minus.
pub(crate) fn is_numeric_literal(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        return !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit());
    }
    let (mantissa, exponent) = match digits.find(['e', 'E']) {
        Some(at) => (&digits[..at], Some(&digits[at + 1..])),
        None => (digits, None),
    };
    let all_digits = |part: &str| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit());
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => all_digits(whole) && all_digits(fraction),
        None => all_digits(mantissa),
    };
    match exponent {
        None => mantissa_ok,
        // An exponent belongs to a real literal only.
        Some(exponent) => {
            let exponent = exponent
                .strip_prefix('+')
                .or_else(|| exponent.strip_prefix('-'))
                .unwrap_or(exponent);
            mantissa_ok && mantissa.contains('.') && all_digits(exponent)
        }
    }
}

/// Whether `name` is an NX identifier.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn set_text(source: &str, tree: &SourceTree, at: usize, text: &str) -> Replacement {
    let node = &tree.nodes[at];
    if node.role != SourceRole::Text
        || node.text_type.is_some()
        || node.flags.contains(&SourceFlag::Raw)
        || node.range.start.line != node.range.end.line
    {
        return Err(SourceEditRefusal::NotEditable);
    }
    if text.trim().is_empty() || text.contains(['\n', '\r']) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    let (start, end) = byte_span(node);
    // Element content has no escapes: a run is the text as written, and what would start a tag,
    // a braced value or an `if`, `for` or `else` block cannot be in one. So the text is written as
    // it is, and kept only when the document then reads it back as one run of exactly that text.
    if !reads_as_one_run(source, start, end, text) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    Ok(Some((start, end, text.to_string())))
}

/// Whether replacing `start..end` of `source` with `text` leaves a run of text that is exactly
/// `text`, with no new parse error.
fn reads_as_one_run(source: &str, start: usize, end: usize, text: &str) -> bool {
    let mut after = String::with_capacity(source.len() + text.len());
    after.push_str(&source[..start]);
    after.push_str(text);
    after.push_str(&source[end..]);
    let before = parse_str(source, "edit.nx");
    let parsed = parse_str(&after, "edit.nx");
    if parsed.errors.len() > before.errors.len() {
        return false;
    }
    let Some(syntax) = parsed.tree else {
        return false;
    };
    let wanted = ByteTextRange::new(
        TextSize::from(start as u32),
        TextSize::from((start + text.len()) as u32),
    );
    fn find(node: SyntaxNode<'_>, wanted: ByteTextRange) -> bool {
        if matches!(node.kind(), SyntaxKind::TEXT_RUN) && node.span() == wanted {
            return true;
        }
        node.children()
            .filter(|child| child.span().contains_range(wanted))
            .any(|child| find(child, wanted))
    }
    find(syntax.root(), wanted)
}

fn remove(source: &str, tree: &SourceTree, at: usize) -> Replacement {
    let node = &tree.nodes[at];
    if node.role != SourceRole::Attribute || node.flags.contains(&SourceFlag::Content) {
        return Err(SourceEditRefusal::NotEditable);
    }
    let (start, end) = byte_span(node);
    let bytes = source.as_bytes();
    let is_blank = |byte: u8| byte == b' ' || byte == b'\t';

    // The space before the attribute, back to the previous token or the start of its line.
    let mut before = start;
    while before > 0 && is_blank(bytes[before - 1]) {
        before -= 1;
    }
    // The space after it, up to the next token or the end of its line.
    let mut after = end;
    while after < bytes.len() && is_blank(bytes[after]) {
        after += 1;
    }
    let starts_line = before == 0 || bytes[before - 1] == b'\n';
    let ends_line = after == bytes.len() || bytes[after] == b'\n' || bytes[after] == b'\r';
    if starts_line && ends_line {
        // Alone on its line: the whole line goes, its line break with it.
        let mut line_end = after;
        if source[line_end..].starts_with("\r\n") {
            line_end += 2;
        } else if line_end < bytes.len() {
            line_end += 1;
        }
        return Ok(Some((before, line_end, String::new())));
    }
    if starts_line {
        // First on its line, with more after it: the space after it goes instead.
        return Ok(Some((start, after, String::new())));
    }
    Ok(Some((before, end, String::new())))
}
