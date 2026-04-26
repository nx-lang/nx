use nx_hir::{LocalDefinitionId, LoweredModule};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    Union,
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
    pub source: ResolvedModuleSource,
    pub lowered_module: Arc<LoweredModule>,
}

/// Provenance for a lowered module preserved inside a resolved program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedModuleSource {
    /// A source-provider module, keyed by a normalized logical identity.
    SourceProvider { identity: String },
    /// A module loaded from a library artifact on disk.
    Library {
        root_path: PathBuf,
        module_path: PathBuf,
    },
}

impl ResolvedModuleSource {
    pub fn source_provider_identity(&self) -> Option<&str> {
        match self {
            Self::SourceProvider { identity } => Some(identity),
            Self::Library { .. } => None,
        }
    }

    pub fn module_path(&self) -> Option<&Path> {
        match self {
            Self::SourceProvider { .. } => None,
            Self::Library { module_path, .. } => Some(module_path.as_path()),
        }
    }

    pub fn prepared_module_identity(&self) -> String {
        match self {
            Self::SourceProvider { identity } => identity.clone(),
            Self::Library { module_path, .. } => module_path.display().to_string(),
        }
    }
}

impl ResolvedModule {
    pub fn source_provider_identity(&self) -> Option<&str> {
        self.source.source_provider_identity()
    }

    pub fn prepared_module_identity(&self) -> String {
        self.source.prepared_module_identity()
    }
}

/// Runtime-facing resolved executable view built from multiple lowered modules.
#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub fingerprint: u64,
    root_modules: Vec<RuntimeModuleId>,
    modules: Vec<ResolvedModule>,
    source_provider_modules: FxHashMap<String, RuntimeModuleId>,
    prepared_modules: FxHashMap<String, RuntimeModuleId>,
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
        let source_provider_modules = build_source_provider_modules(&modules);
        let prepared_modules = build_prepared_modules(&modules);
        let local_items = build_local_items(&modules);
        Self {
            fingerprint,
            root_modules,
            modules,
            source_provider_modules,
            prepared_modules,
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
                source: ResolvedModuleSource::SourceProvider {
                    identity: identity.into(),
                },
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

    /// Returns the source-provider module identifier with the supplied logical identity.
    pub fn source_provider_module_id(&self, identity: &str) -> Option<RuntimeModuleId> {
        self.source_provider_modules.get(identity).copied()
    }

    /// Returns the source-provider module with the supplied logical identity, if present.
    pub fn module_by_source_provider_identity(&self, identity: &str) -> Option<&ResolvedModule> {
        self.source_provider_module_id(identity)
            .and_then(|module_id| self.module(module_id))
    }

    /// Returns the module with the supplied prepared-module identity, if present.
    ///
    /// Prepared-module identities are only used to reconnect lowered peer bindings while
    /// interpreting an already resolved program. Runtime entrypoint dispatch should use
    /// [`RuntimeModuleId`] instead.
    pub fn module_by_prepared_identity(&self, identity: &str) -> Option<&ResolvedModule> {
        self.prepared_modules
            .get(identity)
            .and_then(|module_id| self.module(*module_id))
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

fn build_source_provider_modules(modules: &[ResolvedModule]) -> FxHashMap<String, RuntimeModuleId> {
    let mut source_provider_modules = FxHashMap::default();

    for module in modules {
        if let Some(identity) = module.source_provider_identity() {
            source_provider_modules
                .entry(identity.to_string())
                .or_insert(module.id);
        }
    }

    source_provider_modules
}

fn build_prepared_modules(modules: &[ResolvedModule]) -> FxHashMap<String, RuntimeModuleId> {
    let mut prepared_modules = FxHashMap::default();

    for module in modules {
        prepared_modules
            .entry(module.prepared_module_identity())
            .or_insert(module.id);
    }

    prepared_modules
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
        nx_hir::Item::Union(_) => ResolvedItemKind::Union,
        nx_hir::Item::Record(_) => ResolvedItemKind::Record,
    }
}
