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

use crate::source_tree::{
    SourceDeclarationKind, SourceFlag, SourceNode, SourceProperty, SourceRole, SourceTree,
};
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
    /// Remove an attribute, or an element that is an item of a list with the comments above it.
    Remove,
    /// Add a property an element does not write, as an attribute in the element's layout.
    AddProperty {
        /// The property's name.
        name: String,
        /// Its value.
        value: SourceEditValue,
    },
    /// Move an element that is an item of a list to another position in that list.
    Move {
        /// The position the item takes once moved, counted as the list's keys count its items.
        to: usize,
    },
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
    /// A typed hexadecimal color literal.
    Color {
        /// Hexadecimal source spelling, including the hash.
        text: String,
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
    /// The edits that make the change. Each range is in the document as sent, and no two
    /// overlap, so a host applies them together, as an editor applies a batch of edits.
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
        SourceEdit::SetValue {
            value: SourceEditValue::Color { text },
        } if matches!(tree.nodes[at].role, SourceRole::Call | SourceRole::Element) => {
            set_constructor_color(source, tree, at, text)
        }
        SourceEdit::SetValue { value } => set_value(source, tree, at, value).map(Vec::from_iter),
        SourceEdit::SetText { text } => set_text(source, tree, at, text).map(Vec::from_iter),
        SourceEdit::Remove => remove(source, tree, at),
        SourceEdit::AddProperty { name, value } => {
            add_property(source, tree, at, name, value).map(Vec::from_iter)
        }
        SourceEdit::Move { to } => move_item(source, tree, at, *to),
    };
    match result {
        Ok(mut replacements) => {
            // A replacement that writes what is there already changes nothing.
            replacements.retain(|(start, end, new_text)| {
                source.get(*start..*end) != Some(new_text.as_str())
            });
            replacements.sort_by_key(|(start, end, _)| (*start, *end));
            // Edits that, made together, leave the text as it is (as when an item moves past one
            // just like it) change nothing either.
            if !replacements.is_empty() && applied(source, &replacements) == source {
                replacements.clear();
            }
            let index = LineIndex::new(source);
            let edits = replacements
                .into_iter()
                .map(|(start, end, new_text)| TextEdit {
                    range: index.range_from_bytes(ByteTextRange::new(
                        TextSize::from(start as u32),
                        TextSize::from(end as u32),
                    )),
                    new_text,
                })
                .collect();
            SourceEditAnswer {
                edits,
                refused: None,
            }
        }
        Err(reason) => SourceEditAnswer::refused(reason),
    }
}

/// The text once the replacements, sorted and not overlapping, are made.
fn applied(source: &str, replacements: &[(usize, usize, String)]) -> String {
    let mut text = String::with_capacity(source.len());
    let mut from = 0;
    for (start, end, new_text) in replacements {
        text.push_str(&source[from..*start]);
        text.push_str(new_text);
        from = *end;
    }
    text.push_str(&source[from..]);
    text
}

/// A replacement of the bytes `start..end`, or nothing to do.
type Replacement = Result<Option<(usize, usize, String)>, SourceEditRefusal>;

/// Replacements of bytes in the document as sent, none overlapping, in any order.
type Replacements = Result<Vec<(usize, usize, String)>, SourceEditRefusal>;

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
    let Some(written) = source.get(start..end) else {
        return Err(SourceEditRefusal::NotEditable);
    };
    if written.starts_with('#') != matches!(value, SourceEditValue::Color { .. }) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    let text = spell_value(value)?;
    let owner = parent(tree, node).map(|owner| owner.role);
    let braced = node.flags.contains(&SourceFlag::Braced);
    // A case written qualified, `{ Status.open }`, keeps its qualifier.
    if node.role == SourceRole::Case {
        if let Some(dot) = written.rfind('.') {
            if !matches!(value, SourceEditValue::Case { .. }) {
                return Err(SourceEditRefusal::InvalidValue);
            }
            return Ok(Some((start + dot + 1, end, text)));
        }
    }
    // Only a value written unbraced after `=` takes a minus sign or a bare case name: in braces or
    // in a list, `-3` is an operator and `large` a reference to a name.
    let unbraced =
        matches!(owner, Some(SourceRole::Attribute | SourceRole::Declaration)) && !braced;
    let signed = matches!(value, SourceEditValue::Number { text } if text.starts_with('-'));
    if !unbraced && (signed || matches!(value, SourceEditValue::Case { .. })) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    Ok(Some((start, end, text)))
}

