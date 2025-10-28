//! Scope and symbol resolution.
//!
//! This module provides the infrastructure for resolving identifiers to their
//! definitions, tracking scopes, and detecting undefined references.

use crate::{ExprId, Module, Name};
use la_arena::{Arena, Idx};
use nx_diagnostics::{Diagnostic, TextSpan};
use rustc_hash::FxHashMap;

/// Index into the scope arena.
pub type ScopeId = Idx<Scope>;

/// The kind of symbol (what the identifier refers to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A function definition
    Function,
    /// A variable (from let bindings)
    Variable,
    /// A function parameter
    Parameter,
    /// A type definition
    Type,
}

impl SymbolKind {
    /// Returns a human-readable name for this symbol kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Variable => "variable",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Type => "type",
        }
    }
}

/// A resolved symbol in the program.
///
/// Symbols represent named entities (functions, variables, types, etc.)
/// that can be referenced by identifiers in the source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The name of the symbol
    pub name: Name,
    /// The kind of symbol
    pub kind: SymbolKind,
    /// The source location where this symbol is defined
    pub span: TextSpan,
    /// Optional expression ID for value symbols (variables, parameters)
    pub expr: Option<ExprId>,
}

impl Symbol {
    /// Creates a new symbol.
    pub fn new(name: Name, kind: SymbolKind, span: TextSpan) -> Self {
        Self {
            name,
            kind,
            span,
            expr: None,
        }
    }

    /// Creates a symbol with an associated expression.
    pub fn with_expr(name: Name, kind: SymbolKind, span: TextSpan, expr: ExprId) -> Self {
        Self {
            name,
            kind,
            span,
            expr: Some(expr),
        }
    }
}

/// A lexical scope containing symbol bindings.
///
/// Scopes form a tree structure via parent references, enabling
/// nested scope lookups (e.g., finding variables from outer scopes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Parent scope (None for the root/module scope)
    pub parent: Option<ScopeId>,
    /// Symbols defined in this scope
    symbols: FxHashMap<Name, Symbol>,
}

impl Scope {
    /// Creates a new empty scope with no parent (root scope).
    pub fn new() -> Self {
        Self {
            parent: None,
            symbols: FxHashMap::default(),
        }
    }

    /// Creates a new scope with a parent.
    pub fn with_parent(parent: ScopeId) -> Self {
        Self {
            parent: Some(parent),
            symbols: FxHashMap::default(),
        }
    }

    /// Defines a symbol in this scope.
    ///
    /// If a symbol with the same name already exists, it is shadowed.
    pub fn define(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    /// Looks up a symbol by name in this scope only (does not check parent scopes).
    pub fn lookup_local(&self, name: &Name) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Returns an iterator over all symbols in this scope.
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Returns the number of symbols in this scope.
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns true if this scope has no symbols.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for scopes and symbol resolution.
///
/// This provides arena-based storage for scopes and methods for
/// looking up symbols across the scope hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeManager {
    /// Arena storing all scopes
    scopes: Arena<Scope>,
    /// Root scope (module-level)
    root: ScopeId,
}

impl ScopeManager {
    /// Creates a new scope manager with a root scope.
    pub fn new() -> Self {
        let mut scopes = Arena::new();
        let root = scopes.alloc(Scope::new());
        Self { scopes, root }
    }

    /// Returns the root scope ID.
    pub fn root(&self) -> ScopeId {
        self.root
    }

    /// Creates a new child scope.
    pub fn create_child(&mut self, parent: ScopeId) -> ScopeId {
        let scope = Scope::with_parent(parent);
        self.scopes.alloc(scope)
    }

    /// Gets a scope by ID.
    pub fn get(&self, id: ScopeId) -> &Scope {
        &self.scopes[id]
    }

