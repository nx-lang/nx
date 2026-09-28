//! NX values spelled as NX source, and what each part of that text is.
//!
//! <para>Every field of a record is emitted in property position as `key=value`. NX has no
//! property-element syntax — an element body binds to the single field marked `is_content`, which
//! is schema information no [`Value`] carries — so a property name written in body position has
//! nowhere to go and cannot be read back. Rendering is therefore uniform: the simple/complex split
//! decides line layout only, never whether a field becomes body content.</para>
//!
//! <para>The walk that writes the text also records a node for each value, property and sequence
//! it writes: where it starts and ends, and what it is. [`format_nx_text`] discards them, and
//! [`NxValueText`] keeps them for a viewer. Nothing else decides layout, so annotated text is
//! always the text the command line prints.</para>

use crate::diagnostics::{text_range_to_span, LineIndex, NxTextSpan};
use nx_hir::{ast::spell_type_ref, Item, LoweredModule, Name, RecordField, UnionDef};
use nx_interpreter::Value;
use rustc_hash::FxHashMap;
use serde::Serialize;
use smol_str::SmolStr;
use std::fmt::Write;
use text_size::TextRange;

/// A value spelled as NX text, with a node for each value, property and sequence in it.
///
/// <para>Nodes are in text order, and each parent comes before its children. Offsets count
/// UTF-16 code units, because the reader is JavaScript, whose strings index that way.</para>
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NxValueText {
    /// The value's NX spelling, as `nxlang run` prints it.
    pub text: String,
    /// What each part of `text` is.
    pub nodes: Vec<NxValueNode>,
}

/// What one range of an [`NxValueText`] is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NxValueNode {
    /// First UTF-16 code unit of the node's text.
    pub start: u32,
    /// UTF-16 code unit one past the node's text.
    pub end: u32,
    /// Index of the enclosing node, or `None` at the top.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    /// What kind of thing the node is.
    pub role: NxValueRole,
    /// The node's type, spelled in NX. A property's is its declared type; an empty value and a
    /// function value have none.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// A property's name, or a function value's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether a property is declared optional (`subtitle?:string`).
    #[serde(skip_serializing_if = "is_false")]
    pub optional: bool,
    /// A sequence's length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Where the entry module declares what the node is: its record or component, union case,
    /// property or function. `None` when the entry module does not declare it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<NxTextSpan>,
}

/// The kinds of thing an [`NxValueNode`] can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NxValueRole {
    /// A record, written as an element.
    Record,
    /// One `name=value` property of a record.
    Property,
    /// A sequence with at least one item.
    Sequence,
    /// A constant union case.
    Case,
    /// A number, string or boolean.
    Scalar,
    /// A function value.
    Function,
    /// The empty value, `{}`.
    Empty,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Pretty prints a value as NX source.
///
/// <para>Fails for a value with no NX spelling rather than emitting output that does not read
/// back. Two values are in that position: [`Value::ActionHandler`], which has no source form at
/// all, and a list nested directly inside another list, which has none because a braced value is
/// not an item of a braced value.</para>
///
/// <para>An empty list is not among them — it is emitted as `{}`, on its own and in property
/// position, at every list-typed site, including one whose declared default is itself empty, so
/// that rendering never depends on reasoning about defaults. Emptiness is not what decides the
/// nested case: `{{"a"}}` is ungrammatical, and `{{}}` reads back as the empty value rather than a
/// list holding one.</para>
pub fn format_nx_text(value: &Value) -> Result<String, String> {
    let mut writer = Writer::default();
    writer.value(value, 0)?;
    Ok(writer.output)
}

/// Spells a value as NX text and annotates it, taking types and declarations from `module`, whose
/// source is `source`.
pub(crate) fn annotate_nx_text(
    value: &Value,
    module: &LoweredModule,
    source: &str,
) -> Result<NxValueText, String> {
    let mut writer = Writer::default();
    writer.value(value, 0)?;
    let declarations = Declarations::new(module, source);
    let utf16 = Utf16Offsets::new(&writer.output);
    let nodes = writer
        .nodes
        .into_iter()
        .map(|node| declarations.annotate(node, &utf16))
        .collect();
    Ok(NxValueText {
        text: writer.output,
        nodes,
    })
}

/// A node as the writer records it: byte offsets, and a key to look its declaration up by.
struct RawNode {
    start: usize,
    end: usize,
    parent: Option<usize>,
    role: NxValueRole,
    ty: Option<String>,
    count: Option<usize>,
    key: DeclarationKey,
}

/// What a node's declaration would be, if the entry module declares it.
enum DeclarationKey {
    None,
    Record(Name),
    Property { owner: Name, name: SmolStr },
    Case { union: Name, case: SmolStr },
    Function(SmolStr),
}

/// Writes NX text and records a node for each part of it.
#[derive(Default)]
struct Writer {
    output: String,
    nodes: Vec<RawNode>,
    open: Vec<usize>,
}

impl Writer {
    fn begin(
        &mut self,
        role: NxValueRole,
        ty: Option<String>,
        count: Option<usize>,
        key: DeclarationKey,
    ) {
        let index = self.nodes.len();
        self.nodes.push(RawNode {
            start: self.output.len(),
            end: self.output.len(),
            parent: self.open.last().copied(),
            role,
            ty,
            count,
            key,
        });
        self.open.push(index);
    }

    fn end(&mut self) {
        let index = self.open.pop().expect("every node that begins also ends");
        self.nodes[index].end = self.output.len();
    }

