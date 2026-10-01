//! The NX IR image: schema 5 as a little-endian binary a runtime reads in place.
//!
//! <para>An image is a 16-byte header, a directory of sections, and the sections themselves, each
//! starting on a four-byte boundary. The string table is an offset array over one UTF-8 blob; every
//! other table is an offset array over a pool of 32-bit cells, so entry `k` of any table is a slice
//! the reader takes without decoding the entries before it. `docs/nx-ir-format.md` documents the
//! layout; this file is its one implementation in Rust, shared by the writer, the validating reader
//! and the decoder that rebuilds an [`NxIrArtifact`] from an image.</para>
//!
//! <para>[`NxIrImage::open`] validates everything before it answers a question: the header, the
//! directory, every offset array, the string blob's encoding, and every entry of every table by its
//! kind's layout. After it returns `Ok`, every index the image contains is in range, so a reader
//! can follow them without further checks.</para>

use crate::model::{
    kinds, IrItem, NxIrArtifact, NxIrDebug, NxIrDebugSpans, NxIrModuleEntry, NX_IR_SCHEMA_VERSION,
};
use std::ops::Range;
use std::sync::Arc;

/// The first four bytes of every image.
pub const NX_IR_MAGIC: [u8; 4] = *b"NXIR";
/// The header: magic, schema version, total length, directory entry count.
const HEADER_LEN: usize = 16;
/// A directory entry: kind, offset, length.
const DIRECTORY_ENTRY_LEN: usize = 12;
/// The cell value that spells an absent optional operand.
pub const NONE: u32 = u32::MAX;

/// Section kinds, as listed in the directory. A kind, once assigned, is never reused.
pub mod section {
    pub const STRINGS: u32 = 0;
    pub const MODULE: u32 = 1;
    pub const TYPES: u32 = 2;
    pub const CONSTANTS: u32 = 3;
    pub const NODES: u32 = 4;
    pub const DECLARATIONS: u32 = 5;
    pub const DEBUG: u32 = 6;
}

/// Why bytes could not be opened as an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NxIrImageError {
    /// The bytes do not begin with the NX IR magic.
    NotAnImage,
    /// The image is of a schema version this crate does not read.
    SchemaVersion { found: u32, supported: u32 },
    /// The image is truncated, or a section, offset, index or string in it is out of range.
    Malformed(String),
}

impl std::fmt::Display for NxIrImageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAnImage => write!(formatter, "the input is not an NX IR image"),
            Self::SchemaVersion { found, supported } => write!(
                formatter,
                "NX IR schema version {found} is not supported; this build reads schema version {supported}"
            ),
            Self::Malformed(message) => write!(formatter, "NX IR image is malformed: {message}"),
        }
    }
}

impl std::error::Error for NxIrImageError {}

fn malformed(message: impl Into<String>) -> NxIrImageError {
    NxIrImageError::Malformed(message.into())
}

/// The message of a malformed-image error, for nesting under the entry it was found in.
fn message(error: NxIrImageError) -> String {
    match error {
        NxIrImageError::Malformed(message) => message,
        other => other.to_string(),
    }
}

// ------------------------------------------------------------------------------------------------
// Layouts
// ------------------------------------------------------------------------------------------------

/// One operand of a table entry, and how it is written as cells.
///
/// These are the rules of the format: every layout in `docs/nx-ir-format.md` is a sequence of
/// these, and the writer, the validator and the decoder walk the same sequence.
#[derive(Debug, Clone, Copy)]
enum Op {
    /// One cell holding a flag, a flags word, an element id or a slot number.
    Int,
    /// One cell holding a code the named table assigns.
    Code(&'static [(i64, &'static str)]),
    /// One cell indexing the string table.
    Str,
    /// One cell indexing the type table.
    Type,
    /// One cell indexing the type table, or `NONE` (`-1` in the model).
    OptType,
    /// One cell indexing the constant table.
    Const,
    /// One cell indexing the node table.
    Node,
    /// One cell indexing the node table, or `NONE` (`-1` in the model).
    OptNode,
    /// One cell holding a slot number, or `NONE` (`-1` in the model).
    OptSlot,
    /// One cell indexing the string table, or `NONE` (`-1` in the model).
    OptStr,
    /// A reference written flat in the model: two operands, module slot and name.
    Ref,
    /// A reference written as a pair `[slot, name]` in the model.
    RefPair,
    /// An optional reference: `[]` in the model when absent, `NONE` in the slot cell here.
    OptRef,
    /// A count cell, then each element by the shape: an element of a one-op shape is the item
    /// itself, otherwise a list of items walked with the shape.
    List(&'static [Op]),
    /// A 64-bit integer as two cells, low word first.
    I64,
    /// A 64-bit float's bits as two cells, low word first.
    F64,
}

const NODES: &[Op] = &[Op::Node];
const OPT_NODES: &[Op] = &[Op::OptNode];
const STRS: &[Op] = &[Op::Str];
const REFS: &[Op] = &[Op::RefPair];
const PROPERTY: &[Op] = &[Op::Str, Op::Node];
const FIELD: &[Op] = &[Op::Str, Op::Type, Op::OptNode, Op::Int];
const PARAM: &[Op] = &[Op::Str, Op::Type, Op::Int];
const DECLARED_PARAM: &[Op] = &[Op::Str, Op::Type, Op::OptNode, Op::Int];
const ARM: &[Op] = &[Op::List(NODES), Op::Node];
const UNION_CASE: &[Op] = &[Op::Str, Op::List(FIELD), Op::Int];
const EMIT: &[Op] = &[Op::Str, Op::RefPair];

/// The tables whose entries are cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Types,
    Constants,
    Nodes,
    Declarations,
}

impl Table {
    const ALL: [Table; 4] = [
        Table::Types,
        Table::Constants,
        Table::Nodes,
        Table::Declarations,
    ];

