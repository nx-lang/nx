//! A prepared module: one image, validated, with its declarations indexed by name.
//!
//! <para>Preparation validates the whole image once, through the reader in `nx-ir`, and decodes
//! the type table and the declaration list, which is what linking and the host boundary need. The
//! node table is decoded a node at a time, the first time evaluation reaches the node, so a
//! snippet linked against a large catalog pays for the controls it uses.</para>

use crate::error::{Diagnostic, NxIrRuntimeError, Result};
use crate::value::Value;
use nx_ir::{
    kinds, Cells, NxIrImage, NxIrImageBuf, NxIrImageError, Table, NONE,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1, NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1,
    NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1, NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1,
    NX_IR_REQUIRED_FEATURE_RANGES_V1, NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1, NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION,
};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

const KNOWN_FEATURES: [&str; 7] = [
    NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
    NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1,
    NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1,
    NX_IR_REQUIRED_FEATURE_RANGES_V1,
    NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1,
];

/// How deeply one type may nest others. A type is a tree of shared nodes that is dropped
/// recursively, so an image whose types chain without end is refused rather than prepared.
const MAX_TYPE_DEPTH: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Primitive {
    Int,
    Int32,
    Int64,
    Float32,
    Float64,
    String,
    Boolean,
    Object,
}

/// A type as the image spells it. A `Seq` carries an occurrence over an exactly-one item type.
#[derive(Debug)]
pub(crate) enum Type {
    Primitive(Primitive),
    /// A primitive name this runtime does not know, which fails where a value meets it.
    UnknownPrimitive(Arc<str>),
    Nominal(Ref),
    Seq {
        item: Arc<Type>,
        may_be_empty: bool,
        may_be_many: bool,
    },
    Function,
}

impl Type {
    /// Whether a site of this type admits the empty value: a `?` or `*` occurrence.
    pub(crate) fn admits_empty(&self) -> bool {
        matches!(
            self,
            Type::Seq {
                may_be_empty: true,
                ..
            }
        )
    }

    /// Whether a site of this type holds a list: a `+` or `*` occurrence.
    pub(crate) fn admits_many(&self) -> bool {
        matches!(
            self,
            Type::Seq {
                may_be_many: true,
                ..
            }
        )
    }
}

/// A reference to a declaration: a slot of the module table and a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ref {
    pub slot: u32,
    pub name: Arc<str>,
}

#[derive(Debug)]
pub(crate) struct Field {
    pub name: Arc<str>,
    pub ty: Arc<Type>,
    pub default: Option<u32>,
    pub is_content: bool,
    pub is_required: bool,
}

#[derive(Debug)]
pub(crate) struct Param {
    pub name: Arc<str>,
    /// The parameter's read type: `T?` for one declared `p?:T`.
    pub ty: Arc<Type>,
    pub default: Option<u32>,
    pub is_content: bool,
    pub is_optional: bool,
}

#[derive(Debug)]
pub(crate) struct UnionCase {
    pub name: Arc<str>,
    pub fields: Arc<[Field]>,
    pub is_constant: bool,
}

/// One action a component emits: the local name a parent binds as `on<Name>`, and its record.
#[derive(Debug)]
pub(crate) struct Emit {
    pub name: Arc<str>,
    pub action: Ref,
}

#[derive(Debug)]
pub(crate) struct FunctionDecl {
    pub params: Vec<Param>,
    pub body: u32,
    pub result: Option<Arc<Type>>,
    /// The result type, declared or inferred, is a standalone `T?`.
    pub is_optional_result: bool,
}

#[derive(Debug)]
pub(crate) struct RecordDecl {
    pub fields: Arc<[Field]>,
    pub bases: Arc<[Ref]>,
    pub is_abstract: bool,
    pub update_target: Option<Ref>,
}

#[derive(Debug)]
pub(crate) struct ComponentDecl {
    pub props: Arc<[Field]>,
    pub state: Arc<[Field]>,
    pub body: Option<u32>,
    pub is_abstract: bool,
    /// The effective emits, inherited included, in declaration order.
    pub emits: Vec<Emit>,
}

#[derive(Debug)]
pub(crate) struct UnionDecl {
    pub cases: Vec<UnionCase>,
    pub bases: Arc<[Ref]>,
}

#[derive(Debug)]
pub(crate) enum DeclarationKind {
    Function(FunctionDecl),
    Value { value: u32, ty: Option<Arc<Type>> },
    Record(RecordDecl),
    Component(ComponentDecl),
    Union(UnionDecl),
    TypeAlias,
}

#[derive(Debug)]
pub(crate) struct Declaration {
    pub name: Arc<str>,
    pub kind: DeclarationKind,
}

/// One record or union case of a module, by the `$type` a value of it carries.
#[derive(Debug)]
pub(crate) struct Shape {
    pub fields: Arc<[Field]>,
    pub bases: Arc<[Ref]>,
    pub is_abstract: bool,
    /// The declaration that declares the shape: the record, or the case's union.
    pub declaration: u32,
    /// For an update record, the record it patches.
    pub update_target: Option<Ref>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Idiv,
    Mod,
    Imod,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Fadd32,
    Fsub32,
    Fmul32,
    Fdiv32,
}

