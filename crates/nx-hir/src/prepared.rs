use crate::{
    ast, Component, ComponentEmit, EnumMember, Item, LoweredModule, LoweringDiagnostic, Name,
    Param, RecordField, RecordKind, SourceId, TypeAlias, Visibility,
};
use nx_diagnostics::TextSpan;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stable identity for one top-level definition inside a raw lowered module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalDefinitionId(u32);

impl LocalDefinitionId {
    /// Creates a new local definition identifier from an item index.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the underlying numeric value.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the raw item index for this definition.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Top-level namespace used by prepared binding lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreparedNamespace {
    Value,
    Type,
    Element,
}

/// Kind of top-level declaration reachable through prepared bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreparedItemKind {
    Function,
    Value,
    Component,
    TypeAlias,
    Enum,
    Record,
}

impl PreparedItemKind {
    /// Returns every prepared namespace this kind contributes to.
    pub fn namespaces(self) -> &'static [PreparedNamespace] {
        match self {
            PreparedItemKind::Function => &[PreparedNamespace::Value, PreparedNamespace::Element],
            PreparedItemKind::Value => &[PreparedNamespace::Value],
            PreparedItemKind::Component => &[PreparedNamespace::Element],
            PreparedItemKind::TypeAlias => &[PreparedNamespace::Type],
            PreparedItemKind::Enum => &[PreparedNamespace::Type],
            PreparedItemKind::Record => &[PreparedNamespace::Type, PreparedNamespace::Element],
        }
    }
}

/// Imported function parameter metadata published through a library interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceParam {
    pub name: Name,
    pub ty: ast::TypeRef,
    pub is_content: bool,
    pub span: TextSpan,
}

/// Imported field metadata published through a library interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceField {
    pub name: Name,
    pub ty: ast::TypeRef,
    pub is_content: bool,
    pub is_required: bool,
    pub span: TextSpan,
}

/// Optional back-reference from imported interface metadata to a raw lowered item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRawRef {
    pub module_identity: String,
    pub definition_id: LocalDefinitionId,
}

/// Interface-only representation of one published declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceItemKind {
    Function {
        params: Vec<InterfaceParam>,
        return_type: ast::TypeRef,
        span: TextSpan,
    },
    Value {
        ty: ast::TypeRef,
        span: TextSpan,
    },
    Component {
        is_abstract: bool,
        is_external: bool,
        base: Option<Name>,
        props: Vec<InterfaceField>,
        emits: Vec<ComponentEmit>,
        state: Vec<InterfaceField>,
        span: TextSpan,
    },
    TypeAlias {
        ty: ast::TypeRef,
        span: TextSpan,
    },
    Enum {
        members: Vec<EnumMember>,
        span: TextSpan,
    },
    Record {
        kind: RecordKind,
        is_abstract: bool,
        base: Option<Name>,
        properties: Vec<InterfaceField>,
        span: TextSpan,
    },
}

impl InterfaceItemKind {
    /// Returns the declaration kind described by this interface item.
    pub fn kind(&self) -> PreparedItemKind {
        match self {
            Self::Function { .. } => PreparedItemKind::Function,
            Self::Value { .. } => PreparedItemKind::Value,
            Self::Component { .. } => PreparedItemKind::Component,
            Self::TypeAlias { .. } => PreparedItemKind::TypeAlias,
            Self::Enum { .. } => PreparedItemKind::Enum,
            Self::Record { .. } => PreparedItemKind::Record,
        }
    }
}

/// Published interface metadata for one stable top-level definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceItem {
    pub module_identity: String,
    pub item_name: String,
    pub definition_id: LocalDefinitionId,
    pub visibility: Visibility,
    pub item: InterfaceItemKind,
}

/// Origin of one prepared visible binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedBindingOrigin {
    Local,
    Peer { module_identity: String },
    Imported { module_identity: String },
}

/// Stable target reached by a prepared binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedBindingTarget {
    Local {
        definition_id: LocalDefinitionId,
    },
    Peer {
        module_identity: String,
        definition_id: LocalDefinitionId,
    },
    Imported {
        item: InterfaceItem,
        raw: Option<ImportedRawRef>,
    },
}

