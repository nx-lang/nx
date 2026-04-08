//! Scope and symbol resolution.
//!
//! This module provides the infrastructure for resolving identifiers to their
//! definitions, tracking scopes, and detecting undefined references.

use crate::{ast, ExprId, Item, Name, PreparedItemKind, PreparedModule, PreparedNamespace};
use la_arena::{Arena, Idx};
use nx_diagnostics::{Diagnostic, Label, TextSpan};
use rustc_hash::FxHashMap;

/// Index into the scope arena.
pub type ScopeId = Idx<Scope>;

/// The kind of symbol (what the identifier refers to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A function definition
    Function,
    /// A component definition
    Component,
    /// A variable (from let bindings)
    Variable,
    /// A function parameter
    Parameter,
    /// A type definition
    Type,
    /// An enum member
    EnumMember,
}

impl SymbolKind {
    /// Returns a human-readable name for this symbol kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Component => "component",
            SymbolKind::Variable => "variable",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Type => "type",
            SymbolKind::EnumMember => "enum member",
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

/// Builds scopes from a prepared module and seeds the root scope with prepared top-level bindings.
///
/// Returns a scope manager and a list of diagnostics for undefined identifiers.
pub fn build_scopes(module: &PreparedModule) -> (ScopeManager, Vec<Diagnostic>) {
    let mut manager = ScopeManager::new();
    let diagnostics = Vec::new();

    for namespace in [
        PreparedNamespace::Value,
        PreparedNamespace::Element,
        PreparedNamespace::Type,
    ] {
        for binding in module.bindings(namespace) {
            if manager
                .get(manager.root())
                .lookup_local(&binding.visible_name)
                .is_some()
            {
                continue;
            }

            let Some(resolved) = module.resolve_prepared_item(binding) else {
                continue;
            };
            let Some(span) = resolved_item_span(&resolved) else {
                continue;
            };
            let symbol = Symbol::new(
                binding.visible_name.clone(),
                symbol_kind_from_prepared_kind(binding.kind),
                span,
            );
            manager.define(manager.root(), symbol);
        }
    }

    (manager, diagnostics)
}

/// Checks all identifier expressions in a prepared module and reports undefined references.
pub fn check_undefined_identifiers(
    module: &PreparedModule,
    scope_manager: &ScopeManager,
) -> Vec<Diagnostic> {
    let mut checker = UndefinedIdentifierChecker::new(module, scope_manager);
    checker.check();
    checker.finish()
}

fn symbol_kind_from_prepared_kind(kind: PreparedItemKind) -> SymbolKind {
    match kind {
        PreparedItemKind::Function => SymbolKind::Function,
        PreparedItemKind::Value => SymbolKind::Variable,
        PreparedItemKind::Component => SymbolKind::Component,
        PreparedItemKind::TypeAlias | PreparedItemKind::Enum | PreparedItemKind::Record => {
            SymbolKind::Type
        }
    }
}

fn resolved_item_span(item: &crate::ResolvedPreparedItem) -> Option<TextSpan> {
    match item {
        crate::ResolvedPreparedItem::Raw { item, .. } => Some(match item {
            Item::Function(function) => function.span,
            Item::Value(value) => value.span,
            Item::Component(component) => component.span,
            Item::TypeAlias(alias) => alias.span,
            Item::Enum(enum_def) => enum_def.span,
            Item::Record(record) => record.span,
        }),
        crate::ResolvedPreparedItem::Imported { item, .. } => Some(match &item.item {
            crate::InterfaceItemKind::Function { span, .. }
            | crate::InterfaceItemKind::Value { span, .. }
            | crate::InterfaceItemKind::Component { span, .. }
            | crate::InterfaceItemKind::TypeAlias { span, .. }
            | crate::InterfaceItemKind::Enum { span, .. }
            | crate::InterfaceItemKind::Record { span, .. } => *span,
        }),
    }
}

struct UndefinedIdentifierChecker<'a> {
    module: &'a PreparedModule,
    scope_manager: ScopeManager,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> UndefinedIdentifierChecker<'a> {
    fn new(module: &'a PreparedModule, scope_manager: &'a ScopeManager) -> Self {
        Self {
            module,
            scope_manager: scope_manager.clone(),
            diagnostics: Vec::new(),
        }
    }

    fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn check(&mut self) {
        for item in self.module.raw_module().items() {
            match item {
                Item::Function(function) => {
                    let scope = self.scope_manager.create_child(self.scope_manager.root());
                    self.define_params(scope, &function.params);
                    self.check_expr(function.body, scope);
                }
                Item::Value(value) => {
                    self.check_expr(value.value, self.scope_manager.root());
                }
                Item::Component(component) => {
                    let scope = self.scope_manager.create_child(self.scope_manager.root());
                    let symbols =
                        component
                            .props
                            .iter()
                            .map(|field| (field.name.clone(), SymbolKind::Parameter, field.span))
                            .chain(component.state.iter().map(|field| {
                                (field.name.clone(), SymbolKind::Variable, field.span)
                            }))
                            .collect::<Vec<_>>();
                    for (name, kind, span) in symbols {
                        self.scope_manager_define(scope, name, kind, span);
                    }
                    self.check_expr(component.body, scope);
                }
                Item::TypeAlias(_) | Item::Enum(_) | Item::Record(_) => {}
            }
        }
    }

    fn define_params(&mut self, scope: ScopeId, params: &[crate::Param]) {
        for param in params {
            self.scope_manager_define(scope, param.name.clone(), SymbolKind::Parameter, param.span);
        }
    }

    fn scope_manager_define(
        &mut self,
        scope: ScopeId,
        name: Name,
        kind: SymbolKind,
        span: TextSpan,
    ) {
        self.scope_manager
            .define(scope, Symbol::new(name, kind, span));
    }

