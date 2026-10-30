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
use nx_hir::{
    ast::{spell_type_ref, TypeRef},
    Item, LoweredModule, Name, RecordField, UnionDef,
};
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
    /// The RFC 6901 JSON pointer of the node's value within the value's canonical JSON form: the
    /// empty string for the whole value. A property's is its value's.
    pub pointer: String,
    /// CSS presentation of a validated canonical Color value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
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
    libraries: &[&LoweredModule],
    source: &str,
) -> Result<NxValueText, String> {
    let mut writer = Writer::default();
    writer.value(value, 0)?;
    let declarations = Declarations::new(module, libraries, source);
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
    color: Option<String>,
    start: usize,
    end: usize,
    parent: Option<usize>,
    role: NxValueRole,
    ty: Option<String>,
    count: Option<usize>,
    key: DeclarationKey,
    pointer: String,
}

/// What a node's declaration would be, if the entry module declares it.
enum DeclarationKey {
    None,
    Record(Name),
    /// A property of the record or component `owner`, or, with `state`, a state field of the
    /// component `owner`.
    Property {
        owner: Name,
        name: SmolStr,
        state: bool,
    },
    Case {
        union: Name,
        case: SmolStr,
    },
    Function(SmolStr),
}

/// Writes NX text and records a node for each part of it.
#[derive(Default)]
struct Writer {
    output: String,
    nodes: Vec<RawNode>,
    open: Vec<usize>,
    /// The JSON pointer of the value being written.
    pointer: String,
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
            color: None,
            start: self.output.len(),
            end: self.output.len(),
            parent: self.open.last().copied(),
            role,
            ty,
            count,
            key,
            pointer: self.pointer.clone(),
        });
        self.open.push(index);
    }

    /// Writes `write` with the pointer extended by one reference token, `token`.
    fn within<T>(
        &mut self,
        token: &str,
        write: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let length = self.pointer.len();
        push_token(&mut self.pointer, token);
        let result = write(self);
        if result.is_ok() {
            self.pointer.truncate(length);
        }
        result
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
                    self.within(&i.to_string(), |writer| writer.value(elem, indent))?;
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
        if tag_name.as_str() == "Color" {
            let channels: Option<Vec<f64>> = ["r", "g", "b", "a"]
                .iter()
                .map(|name| match fields.get(*name)? {
                    Value::Float32(value) => Some(*value as f64),
                    Value::Float(value) => Some(*value),
                    Value::Int(value) => Some(*value as f64),
                    _ => None,
                })
                .collect();
            if let Some(channels) = channels {
                if channels
                    .iter()
                    .all(|x| x.is_finite() && (0.0..=1.0).contains(x) && (*x as f32) as f64 == *x)
                {
                    if let Some(index) = self.open.last().copied() {
                        self.nodes[index].color = Some(format!(
                            "rgba({}, {}, {}, {})",
                            channels[0] * 255.0,
                            channels[1] * 255.0,
                            channels[2] * 255.0,
                            channels[3]
                        ));
                    }
                }
            }
        }
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
        self.within(key.as_str(), |writer| {
            writer.keyed_property(owner, key, value, indent, false)
        })
    }

    /// Emits the state fields of the component `owner`, one `key=value` per line, in order.
    fn state_fields(&mut self, owner: &Name, fields: &[(SmolStr, Value)]) -> Result<(), String> {
        for (index, (key, value)) in fields.iter().enumerate() {
            if index > 0 {
                self.output.push('\n');
            }
            self.within(key.as_str(), |writer| {
                writer.keyed_property(owner, key, value, 0, true)
            })?;
        }
        Ok(())
    }

    /// Emits a property whose value's pointer is already the current one.
    fn keyed_property(
        &mut self,
        owner: &Name,
        key: &SmolStr,
        value: &Value,
        indent: usize,
        state: bool,
    ) -> Result<(), String> {
        // The value's own type stands in until the declaration, if there is one, replaces it.
        self.begin(
            NxValueRole::Property,
            value_type(value),
            None,
            DeclarationKey::Property {
                owner: owner.clone(),
                name: key.clone(),
                state,
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
                    self.within(&i.to_string(), |writer| {
                        let scalar = is_scalar(element);
                        if scalar {
                            writer.begin(
                                NxValueRole::Scalar,
                                value_type(element),
                                None,
                                DeclarationKey::None,
                            );
                        }
                        writer.property_value(element, indent)?;
                        if scalar {
                            writer.end();
                        }
                        Ok(())
                    })?;
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
    canonical_color: bool,
    span: TextRange,
    fields: &'a [RecordField],
    base: Option<&'a Name>,
    /// Declared in the entry module, whose source the spans index. A library's declarations give
    /// types but no spans.
    local: bool,
}

/// The entry module's records, components, unions and functions, by name, and a way to spell their
/// spans, with the declarations of the libraries it links for their types.
struct Declarations<'a> {
    shapes: FxHashMap<&'a str, Shape<'a>>,
    /// Each component's state fields, by the component's name.
    states: FxHashMap<&'a str, &'a [RecordField]>,
    aliases: FxHashMap<&'a str, &'a TypeRef>,
    /// Each union, and whether the entry module declares it.
    unions: FxHashMap<&'a str, (&'a UnionDef, bool)>,
    functions: FxHashMap<&'a str, TextRange>,
    source: &'a str,
    lines: LineIndex,
}

