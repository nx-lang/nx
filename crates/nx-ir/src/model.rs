//! The NX IR model, schema 5: the constants, the kind numbers and the tables of one artifact.
//!
//! <para>An artifact carries the module's string, type, constant and node tables and its
//! declaration list, plus a module table naming every module it references. A reference is a
//! module-table slot and a declaration name, never a position, so a module regenerated with a new
//! declaration in the middle still satisfies every artifact compiled against the old one. The
//! layout of every table entry is documented in `docs/nx-ir-format.md`; the kind numbers live in
//! [`kinds`] and are never reused. The model here is what an emitter builds; the bytes a host
//! receives are the image `image` writes from it.</para>

pub const NX_IR_SCHEMA_VERSION: u32 = 5;
pub const NX_IR_RUNTIME_ABI: &str = "nx-ir-runtime-v2";
/// Required by a module that declares a derived update record, so a runtime that predates them
/// refuses the module rather than normalizing a patch as a whole record.
pub const NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1: &str = "update-records-v1";
/// Required by a module that declares a derived property union, so a runtime that predates them
/// rejects the module rather than misreading the declaration.
pub const NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1: &str = "property-unions-v1";
/// Required by a module that calls an update intrinsic, so a runtime that predates them rejects
/// the module rather than failing on an unknown node.
pub const NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1: &str = "update-intrinsics-v1";
/// Required by a module that binds an action handler, so a runtime that predates them refuses the
/// module by name rather than failing on an unknown node.
pub const NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1: &str = "action-handlers-v1";

/// A module carrying a function type, a function referenced as a value, or a call of a
/// function-typed value by name, which a runtime that predates function values cannot run.
pub const NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1: &str = "function-values-v1";

/// Required by a module that iterates a range, so a runtime that predates ranges refuses the module
/// by name rather than failing on an unknown node.
pub const NX_IR_REQUIRED_FEATURE_RANGES_V1: &str = "ranges-v1";

/// The feature a module lists when its nodes use a presence operator (`x?`, `x?.m`, `x ?? y`) or
/// a match arm carries the `{}` pattern, so a runtime that predates them refuses it by name.
pub const NX_IR_REQUIRED_FEATURE_OCCURRENCE_V1: &str = "occurrence-v1";

/// The kind numbers of schema 5. A number, once assigned, is never reused for anything else.
pub mod kinds {
    /// Node kinds: the first element of every `nodes` entry.
    pub mod node {
        /// Retired with schema 5: the language has no null value. The number stays assigned so
        /// no later kind reuses it, and a schema-5 emitter never writes it.
        pub const NULL: i64 = 0;
        pub const BOOL: i64 = 1;
        pub const STRING: i64 = 2;
        pub const NUMBER: i64 = 3;
        pub const SLOT: i64 = 4;
        pub const REFERENCE: i64 = 5;
        pub const BINARY: i64 = 6;
        pub const UNARY: i64 = 7;
        pub const CALL: i64 = 8;
        pub const INTRINSIC: i64 = 9;
        pub const IF: i64 = 10;
        pub const IF_IS: i64 = 11;
        pub const ARRAY: i64 = 12;
        pub const FOR: i64 = 13;
        pub const MEMBER: i64 = 14;
        pub const RECORD: i64 = 15;
        pub const UNION_CASE: i64 = 16;
        pub const ELEMENT: i64 = 17;
        pub const COMPONENT: i64 = 18;
        pub const ACTION_HANDLER: i64 = 19;
        pub const TEXT: i64 = 20;
        pub const NAMED_CALL: i64 = 21;
        pub const FOR_RANGE: i64 = 22;
        /// `x?`: `[23, operand]`.
        pub const EXISTS: i64 = 23;
        /// `x?.m`: `[24, receiver, member]`.
        pub const OPTIONAL_MEMBER: i64 = 24;
        /// `x ?? y`: `[25, left, right]`.
        pub const COALESCE: i64 = 25;

        pub const NAMES: &[(i64, &str)] = &[
            (BOOL, "bool"),
            (STRING, "string"),
            (NUMBER, "number"),
            (SLOT, "slot"),
            (REFERENCE, "reference"),
            (BINARY, "binary"),
            (UNARY, "unary"),
            (CALL, "call"),
            (INTRINSIC, "intrinsic"),
            (IF, "if"),
            (IF_IS, "ifIs"),
            (ARRAY, "array"),
            (FOR, "for"),
            (MEMBER, "member"),
            (RECORD, "record"),
            (UNION_CASE, "unionCase"),
            (ELEMENT, "element"),
            (COMPONENT, "component"),
            (ACTION_HANDLER, "actionHandler"),
            (TEXT, "text"),
            (NAMED_CALL, "namedCall"),
            (FOR_RANGE, "forRange"),
            (EXISTS, "exists"),
            (OPTIONAL_MEMBER, "optionalMember"),
            (COALESCE, "coalesce"),
        ];
    }