    /// Writes a value at the top level, where a sequence is a run of values, one per line.
    fn value(&mut self, value: &Value, indent: usize) -> Result<(), String> {
        match value {
            Value::Int32(_)
            | Value::Int(_)
            | Value::Float32(_)
            | Value::Float(_)
            | Value::Boolean(_) => {
                self.begin(
                    NxValueRole::Scalar,
                    value_type(value),
                    None,
                    DeclarationKey::None,
                );
                write_number_or_boolean(value, &mut self.output);
                self.end();
            }
            Value::String(s) => {
                self.begin(
                    NxValueRole::Scalar,
                    value_type(value),
                    None,
                    DeclarationKey::None,
                );
                self.output.push_str(s.as_str());
                self.end();
            }

            // A constant union case names its union, exactly as the qualified source form does.
            Value::UnionCase { union, case } => {
                self.begin(
                    NxValueRole::Case,
                    Some(union.as_str().to_string()),
                    None,
                    case_key(union, case),
                );
                write!(self.output, "{}.{}", union, case).unwrap();
                self.end();
            }

            // A function value is a reference to a declaration; its source spelling is the name.
            Value::Function { name, .. } => {
                self.begin(
                    NxValueRole::Function,
                    None,
                    None,
                    DeclarationKey::Function(name.clone()),
                );
                self.output.push_str(name.as_str());
                self.end();
            }

            // A top-level sequence is a run of values, one per line -- except an empty one, which
            // has no lines to be a run of. Emitting nothing there would read back as no value
            // rather than as the empty list, so the braced spelling is used, the same one property
            // position uses.
            Value::Array(elements) if elements.is_empty() => {
                self.begin(NxValueRole::Empty, None, None, DeclarationKey::None);
                self.output.push_str("{}");
                self.end();
            }
            Value::Array(elements) => {
                self.begin(
                    NxValueRole::Sequence,
                    Some(sequence_type(elements)),
                    Some(elements.len()),
                    DeclarationKey::None,
                );
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push('\n');
                    }
                    // One value per line cannot say where an inner list ends, so a list of lists
                    // would render as the flattened run of its elements and read back as a
                    // different value. It has no spelling here for the same reason it has none in
                    // property position, and is reported the same way.
                    if matches!(elem, Value::Array(_)) {
                        return Err(unspellable_nested_list());
                    }
                    self.value(elem, indent)?;
                }
                self.end();
            }

            Value::Record { type_name, fields } => {
                self.record(type_name, fields, indent)?;
            }
            Value::ActionHandler { .. } => return Err(unspellable_action_handler()),
        }

        Ok(())
    }

    /// Emits a record as an element whose every field is a property.
    fn record(
        &mut self,
        tag_name: &Name,
        fields: &FxHashMap<SmolStr, Value>,
        indent: usize,
    ) -> Result<(), String> {
        // Sorted for deterministic output.
        let mut field_vec: Vec<_> = fields.iter().collect();
        field_vec.sort_by_key(|(k, _)| k.as_str());

        self.begin(
            NxValueRole::Record,
            Some(tag_name.as_str().to_string()),
            None,
            DeclarationKey::Record(tag_name.clone()),
        );
        write!(self.output, "<{}", tag_name).unwrap();

        if field_vec.iter().any(|(_, value)| is_complex_value(value)) {
            // One property per line. This is layout only — every field is still a property.
            let property_indent = indent + 2;
            for (key, value) in &field_vec {
                self.output.push('\n');
                write!(self.output, "{:width$}", "", width = property_indent).unwrap();
                self.property(tag_name, key, value, property_indent)?;
            }
            self.output.push('\n');
            write!(self.output, "{:width$}/>", "", width = indent).unwrap();
        } else {
            for (key, value) in &field_vec {
                self.output.push(' ');
                self.property(tag_name, key, value, indent)?;
            }
            self.output.push_str(" />");
        }

        self.end();
        Ok(())
    }

    /// Emits one `key=value` property of the record `owner`.
    fn property(
        &mut self,
        owner: &Name,
        key: &SmolStr,
        value: &Value,
        indent: usize,
    ) -> Result<(), String> {
        // The value's own type stands in until the declaration, if there is one, replaces it.
        self.begin(
            NxValueRole::Property,
            value_type(value),
            None,
            DeclarationKey::Property {
                owner: owner.clone(),
                name: key.clone(),
            },
        );
        self.output.push_str(key.as_str());
        self.output.push('=');
        self.property_value(value, indent)?;
        self.end();
        Ok(())
    }

    /// Emits one value in property position, in a form that reads back at a typed site.
    ///
    /// <para>A number, string, boolean or empty value written directly as a property's value gets
    /// no node of its own: the property's node already says everything about it, and hovering the
    /// value should explain the property.</para>
    fn property_value(&mut self, value: &Value, indent: usize) -> Result<(), String> {
        match value {
            Value::String(s) => write!(self.output, "\"{}\"", escape_string(s.as_str())).unwrap(),
            Value::Int32(_)
            | Value::Int(_)
            | Value::Float32(_)
            | Value::Float(_)
            | Value::Boolean(_) => write_number_or_boolean(value, &mut self.output),
            // A bare case name; the declaring union comes from the target type.
            Value::UnionCase { union, case } => {
                self.begin(
                    NxValueRole::Case,
                    Some(union.as_str().to_string()),
                    None,
                    case_key(union, case),
                );
                self.output.push_str(case.as_str());
                self.end();
            }
            // A function value is bound by naming the declaration, `Row={ContactRow}`.
            Value::Function { name, .. } => {
                self.begin(
                    NxValueRole::Function,
                    None,
                    None,
                    DeclarationKey::Function(name.clone()),
                );
                write!(self.output, "{{{}}}", name.as_str()).unwrap();
                self.end();
            }
            // `rhs_expression` admits an element, so a record value needs no braces.
            Value::Record { type_name, fields } => self.record(type_name, fields, indent)?,
            Value::Array(elements) if elements.is_empty() => self.output.push_str("{}"),
            // A sequence needs them.
            Value::Array(elements) => {
                self.begin(
                    NxValueRole::Sequence,
                    Some(sequence_type(elements)),
                    Some(elements.len()),
                    DeclarationKey::None,
                );
                self.output.push('{');
                for (i, element) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push(' ');
                    }
                    // A brace inside a brace does not parse, so `{{"a"}}` is a syntax error; `{{}}`
                    // parses, but as the empty value, since a sequence is flat. A list of lists
                    // therefore has no NX spelling at any depth, empty or not, and is reported
                    // rather than rendered as source that would not read back as itself.
                    if matches!(element, Value::Array(_)) {
                        return Err(unspellable_nested_list());
                    }
                    // An item is not a property's value, so a scalar item is a node of its own.
                    let scalar = is_scalar(element);
                    if scalar {
                        self.begin(
                            NxValueRole::Scalar,
                            value_type(element),
                            None,
                            DeclarationKey::None,
                        );
                    }
                    self.property_value(element, indent)?;
                    if scalar {
                        self.end();
                    }
                }
                self.output.push('}');
                self.end();
            }
            Value::ActionHandler { .. } => return Err(unspellable_action_handler()),
        }

        Ok(())
    }
}