    fn name(self) -> &'static str {
        match self {
            Table::Types => "type",
            Table::Constants => "constant",
            Table::Nodes => "node",
            Table::Declarations => "declaration",
        }
    }

    fn section(self) -> u32 {
        match self {
            Table::Types => section::TYPES,
            Table::Constants => section::CONSTANTS,
            Table::Nodes => section::NODES,
            Table::Declarations => section::DECLARATIONS,
        }
    }

    /// The operands that follow the kind cell of an entry of this kind, or `None` for a kind the
    /// schema does not assign.
    fn layout(self, kind: i64) -> Option<&'static [Op]> {
        use kinds::{constant, declaration, node, ty};
        Some(match self {
            Table::Types => match kind {
                ty::PRIMITIVE => &[Op::Str],
                ty::NOMINAL => &[Op::Ref],
                // `array` (2) and `nullable` (3) were retired with schema 5 and are not laid out,
                // so an image carrying one is refused as malformed.
                ty::SEQ => &[Op::Type, Op::Int],
                ty::FUNCTION => &[Op::Type, Op::List(PARAM)],
                _ => return None,
            },
            Table::Constants => match kind {
                constant::INT => &[Op::I64],
                constant::BIGINT => &[Op::Str],
                constant::FLOAT => &[Op::F64],
                _ => return None,
            },
            Table::Nodes => match kind {
                // `null` (0) was retired with schema 5 and is not laid out.
                node::BOOL => &[Op::Int],
                node::STRING => &[Op::Str],
                node::NUMBER => &[Op::Const],
                node::SLOT => &[Op::Int, Op::Str],
                node::REFERENCE => &[Op::Ref],
                node::BINARY => &[Op::Code(kinds::binary::NAMES), Op::Node, Op::Node],
                node::UNARY => &[Op::Code(kinds::unary::NAMES), Op::Node],
                node::TEXT => &[Op::Node, Op::Str],
                node::CALL => &[Op::Node, Op::List(OPT_NODES)],
                node::NAMED_CALL => &[Op::Node, Op::List(PROPERTY)],
                node::INTRINSIC => &[
                    Op::Code(kinds::intrinsic::NAMES),
                    Op::List(NODES),
                    Op::List(STRS),
                ],
                node::IF => &[Op::Node, Op::Node, Op::OptNode],
                node::IF_IS => &[Op::Node, Op::List(ARM), Op::OptNode],
                node::ARRAY => &[Op::List(NODES)],
                // A range loop has a `for`'s layout; only the kind, and so what a runtime does
                // with the iterable, differs.
                node::FOR | node::FOR_RANGE => &[
                    Op::Int,
                    Op::Str,
                    Op::OptSlot,
                    Op::OptStr,
                    Op::Node,
                    Op::Node,
                ],
                node::MEMBER | node::OPTIONAL_MEMBER => &[Op::Node, Op::Str],
                node::EXISTS => &[Op::Node],
                node::COALESCE => &[Op::Node, Op::Node],
                node::RECORD | node::COMPONENT => &[Op::Ref, Op::List(PROPERTY), Op::List(NODES)],
                node::UNION_CASE => &[Op::Ref, Op::Str, Op::List(PROPERTY), Op::List(NODES)],
                node::ELEMENT => &[Op::Int, Op::Str, Op::List(PROPERTY), Op::List(NODES)],
                node::ACTION_HANDLER => &[Op::Ref, Op::Str, Op::Ref, Op::Int, Op::OptRef, Op::Node],
                _ => return None,
            },
            Table::Declarations => match kind {
                declaration::FUNCTION => &[
                    Op::Str,
                    Op::List(DECLARED_PARAM),
                    Op::Node,
                    Op::OptType,
                    Op::Int,
                ],
                declaration::VALUE => &[Op::Str, Op::Node, Op::OptType],
                declaration::RECORD => &[
                    Op::Str,
                    Op::List(FIELD),
                    Op::List(REFS),
                    Op::Int,
                    Op::OptRef,
                ],
                declaration::COMPONENT => &[
                    Op::Str,
                    Op::List(FIELD),
                    Op::List(FIELD),
                    Op::OptNode,
                    Op::Int,
                    Op::List(EMIT),
                ],
                declaration::UNION => &[Op::Str, Op::List(UNION_CASE), Op::List(REFS), Op::OptRef],
                declaration::TYPE_ALIAS => &[Op::Str],
                _ => return None,
            },
        })
    }
}

// ------------------------------------------------------------------------------------------------
// Writer
// ------------------------------------------------------------------------------------------------

/// Writes an artifact as an image.
///
/// Fails only when the artifact's tables do not follow the layouts, which is a bug in whatever
/// built them.
pub fn write_nx_ir_image(artifact: &NxIrArtifact) -> Result<Vec<u8>, NxIrImageError> {
    let mut sections: Vec<(u32, Vec<u8>)> = Vec::with_capacity(7);
    sections.push((section::STRINGS, write_strings(&artifact.strings)));
    sections.push((section::MODULE, cells_to_bytes(&write_module(artifact)?)));
    for table in Table::ALL {
        let entries = match table {
            Table::Types => &artifact.types,
            Table::Constants => &artifact.constants,
            Table::Nodes => &artifact.nodes,
            Table::Declarations => &artifact.declarations,
        };
        sections.push((
            table.section(),
            cells_to_bytes(&write_table(table, entries)?),
        ));
    }
    if let Some(debug) = &artifact.debug {
        sections.push((section::DEBUG, write_debug(debug)?));
    }

    let directory_len = sections.len() * DIRECTORY_ENTRY_LEN;
    let total =
        HEADER_LEN + directory_len + sections.iter().map(|(_, bytes)| bytes.len()).sum::<usize>();
    let total = u32::try_from(total).map_err(|_| malformed("the image exceeds 4 GiB"))?;

    let mut image = Vec::with_capacity(total as usize);
    image.extend_from_slice(&NX_IR_MAGIC);
    image.extend_from_slice(&artifact.schema_version.to_le_bytes());
    image.extend_from_slice(&total.to_le_bytes());
    image.extend_from_slice(&(sections.len() as u32).to_le_bytes());
    let mut offset = HEADER_LEN + directory_len;
    for (kind, bytes) in &sections {
        image.extend_from_slice(&kind.to_le_bytes());
        image.extend_from_slice(&(offset as u32).to_le_bytes());
        image.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        offset += bytes.len();
    }
    for (_, bytes) in &sections {
        image.extend_from_slice(bytes);
    }
    debug_assert_eq!(image.len(), total as usize);
    Ok(image)
}

fn cells_to_bytes(cells: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(cells.len() * 4);
    for cell in cells {
        bytes.extend_from_slice(&cell.to_le_bytes());
    }
    bytes
}

fn pad_to_four(bytes: &mut Vec<u8>) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
}

fn write_strings(strings: &[String]) -> Vec<u8> {
    let mut cells = Vec::with_capacity(strings.len() + 2);
    cells.push(strings.len() as u32);
    let mut blob = Vec::new();
    cells.push(0);
    for string in strings {
        blob.extend_from_slice(string.as_bytes());
        cells.push(blob.len() as u32);
    }
    pad_to_four(&mut blob);
    let mut bytes = cells_to_bytes(&cells);
    bytes.extend_from_slice(&blob);
    bytes
}

