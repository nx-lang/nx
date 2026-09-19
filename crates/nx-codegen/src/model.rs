use nx_diagnostics::TextSpan;
use nx_hir::{ast, ElementId, ExprId, LocalDefinitionId, Name, UpdateIntrinsic};
use nx_interpreter::{ModuleQualifiedItemRef, ResolvedItemKind, RuntimeModuleId};
use nx_types::Type;
use std::path::PathBuf;

/// Resolved backend-facing model for one executable NX program.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenProgram {
    pub fingerprint: u64,
    /// The workspace identity of the module the program was built for.
    pub entry_identity: String,
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
    /// The version string the host gave the module, if any.
    pub version: Option<String>,
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
    Record {
        fields: Vec<CodegenRecordField>,
        /// The record's own type parameters, in declaration order, for a plain generic record.
        ///
        /// <para>Empty for every other record, including a generic record's `<Target>.Update`
        /// companion, which erases them. Each field's `ty` names these where it uses one, while
        /// its `resolved_ty` has them erased to the top type — IR carries no type arguments, and
        /// an emitter that declares the record generically reads `ty`.</para>
        type_params: Vec<String>,
        /// The record's abstract bases, nearest first.
        ///
        /// A value of this record is acceptable wherever any of them is expected. The fields are
        /// already flattened, so this exists only to answer that question — a runtime holding a
        /// value stamped with this record's name needs it to tell a subtype from a foreign type.
        bases: Vec<CodegenReference>,
        /// Whether the record was declared `abstract`, and so has no values of its own.
        ///
        /// A base-typed site accepts a value of a record that extends it, never one of the base
        /// itself. A runtime taking host input needs this to hold that line, the way analysis holds
        /// it for NX source.
        is_abstract: bool,
        /// The record or component this derived `<Target>.Update` record patches, if it is one.
        ///
        /// <para>An update record's fields are all optional and carry no defaults, so a runtime
        /// normalizing one keeps an absent field absent rather than filling it.</para>
        update_target: Option<CodegenReference>,
    },
    Component(CodegenComponent),
    Union {
        cases: Vec<CodegenUnionCase>,
        /// The union's abstract bases, nearest first, inherited by every case.
        bases: Vec<CodegenReference>,
        /// The record, action, or component a derived `<Target>.Property` union names the fields
        /// of. Present only on property unions, whose cases are all constant.
        property_target: Option<CodegenReference>,
    },
    TypeAlias,
    Unsupported(CodegenUnsupportedConstruct),
}

/// Function parameter metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenParam {
    pub name: String,
    pub ty: ast::TypeRef,
    pub resolved_ty: CodegenTypeRef,
    pub is_content: bool,
    pub span: TextSpan,
}

/// Resolved type reference metadata preserved for NX IR boundary normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenTypeRef {
    Primitive {
        name: String,
    },
    Nominal {
        reference: CodegenReference,
        display: String,
    },
    Array {
        element: Box<CodegenTypeRef>,
    },
    Nullable {
        inner: Box<CodegenTypeRef>,
    },
    Function {
        params: Vec<CodegenFunctionParam>,
        return_type: Box<CodegenTypeRef>,
    },
}

/// One parameter of a resolved function type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenFunctionParam {
    pub name: String,
    pub ty: CodegenTypeRef,
    pub is_content: bool,
}

/// Record field metadata preserved for strongly typed target emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenRecordField {
    pub name: String,
    pub ty: ast::TypeRef,
    pub resolved_ty: CodegenTypeRef,
    pub is_content: bool,
    pub is_required: bool,
    pub default: Option<CodegenExpression>,
    /// The module that declared the field, which is the module its default's spans belong to.
    pub owner_module_id: RuntimeModuleId,
    pub span: TextSpan,
}

/// Component declaration metadata preserved for executable entrypoint emission.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenComponent {
    pub is_abstract: bool,
    pub is_external: bool,
    /// The component's effective type parameters, inherited first. Prop types in `props` are
    /// unerased and may name these; each emitter decides whether to carry or erase them.
    pub type_params: Vec<String>,
    pub props: Vec<CodegenComponentField>,
    pub state: Vec<CodegenComponentField>,
    /// The component's effective emits, inherited first, in declaration order.
    pub emits: Vec<CodegenComponentEmit>,
    pub body: Option<CodegenExpression>,
}

/// One action a component emits: the local name a parent binds as `on<Name>`, and the action
/// record the handler accepts, resolved in the module that declared the emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenComponentEmit {
    pub name: String,
    pub action: CodegenReference,
}

