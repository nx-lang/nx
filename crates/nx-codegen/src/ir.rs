use crate::builder::build_codegen_program;
use crate::model::{
    CodegenComponent, CodegenComponentDescriptor, CodegenComponentField,
    CodegenComponentTargetKind, CodegenDeclaration, CodegenDeclarationKind, CodegenElement,
    CodegenEntrypoint, CodegenExpression, CodegenExpressionKind, CodegenMatchArm, CodegenModule,
    CodegenModuleProvenance, CodegenParam, CodegenProgram, CodegenProperty, CodegenRecordField,
    CodegenReference, CodegenSourceEntry, CodegenStatement, CodegenTypeRef, CodegenUnionCase,
};
use crate::options::CodegenError;
use nx_api::ProgramArtifact;
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use nx_hir::ast::{BinOp, Literal, UnOp};
use nx_interpreter::{ResolvedItemKind, RuntimeModuleId};
use nx_types::{Primitive, Type};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NX_IR_FORMAT_ID: &str = "nx-ir-json";
pub const NX_IR_SCHEMA_VERSION: u32 = 1;
pub const NX_IR_RUNTIME_ABI: &str = "nx-ir-runtime-v1";
pub const NX_IR_REQUIRED_FEATURE_EAGER_V1: &str = "eager-v1";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedNxIr {
    pub json: String,
    pub metadata: NxIrMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrMetadata {
    pub program_fingerprint: u64,
    pub schema_version: u32,
    pub runtime_abi: String,
    pub required_features: Vec<String>,
    pub function_entrypoints: Vec<NxIrEntrypointMetadata>,
    pub component_entrypoints: Vec<NxIrEntrypointMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrEntrypointMetadata {
    pub name: String,
    pub reference: NxIrReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrProgram {
    pub format: String,
    pub schema_version: u32,
    pub runtime_abi: String,
    #[serde(with = "u64_decimal_string")]
    pub program_fingerprint: u64,
    pub required_features: Vec<String>,
    pub function_entrypoints: Vec<NxIrEntrypoint>,
    pub component_entrypoints: Vec<NxIrEntrypoint>,
    pub modules: Vec<NxIrModule>,
    pub sources: Vec<NxIrSourceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrEntrypoint {
    pub name: String,
    pub reference: NxIrReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrModule {
    pub id: String,
    pub runtime_id: u32,
    pub provenance: NxIrModuleProvenance,
    pub imports: Vec<NxIrReference>,
    pub declarations: Vec<NxIrDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NxIrModuleProvenance {
    SourceProvider {
        identity: String,
    },
    Library {
        root_path: String,
        module_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrReference {
    pub module: String,
    pub declaration: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrDeclaration {
    pub id: String,
    pub reference: NxIrReference,
    pub span: NxIrSourceSpan,
    pub kind: NxIrDeclarationKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NxIrDeclarationKind {
    Function {
        params: Vec<NxIrParam>,
        body: NxIrExpression,
        return_type: Option<NxIrSemanticType>,
    },
    Value {
        value: NxIrExpression,
        ty: Option<NxIrSemanticType>,
    },
    Enum {
        members: Vec<String>,
    },
    Record {
        fields: Vec<NxIrRecordField>,
    },
    Component(NxIrComponent),
    Union {
        cases: Vec<NxIrUnionCase>,
    },
    TypeAlias,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrParam {
    pub name: String,
    pub slot: String,
    pub ty: NxIrTypeRef,
    pub is_content: bool,
    pub span: NxIrSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrRecordField {
    pub name: String,
    pub slot: String,
    pub ty: NxIrTypeRef,
    pub is_content: bool,
    pub is_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<NxIrExpression>,
    pub span: NxIrSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrComponent {
    pub is_abstract: bool,
    pub is_external: bool,
    pub props: Vec<NxIrComponentField>,
    pub state: Vec<NxIrComponentField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<NxIrExpression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrComponentField {
    pub name: String,
    pub slot: String,
    pub owner_module: String,
    pub ty: NxIrTypeRef,
    pub is_content: bool,
    pub is_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<NxIrExpression>,
    pub span: NxIrSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrUnionCase {
    pub name: String,
    pub fields: Vec<NxIrRecordField>,
    pub span: NxIrSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrExpression {
    pub id: String,
    pub span: NxIrSourceSpan,
    pub ty: Option<NxIrSemanticType>,
    pub op: NxIrExpressionOp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NxIrExpressionOp {
    Literal {
        value: NxIrLiteral,
    },
    Slot {
        slot: String,
        name: String,
    },
    Reference {
        reference: NxIrReference,
    },
    Binary {
        lhs: Box<NxIrExpression>,
        operator: String,
        rhs: Box<NxIrExpression>,
    },
    Unary {
        operator: String,
        expr: Box<NxIrExpression>,
    },
    Call {
        callee: Box<NxIrExpression>,
        args: Vec<NxIrExpression>,
    },
    If {
        condition: Box<NxIrExpression>,
        then_branch: Box<NxIrExpression>,
        else_branch: Option<Box<NxIrExpression>>,
    },
    IfIs {
        scrutinee: Box<NxIrExpression>,
        arms: Vec<NxIrMatchArm>,
        else_branch: Option<Box<NxIrExpression>>,
    },
    Let {
        name: String,
        slot: String,
        value: Box<NxIrExpression>,
        body: Box<NxIrExpression>,
    },
    Block {
        statements: Vec<NxIrStatement>,
        expression: Option<Box<NxIrExpression>>,
    },
    Array {
        elements: Vec<NxIrExpression>,
    },
    For {
        item: String,
        item_slot: String,
        index: Option<String>,
        index_slot: Option<String>,
        iterable: Box<NxIrExpression>,
        body: Box<NxIrExpression>,
    },
    Index {
        base: Box<NxIrExpression>,
        index: Box<NxIrExpression>,
    },
    Member {
        base: Box<NxIrExpression>,
        member: String,
        reference: Option<NxIrReference>,
    },
    Record {
        name: String,
        fields: Vec<NxIrRecordField>,
        properties: Vec<NxIrProperty>,
        content_field: Option<String>,
        content: Vec<NxIrExpression>,
    },
    UnionCase {
        union: NxIrReference,
        case_name: String,
        fields: Vec<NxIrRecordField>,
        properties: Vec<NxIrProperty>,
        content_field: Option<String>,
        content: Vec<NxIrExpression>,
    },
    EnumMember {
        enumeration: NxIrReference,
        member: String,
    },
    IntrinsicElement {
        element_id: String,
        tag_name: String,
        properties: Vec<NxIrProperty>,
        content: Vec<NxIrExpression>,
    },
    ComponentDescriptor {
        component: NxIrReference,
        target_kind: String,
        properties: Vec<NxIrProperty>,
        content_field: Option<String>,
        content: Vec<NxIrExpression>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrMatchArm {
    pub patterns: Vec<NxIrExpression>,
    pub body: NxIrExpression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NxIrStatement {
    Let {
        name: String,
        slot: String,
        init: NxIrExpression,
        span: NxIrSourceSpan,
    },
    Expr {
        expr: NxIrExpression,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrProperty {
    pub name: String,
    pub value: NxIrExpression,
    pub span: NxIrSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NxIrLiteral {
    String { value: String },
    Int { value: String, number: Option<i64> },
    Float { value: f64 },
    Boolean { value: bool },
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NxIrTypeRef {
    Primitive {
        name: String,
    },
    Nominal {
        reference: NxIrReference,
        display: String,
    },
    Array {
        element: Box<NxIrTypeRef>,
    },
    Nullable {
        inner: Box<NxIrTypeRef>,
    },
    Function {
        params: Vec<NxIrTypeRef>,
        return_type: Box<NxIrTypeRef>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrSemanticType {
    pub display: String,
    pub shape: NxIrSemanticTypeShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NxIrSemanticTypeShape {
    Primitive {
        name: String,
    },
    Array {
        element: Box<NxIrSemanticType>,
    },
    Nullable {
        inner: Box<NxIrSemanticType>,
    },
    Function {
        params: Vec<NxIrSemanticType>,
        return_type: Box<NxIrSemanticType>,
    },
    Named {
        name: String,
    },
    Enum {
        name: String,
        members: Vec<String>,
    },
    Union {
        name: String,
        cases: Vec<String>,
        base: Option<String>,
    },
    UnionCase {
        union: String,
        case_name: String,
    },
    Variable {
        id: u32,
    },
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrSourceSpan {
    pub source: Option<String>,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrSourceEntry {
    pub identity: String,
    pub source: String,
}

pub fn emit_nx_ir(artifact: &ProgramArtifact) -> Result<GeneratedNxIr, CodegenError> {
    let program = build_codegen_program(artifact)?;
    emit_codegen_nx_ir(&program)
}

pub fn emit_codegen_nx_ir(program: &CodegenProgram) -> Result<GeneratedNxIr, CodegenError> {
    validate_ir_program(program)?;

    let ir = NxIrProgram::from_codegen(program);
    let json = serde_json::to_string_pretty(&ir)
        .map(|json| format!("{json}\n"))
        .map_err(|error| {
            CodegenError::single(
                Diagnostic::error("nx-ir-serialization-error")
                    .with_message(format!("failed to serialize NX IR JSON: {error}"))
                    .build(),
            )
        })?;
    let metadata = ir.metadata();

    Ok(GeneratedNxIr { json, metadata })
}

impl NxIrProgram {
    pub fn from_codegen(program: &CodegenProgram) -> Self {
        let mut context = ProgramIrContext::new(program);
        let function_entrypoints = program
            .entrypoints
            .iter()
            .map(ir_entrypoint)
            .collect::<Vec<_>>();
        let component_entrypoints = program
            .component_entrypoints
            .iter()
            .map(ir_entrypoint)
            .collect::<Vec<_>>();
        let modules = program
            .modules
            .iter()
            .map(|module| ir_module(module, &mut context))
            .collect::<Vec<_>>();
        let sources = program
            .source_entries
            .iter()
            .map(ir_source_entry)
            .collect::<Vec<_>>();

        Self {
            format: NX_IR_FORMAT_ID.to_string(),
            schema_version: NX_IR_SCHEMA_VERSION,
            runtime_abi: NX_IR_RUNTIME_ABI.to_string(),
            program_fingerprint: program.fingerprint,
            required_features: vec![NX_IR_REQUIRED_FEATURE_EAGER_V1.to_string()],
            function_entrypoints,
            component_entrypoints,
            modules,
            sources,
        }
    }

    pub fn metadata(&self) -> NxIrMetadata {
        NxIrMetadata {
            program_fingerprint: self.program_fingerprint,
            schema_version: self.schema_version,
            runtime_abi: self.runtime_abi.clone(),
            required_features: self.required_features.clone(),
            function_entrypoints: self
                .function_entrypoints
                .iter()
                .map(|entrypoint| NxIrEntrypointMetadata {
                    name: entrypoint.name.clone(),
                    reference: entrypoint.reference.clone(),
                })
                .collect(),
            component_entrypoints: self
                .component_entrypoints
                .iter()
                .map(|entrypoint| NxIrEntrypointMetadata {
                    name: entrypoint.name.clone(),
                    reference: entrypoint.reference.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct ProgramIrContext {
    module_sources: BTreeMap<u32, Option<String>>,
}

impl ProgramIrContext {
    fn new(program: &CodegenProgram) -> Self {
        let module_sources = program
            .modules
            .iter()
            .map(|module| (module.id.as_u32(), module_source_identity(module)))
            .collect();
        Self { module_sources }
    }

    fn source_for(&self, module_id: RuntimeModuleId) -> Option<String> {
        self.module_sources
            .get(&module_id.as_u32())
            .and_then(|source| source.clone())
    }
}

#[derive(Debug, Clone)]
struct SlotScope {
    frames: Vec<BTreeMap<String, String>>,
}

impl SlotScope {
    fn new() -> Self {
        Self {
            frames: vec![BTreeMap::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(BTreeMap::new());
    }

    fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    fn insert(&mut self, name: &str, slot: impl Into<String>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(name.to_string(), slot.into());
        }
    }

    fn resolve(&self, name: &str) -> Option<&str> {
        self.frames
            .iter()
            .rev()
            .find_map(|frame| frame.get(name).map(String::as_str))
    }
}

fn validate_ir_program(program: &CodegenProgram) -> Result<(), CodegenError> {
    let mut diagnostics = Vec::new();
    for module in &program.modules {
        for declaration in &module.declarations {
            collect_ir_unsupported_diagnostics(module, declaration, &mut diagnostics);
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(CodegenError::new(diagnostics))
    }
}

fn collect_ir_unsupported_diagnostics(
    module: &CodegenModule,
    declaration: &CodegenDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &declaration.kind {
        CodegenDeclarationKind::Unsupported(unsupported) => {
            diagnostics.push(ir_unsupported_diagnostic(
                module,
                unsupported.span,
                &unsupported.message,
            ));
        }
        CodegenDeclarationKind::Function { body, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, body, diagnostics);
        }
        CodegenDeclarationKind::Value { value, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, value, diagnostics);
        }
        CodegenDeclarationKind::Record { fields } => {
            collect_ir_record_field_unsupported_diagnostics(module, fields, diagnostics);
        }
        CodegenDeclarationKind::Component(component) => {
            collect_ir_component_field_unsupported_diagnostics(
                module,
                &component.props,
                diagnostics,
            );
            collect_ir_component_field_unsupported_diagnostics(
                module,
                &component.state,
                diagnostics,
            );
            if let Some(body) = component.body.as_ref() {
                collect_ir_expression_unsupported_diagnostics(module, body, diagnostics);
            }
        }
        CodegenDeclarationKind::Union { cases } => {
            for case in cases {
                collect_ir_record_field_unsupported_diagnostics(module, &case.fields, diagnostics);
            }
        }
        CodegenDeclarationKind::Enum { .. } | CodegenDeclarationKind::TypeAlias => {}
    }
}

fn collect_ir_component_field_unsupported_diagnostics(
    module: &CodegenModule,
    fields: &[CodegenComponentField],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if let Some(default) = field.default.as_ref() {
            collect_ir_expression_unsupported_diagnostics(module, default, diagnostics);
        }
    }
}

fn collect_ir_record_field_unsupported_diagnostics(
    module: &CodegenModule,
    fields: &[CodegenRecordField],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if let Some(default) = field.default.as_ref() {
            collect_ir_expression_unsupported_diagnostics(module, default, diagnostics);
        }
    }
}

fn collect_ir_expression_unsupported_diagnostics(
    module: &CodegenModule,
    expression: &CodegenExpression,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expression.kind {
        CodegenExpressionKind::Unsupported(unsupported) => {
            diagnostics.push(ir_unsupported_diagnostic(
                module,
                unsupported.span,
                &unsupported.message,
            ));
        }
        CodegenExpressionKind::Binary { lhs, rhs, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, lhs, diagnostics);
            collect_ir_expression_unsupported_diagnostics(module, rhs, diagnostics);
        }
        CodegenExpressionKind::Unary { expr, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, expr, diagnostics);
        }
        CodegenExpressionKind::Call { callee, args } => {
            collect_ir_expression_unsupported_diagnostics(module, callee, diagnostics);
            for arg in args {
                collect_ir_expression_unsupported_diagnostics(module, arg, diagnostics);
            }
        }
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_ir_expression_unsupported_diagnostics(module, condition, diagnostics);
            collect_ir_expression_unsupported_diagnostics(module, then_branch, diagnostics);
            if let Some(else_branch) = else_branch {
                collect_ir_expression_unsupported_diagnostics(module, else_branch, diagnostics);
            }
        }
        CodegenExpressionKind::Match {
            scrutinee,
            arms,
            else_branch,
        } => {
            collect_ir_expression_unsupported_diagnostics(module, scrutinee, diagnostics);
            for arm in arms {
                for pattern in &arm.patterns {
                    collect_ir_expression_unsupported_diagnostics(module, pattern, diagnostics);
                }
                collect_ir_expression_unsupported_diagnostics(module, &arm.body, diagnostics);
            }
            if let Some(else_branch) = else_branch {
                collect_ir_expression_unsupported_diagnostics(module, else_branch, diagnostics);
            }
        }
        CodegenExpressionKind::Let { value, body, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, value, diagnostics);
            collect_ir_expression_unsupported_diagnostics(module, body, diagnostics);
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            for statement in statements {
                match statement {
                    CodegenStatement::Let { init, .. } => {
                        collect_ir_expression_unsupported_diagnostics(module, init, diagnostics);
                    }
                    CodegenStatement::Expr(expr) => {
                        collect_ir_expression_unsupported_diagnostics(module, expr, diagnostics);
                    }
                }
            }
            if let Some(expression) = expression {
                collect_ir_expression_unsupported_diagnostics(module, expression, diagnostics);
            }
        }
        CodegenExpressionKind::Array(elements) => {
            for element in elements {
                collect_ir_expression_unsupported_diagnostics(module, element, diagnostics);
            }
        }
        CodegenExpressionKind::For { iterable, body, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, iterable, diagnostics);
            collect_ir_expression_unsupported_diagnostics(module, body, diagnostics);
        }
        CodegenExpressionKind::Index { base, index } => {
            collect_ir_expression_unsupported_diagnostics(module, base, diagnostics);
            collect_ir_expression_unsupported_diagnostics(module, index, diagnostics);
        }
        CodegenExpressionKind::Member { base, .. } => {
            collect_ir_expression_unsupported_diagnostics(module, base, diagnostics);
        }
        CodegenExpressionKind::Record {
            fields,
            properties,
            content,
            ..
        } => {
            collect_ir_record_field_unsupported_diagnostics(module, fields, diagnostics);
            for property in properties {
                collect_ir_expression_unsupported_diagnostics(module, &property.value, diagnostics);
            }
            for item in content {
                collect_ir_expression_unsupported_diagnostics(module, item, diagnostics);
            }
        }
        CodegenExpressionKind::UnionCase {
            fields,
            properties,
            content,
            ..
        } => {
            collect_ir_record_field_unsupported_diagnostics(module, fields, diagnostics);
            for property in properties {
                collect_ir_expression_unsupported_diagnostics(module, &property.value, diagnostics);
            }
            for content in content {
                collect_ir_expression_unsupported_diagnostics(module, content, diagnostics);
            }
        }
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            for property in &descriptor.properties {
                collect_ir_expression_unsupported_diagnostics(module, &property.value, diagnostics);
            }
            for content in &descriptor.content {
                collect_ir_expression_unsupported_diagnostics(module, content, diagnostics);
            }
        }
        CodegenExpressionKind::Element(element) => {
            for property in &element.properties {
                collect_ir_expression_unsupported_diagnostics(module, &property.value, diagnostics);
            }
            for content in &element.content {
                collect_ir_expression_unsupported_diagnostics(module, content, diagnostics);
            }
        }
        CodegenExpressionKind::Literal(_)
        | CodegenExpressionKind::Identifier { .. }
        | CodegenExpressionKind::EnumMember { .. } => {}
    }
}

fn ir_module(module: &CodegenModule, context: &mut ProgramIrContext) -> NxIrModule {
    NxIrModule {
        id: module_id(module.id),
        runtime_id: module.id.as_u32(),
        provenance: ir_module_provenance(&module.provenance),
        imports: module.imports.iter().map(ir_reference).collect(),
        declarations: module
            .declarations
            .iter()
            .map(|declaration| ir_declaration(module.id, declaration, context))
            .collect(),
    }
}

fn ir_declaration(
    module_id_value: RuntimeModuleId,
    declaration: &CodegenDeclaration,
    context: &ProgramIrContext,
) -> NxIrDeclaration {
    let declaration_id = declaration_id(&declaration.reference);
    let source = context.source_for(module_id_value);
    let kind = match &declaration.kind {
        CodegenDeclarationKind::Function {
            params,
            body,
            return_type,
        } => {
            let mut scope = SlotScope::new();
            let params = params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    let param = ir_param(
                        module_id_value,
                        &declaration_id,
                        index,
                        param,
                        source.clone(),
                    );
                    scope.insert(&param.name, param.slot.clone());
                    param
                })
                .collect::<Vec<_>>();
            let body = ir_expression(
                module_id_value,
                source.clone(),
                body,
                &mut scope,
                &format!("{declaration_id}:body"),
            );
            NxIrDeclarationKind::Function {
                params,
                body,
                return_type: return_type.as_ref().map(ir_semantic_type),
            }
        }
        CodegenDeclarationKind::Value { value, ty } => {
            let mut scope = SlotScope::new();
            NxIrDeclarationKind::Value {
                value: ir_expression(
                    module_id_value,
                    source.clone(),
                    value,
                    &mut scope,
                    &format!("{declaration_id}:value"),
                ),
                ty: ty.as_ref().map(ir_semantic_type),
            }
        }
        CodegenDeclarationKind::Enum { members } => NxIrDeclarationKind::Enum {
            members: members.clone(),
        },
        CodegenDeclarationKind::Record { fields } => {
            let scope = SlotScope::new();
            NxIrDeclarationKind::Record {
                fields: ir_record_fields(
                    module_id_value,
                    source.clone(),
                    &declaration_id,
                    "field",
                    fields,
                    &scope,
                ),
            }
        }
        CodegenDeclarationKind::Component(component) => {
            NxIrDeclarationKind::Component(ir_component(
                module_id_value,
                source.clone(),
                &declaration_id,
                component,
                context,
            ))
        }
        CodegenDeclarationKind::Union { cases } => NxIrDeclarationKind::Union {
            cases: cases
                .iter()
                .enumerate()
                .map(|(index, case)| {
                    ir_union_case(
                        module_id_value,
                        source.clone(),
                        &declaration_id,
                        index,
                        case,
                    )
                })
                .collect(),
        },
        CodegenDeclarationKind::TypeAlias | CodegenDeclarationKind::Unsupported(_) => {
            NxIrDeclarationKind::TypeAlias
        }
    };

    NxIrDeclaration {
        id: declaration_id,
        reference: ir_reference(&declaration.reference),
        span: ir_span(source, declaration.span),
        kind,
    }
}

fn ir_component(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    declaration_id: &str,
    component: &CodegenComponent,
    context: &ProgramIrContext,
) -> NxIrComponent {
    let mut prop_scope = SlotScope::new();
    let props = ir_component_fields(
        declaration_id,
        "prop",
        &component.props,
        &mut prop_scope,
        context,
    );

    let mut state_scope = prop_scope.clone();
    let state = ir_component_fields(
        declaration_id,
        "state",
        &component.state,
        &mut state_scope,
        context,
    );

    let mut body_scope = SlotScope::new();
    for field in &props {
        body_scope.insert(&field.name, field.slot.clone());
    }
    for field in &state {
        body_scope.insert(&field.name, field.slot.clone());
    }
    let body = component.body.as_ref().map(|body| {
        ir_expression(
            module_id_value,
            source.clone(),
            body,
            &mut body_scope,
            &format!("{declaration_id}:body"),
        )
    });

    NxIrComponent {
        is_abstract: component.is_abstract,
        is_external: component.is_external,
        props,
        state,
        body,
    }
}

fn ir_param(
    module_id_value: RuntimeModuleId,
    declaration_id: &str,
    index: usize,
    param: &CodegenParam,
    source: Option<String>,
) -> NxIrParam {
    NxIrParam {
        name: param.name.clone(),
        slot: format!("{declaration_id}:param:{index}"),
        ty: ir_type_ref(&param.resolved_ty),
        is_content: param.is_content,
        span: ir_span_for_module(module_id_value, source, param.span),
    }
}

fn ir_component_fields(
    declaration_id: &str,
    slot_kind: &str,
    fields: &[CodegenComponentField],
    scope: &mut SlotScope,
    context: &ProgramIrContext,
) -> Vec<NxIrComponentField> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let slot = format!("{declaration_id}:{slot_kind}:{index}");
            let default_source = context.source_for(field.owner_module_id);
            let default = field.default.as_ref().map(|default| {
                ir_expression(
                    field.owner_module_id,
                    default_source.clone(),
                    default,
                    scope,
                    &format!("{slot}:default"),
                )
            });
            let ir_field = NxIrComponentField {
                name: field.name.clone(),
                slot: slot.clone(),
                owner_module: module_id(field.owner_module_id),
                ty: ir_type_ref(&field.resolved_ty),
                is_content: field.is_content,
                is_required: field.is_required,
                default,
                span: ir_span_for_module(field.owner_module_id, default_source.clone(), field.span),
            };
            scope.insert(&field.name, slot);
            ir_field
        })
        .collect()
}

fn ir_record_fields(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    owner_id: &str,
    slot_kind: &str,
    fields: &[CodegenRecordField],
    outer_scope: &SlotScope,
) -> Vec<NxIrRecordField> {
    let mut scope = outer_scope.clone();
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let slot = format!("{owner_id}:{slot_kind}:{index}");
            let default = field.default.as_ref().map(|default| {
                ir_expression(
                    module_id_value,
                    source.clone(),
                    default,
                    &mut scope,
                    &format!("{slot}:default"),
                )
            });
            let ir_field = NxIrRecordField {
                name: field.name.clone(),
                slot: slot.clone(),
                ty: ir_type_ref(&field.resolved_ty),
                is_content: field.is_content,
                is_required: field.is_required,
                default,
                span: ir_span(source.clone(), field.span),
            };
            scope.insert(&field.name, slot);
            ir_field
        })
        .collect()
}

fn ir_union_case(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    declaration_id: &str,
    index: usize,
    case: &CodegenUnionCase,
) -> NxIrUnionCase {
    let case_id = format!("{declaration_id}:case:{index}");
    NxIrUnionCase {
        name: case.name.clone(),
        fields: ir_record_fields(
            module_id_value,
            source.clone(),
            &case_id,
            "field",
            &case.fields,
            &SlotScope::new(),
        ),
        span: ir_span(source, case.span),
    }
}

fn ir_expression(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    expression: &CodegenExpression,
    scope: &mut SlotScope,
    path: &str,
) -> NxIrExpression {
    let id = expression_id(module_id_value, expression.expr_id, path);
    let op = match &expression.kind {
        CodegenExpressionKind::Literal(literal) => NxIrExpressionOp::Literal {
            value: ir_literal(literal),
        },
        CodegenExpressionKind::Identifier { name, reference } => {
            if let Some(reference) = reference {
                NxIrExpressionOp::Reference {
                    reference: ir_reference(reference),
                }
            } else if let Some(slot) = scope.resolve(name) {
                NxIrExpressionOp::Slot {
                    slot: slot.to_string(),
                    name: name.clone(),
                }
            } else {
                NxIrExpressionOp::Slot {
                    slot: format!("unresolved:{name}"),
                    name: name.clone(),
                }
            }
        }
        CodegenExpressionKind::Binary { lhs, op, rhs } => NxIrExpressionOp::Binary {
            lhs: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                lhs,
                scope,
                &format!("{path}:lhs"),
            )),
            operator: binop_name(*op).to_string(),
            rhs: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                rhs,
                scope,
                &format!("{path}:rhs"),
            )),
        },
        CodegenExpressionKind::Unary { op, expr } => NxIrExpressionOp::Unary {
            operator: unop_name(*op).to_string(),
            expr: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                expr,
                scope,
                &format!("{path}:expr"),
            )),
        },
        CodegenExpressionKind::Call { callee, args } => NxIrExpressionOp::Call {
            callee: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                callee,
                scope,
                &format!("{path}:callee"),
            )),
            args: args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    ir_expression(
                        module_id_value,
                        source.clone(),
                        arg,
                        scope,
                        &format!("{path}:arg:{index}"),
                    )
                })
                .collect(),
        },
        CodegenExpressionKind::If {
            condition,
            then_branch,
            else_branch,
        } => NxIrExpressionOp::If {
            condition: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                condition,
                scope,
                &format!("{path}:condition"),
            )),
            then_branch: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                then_branch,
                scope,
                &format!("{path}:then"),
            )),
            else_branch: else_branch.as_ref().map(|else_branch| {
                Box::new(ir_expression(
                    module_id_value,
                    source.clone(),
                    else_branch,
                    scope,
                    &format!("{path}:else"),
                ))
            }),
        },
        CodegenExpressionKind::Match {
            scrutinee,
            arms,
            else_branch,
        } => NxIrExpressionOp::IfIs {
            scrutinee: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                scrutinee,
                scope,
                &format!("{path}:scrutinee"),
            )),
            arms: arms
                .iter()
                .enumerate()
                .map(|(index, arm)| {
                    ir_match_arm(
                        module_id_value,
                        source.clone(),
                        arm,
                        scope,
                        &format!("{path}:arm:{index}"),
                    )
                })
                .collect(),
            else_branch: else_branch.as_ref().map(|else_branch| {
                Box::new(ir_expression(
                    module_id_value,
                    source.clone(),
                    else_branch,
                    scope,
                    &format!("{path}:else"),
                ))
            }),
        },
        CodegenExpressionKind::Let { name, value, body } => {
            let value = Box::new(ir_expression(
                module_id_value,
                source.clone(),
                value,
                scope,
                &format!("{path}:value"),
            ));
            let slot = format!("{id}:let:{name}");
            scope.push();
            scope.insert(name, slot.clone());
            let body = Box::new(ir_expression(
                module_id_value,
                source.clone(),
                body,
                scope,
                &format!("{path}:body"),
            ));
            scope.pop();
            NxIrExpressionOp::Let {
                name: name.clone(),
                slot,
                value,
                body,
            }
        }
        CodegenExpressionKind::Block {
            statements,
            expression,
        } => {
            scope.push();
            let statements = statements
                .iter()
                .enumerate()
                .map(|(index, statement)| {
                    ir_statement(
                        module_id_value,
                        source.clone(),
                        statement,
                        scope,
                        &format!("{path}:stmt:{index}"),
                    )
                })
                .collect();
            let expression = expression.as_ref().map(|expression| {
                Box::new(ir_expression(
                    module_id_value,
                    source.clone(),
                    expression,
                    scope,
                    &format!("{path}:result"),
                ))
            });
            scope.pop();
            NxIrExpressionOp::Block {
                statements,
                expression,
            }
        }
        CodegenExpressionKind::Array(elements) => NxIrExpressionOp::Array {
            elements: elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    ir_expression(
                        module_id_value,
                        source.clone(),
                        element,
                        scope,
                        &format!("{path}:element:{index}"),
                    )
                })
                .collect(),
        },
        CodegenExpressionKind::For {
            item,
            index,
            iterable,
            body,
        } => {
            let iterable = Box::new(ir_expression(
                module_id_value,
                source.clone(),
                iterable,
                scope,
                &format!("{path}:iterable"),
            ));
            let item_slot = format!("{id}:for:item");
            let index_slot = index.as_ref().map(|_| format!("{id}:for:index"));
            scope.push();
            scope.insert(item, item_slot.clone());
            if let (Some(index_name), Some(index_slot)) = (index.as_ref(), index_slot.as_ref()) {
                scope.insert(index_name, index_slot.clone());
            }
            let body = Box::new(ir_expression(
                module_id_value,
                source.clone(),
                body,
                scope,
                &format!("{path}:body"),
            ));
            scope.pop();
            NxIrExpressionOp::For {
                item: item.clone(),
                item_slot,
                index: index.clone(),
                index_slot,
                iterable,
                body,
            }
        }
        CodegenExpressionKind::Index { base, index } => NxIrExpressionOp::Index {
            base: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                base,
                scope,
                &format!("{path}:base"),
            )),
            index: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                index,
                scope,
                &format!("{path}:index"),
            )),
        },
        CodegenExpressionKind::Member {
            base,
            member,
            reference,
        } => NxIrExpressionOp::Member {
            base: Box::new(ir_expression(
                module_id_value,
                source.clone(),
                base,
                scope,
                &format!("{path}:base"),
            )),
            member: member.clone(),
            reference: reference.as_ref().map(ir_reference),
        },
        CodegenExpressionKind::EnumMember {
            enum_reference,
            member,
        } => NxIrExpressionOp::EnumMember {
            enumeration: ir_reference(enum_reference),
            member: member.clone(),
        },
        CodegenExpressionKind::UnionCase {
            union_reference,
            case_name,
            fields,
            properties,
            content_field,
            content,
        } => NxIrExpressionOp::UnionCase {
            union: ir_reference(union_reference),
            case_name: case_name.clone(),
            fields: ir_record_fields(module_id_value, source.clone(), &id, "field", fields, scope),
            properties: ir_properties(
                module_id_value,
                source.clone(),
                properties,
                scope,
                &format!("{path}:property"),
            ),
            content_field: content_field.clone(),
            content: ir_expressions(
                module_id_value,
                source.clone(),
                content,
                scope,
                &format!("{path}:content"),
            ),
        },
        CodegenExpressionKind::Record {
            name,
            fields,
            properties,
            content_field,
            content,
        } => NxIrExpressionOp::Record {
            name: name.clone(),
            fields: ir_record_fields(module_id_value, source.clone(), &id, "field", fields, scope),
            properties: ir_properties(
                module_id_value,
                source.clone(),
                properties,
                scope,
                &format!("{path}:property"),
            ),
            content_field: content_field.clone(),
            content: ir_expressions(
                module_id_value,
                source.clone(),
                content,
                scope,
                &format!("{path}:content"),
            ),
        },
        CodegenExpressionKind::ComponentDescriptor(descriptor) => {
            ir_component_descriptor_op(module_id_value, source.clone(), descriptor, scope, path)
        }
        CodegenExpressionKind::Element(element) => {
            ir_element_op(module_id_value, source.clone(), element, scope, path)
        }
        CodegenExpressionKind::Unsupported(unsupported) => NxIrExpressionOp::Literal {
            value: NxIrLiteral::String {
                value: unsupported.message.clone(),
            },
        },
    };

    NxIrExpression {
        id,
        span: ir_span(source, expression.span),
        ty: expression.ty.as_ref().map(ir_semantic_type),
        op,
    }
}