    /// Type kinds: the first element of every `types` entry.
    pub mod ty {
        pub const PRIMITIVE: i64 = 0;
        pub const NOMINAL: i64 = 1;
        /// Retired with schema 5, replaced by `SEQ`. The number stays assigned and is never
        /// emitted; a schema-5 reader reports an entry of this kind as malformed.
        pub const ARRAY: i64 = 2;
        /// Retired with schema 5, replaced by `SEQ`; see `ARRAY`.
        pub const NULLABLE: i64 = 3;
        pub const FUNCTION: i64 = 4;
        /// An exactly-one item type under an occurrence: `[5, item, occurrence]`, where the
        /// occurrence cell is `OCCURRENCE_EMPTY | OCCURRENCE_MANY` bits — `1` for `?`, `2` for
        /// `+`, `3` for `*`.
        pub const SEQ: i64 = 5;

        /// Bit 0 of a function type parameter's flags cell: the parameter takes body content.
        pub const FUNCTION_PARAM_CONTENT: i64 = 1;
        /// Bit 1 of a function type parameter's flags cell: the parameter is optional (`p?:T`).
        pub const FUNCTION_PARAM_OPTIONAL: i64 = 2;

        /// Bit 0 of a `SEQ` type's occurrence cell: the type admits no value.
        pub const OCCURRENCE_EMPTY: i64 = 1;
        /// Bit 1 of a `SEQ` type's occurrence cell: the type admits more than one value.
        pub const OCCURRENCE_MANY: i64 = 2;

        pub const NAMES: &[(i64, &str)] = &[
            (PRIMITIVE, "primitive"),
            (NOMINAL, "nominal"),
            (FUNCTION, "function"),
            (SEQ, "seq"),
        ];

        /// How many values a `SEQ` type admits: the two bits of its occurrence cell.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct Occurrence {
            /// The type admits no value: `?` or `*`.
            pub may_be_empty: bool,
            /// The type admits more than one value: `+` or `*`.
            pub may_be_many: bool,
        }