fn write_module(artifact: &NxIrArtifact) -> Result<Vec<u32>, NxIrImageError> {
    let string_index = |value: &str| -> Result<u32, NxIrImageError> {
        artifact
            .strings
            .iter()
            .position(|candidate| candidate == value)
            .map(|index| index as u32)
            .ok_or_else(|| malformed(format!("the string {value:?} is not in the string table")))
    };
    let mut cells = Vec::new();
    cells.push(string_index(&artifact.runtime_abi)?);
    cells.push(artifact.required_features.len() as u32);
    for feature in &artifact.required_features {
        cells.push(string_index(feature)?);
    }
    cells.push(artifact.modules.len() as u32);
    for module in &artifact.modules {
        cells.push(string_index(&module.identity)?);
        cells.push(string_index(&module.version)?);
        cells.push(module.fingerprint as u32);
        cells.push((module.fingerprint >> 32) as u32);
    }
    for entrypoints in [
        &artifact.function_entrypoints,
        &artifact.component_entrypoints,
    ] {
        cells.push(entrypoints.len() as u32);
        cells.extend_from_slice(entrypoints);
    }
    Ok(cells)
}

fn write_table(table: Table, entries: &[IrItem]) -> Result<Vec<u32>, NxIrImageError> {
    let mut pool = Vec::new();
    let mut offsets = Vec::with_capacity(entries.len() + 1);
    offsets.push(0);
    for (index, entry) in entries.iter().enumerate() {
        write_entry(table, entry, &mut pool)
            .map_err(|error| malformed(format!("{} {index}: {error}", table.name())))?;
        offsets.push(pool.len() as u32);
    }
    let mut cells = Vec::with_capacity(1 + offsets.len() + pool.len());
    cells.push(entries.len() as u32);
    cells.extend_from_slice(&offsets);
    cells.extend_from_slice(&pool);
    Ok(cells)
}

fn write_entry(table: Table, entry: &IrItem, pool: &mut Vec<u32>) -> Result<(), String> {
    let items = entry.as_list().ok_or("the entry is not a list")?;
    let kind = items
        .first()
        .and_then(IrItem::as_int)
        .ok_or("the entry has no kind")?;
    let layout = table
        .layout(kind)
        .ok_or_else(|| format!("kind {kind} is not one the schema assigns"))?;
    pool.push(cell(kind)?);
    write_seq(layout, &items[1..], pool)
}

fn write_seq(ops: &[Op], mut items: &[IrItem], pool: &mut Vec<u32>) -> Result<(), String> {
    for op in ops {
        items = write_op(*op, items, pool)?;
    }
    if items.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} operand(s) more than the layout has",
            items.len()
        ))
    }
}

/// Writes one operand from the front of `items` and returns the rest.
fn write_op<'a>(op: Op, items: &'a [IrItem], pool: &mut Vec<u32>) -> Result<&'a [IrItem], String> {
    let (first, rest) = items.split_first().ok_or("an operand is missing")?;
    let int = |item: &IrItem| {
        item.as_int()
            .ok_or("an operand is not an integer".to_string())
    };
    match op {
        Op::Int | Op::Code(_) | Op::Str | Op::Type | Op::Const | Op::Node => {
            pool.push(cell(int(first)?)?);
        }
        Op::OptNode | Op::OptSlot | Op::OptStr | Op::OptType => {
            let value = int(first)?;
            pool.push(if value == -1 { NONE } else { cell(value)? });
        }
        Op::Ref => {
            let (name, rest) = rest
                .split_first()
                .ok_or("a reference is missing its name")?;
            pool.push(cell(int(first)?)?);
            pool.push(cell(int(name)?)?);
            return Ok(rest);
        }
        Op::RefPair => {
            let pair = first.as_list().ok_or("a reference is not a pair")?;
            if pair.len() != 2 {
                return Err("a reference is not a pair".to_string());
            }
            pool.push(cell(int(&pair[0])?)?);
            pool.push(cell(int(&pair[1])?)?);
        }
        Op::OptRef => {
            let pair = first
                .as_list()
                .ok_or("an optional reference is not a list")?;
            match pair {
                [] => {
                    pool.push(NONE);
                    pool.push(NONE);
                }
                [slot, name] => {
                    pool.push(cell(int(slot)?)?);
                    pool.push(cell(int(name)?)?);
                }
                _ => return Err("an optional reference is not a pair".to_string()),
            }
        }
        Op::List(shape) => {
            let elements = first.as_list().ok_or("a list operand is not a list")?;
            pool.push(u32::try_from(elements.len()).map_err(|_| "a list is too long")?);
            for element in elements {
                if let [op] = shape {
                    let rest = write_op(*op, std::slice::from_ref(element), pool)?;
                    if !rest.is_empty() {
                        return Err("a list element has too many operands".to_string());
                    }
                } else {
                    let items = element.as_list().ok_or("a list element is not a list")?;
                    write_seq(shape, items, pool)?;
                }
            }
        }
        Op::I64 => {
            let value = int(first)? as u64;
            pool.push(value as u32);
            pool.push((value >> 32) as u32);
        }
        Op::F64 => {
            let value = match first {
                IrItem::Float(value) => value.to_bits(),
                _ => return Err("a float constant is not a float".to_string()),
            };
            pool.push(value as u32);
            pool.push((value >> 32) as u32);
        }
    }
    Ok(rest)
}

fn cell(value: i64) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{value} does not fit a cell"))
}

fn write_debug(debug: &NxIrDebug) -> Result<Vec<u8>, NxIrImageError> {
    let mut cells = Vec::new();
    for spans in [&debug.spans.declarations, &debug.spans.nodes] {
        cells.push(spans.len() as u32);
        for span in spans {
            for offset in span {
                cells.push(if *offset == -1 {
                    NONE
                } else {
                    cell(*offset).map_err(|error| malformed(format!("span offset {error}")))?
                });
            }
        }
    }
    cells.push(debug.source.len() as u32);
    let mut bytes = cells_to_bytes(&cells);
    bytes.extend_from_slice(debug.source.as_bytes());
    pad_to_four(&mut bytes);
    Ok(bytes)
}

// ------------------------------------------------------------------------------------------------
// Reader
// ------------------------------------------------------------------------------------------------

/// A borrowed run of little-endian 32-bit cells.
///
/// The bytes are not required to be aligned, so a cell is read with `from_le_bytes` rather than
/// through a `&[u32]`; the reader borrows the image and allocates nothing.
#[derive(Debug, Clone, Copy)]
pub struct Cells<'a>(&'a [u8]);

impl<'a> Cells<'a> {
    /// Cells over `bytes`, whose length is a multiple of four.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        debug_assert_eq!(bytes.len() % 4, 0);
        Self(bytes)
    }

    /// The number of cells.
    pub fn len(&self) -> usize {
        self.0.len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The cell at `index`, or `None` past the end.
    pub fn get(&self, index: usize) -> Option<u32> {
        let start = index.checked_mul(4)?;
        let bytes = self.0.get(start..start.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// The cells from `start` to `end`, or `None` when the range is not inside this run.
    pub fn slice(&self, start: usize, end: usize) -> Option<Cells<'a>> {
        if start > end {
            return None;
        }
        self.0
            .get(start.checked_mul(4)?..end.checked_mul(4)?)
            .map(Cells)
    }

    /// Every cell, in order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + 'a {
        self.0
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// The cells as a vector.
    pub fn to_vec(&self) -> Vec<u32> {
        self.iter().collect()
    }
}

/// An offset array over a pool: the string table's shape and every cell table's.
#[derive(Debug, Clone, Copy)]
struct Offsets<'a> {
    count: usize,
    /// `count + 1` cells.
    offsets: Cells<'a>,
}

