# Data Model: Core NX Parsing and Validation

**Feature**: 001-nx-core-parsing
**Date**: 2025-10-26

## Overview

This document defines the core data structures and entities that comprise the NX parsing and type checking system. The model follows a layered architecture:

1. **CST Layer** (tree-sitter): Concrete Syntax Tree with full fidelity
2. **HIR Layer** (Rust): High-level Intermediate Representation (AST)
3. **Type Layer**: Type system and inference

---

## Entity Relationships

```
Source File
    ↓
[tree-sitter parser]
    ↓
CST (Concrete Syntax Tree)
    ↓
[Lowering]
    ↓
HIR Module (arena-based AST)
    ├── Functions
    ├── Types
    ├── Elements
    └── Expressions
    ↓
[Type Checker]
    ↓
TypedModule (with type information)
    ├── SymbolTable
    ├── TypeEnvironment
    └── Diagnostics
```

---

## 1. CST Layer (tree-sitter)

### SyntaxTree

**Description**: Immutable tree-sitter parse tree with typed Rust wrappers

**Fields**:
- `tree: Tree` - tree-sitter parse tree
- `source: String` - original source code
- `source_id: SourceId` - unique file identifier

**Relationships**:
- Contains: `SyntaxNode` (root node)
- Produces: `Diagnostic` (parse errors)

**Validation Rules**:
- Tree must be created from valid UTF-8 source
- Source ID must be unique within a parsing session

### SyntaxNode

**Description**: Typed wrapper over tree-sitter Node

**Fields**:
- `node: Node<'tree>` - tree-sitter node handle
- `kind: SyntaxKind` - typed enum of node types

**Methods**:
- `kind() -> SyntaxKind` - Get node type
- `text() -> &str` - Get node source text
- `span() -> TextSpan` - Get source location
- `children() -> impl Iterator<Item = SyntaxNode>` - Child nodes
- `child_by_field(field: &str) -> Option<SyntaxNode>` - Named child access

**Validation Rules**:
- Node lifetime tied to tree lifetime
- Kind must match tree-sitter node type

### SyntaxKind (enum)

**Description**: Exhaustive enumeration of all node and token types

**Variants**:
```rust
pub enum SyntaxKind {
    // Tokens
    Identifier,
    StringLiteral,
    IntLiteral,
    FloatLiteral,

    // Keywords
    FnKeyword,
    LetKeyword,
    IfKeyword,

    // Nodes
    SourceFile,
    Function,
    Element,
    Expression,
    TypeAnnotation,

    // ... (50+ variants)
}
```

---

## 2. HIR Layer (Abstract Syntax)

### Module

**Description**: Top-level container for a single .nx file's AST

**Fields**:
- `items: Arena<Item>` - All top-level items (functions, types, elements)
- `source_id: SourceId` - Links back to source file
- `scope: ScopeId` - Module-level scope

**Relationships**:
- Contains: `Item` (functions, types, elements)
- Has: `Scope` (for symbol resolution)

**State Transitions**: Immutable after lowering

### Item (enum)

**Description**: Top-level declaration in a module

**Variants**:
```rust
pub enum Item {
    Function(Function),
    TypeAlias(TypeAlias),
    Element(Element),
}
```

### Function

**Description**: Function declaration with parameters, return type, and body

**Fields**:
- `name: Name` - Function identifier
- `params: Vec<Param>` - Parameter list
- `return_type: Option<TypeRef>` - Return type annotation (or inferred)
- `body: Expr` - Function body expression
- `span: TextSpan` - Source location

**Relationships**:
- Contains: `Param` (0+ parameters)
- Contains: `Expr` (body)
- Has: `TypeRef` (return type)

**Validation Rules**:
- Name must be unique within module scope
- Parameters must have explicit types (per spec clarifications)
- Body must be a valid expression

### Param

**Description**: Function parameter

**Fields**:
- `name: Name` - Parameter name
- `ty: TypeRef` - Explicit type annotation
- `span: TextSpan` - Source location