        impl Occurrence {
            /// The suffix that spells this occurrence: `"?"`, `"+"` or `"*"`.
            pub fn suffix(self) -> &'static str {
                match (self.may_be_empty, self.may_be_many) {
                    (false, false) => "",
                    (true, false) => "?",
                    (false, true) => "+",
                    (true, true) => "*",
                }
            }
        }

        impl std::fmt::Display for Occurrence {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.suffix())
            }
        }

        /// The occurrence cell of a `SEQ` type entry.
        pub fn occurrence_cell(may_be_empty: bool, may_be_many: bool) -> i64 {
            i64::from(may_be_empty) * OCCURRENCE_EMPTY + i64::from(may_be_many) * OCCURRENCE_MANY
        }

        /// The occurrence a `SEQ` type entry's cell encodes, or `None` for a cell no suffix
        /// spells (exactly one is never written as a `SEQ`).
        pub fn occurrence_from_cell(cell: i64) -> Option<Occurrence> {
            let occ = Occurrence {
                may_be_empty: cell & OCCURRENCE_EMPTY != 0,
                may_be_many: cell & OCCURRENCE_MANY != 0,
            };
            let spelled = occ.may_be_empty || occ.may_be_many;
            (cell & !(OCCURRENCE_EMPTY | OCCURRENCE_MANY) == 0 && spelled).then_some(occ)
        }
    }

    /// Constant kinds: the first element of every `constants` entry.
    pub mod constant {
        pub const INT: i64 = 0;
        pub const BIGINT: i64 = 1;
        pub const FLOAT: i64 = 2;

        pub const NAMES: &[(i64, &str)] = &[(INT, "int"), (BIGINT, "bigint"), (FLOAT, "float")];
    }

    /// Declaration kinds: the first element of every `declarations` entry.
    pub mod declaration {
        pub const FUNCTION: i64 = 0;
        pub const VALUE: i64 = 1;
        pub const RECORD: i64 = 2;
        pub const COMPONENT: i64 = 3;
        pub const UNION: i64 = 4;
        pub const TYPE_ALIAS: i64 = 5;

        /// Bit 0 of a function declaration's result flags: the result type, declared or inferred,
        /// is a standalone `T?`, so an entry call returns an empty result to the host as `null`.
        pub const RESULT_OPTIONAL: i64 = 1;

        pub const NAMES: &[(i64, &str)] = &[
            (FUNCTION, "function"),
            (VALUE, "value"),
            (RECORD, "record"),
            (COMPONENT, "component"),
            (UNION, "union"),
            (TYPE_ALIAS, "typeAlias"),
        ];
    }

    /// Binary operators: the second element of a `binary` node.
    pub mod binary {
        pub const ADD: i64 = 0;
        pub const SUB: i64 = 1;
        pub const MUL: i64 = 2;
        pub const DIV: i64 = 3;
        pub const IDIV: i64 = 4;
        pub const MOD: i64 = 5;
        pub const IMOD: i64 = 6;
        pub const CONCAT: i64 = 7;
        pub const EQ: i64 = 8;
        pub const NE: i64 = 9;
        pub const LT: i64 = 10;
        pub const LE: i64 = 11;
        pub const GT: i64 = 12;
        pub const GE: i64 = 13;
        pub const AND: i64 = 14;
        pub const OR: i64 = 15;
        pub const FADD32: i64 = 16;
        pub const FSUB32: i64 = 17;
        pub const FMUL32: i64 = 18;
        pub const FDIV32: i64 = 19;

        pub const NAMES: &[(i64, &str)] = &[
            (ADD, "add"),
            (SUB, "sub"),
            (MUL, "mul"),
            (DIV, "div"),
            (IDIV, "idiv"),
            (MOD, "mod"),
            (IMOD, "imod"),
            (CONCAT, "concat"),
            (EQ, "eq"),
            (NE, "ne"),
            (LT, "lt"),
            (LE, "le"),
            (GT, "gt"),
            (GE, "ge"),
            (AND, "and"),
            (OR, "or"),
            (FADD32, "fadd32"),
            (FSUB32, "fsub32"),
            (FMUL32, "fmul32"),
            (FDIV32, "fdiv32"),
        ];
    }

    /// Unary operators: the second element of a `unary` node.
    pub mod unary {
        pub const NEG: i64 = 0;
        pub const NOT: i64 = 1;

        pub const NAMES: &[(i64, &str)] = &[(NEG, "neg"), (NOT, "not")];
    }

    /// Update intrinsics: the second element of an `intrinsic` node.
    pub mod intrinsic {
        pub const APPLY: i64 = 0;
        pub const MERGE: i64 = 1;
        pub const DIFF: i64 = 2;
        pub const CHANGED: i64 = 3;

        pub const NAMES: &[(i64, &str)] = &[
            (APPLY, "apply"),
            (MERGE, "merge"),
            (DIFF, "diff"),
            (CHANGED, "changed"),
        ];
    }

    /// Field flags: bits of the last element of a field entry.
    pub mod field {
        pub const CONTENT: i64 = 1;
        pub const REQUIRED: i64 = 2;
    }

    /// Component flags: bits of the last element of a component declaration.
    pub mod component {
        pub const ABSTRACT: i64 = 1;
        pub const EXTERNAL: i64 = 2;
    }

    /// The name a kind table gives a number, or `None` for a number it does not assign.
    pub fn name(table: &[(i64, &'static str)], kind: i64) -> Option<&'static str> {
        table
            .iter()
            .find(|(number, _)| *number == kind)
            .map(|(_, name)| *name)
    }
}

/// One operand of a table entry: an integer, a float constant's value, or a nested list.
///
/// Every table entry is a list whose first element is its kind; the layouts are in
/// `docs/nx-ir-format.md`. Keeping the operands untyped in Rust is deliberate: the artifact is
/// defined by the document, and this crate's readers walk the lists the same way the image's
/// layouts are written.
#[derive(Debug, Clone, PartialEq)]
pub enum IrItem {
    Int(i64),
    Float(f64),
    List(Vec<IrItem>),
}

impl IrItem {
    pub fn list(items: impl IntoIterator<Item = IrItem>) -> Self {
        Self::List(items.into_iter().collect())
    }

    pub fn ints(items: impl IntoIterator<Item = i64>) -> Self {
        Self::List(items.into_iter().map(IrItem::Int).collect())
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[IrItem]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }
}

/// One NX IR artifact: one module and the tables that encode it.
#[derive(Debug, Clone, PartialEq)]
pub struct NxIrArtifact {
    pub schema_version: u32,
    pub runtime_abi: String,
    pub required_features: Vec<String>,
    /// Slot 0 is the artifact's own module; the rest are the modules it references directly.
    pub modules: Vec<NxIrModuleEntry>,
    /// Declaration indices of the module's top-level functions, in declaration order.
    pub function_entrypoints: Vec<u32>,
    /// Declaration indices of the module's top-level components, in declaration order.
    pub component_entrypoints: Vec<u32>,
    pub strings: Vec<String>,
    pub types: Vec<IrItem>,
    pub constants: Vec<IrItem>,
    pub nodes: Vec<IrItem>,
    pub declarations: Vec<IrItem>,
    pub debug: Option<NxIrDebug>,
}

/// One entry of the module table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxIrModuleEntry {
    pub identity: String,
    /// The version string the build was given for the module, or empty.
    pub version: String,
    /// A hash of the module's identity and source text.
    pub fingerprint: u64,
}

/// The optional debug section: spans parallel to the declaration list and node table, and the
/// module's source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxIrDebug {
    pub spans: NxIrDebugSpans,
    pub source: String,
}

/// Byte offsets into the source; `[-1, -1]` for a node written in another module's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxIrDebugSpans {
    pub declarations: Vec<[i64; 2]>,
    pub nodes: Vec<[i64; 2]>,
}

/// What to emit from a program.
impl NxIrArtifact {
    /// The name of the declaration at `index`, if the entry is well formed.
    pub fn declaration_name(&self, index: u32) -> Option<&str> {
        let entry = self.declarations.get(index as usize)?.as_list()?;
        let name = entry.get(1)?.as_int()?;
        self.strings
            .get(usize::try_from(name).ok()?)
            .map(String::as_str)
    }
}