impl<'a> Declarations<'a> {
    /// Indexes `module`, whose source is `source`, and then `libraries` for what `module` does not
    /// declare itself: a name the entry module declares is the entry module's.
    fn new(module: &'a LoweredModule, libraries: &[&'a LoweredModule], source: &'a str) -> Self {
        let mut declarations = Self {
            shapes: FxHashMap::default(),
            states: FxHashMap::default(),
            aliases: FxHashMap::default(),
            unions: FxHashMap::default(),
            functions: FxHashMap::default(),
            source,
            lines: LineIndex::new(source),
        };
        declarations.index(module, true);
        for library in libraries {
            declarations.index(library, false);
        }
        declarations
    }

    fn index(&mut self, module: &'a LoweredModule, local: bool) {
        for item in module.items() {
            match item {
                Item::Record(record) => {
                    self.shapes.entry(record.name.as_str()).or_insert(Shape {
                        canonical_color: record.name.as_str() == "Color"
                            && crate::standard_library("@nx/color").is_some_and(|library| {
                                library.modules.iter().any(|artifact| {
                                    artifact.file_name == "@nx/color/color.nx"
                                        && artifact.lowered_module.as_ref().is_some_and(|lowered| {
                                            lowered.source_id == module.source_id
                                        })
                                })
                            }),
                        span: record.span,
                        fields: &record.properties,
                        base: record.base.as_ref(),
                        local,
                    });
                }
                // External components too: their props are declared in this source even though
                // their body lives in the host.
                Item::Component(component) => {
                    if local {
                        self.states
                            .insert(component.name.as_str(), component.state.as_slice());
                    }
                    self.shapes.entry(component.name.as_str()).or_insert(Shape {
                        canonical_color: false,
                        span: component.span,
                        fields: &component.props,
                        base: component.base.as_ref(),
                        local,
                    });
                }
                Item::Union(union) => {
                    self.unions
                        .entry(union.name.as_str())
                        .or_insert((union, local));
                }
                Item::TypeAlias(alias) => {
                    self.aliases.entry(alias.name.as_str()).or_insert(&alias.ty);
                }
                Item::Function(function) if local => {
                    self.functions.insert(function.name.as_str(), function.span);
                }
                _ => {}
            }
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
            pointer: node.pointer,
            color: node.color,
        };

        match node.key {
            DeclarationKey::None => {}
            DeclarationKey::Record(name) => {
                if !self.shape(&name).is_some_and(|shape| shape.canonical_color) {
                    annotated.color = None;
                }
                annotated.declaration = self
                    .shape(&name)
                    .map(|shape| {
                        if shape.local {
                            shape.span
                        } else {
                            TextRange::default()
                        }
                    })
                    .or_else(|| self.payload_case(&name).map(|(_, case)| case))
                    .and_then(|range| self.span(range));
            }
            DeclarationKey::Property { owner, name, state } => {
                annotated.name = Some(name.to_string());
                let field = if state {
                    self.state_field(&owner, &name)
                } else {
                    self.field(&owner, &name)
                };
                if let Some(field) = field {
                    annotated.ty = Some(field.ty);
                    annotated.optional = field.optional;
                    annotated.declaration = self.span(field.span);
                }
            }
            DeclarationKey::Case { union, case } => {
                annotated.declaration = self
                    .unions
                    .get(union.as_str())
                    .filter(|(_, local)| *local)
                    .and_then(|(union, _)| union.cases.iter().find(|c| c.name.as_str() == case))
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
    /// declares fields, whose values are records. The case's span is empty for a library's union.
    fn payload_case(&self, name: &Name) -> Option<(&'a UnionDef, TextRange)> {
        let (union, case) = name.as_str().split_once('.')?;
        let (union, local) = self.unions.get(union).copied()?;
        let case = union.cases.iter().find(|c| c.name.as_str() == case)?;
        Some((
            union,
            if local {
                case.span
            } else {
                TextRange::default()
            },
        ))
    }

    /// The declared property `name` of the record type or component `owner`, its own or one it
    /// inherits.
    fn field(&self, owner: &Name, name: &str) -> Option<DeclaredField<'a>> {
        if let Some((union, case_span)) = self.payload_case(owner) {
            let case_name = owner.as_str().split_once('.')?.1;
            let case = union.cases.iter().find(|c| c.name.as_str() == case_name)?;
            return case
                .fields
                .iter()
                .find(|field| field.name.as_str() == name)
                .map(|field| DeclaredField {
                    ty: spell_type_ref(&field.ty),
                    ty_ref: &field.ty,
                    optional: field.optional,
                    span: if case_span.is_empty() {
                        TextRange::default()
                    } else {
                        field.span
                    },
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
                let mut found = declared(field);
                if !current.local {
                    found.span = TextRange::default();
                }
                return Some(found);
            }
            visited += 1;
            if visited > self.shapes.len() {
                return None;
            }
            shape = current.base.and_then(|base| self.shape(base));
        }
        None
    }

    /// The state field `name` of the component `owner`.
    fn state_field(&self, owner: &Name, name: &str) -> Option<DeclaredField<'a>> {
        self.states
            .get(owner.as_str())?
            .iter()
            .find(|field| field.name.as_str() == name)
            .map(declared)
    }

    /// What a number or string at a site declared `ty` is: the primitive or union `ty` names,
    /// through aliases and occurrence suffixes.
    fn expect(&self, ty: &'a TypeRef, depth: usize) -> Expected<'a> {
        match ty {
            TypeRef::Name(name) => self.expect_name(name.as_str(), depth),
            TypeRef::Seq { inner, .. } => self.expect(inner, depth),
            _ => Expected::Any,
        }
    }

    fn expect_name(&self, name: &str, depth: usize) -> Expected<'a> {
        if let Some(primitive) = NUMERIC_PRIMITIVES.iter().find(|p| **p == name) {
            return Expected::Primitive(primitive);
        }
        if let Some((union, _)) = self.unions.get(name) {
            return Expected::Union(union);
        }
        // An alias cycle is the checker's to report; stop following one rather than loop.
        match self.aliases.get(name) {
            Some(target) if depth < self.aliases.len() => self.expect(target, depth + 1),
            _ => Expected::Any,
        }
    }

    /// Converts a canonical JSON value, as the IR runtimes return it, to the interpreter's value,
    /// reading what a number or a string is from `expected` and from the declarations of the
    /// records it holds.
    fn canonical(
        &self,
        json: &serde_json::Value,
        expected: Expected<'a>,
        pointer: &mut String,
    ) -> Result<Value, FormatError> {
        Ok(match json {
            serde_json::Value::Null => Value::empty(),
            serde_json::Value::Bool(value) => Value::Boolean(*value),
            serde_json::Value::Number(number) => canonical_number(number, expected),
            serde_json::Value::String(text) => match expected {
                Expected::Union(union)
                    if union
                        .cases
                        .iter()
                        .any(|case| case.name.as_str() == text && case.fields.is_empty()) =>
                {
                    Value::UnionCase {
                        union: union.name.clone(),
                        case: SmolStr::new(text),
                    }
                }
                _ => Value::String(SmolStr::new(text)),
            },
            serde_json::Value::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    values.push(within_pointer(pointer, &index.to_string(), |pointer| {
                        self.canonical(item, expected, pointer)
                    })?);
                }
                Value::Array(values)
            }
            serde_json::Value::Object(properties) => self.canonical_record(properties, pointer)?,
        })
    }

    fn canonical_record(
        &self,
        properties: &serde_json::Map<String, serde_json::Value>,
        pointer: &mut String,
    ) -> Result<Value, FormatError> {
        let type_name = match properties.get("$type") {
            None => "object",
            Some(serde_json::Value::String(type_name)) => type_name.as_str(),
            Some(_) => {
                return Err(FormatError::invalid(
                    "A record's `$type` must be a string",
                    &format!("{pointer}/$type"),
                ))
            }
        };

        // A function value is the record that names its declaration.
        if type_name == "Function" {
            if let (
                Some(serde_json::Value::String(module)),
                Some(serde_json::Value::String(name)),
            ) = (properties.get("module"), properties.get("name"))
            {
                return Ok(Value::Function {
                    module: SmolStr::new(module),
                    name: SmolStr::new(name),
                });
            }
        }

        let owner = Name::new(type_name);
        let mut fields = FxHashMap::default();
        for (key, value) in properties {
            if key == "$type" {
                continue;
            }
            let expected = self
                .field(&owner, key)
                .map_or(Expected::Any, |field| self.expect(field.ty_ref, 0));
            let value = within_pointer(pointer, key, |pointer| {
                self.canonical(value, expected, pointer)
            })?;
            fields.insert(SmolStr::new(key), value);
        }
        Ok(Value::Record {
            type_name: owner,
            fields,
        })
    }
}

