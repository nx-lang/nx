//! NX IR schema 3: one module per artifact, encoded as flat tables.
//!
//! <para>An artifact carries the module's string, type, constant and node tables and its
//! declaration list, plus a module table naming every module it references. A reference is a
//! module-table slot and a declaration name, never a position, so a module regenerated with a new
//! declaration in the middle still satisfies every artifact compiled against the old one. The
//! layout of every table entry is documented in `docs/nx-ir-format.md`; the kind numbers live in
//! [`kinds`] and are never reused. The model here is what the emitter builds; the bytes a host
//! receives are the image `ir_image` writes from it.</para>

use crate::builder::build_codegen_program;
use crate::ir_image::{write_nx_ir_image, NxIrImageError};
use crate::model::{
    CodegenComponent, CodegenComponentField, CodegenDeclaration, CodegenDeclarationKind,
    CodegenExpression, CodegenExpressionKind, CodegenModule, CodegenModuleProvenance,
    CodegenProgram, CodegenProperty, CodegenRecordField, CodegenReference, CodegenSourceEntry,
    CodegenStatement, CodegenTypeRef,
};
use crate::options::CodegenError;
use nx_api::ProgramArtifact;
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::ast::{BinOp, Literal, UnOp};
use nx_hir::UpdateIntrinsic;
use nx_interpreter::RuntimeModuleId;
use nx_types::{Primitive, Type};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const NX_IR_SCHEMA_VERSION: u32 = 3;
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

/// The kind numbers of schema 3. A number, once assigned, is never reused for anything else.
pub mod kinds {
    /// Node kinds: the first element of every `nodes` entry.
    pub mod node {
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

        pub const NAMES: &[(i64, &str)] = &[
            (NULL, "null"),
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
        ];
    }

    /// Type kinds: the first element of every `types` entry.
    pub mod ty {
        pub const PRIMITIVE: i64 = 0;
        pub const NOMINAL: i64 = 1;
        pub const ARRAY: i64 = 2;
        pub const NULLABLE: i64 = 3;

        pub const NAMES: &[(i64, &str)] = &[
            (PRIMITIVE, "primitive"),
            (NOMINAL, "nominal"),
            (ARRAY, "array"),
            (NULLABLE, "nullable"),
        ];
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

mod u64_decimal_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
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
///
/// The SDKs take the same options as JSON: `{ "modules": [...], "debug": false }`, every key
/// optional. A module's version is not an option: it is part of the module, given when the program
/// is built, so every artifact emitted from one program agrees on it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct NxIrEmitOptions {
    /// The identities of the modules to emit an artifact for. `None` emits the entry module alone;
    /// an empty list emits every module of the program, entry first.
    pub modules: Option<Vec<String>>,
    /// Whether each artifact carries its debug section.
    pub debug: bool,
}

impl NxIrEmitOptions {
    /// Parses the SDKs' JSON form; an empty or blank text is the default options.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json)
    }
}

impl NxIrEmitOptions {
    /// The entry module alone, without debug data: what an SDK emits by default.
    pub fn entry_only() -> Self {
        Self::default()
    }

    /// Every module of the program, with debug data: what the CLI writes.
    pub fn every_module_with_debug() -> Self {
        Self {
            modules: Some(Vec::new()),
            debug: true,
        }
    }
}

/// One emitted artifact: its image and what a host learns about it without opening the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedNxIr {
    /// The identity of the module the artifact carries.
    pub identity: String,
    /// The artifact as an NX IR image.
    pub bytes: Vec<u8>,
    pub metadata: NxIrMetadata,
}

/// What a host learns about an artifact without parsing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrMetadata {
    pub identity: String,
    #[serde(with = "u64_decimal_string")]
    pub fingerprint: u64,
    pub schema_version: u32,
    pub runtime_abi: String,
    pub required_features: Vec<String>,
    pub function_entrypoints: Vec<String>,
    pub component_entrypoints: Vec<String>,
}

/// Emits NX IR artifacts from a program artifact.
pub fn emit_nx_ir(
    artifact: &ProgramArtifact,
    options: &NxIrEmitOptions,
) -> Result<Vec<GeneratedNxIr>, CodegenError> {
    let program = build_codegen_program(artifact)?;
    emit_codegen_nx_ir(&program, options)
}

/// Emits NX IR artifacts from a codegen program.
pub fn emit_codegen_nx_ir(
    program: &CodegenProgram,
    options: &NxIrEmitOptions,
) -> Result<Vec<GeneratedNxIr>, CodegenError> {
    build_nx_ir_artifacts(program, options)?
        .into_iter()
        .map(|artifact| {
            let bytes = write_nx_ir_image(&artifact).map_err(image_error)?;
            let metadata = artifact.metadata();
            Ok(GeneratedNxIr {
                identity: metadata.identity.clone(),
                bytes,
                metadata,
            })
        })
        .collect()
}

fn image_error(error: NxIrImageError) -> CodegenError {
    CodegenError::single(
        Diagnostic::error("nx-ir-serialization-error")
            .with_message(format!("failed to write the NX IR image: {error}"))
            .build(),
    )
}

