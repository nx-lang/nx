//! NX HIR (High-level Intermediate Representation) - AST and semantic model.
//!
//! This crate provides the typed Abstract Syntax Tree (AST) layer for NX, built
//! by lowering the Concrete Syntax Tree (CST) from tree-sitter. The HIR is used
//! for semantic analysis, type checking, and code generation.
//!
//! # Architecture
//!
//! - **CST** (from nx-syntax): Full-fidelity parse tree with all tokens
//! - **HIR** (this crate): Simplified, typed AST for semantic analysis
//! - **Type System** (nx-types): Type checking and inference
//!
//! # Example
//!
//! ```ignore
//! use nx_syntax::parse_str;
//! use nx_hir::lower;
//!
//! let parse_result = parse_str("fn add(a: int, b: int) { a + b }", "example.nx");
//! let module = lower(parse_result.root().unwrap(), parse_result.source_id);
//! ```

pub mod ast;
pub mod db;
pub mod lower;
pub mod records;
pub mod scope;

use la_arena::{Arena, Idx};
use nx_diagnostics::{Diagnostic, Label, Severity, TextSpan};
use rustc_hash::FxHashSet;
use smol_str::SmolStr;

// Re-export lowering function
pub use lower::lower;

// Re-export database types
pub use db::{DatabaseImpl, NxDatabase};

// Re-export scope and symbol types
pub use records::{
    effective_record_shape, effective_record_shape_for_name, is_record_subtype,
    resolve_record_definition, validate_record_definitions, EffectiveRecordShape,
    InvalidBaseReason, RecordResolutionError,
};
pub use scope::{
    build_scopes, check_undefined_identifiers, Scope, ScopeId, ScopeManager, Symbol, SymbolKind,
};

/// Parses, lowers, and validates a module from source text.
///
/// `file_name` is used for diagnostic labels. Import resolution is intentionally out of scope for
/// this helper; callers that need library-aware analysis should do that in a higher-level layer.
pub fn lower_source_module(
    source: &str,
    file_name: &str,
) -> Result<LoweredModule, Vec<Diagnostic>> {
    let parse_result = nx_syntax::parse_str(source, file_name);

    if parse_result
        .errors
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
    {
        return Err(parse_result.errors);
    }

    let tree = match parse_result.tree {
        Some(tree) => tree,
        None => {
            let diagnostic = Diagnostic::error("parse-failed")
                .with_message("Failed to parse source")
                .build();
            return Err(vec![diagnostic]);
        }
    };

    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let module = lower(tree.root(), source_id);

    if !module.diagnostics().is_empty() {
        let diagnostics = module
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                Diagnostic::error("lowering-error")
                    .with_message(diagnostic.message.clone())
                    .with_label(Label::primary(file_name, diagnostic.span))
                    .build()
            })
            .collect();
        return Err(diagnostics);
    }

    Ok(module)
}

/// Interned string identifier for names.
///
/// Uses `SmolStr` for efficient storage and cloning of small strings.
/// Most identifiers in code are short, so this optimizes for the common case.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(SmolStr);

impl Name {
    /// Create a new name from a string slice.
    pub fn new(s: &str) -> Self {
        Self(SmolStr::new(s))
    }

    /// Get the name as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self(SmolStr::new(s))
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Visibility for top-level declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Visible to the declaring file, peer library files, and consumers.
    Export,
    /// Visible only within the declaring library.
    Internal,
    /// Visible only within the declaring source file.
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Internal
    }
}

/// Unique identifier for a source file.
///
/// This is used to track which source file AST nodes came from, enabling
/// proper error reporting and cross-file analysis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    /// Create a new source ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the numeric ID.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// Function parameter with name and type annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// Parameter name
    pub name: Name,
    /// Parameter type (must be explicit per spec)
    pub ty: ast::TypeRef,
    /// Source location
    pub span: TextSpan,
}

impl Param {
    /// Create a new parameter.
    pub fn new(name: Name, ty: ast::TypeRef, span: TextSpan) -> Self {
        Self { name, ty, span }
    }
}

/// Function declaration with parameters, return type, and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// Function name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Parameter list
    pub params: Vec<Param>,
    /// Return type annotation (None means inferred)
    pub return_type: Option<ast::TypeRef>,
    /// Function body expression
    pub body: ExprId,
    /// Source location
    pub span: TextSpan,
}

/// Top-level value declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDef {
    /// Bound name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Optional type annotation
    pub ty: Option<ast::TypeRef>,
    /// Initializer expression
    pub value: ExprId,
    /// Source location
    pub span: TextSpan,
}