    fn flattened_expr_name(&self, expr_id: ExprId) -> Option<Name> {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Ident(name) => Some(name.clone()),
            ast::Expr::Member { base, member, .. } => {
                let mut name = self.flattened_expr_name(*base)?.as_str().to_string();
                name.push('.');
                name.push_str(member.as_str());
                Some(Name::new(&name))
            }
            _ => None,
        }
    }

    fn resolves_prepared_binding(&self, name: &Name) -> bool {
        [
            PreparedNamespace::Value,
            PreparedNamespace::Element,
            PreparedNamespace::Type,
        ]
        .iter()
        .any(|namespace| self.module.resolve_binding(*namespace, name).is_some())
    }

    fn check_expr(&mut self, expr_id: ExprId, scope: ScopeId) {
        match self.module.raw_module().expr(expr_id) {
            ast::Expr::Literal(_) | ast::Expr::Error(_) => {}
            ast::Expr::Ident(name) => {
                if self.scope_manager.resolve(name, scope).is_none() {
                    self.report_undefined(name, self.module.raw_module().expr(expr_id).span());
                }
            }
            ast::Expr::BinaryOp { lhs, rhs, .. } => {
                self.check_expr(*lhs, scope);
                self.check_expr(*rhs, scope);
            }
            ast::Expr::UnaryOp { expr, .. } => {
                self.check_expr(*expr, scope);
            }
            ast::Expr::Call { func, args, .. } => {
                self.check_expr(*func, scope);
                for arg in args {
                    self.check_expr(*arg, scope);
                }
            }
            ast::Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expr(*condition, scope);
                self.check_expr(*then_branch, scope);
                if let Some(else_branch) = else_branch {
                    self.check_expr(*else_branch, scope);
                }
            }
            ast::Expr::Array { elements, .. } => {
                for element in elements {
                    self.check_expr(*element, scope);
                }
            }
            ast::Expr::Index { base, index, .. } => {
                self.check_expr(*base, scope);
                self.check_expr(*index, scope);
            }
            ast::Expr::Member { base, .. } => {
                let is_prepared_top_level = self
                    .flattened_expr_name(expr_id)
                    .as_ref()
                    .is_some_and(|name| self.resolves_prepared_binding(name));

                if !is_prepared_top_level {
                    self.check_expr(*base, scope);
                }
            }
            ast::Expr::RecordLiteral { properties, .. } => {
                for (_, expr) in properties {
                    self.check_expr(*expr, scope);
                }
            }
            ast::Expr::Element { element, .. } => {
                let element = self.module.raw_module().element(*element);
                for property in &element.properties {
                    self.check_expr(property.value, scope);
                }
                for content in &element.content {
                    self.check_expr(*content, scope);
                }
            }
            ast::Expr::ActionHandler { body, span, .. } => {
                let handler_scope = self.scope_manager.create_child(scope);
                self.scope_manager_define(
                    handler_scope,
                    Name::new("action"),
                    SymbolKind::Variable,
                    *span,
                );
                self.check_expr(*body, handler_scope);
            }
            ast::Expr::Block { stmts, expr, .. } => {
                let block_scope = self.scope_manager.create_child(scope);
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let {
                            name, init, span, ..
                        } => {
                            self.check_expr(*init, block_scope);
                            self.scope_manager_define(
                                block_scope,
                                name.clone(),
                                SymbolKind::Variable,
                                *span,
                            );
                        }
                        ast::Stmt::Expr(expr, _) => {
                            self.check_expr(*expr, block_scope);
                        }
                    }
                }
                if let Some(expr) = expr {
                    self.check_expr(*expr, block_scope);
                }
            }
            ast::Expr::For {
                item,
                index,
                iterable,
                body,
                ..
            } => {
                self.check_expr(*iterable, scope);
                let for_scope = self.scope_manager.create_child(scope);
                self.scope_manager_define(
                    for_scope,
                    item.clone(),
                    SymbolKind::Variable,
                    self.module.raw_module().expr(*body).span(),
                );
                if let Some(index_name) = index {
                    self.scope_manager_define(
                        for_scope,
                        index_name.clone(),
                        SymbolKind::Variable,
                        self.module.raw_module().expr(*body).span(),
                    );
                }
                self.check_expr(*body, for_scope);
            }
            ast::Expr::Let {
                name, value, body, ..
            } => {
                self.check_expr(*value, scope);
                let let_scope = self.scope_manager.create_child(scope);
                self.scope_manager_define(
                    let_scope,
                    name.clone(),
                    SymbolKind::Variable,
                    self.module.raw_module().expr(*body).span(),
                );
                self.check_expr(*body, let_scope);
            }
        }
    }

    fn report_undefined(&mut self, name: &Name, span: TextSpan) {
        self.diagnostics.push(
            Diagnostic::error("undefined-identifier")
                .with_message(format!("Undefined identifier '{}'", name))
                .with_label(Label::primary(
                    self.module.module_identity().to_string(),
                    span,
                ))
                .build(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast, InterfaceItem, InterfaceItemKind, InterfaceParam, LocalDefinitionId, LoweredModule,
        PreparedBinding, PreparedBindingOrigin, PreparedBindingTarget, PreparedItemKind,
        PreparedModule, PreparedNamespace, Visibility,
    };
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
        assert_eq!(SymbolKind::Component.as_str(), "component");
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
        let module = LoweredModule::new(crate::SourceId::new(0));
        let prepared = PreparedModule::standalone("empty.nx", module);
        let (manager, diagnostics) = build_scopes(&prepared);

        assert!(manager.get(manager.root()).is_empty());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_build_scopes_registers_component_symbols() {
        let mut module = LoweredModule::new(crate::SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(9));
        let body = module.alloc_expr(crate::ast::Expr::Error(span));
        module.add_item(crate::Item::Component(crate::Component {
            name: Name::new("SearchBox"),
            visibility: crate::Visibility::Internal,
            props: Vec::new(),
            emits: Vec::new(),
            state: Vec::new(),
            body,
            span,
        }));

        let prepared = PreparedModule::standalone("component.nx", module);
        let (manager, diagnostics) = build_scopes(&prepared);
        let symbol = manager
            .resolve(&Name::new("SearchBox"), manager.root())
            .expect("Expected component symbol in root scope");

        assert_eq!(symbol.kind, SymbolKind::Component);
        assert_eq!(symbol.name.as_str(), "SearchBox");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_build_scopes_registers_top_level_value_symbols() {
        let mut module = LoweredModule::new(crate::SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(5));
        let value = module.alloc_expr(crate::ast::Expr::Literal(crate::ast::Literal::Int(42)));
        module.add_item(crate::Item::Value(crate::ValueDef {
            name: Name::new("answer"),
            visibility: crate::Visibility::Export,
            ty: Some(crate::ast::TypeRef::name("int")),
            value,
            span,
        }));

        let prepared = PreparedModule::standalone("value.nx", module);
        let (manager, diagnostics) = build_scopes(&prepared);
        let symbol = manager
            .resolve(&Name::new("answer"), manager.root())
            .expect("Expected top-level value symbol in root scope");

        assert_eq!(symbol.kind, SymbolKind::Variable);
        assert_eq!(symbol.name.as_str(), "answer");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_build_scopes_registers_private_top_level_symbols() {
        let mut module = LoweredModule::new(crate::SourceId::new(0));
        let span = TextSpan::new(TextSize::from(0), TextSize::from(6));
        let value = module.alloc_expr(crate::ast::Expr::Literal(crate::ast::Literal::Int(7)));
        module.add_item(crate::Item::Value(crate::ValueDef {
            name: Name::new("secret"),
            visibility: crate::Visibility::Private,
            ty: Some(crate::ast::TypeRef::name("int")),
            value,
            span,
        }));

        let prepared = PreparedModule::standalone("private.nx", module);
        let (manager, diagnostics) = build_scopes(&prepared);
        let symbol = manager
            .resolve(&Name::new("secret"), manager.root())
            .expect("Expected private symbol in the defining module scope");

        assert_eq!(symbol.kind, SymbolKind::Variable);
        assert_eq!(symbol.name.as_str(), "secret");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn qualified_prepared_binding_does_not_report_undefined_base_identifier() {
        let source = r#"let root() = { Math.addOne(41) }"#;
        let parse = nx_syntax::parse_str(source, "qualified.nx");
        let tree = parse.tree.expect("Expected syntax tree");
        let mut prepared = PreparedModule::standalone(
            "qualified.nx",
            crate::lower(tree.root(), crate::SourceId::new(parse.source_id.as_u32())),
        );

        let imported_function = InterfaceItem {
            module_identity: "math/add.nx".to_string(),
            item_name: "addOne".to_string(),
            definition_id: LocalDefinitionId::new(0),
            visibility: Visibility::Export,
            item: InterfaceItemKind::Function {
                params: vec![InterfaceParam {
                    name: Name::new("n"),
                    ty: ast::TypeRef::name("int"),
                    is_content: false,
                    span: TextSpan::default(),
                }],
                return_type: ast::TypeRef::name("int"),
                span: TextSpan::default(),
            },
        };
        prepared.insert_binding(PreparedBinding {
            visible_name: Name::new("Math.addOne"),
            namespace: PreparedNamespace::Value,
            kind: PreparedItemKind::Function,
            origin: PreparedBindingOrigin::Imported {
                module_identity: imported_function.module_identity.clone(),
            },
            target: PreparedBindingTarget::Imported {
                item: imported_function,
            },
        });

        let (scopes, _) = build_scopes(&prepared);
        let diagnostics = check_undefined_identifiers(&prepared, &scopes);

        assert!(
            diagnostics.is_empty(),
            "Expected qualified prepared bindings to suppress undefined-base diagnostics, got {:?}",
            diagnostics
        );
    }
}
