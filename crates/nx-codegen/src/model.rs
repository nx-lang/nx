use nx_diagnostics::TextSpan;
use nx_hir::{ast, ElementId, ExprId, LocalDefinitionId, Name};
use nx_interpreter::{ModuleQualifiedItemRef, ResolvedItemKind, RuntimeModuleId};
use nx_types::Type;
use std::path::PathBuf;

/// Resolved backend-facing model for one executable NX program.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenProgram {
    pub fingerprint: u64,
    pub modules: Vec<CodegenModule>,
    pub entrypoints: Vec<CodegenEntrypoint>,
    pub component_entrypoints: Vec<CodegenEntrypoint>,
    pub source_entries: Vec<CodegenSourceEntry>,
}

impl CodegenProgram {
    pub fn module(&self, module_id: RuntimeModuleId) -> Option<&CodegenModule> {
        self.modules.iter().find(|module| module.id == module_id)
    }

    pub fn entrypoint(&self, name: &str) -> Option<&CodegenEntrypoint> {
        self.entrypoints
            .iter()
            .find(|entrypoint| entrypoint.name == name)
    }
}

/// One source-map input preserved from the originating artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenSourceEntry {
    pub identity: String,
    pub source: String,
}

/// One lowered module prepared for target emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenModule {
    pub id: RuntimeModuleId,
    pub provenance: CodegenModuleProvenance,
    pub declarations: Vec<CodegenDeclaration>,
    pub imports: Vec<CodegenReference>,
}

/// Origin of one generated module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenModuleProvenance {
    SourceProvider {
        identity: String,
    },
    Library {
        root_path: PathBuf,
        module_path: PathBuf,
    },
}

/// Callable public entrypoint selected from the resolved program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenEntrypoint {
    pub name: String,
    pub reference: CodegenReference,
}

/// Module-qualified declaration/reference captured before emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenReference {
    pub module_id: RuntimeModuleId,
    pub definition_id: LocalDefinitionId,
    pub name: String,
    pub kind: ResolvedItemKind,
}

impl CodegenReference {
    pub fn from_resolved(name: impl Into<String>, reference: &ModuleQualifiedItemRef) -> Self {
        Self {
            module_id: reference.module_id,
            definition_id: reference.definition_id,
            name: name.into(),
            kind: reference.kind,
        }
    }
}

/// Top-level declaration preserved in codegen form.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenDeclaration {
    pub reference: CodegenReference,
    pub span: TextSpan,
    pub kind: CodegenDeclarationKind,
}

/// Supported top-level declaration forms.
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenDeclarationKind {
    Function {
        params: Vec<CodegenParam>,
        body: CodegenExpression,
        return_type: Option<Type>,
    },
    Value {
        value: CodegenExpression,
        ty: Option<Type>,
    },
    Enum {
        members: Vec<String>,
    },
    Record {
        fields: Vec<CodegenRecordField>,
    },
    Component(CodegenComponent),
    Union {
        cases: Vec<CodegenUnionCase>,
    },
    TypeAlias,
    Unsupported(CodegenUnsupportedConstruct),
}

/// Function parameter metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenParam {
    pub name: String,
    pub ty: ast::TypeRef,
    pub is_content: bool,
    pub span: TextSpan,
}

/// Record field metadata preserved for strongly typed target emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenRecordField {
    pub name: String,
    pub ty: ast::TypeRef,
    pub is_content: bool,
    pub is_required: bool,
    pub default: Option<CodegenExpression>,
    pub span: TextSpan,
}

/// Component declaration metadata preserved for executable entrypoint emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenComponent {
    pub is_abstract: bool,
    pub is_external: bool,
    pub props: Vec<CodegenComponentField>,
    pub state: Vec<CodegenComponentField>,
    pub body: Option<CodegenExpression>,
}

/// Prop or state field metadata for component normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenComponentField {
    pub name: String,
    pub ty: ast::TypeRef,
    pub is_content: bool,
    pub is_required: bool,
    pub default: Option<CodegenExpression>,
    pub owner_module_id: RuntimeModuleId,
    pub span: TextSpan,
}