impl BinaryOp {
    pub(crate) fn name(self) -> &'static str {
        kinds::name(kinds::binary::NAMES, self as i64).unwrap_or("binary")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intrinsic {
    Apply,
    Merge,
    Diff,
    Changed,
}

impl Intrinsic {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Intrinsic::Apply => "apply",
            Intrinsic::Merge => "merge",
            Intrinsic::Diff => "diff",
            Intrinsic::Changed => "changed",
        }
    }
}

/// The primitive types a `text` node can name: the ones with a canonical text form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextType {
    Int,
    Float32,
    Float64,
    Boolean,
}

fn text_type(name: &str) -> Option<TextType> {
    Some(match name {
        "int" | "int32" | "int64" => TextType::Int,
        "float32" => TextType::Float32,
        "float64" => TextType::Float64,
        "boolean" => TextType::Boolean,
        _ => return None,
    })
}

pub(crate) type Properties = Box<[(Arc<str>, u32)]>;

#[derive(Debug)]
pub(crate) struct Arm {
    pub patterns: Box<[u32]>,
    pub body: u32,
}

/// A record or component construction: the target, the written properties and the body content.
#[derive(Debug)]
pub(crate) struct Construct {
    pub target: Ref,
    pub properties: Properties,
    pub content: Box<[u32]>,
}

#[derive(Debug)]
pub(crate) struct CaseConstruct {
    pub union: Ref,
    pub case: Arc<str>,
    pub properties: Properties,
    pub content: Box<[u32]>,
}

#[derive(Debug)]
pub(crate) struct ElementNode {
    pub tag: Arc<str>,
    pub properties: Properties,
    pub content: Box<[u32]>,
}

#[derive(Debug)]
pub(crate) struct HandlerNode {
    pub component: Ref,
    pub emit: Arc<str>,
    pub action: Ref,
    pub action_slot: u32,
    pub owner: Option<Ref>,
    pub body: u32,
}

#[derive(Debug)]
pub(crate) struct ForNode {
    pub is_range: bool,
    pub item_slot: u32,
    pub index_slot: Option<u32>,
    pub iterable: u32,
    pub body: u32,
}

/// One node of the node table, decoded. A child is an index into the same table.
#[derive(Debug)]
pub(crate) enum Node {
    Bool(bool),
    String(Arc<str>),
    Number(Value),
    Slot {
        slot: u32,
        name: Arc<str>,
    },
    Reference(Ref),
    Binary {
        op: BinaryOp,
        lhs: u32,
        rhs: u32,
    },
    Unary {
        op: UnaryOp,
        operand: u32,
    },
    Text {
        operand: u32,
        ty: TextType,
    },
    Call {
        callee: u32,
        args: Box<[Option<u32>]>,
    },
    NamedCall {
        callee: u32,
        args: Properties,
    },
    Intrinsic {
        intrinsic: Intrinsic,
        args: Box<[u32]>,
        names: Box<[Arc<str>]>,
    },
    If {
        condition: u32,
        then: u32,
        otherwise: Option<u32>,
    },
    IfIs {
        scrutinee: u32,
        arms: Box<[Arm]>,
        otherwise: Option<u32>,
    },
    Array(Box<[u32]>),
    For(ForNode),
    Member {
        optional: bool,
        base: u32,
        name: Arc<str>,
    },
    Exists(u32),
    Coalesce(u32, u32),
    Record(Construct),
    Component(Construct),
    UnionCase(Box<CaseConstruct>),
    Element(ElementNode),
    ActionHandler(Box<HandlerNode>),
}

impl Node {
    /// Every node this node names, for a walk over a declaration's nodes.
    pub(crate) fn children(&self, visit: &mut dyn FnMut(u32)) {
        let properties = |properties: &Properties, visit: &mut dyn FnMut(u32)| {
            properties.iter().for_each(|(_, node)| visit(*node));
        };
        match self {
            Node::Bool(_)
            | Node::String(_)
            | Node::Number(_)
            | Node::Slot { .. }
            | Node::Reference(_) => {}
            Node::Binary { lhs, rhs, .. } | Node::Coalesce(lhs, rhs) => {
                visit(*lhs);
                visit(*rhs);
            }
            Node::Unary { operand, .. } | Node::Text { operand, .. } | Node::Exists(operand) => {
                visit(*operand)
            }
            Node::Member { base, .. } => visit(*base),
            Node::Call { callee, args } => {
                visit(*callee);
                args.iter().flatten().for_each(|node| visit(*node));
            }
            Node::NamedCall { callee, args } => {
                visit(*callee);
                properties(args, visit);
            }
            Node::Intrinsic { args, .. } => args.iter().for_each(|node| visit(*node)),
            Node::If {
                condition,
                then,
                otherwise,
            } => {
                visit(*condition);
                visit(*then);
                otherwise.iter().for_each(|node| visit(*node));
            }
            Node::IfIs {
                scrutinee,
                arms,
                otherwise,
            } => {
                visit(*scrutinee);
                for arm in arms.iter() {
                    arm.patterns.iter().for_each(|node| visit(*node));
                    visit(arm.body);
                }
                otherwise.iter().for_each(|node| visit(*node));
            }
            Node::Array(items) => items.iter().for_each(|node| visit(*node)),
            Node::For(node) => {
                visit(node.iterable);
                visit(node.body);
            }
            Node::Record(construct) | Node::Component(construct) => {
                properties(&construct.properties, visit);
                construct.content.iter().for_each(|node| visit(*node));
            }
            Node::UnionCase(construct) => {
                properties(&construct.properties, visit);
                construct.content.iter().for_each(|node| visit(*node));
            }
            Node::Element(element) => {
                properties(&element.properties, visit);
                element.content.iter().for_each(|node| visit(*node));
            }
            Node::ActionHandler(handler) => visit(handler.body),
        }
    }
}