/// Spells a value as NX source, or refuses one that cannot be spelled.
pub(crate) fn spell_value(value: &SourceEditValue) -> Result<String, SourceEditRefusal> {
    match value {
        SourceEditValue::String { value } => Ok(quote_string(value)),
        SourceEditValue::Color { text } => {
            let Some(digits) = text.strip_prefix('#') else {
                return Err(SourceEditRefusal::InvalidValue);
            };
            if !matches!(digits.len(), 3 | 4 | 6 | 8)
                || !digits.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(SourceEditRefusal::InvalidValue);
            }
            let expanded = if digits.len() <= 4 {
                digits.chars().flat_map(|ch| [ch, ch]).collect::<String>()
            } else {
                digits.to_string()
            };
            Ok(format!("#{}", expanded.to_ascii_lowercase()))
        }
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
    // The space a run ends with, as before a braced value or a tag, separates it from what follows
    // and stays: the new text takes the place of the words.
    let written = &source[start..end];
    let words = written.trim_end();
    let trail = &written[words.len()..];
    // Element content has no escapes: a run is the text as written, and what would start a tag,
    // a braced value or an `if`, `for` or `else` block cannot be in one. So the text is written as
    // it is, and kept only when the document then reads it back as one run of exactly that text.
    let run = format!("{text}{trail}");
    if !reads_as_one_run(source, start, end, &run) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    Ok(Some((start, start + words.len(), text.to_string())))
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

/// Removes an item of a list, as [`remove_item`] does, or an attribute, as [`remove_attribute`]
/// does.
fn remove(source: &str, tree: &SourceTree, at: usize) -> Replacements {
    if tree.nodes[at].role == SourceRole::Element {
        return remove_item(source, tree, at);
    }
    remove_attribute(source, tree, at).map(Vec::from_iter)
}

/// Removes an attribute with the space before it and a line comment after it, or its whole line when
/// it stands alone there. A comment on the line above stays: it may be about more than the attribute.
fn remove_attribute(source: &str, tree: &SourceTree, at: usize) -> Replacement {
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
    // The space after it, up to the next token or the end of its line. A line comment after it is
    // about it, and goes with it.
    let mut after = end;
    while after < bytes.len() && is_blank(bytes[after]) {
        after += 1;
    }
    let commented = source[after..].starts_with("//");
    if commented {
        after = source[after..]
            .find(['\r', '\n'])
            .map_or(source.len(), |offset| after + offset);
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
    Ok(Some((
        before,
        if commented { after } else { end },
        String::new(),
    )))
}

/// How a list writes its items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListLayout {
    /// Every item stands on lines of its own: an item is its whole lines, with the comments
    /// directly above it.
    Lines,
    /// Every item is on one line, apart from the others by blank space only.
    OneLine,
}

/// The list an item belongs to, read from the document.
struct ItemList {
    /// Each item's span, in order: its whole lines, line breaks included, with the comments
    /// directly above it, in a list written an item to a line; the item alone on one line.
    spans: Vec<(usize, usize)>,
    /// The item's position in the list.
    position: usize,
    layout: ListLayout,
}

/// The list the element at `at` is an item of: a sequence or element content, not a condition's
/// branch or a loop's body, that holds no text and is written an item to a line or on one line.
fn item_list(source: &str, tree: &SourceTree, at: usize) -> Result<ItemList, SourceEditRefusal> {
    let node = &tree.nodes[at];
    let owner_index = node
        .parent
        .filter(|_| node.role == SourceRole::Element)
        .ok_or(SourceEditRefusal::NotEditable)?;
    let owner = &tree.nodes[owner_index as usize];
    let is_list = match owner.role {
        SourceRole::Sequence | SourceRole::Element => true,
        SourceRole::Attribute => owner.flags.contains(&SourceFlag::Content),
        _ => false,
    };
    if !is_list || item_position(&owner.key, &node.key).is_none() {
        return Err(SourceEditRefusal::NotEditable);
    }
    let mut items = Vec::new();
    for (index, child) in tree.nodes.iter().enumerate() {
        if child.parent != Some(owner_index) {
            continue;
        }
        // Text between items is part of what the list reads as; an item does not move past it.
        if child.role == SourceRole::Text {
            return Err(SourceEditRefusal::NotEditable);
        }
        if let Some(position) = item_position(&owner.key, &child.key) {
            items.push((position, index));
        }
    }
    items.sort_unstable();
    let position = items
        .iter()
        .position(|&(_, index)| index == at)
        .ok_or(SourceEditRefusal::NotEditable)?;
    let spans = items
        .iter()
        .map(|&(_, index)| byte_span(&tree.nodes[index]))
        .collect::<Vec<_>>();
    if let Some(lines) = spans
        .iter()
        .map(|&(start, end)| item_lines(source, tree, start, end))
        .collect::<Option<Vec<_>>>()
    {
        return Ok(ItemList {
            spans: lines,
            position,
            layout: ListLayout::Lines,
        });
    }
    let one_line = !source[spans[0].0..spans[spans.len() - 1].1].contains('\n')
        && spans
            .windows(2)
            .all(|pair| is_blank(&source[pair[0].1..pair[1].0]));
    if one_line {
        return Ok(ItemList {
            spans,
            position,
            layout: ListLayout::OneLine,
        });
    }
    Err(SourceEditRefusal::NotEditable)
}

/// The position an item's key gives it in the list keyed `list`, as `steps[2]` gives `2`.
fn item_position(list: &str, key: &str) -> Option<usize> {
    key.strip_prefix(list)?
        .strip_prefix('[')?
        .strip_suffix(']')?
        .parse()
        .ok()
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

fn line_start(source: &str, at: usize) -> usize {
    source[..at].rfind('\n').map_or(0, |line| line + 1)
}

/// The whole lines of an item that stands on lines of its own, with the comments directly above
/// it, from the start of its first line to past the line break of its last; none when something
/// else shares a line with it, other than a line comment after it.
fn item_lines(source: &str, tree: &SourceTree, start: usize, end: usize) -> Option<(usize, usize)> {
    let first = line_start(source, start);
    if !is_blank(&source[first..start]) {
        return None;
    }
    let rest = &source[end..];
    let mut after = end + (rest.len() - rest.trim_start_matches([' ', '\t']).len());
    if source[after..].starts_with("//") {
        after = source[after..]
            .find(['\r', '\n'])
            .map_or(source.len(), |offset| after + offset);
    }
    if source[after..].starts_with("\r\n") {
        after += 2;
    } else if source[after..].starts_with('\n') {
        after += 1;
    } else {
        return None;
    }
    Some((comments_above(source, tree, first), after))
}

/// The start of the first of the comment lines directly above the line starting at `line`, with no
/// blank line between: each a comment that starts its line and ends the next one up.
fn comments_above(source: &str, tree: &SourceTree, line: usize) -> usize {
    let mut start = line;
    while start > 0 {
        let above_end = source[..start - 1]
            .strip_suffix('\r')
            .map_or(start - 1, str::len);
        let above_start = line_start(source, above_end);
        let above = &source[above_start..above_end];
        if is_blank(above) {
            break;
        }
        let text_end = above_start + above.trim_end().len();
        let comment = tree.nodes.iter().find(|node| {
            let (comment_start, comment_end) = byte_span(node);
            matches!(node.role, SourceRole::Comment | SourceRole::DocComment)
                && comment_start + source[comment_start..comment_end].trim_end().len() == text_end
                && is_blank(&source[line_start(source, comment_start)..comment_start])
        });
        match comment {
            Some(comment) => start = line_start(source, byte_span(comment).0),
            None => break,
        }
    }
    start
}

/// What taking the item at `position` out of its list removes: its span and the blank space that
/// separates it from a neighbour, and that space, which goes with the item where it is put back.
/// Written an item to a line, the space is the one after the item, or before it when it is the
/// last; on one line, the one before it, or after it when it is the first.
fn cut(source: &str, list: &ItemList, position: usize) -> (usize, usize, String) {
    let (start, end) = list.spans[position];
    let after = || {
        let &(next, _) = list.spans.get(position + 1)?;
        is_blank(&source[end..next]).then(|| (start, next, source[end..next].to_string()))
    };
    let before = || {
        let &(_, previous) = list.spans.get(position.checked_sub(1)?)?;
        is_blank(&source[previous..start])
            .then(|| (previous, end, source[previous..start].to_string()))
    };
    let found = match list.layout {
        ListLayout::Lines => after().or_else(before),
        ListLayout::OneLine => before().or_else(after),
    };
    if let Some(found) = found {
        return found;
    }
    // Lines apart from their neighbours by more than blank lines: the item's lines go, and of two
    // blank lines that would then stand together, one.
    let mut end = end;
    if list.layout == ListLayout::Lines && start > 0 {
        let above = line_start(source, start - 1);
        let below_end = source[end..].find('\n').map(|offset| end + offset + 1);
        if let Some(below_end) = below_end {
            if is_blank(&source[above..start]) && is_blank(&source[end..below_end]) {
                end = below_end;
            }
        }
    }
    (start, end, String::new())
}

/// Removes an element that is an item of a list: its lines, the comments directly above it and the
/// blank lines after it (before it, when it is the last), or on one line, the item and the space
/// before it, or after it when it is the first.
fn remove_item(source: &str, tree: &SourceTree, at: usize) -> Replacements {
    let list = item_list(source, tree, at)?;
    let (start, end, _) = cut(source, &list, list.position);
    Ok(vec![(start, end, String::new())])
}

/// Moves an element that is an item of a list to the position `to` in that list: its lines, with
/// the comments directly above it, before the item that will follow it or after the last; on one
/// line, the item and the space that separated it.
fn move_item(source: &str, tree: &SourceTree, at: usize, to: usize) -> Replacements {
    let list = item_list(source, tree, at)?;
    let from = list.position;
    if to >= list.spans.len() {
        return Err(SourceEditRefusal::InvalidValue);
    }
    if to == from {
        return Ok(Vec::new());
    }
    let (start, end, space) = cut(source, &list, from);
    let (item_start, item_end) = list.spans[from];
    let item = &source[item_start..item_end];
    let insert = if to < from {
        (list.spans[to].0, format!("{item}{space}"))
    } else {
        (list.spans[to].1, format!("{space}{item}"))
    };
    Ok(vec![
        (start, end, String::new()),
        (insert.0, insert.0, insert.1),
    ])
}

/// Adds the attribute `name=value` to an element, after its last attribute and in its layout: on
/// the tag's line when it writes its attributes there or has none, and on a line of its own at
/// their indentation when it writes them a line each.
fn add_property(
    source: &str,
    tree: &SourceTree,
    at: usize,
    name: &str,
    value: &SourceEditValue,
) -> Replacement {
    let node = &tree.nodes[at];
    if node.role != SourceRole::Element {
        return Err(SourceEditRefusal::NotEditable);
    }
    // The property must be one the element's declaration has, or its union case's, other than its
    // content, and one it does not write yet, even under a condition.
    let declared = element_properties(tree, node).and_then(|properties| {
        properties
            .into_iter()
            .find(|property| property.name == name)
    });
    let Some(declared) = declared else {
        return Err(SourceEditRefusal::InvalidValue);
    };
    if declared.flags.contains(&SourceFlag::Content) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    if tree.nodes.iter().any(|candidate| {
        candidate.role == SourceRole::Attribute
            && candidate.name.as_deref() == Some(name)
            && attribute_element(tree, candidate) == Some(at)
    }) {
        return Err(SourceEditRefusal::InvalidValue);
    }
    // The new attribute goes after the element's last own attribute, not one under a condition.
    let attributes = tree
        .nodes
        .iter()
        .filter(|candidate| {
            candidate.parent == Some(at as u32)
                && candidate.role == SourceRole::Attribute
                && !candidate.flags.contains(&SourceFlag::Content)
        })
        .collect::<Vec<_>>();
    // An attribute's value is spelled as `setValue` spells it there.
    let attribute = format!("{name}={}", spell_value(value)?);
    let (element_start, _) = byte_span(node);
    let bytes = source.as_bytes();
    let line_start = |at: usize| source[..at].rfind('\n').map_or(0, |line| line + 1);

    let Some(last) = attributes
        .iter()
        .max_by_key(|attribute| attribute.range.end_byte)
    else {
        // No attributes: after the tag's name.
        let after_name =
            tag_name_end(source, element_start).ok_or(SourceEditRefusal::NotEditable)?;
        return Ok(Some((after_name, after_name, format!(" {attribute}"))));
    };
    let (last_start, last_end) = byte_span(last);
    let own_line = source[line_start(last_start)..last_start].trim().is_empty();
    if !own_line {
        return Ok(Some((last_end, last_end, format!(" {attribute}"))));
    }
    // A line each: a new line at the last attribute's indentation, after that line's comment when
    // nothing else follows it there, and otherwise right after the attribute.
    let indent = &source[line_start(last_start)..last_start];
    let line_end = source[last_end..]
        .find(['\r', '\n'])
        .map_or(source.len(), |offset| last_end + offset);
    let rest = source[last_end..line_end].trim();
    let insert_at = if rest.is_empty() || rest.starts_with("//") {
        line_end
    } else {
        last_end
    };
    let newline = if source[..line_end].ends_with('\r') || bytes.get(line_end) == Some(&b'\r') {
        "\r\n"
    } else {
        "\n"
    };
    Ok(Some((
        insert_at,
        insert_at,
        format!("{newline}{indent}{attribute}"),
    )))
}

/// The properties an element's declaration has: a record's, an action's or a component's, or for an
/// element of a union's case, the union's shared ones and then the case's.
pub(crate) fn element_properties<'t>(
    tree: &'t SourceTree,
    element: &SourceNode,
) -> Option<Vec<&'t SourceProperty>> {
    let declaration = tree.declarations.get(element.declaration? as usize)?;
    let mut properties = declaration.properties.iter().collect::<Vec<_>>();
    if declaration.kind == SourceDeclarationKind::Union {
        let written = element.name.as_deref()?;
        let case = written.rsplit('.').next()?;
        let case = declaration
            .cases
            .iter()
            .find(|candidate| candidate.name == case)?;
        properties.extend(case.properties.iter());
    }
    Some(properties)
}