impl<'a> Offsets<'a> {
    fn range(&self, index: usize) -> Option<(usize, usize)> {
        if index >= self.count {
            return None;
        }
        Some((
            self.offsets.get(index)? as usize,
            self.offsets.get(index + 1)? as usize,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
struct StringTable<'a> {
    offsets: Offsets<'a>,
    /// UTF-8, checked by `read_strings`, with every offset on a character boundary.
    blob: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct CellTable<'a> {
    offsets: Offsets<'a>,
    pool: Cells<'a>,
}

#[derive(Debug, Clone, Copy)]
struct ModuleSection<'a> {
    runtime_abi: u32,
    features: Cells<'a>,
    /// Four cells per module: identity, version, fingerprint low, fingerprint high.
    modules: Cells<'a>,
    function_entrypoints: Cells<'a>,
    component_entrypoints: Cells<'a>,
}

#[derive(Debug, Clone, Copy)]
struct DebugSection<'a> {
    /// Two cells per declaration.
    declaration_spans: Cells<'a>,
    /// Two cells per node.
    node_spans: Cells<'a>,
    /// UTF-8, checked by `read_debug`.
    source: &'a [u8],
}

/// One entry of the module table, borrowed from an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NxIrModuleRef<'a> {
    pub identity: &'a str,
    pub version: &'a str,
    pub fingerprint: u64,
}

/// A validated view of an image. See the module documentation.
#[derive(Debug, Clone, Copy)]
pub struct NxIrImage<'a> {
    schema_version: u32,
    strings: StringTable<'a>,
    module: ModuleSection<'a>,
    tables: [CellTable<'a>; 4],
    debug: Option<DebugSection<'a>>,
}

/// The image's sections, as the directory lists them.
struct Directory<'a> {
    strings: &'a [u8],
    module: &'a [u8],
    tables: [&'a [u8]; 4],
    debug: Option<&'a [u8]>,
}

impl<'a> NxIrImage<'a> {
    /// Opens and validates an image. See the module documentation for what is checked.
    pub fn open(bytes: &'a [u8]) -> Result<Self, NxIrImageError> {
        let (schema_version, directory) = read_header(bytes)?;
        let strings = read_strings(directory.strings)?;
        let tables = [
            read_table(directory.tables[0], Table::Types)?,
            read_table(directory.tables[1], Table::Constants)?,
            read_table(directory.tables[2], Table::Nodes)?,
            read_table(directory.tables[3], Table::Declarations)?,
        ];
        let bounds = Bounds {
            strings: strings.offsets.count,
            types: tables[0].offsets.count,
            constants: tables[1].offsets.count,
            nodes: tables[2].offsets.count,
            declarations: tables[3].offsets.count,
            modules: 0,
        };
        let module = read_module(directory.module, &bounds)?;
        let bounds = Bounds {
            modules: module.modules.len() / 4,
            ..bounds
        };
        for (table, cells) in Table::ALL.iter().zip(tables.iter()) {
            validate_table(*table, cells, &bounds)?;
        }
        let debug = directory
            .debug
            .map(|bytes| read_debug(bytes, &bounds))
            .transpose()?;
        Ok(Self {
            schema_version,
            strings,
            module,
            tables,
            debug,
        })
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The number of strings in the string table.
    pub fn string_count(&self) -> usize {
        self.strings.offsets.count
    }

    /// String `index`, borrowed from the image, or `None` past the table's end.
    pub fn string(&self, index: u32) -> Option<&'a str> {
        let (start, end) = self.strings.offsets.range(index as usize)?;
        std::str::from_utf8(self.strings.blob.get(start..end)?).ok()
    }

    fn string_at(&self, index: u32) -> &'a str {
        self.string(index).expect("validated string index")
    }

    pub fn runtime_abi(&self) -> &'a str {
        self.string_at(self.module.runtime_abi)
    }

    pub fn required_features(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.module
            .features
            .iter()
            .map(move |index| self.string_at(index))
    }

    /// The module table; slot `0` is the image's own module.
    pub fn modules(&self) -> impl Iterator<Item = NxIrModuleRef<'a>> + '_ {
        (0..self.module.modules.len() / 4).map(move |slot| self.module_at(slot))
    }

    fn module_at(&self, slot: usize) -> NxIrModuleRef<'a> {
        let cells = &self.module.modules;
        let at = |offset: usize| cells.get(slot * 4 + offset).expect("validated module slot");
        NxIrModuleRef {
            identity: self.string_at(at(0)),
            version: self.string_at(at(1)),
            fingerprint: u64::from(at(2)) | (u64::from(at(3)) << 32),
        }
    }

    /// Declaration indices of the module's top-level functions.
    pub fn function_entrypoints(&self) -> Cells<'a> {
        self.module.function_entrypoints
    }

    /// Declaration indices of the module's top-level components.
    pub fn component_entrypoints(&self) -> Cells<'a> {
        self.module.component_entrypoints
    }

    fn table(&self, table: Table) -> &CellTable<'a> {
        &self.tables[table as usize]
    }

    /// The number of entries in `table`.
    pub fn entry_count(&self, table: Table) -> usize {
        self.table(table).offsets.count
    }

    /// Entry `index` of `table`, kind cell first, or `None` past the table's end.
    pub fn entry(&self, table: Table, index: u32) -> Option<Cells<'a>> {
        let table = self.table(table);
        let (start, end) = table.offsets.range(index as usize)?;
        table.pool.slice(start, end)
    }

    /// Whether the image carries its debug section.
    pub fn has_debug(&self) -> bool {
        self.debug.is_some()
    }

    /// The module's source text, when the debug section is present.
    pub fn source(&self) -> Option<&'a str> {
        std::str::from_utf8(self.debug?.source).ok()
    }

    /// The span of declaration `index`, when the debug section is present and the span is known.
    pub fn declaration_span(&self, index: u32) -> Option<(u32, u32)> {
        span_at(self.debug?.declaration_spans, index as usize)
    }

    /// The span of node `index`, when the debug section is present and the span is known.
    pub fn node_span(&self, index: u32) -> Option<(u32, u32)> {
        span_at(self.debug?.node_spans, index as usize)
    }

    /// The name of declaration `index`, or `None` past the list's end.
    pub fn declaration_name(&self, index: u32) -> Option<&'a str> {
        let entry = self.entry(Table::Declarations, index)?;
        self.string(entry.get(1)?)
    }

    /// Calls `visit` with the module slot and the name's string index of every reference any
    /// entry of any table holds, an absent optional reference excepted.
    ///
    /// <para>This is what a linker needs before it reads a single node: which declarations of
    /// which other modules the image names, and which of its own.</para>
    pub fn for_each_reference(&self, mut visit: impl FnMut(u32, u32)) {
        for table in Table::ALL {
            for index in 0..self.entry_count(table) {
                let Some(entry) = self.entry(table, index as u32) else {
                    continue;
                };
                let mut cursor = Cursor::new(entry);
                let Some(layout) = cursor
                    .next()
                    .ok()
                    .and_then(|kind| table.layout(i64::from(kind)))
                else {
                    continue;
                };
                visit_references(&mut cursor, layout, &mut visit);
            }
        }
    }

    /// Rebuilds the owned artifact model from the image.
    pub fn to_artifact(&self) -> NxIrArtifact {
        let strings = (0..self.string_count())
            .map(|index| self.string_at(index as u32).to_string())
            .collect();
        let table = |table: Table| -> Vec<IrItem> {
            (0..self.entry_count(table))
                .map(|index| {
                    decode_entry(
                        table,
                        self.entry(table, index as u32).expect("validated entry"),
                    )
                })
                .collect()
        };
        let debug = self.debug.map(|debug| {
            let spans = |cells: Cells<'a>| -> Vec<[i64; 2]> {
                (0..cells.len() / 2)
                    .map(|index| match span_at(cells, index) {
                        Some((start, end)) => [i64::from(start), i64::from(end)],
                        None => [-1, -1],
                    })
                    .collect()
            };
            NxIrDebug {
                spans: NxIrDebugSpans {
                    declarations: spans(debug.declaration_spans),
                    nodes: spans(debug.node_spans),
                },
                source: String::from_utf8_lossy(debug.source).into_owned(),
            }
        });
        NxIrArtifact {
            schema_version: self.schema_version,
            runtime_abi: self.runtime_abi().to_string(),
            required_features: self.required_features().map(str::to_string).collect(),
            modules: self
                .modules()
                .map(|module| NxIrModuleEntry {
                    identity: module.identity.to_string(),
                    version: module.version.to_string(),
                    fingerprint: module.fingerprint,
                })
                .collect(),
            function_entrypoints: self.function_entrypoints().to_vec(),
            component_entrypoints: self.component_entrypoints().to_vec(),
            strings,
            types: table(Table::Types),
            constants: table(Table::Constants),
            nodes: table(Table::Nodes),
            declarations: table(Table::Declarations),
            debug,
        }
    }
}