/// One entry of a module's module table.
#[derive(Debug, Clone)]
pub(crate) struct TableEntry {
    pub identity: Arc<str>,
    pub version: Arc<str>,
}

#[derive(Debug)]
pub(crate) struct ModuleData {
    image: NxIrImageBuf,
    pub identity: Arc<str>,
    pub version: Arc<str>,
    pub fingerprint: u64,
    /// A hash of the image's bytes: what ties a stored component instance to the very images
    /// that rendered it, since its handlers name nodes and slots by index.
    pub image_hash: u64,
    /// The module table; slot `0` is this module.
    pub table: Vec<TableEntry>,
    pub declarations: Vec<Declaration>,
    pub by_name: HashMap<Arc<str>, u32>,
    pub function_entrypoints: Vec<u32>,
    pub component_entrypoints: Vec<u32>,
    /// The declaration names this module references in each other module of its table, by slot.
    pub external_references: Vec<BTreeSet<Arc<str>>>,
    pub shapes: HashMap<Arc<str>, Vec<Shape>>,
    strings: Vec<OnceLock<Arc<str>>>,
    nodes: Vec<OnceLock<Option<Box<Node>>>>,
}

/// One image, validated and indexed.
///
/// <para>Cloning shares the module; linking never copies it. A host prepares a large module once
/// and links any number of programs against it, on any number of threads.</para>
#[derive(Debug, Clone)]
pub struct PreparedModule {
    pub(crate) data: Arc<ModuleData>,
}

/// A decoding failure: the image is not what validation promised. A message, since the caller
/// knows which declaration to name.
pub(crate) type Malformed = String;

struct Reader<'a> {
    cells: Cells<'a>,
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(cells: Cells<'a>) -> Self {
        Self { cells, position: 0 }
    }

    fn next(&mut self) -> std::result::Result<u32, Malformed> {
        let cell = self
            .cells
            .get(self.position)
            .ok_or_else(|| "an entry ends before its layout does".to_string())?;
        self.position = self.position.saturating_add(1);
        Ok(cell)
    }

    fn optional(&mut self) -> std::result::Result<Option<u32>, Malformed> {
        Ok(Some(self.next()?).filter(|cell| *cell != NONE))
    }

    fn list<T>(
        &mut self,
        mut element: impl FnMut(&mut Self) -> std::result::Result<T, Malformed>,
    ) -> std::result::Result<Vec<T>, Malformed> {
        let count = self.next()? as usize;
        // An element takes at least one cell, so a count the entry cannot hold is refused before
        // anything is allocated for it.
        if count > self.cells.len().saturating_sub(self.position) {
            return Err("a list is longer than its entry".to_string());
        }
        let mut output = Vec::with_capacity(count);
        for _ in 0..count {
            output.push(element(self)?);
        }
        Ok(output)
    }
}

struct Decoder<'a> {
    image: NxIrImage<'a>,
    strings: &'a [OnceLock<Arc<str>>],
}

impl<'a> Decoder<'a> {
    fn string(&self, index: u32) -> std::result::Result<Arc<str>, Malformed> {
        let slot = self
            .strings
            .get(index as usize)
            .ok_or_else(|| format!("string index {index} is out of range"))?;
        if let Some(text) = slot.get() {
            return Ok(Arc::clone(text));
        }
        let text = self
            .image
            .string(index)
            .ok_or_else(|| format!("string index {index} is out of range"))?;
        Ok(Arc::clone(slot.get_or_init(|| Arc::from(text))))
    }

    fn reference(&self, reader: &mut Reader<'_>) -> std::result::Result<Ref, Malformed> {
        let slot = reader.next()?;
        Ok(Ref {
            slot,
            name: self.string(reader.next()?)?,
        })
    }

    fn optional_reference(
        &self,
        reader: &mut Reader<'_>,
    ) -> std::result::Result<Option<Ref>, Malformed> {
        let slot = reader.next()?;
        let name = reader.next()?;
        if slot == NONE {
            return Ok(None);
        }
        Ok(Some(Ref {
            slot,
            name: self.string(name)?,
        }))
    }

    fn properties(&self, reader: &mut Reader<'_>) -> std::result::Result<Properties, Malformed> {
        Ok(reader
            .list(|reader| Ok((self.string(reader.next()?)?, reader.next()?)))?
            .into_boxed_slice())
    }

    fn nodes(&self, reader: &mut Reader<'_>) -> std::result::Result<Box<[u32]>, Malformed> {
        Ok(reader.list(Reader::next)?.into_boxed_slice())
    }