fn ir_match_arm(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    arm: &CodegenMatchArm,
    scope: &mut SlotScope,
    path: &str,
) -> NxIrMatchArm {
    NxIrMatchArm {
        patterns: ir_expressions(
            module_id_value,
            source.clone(),
            &arm.patterns,
            scope,
            &format!("{path}:pattern"),
        ),
        body: ir_expression(
            module_id_value,
            source,
            &arm.body,
            scope,
            &format!("{path}:body"),
        ),
    }
}

fn ir_statement(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    statement: &CodegenStatement,
    scope: &mut SlotScope,
    path: &str,
) -> NxIrStatement {
    match statement {
        CodegenStatement::Let { name, init, span } => {
            let init = ir_expression(
                module_id_value,
                source.clone(),
                init,
                scope,
                &format!("{path}:init"),
            );
            let slot = format!("{}:block:{name}", init.id);
            scope.insert(name, slot.clone());
            NxIrStatement::Let {
                name: name.clone(),
                slot,
                init,
                span: ir_span(source, *span),
            }
        }
        CodegenStatement::Expr(expr) => NxIrStatement::Expr {
            expr: ir_expression(
                module_id_value,
                source,
                expr,
                scope,
                &format!("{path}:expr"),
            ),
        },
    }
}

fn ir_component_descriptor_op(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    descriptor: &CodegenComponentDescriptor,
    scope: &mut SlotScope,
    path: &str,
) -> NxIrExpressionOp {
    NxIrExpressionOp::ComponentDescriptor {
        component: ir_reference(&descriptor.component),
        target_kind: match descriptor.target_kind {
            CodegenComponentTargetKind::Normal => "normal".to_string(),
            CodegenComponentTargetKind::External => "external".to_string(),
        },
        properties: ir_properties(
            module_id_value,
            source.clone(),
            &descriptor.properties,
            scope,
            &format!("{path}:property"),
        ),
        content_field: descriptor.content_field.clone(),
        content: ir_expressions(
            module_id_value,
            source,
            &descriptor.content,
            scope,
            &format!("{path}:content"),
        ),
    }
}