**Validation Rules**:
- Type must be explicitly specified (no inference for params)
- Name must be unique within function scope

### Element

**Description**: NX element (XML-like syntax)

**Fields**:
- `tag: Name` - Element tag name
- `properties: Vec<Property>` - Element properties
- `children: Vec<Element>` - Nested elements
- `close_name: Option<Name>` - Closing tag (must match opening)
- `span: TextSpan` - Source location

**Relationships**:
- Contains: `Property` (0+ key-value pairs)
- Contains: `Element` (0+ children, recursive)

**Validation Rules**:
- Opening and closing tags must match (if closing tag present)
- Property names must be unique within element
- Children must be well-formed elements

### Property

**Description**: Key-value property on an element

**Fields**:
- `key: Name` - Property key
- `value: Expr` - Property value expression
- `span: TextSpan` - Source location

**Validation Rules**:
- Key must be a valid identifier
- Value must be a valid expression

### Expr (enum)

**Description**: Expression AST node

**Variants**:
```rust
pub enum Expr {
    Literal(Literal),
    Ident(Name),
    BinaryOp { lhs: ExprId, op: BinOp, rhs: ExprId },
    UnaryOp { op: UnOp, expr: ExprId },
    Call { func: ExprId, args: Vec<ExprId> },
    If { condition: ExprId, then_branch: ExprId, else_branch: Option<ExprId> },
    Block { stmts: Vec<Stmt>, expr: Option<ExprId> },
    Array { elements: Vec<ExprId> },
    Index { base: ExprId, index: ExprId },
}
```

**Relationships**:
- Stored in: `Arena<Expr>` (referenced by `ExprId`)
- May contain: Other `Expr` (via `ExprId`)

### Literal (enum)

**Description**: Literal value

**Variants**:
```rust
pub enum Literal {
    String(SmolStr),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}
```

### Stmt (enum)

**Description**: Statement AST node

**Variants**:
```rust
pub enum Stmt {
    Let { name: Name, ty: Option<TypeRef>, init: ExprId },
    Expr(ExprId),
}
```

### TypeRef

**Description**: Reference to a type (before type checking)

**Fields**:
```rust
pub enum TypeRef {
    Name(Name),                      // User-defined or primitive type
    Array(Box<TypeRef>),             // T[]
    Nullable(Box<TypeRef>),          // T?
    Function {                        // (T1, T2) => T3
        params: Vec<TypeRef>,
        return_type: Box<TypeRef>,
    },
}
```

---

## 3. Type System

### Type (enum)

**Description**: Actual type after type checking and inference

**Variants**:
```rust
pub enum Type {
    // Primitives
    String,
    Int,
    Float,
    Boolean,
    Void,

    // Compound
    Array(Box<Type>),
    Nullable(Box<Type>),
    Function { params: Vec<Type>, return_type: Box<Type> },

    // User-defined
    Named { name: Name, ty_id: TypeId },

    // Type inference
    Infer(InferenceVar),  // Unresolved type variable
    Error,                // Type error sentinel
}
```

### TypeEnvironment

**Description**: Maps symbols to types

**Fields**:
- `bindings: FxHashMap<Name, Type>` - Symbol → Type mapping
- `parent: Option<TypeEnvId>` - Parent scope for nested scopes

**Methods**:
- `lookup(&self, name: Name) -> Option<Type>` - Resolve type by name
- `insert(&mut self, name: Name, ty: Type)` - Add binding
- `with_parent(parent: TypeEnvId) -> Self` - Create child scope

### InferenceContext

**Description**: Type inference state during type checking

**Fields**:
- `var_counter: u32` - Counter for fresh type variables
- `substitutions: FxHashMap<InferenceVar, Type>` - Solved type variables
- `constraints: Vec<Constraint>` - Unification constraints