    fn constant(&self, index: u32) -> std::result::Result<Value, Malformed> {
        let entry = self
            .image
            .entry(Table::Constants, index)
            .ok_or_else(|| format!("constant index {index} is out of range"))?;
        let mut reader = Reader::new(entry);
        let kind = i64::from(reader.next()?);
        Ok(match kind {
            kinds::constant::INT => {
                let low = u64::from(reader.next()?);
                let high = u64::from(reader.next()?);
                Value::Int((low | (high << 32)) as i64)
            }
            kinds::constant::FLOAT => {
                let low = u64::from(reader.next()?);
                let high = u64::from(reader.next()?);
                Value::Float(f64::from_bits(low | (high << 32)))
            }
            kinds::constant::BIGINT => {
                let digits = self.string(reader.next()?)?;
                Value::Int(
                    digits.parse::<i64>().map_err(|_| {
                        format!("the integer constant '{digits}' does not fit 64 bits")
                    })?,
                )
            }
            other => return Err(format!("unknown constant kind {other}")),
        })
    }

    fn node(&self, index: u32) -> std::result::Result<Node, Malformed> {
        use kinds::node;
        let entry = self
            .image
            .entry(Table::Nodes, index)
            .ok_or_else(|| format!("node index {index} is out of range"))?;
        let mut reader = Reader::new(entry);
        let kind = i64::from(reader.next()?);
        Ok(match kind {
            node::BOOL => Node::Bool(reader.next()? != 0),
            node::STRING => Node::String(self.string(reader.next()?)?),
            node::NUMBER => Node::Number(self.constant(reader.next()?)?),
            node::SLOT => Node::Slot {
                slot: reader.next()?,
                name: self.string(reader.next()?)?,
            },
            node::REFERENCE => Node::Reference(self.reference(&mut reader)?),
            node::BINARY => {
                let code = reader.next()?;
                Node::Binary {
                    op: binary_op(code).ok_or_else(|| format!("unknown binary operator {code}"))?,
                    lhs: reader.next()?,
                    rhs: reader.next()?,
                }
            }
            node::UNARY => {
                let code = i64::from(reader.next()?);
                let op = match code {
                    kinds::unary::NEG => UnaryOp::Neg,
                    kinds::unary::NOT => UnaryOp::Not,
                    other => return Err(format!("unknown unary operator {other}")),
                };
                Node::Unary {
                    op,
                    operand: reader.next()?,
                }
            }
            node::TEXT => {
                let operand = reader.next()?;
                let name = self.string(reader.next()?)?;
                Node::Text {
                    operand,
                    ty: text_type(&name).ok_or_else(|| {
                        format!("'{name}' is not a primitive type with a text form")
                    })?,
                }
            }
            node::CALL => Node::Call {
                callee: reader.next()?,
                args: reader.list(Reader::optional)?.into_boxed_slice(),
            },
            node::NAMED_CALL => Node::NamedCall {
                callee: reader.next()?,
                args: self.properties(&mut reader)?,
            },
            node::INTRINSIC => {
                let code = reader.next()?;
                let intrinsic = match code {
                    0 => Intrinsic::Apply,
                    1 => Intrinsic::Merge,
                    2 => Intrinsic::Diff,
                    3 => Intrinsic::Changed,
                    other => return Err(format!("unknown intrinsic {other}")),
                };
                Node::Intrinsic {
                    intrinsic,
                    args: self.nodes(&mut reader)?,
                    names: reader
                        .list(|reader| self.string(reader.next()?))?
                        .into_boxed_slice(),
                }
            }
            node::IF => Node::If {
                condition: reader.next()?,
                then: reader.next()?,
                otherwise: reader.optional()?,
            },
            node::IF_IS => Node::IfIs {
                scrutinee: reader.next()?,
                arms: reader
                    .list(|reader| {
                        Ok(Arm {
                            patterns: reader.list(Reader::next)?.into_boxed_slice(),
                            body: reader.next()?,
                        })
                    })?
                    .into_boxed_slice(),
                otherwise: reader.optional()?,
            },
            node::ARRAY => Node::Array(self.nodes(&mut reader)?),
            node::FOR | node::FOR_RANGE => {
                let item_slot = reader.next()?;
                let _item_name = reader.next()?;
                let index_slot = reader.optional()?;
                let _index_name = reader.next()?;
                Node::For(ForNode {
                    is_range: kind == node::FOR_RANGE,
                    item_slot,
                    index_slot,
                    iterable: reader.next()?,
                    body: reader.next()?,
                })
            }
            node::MEMBER | node::OPTIONAL_MEMBER => Node::Member {
                optional: kind == node::OPTIONAL_MEMBER,
                base: reader.next()?,
                name: self.string(reader.next()?)?,
            },
            node::EXISTS => Node::Exists(reader.next()?),
            node::COALESCE => Node::Coalesce(reader.next()?, reader.next()?),
            node::RECORD | node::COMPONENT => {
                let construct = Construct {
                    target: self.reference(&mut reader)?,
                    properties: self.properties(&mut reader)?,
                    content: self.nodes(&mut reader)?,
                };
                if kind == node::RECORD {
                    Node::Record(construct)
                } else {
                    Node::Component(construct)
                }
            }
            node::UNION_CASE => Node::UnionCase(Box::new(CaseConstruct {
                union: self.reference(&mut reader)?,
                case: self.string(reader.next()?)?,
                properties: self.properties(&mut reader)?,
                content: self.nodes(&mut reader)?,
            })),
            node::ELEMENT => {
                let _id = reader.next()?;
                Node::Element(ElementNode {
                    tag: self.string(reader.next()?)?,
                    properties: self.properties(&mut reader)?,
                    content: self.nodes(&mut reader)?,
                })
            }
            node::ACTION_HANDLER => Node::ActionHandler(Box::new(HandlerNode {
                component: self.reference(&mut reader)?,
                emit: self.string(reader.next()?)?,
                action: self.reference(&mut reader)?,
                action_slot: reader.next()?,
                owner: self.optional_reference(&mut reader)?,
                body: reader.next()?,
            })),
            other => return Err(format!("unknown node kind {other}")),
        })
    }