/// Type alias definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    /// Alias name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Target type reference
    pub ty: ast::TypeRef,
    /// Source span
    pub span: TextSpan,
}

/// Enum member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumMember {
    /// Member name
    pub name: Name,
    /// Source span
    pub span: TextSpan,
}

/// Enum definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    /// Enum name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Members for the enum
    pub members: Vec<EnumMember>,
    /// Source span
    pub span: TextSpan,
}

/// Record field definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordField {
    /// Field name
    pub name: Name,
    /// Field type
    pub ty: ast::TypeRef,
    /// Default value expression (if present)
    pub default: Option<ExprId>,
    /// Source span
    pub span: TextSpan,
}

/// Distinguishes ordinary records from action records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    /// Standard `type Name = { ... }` record declaration.
    Plain,
    /// `action Name = { ... }` declaration.
    Action,
}

/// Record type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDef {
    /// Record name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Whether this record came from `type` or `action` syntax.
    pub kind: RecordKind,
    /// Whether the record was declared with the `abstract` modifier.
    pub is_abstract: bool,
    /// Optional abstract base record or alias name.
    pub base: Option<Name>,
    /// Property definitions
    pub properties: Vec<RecordField>,
    /// Source span
    pub span: TextSpan,
}

impl RecordDef {
    /// Returns true when this record was declared with the `action` keyword.
    pub fn is_action(&self) -> bool {
        self.kind == RecordKind::Action
    }

    /// Returns true when this record is declared as abstract.
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }
}

/// Distinguishes inline emitted actions from shared emitted action references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentEmitKind {
    /// `ActionName { ... }` declared inline inside `emits`.
    Inline,
    /// `ActionName` or `Namespace.ActionName` referenced from `emits`.
    Shared,
}

/// Metadata for a component-emitted action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentEmit {
    /// Local emitted action name used for `on<ActionName>` bindings.
    pub name: Name,
    /// Exported action type name. Inline emits use `<Component>.<Action>`.
    pub action_name: Name,
    /// Whether this emit was defined inline or referenced.
    pub kind: ComponentEmitKind,
    /// Source span
    pub span: TextSpan,
}

/// Executable component declaration preserved in HIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Component name
    pub name: Name,
    /// Declaration visibility
    pub visibility: Visibility,
    /// Declared props, including optional default expressions
    pub props: Vec<RecordField>,
    /// Declared emitted actions
    pub emits: Vec<ComponentEmit>,
    /// Declared state fields, including optional default expressions
    pub state: Vec<RecordField>,
    /// Lowered component body expression
    pub body: ExprId,
    /// Source span
    pub span: TextSpan,
}

/// Element property (key-value pair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Property key
    pub key: Name,
    /// Property value expression
    pub value: ExprId,
    /// Source location
    pub span: TextSpan,
}

/// NX element (XML-like syntax).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Element tag name
    pub tag: Name,
    /// Element properties
    pub properties: Vec<Property>,
    /// Nested child expressions in source order
    pub children: Vec<ExprId>,
    /// Closing tag name (must match opening tag)
    pub close_name: Option<Name>,
    /// Source location
    pub span: TextSpan,
}

/// Top-level item in a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// Function declaration
    Function(Function),
    /// Top-level value declaration
    Value(ValueDef),
    /// Component declaration
    Component(Component),
    /// Type alias declaration
    TypeAlias(TypeAlias),
    /// Enum declaration
    Enum(EnumDef),
    /// Record declaration
    Record(RecordDef),
}

impl Item {
    /// Returns the declared item name.
    pub fn name(&self) -> &Name {
        match self {
            Item::Function(func) => &func.name,
            Item::Value(value) => &value.name,
            Item::Component(component) => &component.name,
            Item::TypeAlias(alias) => &alias.name,
            Item::Enum(enum_def) => &enum_def.name,
            Item::Record(record_def) => &record_def.name,
        }
    }

    /// Returns the declared item visibility.
    pub fn visibility(&self) -> Visibility {
        match self {
            Item::Function(func) => func.visibility,
            Item::Value(value) => value.visibility,
            Item::Component(component) => component.visibility,
            Item::TypeAlias(alias) => alias.visibility,
            Item::Enum(enum_def) => enum_def.visibility,
            Item::Record(record_def) => record_def.visibility,
        }
    }
}

/// Import kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    /// `import "<path>" [as Alias]`
    Wildcard {
        /// Optional wildcard namespace alias (`import "<path>" as Alias`)
        alias: Option<Name>,
    },
    /// `import { Name, Name as Alias } from "..."`
    Selective {
        /// Selective imports for `import { ... } ...`
        entries: Vec<SelectiveImport>,
    },
}