fn ir_element_op(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    element: &CodegenElement,
    scope: &mut SlotScope,
    path: &str,
) -> NxIrExpressionOp {
    NxIrExpressionOp::IntrinsicElement {
        element_id: format!(
            "{}:element:{}",
            module_id(module_id_value),
            element.element_id
        ),
        tag_name: element.tag.clone(),
        properties: ir_properties(
            module_id_value,
            source.clone(),
            &element.properties,
            scope,
            &format!("{path}:property"),
        ),
        content: ir_expressions(
            module_id_value,
            source,
            &element.content,
            scope,
            &format!("{path}:content"),
        ),
    }
}

fn ir_properties(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    properties: &[CodegenProperty],
    scope: &mut SlotScope,
    path: &str,
) -> Vec<NxIrProperty> {
    properties
        .iter()
        .enumerate()
        .map(|(index, property)| NxIrProperty {
            name: property.name.clone(),
            value: ir_expression(
                module_id_value,
                source.clone(),
                &property.value,
                scope,
                &format!("{path}:{index}"),
            ),
            span: ir_span(source.clone(), property.span),
        })
        .collect()
}

fn ir_expressions(
    module_id_value: RuntimeModuleId,
    source: Option<String>,
    expressions: &[CodegenExpression],
    scope: &mut SlotScope,
    path: &str,
) -> Vec<NxIrExpression> {
    expressions
        .iter()
        .enumerate()
        .map(|(index, expression)| {
            ir_expression(
                module_id_value,
                source.clone(),
                expression,
                scope,
                &format!("{path}:{index}"),
            )
        })
        .collect()
}