    /// Decodes the type table in order. An entry names only earlier entries, so one pass
    /// resolves every type without recursion.
    fn types(&self) -> std::result::Result<Vec<Arc<Type>>, Malformed> {
        let count = self.image.entry_count(Table::Types);
        let mut types: Vec<(Arc<Type>, u32)> = Vec::with_capacity(count);
        for index in 0..count {
            let entry = self
                .image
                .entry(Table::Types, index as u32)
                .ok_or_else(|| format!("type index {index} is out of range"))?;
            let mut reader = Reader::new(entry);
            let earlier = |cell: u32| -> std::result::Result<(Arc<Type>, u32), Malformed> {
                types
                    .get(cell as usize)
                    .map(|(ty, depth)| (Arc::clone(ty), *depth))
                    .ok_or_else(|| {
                        format!("type {index} names type {cell}, which does not precede it")
                    })
            };
            let kind = i64::from(reader.next()?);
            let (ty, depth) = match kind {
                kinds::ty::PRIMITIVE => {
                    let name = self.string(reader.next()?)?;
                    let ty = match &*name {
                        "int" => Type::Primitive(Primitive::Int),
                        "int32" => Type::Primitive(Primitive::Int32),
                        "int64" => Type::Primitive(Primitive::Int64),
                        "float32" => Type::Primitive(Primitive::Float32),
                        "float64" => Type::Primitive(Primitive::Float64),
                        "string" => Type::Primitive(Primitive::String),
                        "boolean" => Type::Primitive(Primitive::Boolean),
                        "object" => Type::Primitive(Primitive::Object),
                        _ => Type::UnknownPrimitive(name),
                    };
                    (ty, 0)
                }
                kinds::ty::NOMINAL => (Type::Nominal(self.reference(&mut reader)?), 0),
                kinds::ty::SEQ => {
                    let (item, depth) = earlier(reader.next()?)?;
                    let occurrence = i64::from(reader.next()?);
                    (
                        Type::Seq {
                            item,
                            may_be_empty: occurrence & kinds::ty::OCCURRENCE_EMPTY != 0,
                            may_be_many: occurrence & kinds::ty::OCCURRENCE_MANY != 0,
                        },
                        depth.saturating_add(1),
                    )
                }
                // The checker related a function value to its type by name, so the runtime keeps
                // nothing of a function type's parameters or result.
                kinds::ty::FUNCTION => (Type::Function, 0),
                other => return Err(format!("unknown type kind {other} at type {index}")),
            };
            if depth > MAX_TYPE_DEPTH {
                return Err(format!(
                    "type {index} nests more than {MAX_TYPE_DEPTH} types"
                ));
            }
            types.push((Arc::new(ty), depth));
        }
        Ok(types.into_iter().map(|(ty, _)| ty).collect())
    }

    fn fields(
        &self,
        reader: &mut Reader<'_>,
        types: &[Arc<Type>],
    ) -> std::result::Result<Arc<[Field]>, Malformed> {
        Ok(Arc::from(reader.list(|reader| {
            let name = self.string(reader.next()?)?;
            let ty = type_at(types, reader.next()?)?;
            let default = reader.optional()?;
            let flags = reader.next()?;
            Ok(Field {
                name,
                ty,
                default,
                is_content: flags & 1 != 0,
                is_required: flags & 2 != 0,
            })
        })?))
    }

    fn references(&self, reader: &mut Reader<'_>) -> std::result::Result<Arc<[Ref]>, Malformed> {
        Ok(Arc::from(reader.list(|reader| self.reference(reader))?))
    }

    fn declaration(
        &self,
        index: u32,
        types: &[Arc<Type>],
    ) -> std::result::Result<Declaration, Malformed> {
        use kinds::declaration;
        let entry = self
            .image
            .entry(Table::Declarations, index)
            .ok_or_else(|| format!("declaration index {index} is out of range"))?;
        let mut reader = Reader::new(entry);
        let kind = i64::from(reader.next()?);
        let name = self.string(reader.next()?)?;
        let kind = match kind {
            declaration::FUNCTION => {
                let params = reader.list(|reader| {
                    let name = self.string(reader.next()?)?;
                    let ty = type_at(types, reader.next()?)?;
                    let default = reader.optional()?;
                    let flags = i64::from(reader.next()?);
                    Ok(Param {
                        name,
                        ty,
                        default,
                        is_content: flags & kinds::ty::FUNCTION_PARAM_CONTENT != 0,
                        is_optional: flags & kinds::ty::FUNCTION_PARAM_OPTIONAL != 0,
                    })
                })?;
                let body = reader.next()?;
                let result = reader
                    .optional()?
                    .map(|cell| type_at(types, cell))
                    .transpose()?;
                let flags = i64::from(reader.next()?);
                DeclarationKind::Function(FunctionDecl {
                    params,
                    body,
                    result,
                    is_optional_result: flags & declaration::RESULT_OPTIONAL != 0,
                })
            }
            declaration::VALUE => DeclarationKind::Value {
                value: reader.next()?,
                ty: reader
                    .optional()?
                    .map(|cell| type_at(types, cell))
                    .transpose()?,
            },
            declaration::RECORD => DeclarationKind::Record(RecordDecl {
                fields: self.fields(&mut reader, types)?,
                bases: self.references(&mut reader)?,
                is_abstract: reader.next()? != 0,
                update_target: self.optional_reference(&mut reader)?,
            }),
            declaration::COMPONENT => {
                let props = self.fields(&mut reader, types)?;
                let state = self.fields(&mut reader, types)?;
                let body = reader.optional()?;
                let flags = reader.next()?;
                let emits = reader.list(|reader| {
                    Ok(Emit {
                        name: self.string(reader.next()?)?,
                        action: self.reference(reader)?,
                    })
                })?;
                DeclarationKind::Component(ComponentDecl {
                    props,
                    state,
                    body,
                    is_abstract: flags & 1 != 0,
                    emits,
                })
            }
            declaration::UNION => {
                let cases = reader.list(|reader| {
                    Ok(UnionCase {
                        name: self.string(reader.next()?)?,
                        fields: self.fields(reader, types)?,
                        is_constant: reader.next()? != 0,
                    })
                })?;
                let bases = self.references(&mut reader)?;
                let _property_target = self.optional_reference(&mut reader)?;
                DeclarationKind::Union(UnionDecl { cases, bases })
            }
            _ => DeclarationKind::TypeAlias,
        };
        Ok(Declaration { name, kind })
    }
}