/// One prepared visible binding in a single namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedBinding {
    pub visible_name: Name,
    pub namespace: PreparedNamespace,
    pub kind: PreparedItemKind,
    pub origin: PreparedBindingOrigin,
    pub target: PreparedBindingTarget,
}

impl PreparedBinding {
    /// Returns the stable target module identity for this binding.
    pub fn module_identity<'a>(&'a self, current_module_identity: &'a str) -> &'a str {
        match &self.target {
            PreparedBindingTarget::Local { .. } => current_module_identity,
            PreparedBindingTarget::Peer {
                module_identity, ..
            } => module_identity,
            PreparedBindingTarget::Imported { item, .. } => item.module_identity.as_str(),
        }
    }

    /// Returns the stable target definition identity for this binding.
    pub fn definition_id(&self) -> LocalDefinitionId {
        match &self.target {
            PreparedBindingTarget::Local { definition_id }
            | PreparedBindingTarget::Peer { definition_id, .. } => *definition_id,
            PreparedBindingTarget::Imported { item, .. } => item.definition_id,
        }
    }
}

/// One resolved prepared binding target used by later semantic phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedPreparedItem {
    Raw {
        module_identity: String,
        definition_id: LocalDefinitionId,
        item: Item,
        origin: PreparedBindingOrigin,
    },
    Imported {
        item: InterfaceItem,
        raw: Option<ImportedRawRef>,
        origin: PreparedBindingOrigin,
    },
}

impl ResolvedPreparedItem {
    /// Returns the kind of declaration this resolved item represents.
    pub fn kind(&self) -> PreparedItemKind {
        match self {
            ResolvedPreparedItem::Raw { item, .. } => prepared_item_kind(item),
            ResolvedPreparedItem::Imported { item, .. } => item.item.kind(),
        }
    }

    /// Returns the stable target module identity.
    pub fn module_identity(&self) -> &str {
        match self {
            ResolvedPreparedItem::Raw {
                module_identity, ..
            } => module_identity.as_str(),
            ResolvedPreparedItem::Imported { item, .. } => item.module_identity.as_str(),
        }
    }