fn ir_entrypoint(entrypoint: &CodegenEntrypoint) -> NxIrEntrypoint {
    NxIrEntrypoint {
        name: entrypoint.name.clone(),
        reference: ir_reference(&entrypoint.reference),
    }
}

fn ir_source_entry(entry: &CodegenSourceEntry) -> NxIrSourceEntry {
    NxIrSourceEntry {
        identity: entry.identity.clone(),
        source: entry.source.clone(),
    }
}

fn ir_module_provenance(provenance: &CodegenModuleProvenance) -> NxIrModuleProvenance {
    match provenance {
        CodegenModuleProvenance::SourceProvider { identity } => {
            NxIrModuleProvenance::SourceProvider {
                identity: identity.clone(),
            }
        }
        CodegenModuleProvenance::Library {
            root_path,
            module_path,
        } => NxIrModuleProvenance::Library {
            root_path: root_path.display().to_string(),
            module_path: module_path.display().to_string(),
        },
    }
}

fn ir_reference(reference: &CodegenReference) -> NxIrReference {
    NxIrReference {
        module: module_id(reference.module_id),
        declaration: declaration_id(reference),
        name: reference.name.clone(),
        kind: resolved_item_kind_name(reference.kind).to_string(),
    }
}

fn ir_literal(literal: &Literal) -> NxIrLiteral {
    match literal {
        Literal::String(value) => NxIrLiteral::String {
            value: value.to_string(),
        },
        Literal::Int(value) => NxIrLiteral::Int {
            value: value.to_string(),
            number: is_js_safe_integer(*value).then_some(*value),
        },
        Literal::Float(value) => NxIrLiteral::Float { value: value.0 },
        Literal::Boolean(value) => NxIrLiteral::Boolean { value: *value },
        Literal::Null => NxIrLiteral::Null,
    }
}