/// The element an attribute sets a property of: its parent, or the element whose property-list
/// condition or match it is written under.
pub(crate) fn attribute_element(tree: &SourceTree, attribute: &SourceNode) -> Option<usize> {
    let mut current = attribute.parent?;
    loop {
        let node = tree.nodes.get(current as usize)?;
        match node.role {
            SourceRole::Element => return Some(current as usize),
            SourceRole::Attribute | SourceRole::Declaration => return None,
            _ => current = node.parent?,
        }
    }
}

/// The byte just past an element's tag name, read from the syntax tree.
fn tag_name_end(source: &str, element_start: usize) -> Option<usize> {
    let parsed = parse_str(source, "edit.nx");
    let syntax = parsed.tree?;
    fn find(node: SyntaxNode<'_>, start: usize) -> Option<usize> {
        if node.kind() == SyntaxKind::ELEMENT && usize::from(node.span().start()) == start {
            return node
                .children()
                .find(|child| child.kind() == SyntaxKind::ELEMENT_NAME)
                .map(|name| usize::from(name.span().end()));
        }
        node.children()
            .filter(|child| {
                let span = child.span();
                usize::from(span.start()) <= start && start < usize::from(span.end())
            })
            .find_map(|child| find(child, start))
    }
    find(syntax.root(), element_start)
}