fn declared(field: &RecordField) -> DeclaredField<'_> {
    DeclaredField {
        ty: spell_type_ref(&field.ty),
        ty_ref: &field.ty,
        optional: field.optional,
        span: field.span,
    }
}

struct DeclaredField<'a> {
    ty: String,
    ty_ref: &'a TypeRef,
    optional: bool,
    span: TextRange,
}

/// The primitives whose values canonical JSON writes as numbers.
const NUMERIC_PRIMITIVES: [&str; 5] = ["int", "int32", "int64", "float32", "float64"];

/// What a site declares a number or a string to be.
#[derive(Clone, Copy)]
enum Expected<'a> {
    /// Nothing the formatter can use: a number is an `int` when whole, a string is a string.
    Any,
    /// A numeric primitive.
    Primitive(&'static str),
    /// A union, whose constant cases canonical JSON writes as bare strings.
    Union(&'a UnionDef),
}

/// A number from canonical JSON, which does not tell a whole `float64` from an `int`: the site's
/// type does.
fn canonical_number(number: &serde_json::Number, expected: Expected<'_>) -> Value {
    let primitive = match expected {
        Expected::Primitive(primitive) => primitive,
        _ => "",
    };
    if let Some(whole) = number.as_i64() {
        return match primitive {
            "float32" => Value::Float32(whole as f32),
            "float64" => Value::Float(whole as f64),
            "int32" => i32::try_from(whole).map_or(Value::Int(whole), Value::Int32),
            _ => Value::Int(whole),
        };
    }
    let real = number.as_f64().unwrap_or(f64::NAN);
    match primitive {
        "float32" => Value::Float32(real as f32),
        _ => Value::Float(real),
    }
}

/// Runs `write` with `pointer` extended by the reference token `token`, and restores it after a
/// success, so that after a failure it still names where the failure was.
fn within_pointer<T>(
    pointer: &mut String,
    token: &str,
    write: impl FnOnce(&mut String) -> Result<T, FormatError>,
) -> Result<T, FormatError> {
    let length = pointer.len();
    push_token(pointer, token);
    let result = write(pointer);
    if result.is_ok() {
        pointer.truncate(length);
    }
    result
}

fn push_token(pointer: &mut String, token: &str) {
    pointer.push('/');
    pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
}

/// Why a canonical value could not be spelled, and where in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormatError {
    /// `nx-value-invalid` for a value outside the canonical format, `nx-text-unspellable` for one
    /// NX text cannot spell.
    pub code: &'static str,
    pub message: String,
    /// The JSON pointer of the value at fault.
    pub pointer: String,
}