/// Prop or state field metadata for component normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenComponentField {
    pub name: String,
    pub ty: ast::TypeRef,
    pub resolved_ty: CodegenTypeRef,
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
    /// Whether this case declares no fields in a union that declares no base.
    ///
    /// A constant case carries nothing beyond its own name, so it is emitted as a bare string
    /// rather than a `$type` object, and a union whose cases are all constant generates what an
    /// `enum` generated.
    pub is_constant: bool,
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
    /// String concatenation: a `+` type analysis found to have a string operand. Both operands
    /// are strings, a non-string one having been wrapped in a [`CodegenExpressionKind::ToText`].
    Concat {
        lhs: Box<CodegenExpression>,
        rhs: Box<CodegenExpression>,
    },
    /// The conversion of a primitive value to its canonical text form, naming the operand's
    /// static type so a target that carries every number the same way can still print a
    /// `float32` as one.
    ToText {
        expr: Box<CodegenExpression>,
        ty: ast::PrimitiveType,
    },
    Call {
        callee: Box<CodegenExpression>,
        args: Vec<CodegenExpression>,
    },
    /// A call of a function-typed value — a parameter, a prop or a local holding a function — with
    /// its arguments by name, `<Row Item={c} Index={i} />`. The callee's declaration is known only
    /// at run time, so the names travel with the call and the runtime binds them by the subset
    /// rule: a name the declaration lacks is dropped, one it has must be present.
    NamedCall {
        callee: Box<CodegenExpression>,
        args: Vec<CodegenProperty>,
    },
    /// A call to one of the update intrinsics, which has no callee declaration: the checker
    /// resolved the name before any binding, and a runtime supplies the operation.
    IntrinsicCall {
        intrinsic: UpdateIntrinsic,
        args: Vec<CodegenExpression>,
        /// For `changed`, the declared field order of the update record its argument is typed
        /// as, which the result is sorted by; `None` for the other intrinsics.
        field_order: Option<Vec<String>>,
    },
    If {
        condition: Box<CodegenExpression>,
        then_branch: Box<CodegenExpression>,
        else_branch: Option<Box<CodegenExpression>>,
    },
    Match {
        scrutinee: Box<CodegenExpression>,
        arms: Vec<CodegenMatchArm>,
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
    UnionCase {
        union_reference: CodegenReference,
        case_name: String,
        fields: Vec<CodegenRecordField>,
        properties: Vec<CodegenProperty>,
        content_field: Option<String>,
        content: Vec<CodegenExpression>,
        /// Whether this case declares no fields in a union that declares no base.
        is_constant: bool,
        /// Whether every case of the declaring union is constant.
        ///
        /// A constant union emits a frozen value object, so its cases are reached through it; a
        /// constant case of a mixed union has no such object and is emitted as a bare string.
        union_is_constant: bool,
    },
    Record {
        name: String,
        /// The record declaration being constructed, when the name resolved to one.
        reference: Option<CodegenReference>,
        fields: Vec<CodegenRecordField>,
        properties: Vec<CodegenProperty>,
        content_field: Option<String>,
        content: Vec<CodegenExpression>,
        /// Whether this constructs a derived update record, whose absent fields stay absent.
        ///
        /// <para>Every other record fills an absent field from its default or with `null`; a patch
        /// must not, because an absent field means "unchanged".</para>
        is_update: bool,
    },
    ComponentDescriptor(CodegenComponentDescriptor),
    Element(CodegenElement),
    /// An action-handler binding, `onTapped=<Update count={count + 1} />`.
    ///
    /// <para>Nothing here is evaluated when the binding is built: the body runs when a host
    /// dispatches the action, against the locals captured where the binding was written.</para>
    ActionHandler(CodegenActionHandler),
    Unsupported(CodegenUnsupportedConstruct),
}

/// The handler a parent binds to one of a component's emits.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenActionHandler {
    /// The component whose emit the handler answers.
    pub component: CodegenReference,
    /// The emit's local name: `Tapped` for `onTapped`.
    pub emit: String,
    /// The action record the handler accepts, resolved in the module that declared the emit.
    /// Its declaration name is the public name a rendered handler reports.
    pub action: CodegenReference,
    /// The component whose declaration the binding was written in, whose state the body may
    /// patch; `None` for a binding at the root.
    pub owner: Option<CodegenReference>,
    /// The body, with `action` bound as a local of the enclosing frame.
    pub body: Box<CodegenExpression>,
}

/// One authored-order arm in a match-style `if is` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenMatchArm {
    pub patterns: Vec<CodegenExpression>,
    pub body: CodegenExpression,
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