fn type_at(types: &[Arc<Type>], cell: u32) -> std::result::Result<Arc<Type>, Malformed> {
    types
        .get(cell as usize)
        .cloned()
        .ok_or_else(|| format!("type index {cell} is out of range"))
}

fn binary_op(code: u32) -> Option<BinaryOp> {
    use kinds::binary;
    Some(match i64::from(code) {
        binary::ADD => BinaryOp::Add,
        binary::SUB => BinaryOp::Sub,
        binary::MUL => BinaryOp::Mul,
        binary::DIV => BinaryOp::Div,
        binary::IDIV => BinaryOp::Idiv,
        binary::MOD => BinaryOp::Mod,
        binary::IMOD => BinaryOp::Imod,
        binary::CONCAT => BinaryOp::Concat,
        binary::EQ => BinaryOp::Eq,
        binary::NE => BinaryOp::Ne,
        binary::LT => BinaryOp::Lt,
        binary::LE => BinaryOp::Le,
        binary::GT => BinaryOp::Gt,
        binary::GE => BinaryOp::Ge,
        binary::AND => BinaryOp::And,
        binary::OR => BinaryOp::Or,
        binary::FADD32 => BinaryOp::Fadd32,
        binary::FSUB32 => BinaryOp::Fsub32,
        binary::FMUL32 => BinaryOp::Fmul32,
        binary::FDIV32 => BinaryOp::Fdiv32,
        _ => return None,
    })
}

fn malformed(message: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        "nx-ir-malformed",
        format!("NX IR image is malformed: {message}."),
    )
}