/// Builds the artifact model for each requested module without serializing it.
pub fn build_nx_ir_artifacts(
    program: &CodegenProgram,
    options: &NxIrEmitOptions,
) -> Result<Vec<NxIrArtifact>, CodegenError> {
    validate_ir_program(program)?;
    let modules = selected_modules(program, options)?;
    let mut artifacts = Vec::with_capacity(modules.len());
    for module in modules {
        artifacts.push(ModuleEmitter::new(program, module, options).emit()?);
    }
    Ok(artifacts)
}

impl NxIrArtifact {
    pub fn metadata(&self) -> NxIrMetadata {
        let own = self.modules.first();
        let names = |indices: &[u32]| {
            indices
                .iter()
                .filter_map(|index| self.declaration_name(*index))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        NxIrMetadata {
            identity: own.map(|entry| entry.identity.clone()).unwrap_or_default(),
            fingerprint: own.map(|entry| entry.fingerprint).unwrap_or_default(),
            schema_version: self.schema_version,
            runtime_abi: self.runtime_abi.clone(),
            required_features: self.required_features.clone(),
            function_entrypoints: names(&self.function_entrypoints),
            component_entrypoints: names(&self.component_entrypoints),
        }
    }

    /// The name of the declaration at `index`, if the entry is well formed.
    pub fn declaration_name(&self, index: u32) -> Option<&str> {
        let entry = self.declarations.get(index as usize)?.as_list()?;
        let name = entry.get(1)?.as_int()?;
        self.strings
            .get(usize::try_from(name).ok()?)
            .map(String::as_str)
    }
}

/// The modules the options select, entry first.
fn selected_modules<'a>(
    program: &'a CodegenProgram,
    options: &NxIrEmitOptions,
) -> Result<Vec<&'a CodegenModule>, CodegenError> {
    let by_identity = program
        .modules
        .iter()
        .map(|module| (module_identity(module), module))
        .collect::<BTreeMap<_, _>>();
    let identities = match &options.modules {
        None => vec![program.entry_identity.clone()],
        Some(identities) if identities.is_empty() => {
            let mut all = by_identity.keys().cloned().collect::<Vec<_>>();
            // The entry module leads, so a caller reading the first artifact reads the program's.
            all.retain(|identity| identity != &program.entry_identity);
            all.insert(0, program.entry_identity.clone());
            all
        }
        Some(identities) => identities.clone(),
    };

    let mut diagnostics = Vec::new();
    let mut modules = Vec::with_capacity(identities.len());
    for identity in identities {
        match by_identity.get(&identity) {
            Some(module) => modules.push(*module),
            None => diagnostics.push(
                Diagnostic::error("nx-ir-unknown-module")
                    .with_message(format!(
                        "NX IR emission was asked for module '{identity}', which the program does not contain"
                    ))
                    .build(),
            ),
        }
    }
    if diagnostics.is_empty() {
        Ok(modules)
    } else {
        Err(CodegenError::new(diagnostics))
    }
}

fn module_identity(module: &CodegenModule) -> String {
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => identity.clone(),
        CodegenModuleProvenance::Library { module_path, .. } => module_path.display().to_string(),
    }
}

fn module_source_entry<'a>(
    program: &'a CodegenProgram,
    module: &CodegenModule,
) -> Option<&'a CodegenSourceEntry> {
    let identity = module_identity(module);
    program
        .source_entries
        .iter()
        .find(|entry| entry.identity == identity)
}

fn module_source<'a>(program: &'a CodegenProgram, module: &CodegenModule) -> Option<&'a str> {
    module_source_entry(program, module).map(|entry| entry.source.as_str())
}

/// The version string the host gave a module; `""` for one it gave none, and for a library module.
fn module_version(program: &CodegenProgram, module: &CodegenModule) -> String {
    module_source_entry(program, module)
        .and_then(|entry| entry.version.clone())
        .unwrap_or_default()
}

/// The fingerprint of a module: a hash of its identity and source text, so a regenerated module
/// with the same text keeps its fingerprint and one with different text does not.
///
/// <para>The hash is FNV-1a over the identity, a zero byte, and the source. It is spelled out here
/// rather than taken from `DefaultHasher`, whose algorithm the standard library explicitly does not
/// promise across releases: the fingerprint travels in the artifact, so the same source must hash
/// the same whatever toolchain emitted it. `docs/nx-ir-format.md` says so as part of the format.
/// </para>
fn module_fingerprint(program: &CodegenProgram, module: &CodegenModule) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    let mut write = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
    };
    write(module_identity(module).as_bytes());
    write(&[0]);
    write(
        module_source(program, module)
            .unwrap_or_default()
            .as_bytes(),
    );
    hash
}

/// The features a runtime must support to run `module`.
fn required_features(module: &CodegenModule) -> Vec<String> {
    let mut features: Vec<String> = Vec::new();
    if module.declarations.iter().any(|declaration| {
        matches!(
            &declaration.kind,
            CodegenDeclarationKind::Record {
                update_target: Some(_),
                ..
            }
        )
    }) {
        features.push(NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1.to_string());
    }
    if module.declarations.iter().any(|declaration| {
        matches!(
            &declaration.kind,
            CodegenDeclarationKind::Union {
                property_target: Some(_),
                ..
            }
        )
    }) {
        features.push(NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1.to_string());
    }
    if module.declarations.iter().any(declaration_calls_intrinsic) {
        features.push(NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1.to_string());
    }
    features
}