fn ir_type_ref(ty: &CodegenTypeRef) -> NxIrTypeRef {
    match ty {
        CodegenTypeRef::Primitive { name } => NxIrTypeRef::Primitive { name: name.clone() },
        CodegenTypeRef::Nominal { reference, display } => NxIrTypeRef::Nominal {
            reference: ir_reference(reference),
            display: display.clone(),
        },
        CodegenTypeRef::Array { element } => NxIrTypeRef::Array {
            element: Box::new(ir_type_ref(element)),
        },
        CodegenTypeRef::Nullable { inner } => NxIrTypeRef::Nullable {
            inner: Box::new(ir_type_ref(inner)),
        },
        CodegenTypeRef::Function {
            params,
            return_type,
        } => NxIrTypeRef::Function {
            params: params.iter().map(ir_type_ref).collect(),
            return_type: Box::new(ir_type_ref(return_type)),
        },
    }
}

fn ir_semantic_type(ty: &Type) -> NxIrSemanticType {
    NxIrSemanticType {
        display: ty.to_string(),
        shape: ir_semantic_type_shape(ty),
    }
}

fn ir_semantic_type_shape(ty: &Type) -> NxIrSemanticTypeShape {
    match ty {
        Type::Primitive(primitive) => NxIrSemanticTypeShape::Primitive {
            name: primitive_name(*primitive).to_string(),
        },
        Type::Array(element) => NxIrSemanticTypeShape::Array {
            element: Box::new(ir_semantic_type(element)),
        },
        Type::Nullable(inner) => NxIrSemanticTypeShape::Nullable {
            inner: Box::new(ir_semantic_type(inner)),
        },
        Type::Function { params, ret } => NxIrSemanticTypeShape::Function {
            params: params.iter().map(ir_semantic_type).collect(),
            return_type: Box::new(ir_semantic_type(ret)),
        },
        Type::Named(name) => NxIrSemanticTypeShape::Named {
            name: name.as_str().to_string(),
        },
        Type::Enum(enum_ty) => NxIrSemanticTypeShape::Enum {
            name: enum_ty.name.as_str().to_string(),
            members: enum_ty
                .members
                .iter()
                .map(|member| member.as_str().to_string())
                .collect(),
        },
        Type::Union(union_ty) => NxIrSemanticTypeShape::Union {
            name: union_ty.name.as_str().to_string(),
            cases: union_ty
                .cases
                .iter()
                .map(|case| case.as_str().to_string())
                .collect(),
            base: union_ty.base.as_ref().map(|base| base.as_str().to_string()),
        },
        Type::UnionCase(case_ty) => NxIrSemanticTypeShape::UnionCase {
            union: case_ty.union.as_str().to_string(),
            case_name: case_ty.case.as_str().to_string(),
        },
        Type::Variable(id) => NxIrSemanticTypeShape::Variable { id: *id },
        Type::Unknown => NxIrSemanticTypeShape::Unknown,
        Type::Error => NxIrSemanticTypeShape::Error,
    }
}

