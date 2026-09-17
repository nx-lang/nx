//! The NX IR image: schema 4 as a little-endian binary a runtime reads in place.
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

use crate::ir::{
    kinds, IrItem, NxIrArtifact, NxIrDebug, NxIrDebugSpans, NxIrModuleEntry, NX_IR_SCHEMA_VERSION,
};

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
const STRS: &[Op] = &[Op::Str];
const REFS: &[Op] = &[Op::RefPair];
const PROPERTY: &[Op] = &[Op::Str, Op::Node];
const FIELD: &[Op] = &[Op::Str, Op::Type, Op::OptNode, Op::Int];
const PARAM: &[Op] = &[Op::Str, Op::Type, Op::Int];
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
                ty::ARRAY | ty::NULLABLE => &[Op::Type],
                _ => return None,
            },
            Table::Constants => match kind {
                constant::INT => &[Op::I64],
                constant::BIGINT => &[Op::Str],
                constant::FLOAT => &[Op::F64],
                _ => return None,
            },
            Table::Nodes => match kind {
                node::NULL => &[],
                node::BOOL => &[Op::Int],
                node::STRING => &[Op::Str],
                node::NUMBER => &[Op::Const],
                node::SLOT => &[Op::Int, Op::Str],
                node::REFERENCE => &[Op::Ref],
                node::BINARY => &[Op::Code(kinds::binary::NAMES), Op::Node, Op::Node],
                node::UNARY => &[Op::Code(kinds::unary::NAMES), Op::Node],
                node::TEXT => &[Op::Node, Op::Str],
                node::CALL => &[Op::Node, Op::List(NODES)],
                node::INTRINSIC => &[
                    Op::Code(kinds::intrinsic::NAMES),
                    Op::List(NODES),
                    Op::List(STRS),
                ],
                node::IF => &[Op::Node, Op::Node, Op::OptNode],
                node::IF_IS => &[Op::Node, Op::List(ARM), Op::OptNode],
                node::ARRAY => &[Op::List(NODES)],
                node::FOR => &[
                    Op::Int,
                    Op::Str,
                    Op::OptSlot,
                    Op::OptStr,
                    Op::Node,
                    Op::Node,
                ],
                node::MEMBER => &[Op::Node, Op::Str],
                node::RECORD | node::COMPONENT => &[Op::Ref, Op::List(PROPERTY), Op::List(NODES)],
                node::UNION_CASE => &[Op::Ref, Op::Str, Op::List(PROPERTY), Op::List(NODES)],
                node::ELEMENT => &[Op::Int, Op::Str, Op::List(PROPERTY), Op::List(NODES)],
                node::ACTION_HANDLER => &[Op::Ref, Op::Str, Op::Ref, Op::Int, Op::OptRef, Op::Node],
                _ => return None,
            },
            Table::Declarations => match kind {
                declaration::FUNCTION => &[Op::Str, Op::List(PARAM), Op::Node],
                declaration::VALUE => &[Op::Str, Op::Node],
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
    while bytes.len() % 4 != 0 {
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
        Op::OptNode | Op::OptSlot | Op::OptStr => {
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
    fn from_bytes(bytes: &'a [u8]) -> Self {
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
            .chunks_exact(4)
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
    blob: &'a str,
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
    source: &'a str,
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
        self.strings.blob.get(start..end)
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
        self.debug.map(|debug| debug.source)
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
                source: debug.source.to_string(),
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
        if offset % 4 != 0 || length % 4 != 0 {
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
    Ok(StringTable { offsets, blob })
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
            .map_err(|error| malformed(format!("{} {index}: {error}", table.name())))?;
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
        source,
    })
}

// ------------------------------------------------------------------------------------------------
// Decoder
// ------------------------------------------------------------------------------------------------

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
        Op::OptNode | Op::OptSlot | Op::OptStr => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_corpus_tests::{emit, load_programs};

    fn cell_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    /// Every corpus artifact, with and without debug data, as the model and its image.
    fn corpus_images() -> Vec<(String, NxIrArtifact, Vec<u8>)> {
        let mut images = Vec::new();
        for program in load_programs() {
            for debug in [false, true] {
                for artifact in emit(&program, debug) {
                    let label = format!(
                        "{}/{}{}",
                        program.name,
                        artifact.modules[0].identity,
                        if debug { " +debug" } else { "" }
                    );
                    let bytes = write_nx_ir_image(&artifact).expect("image");
                    images.push((label, artifact, bytes));
                }
            }
        }
        images
    }

    fn snippet_input() -> NxIrArtifact {
        let program = load_programs()
            .into_iter()
            .find(|program| program.name == "snippet")
            .expect("the snippet program");
        emit(&program, false)
            .into_iter()
            .find(|artifact| artifact.modules[0].identity == "input.nx")
            .expect("input.nx")
    }

    /// The image of the corpus's `snippet` program, checked cell by cell against the layout
    /// `docs/nx-ir-format.md` gives.
    #[test]
    fn the_snippet_image_is_laid_out_as_the_document_says() {
        let artifact = snippet_input();
        let bytes = write_nx_ir_image(&artifact).expect("image");

        assert_eq!(&bytes[0..4], b"NXIR");
        assert_eq!(cell_at(&bytes, 4), 4, "schema version");
        assert_eq!(cell_at(&bytes, 8) as usize, bytes.len(), "total length");
        assert_eq!(cell_at(&bytes, 12), 6, "six sections without debug");

        // The directory: kinds 0 to 5, contiguous, each four-byte aligned.
        let mut expected_offset = 16 + 6 * 12;
        let mut sections = Vec::new();
        for entry in 0..6 {
            let at = 16 + entry * 12;
            assert_eq!(cell_at(&bytes, at), entry as u32, "section kind");
            assert_eq!(
                cell_at(&bytes, at + 4) as usize,
                expected_offset,
                "section offset"
            );
            let length = cell_at(&bytes, at + 8) as usize;
            assert_eq!(length % 4, 0);
            sections.push((expected_offset, length));
            expected_offset += length;
        }
        assert_eq!(expected_offset, bytes.len());

        // Strings: count, count + 1 offsets, blob. The first string is "title".
        let (strings, _) = sections[0];
        assert_eq!(cell_at(&bytes, strings) as usize, artifact.strings.len());
        assert_eq!(cell_at(&bytes, strings + 4), 0);
        assert_eq!(cell_at(&bytes, strings + 8), 5);
        let blob = strings + 4 + 4 * (artifact.strings.len() + 1);
        assert_eq!(&bytes[blob..blob + 5], b"title");

        // Module: runtime ABI, no features, two modules, entrypoints [1] and [].
        let (module, module_len) = sections[1];
        let cells = Cells::from_bytes(&bytes[module..module + module_len]).to_vec();
        let string_index =
            |value: &str| artifact.strings.iter().position(|s| s == value).unwrap() as u32;
        let abi = string_index("nx-ir-runtime-v2");
        let drawnui = artifact.modules[1].fingerprint;
        assert_eq!(
            cells,
            vec![
                abi,
                0,
                2,
                string_index("input.nx"),
                string_index(""),
                artifact.modules[0].fingerprint as u32,
                (artifact.modules[0].fingerprint >> 32) as u32,
                string_index("drawnui.nx"),
                string_index("9"),
                drawnui as u32,
                (drawnui >> 32) as u32,
                1,
                1,
                0,
            ]
        );

        // Declarations: `[1, 0, 0]` (value title = node 0) and `[0, 2, [], 13]` (function root).
        let (declarations, declarations_len) = sections[5];
        let cells =
            Cells::from_bytes(&bytes[declarations..declarations + declarations_len]).to_vec();
        assert_eq!(cells, vec![2, 0, 3, 7, 1, 0, 0, 0, 2, 0, 13]);

        // Node 13, `[18, 1, 14, [[3, 1], [5, 2]], [5, 7, 9, 12]]`: the component descriptor of
        // `SkiaLayout` with two properties and four children.
        let image = NxIrImage::open(&bytes).expect("valid");
        let node = image.entry(Table::Nodes, 13).expect("node 13").to_vec();
        assert_eq!(node, vec![18, 1, 14, 2, 3, 1, 5, 2, 4, 5, 7, 9, 12]);
        assert_eq!(image.string(14), Some("SkiaLayout"));
    }

    #[test]
    fn every_corpus_image_reads_back_as_its_model() {
        for (label, artifact, bytes) in corpus_images() {
            let image = NxIrImage::open(&bytes).unwrap_or_else(|error| panic!("{label}: {error}"));
            assert_eq!(image.to_artifact(), artifact, "{label}");
            assert_eq!(image.has_debug(), artifact.debug.is_some(), "{label}");
        }
    }

    #[test]
    fn a_stripped_image_and_a_debug_image_differ_only_in_the_debug_section() {
        for program in load_programs() {
            let stripped = emit(&program, false);
            let with_debug = emit(&program, true);
            for (stripped, with_debug) in stripped.iter().zip(&with_debug) {
                let stripped_bytes = write_nx_ir_image(stripped).expect("image");
                let debug_bytes = write_nx_ir_image(with_debug).expect("image");
                // Past the header and directory, the debug image is the stripped image's sections
                // followed by the debug section.
                let stripped_body = &stripped_bytes[16 + 6 * 12..];
                let debug_body = &debug_bytes[16 + 7 * 12..];
                assert!(debug_body.starts_with(stripped_body), "{}", program.name);
            }
        }
    }

    #[test]
    fn an_unknown_section_is_skipped() {
        let bytes = write_nx_ir_image(&snippet_input()).expect("image");
        // Rebuild the image with an extra directory entry of kind 99 pointing at four zero bytes
        // appended to the end.
        let count = 7u32;
        let directory_len = 16 + 7 * 12;
        let mut image = Vec::new();
        image.extend_from_slice(&bytes[..12]);
        image.extend_from_slice(&count.to_le_bytes());
        for entry in 0..6 {
            let at = 16 + entry * 12;
            image.extend_from_slice(&bytes[at..at + 4]);
            image.extend_from_slice(&(cell_at(&bytes, at + 4) + 12).to_le_bytes());
            image.extend_from_slice(&bytes[at + 8..at + 12]);
        }
        image.extend_from_slice(&99u32.to_le_bytes());
        image.extend_from_slice(&((bytes.len() + 12) as u32).to_le_bytes());
        image.extend_from_slice(&4u32.to_le_bytes());
        image.extend_from_slice(&bytes[16 + 6 * 12..]);
        image.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(image.len(), bytes.len() + 16);
        assert_eq!(directory_len, 16 + 7 * 12);
        let total = image.len() as u32;
        image[8..12].copy_from_slice(&total.to_le_bytes());

        let opened = NxIrImage::open(&image).expect("the unknown section is skipped");
        assert_eq!(opened.to_artifact(), snippet_input());
    }

    #[test]
    fn bytes_that_are_not_an_image_are_refused() {
        assert_eq!(
            NxIrImage::open(b"").unwrap_err(),
            NxIrImageError::NotAnImage
        );
        assert_eq!(
            NxIrImage::open(b"{\"format\":\"nx-ir-json\"}").unwrap_err(),
            NxIrImageError::NotAnImage
        );
        assert_eq!(
            NxIrImage::open(b"NXI").unwrap_err(),
            NxIrImageError::NotAnImage
        );
    }

    #[test]
    fn another_schema_version_is_refused_naming_both() {
        let mut bytes = write_nx_ir_image(&snippet_input()).expect("image");
        bytes[4..8].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            NxIrImage::open(&bytes).unwrap_err(),
            NxIrImageError::SchemaVersion {
                found: 3,
                supported: 4
            }
        );
    }

    /// Cut at every four-byte boundary, an image is refused rather than read short.
    #[test]
    fn a_truncated_image_is_refused_at_every_boundary() {
        for (label, _, bytes) in corpus_images() {
            for end in (0..bytes.len()).step_by(4) {
                let error = NxIrImage::open(&bytes[..end]).err().unwrap_or_else(|| {
                    panic!(
                        "{label}: an image cut at {end} of {} bytes opened",
                        bytes.len()
                    )
                });
                assert!(
                    matches!(
                        error,
                        NxIrImageError::NotAnImage | NxIrImageError::Malformed(_)
                    ),
                    "{label} at {end}: {error}"
                );
            }
        }
    }

    /// The image whose header says it is longer than it is, and the one that says it is shorter.
    #[test]
    fn a_wrong_total_length_is_refused() {
        let bytes = write_nx_ir_image(&snippet_input()).expect("image");
        let mut longer = bytes.clone();
        longer[8..12].copy_from_slice(&((bytes.len() + 4) as u32).to_le_bytes());
        assert!(NxIrImage::open(&longer).is_err());
        let mut shorter = bytes.clone();
        shorter[8..12].copy_from_slice(&((bytes.len() - 4) as u32).to_le_bytes());
        assert!(NxIrImage::open(&shorter).is_err());
        let mut padded = bytes.clone();
        padded.extend_from_slice(&[0, 0, 0, 0]);
        assert!(NxIrImage::open(&padded).is_err());
    }

    /// Opens damaged bytes and, when they open, reads everything the view answers, so that a
    /// damaged image is either refused or explained and never panics.
    fn open_damaged(damaged: &[u8]) -> bool {
        match NxIrImage::open(damaged) {
            Ok(image) => {
                // Everything the view answers must come from inside the image.
                let artifact = image.to_artifact();
                let _ = crate::explain_nx_ir(&artifact);
                true
            }
            Err(_) => false,
        }
    }

    /// Every cell of `bytes`, overwritten with each of four values: `open` either refuses the image
    /// or answers with a view whose every accessor is in range, and never panics.
    fn damage_every_cell(label: &str, bytes: &[u8]) {
        let mut opened = 0;
        let mut refused = 0;
        for offset in (0..bytes.len()).step_by(4) {
            for value in [0u32, 1, NONE, 0x7FFF_FFF0] {
                let mut damaged = bytes.to_vec();
                damaged[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                if open_damaged(&damaged) {
                    opened += 1;
                } else {
                    refused += 1;
                }
            }
        }
        println!("{label}: {opened} damaged images opened, {refused} refused");
        assert!(refused > 0);
    }

    /// The smallest corpus image, which is always a stripped one, the smallest that carries a
    /// debug section, so that span offsets and the source length are damaged too, and the smallest
    /// that carries an action handler, so that its node and a non-empty `emits` list are too.
    #[test]
    fn every_cell_can_be_damaged_without_a_panic() {
        let images = corpus_images();
        let (label, _, bytes) = images
            .iter()
            .filter(|(_, artifact, _)| {
                artifact.nodes.iter().any(|node| {
                    node.as_list().and_then(|entry| entry[0].as_int())
                        == Some(kinds::node::ACTION_HANDLER)
                })
            })
            .min_by_key(|(_, _, bytes)| bytes.len())
            .expect("a corpus image with an action handler");
        damage_every_cell(label, bytes);
        let (label, _, bytes) = images
            .iter()
            .min_by_key(|(_, _, bytes)| bytes.len())
            .expect("a corpus image");
        damage_every_cell(label, bytes);
        let (label, _, bytes) = images
            .iter()
            .filter(|(_, artifact, _)| artifact.debug.is_some())
            .min_by_key(|(_, _, bytes)| bytes.len())
            .expect("a corpus image with a debug section");
        damage_every_cell(label, bytes);
    }

    /// The byte offset of cell `cell` of entry `index` in `table`, read from the directory.
    fn table_cell_offset(bytes: &[u8], table: Table, index: usize, cell: usize) -> usize {
        let kind = match table {
            Table::Types => section::TYPES,
            Table::Constants => section::CONSTANTS,
            Table::Nodes => section::NODES,
            Table::Declarations => section::DECLARATIONS,
        };
        let section_offset = cell_at(bytes, 16 + kind as usize * 12 + 4) as usize;
        let count = cell_at(bytes, section_offset) as usize;
        let start = cell_at(bytes, section_offset + 4 + 4 * index) as usize;
        section_offset + 4 + 4 * (count + 1) + 4 * (start + cell)
    }

    /// A node that names itself as a child is refused, since the explainer recurses over children
    /// and the format promises that a child precedes its parent.
    #[test]
    fn a_self_referencing_node_is_refused() {
        let artifact = snippet_input();
        let bytes = write_nx_ir_image(&artifact).expect("image");
        let image = NxIrImage::open(&bytes).expect("valid");
        // Node 13 is `[18, 1, 14, [[3, 1], [5, 2]], [5, 7, 9, 12]]`; its first child is cell 9.
        assert_eq!(image.entry(Table::Nodes, 13).unwrap().get(9), Some(5));
        let offset = table_cell_offset(&bytes, Table::Nodes, 13, 9);
        let mut cyclic = bytes.clone();
        cyclic[offset..offset + 4].copy_from_slice(&13u32.to_le_bytes());
        let error = NxIrImage::open(&cyclic).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("node 13: node index 13 is out of range"),
            "{error}"
        );
        // A child that follows its parent is refused the same way.
        let mut forward = bytes.clone();
        forward[offset..offset + 4].copy_from_slice(&14u32.to_le_bytes());
        assert!(NxIrImage::open(&forward).is_err());
    }

    /// Every node and type cell of every corpus image, overwritten with its own entry index: the
    /// image is refused or explained, and the explainer's walk over children terminates.
    #[test]
    fn every_node_and_type_cell_can_name_its_own_entry_without_a_panic() {
        for (label, _, bytes) in corpus_images() {
            let image = NxIrImage::open(&bytes).expect("valid");
            for table in [Table::Nodes, Table::Types] {
                for index in 0..image.entry_count(table) {
                    let len = image.entry(table, index as u32).unwrap().len();
                    for cell in 0..len {
                        let offset = table_cell_offset(&bytes, table, index, cell);
                        let mut damaged = bytes.clone();
                        damaged[offset..offset + 4].copy_from_slice(&(index as u32).to_le_bytes());
                        let _ = open_damaged(&damaged);
                    }
                }
            }
            println!("{label}: every node and type cell probed with its own index");
        }
    }

    #[test]
    fn an_index_past_a_table_is_refused_rather_than_followed() {
        let artifact = snippet_input();
        let bytes = write_nx_ir_image(&artifact).expect("image");
        let image = NxIrImage::open(&bytes).expect("valid");
        // Node 0 is `[2, 1]`, the string literal "Conformance"; point it past the string table.
        let node_section = 16 + 4 * 12;
        let nodes_offset = cell_at(&bytes, node_section + 4) as usize;
        let node_count = cell_at(&bytes, nodes_offset) as usize;
        let pool = nodes_offset + 4 + 4 * (node_count + 1);
        assert_eq!(image.entry(Table::Nodes, 0).unwrap().to_vec(), vec![2, 1]);
        let mut damaged = bytes.clone();
        damaged[pool + 4..pool + 8].copy_from_slice(&(artifact.strings.len() as u32).to_le_bytes());
        let error = NxIrImage::open(&damaged).unwrap_err();
        assert!(
            error.to_string().contains("node 0: string index"),
            "{error}"
        );
    }
}