fn write_number_or_boolean(value: &Value, output: &mut String) {
    match value {
        Value::Int32(n) => write!(output, "{}", n).unwrap(),
        Value::Int(n) => write!(output, "{}", n).unwrap(),
        Value::Float32(f) => output.push_str(&format_real_literal(f.to_string(), f.is_finite())),
        Value::Float(f) => output.push_str(&format_real_literal(f.to_string(), f.is_finite())),
        Value::Boolean(b) => write!(output, "{}", b).unwrap(),
        _ => unreachable!("only numbers and booleans are written here"),
    }
}

fn case_key(union: &Name, case: &SmolStr) -> DeclarationKey {
    DeclarationKey::Case {
        union: union.clone(),
        case: case.clone(),
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Int32(_)
            | Value::Int(_)
            | Value::Float32(_)
            | Value::Float(_)
            | Value::String(_)
            | Value::Boolean(_)
    )
}

/// A value's type spelled in NX, as far as the value alone says.
fn value_type(value: &Value) -> Option<String> {
    match value {
        Value::Int32(_) => Some("int32".to_string()),
        Value::Int(_) => Some("int".to_string()),
        Value::Float32(_) => Some("float32".to_string()),
        Value::Float(_) => Some("float64".to_string()),
        Value::String(_) => Some("string".to_string()),
        Value::Boolean(_) => Some("boolean".to_string()),
        Value::UnionCase { union, .. } => Some(union.as_str().to_string()),
        Value::Record { type_name, .. } => Some(type_name.as_str().to_string()),
        Value::Array(elements) if !elements.is_empty() => Some(sequence_type(elements)),
        Value::Array(_) | Value::Function { .. } | Value::ActionHandler { .. } => None,
    }
}

/// `T*` for a sequence whose items all have the type `T`, and `object*` otherwise.
fn sequence_type(elements: &[Value]) -> String {
    let mut types = elements.iter().map(value_type);
    let first = types.next().flatten();
    match first {
        Some(first) if types.all(|ty| ty.as_deref() == Some(first.as_str())) => {
            format!("{}*", first)
        }
        _ => "object*".to_string(),
    }
}

/// What a record value can be an instance of: a record type, or a component, whose props are
/// declared the way a record's fields are.
#[derive(Clone, Copy)]
struct Shape<'a> {
    span: TextRange,
    fields: &'a [RecordField],
    base: Option<&'a Name>,
}

/// The entry module's records, components, unions and functions, by name, and a way to spell their
/// spans.
struct Declarations<'a> {
    shapes: FxHashMap<&'a str, Shape<'a>>,
    unions: FxHashMap<&'a str, &'a UnionDef>,
    functions: FxHashMap<&'a str, TextRange>,
    source: &'a str,
    lines: LineIndex,
}

impl<'a> Declarations<'a> {
    fn new(module: &'a LoweredModule, source: &'a str) -> Self {
        let mut shapes = FxHashMap::default();
        let mut unions = FxHashMap::default();
        let mut functions = FxHashMap::default();
        for item in module.items() {
            match item {
                Item::Record(record) => {
                    shapes.insert(
                        record.name.as_str(),
                        Shape {
                            span: record.span,
                            fields: &record.properties,
                            base: record.base.as_ref(),
                        },
                    );
                }
                // External components too: their props are declared in this source even though
                // their body lives in the host.
                Item::Component(component) => {
                    shapes.insert(
                        component.name.as_str(),
                        Shape {
                            span: component.span,
                            fields: &component.props,
                            base: component.base.as_ref(),
                        },
                    );
                }
                Item::Union(union) => {
                    unions.insert(union.name.as_str(), union);
                }
                Item::Function(function) => {
                    functions.insert(function.name.as_str(), function.span);
                }
                _ => {}
            }
        }
        Self {
            shapes,
            unions,
            functions,
            source,
            lines: LineIndex::new(source),
        }
    }

    fn span(&self, range: TextRange) -> Option<NxTextSpan> {
        // A declaration synthesized by lowering (the implicit `root`, for one) may carry an empty
        // span, which names nothing to show.
        (!range.is_empty()).then(|| text_range_to_span(range, self.source, &self.lines))
    }

    fn annotate(&self, node: RawNode, utf16: &Utf16Offsets) -> NxValueNode {
        let mut annotated = NxValueNode {
            start: utf16.at(node.start),
            end: utf16.at(node.end),
            parent: node.parent.map(|parent| parent as u32),
            role: node.role,
            ty: node.ty,
            name: None,
            optional: false,
            count: node.count.map(|count| count as u32),
            declaration: None,
        };

        match node.key {
            DeclarationKey::None => {}
            DeclarationKey::Record(name) => {
                annotated.declaration = self
                    .shape(&name)
                    .map(|shape| shape.span)
                    .or_else(|| self.payload_case(&name).map(|(_, case)| case))
                    .and_then(|range| self.span(range));
            }
            DeclarationKey::Property { owner, name } => {
                annotated.name = Some(name.to_string());
                if let Some(field) = self.field(&owner, &name) {
                    annotated.ty = Some(field.ty);
                    annotated.optional = field.optional;
                    annotated.declaration = self.span(field.span);
                }
            }
            DeclarationKey::Case { union, case } => {
                annotated.declaration = self
                    .unions
                    .get(union.as_str())
                    .and_then(|union| union.cases.iter().find(|c| c.name.as_str() == case))
                    .and_then(|case| self.span(case.span));
            }
            DeclarationKey::Function(name) => {
                annotated.name = Some(name.to_string());
                annotated.declaration = self
                    .functions
                    .get(name.as_str())
                    .and_then(|range| self.span(*range));
            }
        }

        annotated
    }

    fn shape(&self, name: &Name) -> Option<Shape<'a>> {
        self.shapes.get(name.as_str()).copied()
    }

    /// The union and the case a record type name like `LoadState.failed` names: a union case that
    /// declares fields, whose values are records.
    fn payload_case(&self, name: &Name) -> Option<(&'a UnionDef, TextRange)> {
        let (union, case) = name.as_str().split_once('.')?;
        let union = self.unions.get(union).copied()?;
        let case = union.cases.iter().find(|c| c.name.as_str() == case)?;
        Some((union, case.span))
    }

    /// The declared property `name` of the record type or component `owner`, its own or one it
    /// inherits.
    fn field(&self, owner: &Name, name: &str) -> Option<DeclaredField> {
        if let Some((union, _)) = self.payload_case(owner) {
            let case_name = owner.as_str().split_once('.')?.1;
            let case = union.cases.iter().find(|c| c.name.as_str() == case_name)?;
            return case
                .fields
                .iter()
                .find(|field| field.name.as_str() == name)
                .map(|field| DeclaredField {
                    ty: spell_type_ref(&field.ty),
                    optional: field.optional,
                    span: field.span,
                });
        }

        // Follow the base chain, guarding against a cycle the checker would already have
        // reported.
        let mut shape = self.shape(owner);
        let mut visited = 0;
        while let Some(current) = shape {
            if let Some(field) = current
                .fields
                .iter()
                .find(|field| field.name.as_str() == name)
            {
                return Some(DeclaredField {
                    ty: spell_type_ref(&field.ty),
                    optional: field.optional,
                    span: field.span,
                });
            }
            visited += 1;
            if visited > self.shapes.len() {
                return None;
            }
            shape = current.base.and_then(|base| self.shape(base));
        }
        None
    }
}

