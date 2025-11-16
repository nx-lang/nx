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
    /// Nested child elements
    pub children: Vec<ElementId>,
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
    /// Type alias declaration
    TypeAlias(TypeAlias),
    /// Enum declaration
    Enum(EnumDef),
    /// Element declaration
    Element(ElementId),
}

/// Arena index for expressions.
pub type ExprId = Idx<ast::Expr>;

/// Arena index for elements.
pub type ElementId = Idx<Element>;

/// High-level intermediate representation of a module.
///
/// A module corresponds to a single .nx source file and contains all top-level
/// items (functions, types, elements) along with the expression arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Source file identifier
    pub source_id: SourceId,
    /// Top-level items
    items: Vec<Item>,
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
            items: Vec::new(),
            exprs: Arena::new(),
            elements: Arena::new(),
        }
    }

    /// Get all top-level items.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Find an item by name.
    pub fn find_item(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|item| match item {
            Item::Function(func) => func.name.as_str() == name,
            Item::TypeAlias(alias) => alias.name.as_str() == name,
            Item::Enum(enum_def) => enum_def.name.as_str() == name,
            Item::Element(_) => false,
        })
    }

    /// Add a new item to the module.
    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Allocate a new expression in the arena.
    pub fn alloc_expr(&mut self, expr: ast::Expr) -> ExprId {
        self.exprs.alloc(expr)
    }

    /// Get an expression by ID.
    pub fn expr(&self, id: ExprId) -> &ast::Expr {
        &self.exprs[id]
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