fn ir_span_for_module(
    _module_id_value: RuntimeModuleId,
    source: Option<String>,
    span: TextSpan,
) -> NxIrSourceSpan {
    ir_span(source, span)
}

fn ir_span(source: Option<String>, span: TextSpan) -> NxIrSourceSpan {
    NxIrSourceSpan {
        source,
        start: span.start().into(),
        end: span.end().into(),
    }
}

fn module_source_identity(module: &CodegenModule) -> Option<String> {
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => Some(identity.clone()),
        CodegenModuleProvenance::Library { module_path, .. } => {
            Some(module_path.display().to_string())
        }
    }
}

fn module_id(module_id_value: RuntimeModuleId) -> String {
    format!("m{}", module_id_value.as_u32())
}

fn declaration_id(reference: &CodegenReference) -> String {
    format!(
        "{}:d{}",
        module_id(reference.module_id),
        reference.definition_id.index()
    )
}

fn expression_id(module_id_value: RuntimeModuleId, expr_id: u32, path: &str) -> String {
    format!("{}:e{}:{path}", module_id(module_id_value), expr_id)
}

fn resolved_item_kind_name(kind: ResolvedItemKind) -> &'static str {
    match kind {
        ResolvedItemKind::Function => "function",
        ResolvedItemKind::Value => "value",
        ResolvedItemKind::Component => "component",
        ResolvedItemKind::TypeAlias => "typeAlias",
        ResolvedItemKind::Enum => "enum",
        ResolvedItemKind::Union => "union",
        ResolvedItemKind::Record => "record",
    }
}