/// Individual selective import entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectiveImport {
    /// Imported symbol name
    pub name: Name,
    /// Optional qualifier prefix introduced by `as Prefix.Name`.
    pub qualifier: Option<Name>,
    /// Source span for this selective import entry.
    pub span: TextSpan,
}

/// Lowered import statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// Library path from the import statement.
    pub library_path: String,
    /// Wildcard or selective import data
    pub kind: ImportKind,
    /// Source span for the full import statement.
    pub span: TextSpan,
}

/// Arena index for expressions.
pub type ExprId = Idx<ast::Expr>;

/// Arena index for elements.
pub type ElementId = Idx<Element>;

/// High-level intermediate representation of a module.
///
/// A module corresponds to a single .nx source file and contains all top-level
/// items (functions, type aliases, enums, records) along with the expression
/// and element arenas. Top-level elements are represented as implicit 'root'
/// functions rather than as separate items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredModule {
    /// Source file identifier
    pub source_id: SourceId,
    /// Import statements in source order.
    pub imports: Vec<Import>,
    /// Top-level items
    items: Vec<Item>,
    /// Imported interface-only items synthesized for transient analysis.
    external_items: FxHashSet<usize>,
    /// Lowering-time diagnostics
    diagnostics: Vec<LoweringDiagnostic>,
    /// Arena for all expressions
    exprs: Arena<ast::Expr>,
    /// Arena for all elements
    elements: Arena<Element>,
}

impl LoweredModule {
    /// Create a new empty module.
    pub fn new(source_id: SourceId) -> Self {
        Self {
            source_id,
            imports: Vec::new(),
            items: Vec::new(),
            external_items: FxHashSet::default(),
            diagnostics: Vec::new(),
            exprs: Arena::new(),
            elements: Arena::new(),
        }
    }

    /// Get all top-level items.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Get all lowering diagnostics.
    pub fn diagnostics(&self) -> &[LoweringDiagnostic] {
        &self.diagnostics
    }

    /// Find an item by name.
    pub fn find_item(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.name().as_str() == name)
    }

    /// Add a new item to the module.
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Add a synthesized interface-only item used during transient analysis.
    pub fn add_external_item(&mut self, item: Item) {
        let index = self.items.len();
        self.items.push(item);
        self.external_items.insert(index);
    }

    /// Returns true when the item at `index` is a synthesized external interface item.
    pub fn is_external_item(&self, index: usize) -> bool {
        self.external_items.contains(&index)
    }

    /// Add a lowering diagnostic.
    pub fn add_diagnostic(&mut self, diagnostic: LoweringDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Allocate a new expression in the arena.
    pub fn alloc_expr(&mut self, expr: ast::Expr) -> ExprId {
        self.exprs.alloc(expr)
    }

    /// Get an expression by ID.
    pub fn expr(&self, id: ExprId) -> &ast::Expr {
        &self.exprs[id]
    }

    /// Get the number of lowered expressions in the arena.
    pub fn expr_count(&self) -> usize {
        self.exprs.len()
    }

    /// Allocate a new element in the arena.
    pub fn alloc_element(&mut self, element: Element) -> ElementId {
        self.elements.alloc(element)
    }

    /// Get an element by ID.
    pub fn element(&self, id: ElementId) -> &Element {
        &self.elements[id]
    }
}

/// Lowering diagnostic produced while converting syntax to HIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringDiagnostic {
    /// Human-readable message
    pub message: String,
    /// Source span
    pub span: TextSpan,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::TextSize;

    #[test]
    fn test_name_creation() {
        let name = Name::new("hello");
        assert_eq!(name.as_str(), "hello");
    }

    #[test]
    fn test_name_equality() {
        let name1 = Name::new("test");
        let name2 = Name::new("test");
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_source_id() {
        let id1 = SourceId::new(42);
        let id2 = SourceId::new(42);
        assert_eq!(id1, id2);
        assert_eq!(id1.as_u32(), 42);
    }

    #[test]
    fn test_module_creation() {
        let module = LoweredModule::new(SourceId::new(0));
        assert_eq!(module.items().len(), 0);
    }

    #[test]
    fn test_module_find_item() {
        let mut module = LoweredModule::new(SourceId::new(0));

        let func = Function {
            name: Name::new("test"),
            visibility: Visibility::Export,
            params: Vec::new(),
            return_type: None,
            body: module.alloc_expr(ast::Expr::Literal(ast::Literal::Null)),
            span: TextSpan::new(TextSize::from(0), TextSize::from(10)),
        };

        module.add_item(Item::Function(func));

        assert!(module.find_item("test").is_some());
        assert!(module.find_item("nonexistent").is_none());
    }
}
