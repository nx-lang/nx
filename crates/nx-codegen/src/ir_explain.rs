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
            kinds::ty::SEQ => {
                let cell = self.int_operand(entry, 2, "seq type")?;
                let occ = kinds::ty::occurrence_from_cell(cell)
                    .ok_or_else(|| self.malformed(format!("unknown occurrence {cell}")))?;
                format!(
                    "{}{occ}",
                    self.ty_under_suffix(self.int_operand(entry, 1, "seq type")?)?
                )
            }
            // Retired with schema 5; a schema-5 image never carries them.
            kinds::ty::ARRAY | kinds::ty::NULLABLE => {
                return Err(self.malformed(format!(
                    "type kind {kind} was retired with schema 5; a sequence is the seq kind"
                )))
            }
            kinds::ty::FUNCTION => {
                // The parts are read here; the spelling is `nx-hir`'s, shared with the checker's
                // diagnostics and the editor's hovers.
                let mut params: Vec<(bool, String, bool, String)> = Vec::new();
                for param in self.list_operand(entry, 2, "function type")? {
                    let param = self.list(param, "function type parameter")?;
                    let name = self.string(self.int_operand(param, 0, "parameter name")?)?;
                    let ty = self.ty(self.int_operand(param, 1, "parameter type")?)?;
                    let flags = self.int_operand(param, 2, "parameter flags")?;
                    let optional = flags & kinds::ty::FUNCTION_PARAM_OPTIONAL != 0;
                    // The artifact records an optional parameter's read type; source spells the
                    // declared type after `name?:`, so the zero the mark adds comes back off.
                    let ty = if optional {
                        declared_from_read_type(&ty)
                    } else {
                        ty
                    };
                    params.push((
                        flags & kinds::ty::FUNCTION_PARAM_CONTENT != 0,
                        name.to_string(),
                        optional,
                        ty,
                    ));
                }
                let result = self.ty(self.int_operand(entry, 1, "function result")?)?;
                nx_hir::ast::spell_function_type(
                    params.iter().map(|(is_content, name, optional, ty)| {
                        nx_hir::ast::SpelledParam {
                            is_content: *is_content,
                            name,
                            optional: *optional,
                            ty,
                        }
                    }),
                    &result,
                )
            }
            other => return Err(self.malformed(format!("unknown type kind {other}"))),
        })
    }

    /// A type under an occurrence suffix, parenthesized when it is a function type, because a
    /// suffix written after a function type's result binds to the result.
    fn ty_under_suffix(&self, index: i64) -> Result<String, ExplainError> {
        let text = self.ty(index)?;
        let is_function = usize::try_from(index)
            .ok()
            .and_then(|index| self.artifact.types.get(index))
            .and_then(|entry| entry.as_list())
            .and_then(|entry| entry.first())
            .and_then(IrItem::as_int)
            == Some(kinds::ty::FUNCTION);
        Ok(if is_function {
            format!("({text})")
        } else {
            text
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
                        let default = self.int_operand(param, 2, "parameter")?;
                        let flags = self.int_operand(param, 3, "parameter")?;
                        let content = if flags & kinds::ty::FUNCTION_PARAM_CONTENT != 0 {
                            "content "
                        } else {
                            ""
                        };
                        // The parameter is typed by its read type; an optional one is shown as
                        // source declares it, `name?: T`.
                        let ty = self.ty(self.int_operand(param, 1, "parameter")?)?;
                        let (mark, ty) = if flags & kinds::ty::FUNCTION_PARAM_OPTIONAL != 0 {
                            ("?", declared_from_read_type(&ty))
                        } else {
                            ("", ty)
                        };
                        let default = if default >= 0 {
                            format!(" = {}", self.node(default)?.join(" "))
                        } else {
                            String::new()
                        };
                        Ok(format!(
                            "{content}{}{mark}: {ty}{default}",
                            self.string(self.int_operand(param, 0, "parameter")?)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, ExplainError>>()?;
                let flags = self.int_operand(entry, 5, "function")?;
                let optional = if flags & kinds::declaration::RESULT_OPTIONAL != 0 {
                    " (optional result)"
                } else {
                    ""
                };
                let result = format!("{}{optional}", self.declared_suffix(entry, 4, "function")?);
                lines.push(format!(
                    "function {name}({}){result}{span} =",
                    params.join(", ")
                ));
                lines.extend(indent(self.node(self.int_operand(entry, 3, "function")?)?));
            }
            kinds::declaration::VALUE => {
                let result = self.declared_suffix(entry, 3, "value")?;
                lines.push(format!("value {name}{result}{span} ="));
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
                let emits = self.list_operand(entry, 6, "component")?;
                if !emits.is_empty() {
                    lines.push("  emits".to_string());
                    for emit in emits {
                        let emit = self.list(emit, "emit")?;
                        let name = self.string(self.int_operand(emit, 0, "emit")?)?;
                        let action = self.reference_item(self.operand(emit, 1, "emit")?)?;
                        lines.push(format!("    {name} = {action}"));
                    }
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

    /// `: T` for the declared type at `at`, or nothing when the declaration declares none.
    fn declared_suffix(
        &self,
        entry: &[IrItem],
        at: usize,
        what: &str,
    ) -> Result<String, ExplainError> {
        match self.int_operand(entry, at, what)? {
            -1 => Ok(String::new()),
            declared => Ok(format!(": {}", self.ty(declared)?)),
        }
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
            // Retired with schema 5: the language has no null value.
            kinds::node::NULL => return Err(self.malformed(
                "node kind 0 (null) was retired with schema 5; the empty value is an empty array",
            )),
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
            kinds::node::TEXT => {
                let operand = self.node(self.int_operand(entry, 1, "text")?)?;
                let ty = self.string(self.int_operand(entry, 2, "text")?)?;
                call_like(format!("text<{ty}>"), vec![operand])
            }
            // An argument the call left out is shown as `_`; the function fills it.
            kinds::node::CALL => {
                let callee = self.node(self.int_operand(entry, 1, "call")?)?;
                let args = self
                    .list_operand(entry, 2, "call")?
                    .iter()
                    .map(|arg| match self.int(arg, "node index")? {
                        -1 => Ok(vec!["_".to_string()]),
                        index => self.node(index),
                    })
                    .collect::<Result<Vec<_>, ExplainError>>()?;
                let callee = callee.join(" ");
                call_like(callee, args)
            }
            // The element form is the source form of a call by name, so it is rendered as one.
            kinds::node::NAMED_CALL => {
                let callee = self.node(self.int_operand(entry, 1, "namedCall")?)?;
                self.element_like(
                    callee.join(" "),
                    self.list_operand(entry, 2, "namedCall")?,
                    &[],
                )?
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
                // An array with no elements is the empty value, spelled as source spells it, and
                // as the `{}` pattern it also is.
                if elements.is_empty() {
                    vec!["{}".to_string()]
                } else {
                    join_inline(&elements, "[", ", ", "]")
                }
            }
            // A range loop reads as a `for` over a range, because that is what it is: the kind
            // says what the iterable evaluates to, not how the loop is written.
            kinds::node::FOR | kinds::node::FOR_RANGE => {
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
            kinds::node::OPTIONAL_MEMBER => {
                let base = self.node(self.int_operand(entry, 1, "optionalMember")?)?;
                let member = self.string(self.int_operand(entry, 2, "optionalMember")?)?;
                let mut lines = base;
                let last = lines.len() - 1;
                let _ = write!(lines[last], "?.{member}");
                lines
            }
            kinds::node::EXISTS => {
                let mut lines = self.node(self.int_operand(entry, 1, "exists")?)?;
                let last = lines.len() - 1;
                lines[last].push('?');
                lines
            }
            kinds::node::COALESCE => {
                let lhs = self.node(self.int_operand(entry, 1, "coalesce")?)?;
                let rhs = self.node(self.int_operand(entry, 2, "coalesce")?)?;
                join_inline(&[lhs, vec!["??".to_string()], rhs], "(", " ", ")")
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
            kinds::node::ACTION_HANDLER => {
                let component = self.reference(
                    self.int_operand(entry, 1, "actionHandler")?,
                    self.int_operand(entry, 2, "actionHandler")?,
                )?;
                let emit = self.string(self.int_operand(entry, 3, "actionHandler")?)?;
                let action = self.reference(
                    self.int_operand(entry, 4, "actionHandler")?,
                    self.int_operand(entry, 5, "actionHandler")?,
                )?;
                let slot = self.int_operand(entry, 6, "actionHandler")?;
                let owner = self
                    .optional_reference_suffix(self.operand(entry, 7, "actionHandler")?, "owner")?;
                // The slot is printed because the body's reads of `action` print only the name.
                let mut lines = vec![format!(
                    "handler {component}.{emit} action@{slot}:{action}{owner} =>"
                )];
                lines.extend(indent(self.node(self.int_operand(
                    entry,
                    8,
                    "actionHandler",
                )?)?));
                lines
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

/// The declared type of an optional parameter, spelled, from its spelled read type: `T` from
/// `T?`, `T+` from `T*`. A function type under the suffix loses the parentheses the suffix needed.
fn declared_from_read_type(read: &str) -> String {
    let (base, suffix) = if let Some(base) = read.strip_suffix('?') {
        (base, "")
    } else if let Some(base) = read.strip_suffix('*') {
        (base, "+")
    } else {
        return read.to_string();
    };
    let base = if suffix.is_empty() && base.starts_with("(<function") && base.ends_with(')') {
        &base[1..base.len() - 1]
    } else {
        base
    };
    format!("{base}{suffix}")
}