    /// Returns the stable local definition identity within the owning module.
    pub fn definition_id(&self) -> LocalDefinitionId {
        match self {
            ResolvedPreparedItem::Raw { definition_id, .. } => *definition_id,
            ResolvedPreparedItem::Imported { item, .. } => item.definition_id,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct PreparedBindings {
    value: FxHashMap<Name, PreparedBinding>,
    ty: FxHashMap<Name, PreparedBinding>,
    element: FxHashMap<Name, PreparedBinding>,
}

impl PreparedBindings {
    fn map(&self, namespace: PreparedNamespace) -> &FxHashMap<Name, PreparedBinding> {
        match namespace {
            PreparedNamespace::Value => &self.value,
            PreparedNamespace::Type => &self.ty,
            PreparedNamespace::Element => &self.element,
        }
    }

    fn map_mut(&mut self, namespace: PreparedNamespace) -> &mut FxHashMap<Name, PreparedBinding> {
        match namespace {
            PreparedNamespace::Value => &mut self.value,
            PreparedNamespace::Type => &mut self.ty,
            PreparedNamespace::Element => &mut self.element,
        }
    }
}

/// Prepared analysis surface for one source file.
///
/// This preserves the raw file-local module while exposing explicit visible bindings for the
/// caller-prepared namespace used by semantic validation, scope building, and type analysis.
#[derive(Debug, Clone)]
pub struct PreparedModule {
    module_identity: String,
    raw_module: LoweredModule,
    bindings: PreparedBindings,
    peer_modules: FxHashMap<String, Arc<LoweredModule>>,
    diagnostics: Vec<LoweringDiagnostic>,
}

impl PreparedModule {
    /// Creates a prepared module with local bindings for the raw file-local definitions.
    pub fn new(module_identity: impl Into<String>, raw_module: LoweredModule) -> Self {
        let module_identity = module_identity.into();
        let mut prepared = Self {
            module_identity,
            raw_module,
            bindings: PreparedBindings::default(),
            peer_modules: FxHashMap::default(),
            diagnostics: Vec::new(),
        };
        prepared.add_local_bindings();
        prepared
    }

    /// Creates a trivial standalone prepared module from one raw lowered module.
    pub fn standalone(module_identity: impl Into<String>, raw_module: LoweredModule) -> Self {
        Self::new(module_identity, raw_module)
    }

    /// Returns the preserved raw module identity.
    pub fn module_identity(&self) -> &str {
        self.module_identity.as_str()
    }

    /// Returns the preserved raw lowered module.
    pub fn raw_module(&self) -> &LoweredModule {
        &self.raw_module
    }

    /// Returns mutable access to the preserved raw lowered module.
    pub fn raw_module_mut(&mut self) -> &mut LoweredModule {
        &mut self.raw_module
    }

    /// Consumes the prepared module and returns the preserved raw lowered module.
    pub fn into_raw_module(self) -> LoweredModule {
        self.raw_module
    }

    /// Returns the preserved source id.
    pub fn source_id(&self) -> SourceId {
        self.raw_module.source_id
    }

    /// Returns prepared binding diagnostics accumulated during namespace construction.
    pub fn diagnostics(&self) -> &[LoweringDiagnostic] {
        &self.diagnostics
    }

    /// Records one prepared-binding diagnostic.
    pub fn add_diagnostic(&mut self, diagnostic: LoweringDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Registers a peer raw module that local prepared bindings may resolve into.
    pub fn add_peer_module(
        &mut self,
        module_identity: impl Into<String>,
        module: Arc<LoweredModule>,
    ) {
        self.peer_modules.insert(module_identity.into(), module);
    }

    /// Inserts one prepared visible binding.
    pub fn insert_binding(&mut self, binding: PreparedBinding) {
        self.bindings
            .map_mut(binding.namespace)
            .insert(binding.visible_name.clone(), binding);
    }

    /// Returns true when a visible binding already exists in the given namespace.
    pub fn has_binding(&self, namespace: PreparedNamespace, name: &Name) -> bool {
        self.bindings.map(namespace).contains_key(name)
    }

    /// Resolves a visible binding by namespace and name.
    pub fn resolve_binding(
        &self,
        namespace: PreparedNamespace,
        name: &Name,
    ) -> Option<&PreparedBinding> {
        self.bindings.map(namespace).get(name)
    }

    /// Returns every visible binding in one namespace.
    pub fn bindings(&self, namespace: PreparedNamespace) -> impl Iterator<Item = &PreparedBinding> {
        self.bindings.map(namespace).values()
    }

    /// Resolves the stable target reached by one prepared binding.
    pub fn resolve_prepared_item(&self, binding: &PreparedBinding) -> Option<ResolvedPreparedItem> {
        match &binding.target {
            PreparedBindingTarget::Local { definition_id } => self
                .raw_module
                .item_by_definition(*definition_id)
                .cloned()
                .map(|item| ResolvedPreparedItem::Raw {
                    module_identity: self.module_identity.clone(),
                    definition_id: *definition_id,
                    item,
                    origin: binding.origin.clone(),
                }),
            PreparedBindingTarget::Peer {
                module_identity,
                definition_id,
            } => self
                .peer_modules
                .get(module_identity)
                .and_then(|module| module.item_by_definition(*definition_id).cloned())
                .map(|item| ResolvedPreparedItem::Raw {
                    module_identity: module_identity.clone(),
                    definition_id: *definition_id,
                    item,
                    origin: binding.origin.clone(),
                }),
            PreparedBindingTarget::Imported { item, raw } => Some(ResolvedPreparedItem::Imported {
                item: item.clone(),
                raw: raw.clone(),
                origin: binding.origin.clone(),
            }),
        }
    }

    /// Resolves an imported raw item reference back to a lowered item when available.
    pub fn resolve_imported_raw_item(&self, raw: &ImportedRawRef) -> Option<Item> {
        self.peer_modules
            .get(&raw.module_identity)
            .and_then(|module| module.item_by_definition(raw.definition_id).cloned())
    }

    fn add_local_bindings(&mut self) {
        let bindings = self
            .raw_module
            .items()
            .iter()
            .enumerate()
            .flat_map(|(index, item)| {
                let definition_id = LocalDefinitionId::new(index as u32);
                binding_specs_for_item(item)
                    .iter()
                    .copied()
                    .map(move |(namespace, kind)| PreparedBinding {
                        visible_name: item.name().clone(),
                        namespace,
                        kind,
                        origin: PreparedBindingOrigin::Local,
                        target: PreparedBindingTarget::Local { definition_id },
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for binding in bindings {
            self.insert_binding(binding);
        }
    }
}

/// Returns the stable local definition identity for one raw top-level item index.
pub fn local_definition_id(index: usize) -> LocalDefinitionId {
    LocalDefinitionId::new(index as u32)
}

/// Returns the prepared item kind for one raw HIR item.
pub fn prepared_item_kind(item: &Item) -> PreparedItemKind {
    match item {
        Item::Function(_) => PreparedItemKind::Function,
        Item::Value(_) => PreparedItemKind::Value,
        Item::Component(_) => PreparedItemKind::Component,
        Item::TypeAlias(_) => PreparedItemKind::TypeAlias,
        Item::Enum(_) => PreparedItemKind::Enum,
        Item::Record(_) => PreparedItemKind::Record,
    }
}

/// Returns the visible namespaces contributed by one raw HIR item.
pub fn binding_specs_for_item(item: &Item) -> Vec<(PreparedNamespace, PreparedItemKind)> {
    let kind = prepared_item_kind(item);
    kind.namespaces()
        .iter()
        .copied()
        .map(|namespace| (namespace, kind))
        .collect()
}

/// Converts imported interface metadata into a type-alias-like view when possible.
pub fn interface_type_alias(item: &InterfaceItem) -> Option<TypeAlias> {
    match &item.item {
        InterfaceItemKind::TypeAlias { ty, span } => Some(TypeAlias {
            name: Name::new(item.item_name.as_str()),
            visibility: item.visibility,
            ty: ty.clone(),
            span: *span,
        }),
        _ => None,
    }
}

/// Converts imported interface metadata into record-like view when possible.
pub fn interface_record(item: &InterfaceItem) -> Option<crate::RecordDef> {
    match &item.item {
        InterfaceItemKind::Record {
            kind,
            is_abstract,
            base,
            properties,
            span,
        } => Some(crate::RecordDef {
            name: Name::new(item.item_name.as_str()),
            visibility: item.visibility,
            kind: *kind,
            is_abstract: *is_abstract,
            base: base.clone(),
            properties: properties
                .iter()
                .map(|field| RecordField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    is_content: field.is_content,
                    default: None,
                    span: field.span,
                })
                .collect(),
            span: *span,
        }),
        _ => None,
    }
}

/// Converts imported interface metadata into component-like view when possible.
pub fn interface_component(item: &InterfaceItem) -> Option<Component> {
    match &item.item {
        InterfaceItemKind::Component {
            is_abstract,
            is_external,
            base,
            props,
            emits,
            state,
            span,
        } => Some(Component {
            name: Name::new(item.item_name.as_str()),
            visibility: item.visibility,
            is_abstract: *is_abstract,
            is_external: *is_external,
            base: base.clone(),
            props: props
                .iter()
                .map(|field| RecordField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    is_content: field.is_content,
                    default: None,
                    span: field.span,
                })
                .collect(),
            emits: emits.clone(),
            state: state
                .iter()
                .map(|field| RecordField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    is_content: field.is_content,
                    default: None,
                    span: field.span,
                })
                .collect(),
            body: None,
            span: *span,
        }),
        _ => None,
    }
}

/// Converts imported interface metadata into enum-like view when possible.
pub fn interface_enum(item: &InterfaceItem) -> Option<crate::EnumDef> {
    match &item.item {
        InterfaceItemKind::Enum { members, span } => Some(crate::EnumDef {
            name: Name::new(item.item_name.as_str()),
            visibility: item.visibility,
            members: members.clone(),
            span: *span,
        }),
        _ => None,
    }
}

/// Converts imported interface metadata into function signature view when possible.
pub fn interface_function_signature(
    item: &InterfaceItem,
) -> Option<(Name, Visibility, Vec<Param>, ast::TypeRef, TextSpan)> {
    match &item.item {
        InterfaceItemKind::Function {
            params,
            return_type,
            span,
        } => Some((
            Name::new(item.item_name.as_str()),
            item.visibility,
            params
                .iter()
                .map(|param| Param {
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    is_content: param.is_content,
                    span: param.span,
                })
                .collect(),
            return_type.clone(),
            *span,
        )),
        _ => None,
    }
}
