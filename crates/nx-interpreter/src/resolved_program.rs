use nx_hir::{LocalDefinitionId, LoweredModule};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stable identifier for one lowered module within a resolved program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeModuleId(u32);

impl RuntimeModuleId {
    /// Creates a new runtime module identifier.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the underlying numeric identifier.
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Runtime-visible item kinds indexed by a resolved program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedItemKind {
    Function,
    Value,
    Component,
    TypeAlias,
    Enum,
    Record,
}

/// Module-qualified reference to one runtime-visible item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleQualifiedItemRef {
    pub module_id: RuntimeModuleId,
    pub definition_id: LocalDefinitionId,
    pub kind: ResolvedItemKind,
}

/// Module-qualified reference to one lowered expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleQualifiedExprRef {
    pub module_id: RuntimeModuleId,
    pub expr_id: u32,
}

/// One lowered module preserved inside a resolved program.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub id: RuntimeModuleId,
    pub identity: String,
    pub lowered_module: Arc<LoweredModule>,
}

/// Runtime-facing resolved executable view built from multiple lowered modules.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub fingerprint: u64,
    root_modules: Vec<RuntimeModuleId>,
    modules: Vec<ResolvedModule>,
    local_items: FxHashMap<RuntimeModuleId, FxHashMap<String, ModuleQualifiedItemRef>>,
    pub entry_functions: FxHashMap<String, ModuleQualifiedItemRef>,
    pub entry_components: FxHashMap<String, ModuleQualifiedItemRef>,
    pub entry_records: FxHashMap<String, ModuleQualifiedItemRef>,
    pub entry_enums: FxHashMap<String, ModuleQualifiedItemRef>,
    pub imports: FxHashMap<RuntimeModuleId, FxHashMap<String, ModuleQualifiedItemRef>>,
}

impl ResolvedProgram {
    /// Creates a resolved program from its precomputed modules and lookup tables.
    pub fn new(
        fingerprint: u64,
        root_modules: Vec<RuntimeModuleId>,
        modules: Vec<ResolvedModule>,
        entry_functions: FxHashMap<String, ModuleQualifiedItemRef>,
        entry_components: FxHashMap<String, ModuleQualifiedItemRef>,
        entry_records: FxHashMap<String, ModuleQualifiedItemRef>,
        entry_enums: FxHashMap<String, ModuleQualifiedItemRef>,
        imports: FxHashMap<RuntimeModuleId, FxHashMap<String, ModuleQualifiedItemRef>>,
    ) -> Self {
        let local_items = build_local_items(&modules);
        Self {
            fingerprint,
            root_modules,
            modules,
            local_items,
            entry_functions,
            entry_components,
            entry_records,
            entry_enums,
            imports,
        }
    }

    /// Creates a resolved program containing exactly one root module.
    pub fn single_root_module(
        fingerprint: u64,
        identity: impl Into<String>,
        lowered_module: Arc<LoweredModule>,
    ) -> Self {
        let module_id = RuntimeModuleId::new(0);
        Self::new(
            fingerprint,
            vec![module_id],
            vec![ResolvedModule {
                id: module_id,
                identity: identity.into(),
                lowered_module,
            }],
            FxHashMap::default(),
            FxHashMap::default(),
            FxHashMap::default(),
            FxHashMap::default(),
            FxHashMap::default(),
        )
    }

    /// Returns the preserved root module identifiers for this program.
    pub fn root_modules(&self) -> &[RuntimeModuleId] {
        &self.root_modules
    }

    /// Returns every preserved lowered module in this resolved program.
    pub fn modules(&self) -> &[ResolvedModule] {
        &self.modules
    }

    /// Returns the resolved module for a module identifier, if present.
    pub fn module(&self, module_id: RuntimeModuleId) -> Option<&ResolvedModule> {
        self.modules.iter().find(|module| module.id == module_id)
    }

    /// Looks up an entrypoint function by name.
    pub fn entry_function(&self, name: &str) -> Option<&ModuleQualifiedItemRef> {
        self.entry_functions.get(name)
    }

    /// Looks up an entrypoint component by name.
    pub fn entry_component(&self, name: &str) -> Option<&ModuleQualifiedItemRef> {
        self.entry_components.get(name)
    }

    /// Looks up a local item owned by one preserved module.
    pub fn local_item(
        &self,
        module_id: RuntimeModuleId,
        visible_name: &str,
    ) -> Option<&ModuleQualifiedItemRef> {
        self.local_items
            .get(&module_id)
            .and_then(|items| items.get(visible_name))
    }

    /// Looks up an imported symbol visible from one module.
    pub fn imported_item(
        &self,
        module_id: RuntimeModuleId,
        visible_name: &str,
    ) -> Option<&ModuleQualifiedItemRef> {
        self.imports
            .get(&module_id)
            .and_then(|items| items.get(visible_name))
    }

    /// Returns every imported or peer-visible item prepared for one module.
    pub fn imported_items(
        &self,
        module_id: RuntimeModuleId,
    ) -> Option<&FxHashMap<String, ModuleQualifiedItemRef>> {
        self.imports.get(&module_id)
    }
}

fn build_local_items(
    modules: &[ResolvedModule],
) -> FxHashMap<RuntimeModuleId, FxHashMap<String, ModuleQualifiedItemRef>> {
    let mut local_items = FxHashMap::default();

    for module in modules {
        let mut items = FxHashMap::default();
        for (index, item) in module.lowered_module.items().iter().enumerate() {
            items
                .entry(item.name().as_str().to_string())
                .or_insert_with(|| ModuleQualifiedItemRef {
                    module_id: module.id,
                    definition_id: LocalDefinitionId::new(index as u32),
                    kind: resolved_item_kind(item),
                });
        }
        local_items.insert(module.id, items);
    }

    local_items
}

fn resolved_item_kind(item: &nx_hir::Item) -> ResolvedItemKind {
    match item {
        nx_hir::Item::Function(_) => ResolvedItemKind::Function,
        nx_hir::Item::Value(_) => ResolvedItemKind::Value,
        nx_hir::Item::Component(_) => ResolvedItemKind::Component,
        nx_hir::Item::TypeAlias(_) => ResolvedItemKind::TypeAlias,
        nx_hir::Item::Enum(_) => ResolvedItemKind::Enum,
        nx_hir::Item::Record(_) => ResolvedItemKind::Record,
    }
}
