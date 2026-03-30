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
pub mod scope;

use la_arena::{Arena, Idx};
use nx_diagnostics::TextSpan;
use smol_str::SmolStr;

// Re-export lowering function
pub use lower::lower;

// Re-export database types
pub use db::{DatabaseImpl, NxDatabase};

// Re-export scope and symbol types
pub use scope::{
    build_scopes, check_undefined_identifiers, Scope, ScopeId, ScopeManager, Symbol, SymbolKind,
};

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
    /// Parameter list
    pub params: Vec<Param>,
    /// Return type annotation (None means inferred)
    pub return_type: Option<ast::TypeRef>,
    /// Function body expression
    pub body: ExprId,
    /// Source location
    pub span: TextSpan,
}

/// Type alias definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    /// Alias name
    pub name: Name,
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
    /// Whether this record came from `type` or `action` syntax.
    pub kind: RecordKind,
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
    /// Public action type name. Inline emits use `<Component>.<Action>`.
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
    /// Component declaration
    Component(Component),
    /// Type alias declaration
    TypeAlias(TypeAlias),
    /// Enum declaration
    Enum(EnumDef),
    /// Record declaration
    Record(RecordDef),
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
    /// Optional local alias
    pub alias: Option<Name>,
}

/// Lowered import statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// Module path from `from "<path>"`
    pub path: String,
    /// Wildcard or selective import data
    pub kind: ImportKind,
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
pub struct Module {
    /// Source file identifier
    pub source_id: SourceId,
    /// Optional prelude/module content type path.
    pub content_type: Option<String>,
    /// Import statements in source order.
    pub imports: Vec<Import>,
    /// Top-level items
    items: Vec<Item>,
    /// Lowering-time diagnostics
    diagnostics: Vec<LoweringDiagnostic>,
    /// Arena for all expressions
    exprs: Arena<ast::Expr>,
    /// Arena for all elements
    elements: Arena<Element>,
}

impl Module {
    /// Create a new empty module.
    pub fn new(source_id: SourceId) -> Self {
        Self {
            source_id,
            content_type: None,
            imports: Vec::new(),
            items: Vec::new(),
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
        self.items.iter().find(|item| match item {
            Item::Function(func) => func.name.as_str() == name,
            Item::Component(component) => component.name.as_str() == name,
            Item::TypeAlias(alias) => alias.name.as_str() == name,
            Item::Enum(enum_def) => enum_def.name.as_str() == name,
            Item::Record(record_def) => record_def.name.as_str() == name,
        })
    }

    /// Add a new item to the module.
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
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
        let module = Module::new(SourceId::new(0));
        assert_eq!(module.items().len(), 0);
    }

    #[test]
    fn test_module_find_item() {
        let mut module = Module::new(SourceId::new(0));

        let func = Function {
            name: Name::new("test"),
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