struct DeclaredField {
    ty: String,
    optional: bool,
    span: TextRange,
}

/// UTF-16 offsets for every byte offset of a text, so a node's byte range converts in O(1).
struct Utf16Offsets {
    offsets: Vec<u32>,
}

impl Utf16Offsets {
    fn new(text: &str) -> Self {
        let mut offsets = vec![0u32; text.len() + 1];
        let mut units = 0u32;
        for (index, ch) in text.char_indices() {
            for slot in &mut offsets[index..index + ch.len_utf8()] {
                *slot = units;
            }
            units += ch.len_utf16() as u32;
        }
        offsets[text.len()] = units;
        Self { offsets }
    }

    fn at(&self, byte: usize) -> u32 {
        self.offsets[byte]
    }
}

fn unspellable_action_handler() -> String {
    "Cannot format an action handler: it has no NX source spelling".to_string()
}

fn unspellable_nested_list() -> String {
    "Cannot format a list nested inside a list: NX has no source spelling for it, because a braced \
     value is not admitted as an item of another braced value"
        .to_string()
}

/// Renders a float so it reads back as a real literal rather than an integer one.
///
/// `1.0` formats as `1` by default. An integer literal does bind at a float-typed site, but
/// rendered output has to read back wherever it is pasted, including sites that supply no expected
/// type: `let x = 1` infers `int`, so dropping the `.0` would round-trip a float as an integer.
/// A real value as an NX literal, from its shortest round-trip digits. A `float32` is rendered from
/// its own digits (`0.1`), not those of its `float64` widening, and reads back as the same value.
fn format_real_literal(rendered: String, finite: bool) -> String {
    if rendered.contains(['.', 'e', 'E']) || !finite {
        rendered
    } else {
        format!("{}.0", rendered)
    }
}