/// Where each part of a validated image lies in its bytes.
///
/// <para>[`NxIrImage`] borrows the bytes, so something that owns them cannot also hold the view.
/// It holds this instead: the ranges validation found, from which the view is rebuilt by slicing,
/// without reading a cell.</para>
#[derive(Debug, Clone)]
struct Layout {
    schema_version: u32,
    string_count: usize,
    string_offsets: Range<usize>,
    string_blob: Range<usize>,
    runtime_abi: u32,
    features: Range<usize>,
    modules: Range<usize>,
    function_entrypoints: Range<usize>,
    component_entrypoints: Range<usize>,
    /// Entry count, offsets and pool of each cell table, in [`Table::ALL`] order.
    tables: [(usize, Range<usize>, Range<usize>); 4],
    /// Declaration spans, node spans and source.
    debug: Option<(Range<usize>, Range<usize>, Range<usize>)>,
}

/// The range `part` occupies in `bytes`. `part` is a subslice of `bytes`, or empty.
fn range_in(bytes: &[u8], part: &[u8]) -> Range<usize> {
    let start = (part.as_ptr() as usize).wrapping_sub(bytes.as_ptr() as usize);
    match start.checked_add(part.len()) {
        Some(end) if !part.is_empty() && end <= bytes.len() => start..end,
        _ => 0..0,
    }
}

impl Layout {
    fn of(bytes: &[u8], image: &NxIrImage<'_>) -> Self {
        let range = |part: &[u8]| range_in(bytes, part);
        let table = |table: &CellTable<'_>| {
            (
                table.offsets.count,
                range(table.offsets.offsets.0),
                range(table.pool.0),
            )
        };
        Self {
            schema_version: image.schema_version,
            string_count: image.strings.offsets.count,
            string_offsets: range(image.strings.offsets.offsets.0),
            string_blob: range(image.strings.blob),
            runtime_abi: image.module.runtime_abi,
            features: range(image.module.features.0),
            modules: range(image.module.modules.0),
            function_entrypoints: range(image.module.function_entrypoints.0),
            component_entrypoints: range(image.module.component_entrypoints.0),
            tables: [
                table(&image.tables[0]),
                table(&image.tables[1]),
                table(&image.tables[2]),
                table(&image.tables[3]),
            ],
            debug: image.debug.map(|debug| {
                (
                    range(debug.declaration_spans.0),
                    range(debug.node_spans.0),
                    range(debug.source),
                )
            }),
        }
    }

    fn view<'a>(&self, bytes: &'a [u8]) -> NxIrImage<'a> {
        let part = |range: &Range<usize>| bytes.get(range.clone()).unwrap_or(&[]);
        let cells = |range: &Range<usize>| Cells(part(range));
        let table = |(count, offsets, pool): &(usize, Range<usize>, Range<usize>)| CellTable {
            offsets: Offsets {
                count: *count,
                offsets: cells(offsets),
            },
            pool: cells(pool),
        };
        NxIrImage {
            schema_version: self.schema_version,
            strings: StringTable {
                offsets: Offsets {
                    count: self.string_count,
                    offsets: cells(&self.string_offsets),
                },
                blob: part(&self.string_blob),
            },
            module: ModuleSection {
                runtime_abi: self.runtime_abi,
                features: cells(&self.features),
                modules: cells(&self.modules),
                function_entrypoints: cells(&self.function_entrypoints),
                component_entrypoints: cells(&self.component_entrypoints),
            },
            tables: [
                table(&self.tables[0]),
                table(&self.tables[1]),
                table(&self.tables[2]),
                table(&self.tables[3]),
            ],
            debug: self
                .debug
                .as_ref()
                .map(|(declarations, nodes, source)| DebugSection {
                    declaration_spans: cells(declarations),
                    node_spans: cells(nodes),
                    source: part(source),
                }),
        }
    }
}

/// A validated image together with the bytes it reads.
///
/// <para>[`NxIrImage`] borrows its bytes, which suits a caller that reads an image and drops it.
/// A caller that keeps an image — a runtime holding a prepared module — owns the bytes here
/// instead. The image is validated once, by [`NxIrImageBuf::open`]; [`NxIrImageBuf::image`] then
/// rebuilds the view from the recorded layout and reads nothing, so it is cheap enough to call
/// wherever a view is wanted. The bytes are shared, never copied, by a clone.</para>
#[derive(Debug, Clone)]
pub struct NxIrImageBuf {
    bytes: Arc<[u8]>,
    layout: Layout,
}