    /// Gets a mutable reference to a scope by ID.
    pub fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id]
    }

    /// Resolves a name by searching the scope hierarchy.
    ///
    /// Starts at the given scope and walks up the parent chain until
    /// the symbol is found or we reach the root.
    pub fn resolve(&self, name: &Name, scope: ScopeId) -> Option<&Symbol> {
        let mut current = Some(scope);

        while let Some(scope_id) = current {
            let scope = &self.scopes[scope_id];

            if let Some(symbol) = scope.lookup_local(name) {
                return Some(symbol);
            }

            current = scope.parent;
        }

        None
    }

    /// Defines a symbol in the given scope.
    pub fn define(&mut self, scope: ScopeId, symbol: Symbol) {
        self.scopes[scope].define(symbol);
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds scopes from a module and resolves all identifiers.
///
/// Returns a scope manager and a list of diagnostics for undefined identifiers.
pub fn build_scopes(module: &Module) -> (ScopeManager, Vec<Diagnostic>) {
    let mut manager = ScopeManager::new();
    let mut diagnostics = Vec::new();

    // First pass: define all top-level symbols
    for item in module.items() {
        match item {
            crate::Item::Function(func) => {
                let symbol = Symbol::new(
                    func.name.clone(),
                    SymbolKind::Function,
                    func.span,
                );
                manager.define(manager.root(), symbol);
            }
            crate::Item::TypeAlias => {
                // TODO: Handle type aliases when implemented
            }
            crate::Item::Element(_) => {
                // Elements don't introduce symbols at module scope
            }
        }
    }

    // TODO: Second pass - validate all identifier references
    // This will be implemented when we add expression traversal

    (manager, diagnostics)
}

/// Checks all identifier expressions in a module and reports undefined references.
pub fn check_undefined_identifiers(
    _module: &Module,
    _scope_manager: &ScopeManager,
) -> Vec<Diagnostic> {
    // TODO: Implement when we have expression traversal
    // For now, return empty diagnostics
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::TextSize;

    #[test]
    fn test_symbol_creation() {
        let name = Name::new("x");
        let span = TextSpan::new(TextSize::from(0), TextSize::from(5));
        let symbol = Symbol::new(name.clone(), SymbolKind::Variable, span);

        assert_eq!(symbol.name, name);
        assert_eq!(symbol.kind, SymbolKind::Variable);
        assert_eq!(symbol.span, span);
        assert!(symbol.expr.is_none());
    }

    #[test]
    fn test_symbol_kind_str() {
        assert_eq!(SymbolKind::Function.as_str(), "function");
        assert_eq!(SymbolKind::Variable.as_str(), "variable");
        assert_eq!(SymbolKind::Parameter.as_str(), "parameter");
        assert_eq!(SymbolKind::Type.as_str(), "type");
    }

    #[test]
    fn test_scope_creation() {
        let scope = Scope::new();
        assert!(scope.parent.is_none());
        assert!(scope.is_empty());
    }

    #[test]
    fn test_scope_define() {
        let mut scope = Scope::new();
        let name = Name::new("x");
        let span = TextSpan::new(TextSize::from(0), TextSize::from(1));
        let symbol = Symbol::new(name.clone(), SymbolKind::Variable, span);

        scope.define(symbol);
        assert_eq!(scope.len(), 1);
        assert!(scope.lookup_local(&name).is_some());
    }

    #[test]
    fn test_scope_lookup() {
        let mut scope = Scope::new();
        let name = Name::new("test");
        let span = TextSpan::new(TextSize::from(0), TextSize::from(4));
        let symbol = Symbol::new(name.clone(), SymbolKind::Function, span);

        scope.define(symbol.clone());

        let found = scope.lookup_local(&name);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, name);
        assert_eq!(found.unwrap().kind, SymbolKind::Function);
    }

    #[test]
    fn test_scope_manager_creation() {
        let manager = ScopeManager::new();
        let root_scope = manager.get(manager.root());
        assert!(root_scope.parent.is_none());
        assert!(root_scope.is_empty());
    }

    #[test]
    fn test_scope_manager_child() {
        let mut manager = ScopeManager::new();
        let root = manager.root();
        let child = manager.create_child(root);

        let child_scope = manager.get(child);
        assert_eq!(child_scope.parent, Some(root));
    }

    #[test]
    fn test_scope_manager_resolve_local() {
        let mut manager = ScopeManager::new();
        let root = manager.root();

        let name = Name::new("foo");
        let span = TextSpan::new(TextSize::from(0), TextSize::from(3));
        let symbol = Symbol::new(name.clone(), SymbolKind::Function, span);

        manager.define(root, symbol);

        let found = manager.resolve(&name, root);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, name);
    }

    #[test]
    fn test_scope_manager_resolve_parent() {
        let mut manager = ScopeManager::new();
        let root = manager.root();
        let child = manager.create_child(root);

        // Define symbol in root
        let name = Name::new("outer");
        let span = TextSpan::new(TextSize::from(0), TextSize::from(5));
        let symbol = Symbol::new(name.clone(), SymbolKind::Variable, span);
        manager.define(root, symbol);

        // Resolve from child - should find in parent
        let found = manager.resolve(&name, child);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, name);
    }

    #[test]
    fn test_scope_manager_shadowing() {
        let mut manager = ScopeManager::new();
        let root = manager.root();
        let child = manager.create_child(root);

        let name = Name::new("x");
        let span1 = TextSpan::new(TextSize::from(0), TextSize::from(1));
        let span2 = TextSpan::new(TextSize::from(10), TextSize::from(11));

        // Define in root
        let symbol1 = Symbol::new(name.clone(), SymbolKind::Variable, span1);
        manager.define(root, symbol1);

        // Shadow in child
        let symbol2 = Symbol::new(name.clone(), SymbolKind::Variable, span2);
        manager.define(child, symbol2);

        // Resolve from child - should find child's version
        let found = manager.resolve(&name, child);
        assert!(found.is_some());
        assert_eq!(found.unwrap().span, span2); // Child's span, not parent's
    }

    #[test]
    fn test_scope_manager_undefined() {
        let manager = ScopeManager::new();
        let root = manager.root();
        let name = Name::new("undefined");

        let found = manager.resolve(&name, root);
        assert!(found.is_none());
    }

    #[test]
    fn test_build_scopes_empty_module() {
        let module = Module::new(crate::SourceId::new(0));
        let (manager, diagnostics) = build_scopes(&module);

        assert!(manager.get(manager.root()).is_empty());
        assert!(diagnostics.is_empty());
    }
}