impl PreparedModule {
    /// Validates an image and indexes its module.
    ///
    /// <para>Everything `docs/nx-ir-format.md` requires a reader to check is checked here, before
    /// anything else is read: the header, every section, offset and index, the runtime ABI and
    /// the required features. The bytes are kept and read in place.</para>
    pub fn prepare(bytes: impl Into<Arc<[u8]>>) -> Result<Self> {
        let buffer = NxIrImageBuf::open(bytes).map_err(|error| NxIrRuntimeError {
            diagnostics: vec![match error {
                NxIrImageError::NotAnImage => Diagnostic::new("nx-ir-format", "The input is not an NX IR image."),
                NxIrImageError::SchemaVersion { found, .. } => Diagnostic::new(
                    "nx-ir-schema-version",
                    format!(
                        "NX IR schema version {found} is not supported; this runtime reads schema version {NX_IR_SCHEMA_VERSION}."
                    ),
                ),
                NxIrImageError::Malformed(message) => malformed(message),
            }],
        })?;
        let image = buffer.image();
        let mut diagnostics = Vec::new();

        if image.runtime_abi() != NX_IR_RUNTIME_ABI {
            diagnostics.push(Diagnostic::new(
                "nx-ir-runtime-abi",
                format!(
                    "NX IR runtime ABI '{}' is not supported; this runtime implements '{NX_IR_RUNTIME_ABI}'.",
                    image.runtime_abi()
                ),
            ));
        }
        for feature in image.required_features() {
            if !KNOWN_FEATURES.contains(&feature) {
                diagnostics.push(Diagnostic::new(
                    "nx-ir-required-feature",
                    format!("Unsupported NX IR required feature '{feature}'."),
                ));
            }
        }
        if !diagnostics.is_empty() {
            return Err(NxIrRuntimeError { diagnostics });
        }

        check_nodes(&image, &mut diagnostics);

        let strings: Vec<OnceLock<Arc<str>>> =
            (0..image.string_count()).map(|_| OnceLock::new()).collect();
        let decoder = Decoder {
            image,
            strings: &strings,
        };
        let failed = |message: Malformed| NxIrRuntimeError {
            diagnostics: vec![malformed(message)],
        };
        let table: Vec<TableEntry> = image
            .modules()
            .map(|module| TableEntry {
                identity: Arc::from(module.identity),
                version: Arc::from(module.version),
            })
            .collect();
        let (identity, version) = match table.first() {
            Some(own) => (Arc::clone(&own.identity), Arc::clone(&own.version)),
            None => return Err(failed("the module table is empty".to_string())),
        };
        let fingerprint = image
            .modules()
            .next()
            .map(|own| own.fingerprint)
            .unwrap_or(0);

        let types = decoder.types().map_err(failed)?;
        let mut declarations = Vec::with_capacity(image.entry_count(Table::Declarations));
        let mut by_name: HashMap<Arc<str>, u32> = HashMap::new();
        for index in 0..image.entry_count(Table::Declarations) as u32 {
            let declaration = decoder.declaration(index, &types).map_err(failed)?;
            if by_name
                .insert(Arc::clone(&declaration.name), index)
                .is_some()
            {
                diagnostics.push(Diagnostic::new(
                    "nx-ir-duplicate-declaration",
                    format!("Module '{identity}' declares '{}' twice.", declaration.name),
                ));
            }
            declarations.push(declaration);
        }

        let mut external_references: Vec<BTreeSet<Arc<str>>> =
            table.iter().map(|_| BTreeSet::new()).collect();
        let mut reference_error = None;
        image.for_each_reference(|slot, name| {
            match (
                decoder.string(name),
                external_references.get_mut(slot as usize),
            ) {
                (Ok(name), Some(names)) => {
                    names.insert(name);
                }
                (Err(message), _) => reference_error = Some(message),
                (_, None) => reference_error = Some(format!("module slot {slot} is out of range")),
            }
        });
        if let Some(message) = reference_error {
            return Err(failed(message));
        }
        if let Some(local) = external_references.first_mut() {
            for name in std::mem::take(local) {
                if !by_name.contains_key(&name) {
                    diagnostics.push(Diagnostic::new(
                        "nx-ir-reference",
                        format!("Module '{identity}' references its own declaration '{name}', which it does not declare."),
                    ));
                }
            }
        }

        let mut entrypoints =
            |indices: Cells<'_>, what: &str, wanted: fn(&DeclarationKind) -> bool| -> Vec<u32> {
                let mut output = Vec::with_capacity(indices.len());
                for index in indices.iter() {
                    match declarations.get(index as usize) {
                        Some(declaration) if wanted(&declaration.kind) => output.push(index),
                        _ => diagnostics.push(Diagnostic::new(
                            "nx-ir-entrypoint",
                            format!("{what} entrypoint {index} is invalid."),
                        )),
                    }
                }
                output
            };
        let function_entrypoints = entrypoints(image.function_entrypoints(), "function", |kind| {
            matches!(kind, DeclarationKind::Function(_))
        });
        let component_entrypoints =
            entrypoints(image.component_entrypoints(), "component", |kind| {
                matches!(kind, DeclarationKind::Component(_))
            });

        if !diagnostics.is_empty() {
            return Err(NxIrRuntimeError { diagnostics });
        }

        let shapes = index_shapes(&declarations);
        let image_hash = fnv1a(buffer.bytes());
        let nodes = (0..image.entry_count(Table::Nodes))
            .map(|_| OnceLock::new())
            .collect();
        Ok(Self {
            data: Arc::new(ModuleData {
                image: buffer,
                identity,
                version,
                fingerprint,
                image_hash,
                table,
                declarations,
                by_name,
                function_entrypoints,
                component_entrypoints,
                external_references,
                shapes,
                strings,
                nodes,
            }),
        })
    }

    /// The module's identity: the name other modules reference it by.
    pub fn identity(&self) -> &str {
        &self.data.identity
    }

    /// The version the module was built with, which linking compares.
    pub fn version(&self) -> &str {
        &self.data.version
    }

    /// A hash of the module's identity and source text.
    pub fn fingerprint(&self) -> u64 {
        self.data.fingerprint
    }

    /// The validated image the module is read from.
    pub fn image(&self) -> NxIrImage<'_> {
        self.data.image.image()
    }

    /// The identities of the modules this module's table names, itself excepted.
    pub fn referenced_modules(&self) -> impl Iterator<Item = &str> {
        self.data.table.iter().skip(1).map(|entry| &*entry.identity)
    }

    /// The public names of the module's function entrypoints, in declaration order.
    pub fn function_entrypoints(&self) -> impl Iterator<Item = &str> {
        self.data.entrypoint_names(&self.data.function_entrypoints)
    }

    /// The public names of the module's component entrypoints, in declaration order.
    pub fn component_entrypoints(&self) -> impl Iterator<Item = &str> {
        self.data.entrypoint_names(&self.data.component_entrypoints)
    }
}

impl ModuleData {
    fn entrypoint_names<'a>(&'a self, indices: &'a [u32]) -> impl Iterator<Item = &'a str> {
        indices
            .iter()
            .filter_map(|index| self.declarations.get(*index as usize))
            .map(|declaration| &*declaration.name)
    }