impl NxIrImageBuf {
    /// Validates `bytes` as [`NxIrImage::open`] does and keeps them.
    pub fn open(bytes: impl Into<Arc<[u8]>>) -> Result<Self, NxIrImageError> {
        let bytes: Arc<[u8]> = bytes.into();
        let layout = Layout::of(&bytes, &NxIrImage::open(&bytes)?);
        Ok(Self { bytes, layout })
    }

    /// The image's bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The validated view, rebuilt without validating again.
    pub fn image(&self) -> NxIrImage<'_> {
        self.layout.view(&self.bytes)
    }
}

fn span_at(cells: Cells<'_>, index: usize) -> Option<(u32, u32)> {
    let start = cells.get(index * 2)?;
    let end = cells.get(index * 2 + 1)?;
    if start == NONE || end == NONE {
        None
    } else {
        Some((start, end))
    }
}

fn read_header(bytes: &[u8]) -> Result<(u32, Directory<'_>), NxIrImageError> {
    if bytes.len() < 4 || bytes[..4] != NX_IR_MAGIC {
        return Err(NxIrImageError::NotAnImage);
    }
    let header = Cells::from_bytes(
        bytes
            .get(4..HEADER_LEN)
            .ok_or_else(|| malformed("the header is cut short"))?,
    );
    let schema_version = header.get(0).unwrap();
    if schema_version != NX_IR_SCHEMA_VERSION {
        return Err(NxIrImageError::SchemaVersion {
            found: schema_version,
            supported: NX_IR_SCHEMA_VERSION,
        });
    }
    let total = header.get(1).unwrap() as usize;
    if total != bytes.len() {
        return Err(malformed(format!(
            "the header says the image is {total} bytes but {} were given",
            bytes.len()
        )));
    }
    let count = header.get(2).unwrap() as usize;
    let directory_end = count
        .checked_mul(DIRECTORY_ENTRY_LEN)
        .and_then(|len| len.checked_add(HEADER_LEN))
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| {
            malformed(format!(
                "the directory of {count} entries does not fit the image"
            ))
        })?;
    let directory = Cells::from_bytes(&bytes[HEADER_LEN..directory_end]);

    let mut sections: [Option<&[u8]>; 7] = [None; 7];
    for entry in 0..count {
        let kind = directory.get(entry * 3).unwrap();
        let offset = directory.get(entry * 3 + 1).unwrap() as usize;
        let length = directory.get(entry * 3 + 2).unwrap() as usize;
        if !offset.is_multiple_of(4) || !length.is_multiple_of(4) {
            return Err(malformed(format!(
                "section {kind} is not four-byte aligned"
            )));
        }
        let section = offset
            .checked_add(length)
            .and_then(|end| bytes.get(offset..end))
            .ok_or_else(|| malformed(format!("section {kind} lies outside the image")))?;
        if offset < directory_end {
            return Err(malformed(format!("section {kind} overlaps the header")));
        }
        let Some(slot) = sections.get_mut(kind as usize) else {
            // A kind this reader does not know: skipped, so a section can be added without a
            // schema change.
            continue;
        };
        if slot.is_some() {
            return Err(malformed(format!("section {kind} is listed twice")));
        }
        *slot = Some(section);
    }
    let required = |kind: u32| {
        sections[kind as usize].ok_or_else(|| malformed(format!("section {kind} is missing")))
    };
    Ok((
        schema_version,
        Directory {
            strings: required(section::STRINGS)?,
            module: required(section::MODULE)?,
            tables: [
                required(section::TYPES)?,
                required(section::CONSTANTS)?,
                required(section::NODES)?,
                required(section::DECLARATIONS)?,
            ],
            debug: sections[section::DEBUG as usize],
        },
    ))
}

/// Reads `count` and the `count + 1` offsets that head an offset-array section, and returns them
/// with the bytes that follow.
fn read_offsets<'a>(
    bytes: &'a [u8],
    what: &str,
) -> Result<(Offsets<'a>, &'a [u8]), NxIrImageError> {
    let cells = Cells::from_bytes(bytes);
    let count = cells
        .get(0)
        .ok_or_else(|| malformed(format!("the {what} section is empty")))? as usize;
    let offsets_end = count
        .checked_add(2)
        .filter(|end| *end <= cells.len())
        .ok_or_else(|| malformed(format!("the {what} offsets do not fit their section")))?;
    let offsets = cells.slice(1, offsets_end).unwrap();
    let mut previous = 0;
    for offset in offsets.iter() {
        if offset < previous {
            return Err(malformed(format!("the {what} offsets decrease")));
        }
        previous = offset;
    }
    if offsets.get(0) != Some(0) {
        return Err(malformed(format!(
            "the {what} offsets do not start at zero"
        )));
    }
    Ok((Offsets { count, offsets }, &bytes[offsets_end * 4..]))
}

fn read_strings(bytes: &[u8]) -> Result<StringTable<'_>, NxIrImageError> {
    let (offsets, rest) = read_offsets(bytes, "string")?;
    let blob_len = offsets.offsets.get(offsets.count).unwrap() as usize;
    let padded = blob_len.div_ceil(4) * 4;
    if padded != rest.len() {
        return Err(malformed(format!(
            "the string blob is {blob_len} bytes but its section leaves {} for it",
            rest.len()
        )));
    }
    let blob = std::str::from_utf8(&rest[..blob_len])
        .map_err(|error| malformed(format!("the string blob is not UTF-8: {error}")))?;
    for offset in offsets.offsets.iter() {
        if !blob.is_char_boundary(offset as usize) {
            return Err(malformed(format!(
                "string offset {offset} is inside a character"
            )));
        }
    }
    Ok(StringTable {
        offsets,
        blob: blob.as_bytes(),
    })
}

fn read_table(bytes: &[u8], table: Table) -> Result<CellTable<'_>, NxIrImageError> {
    let (offsets, rest) = read_offsets(bytes, table.name())?;
    let pool = Cells::from_bytes(rest);
    let end = offsets.offsets.get(offsets.count).unwrap() as usize;
    if end != pool.len() {
        return Err(malformed(format!(
            "the {} pool is {} cells but its offsets end at {end}",
            table.name(),
            pool.len()
        )));
    }
    Ok(CellTable { offsets, pool })
}

/// The table sizes an operand may index.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    strings: usize,
    types: usize,
    constants: usize,
    nodes: usize,
    declarations: usize,
    modules: usize,
}