fn declaration_calls_intrinsic(declaration: &CodegenDeclaration) -> bool {
    let mut found = false;
    visit_declaration_expressions(declaration, &mut |expression| {
        if matches!(expression.kind, CodegenExpressionKind::IntrinsicCall { .. }) {
            found = true;
        }
    });
    found
}

/// Calls `visit` on every expression a declaration owns, parents before children.
fn visit_declaration_expressions(
    declaration: &CodegenDeclaration,
    visit: &mut dyn FnMut(&CodegenExpression),
) {
    let mut fields = |fields: &[CodegenRecordField]| {
        for field in fields {
            if let Some(default) = &field.default {
                visit_expression(default, visit);
            }
        }
    };
    match &declaration.kind {
        CodegenDeclarationKind::Function { body, .. } => visit_expression(body, visit),
        CodegenDeclarationKind::Value { value, .. } => visit_expression(value, visit),
        CodegenDeclarationKind::Record { fields: items, .. } => fields(items),
        CodegenDeclarationKind::Union { cases, .. } => {
            for case in cases {
                fields(&case.fields);
            }
        }
        CodegenDeclarationKind::Component(component) => {
            for field in component.props.iter().chain(component.state.iter()) {
                if let Some(default) = &field.default {
                    visit_expression(default, visit);
                }
            }
            if let Some(body) = &component.body {
                visit_expression(body, visit);
            }
        }
        CodegenDeclarationKind::TypeAlias | CodegenDeclarationKind::Unsupported(_) => {}
    }
}

fn visit_expressions(items: &[CodegenExpression], visit: &mut dyn FnMut(&CodegenExpression)) {
    for item in items {
        visit_expression(item, visit);
    }
}

fn visit_expression(expression: &CodegenExpression, visit: &mut dyn FnMut(&CodegenExpression)) {
    visit(expression);
    match &expression.kind {
        CodegenExpressionKind::Literal(_)
        | CodegenExpressionKind::Identifier { .. }
        | CodegenExpressionKind::Unsupported(_) => {}
        CodegenExpressionKind::Binary { lhs, rhs, .. } => {
            visit_expression(lhs, visit);
            visit_expression(rhs, visit);
        }
        CodegenExpressionKind::Unary { expr, .. } => visit_expression(expr, visit),
        CodegenExpressionKind::Call { callee, args } => {
            visit_expression(callee, visit);
            visit_expressions(args, visit);
        }
        CodegenExpressionKind::IntrinsicCall { args, .. } => visit_expressions(args, visit),
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expression(condition, visit);
            visit_expression(then_branch, visit);
            if let Some(else_branch) = else_branch {
                visit_expression(else_branch, visit);
            }
        }
        CodegenExpressionKind::Match {
            scrutinee,
            arms,
            else_branch,
        } => {
            visit_expression(scrutinee, visit);
            for arm in arms {
                visit_expressions(&arm.patterns, visit);
                visit_expression(&arm.body, visit);
            }
            if let Some(else_branch) = else_branch {
                visit_expression(else_branch, visit);
            }
        }
        CodegenExpressionKind::Let { value, body, .. } => {
            visit_expression(value, visit);
            visit_expression(body, visit);
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            for statement in statements {
                match statement {
                    CodegenStatement::Let { init, .. } => visit_expression(init, visit),
                    CodegenStatement::Expr(expr) => visit_expression(expr, visit),
                }
            }
            if let Some(expression) = expression {
                visit_expression(expression, visit);
            }
        }
        CodegenExpressionKind::Array(elements) => visit_expressions(elements, visit),
        CodegenExpressionKind::For { iterable, body, .. } => {
            visit_expression(iterable, visit);
            visit_expression(body, visit);
        }
        CodegenExpressionKind::Index { base, index } => {
            visit_expression(base, visit);
            visit_expression(index, visit);
        }
        CodegenExpressionKind::Member { base, .. } => visit_expression(base, visit),
        CodegenExpressionKind::UnionCase {
            properties,
            content,
            fields,
            ..
        }
        | CodegenExpressionKind::Record {
            properties,
            content,
            fields,
            ..
        } => {
            for property in properties {
                visit_expression(&property.value, visit);
            }
            visit_expressions(content, visit);
            for field in fields {
                if let Some(default) = &field.default {
                    visit_expression(default, visit);
                }
            }
        }
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            for property in &descriptor.properties {
                visit_expression(&property.value, visit);
            }
            visit_expressions(&descriptor.content, visit);
        }
        CodegenExpressionKind::Element(element) => {
            for property in &element.properties {
                visit_expression(&property.value, visit);
            }
            visit_expressions(&element.content, visit);
        }
    }
}