impl FormatError {
    fn invalid(message: &str, pointer: &str) -> Self {
        Self {
            code: "nx-value-invalid",
            message: message.to_string(),
            pointer: pointer.to_string(),
        }
    }
}

/// How a host asks for a canonical value to be spelled.
#[derive(Debug, Clone, Default)]
pub(crate) struct FormatOptions {
    /// The NX type of the whole value, such as `float64` or `Status?`, when the host knows it.
    pub ty: Option<String>,
    /// The component whose state the value is: an object of its state fields, spelled one
    /// `name=value` per line in declaration order.
    pub state: Option<String>,
}

/// Spells a value in canonical JSON, as the IR runtimes return it, as annotated NX text, taking
/// types and declarations from `module`, whose source is `source`.
///
/// <para>An `ActionHandler` record is spelled as the record it is. A number or a string is read
/// by its site's declared type, since canonical JSON writes a whole `float64` as it writes an
/// `int`, and a constant union case as a bare string.</para>
pub(crate) fn annotate_canonical_nx_text(
    value: &serde_json::Value,
    options: &FormatOptions,
    module: &LoweredModule,
    libraries: &[&LoweredModule],
    source: &str,
) -> Result<NxValueText, FormatError> {
    let declarations = Declarations::new(module, libraries, source);
    let mut pointer = String::new();
    let mut writer = Writer::default();
    let written = if let Some(component) = &options.state {
        let state = declarations.states.get(component.as_str()).ok_or_else(|| {
            FormatError::invalid(
                &format!("The program declares no component `{component}`"),
                "",
            )
        })?;
        let serde_json::Value::Object(properties) = value else {
            return Err(FormatError::invalid(
                "A component's state must be an object",
                "",
            ));
        };
        // Declared fields in declaration order, then any the declaration does not know, by name.
        let mut keys: Vec<&str> = state
            .iter()
            .map(|field| field.name.as_str())
            .filter(|name| properties.contains_key(*name))
            .collect();
        let mut unknown: Vec<&str> = properties
            .keys()
            .map(String::as_str)
            .filter(|key| !keys.contains(key))
            .collect();
        unknown.sort_unstable();
        keys.extend(unknown);
        let owner = Name::new(component);
        let mut fields = Vec::with_capacity(keys.len());
        for key in keys {
            let expected = declarations
                .state_field(&owner, key)
                .map_or(Expected::Any, |field| declarations.expect(field.ty_ref, 0));
            let value = within_pointer(&mut pointer, key, |pointer| {
                declarations.canonical(&properties[key], expected, pointer)
            })?;
            fields.push((SmolStr::new(key), value));
        }
        writer.state_fields(&owner, &fields)
    } else {
        let expected = options.ty.as_deref().map_or(Expected::Any, |ty| {
            declarations.expect_name(ty.trim_end_matches(['?', '*', '+']), 0)
        });
        let value = declarations.canonical(value, expected, &mut pointer)?;
        writer.value(&value, 0)
    };
    written.map_err(|message| FormatError {
        code: "nx-text-unspellable",
        message,
        pointer: writer.pointer.clone(),
    })?;
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
            annotate_nx_text(value, &module, &[], "").expect("value should have an NX spelling")
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
                serde_json::json!({
                    "start": 0, "end": 15, "role": "record", "type": "User", "pointer": ""
                })
            );
            assert_eq!(
                json["nodes"][1],
                serde_json::json!({
                    "start": 6, "end": 12, "parent": 0, "role": "property", "type": "string",
                    "name": "id", "pointer": "/id"
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

    /// Values in canonical JSON, as the IR runtimes return them, spelled against a program.
    mod canonical {
        use crate::{
            eval_program_artifact_nx_text, format_program_artifact_value, FormatValueError,
            NxValueRole, NxValueText, ProgramBuildContext,
        };
        use serde_json::json;

        const BUTTON: &str = "external component <Button label:string emits { Tapped { } } />\n";

        fn program(source: &str) -> crate::ProgramArtifact {
            crate::load_program_artifact_from_source(
                source,
                "playground.nx",
                &ProgramBuildContext::empty(),
            )
            .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"))
        }

        fn format(source: &str, request: serde_json::Value) -> NxValueText {
            format_program_artifact_value(&program(source), &request.to_string())
                .unwrap_or_else(|error| panic!("{error:?}"))
        }

        fn failure(source: &str, request: serde_json::Value) -> crate::NxDiagnostic {
            match format_program_artifact_value(&program(source), &request.to_string()) {
                Err(FormatValueError::Value(diagnostics)) => diagnostics[0].clone(),
                other => panic!("expected a value error, got {other:?}"),
            }
        }

        fn rendered_row() -> serde_json::Value {
            json!({ "$type": "Row", "content": [{
                "$type": "Button",
                "label": "Add",
                "onTapped": { "$type": "ActionHandler", "action": "Button.Tapped", "token": "h1-1" }
            }] })
        }

        #[test]
        fn a_library_union_and_record_spell_as_root_does() {
            let source = "import \"@nx/agent\"\n\
                          type Call = { method:HttpMethod limits:AgentLimits }\n\
                          let root() = <Call method=post limits=<AgentLimits maxSteps=6 /> />\n";
            let expected = eval_program_artifact_nx_text(&program(source))
                .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
            let value = json!({
                "$type": "Call",
                "method": "post",
                "limits": { "$type": "AgentLimits", "maxSteps": 6 }
            });
            let text = format(source, json!({ "value": value }));
            assert_eq!(text.text, expected.text);
            assert!(text.text.contains("method=post"), "{}", text.text);
            let case = text
                .nodes
                .iter()
                .find(|node| node.role == NxValueRole::Case)
                .expect("the method should be a case");
            assert_eq!(case.ty.as_deref(), Some("HttpMethod"));
            assert!(
                case.declaration.is_none(),
                "a library's spans index another source"
            );
            let limits = text
                .nodes
                .iter()
                .find(|node| {
                    node.ty.as_deref() == Some("AgentLimits") && node.role == NxValueRole::Record
                })
                .expect("the limits record");
            assert!(limits.declaration.is_none());
            let steps = text
                .nodes
                .iter()
                .find(|node| node.name.as_deref() == Some("maxSteps"))
                .expect("the maxSteps property");
            assert_eq!(steps.ty.as_deref(), Some("int"));
            assert!(steps.optional);
            assert!(steps.declaration.is_none());
            assert_eq!(text.nodes, expected.nodes);
        }

        #[test]
        fn a_rendered_component_spells_its_handler_as_a_record() {
            let text = format(BUTTON, json!({ "value": rendered_row() }));
            assert_eq!(
                text.text,
                "<Row\n  content={<Button\n    label=\"Add\"\n    onTapped=<ActionHandler \
                 action=\"Button.Tapped\" token=\"h1-1\" />\n  />}\n/>"
            );
            let button = text
                .nodes
                .iter()
                .find(|node| {
                    node.ty.as_deref() == Some("Button") && node.role == NxValueRole::Record
                })
                .expect("a Button node");
            assert_eq!(
                button.declaration.as_ref().map(|span| span.start_line),
                Some(1)
            );
        }

        #[test]
        fn nodes_carry_their_values_pointers() {
            let text = format(BUTTON, json!({ "value": rendered_row() }));
            let pointers = |role: NxValueRole, ty: &str| -> Vec<String> {
                text.nodes
                    .iter()
                    .filter(|node| node.role == role && node.ty.as_deref() == Some(ty))
                    .map(|node| node.pointer.clone())
                    .collect()
            };
            assert_eq!(pointers(NxValueRole::Record, "Row"), [""]);
            assert_eq!(pointers(NxValueRole::Record, "Button"), ["/content/0"]);
            assert_eq!(
                pointers(NxValueRole::Record, "ActionHandler"),
                ["/content/0/onTapped"]
            );
            let on_tapped = text
                .nodes
                .iter()
                .find(|node| node.name.as_deref() == Some("onTapped"))
                .expect("the onTapped property");
            assert_eq!(on_tapped.pointer, "/content/0/onTapped");
        }

        #[test]
        fn a_pointer_token_escapes_tilde_and_slash() {
            let text = format("", json!({ "value": { "$type": "Odd", "a/b~c": 1 } }));
            assert_eq!(text.nodes[1].pointer, "/a~1b~0c");
        }

        #[test]
        fn a_number_is_spelled_by_its_declared_type() {
            let source = "type Size = { width:float64 height?:float32 count:int32 }";
            let text = format(
                source,
                json!({ "value": { "$type": "Size", "width": 3, "height": 2, "count": 4 } }),
            );
            assert_eq!(text.text, "<Size count=4 height=2.0 width=3.0 />");
        }

        #[test]
        fn the_type_option_types_the_whole_value() {
            assert_eq!(
                format("", json!({ "value": 3, "type": "float64" })).text,
                "3.0"
            );
            assert_eq!(format("", json!({ "value": 3 })).text, "3");
            assert_eq!(
                format("", json!({ "value": [1, 2], "type": "float64*" })).text,
                "1.0\n2.0"
            );
        }

        #[test]
        fn a_constant_case_is_read_from_its_union() {
            let source = "type Status = active | paused\ntype Job = { status:Status name:string }";
            let text = format(
                source,
                json!({ "value": { "$type": "Job", "status": "paused", "name": "paused" } }),
            );
            assert_eq!(text.text, "<Job name=\"paused\" status=paused />");
            assert!(text.nodes.iter().any(|node| node.role == NxValueRole::Case));
        }

        #[test]
        fn a_components_state_is_one_field_per_line_in_declaration_order() {
            let source = "component <Counter /> = {\n  state { total:int = 0 average:float64 = 0 }\n  <Label />\n}";
            let text = format(
                source,
                json!({ "value": { "average": 2, "total": 4 }, "state": "Counter" }),
            );
            assert_eq!(text.text, "total=4\naverage=2.0");
            let total = &text.nodes[0];
            assert_eq!(total.role, NxValueRole::Property);
            assert_eq!(total.pointer, "/total");
            assert_eq!(total.ty.as_deref(), Some("int"));
            assert_eq!(
                total.declaration.as_ref().map(|span| span.start_line),
                Some(2)
            );
        }

        #[test]
        fn a_type_that_is_not_a_string_is_invalid_and_named() {
            let diagnostic = failure("", json!({ "value": { "items": [{ "$type": 3 }] } }));
            assert_eq!(diagnostic.code.as_deref(), Some("nx-value-invalid"));
            assert!(
                diagnostic.message.contains("`/items/0/$type`"),
                "{}",
                diagnostic.message
            );
        }

        #[test]
        fn a_nested_list_is_unspellable_and_named() {
            let diagnostic = failure("", json!({ "value": { "$type": "Grid", "rows": [[1]] } }));
            assert_eq!(diagnostic.code.as_deref(), Some("nx-text-unspellable"));
            assert!(
                diagnostic.message.contains("`/rows`"),
                "{}",
                diagnostic.message
            );
        }

        #[test]
        fn a_malformed_request_is_a_request_error() {
            let error = format_program_artifact_value(&program(""), "{\"values\": 1}").unwrap_err();
            assert!(matches!(error, FormatValueError::Request(_)), "{error:?}");
        }

        /// What root evaluates to, converted to canonical JSON and spelled again, reads as
        /// `evaluateNx` spells it.
        #[test]
        fn the_text_is_root_s_text() {
            let source = "type Status = active | paused\n\
                          type Size = { width:float64 }\n\
                          type Job = { status:Status name:string size:Size tags?:string+ }\n\
                          <Job status=paused name=\"Ada\" size=<Size width=3 /> tags={\"a\" \"b\"} />";
            let artifact = program(source);
            let expected = eval_program_artifact_nx_text(&artifact).expect("root evaluates");
            let crate::EvalResult::Ok(value) = crate::eval_program_artifact(&artifact) else {
                panic!("root evaluates");
            };
            let value: serde_json::Value =
                serde_json::from_str(&value.to_json_string().unwrap()).unwrap();
            let request = json!({ "value": as_javascript(value) });
            let text = format_program_artifact_value(&artifact, &request.to_string()).unwrap();
            assert_eq!(text, expected);
        }

        /// A value as JavaScript's `JSON.stringify` writes it, where a whole number has no `.0`.
        fn as_javascript(value: serde_json::Value) -> serde_json::Value {
            match value {
                serde_json::Value::Number(number) => match number.as_f64() {
                    Some(real) if number.is_f64() && real.fract() == 0.0 => json!(real as i64),
                    _ => serde_json::Value::Number(number),
                },
                serde_json::Value::Array(items) => {
                    serde_json::Value::Array(items.into_iter().map(as_javascript).collect())
                }
                serde_json::Value::Object(properties) => serde_json::Value::Object(
                    properties
                        .into_iter()
                        .map(|(key, value)| (key, as_javascript(value)))
                        .collect(),
                ),
                other => other,
            }
        }
    }
    #[test]
    fn evaluated_color_swatches_require_canonical_declarations() {
        let entry = nx_types::analyze_str("let x = 0", "main.nx")
            .lowered_module
            .unwrap();
        let library = crate::standard_library("@nx/color").unwrap();
        let color = library
            .modules
            .iter()
            .find(|module| module.file_name == "@nx/color/color.nx")
            .unwrap()
            .lowered_module
            .as_ref()
            .unwrap();
        let value = Value::Record {
            type_name: Name::new("Color"),
            fields: [("r", 0.125_f32), ("g", 0.5), ("b", 1.0), ("a", 0.5)]
                .into_iter()
                .map(|(name, value)| (SmolStr::new(name), Value::Float32(value)))
                .collect(),
        };
        let rendered = annotate_nx_text(&value, &entry, &[color], "").unwrap();
        assert!(rendered.nodes[0].color.is_some());
        let source = "type Color = { r:float32 g:float32 b:float32 a:float32 }";
        let decoy = nx_types::analyze_str(source, "decoy.nx")
            .lowered_module
            .unwrap();
        let rendered = annotate_nx_text(&value, &decoy, &[color], source).unwrap();
        assert!(rendered.nodes[0].color.is_none());
    }
}