/// One discriminated union case prepared for type and value emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenUnionCase {
    pub name: String,
    pub fields: Vec<CodegenRecordField>,
    pub span: TextSpan,
}

/// Codegen expression with source and type metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenExpression {
    pub expr_id: u32,
    pub span: TextSpan,
    pub ty: Option<Type>,
    pub kind: CodegenExpressionKind,
}

/// Supported eager expression subset.
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenExpressionKind {
    Literal(ast::Literal),
    Identifier {
        name: String,
        reference: Option<CodegenReference>,
    },
    Binary {
        lhs: Box<CodegenExpression>,
        op: ast::BinOp,
        rhs: Box<CodegenExpression>,
    },
    Unary {
        op: ast::UnOp,
        expr: Box<CodegenExpression>,
    },
    Call {
        callee: Box<CodegenExpression>,
        args: Vec<CodegenExpression>,
    },
    If {
        condition: Box<CodegenExpression>,
        then_branch: Box<CodegenExpression>,
        else_branch: Option<Box<CodegenExpression>>,
    },
    Let {
        name: String,
        value: Box<CodegenExpression>,
        body: Box<CodegenExpression>,
    },
    Block {
        statements: Vec<CodegenStatement>,
        expression: Option<Box<CodegenExpression>>,
    },
    Array(Vec<CodegenExpression>),
    For {
        item: String,
        index: Option<String>,
        iterable: Box<CodegenExpression>,
        body: Box<CodegenExpression>,
    },
    Index {
        base: Box<CodegenExpression>,
        index: Box<CodegenExpression>,
    },
    Member {
        base: Box<CodegenExpression>,
        member: String,
        reference: Option<CodegenReference>,
    },
    EnumMember {
        enum_reference: CodegenReference,
        member: String,
    },
    UnionCase {
        union_reference: CodegenReference,
        case_name: String,
        fields: Vec<CodegenRecordField>,
        properties: Vec<CodegenProperty>,
        content_field: Option<String>,
        content: Vec<CodegenExpression>,
    },
    Record {
        name: String,
        fields: Vec<CodegenRecordField>,
        properties: Vec<CodegenProperty>,
    },
    ComponentDescriptor(CodegenComponentDescriptor),
    Element(CodegenElement),
    Unsupported(CodegenUnsupportedConstruct),
}

/// Statement forms supported inside codegen blocks.
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenStatement {
    Let {
        name: String,
        init: CodegenExpression,
        span: TextSpan,
    },
    Expr(CodegenExpression),
}

/// Record or element property in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenProperty {
    pub name: String,
    pub value: CodegenExpression,
    pub span: TextSpan,
}

/// Element expression that resolves to a concrete component descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenComponentDescriptor {
    pub component: CodegenReference,
    pub target_kind: CodegenComponentTargetKind,
    pub properties: Vec<CodegenProperty>,
    pub content_field: Option<String>,
    pub content: Vec<CodegenExpression>,
}

/// Component element target behavior preserved for executable emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenComponentTargetKind {
    Normal,
    External,
}

/// Element expression that serializes to a record-like NxValue payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenElement {
    pub element_id: u32,
    pub tag: String,
    pub properties: Vec<CodegenProperty>,
    pub content: Vec<CodegenExpression>,
}

impl CodegenElement {
    pub fn from_id(element_id: ElementId, tag: &Name) -> Self {
        Self {
            element_id: element_id.into_raw().into_u32(),
            tag: tag.as_str().to_string(),
            properties: Vec::new(),
            content: Vec::new(),
        }
    }
}

/// Unsupported executable construct captured with enough context to produce diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenUnsupportedConstruct {
    pub message: String,
    pub span: TextSpan,
}

pub(crate) fn expr_id_u32(expr_id: ExprId) -> u32 {
    expr_id.into_raw().into_u32()
}
