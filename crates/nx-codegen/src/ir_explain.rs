//! Renders an NX IR artifact as text with every table index resolved.
//!
//! <para>The tables are for machines. This is the one reader for people: declarations by name and
//! kind, references by module identity and declaration name, types spelled as NX spells them, nodes
//! as an indented expression tree, and spans as line and column when the debug section is present.
//! Tests and the CLI both go through it, so there is no second interpretation of the tables to
//! drift from this one.</para>

use crate::ir::{kinds, IrItem, NxIrArtifact};
use crate::ir_image::{NxIrImage, NxIrImageError};
use std::fmt::Write as _;

/// Why an artifact could not be explained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplainError {
    /// The bytes are not an NX IR image, or the image or model is not of the expected shape.
    Malformed(String),
    /// The artifact is of a schema version this crate does not read.
    SchemaVersion { found: u32, supported: u32 },
}

impl From<NxIrImageError> for ExplainError {
    fn from(error: NxIrImageError) -> Self {
        match error {
            NxIrImageError::NotAnImage => {
                Self::Malformed("the input is not an NX IR image".to_string())
            }
            NxIrImageError::SchemaVersion { found, supported } => {
                Self::SchemaVersion { found, supported }
            }
            NxIrImageError::Malformed(message) => Self::Malformed(message),
        }
    }
}

impl std::fmt::Display for ExplainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(message) => write!(formatter, "NX IR artifact is malformed: {message}"),
            Self::SchemaVersion { found, supported } => write!(
                formatter,
                "NX IR schema version {found} is not supported; this build reads schema version {supported}"
            ),
        }
    }
}

impl std::error::Error for ExplainError {}

/// Explains an artifact given as an image. The image is validated before it is read, so a
/// malformed or truncated file is an error, never a panic.
pub fn explain_nx_ir_image(bytes: &[u8]) -> Result<String, ExplainError> {
    let image = NxIrImage::open(bytes)?;
    explain_nx_ir(&image.to_artifact())
}

/// Explains an artifact model.
pub fn explain_nx_ir(artifact: &NxIrArtifact) -> Result<String, ExplainError> {
    Explainer { artifact }.explain()
}

struct Explainer<'a> {
    artifact: &'a NxIrArtifact,
}

type Lines = Vec<String>;

impl<'a> Explainer<'a> {
    fn explain(&self) -> Result<String, ExplainError> {
        let artifact = self.artifact;
        let mut output = String::new();
        let own = artifact
            .modules
            .first()
            .ok_or_else(|| self.malformed("the module table is empty"))?;
        let _ = writeln!(
            output,
            "module {} fingerprint {}",
            own.identity, own.fingerprint
        );
        for entry in artifact.modules.iter().skip(1) {
            let _ = writeln!(
                output,
                "links {} version {} fingerprint {}",
                entry.identity,
                json_string(&entry.version),
                entry.fingerprint
            );
        }
        if !artifact.required_features.is_empty() {
            let _ = writeln!(output, "requires {}", artifact.required_features.join(" "));
        }
        let entrypoints = |indices: &[u32]| -> Result<String, ExplainError> {
            Ok(indices
                .iter()
                .map(|index| self.declaration_name(*index as usize))
                .collect::<Result<Vec<_>, _>>()?
                .join(" "))
        };
        let _ = writeln!(
            output,
            "entrypoints functions [{}] components [{}]",
            entrypoints(&artifact.function_entrypoints)?,
            entrypoints(&artifact.component_entrypoints)?
        );
        for (index, declaration) in artifact.declarations.iter().enumerate() {
            output.push('\n');
            for line in self.declaration(index, declaration)? {
                output.push_str(&line);
                output.push('\n');
            }
        }
        Ok(output)
    }

    fn malformed(&self, message: impl Into<String>) -> ExplainError {
        ExplainError::Malformed(message.into())
    }