fn set_constructor_color(
    source: &str,
    tree: &SourceTree,
    at: usize,
    text: &str,
) -> Result<Vec<(usize, usize, String)>, SourceEditRefusal> {
    let node = &tree.nodes[at];
    if !is_whole_value(tree, at) || node.color.is_none() {
        return Err(SourceEditRefusal::NotEditable);
    }
    let declaration = node
        .declaration
        .and_then(|index| tree.declarations.get(index as usize))
        .ok_or(SourceEditRefusal::NotEditable)?;
    if !matches!(
        declaration.module.as_str(),
        "@nx/color/color.nx" | "@nx/color/operations.nx" | "@nx/graphics/graphics.nx"
    ) {
        return Err(SourceEditRefusal::NotEditable);
    }
    let parsed = nx_value::color::parse(text).map_err(|_| SourceEditRefusal::InvalidValue)?;
    let channels =
        nx_value::color::channels(&parsed).map_err(|_| SourceEditRefusal::InvalidValue)?;
    if node.role == SourceRole::Element && declaration.name == "Color" {
        let mut replacements = Vec::new();
        for (i, name) in ["r", "g", "b", "a"].iter().enumerate() {
            let attribute = tree.nodes.iter().position(|candidate| {
                candidate.parent == Some(at as u32)
                    && candidate.role == SourceRole::Attribute
                    && candidate.name.as_deref() == Some(*name)
            });
            if let Some(attribute) = attribute {
                let literal = tree
                    .nodes
                    .iter()
                    .find(|candidate| {
                        candidate.parent == Some(attribute as u32)
                            && candidate.role == SourceRole::Literal
                    })
                    .ok_or(SourceEditRefusal::NotEditable)?;
                let (start, end) = byte_span(literal);
                let old = source[start..end]
                    .parse::<f64>()
                    .map_err(|_| SourceEditRefusal::NotEditable)?;
                if (old * 255.0).round() != (channels[i] * 255.0).round() {
                    replacements.push((start, end, (channels[i] as f32).to_string()));
                }
            } else if i == 3 && channels[3] != 1.0 {
                replacements.extend(add_property(
                    source,
                    tree,
                    at,
                    "a",
                    &SourceEditValue::Number {
                        text: (channels[3] as f32).to_string(),
                    },
                )?);
            }
        }
        return Ok(replacements);
    }
    let args: Vec<_> = tree
        .nodes
        .iter()
        .filter(|candidate| {
            candidate.parent == Some(at as u32) && candidate.role == SourceRole::Literal
        })
        .collect();
    if declaration.name == "parse" && args.len() == 1 {
        let (start, end) = byte_span(args[0]);
        let original_text = serde_json::from_str::<String>(&source[start..end])
            .map_err(|_| SourceEditRefusal::NotEditable)?;
        let original =
            nx_value::color::parse(&original_text).map_err(|_| SourceEditRefusal::NotEditable)?;
        let mut merged =
            nx_value::color::channels(&original).map_err(|_| SourceEditRefusal::NotEditable)?;
        for i in 0..4 {
            if (merged[i] * 255.0).round() != (channels[i] * 255.0).round() {
                merged[i] = channels[i];
            }
        }
        let bytes = merged.map(|channel| (channel * 255.0).round() as u8);
        if merged
            .iter()
            .zip(bytes)
            .all(|(channel, byte)| *channel as f32 == (byte as f64 / 255.0) as f32)
        {
            let original = original_text.trim_ascii();
            let explicit_alpha = original.starts_with('#') && matches!(original.len(), 5 | 9);
            let mut hex = format!("#{:02x}{:02x}{:02x}", bytes[0], bytes[1], bytes[2]);
            if bytes[3] != 255 || explicit_alpha {
                hex.push_str(&format!("{:02x}", bytes[3]));
            }
            return Ok(vec![(start, end, quote_string(&hex))]);
        }
        let value = nx_value::color::make(merged).map_err(|_| SourceEditRefusal::InvalidValue)?;
        let op = nx_value::color::NAMES
            .iter()
            .position(|name| *name == "toCss")
            .unwrap();
        let nx_value::NxValue::String(css) = nx_value::color::evaluate(op as u32, &[value])
            .map_err(|_| SourceEditRefusal::InvalidValue)?
        else {
            return Err(SourceEditRefusal::InvalidValue);
        };
        return Ok(vec![(start, end, quote_string(&css))]);
    }
    if !matches!(declaration.name.as_str(), "rgb" | "rgba" | "srgb")
        || !(3..=4).contains(&args.len())
    {
        return Err(SourceEditRefusal::NotEditable);
    }
    let normalized = declaration.name == "srgb";
    let mut edits = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let (start, end) = byte_span(arg);
        let previous = source[start..end]
            .parse::<f64>()
            .map_err(|_| SourceEditRefusal::NotEditable)?;
        let scale = if i == 3 || normalized { 1.0 } else { 255.0 };
        // A byte picker cannot express existing sub-byte precision. Leave a channel alone when
        // its displayed byte did not change, including an unchanged alpha.
        if (previous / scale * 255.0).round() == (channels[i] * 255.0).round() {
            continue;
        }
        edits.push((
            start,
            end,
            if scale == 255.0 {
                (channels[i] * 255.0).round().to_string()
            } else {
                (channels[i] as f32).to_string()
            },
        ));
    }
    if args.len() == 3 && channels[3] != 1.0 {
        if declaration.name == "rgb" {
            return Err(SourceEditRefusal::NotEditable);
        }
        let (_, end) = byte_span(args[2]);
        edits.push((end, end, format!(", {}", channels[3] as f32)));
    }
    Ok(edits)
}
