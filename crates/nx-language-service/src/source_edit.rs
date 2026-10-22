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
    /// Remove an attribute.
    Remove,
    /// Add a property an element does not write, as an attribute in the element's layout.
    AddProperty {
        /// The property's name.
        name: String,
        /// Its value.
        value: SourceEditValue,
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
    /// A reference to a named value, written as its path: in braces as an attribute's value, and
    /// bare elsewhere.
    Reference {
        /// The path, identifiers joined by dots, such as `theme.accent`.
        path: String,
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
        SourceEdit::SetValue { value } => set_value(source, tree, at, value),
        SourceEdit::SetText { text } => set_text(source, tree, at, text),
        SourceEdit::Remove => remove(source, tree, at),
        SourceEdit::AddProperty { name, value } => add_property(source, tree, at, name, value),
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
    let (start, end) = byte_span(node);
    let Some(written) = source.get(start..end) else {
        return Err(SourceEditRefusal::NotEditable);
    };
    let is_reference = matches!(node.role, SourceRole::Reference | SourceRole::Member)
        && is_reference_path(written);
    if !(matches!(node.role, SourceRole::Literal | SourceRole::Case) || is_reference)
        || !is_whole_value(tree, at)
    {
        return Err(SourceEditRefusal::NotEditable);
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
    if let SourceEditValue::Reference { .. } = value {
        // An attribute's value needs braces around a reference; braces already written stay.
        let text = if owner == Some(SourceRole::Attribute) && !braced {
            format!("{{{text}}}")
        } else {
            text
        };
        return Ok(Some((start, end, text)));
    }
    // A literal or a case over a braced reference at an attribute or a `let` is written as the place
    // usually spells it, its braces dropped.
    if is_reference
        && braced
        && matches!(owner, Some(SourceRole::Attribute | SourceRole::Declaration))
    {
        if let Some((open, close)) = enclosing_braces(source, start, end) {
            return Ok(Some((open, close, text)));
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

/// The bytes from the `{` before `start` to just past the `}` after `end`, when only blanks lie
/// between them and the value.
fn enclosing_braces(source: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let before = source[..start].trim_end();
    let after = source[end..].trim_start();
    if !before.ends_with('{') || !after.starts_with('}') {
        return None;
    }
    let open = before.len() - 1;
    let close = source.len() - after.len() + 1;
    Some((open, close))
}

/// Whether `text` is a reference path: identifiers joined by dots, with nothing between them.
pub(crate) fn is_reference_path(text: &str) -> bool {
    !text.is_empty()
        && text
            .split('.')
            .all(|part| is_identifier(part) && part != "true" && part != "false")
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
        SourceEditValue::Reference { path } => {
            if is_reference_path(path) {
                Ok(path.clone())
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

/// Removes an attribute with the space before it and a line comment after it, or its whole line when
/// it stands alone there. A comment on the line above stays: it may be about more than the attribute.
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
    // An attribute's value is spelled as `setValue` spells it there: unbraced, with braces around a
    // reference.
    let spelled = spell_value(value)?;
    let spelled = match value {
        SourceEditValue::Reference { .. } => format!("{{{spelled}}}"),
        _ => spelled,
    };
    let attribute = format!("{name}={spelled}");
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