    fn list<'b>(&self, item: &'b IrItem, what: &str) -> Result<&'b [IrItem], ExplainError> {
        item.as_list()
            .ok_or_else(|| self.malformed(format!("{what} is not a list")))
    }

    fn int(&self, item: &IrItem, what: &str) -> Result<i64, ExplainError> {
        item.as_int()
            .ok_or_else(|| self.malformed(format!("{what} is not an integer")))
    }

    fn operand<'b>(
        &self,
        entry: &'b [IrItem],
        index: usize,
        what: &str,
    ) -> Result<&'b IrItem, ExplainError> {
        entry
            .get(index)
            .ok_or_else(|| self.malformed(format!("{what} is missing operand {index}")))
    }

    fn int_operand(&self, entry: &[IrItem], index: usize, what: &str) -> Result<i64, ExplainError> {
        self.int(self.operand(entry, index, what)?, what)
    }

    fn list_operand<'b>(
        &self,
        entry: &'b [IrItem],
        index: usize,
        what: &str,
    ) -> Result<&'b [IrItem], ExplainError> {
        self.list(self.operand(entry, index, what)?, what)
    }

    fn string(&self, index: i64) -> Result<&'a str, ExplainError> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.artifact.strings.get(index))
            .map(String::as_str)
            .ok_or_else(|| self.malformed(format!("string index {index} is out of range")))
    }

    fn declaration_name(&self, index: usize) -> Result<&'a str, ExplainError> {
        let entry =
            self.artifact.declarations.get(index).ok_or_else(|| {
                self.malformed(format!("declaration index {index} is out of range"))
            })?;
        let entry = self.list(entry, "declaration")?;
        self.string(self.int_operand(entry, 1, "declaration")?)
    }

    fn module_identity(&self, slot: i64) -> Result<&'a str, ExplainError> {
        usize::try_from(slot)
            .ok()
            .and_then(|slot| self.artifact.modules.get(slot))
            .map(|entry| entry.identity.as_str())
            .ok_or_else(|| self.malformed(format!("module slot {slot} is out of range")))
    }

    /// A reference as `name` for the artifact's own module and `identity:name` for another.
    fn reference(&self, slot: i64, name: i64) -> Result<String, ExplainError> {
        let name = self.string(name)?;
        if slot == 0 {
            Ok(name.to_string())
        } else {
            Ok(format!("{}:{}", self.module_identity(slot)?, name))
        }
    }

    fn reference_item(&self, item: &IrItem) -> Result<String, ExplainError> {
        let entry = self.list(item, "reference")?;
        self.reference(
            self.int_operand(entry, 0, "reference")?,
            self.int_operand(entry, 1, "reference")?,
        )
    }

    fn ty(&self, index: i64) -> Result<String, ExplainError> {
        let entry = usize::try_from(index)
            .ok()
            .and_then(|index| self.artifact.types.get(index))
            .ok_or_else(|| self.malformed(format!("type index {index} is out of range")))?;
        let entry = self.list(entry, "type")?;
        let kind = self.int_operand(entry, 0, "type")?;
        Ok(match kind {
            kinds::ty::PRIMITIVE => self
                .string(self.int_operand(entry, 1, "primitive type")?)?
                .to_string(),
            kinds::ty::NOMINAL => self.reference(
                self.int_operand(entry, 1, "nominal type")?,
                self.int_operand(entry, 2, "nominal type")?,
            )?,
            kinds::ty::ARRAY => {
                format!("{}[]", self.ty(self.int_operand(entry, 1, "array type")?)?)
            }
            kinds::ty::NULLABLE => format!(
                "{}?",
                self.ty(self.int_operand(entry, 1, "nullable type")?)?
            ),
            other => return Err(self.malformed(format!("unknown type kind {other}"))),
        })
    }

    fn span_suffix(&self, spans: &[[i64; 2]], index: usize) -> String {
        let Some(debug) = &self.artifact.debug else {
            return String::new();
        };
        let Some([start, end]) = spans.get(index) else {
            return String::new();
        };
        if *start < 0 {
            return String::new();
        }
        let (line, column) = line_column(&debug.source, *start as usize);
        let (end_line, end_column) = line_column(&debug.source, *end as usize);
        format!(" @{line}:{column}-{end_line}:{end_column}")
    }

    fn declaration(&self, index: usize, declaration: &IrItem) -> Result<Lines, ExplainError> {
        let entry = self.list(declaration, "declaration")?;
        let kind = self.int_operand(entry, 0, "declaration")?;
        let name = self.string(self.int_operand(entry, 1, "declaration")?)?;
        let span = self
            .artifact
            .debug
            .as_ref()
            .map(|debug| self.span_suffix(&debug.spans.declarations, index))
            .unwrap_or_default();
        let mut lines = Vec::new();
        match kind {
            kinds::declaration::FUNCTION => {
                let params = self
                    .list_operand(entry, 2, "function")?
                    .iter()
                    .map(|param| {
                        let param = self.list(param, "parameter")?;
                        let content = if self.int_operand(param, 2, "parameter")? != 0 {
                            "content "
                        } else {
                            ""
                        };
                        Ok(format!(
                            "{content}{}: {}",
                            self.string(self.int_operand(param, 0, "parameter")?)?,
                            self.ty(self.int_operand(param, 1, "parameter")?)?
                        ))
                    })
                    .collect::<Result<Vec<_>, ExplainError>>()?;
                lines.push(format!("function {name}({}){span} =", params.join(", ")));
                lines.extend(indent(self.node(self.int_operand(entry, 3, "function")?)?));
            }
            kinds::declaration::VALUE => {
                lines.push(format!("value {name}{span} ="));
                lines.extend(indent(self.node(self.int_operand(entry, 2, "value")?)?));
            }
            kinds::declaration::RECORD => {
                let bases = self.references_suffix(self.list_operand(entry, 3, "record")?)?;
                let is_abstract = if self.int_operand(entry, 4, "record")? != 0 {
                    " abstract"
                } else {
                    ""
                };
                let update_target =
                    self.optional_reference_suffix(self.operand(entry, 5, "record")?, "update of")?;
                lines.push(format!(
                    "record {name}{is_abstract}{bases}{update_target}{span}"
                ));
                lines.extend(indent(self.fields(self.list_operand(entry, 2, "record")?)?));
            }
            kinds::declaration::COMPONENT => {
                let flags = self.int_operand(entry, 5, "component")?;
                let mut modifiers = String::new();
                if flags & kinds::component::ABSTRACT != 0 {
                    modifiers.push_str(" abstract");
                }
                if flags & kinds::component::EXTERNAL != 0 {
                    modifiers.push_str(" external");
                }
                lines.push(format!("component {name}{modifiers}{span}"));
                let props = self.fields(self.list_operand(entry, 2, "component")?)?;
                if !props.is_empty() {
                    lines.push("  props".to_string());
                    lines.extend(indent(indent(props)));
                }
                let state = self.fields(self.list_operand(entry, 3, "component")?)?;
                if !state.is_empty() {
                    lines.push("  state".to_string());
                    lines.extend(indent(indent(state)));
                }
                let body = self.int_operand(entry, 4, "component")?;
                if body >= 0 {
                    lines.push("  body =".to_string());
                    lines.extend(indent(indent(self.node(body)?)));
                }
            }
            kinds::declaration::UNION => {
                let bases = self.references_suffix(self.list_operand(entry, 3, "union")?)?;
                let property_target = self
                    .optional_reference_suffix(self.operand(entry, 4, "union")?, "property of")?;
                lines.push(format!("union {name}{bases}{property_target}{span}"));
                for case in self.list_operand(entry, 2, "union")? {
                    let case = self.list(case, "union case")?;
                    let case_name = self.string(self.int_operand(case, 0, "union case")?)?;
                    let constant = if self.int_operand(case, 2, "union case")? != 0 {
                        " constant"
                    } else {
                        ""
                    };
                    lines.push(format!("  case {case_name}{constant}"));
                    lines.extend(indent(indent(self.fields(self.list_operand(
                        case,
                        1,
                        "union case",
                    )?)?)));
                }
            }
            kinds::declaration::TYPE_ALIAS => lines.push(format!("alias {name}{span}")),
            other => return Err(self.malformed(format!("unknown declaration kind {other}"))),
        }
        Ok(lines)
    }

    fn references_suffix(&self, references: &[IrItem]) -> Result<String, ExplainError> {
        if references.is_empty() {
            return Ok(String::new());
        }
        let names = references
            .iter()
            .map(|reference| self.reference_item(reference))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!(" extends {}", names.join(", ")))
    }

    fn optional_reference_suffix(
        &self,
        reference: &IrItem,
        label: &str,
    ) -> Result<String, ExplainError> {
        let entry = self.list(reference, "optional reference")?;
        if entry.is_empty() {
            return Ok(String::new());
        }
        Ok(format!(" {label} {}", self.reference_item(reference)?))
    }

    fn fields(&self, fields: &[IrItem]) -> Result<Lines, ExplainError> {
        let mut lines = Vec::new();
        for field in fields {
            let field = self.list(field, "field")?;
            let name = self.string(self.int_operand(field, 0, "field")?)?;
            let ty = self.ty(self.int_operand(field, 1, "field")?)?;
            let default = self.int_operand(field, 2, "field")?;
            let flags = self.int_operand(field, 3, "field")?;
            let mut line = String::new();
            if flags & kinds::field::CONTENT != 0 {
                line.push_str("content ");
            }
            let _ = write!(line, "{name}: {ty}");
            if flags & kinds::field::REQUIRED != 0 {
                line.push_str(" required");
            }
            if default >= 0 {
                let value = self.node(default)?;
                if value.len() == 1 {
                    let _ = write!(line, " = {}", value[0]);
                    lines.push(line);
                } else {
                    line.push_str(" =");
                    lines.push(line);
                    lines.extend(indent(value));
                }
            } else {
                lines.push(line);
            }
        }
        Ok(lines)
    }

    fn node_entry(&self, index: i64) -> Result<&'a [IrItem], ExplainError> {
        let entry = usize::try_from(index)
            .ok()
            .and_then(|index| self.artifact.nodes.get(index))
            .ok_or_else(|| self.malformed(format!("node index {index} is out of range")))?;
        self.list(entry, "node")
    }

    /// Renders a node as one or more lines; a node that fits on one line is one line.
    fn node(&self, index: i64) -> Result<Lines, ExplainError> {
        let entry = self.node_entry(index)?;
        let kind = self.int_operand(entry, 0, "node")?;
        Ok(match kind {
            kinds::node::NULL => vec!["null".to_string()],
            kinds::node::BOOL => vec![if self.int_operand(entry, 1, "bool")? != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }],
            kinds::node::STRING => vec![json_string(
                self.string(self.int_operand(entry, 1, "string")?)?,
            )],
            kinds::node::NUMBER => vec![self.constant(self.int_operand(entry, 1, "number")?)?],
            kinds::node::SLOT => vec![self
                .string(self.int_operand(entry, 2, "slot")?)?
                .to_string()],
            kinds::node::REFERENCE => vec![self.reference(
                self.int_operand(entry, 1, "reference")?,
                self.int_operand(entry, 2, "reference")?,
            )?],
            kinds::node::BINARY => {
                let op = self.int_operand(entry, 1, "binary")?;
                let op = kinds::name(kinds::binary::NAMES, op)
                    .ok_or_else(|| self.malformed(format!("unknown binary operator {op}")))?;
                let lhs = self.node(self.int_operand(entry, 2, "binary")?)?;
                let rhs = self.node(self.int_operand(entry, 3, "binary")?)?;
                join_inline(&[lhs, vec![op.to_string()], rhs], "(", " ", ")")
            }
            kinds::node::UNARY => {
                let op = self.int_operand(entry, 1, "unary")?;
                let op = kinds::name(kinds::unary::NAMES, op)
                    .ok_or_else(|| self.malformed(format!("unknown unary operator {op}")))?;
                let operand = self.node(self.int_operand(entry, 2, "unary")?)?;
                prefixed(format!("{op} "), operand)
            }
            kinds::node::CALL => {
                let callee = self.node(self.int_operand(entry, 1, "call")?)?;
                let args = self.nodes(self.list_operand(entry, 2, "call")?)?;
                let callee = callee.join(" ");
                call_like(callee, args)
            }
            kinds::node::INTRINSIC => {
                let op = self.int_operand(entry, 1, "intrinsic")?;
                let op = kinds::name(kinds::intrinsic::NAMES, op)
                    .ok_or_else(|| self.malformed(format!("unknown intrinsic {op}")))?;
                let args = self.nodes(self.list_operand(entry, 2, "intrinsic")?)?;
                let order = self
                    .list_operand(entry, 3, "intrinsic")?
                    .iter()
                    .map(|name| self.string(self.int(name, "field order")?))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut lines = call_like(op.to_string(), args);
                if !order.is_empty() {
                    let last = lines.len() - 1;
                    let _ = write!(lines[last], " ordering {}", order.join(", "));
                }
                lines
            }
            kinds::node::IF => {
                let mut lines = prefixed(
                    "if ".to_string(),
                    self.node(self.int_operand(entry, 1, "if")?)?,
                );
                let last = lines.len() - 1;
                lines[last].push_str(" then");
                lines.extend(indent(self.node(self.int_operand(entry, 2, "if")?)?));
                let else_branch = self.int_operand(entry, 3, "if")?;
                if else_branch >= 0 {
                    lines.push("else".to_string());
                    lines.extend(indent(self.node(else_branch)?));
                }
                lines
            }
            kinds::node::IF_IS => {
                let mut lines = prefixed(
                    "if ".to_string(),
                    self.node(self.int_operand(entry, 1, "ifIs")?)?,
                );
                let last = lines.len() - 1;
                lines[last].push_str(" is");
                for arm in self.list_operand(entry, 2, "ifIs")? {
                    let arm = self.list(arm, "arm")?;
                    let patterns = self.nodes(self.list_operand(arm, 0, "arm")?)?;
                    let mut pattern_lines = join_inline(&patterns, "", " | ", "");
                    let last = pattern_lines.len() - 1;
                    pattern_lines[last].push_str(" =>");
                    lines.extend(indent(pattern_lines));
                    lines.extend(indent(indent(self.node(self.int_operand(arm, 1, "arm")?)?)));
                }
                let else_branch = self.int_operand(entry, 3, "ifIs")?;
                if else_branch >= 0 {
                    lines.push("  else =>".to_string());
                    lines.extend(indent(indent(self.node(else_branch)?)));
                }
                lines
            }
            kinds::node::ARRAY => {
                let elements = self.nodes(self.list_operand(entry, 1, "array")?)?;
                join_inline(&elements, "[", ", ", "]")
            }
            kinds::node::FOR => {
                let item = self.string(self.int_operand(entry, 2, "for")?)?;
                let index_name = self.int_operand(entry, 4, "for")?;
                let binding = if index_name >= 0 {
                    format!("{item}, {}", self.string(index_name)?)
                } else {
                    item.to_string()
                };
                let mut lines = prefixed(
                    format!("for {binding} in "),
                    self.node(self.int_operand(entry, 5, "for")?)?,
                );
                let last = lines.len() - 1;
                lines[last].push_str(" yield");
                lines.extend(indent(self.node(self.int_operand(entry, 6, "for")?)?));
                lines
            }
            kinds::node::MEMBER => {
                let base = self.node(self.int_operand(entry, 1, "member")?)?;
                let member = self.string(self.int_operand(entry, 2, "member")?)?;
                let mut lines = base;
                let last = lines.len() - 1;
                let _ = write!(lines[last], ".{member}");
                lines
            }
            kinds::node::RECORD => {
                let name = self.reference(
                    self.int_operand(entry, 1, "record")?,
                    self.int_operand(entry, 2, "record")?,
                )?;
                self.element_like(
                    name,
                    self.list_operand(entry, 3, "record")?,
                    self.list_operand(entry, 4, "record")?,
                )?
            }
            kinds::node::UNION_CASE => {
                let union = self.reference(
                    self.int_operand(entry, 1, "unionCase")?,
                    self.int_operand(entry, 2, "unionCase")?,
                )?;
                let case = self.string(self.int_operand(entry, 3, "unionCase")?)?;
                let properties = self.list_operand(entry, 4, "unionCase")?;
                let content = self.list_operand(entry, 5, "unionCase")?;
                if properties.is_empty() && content.is_empty() {
                    vec![format!("{union}.{case}")]
                } else {
                    self.element_like(format!("{union}.{case}"), properties, content)?
                }
            }
            kinds::node::ELEMENT => {
                let tag = self.string(self.int_operand(entry, 2, "element")?)?;
                self.element_like(
                    tag.to_string(),
                    self.list_operand(entry, 3, "element")?,
                    self.list_operand(entry, 4, "element")?,
                )?
            }
            kinds::node::COMPONENT => {
                let name = self.reference(
                    self.int_operand(entry, 1, "component")?,
                    self.int_operand(entry, 2, "component")?,
                )?;
                self.element_like(
                    name,
                    self.list_operand(entry, 3, "component")?,
                    self.list_operand(entry, 4, "component")?,
                )?
            }
            other => return Err(self.malformed(format!("unknown node kind {other}"))),
        })
    }

    fn nodes(&self, indices: &[IrItem]) -> Result<Vec<Lines>, ExplainError> {
        indices
            .iter()
            .map(|index| self.node(self.int(index, "node index")?))
            .collect()
    }

    fn constant(&self, index: i64) -> Result<String, ExplainError> {
        let entry = usize::try_from(index)
            .ok()
            .and_then(|index| self.artifact.constants.get(index))
            .ok_or_else(|| self.malformed(format!("constant index {index} is out of range")))?;
        let entry = self.list(entry, "constant")?;
        let kind = self.int_operand(entry, 0, "constant")?;
        let value = self.operand(entry, 1, "constant")?;
        Ok(match (kind, value) {
            (kinds::constant::BIGINT, IrItem::Int(digits)) => self.string(*digits)?.to_string(),
            (_, IrItem::Int(value)) => value.to_string(),
            (_, IrItem::Float(value)) => format_float(*value),
            (_, IrItem::List(_)) => return Err(self.malformed("a constant is a list")),
        })
    }

    /// Renders a construction the way NX writes it: `<Name prop=value>content</Name>`.
    fn element_like(
        &self,
        name: String,
        properties: &[IrItem],
        content: &[IrItem],
    ) -> Result<Lines, ExplainError> {
        let mut head = format!("<{name}");
        let mut trailing = Vec::new();
        for property in properties {
            let property = self.list(property, "property")?;
            let key = self.string(self.int_operand(property, 0, "property")?)?;
            let value = self.node(self.int_operand(property, 1, "property")?)?;
            if value.len() == 1 {
                let _ = write!(head, " {key}={}", value[0]);
            } else {
                let mut lines = prefixed(format!("{key}="), value);
                trailing.append(&mut lines);
            }
        }
        let content = self.nodes(content)?;
        let mut lines = Vec::new();
        if trailing.is_empty() && content.is_empty() {
            lines.push(format!("{head} />"));
            return Ok(lines);
        }
        if trailing.is_empty() {
            head.push('>');
            lines.push(head);
        } else {
            lines.push(head);
            lines.extend(indent(trailing));
            lines.push(">".to_string());
        }
        for child in content {
            lines.extend(indent(child));
        }
        lines.push(format!("</{name}>"));
        Ok(lines)
    }
}