fn primitive_name(primitive: Primitive) -> &'static str {
    match primitive {
        Primitive::I32 => "i32",
        Primitive::I64 => "i64",
        Primitive::Int => "int",
        Primitive::F32 => "f32",
        Primitive::F64 => "f64",
        Primitive::Float => "float",
        Primitive::String => "string",
        Primitive::Bool => "bool",
        Primitive::Void => "void",
    }
}

fn binop_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "sub",
        BinOp::Mul => "mul",
        BinOp::Div => "div",
        BinOp::Mod => "mod",
        BinOp::Eq => "eq",
        BinOp::Ne => "ne",
        BinOp::Lt => "lt",
        BinOp::Le => "le",
        BinOp::Gt => "gt",
        BinOp::Ge => "ge",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Concat => "concat",
    }
}

fn unop_name(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "neg",
        UnOp::Not => "not",
    }
}

fn is_js_safe_integer(value: i64) -> bool {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value)
}

fn ir_unsupported_diagnostic(module: &CodegenModule, span: TextSpan, message: &str) -> Diagnostic {
    Diagnostic::error("nx-ir-unsupported-construct")
        .with_message(message.to_string())
        .with_label(Label::primary(module_diagnostic_identity(module), span))
        .build()
}

fn module_diagnostic_identity(module: &CodegenModule) -> String {
    match &module.provenance {
        CodegenModuleProvenance::SourceProvider { identity } => identity.clone(),
        CodegenModuleProvenance::Library { module_path, .. } => module_path.display().to_string(),
    }
}