/// Whether a value forces the one-property-per-line layout.
fn is_complex_value(value: &Value) -> bool {
    match value {
        Value::Record { .. } | Value::ActionHandler { .. } => true,
        Value::Array(elements) => !elements.is_empty(),
        _ => false,
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;
    use smol_str::SmolStr;

    /// Formats a value that is expected to have an NX spelling.
    fn formatted(value: &Value) -> String {
        format_nx_text(value).expect("value should have an NX spelling")
    }

    /// Every scalar in attribute position must be emitted in a form that reads back.
    #[test]
    fn test_format_attribute_scalars_are_unquoted() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("w"), Value::Float(1.5));
        fields.insert(SmolStr::new("flag"), Value::Boolean(true));
        fields.insert(SmolStr::new("opt"), Value::empty());
        fields.insert(
            SmolStr::new("fit"),
            Value::UnionCase {
                union: nx_hir::Name::new("Fit"),
                case: SmolStr::new("cover"),
            },
        );
        // A payloadless case is a `Value::UnionCase` now, not an empty dotted record, so there is
        // nothing left for a heuristic to guess at.
        fields.insert(
            SmolStr::new("state"),
            Value::UnionCase {
                union: nx_hir::Name::new("LoadState"),
                case: SmolStr::new("loading"),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let formatted = formatted(&value);
        assert_eq!(
            formatted.trim(),
            "<Box fit=cover flag=true opt={} state=loading w=1.5 />"
        );
        assert!(
            !formatted.contains('"'),
            "no scalar should be quoted: {formatted}"
        );
    }

    /// A `float32` prints its own shortest digits, which read back at a `float32` site as the same
    /// value, rather than the digits of its `float64` widening.
    #[test]
    fn test_format_float32_prints_its_own_digits() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("v"), Value::Float32(0.1));
        fields.insert(SmolStr::new("n"), Value::Float32(2.0));
        let value = Value::Record {
            type_name: nx_hir::Name::new("F"),
            fields,
        };

        assert_eq!(formatted(&value).trim(), "<F n=2.0 v=0.1 />");
        assert_eq!(formatted(&Value::Float32(0.1)), "0.1");
    }

    /// A float keeps its real-literal spelling: at a site with no expected type the spelling is
    /// the only thing that distinguishes it from an integer.
    #[test]
    fn test_format_attribute_negative_float_keeps_real_spelling() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("neg"), Value::Float(-1.0));
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let formatted = formatted(&value);
        assert!(formatted.contains("neg=-1.0"), "got: {formatted}");
        assert!(!formatted.contains("neg=-1 "), "got: {formatted}");
        assert!(!formatted.contains("neg=\"-1\""), "got: {formatted}");
    }

    /// Formatted output must re-parse and type check against the types it came from.
    #[test]
    fn test_format_attribute_output_round_trips() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("w"), Value::Float(1.5));
        fields.insert(SmolStr::new("neg"), Value::Float(-1.0));
        fields.insert(SmolStr::new("n"), Value::Int(42));
        fields.insert(SmolStr::new("flag"), Value::Boolean(true));
        fields.insert(SmolStr::new("opt"), Value::empty());
        fields.insert(
            SmolStr::new("fit"),
            Value::UnionCase {
                union: nx_hir::Name::new("Fit"),
                case: SmolStr::new("cover"),
            },
        );
        fields.insert(
            SmolStr::new("state"),
            Value::Record {
                type_name: nx_hir::Name::new("LoadState.loading"),
                fields: FxHashMap::default(),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let source = format!(
            "type Fit = fill | contain | cover\n\
             type LoadState = idle | loading\n\
             type Box = {{ w: float64 neg: float64 n: int flag: boolean opt?: string \
             fit: Fit state: LoadState }}\n{}",
            formatted(&value)
        );

        let result = nx_types::check_str(&source, "roundtrip.nx");
        assert!(
            result.errors().is_empty(),
            "formatted output should type check, got: {:?}\nsource:\n{}",
            result.errors(),
            source
        );
    }

    #[test]
    fn test_format_int() {
        let value = Value::Int(42);
        assert_eq!(formatted(&value), "42");
    }

    #[test]
    fn test_format_float() {
        let value = Value::Float(2.75);
        assert_eq!(formatted(&value), "2.75");
    }

    /// A whole-valued float is the case an integer literal at a float site could tempt one to
    /// shorten. It must not be: `let x = 24` infers `int`, so `24` does not read back as a float.
    #[test]
    fn test_format_whole_valued_float_keeps_its_real_spelling() {
        assert_eq!(formatted(&Value::Float(24.0)), "24.0");
        assert_eq!(formatted(&Value::Float(0.0)), "0.0");
        assert_eq!(formatted(&Value::Float(-8.0)), "-8.0");
    }

    #[test]
    fn test_format_string() {
        let value = Value::String(SmolStr::new("hello world"));
        assert_eq!(formatted(&value), "hello world");
    }

    #[test]
    fn test_format_boolean() {
        assert_eq!(formatted(&Value::Boolean(true)), "true");
        assert_eq!(formatted(&Value::Boolean(false)), "false");
    }

    /// The empty value — what an optional property that was not written holds — is the empty
    /// sequence, and `{}` is its spelling.
    #[test]
    fn test_format_empty_value() {
        assert_eq!(formatted(&Value::empty()), "{}");
    }

    #[test]
    fn test_format_simple_record() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
        fields.insert(SmolStr::new("age"), Value::Int(30));

        let value = Value::Record {
            type_name: nx_hir::Name::new("result"),
            fields,
        };
        let output = formatted(&value);

        // Should be a self-closing tag with attributes
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        // A number is emitted unquoted: `age="30"` is a string at an int-typed site.
        assert!(output.contains("age=30"));
        assert!(output.contains("/>"));
    }

    #[test]
    fn test_format_constant_case_value() {
        let value = Value::UnionCase {
            union: nx_hir::Name::new("Status"),
            case: SmolStr::new("active"),
        };
        assert_eq!(formatted(&value), "Status.active");
    }

    #[test]
    fn test_format_array_of_primitives() {
        let value = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(formatted(&value), "1\n2\n3");
    }

    #[test]
    fn test_format_nested_record() {
        let mut inner_fields = FxHashMap::default();
        inner_fields.insert(SmolStr::new("city"), Value::String(SmolStr::new("Boston")));
        inner_fields.insert(SmolStr::new("zip"), Value::String(SmolStr::new("02101")));

        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
        fields.insert(
            SmolStr::new("address"),
            Value::Record {
                type_name: nx_hir::Name::new("Address"),
                fields: inner_fields,
            },
        );

        let value = Value::Record {
            type_name: nx_hir::Name::new("result"),
            fields,
        };
        let output = formatted(&value);

        // The nested record is a property value, not body content: the property name `address`
        // has nowhere to go in body position, so emitting it there loses which field it bound to.
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        assert!(
            output.contains("address=<Address"),
            "a record-valued property is an unbraced element, got: {output}"
        );
        assert!(output.contains("city=\"Boston\""));
        assert!(
            !output.contains("</result>"),
            "no field becomes body content, got: {output}"
        );
    }

    #[test]
    fn test_format_string_with_special_chars() {
        let value = Value::String(SmolStr::new("Hello \"World\"\nNew line"));
        assert_eq!(formatted(&value), "Hello \"World\"\nNew line");
    }

    #[test]
    fn test_format_action_handler() {
        let mut module = nx_hir::LoweredModule::new(nx_hir::SourceId::new(0));
        let body = module.alloc_expr(nx_hir::ast::Expr::Literal(nx_hir::ast::Literal::Int(0)));
        let value = Value::ActionHandler {
            module_id: nx_interpreter::RuntimeModuleId::new(0),
            component: nx_hir::Name::new("SearchBox"),
            emit: nx_hir::Name::new("SearchSubmitted"),
            action_name: nx_hir::Name::new("SearchSubmitted"),
            action_module_identity: "format-test.nx".to_string(),
            body,
            captured: FxHashMap::default(),
            owner: None,
            owner_state: Vec::new(),
            token: None,
        };

        // `<ActionHandler ... />` is not a real element, so printing one produced output that
        // could never be read back. It fails explicitly instead.
        let error = format_nx_text(&value).expect_err("an action handler has no NX spelling");
        assert!(error.contains("action handler"), "got: {error}");
    }

    /// An empty qualified record is not a union case. Formatting must not rewrite it into one.
    ///
    /// This is RF2 in `contextual-literal-binding`'s `review.md`. It is asserted on the value
    /// rather than through a source round-trip because the interpreter cannot yet construct a
    /// record imported under a module alias (`RecordTypeNotFound`), so `<div data={<foo.bar />} />`
    /// has no end-to-end spelling today. The defect is entirely in this module regardless.
    #[test]
    fn test_format_empty_qualified_record_is_not_rendered_as_a_union_case() {
        let mut fields = FxHashMap::default();
        fields.insert(
            SmolStr::new("data"),
            Value::Record {
                type_name: nx_hir::Name::new("foo.bar"),
                fields: FxHashMap::default(),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields,
        };

        let formatted = formatted(&value);

        assert_ne!(
            formatted.trim(),
            "<div data=bar />",
            "an empty qualified record must not be rewritten as a bare union case name"
        );
        assert!(
            formatted.contains("data="),
            "the property name must survive, got `{}`",
            formatted.trim()
        );
        assert!(
            formatted.contains("foo.bar"),
            "the record's own type name must survive, got `{}`",
            formatted.trim()
        );
    }

    /// Evaluates a source's single element function and returns the value it produces.
    fn evaluate(source: &str, function: &str) -> Value {
        let parsed = nx_syntax::parse_str(source, "roundtrip.nx");
        assert!(
            parsed.errors.is_empty(),
            "rendered output should parse, got: {:?}\nsource:\n{}",
            parsed.errors,
            source
        );
        let module = nx_hir::lower(parsed.root().expect("root"), nx_hir::SourceId::new(0));
        nx_interpreter::Interpreter::new()
            .execute_function(&module, function, Vec::new())
            .unwrap_or_else(|e| panic!("rendered output should evaluate, got: {e}"))
    }

    /// Superseded `test_format_empty_list_property_has_no_readable_spelling`: `{}` is the spelling.
    #[test]
    fn test_format_empty_list_property_emits_the_empty_braced_form() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("items"), Value::Array(Vec::new()));
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let formatted = formatted(&value);
        assert!(formatted.contains("items={}"), "got: {formatted}");
        assert!(!formatted.contains("items=\""), "got: {formatted}");
    }

    /// The point of the spelling: what is rendered has to read back as the value it came from.
    #[test]
    fn test_format_empty_list_round_trips() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("items"), Value::Array(Vec::new()));
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let source = format!(
            "type Box = {{ items?: string+ }}\nlet <make /> = {{ {} }}\n",
            formatted(&value)
        );

        let result = nx_types::check_str(&source, "roundtrip.nx");
        assert!(
            result.errors().is_empty(),
            "formatted output should type check, got: {:?}\nsource:\n{}",
            result.errors(),
            source
        );

        // An optional field holding the empty value is not stored, so the record carries no
        // entry for it; an entry, if one were stored, would have to be the empty value.
        match evaluate(&source, "make") {
            Value::Record { fields, .. } => match fields.get(&SmolStr::new("items")) {
                None => {}
                Some(value) if value.is_empty_value() => {}
                other => panic!("expected the empty value, got: {other:?}"),
            },
            other => panic!("expected a record, got: {other:?}"),
        }
    }

    /// An action handler is still unspellable, so the failure path is still exercised.
    #[test]
    fn test_format_action_handler_is_still_unspellable() {
        let mut module = nx_hir::LoweredModule::new(nx_hir::SourceId::new(0));
        let body = module.alloc_expr(nx_hir::ast::Expr::Literal(nx_hir::ast::Literal::Int(0)));
        let value = Value::ActionHandler {
            module_id: nx_interpreter::RuntimeModuleId::new(0),
            component: nx_hir::Name::new("Button"),
            emit: nx_hir::Name::new("Click"),
            action_name: nx_hir::Name::new("Button.Click"),
            action_module_identity: "test.nx".to_string(),
            body,
            captured: FxHashMap::default(),
            owner: None,
            owner_state: Vec::new(),
            token: None,
        };

        let error = format_nx_text(&value).expect_err("an action handler has no NX spelling");
        assert!(error.contains("action handler"), "got: {error}");
    }

    /// A list with no elements has a spelling on its own, not only in property position.
    ///
    /// The top-level form is a run of values one per line, and an empty run is no output at all,
    /// which reads back as no value rather than as the empty list. The braced spelling is the one
    /// thing that reads back as what was rendered.
    #[test]
    fn test_format_top_level_empty_list_is_the_braced_form() {
        let rendered =
            format_nx_text(&Value::Array(Vec::new())).expect("an empty list has an NX spelling");
        assert_eq!(rendered, "{}");
    }

    /// One value per line cannot say where an inner list ends, so a list of lists would render as
    /// the flattened run of its elements and read back as a different value. It is reported here
    /// for the same reason it is reported in property position.
    #[test]
    fn test_format_top_level_nested_list_is_unspellable() {
        let value = Value::Array(vec![
            Value::Array(vec![Value::String(SmolStr::new("a"))]),
            Value::Array(vec![Value::String(SmolStr::new("b"))]),
        ]);

        let error = format_nx_text(&value).expect_err("a nested list has no NX spelling");
        assert!(error.contains("nested inside a list"), "got: {error}");
    }

    /// A list of lists is reported, not rendered.
    ///
    /// The inner braces would be emitted as `{{}}`, which reads back as the empty value rather than
    /// a list holding an empty list, since a sequence is flat. Giving the empty list a
    /// spelling removed the `is_empty` guard that used to reject this case for an unrelated reason;
    /// the case is caught on its own terms instead.
    #[test]
    fn test_format_empty_list_nested_in_a_list_is_unspellable() {
        let mut fields = FxHashMap::default();
        fields.insert(
            SmolStr::new("rows"),
            Value::Array(vec![Value::Array(Vec::new())]),
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Grid"),
            fields,
        };

        let error = format_nx_text(&value).expect_err("a nested list has no NX spelling");
        assert!(error.contains("nested inside a list"), "got: {error}");
    }

    /// Emptiness is not what makes the nested case unspellable, so a non-empty inner list is
    /// reported the same way. This one was already unrenderable before the empty list had a
    /// spelling; it was simply emitted rather than reported.
    #[test]
    fn test_format_non_empty_list_nested_in_a_list_is_unspellable() {
        let mut fields = FxHashMap::default();
        fields.insert(
            SmolStr::new("rows"),
            Value::Array(vec![Value::Array(vec![Value::String(SmolStr::new("a"))])]),
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Grid"),
            fields,
        };

        let error = format_nx_text(&value).expect_err("a nested list has no NX spelling");
        assert!(error.contains("nested inside a list"), "got: {error}");
    }

    /// The guard is about a list directly inside a list. A record between them has an element form,
    /// so a list of records that themselves hold lists still renders.
    #[test]
    fn test_format_list_of_records_holding_lists_still_renders() {
        let mut row = FxHashMap::default();
        row.insert(SmolStr::new("cells"), Value::Array(Vec::new()));
        let mut fields = FxHashMap::default();
        fields.insert(
            SmolStr::new("rows"),
            Value::Array(vec![Value::Record {
                type_name: nx_hir::Name::new("Row"),
                fields: row,
            }]),
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Grid"),
            fields,
        };

        let rendered = formatted(&value);
        assert!(rendered.contains("cells={}"), "got: {rendered}");
    }

    /// Node tests: what the formatter records beside the text.
    mod annotations {
        use super::super::*;
        use crate::{
            eval_program_artifact_nx_text, load_program_artifact_from_source, NxDiagnostic,
            ProgramBuildContext,
        };
        use rustc_hash::FxHashMap;
        use smol_str::SmolStr;

        /// Annotates a value against an empty module, so nothing has a declaration.
        fn annotated(value: &Value) -> NxValueText {
            let module = nx_hir::LoweredModule::new(nx_hir::SourceId::new(0));
            annotate_nx_text(value, &module, "").expect("value should have an NX spelling")
        }

        /// The text a node covers, sliced by its UTF-16 offsets as JavaScript would slice it.
        fn slice(text: &NxValueText, node: &NxValueNode) -> String {
            let units: Vec<u16> = text.text.encode_utf16().collect();
            String::from_utf16(&units[node.start as usize..node.end as usize]).unwrap()
        }

        fn record(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
            Value::Record {
                type_name: nx_hir::Name::new(type_name),
                fields: fields
                    .into_iter()
                    .map(|(name, value)| (SmolStr::new(name), value))
                    .collect::<FxHashMap<_, _>>(),
            }
        }

        fn string(value: &str) -> Value {
            Value::String(SmolStr::new(value))
        }

        fn evaluate(source: &str) -> Result<NxValueText, Vec<NxDiagnostic>> {
            let program = load_program_artifact_from_source(
                source,
                "input.nx",
                &ProgramBuildContext::empty(),
            )?;
            eval_program_artifact_nx_text(&program)
        }

        fn evaluated(source: &str) -> NxValueText {
            evaluate(source).unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"))
        }

        fn failure(source: &str) -> NxDiagnostic {
            let diagnostics = evaluate(source).expect_err("evaluation should fail");
            diagnostics.into_iter().next().expect("a diagnostic")
        }

        fn property<'a>(text: &'a NxValueText, name: &str) -> &'a NxValueNode {
            text.nodes
                .iter()
                .find(|node| {
                    node.role == NxValueRole::Property && node.name.as_deref() == Some(name)
                })
                .unwrap_or_else(|| panic!("no property node `{name}` in {:?}", text.nodes))
        }

        #[test]
        fn a_one_line_record_has_a_node_and_one_per_property() {
            let text = annotated(&record(
                "User",
                vec![("id", string("1")), ("name", string("Ada"))],
            ));
            assert_eq!(text.text, r#"<User id="1" name="Ada" />"#);
            assert_eq!(text.nodes.len(), 3);

            let user = &text.nodes[0];
            assert_eq!(user.role, NxValueRole::Record);
            assert_eq!(user.ty.as_deref(), Some("User"));
            assert_eq!(user.parent, None);
            assert_eq!(slice(&text, user), text.text);

            let id = &text.nodes[1];
            assert_eq!(id.role, NxValueRole::Property);
            assert_eq!(id.parent, Some(0));
            assert_eq!(slice(&text, id), r#"id="1""#);
            assert_eq!(slice(&text, property(&text, "name")), r#"name="Ada""#);
        }

        #[test]
        fn a_multi_line_record_nests_the_record_under_its_property() {
            let text = annotated(&record(
                "Person",
                vec![
                    ("name", string("Ada")),
                    ("home", record("Address", vec![("city", string("London"))])),
                ],
            ));
            assert_eq!(
                text.text,
                "<Person\n  home=<Address city=\"London\" />\n  name=\"Ada\"\n/>"
            );

            let home = property(&text, "home");
            assert_eq!(slice(&text, home), r#"home=<Address city="London" />"#);
            let home_index = text.nodes.iter().position(|node| node == home).unwrap() as u32;
            let address = text
                .nodes
                .iter()
                .find(|node| {
                    node.role == NxValueRole::Record && node.ty.as_deref() == Some("Address")
                })
                .unwrap();
            assert_eq!(address.role, NxValueRole::Record);
            assert_eq!(address.parent, Some(home_index));
            assert_eq!(slice(&text, address), r#"<Address city="London" />"#);
        }

        #[test]
        fn a_sequence_of_records_covers_the_run_and_parents_each_record() {
            let users = (1..=3)
                .map(|n| record("User", vec![("id", Value::Int(n))]))
                .collect();
            let text = annotated(&Value::Array(users));
            assert_eq!(text.text, "<User id=1 />\n<User id=2 />\n<User id=3 />");

            let sequence = &text.nodes[0];
            assert_eq!(sequence.role, NxValueRole::Sequence);
            assert_eq!(sequence.ty.as_deref(), Some("User*"));
            assert_eq!(sequence.count, Some(3));
            assert_eq!(slice(&text, sequence), text.text);

            let records: Vec<_> = text
                .nodes
                .iter()
                .filter(|node| node.role == NxValueRole::Record)
                .collect();
            assert_eq!(records.len(), 3);
            assert!(records.iter().all(|node| node.parent == Some(0)));
            assert_eq!(slice(&text, records[1]), "<User id=2 />");
        }

        #[test]
        fn a_mixed_sequence_is_a_sequence_of_objects() {
            let text = annotated(&Value::Array(vec![Value::Int(1), string("a")]));
            assert_eq!(text.nodes[0].ty.as_deref(), Some("object*"));
        }

        #[test]
        fn a_constant_case_is_a_case_of_its_union() {
            let text = annotated(&Value::UnionCase {
                union: nx_hir::Name::new("Status"),
                case: SmolStr::new("active"),
            });
            assert_eq!(text.text, "Status.active");
            assert_eq!(text.nodes.len(), 1);
            assert_eq!(text.nodes[0].role, NxValueRole::Case);
            assert_eq!(text.nodes[0].ty.as_deref(), Some("Status"));
        }

        #[test]
        fn the_empty_value_is_one_empty_node() {
            let text = annotated(&Value::empty());
            assert_eq!(text.text, "{}");
            assert_eq!(text.nodes.len(), 1);
            assert_eq!(text.nodes[0].role, NxValueRole::Empty);
            assert_eq!(text.nodes[0].ty, None);
        }

        #[test]
        fn offsets_count_utf16_code_units() {
            let text = annotated(&record(
                "Note",
                vec![("a", string("😀")), ("b", Value::Int(1))],
            ));
            assert_eq!(text.text, r#"<Note a="😀" b=1 />"#);
            let b = property(&text, "b");
            // The emoji is four bytes and two UTF-16 units, so a byte offset would land two
            // units late, at 15.
            assert_eq!(b.start, 13);
            assert_eq!(slice(&text, b), "b=1");
        }

        #[test]
        fn the_nodes_serialize_as_the_sdk_reads_them() {
            let text = annotated(&record("User", vec![("id", string("1"))]));
            let json = serde_json::to_value(&text).unwrap();
            assert_eq!(
                json["nodes"][0],
                serde_json::json!({ "start": 0, "end": 15, "role": "record", "type": "User" })
            );
            assert_eq!(
                json["nodes"][1],
                serde_json::json!({
                    "start": 6, "end": 12, "parent": 0, "role": "property", "type": "string",
                    "name": "id"
                })
            );
        }

        #[test]
        fn a_record_and_its_properties_are_declared_in_the_source() {
            let text =
                evaluated("type User = { id:string name:string }\n<User id=\"1\" name=\"Ada\" />");
            assert_eq!(text.text, r#"<User id="1" name="Ada" />"#);

            let user = &text.nodes[0];
            let declaration = user.declaration.as_ref().expect("User is declared");
            assert_eq!((declaration.start_line, declaration.start_column), (1, 1));

            for name in ["id", "name"] {
                let node = property(&text, name);
                assert_eq!(node.ty.as_deref(), Some("string"));
                assert_eq!(node.parent, Some(0));
                let declaration = node.declaration.as_ref().expect("the property is declared");
                assert_eq!(declaration.start_line, 1);
                assert!(declaration.start_column > 13, "{declaration:?}");
            }
        }

        #[test]
        fn a_property_carries_its_declared_type_and_optionality() {
            let text = evaluated(
                "type Card = { title:string subtitle?:string count:int = 2 }\n\
                 <Card title=\"a\" subtitle=\"b\" />",
            );
            let subtitle = property(&text, "subtitle");
            assert_eq!(subtitle.ty.as_deref(), Some("string"));
            assert!(subtitle.optional);
            assert!(!property(&text, "title").optional);
            assert_eq!(property(&text, "count").ty.as_deref(), Some("int"));
        }

        #[test]
        fn an_inherited_property_is_declared_on_its_base() {
            let text = evaluated(
                "abstract type Shape = { name:string }\n\
                 type Circle extends Shape = { radius:int }\n\
                 <Circle name=\"c\" radius=2 />",
            );
            let name = property(&text, "name");
            assert_eq!(name.ty.as_deref(), Some("string"));
            assert_eq!(
                name.declaration.as_ref().map(|span| span.start_line),
                Some(1)
            );
        }

        #[test]
        fn a_sequence_of_declared_records() {
            let text = evaluated(
                "type User = { id:string }\n\
                 let root(): User* = { <User id=\"1\" /> <User id=\"2\" /> <User id=\"3\" /> }",
            );
            assert_eq!(text.nodes[0].role, NxValueRole::Sequence);
            assert_eq!(text.nodes[0].ty.as_deref(), Some("User*"));
            assert_eq!(text.nodes[0].count, Some(3));
            assert!(text
                .nodes
                .iter()
                .filter(|node| node.role == NxValueRole::Record)
                .all(|node| node.declaration.is_some()));
        }

        #[test]
        fn a_case_is_declared_at_its_case() {
            let text = evaluated("type Status = active | retired\nlet root(): Status = retired");
            assert_eq!(text.text, "Status.retired");
            let declaration = text.nodes[0]
                .declaration
                .as_ref()
                .expect("the case is declared");
            assert_eq!(declaration.start_line, 1);
        }

        #[test]
        fn a_component_and_its_props_are_declared_in_the_source() {
            let text = evaluated(
                "external component <Button label:string emits { Tapped { } } />\n\
                 component <Counter start:int = 0 /> = {\n  state { count:int = {start} }\n  <Button label=\"Add\" />\n}\n\
                 let root() = { <Counter start=3 /> <Button label=\"Reset\" /> }",
            );
            let counter = text
                .nodes
                .iter()
                .find(|node| {
                    node.role == NxValueRole::Record && node.ty.as_deref() == Some("Counter")
                })
                .expect("a Counter node");
            assert_eq!(
                counter.declaration.as_ref().map(|span| span.start_line),
                Some(2)
            );
            let button = text
                .nodes
                .iter()
                .find(|node| {
                    node.role == NxValueRole::Record && node.ty.as_deref() == Some("Button")
                })
                .expect("a Button node");
            assert_eq!(
                button.declaration.as_ref().map(|span| span.start_line),
                Some(1)
            );
            let start = property(&text, "start");
            assert_eq!(start.ty.as_deref(), Some("int"));
            assert_eq!(
                start.declaration.as_ref().map(|span| span.start_line),
                Some(2)
            );
            let label = property(&text, "label");
            assert_eq!(label.ty.as_deref(), Some("string"));
            assert!(label.declaration.is_some());
        }

        #[test]
        fn a_prelude_record_has_no_declaration() {
            let text = evaluated("let root() = <Range T=int start=1 end=3 endInclusive=false />");
            assert_eq!(text.nodes[0].ty.as_deref(), Some("Range"));
            assert!(text.nodes.iter().all(|node| node.declaration.is_none()));
        }

        #[test]
        fn no_root_is_a_diagnostic() {
            let diagnostic = failure("type User = { id:string }");
            assert_eq!(diagnostic.code.as_deref(), Some("no-root"));
        }

        #[test]
        fn a_runtime_error_carries_its_span() {
            let diagnostic = failure("let root() = { 1 / 0 }");
            assert_eq!(diagnostic.code.as_deref(), Some("runtime-error"));
            assert_eq!(diagnostic.message, "Division by zero");
            let label = diagnostic.labels.first().expect("the error is located");
            assert_eq!(label.file, "input.nx");
            assert_eq!(
                (label.span.start_byte, label.span.end_byte),
                (15, 20),
                "the span should cover `1 / 0`"
            );
        }

        #[test]
        fn a_runtime_error_in_an_imported_function_is_labeled_at_the_call() {
            let workspace = crate::NxWorkspace::new(vec![
                crate::NxWorkspaceModule::from_source(
                    "ui/math.nx",
                    "export let divide(n:int): int = { 10 / n }",
                )
                .unwrap(),
                crate::NxWorkspaceModule::from_source(
                    "app/main.nx",
                    "import \"../ui/math.nx\"\nlet root() = {\n  divide(0)\n}",
                )
                .unwrap(),
            ])
            .unwrap();
            let program = crate::build_workspace_program_artifact(
                &workspace,
                "app/main.nx",
                &ProgramBuildContext::empty(),
            )
            .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
            let diagnostics =
                eval_program_artifact_nx_text(&program).expect_err("division by zero");
            let diagnostic = &diagnostics[0];
            assert_eq!(diagnostic.message, "Division by zero");
            let label = diagnostic.labels.first().expect("labeled at the call");
            assert_eq!(label.file, "app/main.nx");
            assert_eq!(label.span.start_line, 3, "{label:?}");
        }

        #[test]
        fn an_action_handler_is_unspellable() {
            let diagnostic = failure(
                "action SearchRequested = { query:string }\n\
                 action DoSearch = { query:string }\n\
                 external component <SearchBox emits { SearchRequested } />\n\
                 let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }",
            );
            assert_eq!(diagnostic.code.as_deref(), Some("nx-text-unspellable"));
            assert!(
                diagnostic.message.contains("action handler"),
                "{}",
                diagnostic.message
            );
        }
    }
}