fn indent(lines: Lines) -> Lines {
    lines.into_iter().map(|line| format!("  {line}")).collect()
}

/// Prepends `prefix` to the first line of a rendering.
fn prefixed(prefix: String, mut lines: Lines) -> Lines {
    if lines.is_empty() {
        lines.push(prefix);
    } else {
        lines[0] = format!("{prefix}{}", lines[0]);
    }
    lines
}

/// Joins renderings on one line when each fits on one; otherwise one per line, indented.
fn join_inline(parts: &[Lines], open: &str, separator: &str, close: &str) -> Lines {
    if parts.iter().all(|part| part.len() == 1) {
        let inline = parts
            .iter()
            .map(|part| part[0].as_str())
            .collect::<Vec<_>>()
            .join(separator);
        return vec![format!("{open}{inline}{close}")];
    }
    let mut lines = vec![open.trim_end().to_string()];
    for part in parts {
        lines.extend(indent(part.clone()));
    }
    lines.push(close.trim_start().to_string());
    lines.into_iter().filter(|line| !line.is_empty()).collect()
}

fn call_like(callee: String, args: Vec<Lines>) -> Lines {
    if args.iter().all(|arg| arg.len() == 1) {
        let inline = args
            .iter()
            .map(|arg| arg[0].as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return vec![format!("{callee}({inline})")];
    }
    let mut lines = vec![format!("{callee}(")];
    for arg in args {
        lines.extend(indent(arg));
    }
    lines.push(")".to_string());
    lines
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

/// One-based line and column of a byte offset, counting columns in characters.
///
/// <para>The offset comes from an artifact, which may have been written by another toolchain or
/// edited by hand, so it need not land on a character boundary of this source. An offset past the
/// end reads as the end, and one inside a character reads as that character's start, rather than
/// panicking the tool that exists to explain such artifacts.</para>
fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map(|last| last.chars().count())
        .unwrap_or(0)
        + 1;
    (line, column)
}