/// Refuses a program that carries a construct no IR node represents, before any artifact is
/// built, so a failure never leaves a partial artifact behind.
fn validate_ir_program(program: &CodegenProgram) -> Result<(), CodegenError> {
    let mut diagnostics = Vec::new();
    for module in &program.modules {
        for declaration in &module.declarations {
            if let CodegenDeclarationKind::Unsupported(unsupported) = &declaration.kind {
                diagnostics.push(unsupported_diagnostic(
                    module,
                    unsupported.span,
                    &unsupported.message,
                ));
            }
            visit_declaration_expressions(declaration, &mut |expression| {
                if let CodegenExpressionKind::Unsupported(unsupported) = &expression.kind {
                    diagnostics.push(unsupported_diagnostic(
                        module,
                        unsupported.span,
                        &unsupported.message,
                    ));
                }
            });
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(CodegenError::new(diagnostics))
    }
}

fn unsupported_diagnostic(module: &CodegenModule, span: TextSpan, message: &str) -> Diagnostic {
    Diagnostic::error("nx-ir-unsupported-construct")
        .with_message(message.to_string())
        .with_label(Label::primary(module_identity(module), span))
        .build()
}

/// A table that stores each distinct entry once and hands out its index.
struct Interner<T> {
    items: Vec<T>,
    index: HashMap<String, i64>,
}

impl<T> Interner<T> {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, key: String, make: impl FnOnce() -> T) -> i64 {
        if let Some(index) = self.index.get(&key) {
            return *index;
        }
        let index = self.items.len() as i64;
        self.items.push(make());
        self.index.insert(key, index);
        index
    }
}

/// The locals of the declaration being emitted: which integer each visible name reads, and the
/// next integer to hand out.
struct Frame {
    next_slot: i64,
    scopes: Vec<BTreeMap<String, i64>>,
}

impl Frame {
    fn new() -> Self {
        Self {
            next_slot: 0,
            scopes: vec![BTreeMap::new()],
        }
    }

    /// Reserves `count` consecutive slots and returns the first, for the leading parameters or
    /// fields of a frame, which take their declaration position whether or not their defaults
    /// allocate locals of their own.
    fn reserve(&mut self, count: usize) -> i64 {
        let first = self.next_slot;
        self.next_slot += count as i64;
        first
    }

    fn alloc(&mut self, name: &str) -> i64 {
        let slot = self.reserve(1);
        self.bind(name, slot);
        slot
    }

    fn bind(&mut self, name: &str, slot: i64) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), slot);
        }
    }

    fn resolve(&self, name: &str) -> Option<i64> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn push(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}