**Methods**:
- `fresh_var() -> InferenceVar` - Create new type variable
- `unify(ty1: Type, ty2: Type) -> Result<(), TypeError>` - Unify types
- `resolve(var: InferenceVar) -> Option<Type>` - Look up solved variable

### Constraint

**Description**: Type equality constraint for inference

**Fields**:
```rust
pub struct Constraint {
    pub lhs: Type,
    pub rhs: Type,
    pub span: TextSpan,  // For error reporting
}
```

---

## 4. Diagnostics

### Diagnostic

**Description**: Error, warning, or info message with source context

**Fields**:
- `severity: Severity` - Error, Warning, or Info
- `message: String` - Primary error message
- `span: TextSpan` - Source location
- `labels: Vec<Label>` - Additional annotated spans
- `notes: Vec<String>` - Help text

**Relationships**:
- References: `TextSpan` (source location)
- Contains: `Label` (additional context)

**Validation Rules**:
- Span must be valid within source file
- Message must be non-empty

### Severity (enum)

```rust
pub enum Severity {
    Error,
    Warning,
    Info,
}
```

### Label

**Description**: Annotated source span for multi-location errors

**Fields**:
- `span: TextSpan` - Source location
- `message: String` - Annotation text
- `style: LabelStyle` - Primary or Secondary

---

## 5. Supporting Types

### Name

**Description**: Interned string identifier

```rust
pub struct Name(SmolStr);
```

**Rationale**: Small string optimization, cheap clone/compare

### TextSpan

**Description**: Source code location

**Fields**:
- `start: TextSize` - Byte offset of start
- `end: TextSize` - Byte offset of end

**Methods**:
- `len() -> usize` - Length in bytes
- `contains(&self, pos: TextSize) -> bool` - Point containment

### SourceId

**Description**: Unique identifier for a source file

```rust
pub struct SourceId(u32);
```

**Rationale**: Compact, efficient for lookups

### Scope

**Description**: Symbol resolution scope

**Fields**:
- `symbols: FxHashMap<Name, Symbol>` - Name → Symbol mapping
- `parent: Option<ScopeId>` - Parent scope

### Symbol

**Description**: Resolved identifier in scope

**Fields**:
```rust
pub struct Symbol {
    pub name: Name,
    pub kind: SymbolKind,
    pub ty: Type,
    pub span: TextSpan,
}
```

**SymbolKind**:
```rust
pub enum SymbolKind {
    Function,
    Variable,
    Parameter,
    Type,
}
```

---

## Arena-Based Storage

All AST nodes use arena allocation for performance:

```rust
pub struct Module {
    items: Arena<Item>,
    exprs: Arena<Expr>,
    types: Arena<TypeRef>,
    // ... other arenas
}
```

**Rationale**:
- Fast allocation (bump allocator)
- Fast deallocation (drop entire arena)
- Stable IDs (arena indices)
- Good memory locality

---

## Salsa Integration

HIR and type checking integrate with Salsa for incremental computation:

```rust
#[salsa::query_group(NxDatabaseStorage)]
pub trait NxDatabase {
    // Input queries
    #[salsa::input]
    fn source_text(&self, file: SourceId) -> Arc<String>;

    // Derived queries
    fn parse(&self, file: SourceId) -> Arc<SyntaxTree>;
    fn lower(&self, file: SourceId) -> Arc<Module>;
    fn type_check(&self, file: SourceId) -> Arc<TypedModule>;
}
```

---

## Summary

This data model provides:

1. **CST Layer**: Full-fidelity parse tree via tree-sitter
2. **HIR Layer**: Simplified AST for semantic analysis
3. **Type Layer**: Type representation and inference
4. **Diagnostics**: Rich error reporting with source context
5. **Salsa Integration**: Incremental computation for performance

All entities follow Rust best practices:
- Immutable after construction
- Arena allocation for AST nodes
- Interned strings for identifiers
- Explicit lifetimes where needed

---

**Data Model Status**: ✅ Complete - Ready for implementation