    pub(crate) fn image(&self) -> NxIrImage<'_> {
        self.image.image()
    }

    pub(crate) fn declaration(&self, index: u32) -> Option<&Declaration> {
        self.declarations.get(index as usize)
    }

    pub(crate) fn find(&self, name: &str) -> Option<(u32, &Declaration)> {
        let index = *self.by_name.get(name)?;
        Some((index, self.declarations.get(index as usize)?))
    }

    /// The entrypoint of one of the two lists that `name` names.
    pub(crate) fn entrypoint(&self, indices: &[u32], name: &str) -> Option<(u32, &Declaration)> {
        let (index, declaration) = self.find(name)?;
        indices.contains(&index).then_some((index, declaration))
    }

    pub(crate) fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Node `index`, decoded the first time it is asked for.
    pub(crate) fn node(&self, index: u32) -> std::result::Result<&Node, Malformed> {
        let slot = self
            .nodes
            .get(index as usize)
            .ok_or_else(|| format!("node index {index} is out of range"))?;
        if let Some(node) = slot.get() {
            return node
                .as_deref()
                .ok_or_else(|| format!("node {index} is malformed"));
        }
        let decoder = Decoder {
            image: self.image.image(),
            strings: &self.strings,
        };
        let decoded = decoder.node(index);
        let message = decoded.as_ref().err().cloned();
        let stored = slot.get_or_init(|| decoded.ok().map(Box::new));
        match (stored.as_deref(), message) {
            (Some(node), _) => Ok(node),
            (None, Some(message)) => Err(format!("node {index}: {message}")),
            (None, None) => Err(format!("node {index} is malformed")),
        }
    }
}

/// What the reader's layouts do not say about nodes: a `text` node names a type with a text
/// form, and a presence operator or a `{}` pattern appears only in a module that lists
/// `occurrence-v1`.
fn check_nodes(image: &NxIrImage<'_>, diagnostics: &mut Vec<Diagnostic>) {
    use kinds::node;
    let lists_occurrence = image
        .required_features()
        .any(|feature| feature == NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1);
    let missing = |what: &str| {
        malformed(format!(
            "{what}, which requires the feature '{NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1}' the module does not list"
        ))
    };
    for index in 0..image.entry_count(Table::Nodes) as u32 {
        let Some(entry) = image.entry(Table::Nodes, index) else {
            continue;
        };
        let kind = entry.get(0).map(i64::from).unwrap_or(node::NULL);
        if kind == node::TEXT {
            let name = entry
                .get(2)
                .and_then(|cell| image.string(cell))
                .unwrap_or("");
            if text_type(name).is_none() {
                diagnostics.push(malformed(format!(
                    "node {index} names '{name}', which is not a primitive type with a text form"
                )));
            }
            continue;
        }
        if lists_occurrence {
            continue;
        }
        let operator = match kind {
            node::EXISTS => Some("exists"),
            node::OPTIONAL_MEMBER => Some("optionalMember"),
            node::COALESCE => Some("coalesce"),
            _ => None,
        };
        if let Some(operator) = operator {
            diagnostics.push(missing(&format!("node {index} is an '{operator}' node")));
            continue;
        }
        if kind != node::IF_IS {
            continue;
        }
        // `[11, node, [[[node...], node]...], node?]`: each arm's patterns, looking for an empty
        // `array` node, which is the `{}` pattern.
        let mut reader = Reader::new(entry);
        let patterns = (|| -> std::result::Result<Vec<u32>, Malformed> {
            reader.next()?;
            reader.next()?;
            let arms = reader.list(|reader| {
                let patterns = reader.list(Reader::next)?;
                reader.next()?;
                Ok(patterns)
            })?;
            Ok(arms.into_iter().flatten().collect())
        })()
        .unwrap_or_default();
        let is_empty_pattern = |pattern: &u32| {
            image.entry(Table::Nodes, *pattern).is_some_and(|node| {
                node.len() == 2
                    && node.get(0).map(i64::from) == Some(node::ARRAY)
                    && node.get(1) == Some(0)
            })
        };
        if patterns.iter().any(is_empty_pattern) {
            diagnostics.push(missing(&format!("node {index} matches the '{{}}' pattern")));
        }
    }
}

/// Indexes every nominal shape a module declares by the `$type` its values carry.
///
/// <para>An abstract record is indexed too, even though nothing may be an instance of one: a
/// value that names one is a value to reject, and rejecting it by name reads better than
/// reporting a type the program does not have. A union contributes one entry per non-constant
/// case, under `Union.case`; a constant case is a bare name with no schema to normalize.</para>
/// FNV-1a over `bytes`, the hash the format already uses for a module's fingerprint.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn index_shapes(declarations: &[Declaration]) -> HashMap<Arc<str>, Vec<Shape>> {
    let mut shapes: HashMap<Arc<str>, Vec<Shape>> = HashMap::new();
    for (index, declaration) in declarations.iter().enumerate() {
        let index = index as u32;
        match &declaration.kind {
            DeclarationKind::Record(record) => {
                shapes
                    .entry(Arc::clone(&declaration.name))
                    .or_default()
                    .push(Shape {
                        fields: Arc::clone(&record.fields),
                        bases: Arc::clone(&record.bases),
                        is_abstract: record.is_abstract,
                        declaration: index,
                        update_target: record.update_target.clone(),
                    });
            }
            DeclarationKind::Union(union) => {
                for case in union.cases.iter().filter(|case| !case.is_constant) {
                    shapes
                        .entry(Arc::from(format!("{}.{}", declaration.name, case.name)))
                        .or_default()
                        .push(Shape {
                            fields: Arc::clone(&case.fields),
                            bases: Arc::clone(&union.bases),
                            is_abstract: false,
                            declaration: index,
                            update_target: None,
                        });
                }
            }
            _ => {}
        }
    }
    shapes
}