struct ModuleEmitter<'a> {
    program: &'a CodegenProgram,
    module: &'a CodegenModule,
    options: &'a NxIrEmitOptions,
    strings: Interner<String>,
    types: Interner<IrItem>,
    constants: Interner<IrItem>,
    nodes: Vec<IrItem>,
    node_spans: Vec<[i64; 2]>,
    declarations: Vec<IrItem>,
    declaration_spans: Vec<[i64; 2]>,
    /// The module table, by slot: the module itself first, then every module referenced, in the
    /// order the emitter first met them.
    module_slots: Vec<RuntimeModuleId>,
    frame: Frame,
    /// The module whose text the expressions being emitted were written in. Spans index the
    /// artifact's own source only, so an expression inherited from another module gets none.
    span_module: RuntimeModuleId,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ModuleEmitter<'a> {
    fn new(
        program: &'a CodegenProgram,
        module: &'a CodegenModule,
        options: &'a NxIrEmitOptions,
    ) -> Self {
        Self {
            program,
            module,
            options,
            strings: Interner::new(),
            types: Interner::new(),
            constants: Interner::new(),
            nodes: Vec::new(),
            node_spans: Vec::new(),
            declarations: Vec::new(),
            declaration_spans: Vec::new(),
            module_slots: vec![module.id],
            frame: Frame::new(),
            span_module: module.id,
            diagnostics: Vec::new(),
        }
    }

    fn emit(mut self) -> Result<NxIrArtifact, CodegenError> {
        let mut function_entrypoints = Vec::new();
        let mut component_entrypoints = Vec::new();
        for declaration in &self.module.declarations {
            let index = self.declarations.len() as u32;
            match &declaration.kind {
                CodegenDeclarationKind::Function { .. } => function_entrypoints.push(index),
                CodegenDeclarationKind::Component(_) => component_entrypoints.push(index),
                _ => {}
            }
            self.declaration(declaration);
        }
        if !self.diagnostics.is_empty() {
            return Err(CodegenError::new(self.diagnostics));
        }

        let modules = self
            .module_slots
            .clone()
            .iter()
            .map(|id| {
                let module = self
                    .program
                    .module(*id)
                    .expect("a referenced module is part of the program");
                let identity = module_identity(module);
                let version = module_version(self.program, module);
                // The module section of the image names these through the string table, after
                // every string the declarations use.
                self.string(&identity);
                self.string(&version);
                NxIrModuleEntry {
                    version,
                    fingerprint: module_fingerprint(self.program, module),
                    identity,
                }
            })
            .collect();
        let required_features = required_features(self.module);
        self.string(NX_IR_RUNTIME_ABI);
        for feature in &required_features {
            self.string(feature);
        }
        let debug = self.options.debug.then(|| NxIrDebug {
            spans: NxIrDebugSpans {
                declarations: self.declaration_spans,
                nodes: self.node_spans,
            },
            source: module_source(self.program, self.module)
                .unwrap_or_default()
                .to_string(),
        });

        Ok(NxIrArtifact {
            schema_version: NX_IR_SCHEMA_VERSION,
            runtime_abi: NX_IR_RUNTIME_ABI.to_string(),
            required_features,
            modules,
            function_entrypoints,
            component_entrypoints,
            strings: self.strings.items,
            types: self.types.items,
            constants: self.constants.items,
            nodes: self.nodes,
            declarations: self.declarations,
            debug,
        })
    }

    fn string(&mut self, value: &str) -> i64 {
        self.strings.intern(value.to_string(), || value.to_string())
    }

    fn module_slot(&mut self, id: RuntimeModuleId) -> i64 {
        if let Some(slot) = self.module_slots.iter().position(|slot| *slot == id) {
            return slot as i64;
        }
        self.module_slots.push(id);
        (self.module_slots.len() - 1) as i64
    }

    /// A reference as its two operands: module slot and name.
    fn reference(&mut self, reference: &CodegenReference) -> (i64, i64) {
        let slot = self.module_slot(reference.module_id);
        let name = self.string(&reference.name);
        (slot, name)
    }

    fn reference_item(&mut self, reference: &CodegenReference) -> IrItem {
        let (slot, name) = self.reference(reference);
        IrItem::ints([slot, name])
    }

    fn optional_reference(&mut self, reference: Option<&CodegenReference>) -> IrItem {
        match reference {
            Some(reference) => self.reference_item(reference),
            None => IrItem::List(Vec::new()),
        }
    }

    fn references(&mut self, references: &[CodegenReference]) -> IrItem {
        let items = references
            .iter()
            .map(|reference| self.reference_item(reference))
            .collect::<Vec<_>>();
        IrItem::List(items)
    }

    fn intern_type(&mut self, entry: IrItem) -> i64 {
        let key = format!("{entry:?}");
        self.types.intern(key, || entry)
    }

    fn type_ref(&mut self, ty: &CodegenTypeRef) -> i64 {
        let entry = match ty {
            CodegenTypeRef::Primitive { name } => {
                let name = self.string(name);
                IrItem::ints([kinds::ty::PRIMITIVE, name])
            }
            CodegenTypeRef::Nominal { reference, .. } => {
                let (slot, name) = self.reference(reference);
                IrItem::ints([kinds::ty::NOMINAL, slot, name])
            }
            CodegenTypeRef::Array { element } => {
                let element = self.type_ref(element);
                IrItem::ints([kinds::ty::ARRAY, element])
            }
            CodegenTypeRef::Nullable { inner } => {
                let inner = self.type_ref(inner);
                IrItem::ints([kinds::ty::NULLABLE, inner])
            }
            // NX has no syntax for a function type, so nothing reaches here; the top type stands
            // in rather than a kind the corpus could never cover.
            CodegenTypeRef::Function { .. } => {
                let name = self.string("object");
                IrItem::ints([kinds::ty::PRIMITIVE, name])
            }
        };
        self.intern_type(entry)
    }

    fn constant(&mut self, entry: IrItem) -> i64 {
        let key = format!("{entry:?}");
        self.constants.intern(key, || entry)
    }

    fn span(&self, span: TextSpan) -> [i64; 2] {
        if self.span_module == self.module.id {
            [u32::from(span.start()) as i64, u32::from(span.end()) as i64]
        } else {
            [-1, -1]
        }
    }

    fn push_node(&mut self, entry: IrItem, span: TextSpan) -> i64 {
        let index = self.nodes.len() as i64;
        self.nodes.push(entry);
        let span = self.span(span);
        self.node_spans.push(span);
        index
    }

    fn declaration(&mut self, declaration: &CodegenDeclaration) {
        self.frame = Frame::new();
        self.span_module = self.module.id;
        let name = self.string(&declaration.reference.name);
        let entry = match &declaration.kind {
            CodegenDeclarationKind::Function { params, body, .. } => {
                let first = self.frame.reserve(params.len());
                let params = params
                    .iter()
                    .enumerate()
                    .map(|(index, param)| {
                        self.frame.bind(&param.name, first + index as i64);
                        let name = self.string(&param.name);
                        let ty = self.type_ref(&param.resolved_ty);
                        IrItem::ints([name, ty, i64::from(param.is_content)])
                    })
                    .collect::<Vec<_>>();
                let body = self.expression(body);
                IrItem::list([
                    IrItem::Int(kinds::declaration::FUNCTION),
                    IrItem::Int(name),
                    IrItem::List(params),
                    IrItem::Int(body),
                ])
            }
            CodegenDeclarationKind::Value { value, .. } => {
                let value = self.expression(value);
                IrItem::ints([kinds::declaration::VALUE, name, value])
            }
            CodegenDeclarationKind::Record {
                fields,
                bases,
                is_abstract,
                update_target,
            } => {
                let fields = self.record_fields(fields);
                let bases = self.references(bases);
                let update_target = self.optional_reference(update_target.as_ref());
                IrItem::list([
                    IrItem::Int(kinds::declaration::RECORD),
                    IrItem::Int(name),
                    fields,
                    bases,
                    IrItem::Int(i64::from(*is_abstract)),
                    update_target,
                ])
            }
            CodegenDeclarationKind::Component(component) => self.component(name, component),
            CodegenDeclarationKind::Union {
                cases,
                bases,
                property_target,
            } => {
                let cases = cases
                    .iter()
                    .map(|case| {
                        self.frame = Frame::new();
                        let case_name = self.string(&case.name);
                        let fields = self.record_fields(&case.fields);
                        IrItem::list([
                            IrItem::Int(case_name),
                            fields,
                            IrItem::Int(i64::from(case.is_constant)),
                        ])
                    })
                    .collect::<Vec<_>>();
                let bases = self.references(bases);
                let property_target = self.optional_reference(property_target.as_ref());
                IrItem::list([
                    IrItem::Int(kinds::declaration::UNION),
                    IrItem::Int(name),
                    IrItem::List(cases),
                    bases,
                    property_target,
                ])
            }
            CodegenDeclarationKind::TypeAlias | CodegenDeclarationKind::Unsupported(_) => {
                IrItem::ints([kinds::declaration::TYPE_ALIAS, name])
            }
        };
        self.declarations.push(entry);
        self.span_module = self.module.id;
        let span = self.span(declaration.span);
        self.declaration_spans.push(span);
    }

    fn component(&mut self, name: i64, component: &CodegenComponent) -> IrItem {
        let first = self
            .frame
            .reserve(component.props.len() + component.state.len());
        let props = self.component_fields(&component.props, first);
        let state = self.component_fields(&component.state, first + component.props.len() as i64);
        self.span_module = self.module.id;
        let body = match &component.body {
            Some(body) => self.expression(body),
            None => -1,
        };
        let flags = i64::from(component.is_abstract) * kinds::component::ABSTRACT
            + i64::from(component.is_external) * kinds::component::EXTERNAL;
        IrItem::list([
            IrItem::Int(kinds::declaration::COMPONENT),
            IrItem::Int(name),
            props,
            state,
            IrItem::Int(body),
            IrItem::Int(flags),
        ])
    }

    /// Emits a component's props or state, whose slots start at `first`. Each field's default
    /// sees the fields before it; an inherited default was written in its owner's text.
    fn component_fields(&mut self, fields: &[CodegenComponentField], first: i64) -> IrItem {
        let items = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                self.span_module = field.owner_module_id;
                let default = match &field.default {
                    Some(default) => self.expression(default),
                    None => -1,
                };
                self.frame.bind(&field.name, first + index as i64);
                let name = self.string(&field.name);
                let ty = self.type_ref(&field.resolved_ty);
                let flags = i64::from(field.is_content) * kinds::field::CONTENT
                    + i64::from(field.is_required) * kinds::field::REQUIRED;
                IrItem::ints([name, ty, default, flags])
            })
            .collect::<Vec<_>>();
        IrItem::List(items)
    }

    /// Emits a record's or union case's fields into the current frame, each default seeing the
    /// fields before it. An inherited default was written in its owner's text, so its spans only
    /// mean anything in that module.
    fn record_fields(&mut self, fields: &[CodegenRecordField]) -> IrItem {
        let first = self.frame.reserve(fields.len());
        let items = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                self.span_module = field.owner_module_id;
                let default = match &field.default {
                    Some(default) => self.expression(default),
                    None => -1,
                };
                self.frame.bind(&field.name, first + index as i64);
                let name = self.string(&field.name);
                let ty = self.type_ref(&field.resolved_ty);
                let flags = i64::from(field.is_content) * kinds::field::CONTENT
                    + i64::from(field.is_required) * kinds::field::REQUIRED;
                IrItem::ints([name, ty, default, flags])
            })
            .collect::<Vec<_>>();
        self.span_module = self.module.id;
        IrItem::List(items)
    }

    fn expressions(&mut self, expressions: &[CodegenExpression]) -> IrItem {
        let items = expressions
            .iter()
            .map(|expression| self.expression(expression))
            .collect::<Vec<_>>();
        IrItem::ints(items)
    }

    /// Emits properties sorted by name, each as `[name, value]`.
    fn properties(&mut self, properties: &[CodegenProperty]) -> IrItem {
        let mut sorted = properties.iter().collect::<Vec<_>>();
        sorted.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let items = sorted
            .into_iter()
            .map(|property| {
                let value = self.expression(&property.value);
                let name = self.string(&property.name);
                IrItem::ints([name, value])
            })
            .collect::<Vec<_>>();
        IrItem::List(items)
    }

    fn unresolved(&mut self, name: &str, span: TextSpan) {
        self.diagnostics.push(
            Diagnostic::error("nx-ir-unresolved-name")
                .with_message(format!(
                    "'{name}' is neither a local of this declaration nor a top-level declaration, so no NX IR node can reach it"
                ))
                .with_label(Label::primary(module_identity(self.module), span))
                .build(),
        );
    }

    fn expression(&mut self, expression: &CodegenExpression) -> i64 {
        let span = expression.span;
        let entry = match &expression.kind {
            CodegenExpressionKind::Literal(literal) => self.literal(literal),
            CodegenExpressionKind::Identifier { name, reference } => match reference {
                Some(reference) => {
                    let (slot, name) = self.reference(reference);
                    IrItem::ints([kinds::node::REFERENCE, slot, name])
                }
                None => match self.frame.resolve(name) {
                    Some(slot) => {
                        let name = self.string(name);
                        IrItem::ints([kinds::node::SLOT, slot, name])
                    }
                    None => {
                        self.unresolved(name, span);
                        IrItem::ints([kinds::node::NULL])
                    }
                },
            },
            CodegenExpressionKind::Binary { lhs, op, rhs } => {
                let lhs = self.expression(lhs);
                let rhs = self.expression(rhs);
                let op = binary_operator(*op, expression.ty.as_ref());
                IrItem::ints([kinds::node::BINARY, op, lhs, rhs])
            }
            CodegenExpressionKind::Unary { op, expr } => {
                let operand = self.expression(expr);
                let op = match op {
                    UnOp::Neg => kinds::unary::NEG,
                    UnOp::Not => kinds::unary::NOT,
                };
                IrItem::ints([kinds::node::UNARY, op, operand])
            }
            CodegenExpressionKind::Call { callee, args } => {
                let callee = self.expression(callee);
                let args = self.expressions(args);
                IrItem::list([IrItem::Int(kinds::node::CALL), IrItem::Int(callee), args])
            }
            CodegenExpressionKind::IntrinsicCall {
                intrinsic,
                args,
                field_order,
            } => {
                let args = self.expressions(args);
                let op = match intrinsic {
                    UpdateIntrinsic::Apply => kinds::intrinsic::APPLY,
                    UpdateIntrinsic::Merge => kinds::intrinsic::MERGE,
                    UpdateIntrinsic::Diff => kinds::intrinsic::DIFF,
                    UpdateIntrinsic::Changed => kinds::intrinsic::CHANGED,
                };
                let order = field_order
                    .iter()
                    .flatten()
                    .map(|name| self.string(name))
                    .collect::<Vec<_>>();
                IrItem::list([
                    IrItem::Int(kinds::node::INTRINSIC),
                    IrItem::Int(op),
                    args,
                    IrItem::ints(order),
                ])
            }
            CodegenExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.expression(condition);
                let then_branch = self.expression(then_branch);
                let else_branch = match else_branch {
                    Some(else_branch) => self.expression(else_branch),
                    None => -1,
                };
                IrItem::ints([kinds::node::IF, condition, then_branch, else_branch])
            }
            CodegenExpressionKind::Match {
                scrutinee,
                arms,
                else_branch,
            } => {
                let scrutinee = self.expression(scrutinee);
                let arms = arms
                    .iter()
                    .map(|arm| {
                        let patterns = self.expressions(&arm.patterns);
                        let body = self.expression(&arm.body);
                        IrItem::list([patterns, IrItem::Int(body)])
                    })
                    .collect::<Vec<_>>();
                let else_branch = match else_branch {
                    Some(else_branch) => self.expression(else_branch),
                    None => -1,
                };
                IrItem::list([
                    IrItem::Int(kinds::node::IF_IS),
                    IrItem::Int(scrutinee),
                    IrItem::List(arms),
                    IrItem::Int(else_branch),
                ])
            }
            CodegenExpressionKind::Let { name, .. } => {
                self.diagnostics.push(
                    Diagnostic::error("nx-ir-unsupported-construct")
                        .with_message(format!(
                            "a let binding of '{name}' has no NX IR node; NX source cannot write one"
                        ))
                        .with_label(Label::primary(module_identity(self.module), span))
                        .build(),
                );
                IrItem::ints([kinds::node::NULL])
            }
            CodegenExpressionKind::Block {
                statements,
                expression: result,
            } => {
                // A block is its result: NX source writes no statements, so a block with any is
                // a construct the IR has no node for.
                if !statements.is_empty() {
                    self.diagnostics.push(
                        Diagnostic::error("nx-ir-unsupported-construct")
                            .with_message(
                                "a block with statements has no NX IR node; NX source cannot write one",
                            )
                            .with_label(Label::primary(module_identity(self.module), span))
                            .build(),
                    );
                }
                return match result {
                    Some(result) => self.expression(result),
                    None => self.push_node(IrItem::ints([kinds::node::NULL]), span),
                };
            }
            CodegenExpressionKind::Array(elements) => {
                let elements = self.expressions(elements);
                IrItem::list([IrItem::Int(kinds::node::ARRAY), elements])
            }
            CodegenExpressionKind::For {
                item,
                index,
                iterable,
                body,
            } => {
                let iterable = self.expression(iterable);
                self.frame.push();
                let item_slot = self.frame.alloc(item);
                let item_name = self.string(item);
                let (index_slot, index_name) = match index {
                    Some(index) => (self.frame.alloc(index), self.string(index)),
                    None => (-1, -1),
                };
                let body = self.expression(body);
                self.frame.pop();
                IrItem::ints([
                    kinds::node::FOR,
                    item_slot,
                    item_name,
                    index_slot,
                    index_name,
                    iterable,
                    body,
                ])
            }
            // NX has no syntax for an index expression, so nothing reaches here.
            CodegenExpressionKind::Index { .. } => {
                self.diagnostics.push(
                    Diagnostic::error("nx-ir-unsupported-construct")
                        .with_message(
                            "an index expression has no NX IR node; NX source cannot write one",
                        )
                        .with_label(Label::primary(module_identity(self.module), span))
                        .build(),
                );
                IrItem::ints([kinds::node::NULL])
            }
            CodegenExpressionKind::Member {
                base,
                member,
                reference,
            } => {
                // A member chain that analysis resolved to a declaration is that declaration.
                if let Some(reference) = reference {
                    let (slot, name) = self.reference(reference);
                    IrItem::ints([kinds::node::REFERENCE, slot, name])
                } else {
                    let base = self.expression(base);
                    let member = self.string(member);
                    IrItem::ints([kinds::node::MEMBER, base, member])
                }
            }
            CodegenExpressionKind::Record {
                name,
                reference,
                properties,
                content,
                ..
            } => {
                let Some(reference) = reference else {
                    self.diagnostics.push(
                        Diagnostic::error("nx-ir-unresolved-name")
                            .with_message(format!(
                                "record construction of '{name}' does not reach a record declaration"
                            ))
                            .with_label(Label::primary(module_identity(self.module), span))
                            .build(),
                    );
                    return self.push_node(IrItem::ints([kinds::node::NULL]), span);
                };
                let properties = self.properties(properties);
                let content = self.expressions(content);
                let (slot, name) = self.reference(reference);
                IrItem::list([
                    IrItem::Int(kinds::node::RECORD),
                    IrItem::Int(slot),
                    IrItem::Int(name),
                    properties,
                    content,
                ])
            }
            CodegenExpressionKind::UnionCase {
                union_reference,
                case_name,
                properties,
                content,
                ..
            } => {
                let properties = self.properties(properties);
                let content = self.expressions(content);
                let (slot, union) = self.reference(union_reference);
                let case = self.string(case_name);
                IrItem::list([
                    IrItem::Int(kinds::node::UNION_CASE),
                    IrItem::Int(slot),
                    IrItem::Int(union),
                    IrItem::Int(case),
                    properties,
                    content,
                ])
            }
            CodegenExpressionKind::ComponentDescriptor(descriptor) => {
                let properties = self.properties(&descriptor.properties);
                let content = self.expressions(&descriptor.content);
                let (slot, name) = self.reference(&descriptor.component);
                IrItem::list([
                    IrItem::Int(kinds::node::COMPONENT),
                    IrItem::Int(slot),
                    IrItem::Int(name),
                    properties,
                    content,
                ])
            }
            CodegenExpressionKind::Element(element) => {
                let properties = self.properties(&element.properties);
                let content = self.expressions(&element.content);
                let tag = self.string(&element.tag);
                IrItem::list([
                    IrItem::Int(kinds::node::ELEMENT),
                    IrItem::Int(i64::from(element.element_id)),
                    IrItem::Int(tag),
                    properties,
                    content,
                ])
            }
            // Refused by `validate_ir_program` before emission starts.
            CodegenExpressionKind::Unsupported(_) => IrItem::ints([kinds::node::NULL]),
        };
        self.push_node(entry, span)
    }

    fn literal(&mut self, literal: &Literal) -> IrItem {
        match literal {
            Literal::String(value) => {
                let value = self.string(value);
                IrItem::ints([kinds::node::STRING, value])
            }
            Literal::Int(value) => {
                let constant = if is_js_safe_integer(*value) {
                    IrItem::ints([kinds::constant::INT, *value])
                } else {
                    let digits = self.string(&value.to_string());
                    IrItem::ints([kinds::constant::BIGINT, digits])
                };
                let constant = self.constant(constant);
                IrItem::ints([kinds::node::NUMBER, constant])
            }
            Literal::Float(value) => {
                let constant = self.constant(IrItem::list([
                    IrItem::Int(kinds::constant::FLOAT),
                    IrItem::Float(value.0),
                ]));
                IrItem::ints([kinds::node::NUMBER, constant])
            }
            Literal::Boolean(value) => IrItem::ints([kinds::node::BOOL, i64::from(*value)]),
            Literal::Null => IrItem::ints([kinds::node::NULL]),
        }
    }
}