impl Bounds {
    fn check(&self, what: &str, index: u32, count: usize) -> Result<(), NxIrImageError> {
        if (index as usize) < count {
            Ok(())
        } else {
            Err(malformed(format!(
                "{what} index {index} is out of range (the table has {count})"
            )))
        }
    }
}

/// A cursor over one entry's cells.
struct Cursor<'a> {
    cells: Cells<'a>,
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(cells: Cells<'a>) -> Self {
        Self { cells, position: 0 }
    }

    fn next(&mut self) -> Result<u32, NxIrImageError> {
        let cell = self
            .cells
            .get(self.position)
            .ok_or_else(|| malformed("an entry ends before its layout does"))?;
        self.position += 1;
        Ok(cell)
    }

    fn finished(&self) -> bool {
        self.position == self.cells.len()
    }
}

fn read_module<'a>(bytes: &'a [u8], bounds: &Bounds) -> Result<ModuleSection<'a>, NxIrImageError> {
    let cells = Cells::from_bytes(bytes);
    let mut cursor = Cursor::new(cells);
    let runtime_abi = cursor.next()?;
    bounds.check("runtime ABI string", runtime_abi, bounds.strings)?;
    let list = |cursor: &mut Cursor<'a>,
                what: &str,
                count: usize|
     -> Result<Cells<'a>, NxIrImageError> {
        let len = cursor.next()? as usize;
        let start = cursor.position;
        let cells = cells
            .slice(start, start.saturating_add(len))
            .ok_or_else(|| malformed(format!("the {what} list does not fit the module section")))?;
        for index in cells.iter() {
            bounds.check(what, index, count)?;
        }
        cursor.position = start + len;
        Ok(cells)
    };
    let features = list(&mut cursor, "required feature string", bounds.strings)?;
    let module_count = cursor.next()? as usize;
    if module_count == 0 {
        return Err(malformed("the module table is empty"));
    }
    let start = cursor.position;
    let modules = cells
        .slice(start, start.saturating_add(module_count.saturating_mul(4)))
        .ok_or_else(|| malformed("the module table does not fit the module section"))?;
    for slot in 0..module_count {
        bounds.check(
            "module identity string",
            modules.get(slot * 4).unwrap(),
            bounds.strings,
        )?;
        bounds.check(
            "module version string",
            modules.get(slot * 4 + 1).unwrap(),
            bounds.strings,
        )?;
    }
    cursor.position = start + module_count * 4;
    let function_entrypoints = list(
        &mut cursor,
        "function entrypoint declaration",
        bounds.declarations,
    )?;
    let component_entrypoints = list(
        &mut cursor,
        "component entrypoint declaration",
        bounds.declarations,
    )?;
    if !cursor.finished() {
        return Err(malformed(
            "the module section has cells after the entrypoints",
        ));
    }
    Ok(ModuleSection {
        runtime_abi,
        features,
        modules,
        function_entrypoints,
        component_entrypoints,
    })
}

fn validate_table(
    table: Table,
    cells: &CellTable<'_>,
    bounds: &Bounds,
) -> Result<(), NxIrImageError> {
    for index in 0..cells.offsets.count {
        let (start, end) = cells.offsets.range(index).unwrap();
        let entry = cells
            .pool
            .slice(start, end)
            .ok_or_else(|| malformed(format!("{} {index} lies outside its pool", table.name())))?;
        // A node's children precede it and a type's inner type precedes it, so an entry may only
        // name earlier entries of its own table and no entry can reach itself. Bounding by the
        // entry's index is what makes the walk in `ir_explain` finite on any accepted image.
        let bounds = Bounds {
            nodes: if table == Table::Nodes {
                index
            } else {
                bounds.nodes
            },
            types: if table == Table::Types {
                index
            } else {
                bounds.types
            },
            ..*bounds
        };
        validate_entry(table, entry, &bounds)
            .and_then(|()| {
                if table == Table::Types {
                    validate_seq_type(cells, entry)
                } else {
                    Ok(())
                }
            })
            .map_err(|error| malformed(format!("{} {index}: {error}", table.name())))?;
    }
    Ok(())
}

/// The rules a `seq` type entry keeps beyond its layout: the occurrence cell spells a suffix, and
/// the item is an exactly-one type, never itself a `seq`. The entry's layout was checked already,
/// so its item names an earlier type entry.
fn validate_seq_type(types: &CellTable<'_>, entry: Cells<'_>) -> Result<(), String> {
    if entry.get(0).map(i64::from) != Some(kinds::ty::SEQ) {
        return Ok(());
    }
    let (Some(item), Some(cell)) = (entry.get(1), entry.get(2)) else {
        return Err("the seq type is shorter than its layout".to_string());
    };
    if kinds::ty::occurrence_from_cell(i64::from(cell)).is_none() {
        return Err(format!(
            "the seq type carries the occurrence cell {cell}, which spells no suffix"
        ));
    }
    let item_kind = types
        .offsets
        .range(item as usize)
        .and_then(|(start, end)| types.pool.slice(start, end))
        .and_then(|item_entry| item_entry.get(0));
    if item_kind.map(i64::from) == Some(kinds::ty::SEQ) {
        return Err(format!(
            "the seq type's item type {item} is itself a seq type"
        ));
    }
    Ok(())
}

fn validate_entry(table: Table, entry: Cells<'_>, bounds: &Bounds) -> Result<(), String> {
    let mut cursor = Cursor::new(entry);
    let kind = cursor.next().map_err(message)?;
    let layout = table
        .layout(i64::from(kind))
        .ok_or_else(|| format!("kind {kind} is not one the schema assigns"))?;
    validate_seq(&mut cursor, layout, bounds).map_err(message)?;
    if cursor.finished() {
        Ok(())
    } else {
        Err(format!(
            "the entry has {} cell(s) more than its layout",
            entry.len() - cursor.position
        ))
    }
}

fn validate_seq(
    cursor: &mut Cursor<'_>,
    ops: &[Op],
    bounds: &Bounds,
) -> Result<(), NxIrImageError> {
    for op in ops {
        validate_op(cursor, *op, bounds)?;
    }
    Ok(())
}