/// The operator code of a binary expression. Division and remainder are integer operations when
/// the expression's own type is an integer type, which is what the interpreter does too.
fn binary_operator(op: BinOp, ty: Option<&Type>) -> i64 {
    let integer = matches!(
        ty,
        Some(Type::Primitive(
            Primitive::Int | Primitive::Int32 | Primitive::Int64
        ))
    );
    match op {
        BinOp::Add => kinds::binary::ADD,
        BinOp::Sub => kinds::binary::SUB,
        BinOp::Mul => kinds::binary::MUL,
        BinOp::Div if integer => kinds::binary::IDIV,
        BinOp::Div => kinds::binary::DIV,
        BinOp::Mod if integer => kinds::binary::IMOD,
        BinOp::Mod => kinds::binary::MOD,
        BinOp::Eq => kinds::binary::EQ,
        BinOp::Ne => kinds::binary::NE,
        BinOp::Lt => kinds::binary::LT,
        BinOp::Le => kinds::binary::LE,
        BinOp::Gt => kinds::binary::GT,
        BinOp::Ge => kinds::binary::GE,
        BinOp::And => kinds::binary::AND,
        BinOp::Or => kinds::binary::OR,
        BinOp::Concat => kinds::binary::CONCAT,
    }
}

fn is_js_safe_integer(value: i64) -> bool {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value)
}