fn validate_op(cursor: &mut Cursor<'_>, op: Op, bounds: &Bounds) -> Result<(), NxIrImageError> {
    let optional =
        |cursor: &mut Cursor<'_>, what: &str, count: usize| -> Result<(), NxIrImageError> {
            let cell = cursor.next()?;
            if cell == NONE {
                Ok(())
            } else {
                bounds.check(what, cell, count)
            }
        };
    let reference = |cursor: &mut Cursor<'_>| -> Result<(), NxIrImageError> {
        let slot = cursor.next()?;
        let name = cursor.next()?;
        bounds.check("module slot", slot, bounds.modules)?;
        bounds.check("reference name string", name, bounds.strings)
    };
    match op {
        Op::Int | Op::OptSlot => {
            cursor.next()?;
        }
        Op::Code(names) => {
            let code = cursor.next()?;
            if kinds::name(names, i64::from(code)).is_none() {
                return Err(malformed(format!(
                    "code {code} is not one the schema assigns"
                )));
            }
        }
        Op::Str => bounds.check("string", cursor.next()?, bounds.strings)?,
        Op::Type => bounds.check("type", cursor.next()?, bounds.types)?,
        Op::Const => bounds.check("constant", cursor.next()?, bounds.constants)?,
        Op::Node => bounds.check("node", cursor.next()?, bounds.nodes)?,
        Op::OptNode => optional(cursor, "node", bounds.nodes)?,
        Op::OptType => optional(cursor, "type", bounds.types)?,
        Op::OptStr => optional(cursor, "string", bounds.strings)?,
        Op::Ref | Op::RefPair => reference(cursor)?,
        Op::OptRef => {
            let slot = cursor.next()?;
            let name = cursor.next()?;
            if slot != NONE {
                bounds.check("module slot", slot, bounds.modules)?;
                bounds.check("reference name string", name, bounds.strings)?;
            }
        }
        Op::List(shape) => {
            let count = cursor.next()?;
            for _ in 0..count {
                validate_seq(cursor, shape, bounds)?;
            }
        }
        Op::I64 | Op::F64 => {
            cursor.next()?;
            cursor.next()?;
        }
    }
    Ok(())
}

fn read_debug<'a>(bytes: &'a [u8], bounds: &Bounds) -> Result<DebugSection<'a>, NxIrImageError> {
    let cells = Cells::from_bytes(bytes);
    let mut cursor = Cursor::new(cells);
    let spans = |cursor: &mut Cursor<'a>,
                 what: &str,
                 expected: usize|
     -> Result<Cells<'a>, NxIrImageError> {
        let count = cursor.next()? as usize;
        if count != expected {
            return Err(malformed(format!(
                "the debug section has {count} {what} spans for {expected} {what}s"
            )));
        }
        let start = cursor.position;
        let spans = cells
            .slice(start, start.saturating_add(count.saturating_mul(2)))
            .ok_or_else(|| malformed(format!("the {what} spans do not fit the debug section")))?;
        cursor.position = start + count * 2;
        Ok(spans)
    };
    let declaration_spans = spans(&mut cursor, "declaration", bounds.declarations)?;
    let node_spans = spans(&mut cursor, "node", bounds.nodes)?;
    let source_len = cursor.next()? as usize;
    let source_start = cursor.position * 4;
    let padded = source_len.div_ceil(4) * 4;
    if source_start.saturating_add(padded) != bytes.len() {
        return Err(malformed(format!(
            "the source is {source_len} bytes but the debug section leaves {} for it",
            bytes.len().saturating_sub(source_start)
        )));
    }
    let source = std::str::from_utf8(&bytes[source_start..source_start + source_len])
        .map_err(|error| malformed(format!("the source is not UTF-8: {error}")))?;
    Ok(DebugSection {
        declaration_spans,
        node_spans,
        source: source.as_bytes(),
    })
}

// ------------------------------------------------------------------------------------------------
// Decoder
// ------------------------------------------------------------------------------------------------

/// Walks a validated entry's operands by its layout, reporting each reference.
fn visit_references(cursor: &mut Cursor<'_>, ops: &[Op], visit: &mut impl FnMut(u32, u32)) {
    for op in ops {
        match *op {
            Op::Int
            | Op::Code(_)
            | Op::Str
            | Op::Type
            | Op::Const
            | Op::Node
            | Op::OptNode
            | Op::OptSlot
            | Op::OptStr
            | Op::OptType => {
                let _ = cursor.next();
            }
            Op::I64 | Op::F64 => {
                let _ = cursor.next();
                let _ = cursor.next();
            }
            Op::Ref | Op::RefPair | Op::OptRef => {
                if let (Ok(slot), Ok(name)) = (cursor.next(), cursor.next()) {
                    if slot != NONE {
                        visit(slot, name);
                    }
                }
            }
            Op::List(shape) => {
                let count = cursor.next().unwrap_or(0);
                for _ in 0..count {
                    if cursor.finished() {
                        break;
                    }
                    visit_references(cursor, shape, visit);
                }
            }
        }
    }
}

/// Rebuilds a validated entry as the model's nested list. The cursor cannot run out or meet an
/// unknown kind, because validation walked the same layout; a `None` is a bug in this file.
fn decode_entry(table: Table, entry: Cells<'_>) -> IrItem {
    let mut cursor = Cursor::new(entry);
    let kind = cursor.next().expect("validated kind");
    let layout = table.layout(i64::from(kind)).expect("validated kind");
    let mut items = vec![IrItem::Int(i64::from(kind))];
    decode_seq(&mut cursor, layout, &mut items);
    IrItem::List(items)
}

fn decode_seq(cursor: &mut Cursor<'_>, ops: &[Op], items: &mut Vec<IrItem>) {
    for op in ops {
        decode_op(cursor, *op, items);
    }
}

fn decode_op(cursor: &mut Cursor<'_>, op: Op, items: &mut Vec<IrItem>) {
    let mut next = || i64::from(cursor.next().expect("validated cell"));
    match op {
        Op::Int | Op::Code(_) | Op::Str | Op::Type | Op::Const | Op::Node => {
            items.push(IrItem::Int(next()));
        }
        Op::OptNode | Op::OptSlot | Op::OptStr | Op::OptType => {
            let cell = next();
            items.push(IrItem::Int(if cell == i64::from(NONE) { -1 } else { cell }));
        }
        Op::Ref => {
            items.push(IrItem::Int(next()));
            items.push(IrItem::Int(next()));
        }
        Op::RefPair => {
            let slot = next();
            let name = next();
            items.push(IrItem::ints([slot, name]));
        }
        Op::OptRef => {
            let slot = next();
            let name = next();
            items.push(if slot == i64::from(NONE) {
                IrItem::list([])
            } else {
                IrItem::ints([slot, name])
            });
        }
        Op::List(shape) => {
            let count = next();
            let mut elements = Vec::with_capacity(count as usize);
            for _ in 0..count {
                if let [op] = shape {
                    decode_op(cursor, *op, &mut elements);
                } else {
                    let mut element = Vec::with_capacity(shape.len());
                    decode_seq(cursor, shape, &mut element);
                    elements.push(IrItem::List(element));
                }
            }
            items.push(IrItem::List(elements));
        }
        Op::I64 => {
            let low = next() as u64;
            let high = next() as u64;
            items.push(IrItem::Int((low | (high << 32)) as i64));
        }
        Op::F64 => {
            let low = next() as u64;
            let high = next() as u64;
            items.push(IrItem::Float(f64::from_bits(low | (high << 32))));
        }
    }
}
