use nx_api::{
    build_program_artifact_from_source, LibraryArtifact, LibraryRegistry, ProgramBuildContext,
};
use nx_hir::{
    ast::{Expr, Literal, OrderedFloat, TypeRef},
    Component, ImportKind, InterfaceItemKind, Item, LocalDefinitionId, LoweredModule,
    ModuleNamespace, PreparedItemKind, PreparedModule, PreparedNamespace, RecordDef, RecordField,
    RecordKind, SelectiveImport, TypeAlias, UnionCaseDef, UnionCaseField, UnionDef, Visibility,
};
use nx_types::ModuleArtifact;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedAlias {
    pub name: String,
    pub target: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedRecordField {
    pub name: String,
    pub ty: TypeRef,
    pub default_value: Option<ExportedFieldDefault>,
    /// Identity of the module that declared the field, when it is not the module that owns the
    /// exported declaration carrying it.
    ///
    /// <para>Only an update companion copies fields from another module, so only its fields ever
    /// carry one. The field's type was written in that module's namespace, which is where it has
    /// to be resolved.</para>
    pub declaring_module: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportedFieldDefault {
    Literal(ExportedLiteralDefault),
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportedLiteralDefault {
    String(String),
    Int(i64),
    Float(OrderedFloat),
    Boolean(bool),
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedRecord {
    pub name: String,
    pub kind: RecordKind,
    pub is_abstract: bool,
    pub base: Option<String>,
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedUnionCase {
    pub name: String,
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedUnion {
    pub name: String,
    pub base: Option<String>,
    pub cases: Vec<ExportedUnionCase>,
    /// The record, action, or component this is the generated `<Target>_property` companion of.
    ///
    /// <para>A property companion is the derived `<Target>.Property` union under its generated
    /// name: one constant case per effective field, so it generates exactly what a constant union
    /// generates.</para>
    pub property_target: Option<String>,
}

impl ExportedUnion {
    /// Whether `case` declares no fields in a union that declares no base, so it carries nothing
    /// beyond its own name.
    pub fn is_constant_case(&self, case: &ExportedUnionCase) -> bool {
        self.base.is_none() && case.fields.is_empty()
    }

    /// Whether every case is constant. This is what an `enum` declared, and it generates a CLR
    /// `enum` in C# and an `as const` object in TypeScript.
    pub fn is_constant(&self) -> bool {
        !self.cases.is_empty()
            && self.base.is_none()
            && self.cases.iter().all(|case| case.fields.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExportedPolymorphicDescendant {
    Record {
        name: String,
    },
    UnionCase {
        union_name: String,
        case_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedExternalState {
    pub component_name: String,
    pub name: String,
    pub fields: Vec<ExportedRecordField>,
}

/// The generated `<Target>_update` companion of an exported record, action, or stateful component.
///
/// <para>It has the target's effective fields — a component's state fields — every one optional,
/// and carries the `<Target>.Update` discriminator. An absent field means "unchanged" and a present
/// `null` means "set to null", so each host surface keeps the two apart.</para>
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedUpdate {
    pub target_name: String,
    pub name: String,
    pub discriminator: String,
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportedTypeKind {
    Alias {
        target: TypeRef,
        target_is_reference: bool,
    },
    Union {
        /// Whether every case of this union is constant, which decides its host mapping.
        is_constant: bool,
    },
    Record,
    Component,
}

impl ImportedTypeKind {
    pub fn is_reference(&self) -> bool {
        match self {
            Self::Alias {
                target_is_reference,
                ..
            } => *target_is_reference,
            Self::Union { is_constant } => !is_constant,
            Self::Record | Self::Component => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedType {
    pub visible_name: String,
    pub exported_name: String,
    pub library_name: String,
    pub kind: ImportedTypeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportedType {
    Alias(ExportedAlias),
    Union(ExportedUnion),
    Record(ExportedRecord),
    ExternalState(ExportedExternalState),
    Update(ExportedUpdate),
}

impl ExportedType {
    pub fn name(&self) -> &str {
        match self {
            Self::Alias(alias) => &alias.name,
            Self::Union(union_def) => &union_def.name,
            Self::Record(record) => &record.name,
            Self::ExternalState(state) => &state.name,
            Self::Update(update) => &update.name,
        }
    }

    /// Returns the warning subject for a companion typegen generates rather than the author
    /// declared, or `None` for a declared item.
    ///
    /// <para>A generated companion yields to a declaration of the same name, and two companions
    /// that would share a name are both skipped, so neither silently shadows the other.</para>
    fn generated_companion(&self) -> Option<String> {
        match self {
            Self::ExternalState(state) => Some(format!(
                "component state contract '{}' for external component '{}'",
                state.name, state.component_name
            )),
            Self::Update(update) => Some(format!(
                "update record '{}' for '{}'",
                update.name, update.target_name
            )),
            Self::Union(union_def) => union_def
                .property_target
                .as_ref()
                .map(|target| format!("property union '{}' for '{}'", union_def.name, target)),
            Self::Alias(_) | Self::Record(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypeDecl {
    pub visibility: Visibility,
    pub item: ExportedType,
}

impl ExportedTypeDecl {
    pub fn name(&self) -> &str {
        self.item.name()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedModule {
    pub module_path: PathBuf,
    pub imported_types: Vec<ImportedType>,
    pub declarations: Vec<ExportedTypeDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypeGraph {
    pub modules: Vec<ExportedModule>,
    owners: FxHashMap<String, PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedTypeGraphBuild {
    pub graph: ExportedTypeGraph,
    pub warnings: Vec<String>,
}

#[derive(Default)]
struct ImportedTypeCollector {
    /// Loads and caches every imported library, so the same registry can back the build context
    /// the source module is prepared against.
    registry: LibraryRegistry,
    dependency_cache: FxHashMap<PathBuf, Result<Arc<CachedImportedLibrary>, String>>,
    /// The canonical root of the library each loaded module identity belongs to.
    ///
    /// <para>Membership in a library's namespaces decides ownership, so the mapping does not
    /// depend on how canonical paths are spelled.</para>
    library_roots_by_module_identity: FxHashMap<String, PathBuf>,
    /// The canonical root of the library being generated, in library mode.
    ///
    /// <para>A declaration this library owns is a peer the graph already declares, so a reference
    /// to it needs no imported type.</para>
    generating_library_root: Option<PathBuf>,
}

struct ImportedTypesBuild {
    imported_types: Vec<ImportedType>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CachedImportedLibrary {
    library_name: String,
    export_kinds: FxHashMap<String, PreparedItemKind>,
    /// Exported derived declarations — `<Target>.Update` and `<Target>.Property` — which the
    /// dependency generates as companions and an import of the target brings along.
    derived_exports: FxHashSet<String>,
    /// Exported names whose union declares no base and no case fields.
    constant_unions: FxHashSet<String>,
    alias_targets: FxHashMap<String, TypeRef>,
    wildcard_importable_export_names: Vec<String>,
    /// The namespace each of the library's modules resolves type names in, keyed by module
    /// identity.
    namespaces: FxHashMap<String, Arc<ModuleNamespace>>,
    /// Every interface item of the library, keyed by the origin a namespace entry resolves to.
    items_by_origin: FxHashMap<(String, LocalDefinitionId), CachedInterfaceItem>,
}

/// What origin lookup needs to know about one declaration of a loaded library.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CachedInterfaceItem {
    name: String,
    visibility: Visibility,
    kind: PreparedItemKind,
}

#[derive(Clone)]
struct ImportedVisibleNameOrigin {
    library_path: String,
    exported_name: String,
}

impl ExportedTypeGraph {
    /// Builds the graph for one source file, resolving its imports the way the compiler does.
    ///
    /// <para>The file is analyzed against a build context holding every library it imports, so
    /// the prepared module on the resulting artifact resolves cross-library references — a
    /// record's inherited fields from a library base, say — exactly as the checker sees them.</para>
    pub fn from_source_with_warnings(
        source: &str,
        source_path: &Path,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let file_name = source_path.display().to_string();
        let module = nx_hir::lower_source_module(source, &file_name).map_err(|diagnostics| {
            let messages = diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message().to_string())
                .collect::<Vec<_>>();
            format!("Failed to lower '{}': {}", file_name, messages.join("; "))
        })?;
        let mut imported_type_collector = ImportedTypeCollector::default();
        let imported_build = imported_type_collector.collect_for_module(&module, source_path);
        let build_context = ProgramBuildContext::from_registry(&imported_type_collector.registry);
        let program = build_program_artifact_from_source(source, &file_name, &build_context)
            .map_err(|error| format!("Failed to analyze '{}': {}", file_name, error))?;
        let artifact = program
            .root_modules
            .into_iter()
            .next()
            .ok_or_else(|| format!("Analysis of '{}' produced no module", file_name))?;
        Self::from_artifact_with_warnings(
            &artifact,
            source_path,
            imported_build,
            &imported_type_collector,
        )
    }

    #[allow(dead_code)]
    pub fn from_module(module: &ModuleArtifact, source_path: &Path) -> Result<Self, String> {
        Ok(Self::from_module_with_warnings(module, source_path)?.graph)
    }

    /// Builds the graph for an already analyzed module, loading its imports for their names only.
    pub fn from_module_with_warnings(
        module: &ModuleArtifact,
        source_path: &Path,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let mut imported_type_collector = ImportedTypeCollector::default();
        let imported_build = match module.lowered_module.as_deref() {
            Some(lowered) => imported_type_collector.collect_for_module(lowered, source_path),
            None => ImportedTypesBuild {
                imported_types: Vec::new(),
                warnings: Vec::new(),
            },
        };
        Self::from_artifact_with_warnings(
            module,
            source_path,
            imported_build,
            &imported_type_collector,
        )
    }

    fn from_artifact_with_warnings(
        module: &ModuleArtifact,
        source_path: &Path,
        imported_build: ImportedTypesBuild,
        imported_type_collector: &ImportedTypeCollector,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let file_name = source_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("types.nx"));
        let module_path = module_output_stem(&file_name)?;
        let mut modules = vec![ExportedModule {
            module_path: module_path.clone(),
            imported_types: imported_build.imported_types.clone(),
            declarations: collect_exported_declarations(module),
        }];
        let mut warnings = Vec::new();
        imported_type_collector.resolve_inherited_field_types(&mut modules, &mut warnings);
        let mut build = Self::build_from_exported_modules(modules)?;

        if build.graph.modules.is_empty() {
            build.graph.modules.push(ExportedModule {
                module_path,
                imported_types: imported_build.imported_types,
                declarations: Vec::new(),
            });
        }

        build.warnings.extend(imported_build.warnings);
        build.warnings.extend(warnings);
        Ok(build)
    }

    #[allow(dead_code)]
    pub fn from_library(library: &LibraryArtifact) -> Result<Self, String> {
        Ok(Self::from_library_with_warnings(library)?.graph)
    }

    pub fn from_library_with_warnings(
        library: &LibraryArtifact,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let mut imported_type_collector = ImportedTypeCollector::default();
        imported_type_collector.register_generating_library(library)?;
        let mut modules = Vec::new();
        let mut warnings = Vec::new();

        for artifact in &library.modules {
            let Some(module) = artifact.lowered_module.as_ref() else {
                continue;
            };

            let declarations = collect_exported_declarations(artifact);
            if declarations.is_empty() {
                continue;
            }

            let module_file = Path::new(&artifact.file_name);
            let relative_path = module_file.strip_prefix(&library.root_path).map_err(|_| {
                format!(
                    "Module '{}' is not located under library root '{}'",
                    module_file.display(),
                    library.root_path.display()
                )
            })?;
            let imported_build = imported_type_collector.collect_for_module(module, module_file);
            warnings.extend(imported_build.warnings);

            modules.push(ExportedModule {
                module_path: module_output_stem(relative_path)?,
                imported_types: imported_build.imported_types,
                declarations,
            });
        }

        modules.sort_by(|left, right| left.module_path.cmp(&right.module_path));
        imported_type_collector.resolve_inherited_field_types(&mut modules, &mut warnings);
        let mut build = Self::build_from_exported_modules(modules)?;
        build.warnings.extend(warnings);
        Ok(build)
    }

    pub fn owner_module(&self, type_name: &str) -> Option<&Path> {
        self.owners.get(type_name).map(PathBuf::as_path)
    }

    pub fn declaration(&self, type_name: &str) -> Option<&ExportedTypeDecl> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .find(|declaration| declaration.name() == type_name)
    }

    pub fn record(&self, type_name: &str) -> Option<&ExportedRecord> {
        match &self.declaration(type_name)?.item {
            ExportedType::Record(record) => Some(record),
            _ => None,
        }
    }

    pub fn resolve_record(&self, type_name: &str) -> Option<&ExportedRecord> {
        match &self
            .resolve_named_type_declaration(type_name, &mut BTreeSet::new())?
            .item
        {
            ExportedType::Record(record) => Some(record),
            _ => None,
        }
    }

    pub fn resolved_record_base<'a>(
        &'a self,
        record: &ExportedRecord,
    ) -> Option<&'a ExportedRecord> {
        let base_name = record.base.as_deref()?;
        self.resolve_record(base_name)
    }

    pub fn concrete_descendants<'a>(&'a self, type_name: &str) -> Vec<&'a ExportedRecord> {
        let Some(record) = self.record(type_name) else {
            return Vec::new();
        };
        if !record.is_abstract {
            return Vec::new();
        }

        let mut names = BTreeSet::new();
        self.collect_concrete_descendant_names(type_name, &mut names);
        names
            .into_iter()
            .filter_map(|name| self.record(&name))
            .collect()
    }

    pub fn polymorphic_descendants(&self, type_name: &str) -> Vec<ExportedPolymorphicDescendant> {
        let Some(record) = self.record(type_name) else {
            return Vec::new();
        };
        if !record.is_abstract {
            return Vec::new();
        }

        let mut descendants = BTreeSet::new();
        for record in self.concrete_descendants(type_name) {
            descendants.insert(ExportedPolymorphicDescendant::Record {
                name: record.name.clone(),
            });
        }

        for union_def in self.unions() {
            if !self.union_extends_record(union_def, type_name) {
                continue;
            }

            for case in &union_def.cases {
                descendants.insert(ExportedPolymorphicDescendant::UnionCase {
                    union_name: union_def.name.clone(),
                    case_name: case.name.clone(),
                });
            }
        }

        descendants.into_iter().collect()
    }

    fn build_from_exported_modules(
        modules: Vec<ExportedModule>,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let mut explicit_owners = FxHashMap::default();
        let mut generated_candidates = FxHashMap::<String, Vec<(usize, usize)>>::default();
        let mut warnings = Vec::new();

        for (module_index, module) in modules.iter().enumerate() {
            for (declaration_index, declaration) in module.declarations.iter().enumerate() {
                let name = declaration.name().to_string();
                if declaration.item.generated_companion().is_some() {
                    generated_candidates
                        .entry(name)
                        .or_default()
                        .push((module_index, declaration_index));
                } else {
                    explicit_owners
                        .entry(name)
                        .or_insert_with(|| module.module_path.clone());
                }
            }
        }

        let mut generated_names_to_skip = FxHashSet::default();
        let mut candidate_names = generated_candidates.keys().cloned().collect::<Vec<_>>();
        candidate_names.sort();
        for name in candidate_names {
            let candidates = &generated_candidates[&name];
            let reason = if explicit_owners.contains_key(&name) {
                format!("it conflicts with exported declaration '{}'", name)
            } else if candidates.len() > 1 {
                "another exported declaration would generate the same companion name".to_string()
            } else {
                continue;
            };

            generated_names_to_skip.insert(name.clone());
            for (module_index, declaration_index) in candidates {
                let Some(subject) = modules[*module_index]
                    .declarations
                    .get(*declaration_index)
                    .and_then(|declaration| declaration.item.generated_companion())
                else {
                    continue;
                };
                warnings.push(format!("Skipping generated {} because {}", subject, reason));
            }
        }

        let mut owners = FxHashMap::default();
        let mut filtered_modules = Vec::with_capacity(modules.len());

        for module in modules {
            let declarations = module
                .declarations
                .into_iter()
                .filter(|declaration| {
                    declaration.item.generated_companion().is_none()
                        || !generated_names_to_skip.contains(declaration.name())
                })
                .collect::<Vec<_>>();

            if declarations.is_empty() {
                continue;
            }

            for declaration in &declarations {
                if declaration.item.generated_companion().is_none() {
                    owners
                        .entry(declaration.name().to_string())
                        .or_insert_with(|| module.module_path.clone());
                }
            }

            filtered_modules.push(ExportedModule {
                module_path: module.module_path,
                imported_types: module.imported_types,
                declarations,
            });
        }

        for module in &filtered_modules {
            for declaration in &module.declarations {
                if declaration.item.generated_companion().is_some() {
                    owners
                        .entry(declaration.name().to_string())
                        .or_insert_with(|| module.module_path.clone());
                }
            }
        }

        let mut graph = Self {
            modules: filtered_modules,
            owners,
        };
        graph.rewrite_update_type_references(&mut warnings);
        Ok(ExportedTypeGraphBuild { graph, warnings })
    }

    /// Points every `<T>.Update` and `<T>.Property` type reference at the `<T>_update` or
    /// `<T>_property` companion that stands for it.
    ///
    /// <para>A field typed `User.Update` must generate as the companion, since that is the only
    /// declaration of the patch either language sees, and a field typed `User.Property` likewise.
    /// A derived name reached through an import is left for the emitters, which import the
    /// dependency's companion. Where no companion exists — the target is not exported, or the
    /// companion name was taken by an explicit declaration — the reference is left as written and a
    /// warning says why the generated code will not resolve it.</para>
    fn rewrite_update_type_references(&mut self, warnings: &mut Vec<String>) {
        let companions = self
            .modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .filter_map(|declaration| match &declaration.item {
                ExportedType::Update(update) => {
                    Some((update.discriminator.clone(), update.name.clone()))
                }
                ExportedType::Union(union_def) => {
                    union_def.property_target.as_ref().map(|target| {
                        (
                            nx_hir::property_union_name(target).as_str().to_string(),
                            union_def.name.clone(),
                        )
                    })
                }
                _ => None,
            })
            .collect::<FxHashMap<_, _>>();
        let imported = self
            .modules
            .iter()
            .flat_map(|module| module.imported_types.iter())
            .map(|imported_type| imported_type.visible_name.clone())
            .collect::<FxHashSet<_>>();

        let mut unresolved = BTreeSet::new();
        let mut rename = |ty: &mut TypeRef| {
            rewrite_type_ref_names(ty, &mut |name| {
                if !nx_hir::is_update_record_name(name) && !nx_hir::is_property_union_name(name) {
                    return None;
                }
                match companions.get(name) {
                    Some(companion) => Some(companion.clone()),
                    None if imported.contains(name) => None,
                    None => {
                        unresolved.insert(name.to_string());
                        None
                    }
                }
            });
        };

        for module in &mut self.modules {
            for declaration in &mut module.declarations {
                match &mut declaration.item {
                    ExportedType::Alias(alias) => rename(&mut alias.target),
                    ExportedType::Union(union_def) => {
                        for case in &mut union_def.cases {
                            for field in &mut case.fields {
                                rename(&mut field.ty);
                            }
                        }
                    }
                    ExportedType::Record(record) => {
                        for field in &mut record.fields {
                            rename(&mut field.ty);
                        }
                    }
                    ExportedType::ExternalState(state) => {
                        for field in &mut state.fields {
                            rename(&mut field.ty);
                        }
                    }
                    ExportedType::Update(update) => {
                        for field in &mut update.fields {
                            rename(&mut field.ty);
                        }
                    }
                }
            }
        }

        for name in unresolved {
            let (target, suffix) = name
                .rsplit_once('.')
                .map(|(target, suffix)| (target, suffix.to_ascii_lowercase()))
                .unwrap_or((&name, String::new()));
            warnings.push(format!(
                "Type reference '{}' has no generated companion '{}_{}' to resolve to; the generated code will not compile until '{}' is exported and the companion name is free",
                name, target, suffix, target
            ));
        }
    }

    fn resolve_named_type_declaration<'a>(
        &'a self,
        type_name: &str,
        seen_aliases: &mut BTreeSet<String>,
    ) -> Option<&'a ExportedTypeDecl> {
        let declaration = self.declaration(type_name)?;
        match &declaration.item {
            ExportedType::Alias(alias) => {
                let TypeRef::Name(target_name) = &alias.target else {
                    return Some(declaration);
                };

                if !seen_aliases.insert(type_name.to_string()) {
                    return None;
                }

                let resolved =
                    self.resolve_named_type_declaration(target_name.as_str(), seen_aliases);
                seen_aliases.remove(type_name);
                resolved
            }
            _ => Some(declaration),
        }
    }

    fn collect_concrete_descendant_names(&self, type_name: &str, out: &mut BTreeSet<String>) {
        for record in self.records() {
            let Some(base_record) = self.resolved_record_base(record) else {
                continue;
            };

            if base_record.name != type_name {
                continue;
            }

            if record.is_abstract {
                self.collect_concrete_descendant_names(&record.name, out);
            } else {
                out.insert(record.name.clone());
            }
        }
    }

    fn union_extends_record(&self, union_def: &ExportedUnion, type_name: &str) -> bool {
        let Some(base_name) = union_def.base.as_deref() else {
            return false;
        };
        let Some(mut current) = self.resolve_record(base_name) else {
            return false;
        };

        loop {
            if current.name == type_name {
                return true;
            }

            let Some(base_record) = self.resolved_record_base(current) else {
                return false;
            };
            current = base_record;
        }
    }

    fn records(&self) -> impl Iterator<Item = &ExportedRecord> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .filter_map(|declaration| match &declaration.item {
                ExportedType::Record(record) => Some(record),
                _ => None,
            })
    }

    fn unions(&self) -> impl Iterator<Item = &ExportedUnion> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .filter_map(|declaration| match &declaration.item {
                ExportedType::Union(union_def) => Some(union_def),
                _ => None,
            })
    }
}

fn collect_exported_declarations(artifact: &ModuleArtifact) -> Vec<ExportedTypeDecl> {
    let mut declarations = Vec::new();
    let Some(module) = artifact.lowered_module.as_deref() else {
        return declarations;
    };
    let prepared = artifact.prepared_module.as_deref();

    for item in module.items() {
        if item.visibility() != Visibility::Export {
            continue;
        }

        match item {
            Item::TypeAlias(alias) => declarations.push(ExportedTypeDecl {
                visibility: alias.visibility,
                item: ExportedType::Alias(export_alias(alias)),
            }),
            // A derived property union is exported as its `<Target>_property` companion, never
            // under its own dotted name.
            Item::Union(union_def) => declarations.push(ExportedTypeDecl {
                visibility: union_def.visibility,
                item: ExportedType::Union(match union_def.property_target() {
                    Some(target) => export_property_union(union_def, target),
                    None => export_union(module, union_def),
                }),
            }),
            // A derived update record is exported as its `<Target>_update` companion, never under
            // its own dotted name.
            Item::Record(record) => match record.update_target() {
                Some(target) => declarations.push(ExportedTypeDecl {
                    visibility: record.visibility,
                    item: ExportedType::Update(export_update(prepared, record, target)),
                }),
                None => declarations.push(ExportedTypeDecl {
                    visibility: record.visibility,
                    item: ExportedType::Record(export_record(module, record)),
                }),
            },
            Item::Component(component) => {
                if let Some(record) = export_external_component_contract(module, component) {
                    declarations.push(ExportedTypeDecl {
                        visibility: component.visibility,
                        item: ExportedType::Record(record),
                    });
                }

                if let Some(state) = export_external_state(component) {
                    declarations.push(ExportedTypeDecl {
                        visibility: component.visibility,
                        item: ExportedType::ExternalState(state),
                    });
                }
            }
            _ => {}
        };
    }

    declarations
}

impl ImportedTypeCollector {
    fn collect_for_module(
        &mut self,
        module: &LoweredModule,
        source_path: &Path,
    ) -> ImportedTypesBuild {
        let mut imported_types = Vec::new();
        let mut warnings = Vec::new();
        let mut seen_visible_names = FxHashMap::<String, ImportedVisibleNameOrigin>::default();
        let mut dependencies_by_visible_name =
            FxHashMap::<String, Arc<CachedImportedLibrary>>::default();

        for import in &module.imports {
            let dependency_root = match resolve_dependency_root(source_path, &import.library_path) {
                Ok(path) => path,
                Err(error) => {
                    warnings.push(format!(
                        "Could not resolve imported library '{}' from '{}': {}",
                        import.library_path,
                        source_path.display(),
                        error
                    ));
                    continue;
                }
            };
            let dependency = match self.load_dependency(&dependency_root) {
                Ok(dependency) => dependency,
                Err(error) => {
                    warnings.push(format!(
                        "Could not analyze imported library '{}' resolved from '{}': {}",
                        import.library_path,
                        source_path.display(),
                        error
                    ));
                    continue;
                }
            };

            match &import.kind {
                ImportKind::Wildcard { alias } => {
                    for exported_name in &dependency.wildcard_importable_export_names {
                        let visible_name = match alias {
                            Some(alias) => format!("{}.{}", alias.as_str(), exported_name),
                            None => exported_name.clone(),
                        };

                        if let Some(origin) = seen_visible_names.get(&visible_name) {
                            warnings.push(imported_visible_name_collision_warning(
                                source_path,
                                &visible_name,
                                origin,
                                &import.library_path,
                                exported_name,
                            ));
                            continue;
                        }
                        seen_visible_names.insert(
                            visible_name.clone(),
                            ImportedVisibleNameOrigin {
                                library_path: import.library_path.clone(),
                                exported_name: exported_name.clone(),
                            },
                        );

                        if let Some(imported_type) =
                            dependency.imported_type(&visible_name, exported_name)
                        {
                            imported_types.push(imported_type);
                            dependencies_by_visible_name
                                .insert(visible_name.clone(), dependency.clone());
                        }
                    }
                }
                ImportKind::Selective { entries } => {
                    for entry in entries {
                        let exported_name = entry.name.as_str().to_string();
                        let visible_name = visible_name_for_selective_import(entry);

                        if let Some(origin) = seen_visible_names.get(&visible_name) {
                            warnings.push(imported_visible_name_collision_warning(
                                source_path,
                                &visible_name,
                                origin,
                                &import.library_path,
                                &exported_name,
                            ));
                            continue;
                        }
                        seen_visible_names.insert(
                            visible_name.clone(),
                            ImportedVisibleNameOrigin {
                                library_path: import.library_path.clone(),
                                exported_name: exported_name.clone(),
                            },
                        );

                        match dependency.imported_type(&visible_name, &exported_name) {
                            Some(imported_type) => {
                                imported_types.push(imported_type);
                                dependencies_by_visible_name
                                    .insert(visible_name.clone(), dependency.clone());
                            }
                            None => warnings.push(dependency.unsupported_import_warning(
                                source_path,
                                &import.library_path,
                                &exported_name,
                            )),
                        }
                    }
                }
            }
        }

        // A derived name on an imported declaration — `User.Update`, `User.Property` — names
        // the dependency's companion. Only the ones the module actually references are imported,
        // so a module that imports `User` and never patches it imports exactly what it did before.
        let mut referenced_names = BTreeSet::new();
        for item in module.items() {
            for ty in nx_hir::item_type_refs(item) {
                referenced_names.extend(type_ref_name_strings(ty));
            }
        }
        for name in referenced_names {
            let Some((visible_name, suffix)) = name.rsplit_once('.') else {
                continue;
            };
            let Some(dependency) = dependencies_by_visible_name.get(visible_name) else {
                continue;
            };
            let Some(exported_name) = imported_types
                .iter()
                .find(|imported_type| imported_type.visible_name == visible_name)
                .map(|imported_type| imported_type.exported_name.clone())
            else {
                continue;
            };
            if let Some(derived) =
                dependency.derived_imported_type(visible_name, &exported_name, suffix)
            {
                if !imported_types
                    .iter()
                    .any(|imported_type| imported_type.visible_name == derived.visible_name)
                {
                    imported_types.push(derived);
                }
            }
        }

        ImportedTypesBuild {
            imported_types,
            warnings,
        }
    }

    /// Loads a library and, through the registry, everything it depends on.
    ///
    /// <para>The transitive dependencies are cached too, since a base declared in a dependency may
    /// type a field by something *its* dependency exports.</para>
    fn load_dependency(
        &mut self,
        dependency_root: &Path,
    ) -> Result<Arc<CachedImportedLibrary>, String> {
        if let Some(cached) = self.dependency_cache.get(dependency_root) {
            return cached.clone();
        }

        let loaded = self
            .registry
            .load_library_artifact(dependency_root)
            .map_err(|error| {
                format!(
                    "failed to build library artifact for '{}': {}",
                    dependency_root.display(),
                    error
                )
            });
        let cached = loaded
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|dependency| self.cache_library(dependency, dependency_root));
        self.dependency_cache
            .insert(dependency_root.to_path_buf(), cached.clone());

        if let Ok(dependency) = &loaded {
            for transitive_root in &dependency.dependency_roots {
                // A dependency that fails to load reports where it is imported, not here.
                let _ = self.load_dependency(transitive_root);
            }
        }

        cached
    }

    /// Indexes a library's modules so a declaring module identity maps back to it.
    fn cache_library(
        &mut self,
        library: &LibraryArtifact,
        library_root: &Path,
    ) -> Result<Arc<CachedImportedLibrary>, String> {
        let cached = Arc::new(build_cached_imported_library(library, library_root)?);
        for module_identity in cached.namespaces.keys() {
            self.library_roots_by_module_identity
                .insert(module_identity.clone(), library_root.to_path_buf());
        }
        Ok(cached)
    }

    /// Returns the cached library that owns `module_identity`, if one was loaded.
    fn library_for_module(&self, module_identity: &str) -> Option<(&Path, &CachedImportedLibrary)> {
        let root = self.library_roots_by_module_identity.get(module_identity)?;
        let cached = self.dependency_cache.get(root)?.as_ref().ok()?;
        Some((root.as_path(), cached))
    }

    /// Indexes the library being generated so its own module identities resolve as peers, through
    /// the same lookup a dependency's identities take.
    fn register_generating_library(&mut self, library: &LibraryArtifact) -> Result<(), String> {
        let cached = self.cache_library(library, &library.root_path)?;
        self.dependency_cache
            .insert(library.root_path.clone(), Ok(cached));
        self.generating_library_root = Some(library.root_path.clone());
        Ok(())
    }

    /// Resolves the type of every companion field inherited from another module in the namespace
    /// of the module that declared it.
    ///
    /// <para>An update companion copies its target's inherited fields, and their types were
    /// written in the base's module. A name that resolves to a peer of the generating library is
    /// rewritten to the peer's exported name, which the graph already declares. A name that
    /// resolves to a dependency's export gets an [`ImportedType`] on the generating module — the
    /// same one an explicit import would have produced — and is rewritten to its visible name. A
    /// name that resolves to nothing typegen can reference is left as written, with a warning
    /// that says why.</para>
    fn resolve_inherited_field_types(
        &self,
        modules: &mut [ExportedModule],
        warnings: &mut Vec<String>,
    ) {
        let declared_names = modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .map(|declaration| declaration.name().to_string())
            .collect::<FxHashSet<_>>();

        for module in modules.iter_mut() {
            let imported_types = &mut module.imported_types;
            for declaration in &mut module.declarations {
                let ExportedType::Update(update) = &mut declaration.item else {
                    continue;
                };
                let companion_name = update.name.clone();
                for field in &mut update.fields {
                    let Some(declaring_module) = field.declaring_module.as_deref() else {
                        continue;
                    };
                    let names = type_ref_name_strings(&field.ty).collect::<BTreeSet<_>>();
                    let mut renames = FxHashMap::default();
                    for name in names {
                        // An unresolved `X.Update` is reported by the companion rewrite instead.
                        if is_primitive_type_name(&name) || nx_hir::is_update_record_name(&name) {
                            continue;
                        }
                        match self.resolve_inherited_type_name(
                            declaring_module,
                            &name,
                            &declared_names,
                            imported_types,
                        ) {
                            Ok(resolved) => {
                                if resolved != name {
                                    renames.insert(name, resolved);
                                }
                            }
                            Err(reason) => warnings.push(format!(
                                "Update companion '{}' inherits field '{}' typed '{}' from a base declared in another module, and {}",
                                companion_name, field.name, name, reason
                            )),
                        }
                    }
                    if !renames.is_empty() {
                        rewrite_type_ref_names(&mut field.ty, &mut |name| {
                            renames.get(name).cloned()
                        });
                    }
                }
            }
        }
    }

    /// Resolves one type name written in `declaring_module` to the name the generating module
    /// should use for it, adding an imported type when the origin is a dependency's export.
    ///
    /// <para>The error is the reason the name cannot be resolved, phrased to follow "and", ending
    /// with what the author can do about it where there is something.</para>
    fn resolve_inherited_type_name(
        &self,
        declaring_module: &str,
        name: &str,
        declared_names: &FxHashSet<String>,
        imported_types: &mut Vec<ImportedType>,
    ) -> Result<String, String> {
        let namespace = self
            .library_for_module(declaring_module)
            .and_then(|(_, library)| library.namespaces.get(declaring_module))
            .ok_or_else(|| {
                format!(
                    "the declaring module '{}' belongs to no loaded library; the generated code will not resolve it",
                    declaring_module
                )
            })?;
        let type_name = nx_hir::Name::new(name);
        let origin = namespace
            .entry(PreparedNamespace::Type, &type_name)
            .or_else(|| namespace.entry(PreparedNamespace::Element, &type_name))
            .ok_or_else(|| {
                format!(
                    "it does not resolve in '{}'; the generated code will not resolve it either",
                    declaring_module
                )
            })?;
        let (origin_root, origin_library) = self
            .library_for_module(origin.module_identity())
            .ok_or_else(|| {
                format!(
                    "it resolves to a declaration in '{}', which belongs to no loaded library; the generated code will not resolve it",
                    origin.module_identity()
                )
            })?;
        let item = origin_library
            .items_by_origin
            .get(&(origin.module_identity().to_string(), origin.definition_id()))
            .ok_or_else(|| {
                format!(
                    "it resolves to a declaration in '{}' that library '{}' has no interface item for; the generated code will not resolve it",
                    origin.module_identity(),
                    origin_library.library_name
                )
            })?;

        if item.visibility != Visibility::Export {
            let library_name = &origin_library.library_name;
            return Err(if item.name == name {
                format!(
                    "library '{}' does not export '{}'; export it from '{}' so the generated code can reference it",
                    library_name, name, library_name
                )
            } else {
                format!(
                    "it resolves to '{}', which library '{}' does not export; export '{}' from '{}' so the generated code can reference it",
                    item.name, library_name, item.name, library_name
                )
            });
        }

        let is_peer = self.generating_library_root.as_deref() == Some(origin_root);
        if is_peer {
            if !matches!(
                item.kind,
                PreparedItemKind::TypeAlias
                    | PreparedItemKind::Union
                    | PreparedItemKind::Record
                    | PreparedItemKind::Component
            ) {
                return Err(format!(
                    "it resolves to '{}', whose kind '{:?}' has no generated representation; the generated code will not resolve it",
                    item.name, item.kind
                ));
            }
            // The graph declares the peer under its exported name, but an import of that name
            // wins over the graph in the emitters, so the bare name would mean the import.
            if let Some(shadowing) = imported_types
                .iter()
                .find(|imported| imported.visible_name == item.name)
            {
                return Err(format!(
                    "the import of '{}' from '{}' shadows the peer declaration '{}' it resolves to; import '{}' under a qualifier so the generated code can reference both",
                    shadowing.visible_name, shadowing.library_name, item.name, shadowing.exported_name
                ));
            }
            return Ok(item.name.clone());
        }

        if let Some(existing) = imported_types.iter().find(|imported| {
            imported.library_name == origin_library.library_name
                && imported.exported_name == item.name
        }) {
            return Ok(existing.visible_name.clone());
        }

        let name_is_taken = declared_names.contains(&item.name)
            || imported_types
                .iter()
                .any(|imported| imported.visible_name == item.name);
        let visible_name = if name_is_taken {
            format!("{}.{}", origin_library.library_name, item.name)
        } else {
            item.name.clone()
        };
        let imported_type = origin_library
            .imported_type(&visible_name, &item.name)
            .ok_or_else(|| {
                format!(
                    "it resolves to '{}' in library '{}', whose kind '{:?}' has no generated cross-library representation; the generated code will not resolve it",
                    item.name, origin_library.library_name, item.kind
                )
            })?;
        imported_types.push(imported_type);
        Ok(visible_name)
    }
}

fn resolve_dependency_root(source_path: &Path, library_path: &str) -> std::io::Result<PathBuf> {
    let candidate = if Path::new(library_path).is_absolute() {
        PathBuf::from(library_path)
    } else {
        source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(library_path)
    };

    fs::canonicalize(candidate)
}

fn visible_name_for_selective_import(entry: &SelectiveImport) -> String {
    match entry.qualifier.as_ref() {
        Some(prefix) => format!("{}.{}", prefix.as_str(), entry.name.as_str()),
        None => entry.name.as_str().to_string(),
    }
}

fn imported_visible_name_collision_warning(
    source_path: &Path,
    visible_name: &str,
    first_origin: &ImportedVisibleNameOrigin,
    second_library_path: &str,
    second_exported_name: &str,
) -> String {
    format!(
        "Skipping imported type '{}' from '{}' in '{}': visible imported name '{}' already maps to '{}' from '{}'; first match wins",
        second_exported_name,
        second_library_path,
        source_path.display(),
        visible_name,
        first_origin.exported_name,
        first_origin.library_path
    )
}

impl CachedImportedLibrary {
    fn imported_type(&self, visible_name: &str, exported_name: &str) -> Option<ImportedType> {
        let kind = match self.export_kinds.get(exported_name)? {
            PreparedItemKind::TypeAlias => {
                let target = self.resolve_alias_target(exported_name, &mut BTreeSet::new())?;
                let target_is_reference = self.type_ref_is_reference(&target, &mut BTreeSet::new());
                ImportedTypeKind::Alias {
                    target,
                    target_is_reference,
                }
            }
            PreparedItemKind::Union => ImportedTypeKind::Union {
                is_constant: self.constant_unions.contains(exported_name),
            },
            PreparedItemKind::Record => ImportedTypeKind::Record,
            PreparedItemKind::Component => ImportedTypeKind::Component,
            _ => return None,
        };

        Some(ImportedType {
            visible_name: visible_name.to_string(),
            exported_name: exported_name.to_string(),
            library_name: self.library_name.clone(),
            kind,
        })
    }

    /// The companion a derived name on an imported declaration reaches: `<visible>.Update` names
    /// the dependency's `<exported>_update`, and `<visible>.Property` its `<exported>_property`.
    fn derived_imported_type(
        &self,
        visible_name: &str,
        exported_name: &str,
        suffix: &str,
    ) -> Option<ImportedType> {
        let (companion_suffix, kind) = match suffix {
            nx_hir::UPDATE_RECORD_SUFFIX => ("_update", ImportedTypeKind::Record),
            nx_hir::PROPERTY_UNION_SUFFIX => {
                ("_property", ImportedTypeKind::Union { is_constant: true })
            }
            _ => return None,
        };
        if !self
            .derived_exports
            .contains(&format!("{}.{}", exported_name, suffix))
        {
            return None;
        }
        Some(ImportedType {
            visible_name: format!("{}.{}", visible_name, suffix),
            exported_name: format!("{}{}", exported_name, companion_suffix),
            library_name: self.library_name.clone(),
            kind,
        })
    }

    fn resolve_alias_target(
        &self,
        exported_name: &str,
        seen_aliases: &mut BTreeSet<String>,
    ) -> Option<TypeRef> {
        let target = self.alias_targets.get(exported_name)?.clone();
        let TypeRef::Name(target_name) = &target else {
            return Some(target);
        };
        if !matches!(
            self.export_kinds.get(target_name.as_str()),
            Some(PreparedItemKind::TypeAlias)
        ) {
            return Some(target);
        }

        if !seen_aliases.insert(exported_name.to_string()) {
            return None;
        }

        let resolved = self.resolve_alias_target(target_name.as_str(), seen_aliases);
        seen_aliases.remove(exported_name);
        resolved
    }

    fn type_ref_is_reference(&self, ty: &TypeRef, seen_aliases: &mut BTreeSet<String>) -> bool {
        match ty {
            TypeRef::Nullable(inner) => self.type_ref_is_reference(inner, seen_aliases),
            TypeRef::Array(_) | TypeRef::Function { .. } => true,
            TypeRef::Name(name) => self.type_name_is_reference(name.as_str(), seen_aliases),
        }
    }

    fn type_name_is_reference(&self, name: &str, seen_aliases: &mut BTreeSet<String>) -> bool {
        match name {
            "string" | "object" | "unknown" | "error" => true,
            "int" | "int32" | "int64" | "float32" | "float64" | "boolean" | "void" => false,
            other => match self.export_kinds.get(other) {
                Some(PreparedItemKind::Union) if self.constant_unions.contains(other) => false,
                Some(
                    PreparedItemKind::Union
                    | PreparedItemKind::Record
                    | PreparedItemKind::Component,
                ) => true,
                Some(PreparedItemKind::TypeAlias) => {
                    if !seen_aliases.insert(other.to_string()) {
                        return true;
                    }

                    let is_reference = self
                        .resolve_alias_target(other, &mut BTreeSet::new())
                        .map(|target| self.type_ref_is_reference(&target, seen_aliases))
                        .unwrap_or(true);
                    seen_aliases.remove(other);
                    is_reference
                }
                _ => true,
            },
        }
    }

    fn unsupported_import_warning(
        &self,
        source_path: &Path,
        library_path: &str,
        exported_name: &str,
    ) -> String {
        match self.export_kinds.get(exported_name) {
            Some(kind) => format!(
                "Skipping imported type '{}' from '{}' in '{}': exported item kind '{:?}' is not supported for generated cross-library references",
                exported_name,
                library_path,
                source_path.display(),
                kind
            ),
            None => format!(
                "Skipping imported type '{}' from '{}' in '{}': the dependency does not export that name",
                exported_name,
                library_path,
                source_path.display()
            ),
        }
    }
}

fn build_cached_imported_library(
    dependency: &LibraryArtifact,
    dependency_root: &Path,
) -> Result<CachedImportedLibrary, String> {
    let library_name = dependency_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "dependency root '{}' does not have a valid UTF-8 directory name",
                dependency_root.display()
            )
        })?
        .to_string();
    let mut export_kinds = FxHashMap::default();
    let mut derived_exports = FxHashSet::default();
    let mut constant_unions = FxHashSet::default();
    let mut alias_targets = FxHashMap::default();
    let mut wildcard_importable_export_names = Vec::new();
    let items_by_origin = dependency
        .interface_items
        .iter()
        .map(|item| {
            (
                (item.module_identity.clone(), item.definition_id),
                CachedInterfaceItem {
                    name: item.item_name.clone(),
                    visibility: item.visibility,
                    kind: item.item.kind(),
                },
            )
        })
        .collect();

    for (exported_name, item_indices) in &dependency.exported_items {
        let Some(index) = item_indices.first() else {
            continue;
        };
        let Some(interface_item) = dependency.interface_items.get(*index) else {
            continue;
        };

        // A derived update record or property union is generated as its target's `_update` or
        // `_property` companion; it is never a type a consumer names by its dotted NX name.
        if matches!(
            &interface_item.item,
            InterfaceItemKind::Record {
                kind: RecordKind::Update { .. },
                ..
            } | InterfaceItemKind::Union {
                property_target: Some(_),
                ..
            }
        ) {
            derived_exports.insert(exported_name.clone());
            continue;
        }

        let kind = interface_item.item.kind();
        // Wildcard imports intentionally collect only the exports codegen can resolve as
        // cross-library type references; unsupported kinds are ignored instead of warned to
        // avoid noise from unrelated value/function exports.
        if matches!(
            kind,
            PreparedItemKind::TypeAlias
                | PreparedItemKind::Union
                | PreparedItemKind::Record
                | PreparedItemKind::Component
        ) {
            wildcard_importable_export_names.push(exported_name.clone());
        }
        if let InterfaceItemKind::TypeAlias { ty, .. } = &interface_item.item {
            alias_targets.insert(exported_name.clone(), ty.clone());
        }
        if let InterfaceItemKind::Union { base, cases, .. } = &interface_item.item {
            if base.is_none()
                && !cases.is_empty()
                && cases.iter().all(|case| case.fields.is_empty())
            {
                constant_unions.insert(exported_name.clone());
            }
        }
        export_kinds.insert(exported_name.clone(), kind);
    }

    wildcard_importable_export_names.sort();

    Ok(CachedImportedLibrary {
        library_name,
        export_kinds,
        derived_exports,
        constant_unions,
        alias_targets,
        wildcard_importable_export_names,
        namespaces: dependency.namespaces.clone(),
        items_by_origin,
    })
}

fn export_alias(def: &TypeAlias) -> ExportedAlias {
    ExportedAlias {
        name: def.name.as_str().to_string(),
        target: def.ty.clone(),
    }
}

fn export_record(module: &LoweredModule, def: &RecordDef) -> ExportedRecord {
    ExportedRecord {
        name: def.name.as_str().to_string(),
        kind: def.kind.clone(),
        is_abstract: def.is_abstract,
        base: def.base.as_ref().map(|name| name.as_str().to_string()),
        fields: def
            .properties
            .iter()
            .map(|field| export_record_field(module, field))
            .collect(),
    }
}

fn export_union(module: &LoweredModule, def: &UnionDef) -> ExportedUnion {
    ExportedUnion {
        name: def.name.as_str().to_string(),
        base: def.base.as_ref().map(|name| name.as_str().to_string()),
        cases: def
            .cases
            .iter()
            .map(|case| export_union_case(module, case))
            .collect(),
        property_target: None,
    }
}

/// Exports the `<Target>_property` companion: the derived property union's cases, which analysis
/// has already completed with the target's inherited fields, in `T.Property` case order.
fn export_property_union(def: &UnionDef, target: &nx_hir::Name) -> ExportedUnion {
    ExportedUnion {
        name: format!("{}_property", target.as_str()),
        base: None,
        cases: def
            .cases
            .iter()
            .map(|case| ExportedUnionCase {
                name: case.name.as_str().to_string(),
                fields: Vec::new(),
            })
            .collect(),
        property_target: Some(target.as_str().to_string()),
    }
}

fn export_union_case(module: &LoweredModule, case: &UnionCaseDef) -> ExportedUnionCase {
    ExportedUnionCase {
        name: case.name.as_str().to_string(),
        fields: case
            .fields
            .iter()
            .map(|field| export_union_case_field(module, field))
            .collect(),
    }
}

fn export_external_component_contract(
    module: &LoweredModule,
    component: &Component,
) -> Option<ExportedRecord> {
    if !component.is_external {
        return None;
    }

    Some(ExportedRecord {
        name: component.name.as_str().to_string(),
        kind: RecordKind::Plain,
        is_abstract: component.is_abstract,
        base: component
            .base
            .as_ref()
            .map(|name| name.as_str().to_string()),
        fields: component
            .props
            .iter()
            .map(|field| export_record_field(module, field))
            .collect(),
    })
}

/// Returns true for a type name both emitters map to a host primitive rather than a declaration.
fn is_primitive_type_name(name: &str) -> bool {
    matches!(
        name,
        "string" | "int" | "int32" | "int64" | "float32" | "float64" | "boolean" | "object"
    )
}

/// Every type name `ty` mentions, as owned strings.
fn type_ref_name_strings(ty: &TypeRef) -> impl Iterator<Item = String> + '_ {
    nx_hir::type_ref_names(ty)
        .into_iter()
        .map(|name| name.as_str().to_string())
}

/// Replaces every named type in `ty` for which `rename` returns a new name.
fn rewrite_type_ref_names(ty: &mut TypeRef, rename: &mut impl FnMut(&str) -> Option<String>) {
    match ty {
        TypeRef::Name(name) => {
            if let Some(renamed) = rename(name.as_str()) {
                *name = nx_hir::Name::new(&renamed);
            }
        }
        TypeRef::Array(inner) | TypeRef::Nullable(inner) => rewrite_type_ref_names(inner, rename),
        TypeRef::Function {
            params,
            return_type,
        } => {
            for param in params {
                rewrite_type_ref_names(param, rename);
            }
            rewrite_type_ref_names(return_type, rename);
        }
    }
}

/// Exports the `<Target>_update` companion with the target's effective fields.
///
/// <para>An update companion extends nothing — a patch of `User` is not a patch of its base — so
/// the inherited fields are written on the companion itself. They come from the prepared module,
/// which resolves the target's base chain the way the checker does, across libraries included.
/// Without a prepared module, or when the chain does not resolve, the declared fields are all
/// there is.</para>
fn export_update(
    prepared: Option<&PreparedModule>,
    record: &RecordDef,
    target: &nx_hir::Name,
) -> ExportedUpdate {
    let fields = prepared
        .and_then(|prepared| {
            nx_hir::effective_record_shape(prepared, record)
                .ok()
                .map(|shape| (prepared.module_identity(), shape))
        })
        .map(|(module_identity, shape)| {
            shape
                .fields
                .into_iter()
                .map(|field| ExportedRecordField {
                    name: field.name.as_str().to_string(),
                    ty: field.ty,
                    default_value: None,
                    declaring_module: (field.module_identity != module_identity)
                        .then_some(field.module_identity),
                })
                .collect()
        })
        .unwrap_or_else(|| {
            record
                .properties
                .iter()
                .map(|field| ExportedRecordField {
                    name: field.name.as_str().to_string(),
                    ty: field.ty.clone(),
                    default_value: None,
                    declaring_module: None,
                })
                .collect()
        });
    ExportedUpdate {
        target_name: target.as_str().to_string(),
        name: format!("{}_update", target.as_str()),
        discriminator: record.name.as_str().to_string(),
        fields,
    }
}

fn export_external_state(component: &Component) -> Option<ExportedExternalState> {
    if !component.is_external || component.state.is_empty() {
        return None;
    }

    Some(ExportedExternalState {
        component_name: component.name.as_str().to_string(),
        name: format!("{}_state", component.name.as_str()),
        fields: component
            .state
            .iter()
            .map(|field| ExportedRecordField {
                name: field.name.as_str().to_string(),
                ty: field.ty.clone(),
                default_value: None,
                declaring_module: None,
            })
            .collect(),
    })
}

fn export_record_field(module: &LoweredModule, field: &RecordField) -> ExportedRecordField {
    ExportedRecordField {
        name: field.name.as_str().to_string(),
        ty: field.ty.clone(),
        default_value: export_field_default(module, field.default),
        declaring_module: None,
    }
}

fn export_union_case_field(module: &LoweredModule, field: &UnionCaseField) -> ExportedRecordField {
    ExportedRecordField {
        name: field.name.as_str().to_string(),
        ty: field.ty.clone(),
        default_value: export_field_default(module, field.default),
        declaring_module: None,
    }
}

fn export_field_default(
    module: &LoweredModule,
    default_expr: Option<nx_hir::ExprId>,
) -> Option<ExportedFieldDefault> {
    let default_expr = default_expr?;
    match module.expr(default_expr) {
        Expr::Literal(literal) => Some(ExportedFieldDefault::Literal(export_literal_default(
            literal,
        ))),
        _ => Some(ExportedFieldDefault::Unsupported),
    }
}

fn export_literal_default(literal: &Literal) -> ExportedLiteralDefault {
    match literal {
        Literal::String(value) => ExportedLiteralDefault::String(value.as_str().to_string()),
        Literal::Int(value) => ExportedLiteralDefault::Int(*value),
        Literal::Float(value) => ExportedLiteralDefault::Float(*value),
        Literal::Boolean(value) => ExportedLiteralDefault::Boolean(*value),
        Literal::Null => ExportedLiteralDefault::Null,
    }
}

fn module_output_stem(path: &Path) -> Result<PathBuf, String> {
    let stem = path.with_extension("");
    if stem.as_os_str().is_empty() {
        return Err(format!(
            "Could not derive a module output path from '{}'",
            path.display()
        ));
    }

    Ok(stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typegen::test_support::write_library;
    use nx_api::build_library_artifact_from_directory;
    use nx_hir::{lower, LoweredModule, SourceId};
    use nx_syntax::parse_str;
    use std::fs;
    use tempfile::TempDir;

    fn lower_module(source: &str, file_name: &str) -> LoweredModule {
        let parse_result = parse_str(source, file_name);
        let tree = parse_result.tree.expect("expected parse tree");
        lower(tree.root(), SourceId::new(0))
    }

    fn analyze_module(source: &str, file_name: &str) -> ModuleArtifact {
        nx_types::analyze_str(source, file_name)
    }

    #[test]
    fn resolves_abstract_record_alias_bases() {
        let source = r#"
            export abstract type Question = { label:string }
            export type QuestionBaseAlias = Question
            export type ShortTextQuestion extends QuestionBaseAlias = { placeholder:string? }
        "#;
        let module = analyze_module(source, "types.nx");
        let graph = ExportedTypeGraph::from_module(&module, Path::new("types.nx")).unwrap();

        let short_text = graph
            .record("ShortTextQuestion")
            .expect("short text record");
        let base = graph
            .resolved_record_base(short_text)
            .expect("resolved abstract base");
        assert_eq!(base.name, "Question");
        assert!(base.is_abstract);
    }

    #[test]
    fn collects_transitive_concrete_descendants_across_modules() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("base.nx"),
            "export abstract type Question = { label:string }",
        )
        .expect("base file");
        fs::write(
            library_dir.join("derived.nx"),
            "export abstract type TextQuestion extends Question = { placeholder:string? }",
        )
        .expect("derived file");
        fs::write(
            library_dir.join("short-text.nx"),
            "export type ShortTextQuestion extends TextQuestion = { maxLength:int? }",
        )
        .expect("short text file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let graph = ExportedTypeGraph::from_library(&artifact).unwrap();

        let descendants = graph
            .concrete_descendants("Question")
            .into_iter()
            .map(|record| record.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(descendants, vec!["ShortTextQuestion"]);
        assert_eq!(
            graph
                .owner_module("ShortTextQuestion")
                .expect("short text module"),
            Path::new("short-text")
        );
    }

    #[test]
    fn collects_exported_external_component_state_contracts() {
        let source = r#"
            export external component <SearchBox placeholder:string /> = {
              state { query:string = "docs" }
            }
        "#;
        let module = analyze_module(source, "components.nx");
        let graph = ExportedTypeGraph::from_module(&module, Path::new("components.nx")).unwrap();

        let declaration = graph
            .declaration("SearchBox_state")
            .expect("SearchBox_state declaration");
        match &declaration.item {
            ExportedType::ExternalState(state) => {
                assert_eq!(state.component_name, "SearchBox");
                assert_eq!(state.fields.len(), 1);
                assert_eq!(state.fields[0].name, "query");
                assert!(state.fields[0].default_value.is_none());
            }
            other => panic!("Expected generated external state contract, got {other:?}"),
        }
    }

    #[test]
    fn skips_external_component_state_generation_when_name_conflicts_with_export() {
        let source = r#"
            export type SearchBox_state = string
            export external component <SearchBox /> = {
              state { query:string }
            }
        "#;
        let module = analyze_module(source, "components.nx");
        let build =
            ExportedTypeGraph::from_module_with_warnings(&module, Path::new("components.nx"))
                .expect("graph build");

        assert!(build.graph.declaration("SearchBox_state").is_some());
        assert!(
            !matches!(
                build
                    .graph
                    .declaration("SearchBox_state")
                    .map(|declaration| &declaration.item),
                Some(ExportedType::ExternalState(_))
            ),
            "explicit export should win when generated state collides"
        );
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("SearchBox_state"));
    }

    #[test]
    fn skips_external_component_state_generation_when_alias_module_sorts_first() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("a-alias.nx"),
            "export type SearchBox_state = string",
        )
        .expect("alias file");
        fs::write(
            library_dir.join("z-search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { query:string }
}"#,
        )
        .expect("component file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let build = ExportedTypeGraph::from_library_with_warnings(&artifact).expect("graph build");

        assert!(
            !matches!(
                build
                    .graph
                    .declaration("SearchBox_state")
                    .map(|declaration| &declaration.item),
                Some(ExportedType::ExternalState(_))
            ),
            "explicit export should win when alias module sorts first"
        );
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("SearchBox_state"));
    }

    #[test]
    fn skips_external_component_state_generation_when_component_module_sorts_first() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("a-search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { query:string }
}"#,
        )
        .expect("component file");
        fs::write(
            library_dir.join("z-alias.nx"),
            "export type SearchBox_state = string",
        )
        .expect("alias file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let build = ExportedTypeGraph::from_library_with_warnings(&artifact).expect("graph build");

        assert!(
            !matches!(
                build
                    .graph
                    .declaration("SearchBox_state")
                    .map(|declaration| &declaration.item),
                Some(ExportedType::ExternalState(_))
            ),
            "explicit export should win when component module sorts first"
        );
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("SearchBox_state"));
    }

    #[test]
    fn skips_external_component_state_generation_when_multiple_components_share_name() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        fs::create_dir_all(&library_dir).expect("library dir");
        fs::write(
            library_dir.join("a-search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { query:string }
}"#,
        )
        .expect("component file");
        fs::write(
            library_dir.join("z-search-box.nx"),
            r#"export external component <SearchBox /> = {
  state { theme:string }
}"#,
        )
        .expect("component file");

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let build = ExportedTypeGraph::from_library_with_warnings(&artifact).expect("graph build");

        assert!(build.graph.declaration("SearchBox_state").is_none());
        // Both components would also generate `SearchBox_update` and `SearchBox_property`, which
        // are skipped the same way.
        let state_warnings = build
            .warnings
            .iter()
            .filter(|warning| warning.contains("SearchBox_state"))
            .count();
        assert_eq!(state_warnings, 2);
        assert!(build.graph.declaration("SearchBox_update").is_none());
        assert!(build.graph.declaration("SearchBox_property").is_none());
        assert_eq!(build.warnings.len(), 6);
    }

    #[test]
    fn warns_when_imported_library_cannot_be_resolved() {
        let temp_dir = TempDir::new().expect("temp dir");
        let source_path = temp_dir.path().join("chat-link.nx");
        let module = lower_module(
            r#"import "../question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
            source_path.to_str().expect("source path"),
        );
        let mut collector = ImportedTypeCollector::default();

        let build = collector.collect_for_module(&module, &source_path);

        assert!(build.imported_types.is_empty());
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("../question-flow"));
        assert!(build.warnings[0].contains("chat-link.nx"));
    }

    #[test]
    fn caches_imported_library_analysis_by_resolved_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let dependency_dir = temp_dir.path().join("question-flow");
        fs::create_dir_all(&dependency_dir).expect("dependency dir");
        fs::write(
            dependency_dir.join("QuestionFlow.nx"),
            "export type QuestionFlow = { id:string }",
        )
        .expect("dependency file");
        let source = r#"import "./question-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#;
        let first_source_path = temp_dir.path().join("chat-link.nx");
        let second_source_path = temp_dir.path().join("search-link.nx");
        let first_module = lower_module(source, first_source_path.to_str().expect("first path"));
        let second_module = lower_module(source, second_source_path.to_str().expect("second path"));
        let mut collector = ImportedTypeCollector::default();

        let first_build = collector.collect_for_module(&first_module, &first_source_path);
        let second_build = collector.collect_for_module(&second_module, &second_source_path);

        assert_eq!(first_build.imported_types.len(), 1);
        assert_eq!(second_build.imported_types.len(), 1);
        assert!(first_build.warnings.is_empty());
        assert!(second_build.warnings.is_empty());
        assert_eq!(collector.dependency_cache.len(), 1);
    }

    #[test]
    fn loads_transitive_dependencies_and_maps_their_module_identities() {
        let temp_dir = TempDir::new().expect("temp dir");
        let tags_dir = temp_dir.path().join("tags");
        let named_dir = temp_dir.path().join("named");
        fs::create_dir_all(&tags_dir).expect("tags dir");
        fs::create_dir_all(&named_dir).expect("named dir");
        fs::write(tags_dir.join("Tag.nx"), "export type Tag = a | b").expect("tags file");
        fs::write(
            named_dir.join("Named.nx"),
            r#"import { Tag } from "../tags"

export abstract type Named = { name:string tag:Tag }
"#,
        )
        .expect("named file");
        let source_path = temp_dir.path().join("people.nx");
        let module = lower_module(
            r#"import { Named } from "./named"

export type User extends Named = { email:string }
"#,
            source_path.to_str().expect("source path"),
        );
        let mut collector = ImportedTypeCollector::default();

        let build = collector.collect_for_module(&module, &source_path);

        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        assert_eq!(collector.dependency_cache.len(), 2);
        let named_identity = fs::canonicalize(named_dir.join("Named.nx"))
            .expect("named identity")
            .display()
            .to_string();
        let tags_identity = fs::canonicalize(tags_dir.join("Tag.nx"))
            .expect("tags identity")
            .display()
            .to_string();
        let (named_root, named) = collector
            .library_for_module(&named_identity)
            .expect("named module resolves to its library");
        assert_eq!(
            named_root,
            fs::canonicalize(&named_dir).expect("named root")
        );
        assert_eq!(named.library_name, "named");
        let (tags_root, tags) = collector
            .library_for_module(&tags_identity)
            .expect("transitive module resolves to its library");
        assert_eq!(tags_root, fs::canonicalize(&tags_dir).expect("tags root"));
        assert_eq!(tags.library_name, "tags");
    }

    #[test]
    fn warns_when_wildcard_imports_collide_on_visible_name() {
        let temp_dir = TempDir::new().expect("temp dir");
        let first_dependency_dir = temp_dir.path().join("question-flow");
        let second_dependency_dir = temp_dir.path().join("survey-flow");
        fs::create_dir_all(&first_dependency_dir).expect("first dependency dir");
        fs::create_dir_all(&second_dependency_dir).expect("second dependency dir");
        fs::write(
            first_dependency_dir.join("QuestionFlow.nx"),
            "export type QuestionFlow = { id:string }",
        )
        .expect("first dependency file");
        fs::write(
            second_dependency_dir.join("QuestionFlow.nx"),
            "export type QuestionFlow = { name:string }",
        )
        .expect("second dependency file");
        let source_path = temp_dir.path().join("chat-link.nx");
        let module = lower_module(
            r#"import "./question-flow"
import "./survey-flow"

export type QuestionFlowInitialExperience = {
  questionFlow: QuestionFlow
}
"#,
            source_path.to_str().expect("source path"),
        );
        let mut collector = ImportedTypeCollector::default();

        let build = collector.collect_for_module(&module, &source_path);

        assert_eq!(build.imported_types.len(), 1);
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("QuestionFlow"));
        assert!(build.warnings[0].contains("./question-flow"));
        assert!(build.warnings[0].contains("./survey-flow"));
        assert!(build.warnings[0].contains("first match wins"));
    }

    #[test]
    fn warns_when_selective_import_targets_unsupported_kind() {
        let temp_dir = TempDir::new().expect("temp dir");
        let dependency_dir = temp_dir.path().join("question-flow");
        fs::create_dir_all(&dependency_dir).expect("dependency dir");
        fs::write(
            dependency_dir.join("QuestionFlow.nx"),
            "export let answer(): int = { 42 }",
        )
        .expect("dependency file");
        let source_path = temp_dir.path().join("chat-link.nx");
        let module = lower_module(
            r#"import { answer } from "./question-flow"

export type ChatLink = string
"#,
            source_path.to_str().expect("source path"),
        );
        let mut collector = ImportedTypeCollector::default();

        let build = collector.collect_for_module(&module, &source_path);

        assert!(build.imported_types.is_empty());
        assert_eq!(build.warnings.len(), 1);
        assert!(build.warnings[0].contains("answer"));
        assert!(build.warnings[0].contains("./question-flow"));
        assert!(build.warnings[0].contains("Function"));
        assert!(build.warnings[0].contains("not supported"));
    }

    fn update_companion<'a>(graph: &'a ExportedTypeGraph, name: &str) -> &'a ExportedUpdate {
        match graph.declaration(name).map(|declaration| &declaration.item) {
            Some(ExportedType::Update(update)) => update,
            other => panic!("Expected update companion '{name}', got {other:?}"),
        }
    }

    fn field_names(fields: &[ExportedRecordField]) -> Vec<&str> {
        fields.iter().map(|field| field.name.as_str()).collect()
    }

    #[test]
    fn exports_update_companions_for_records_actions_and_stateful_components() {
        let module = analyze_module(
            r#"
            export type User = { name:string = "anon" email:string? }
            export action Saved = { id:int }
            export component <Counter step:int = 1 /> = { state { count:int = 0 } <Label /> }
            export component <Plain /> = { <Label /> }
            type Hidden = { secret:string }
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");
        assert!(build.warnings.is_empty(), "{:?}", build.warnings);

        let user = update_companion(&build.graph, "User_update");
        assert_eq!(user.discriminator, "User.Update");
        assert_eq!(field_names(&user.fields), vec!["name", "email"]);
        assert!(user
            .fields
            .iter()
            .all(|field| field.default_value.is_none()));

        assert_eq!(
            update_companion(&build.graph, "Saved_update").discriminator,
            "Saved.Update"
        );

        let counter = update_companion(&build.graph, "Counter_update");
        assert_eq!(
            field_names(&counter.fields),
            vec!["count"],
            "a component's companion carries its state and none of its props"
        );

        assert!(build.graph.declaration("Plain_update").is_none());
        assert!(build.graph.declaration("Hidden_update").is_none());
        assert!(
            build.graph.declaration("User.Update").is_none(),
            "an update record is never exported under its dotted name"
        );
    }

    #[test]
    fn update_companions_flatten_inherited_fields() {
        let module = analyze_module(
            r#"
            export abstract type Named = { name:string }
            export type User extends Named = { email:string }
            "#,
            "types.nx",
        );
        let graph = ExportedTypeGraph::from_module(&module, Path::new("types.nx")).expect("graph");

        assert_eq!(
            field_names(&update_companion(&graph, "User_update").fields),
            vec!["name", "email"]
        );
        assert_eq!(
            field_names(&update_companion(&graph, "Named_update").fields),
            vec!["name"]
        );
    }

    fn field_type<'a>(update: &'a ExportedUpdate, name: &str) -> &'a TypeRef {
        &update
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("field '{name}' on '{}'", update.name))
            .ty
    }

    #[test]
    fn inherited_companion_field_typed_by_a_peer_resolves_without_an_import() {
        let temp_dir = TempDir::new().expect("temp dir");
        let library_dir = temp_dir.path().join("ui");
        write_library(
            &library_dir,
            &[
                ("tag.nx", "export type Tag = a | b"),
                (
                    "named.nx",
                    "export abstract type Named = { name:string tag:Tag }",
                ),
                (
                    "user.nx",
                    "export type User extends Named = { email:string }",
                ),
            ],
        );

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let build = ExportedTypeGraph::from_library_with_warnings(&artifact).expect("graph build");

        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        let user = update_companion(&build.graph, "User_update");
        assert_eq!(
            field_type(user, "tag"),
            &TypeRef::Name(nx_hir::Name::new("Tag"))
        );
        let user_module = build
            .graph
            .modules
            .iter()
            .find(|module| module.module_path == Path::new("user"))
            .expect("user module");
        assert!(user_module.imported_types.is_empty());
    }

    #[test]
    fn inherited_companion_field_typed_through_a_peer_alias_resolves_to_the_origin() {
        let temp_dir = TempDir::new().expect("temp dir");
        let tags_dir = temp_dir.path().join("tags");
        let library_dir = temp_dir.path().join("people");
        write_library(&tags_dir, &[("tag.nx", "export type Tag = a | b")]);
        write_library(
            &library_dir,
            &[
                (
                    "named.nx",
                    r#"import { Tag as t.Tag } from "../tags"

export abstract type Named = { name:string tag:t.Tag }
"#,
                ),
                (
                    "user.nx",
                    "export type User extends Named = { email:string }",
                ),
            ],
        );

        let artifact = build_library_artifact_from_directory(&library_dir).expect("library build");
        let build = ExportedTypeGraph::from_library_with_warnings(&artifact).expect("graph build");

        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        let user = update_companion(&build.graph, "User_update");
        assert_eq!(
            field_type(user, "tag"),
            &TypeRef::Name(nx_hir::Name::new("Tag"))
        );
        let user_module = build
            .graph
            .modules
            .iter()
            .find(|module| module.module_path == Path::new("user"))
            .expect("user module");
        assert_eq!(
            user_module.imported_types,
            vec![ImportedType {
                visible_name: "Tag".to_string(),
                exported_name: "Tag".to_string(),
                library_name: "tags".to_string(),
                kind: ImportedTypeKind::Union { is_constant: true },
            }]
        );
    }

    #[test]
    fn inherited_companion_field_typed_by_a_dependency_export_synthesizes_an_import() {
        let temp_dir = TempDir::new().expect("temp dir");
        write_library(
            &temp_dir.path().join("named"),
            &[(
                "Named.nx",
                "export type Tag = a | b
export abstract type Named = { name:string tag:Tag }",
            )],
        );
        let source_path = temp_dir.path().join("people.nx");
        let source = r#"import { Named } from "./named"

export type User extends Named = { email:string }
"#;

        let build = ExportedTypeGraph::from_source_with_warnings(source, &source_path)
            .expect("graph build");

        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        let user = update_companion(&build.graph, "User_update");
        assert_eq!(
            field_type(user, "tag"),
            &TypeRef::Name(nx_hir::Name::new("Tag"))
        );
        assert_eq!(
            build.graph.modules[0].imported_types,
            vec![
                ImportedType {
                    visible_name: "Named".to_string(),
                    exported_name: "Named".to_string(),
                    library_name: "named".to_string(),
                    kind: ImportedTypeKind::Record,
                },
                ImportedType {
                    visible_name: "Tag".to_string(),
                    exported_name: "Tag".to_string(),
                    library_name: "named".to_string(),
                    kind: ImportedTypeKind::Union { is_constant: true },
                },
            ]
        );
    }

    #[test]
    fn synthesized_import_is_qualified_when_the_module_declares_the_same_name() {
        let temp_dir = TempDir::new().expect("temp dir");
        write_library(
            &temp_dir.path().join("named"),
            &[(
                "Named.nx",
                "export type Tag = a | b
export abstract type Named = { name:string tag:Tag }",
            )],
        );
        let source_path = temp_dir.path().join("people.nx");
        let source = r#"import { Named } from "./named"

export type Tag = { label:string }
export type User extends Named = { email:string }
"#;

        let build = ExportedTypeGraph::from_source_with_warnings(source, &source_path)
            .expect("graph build");

        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        let user = update_companion(&build.graph, "User_update");
        assert_eq!(
            field_type(user, "tag"),
            &TypeRef::Name(nx_hir::Name::new("named.Tag"))
        );
        let synthesized = build.graph.modules[0]
            .imported_types
            .iter()
            .find(|imported| imported.exported_name == "Tag")
            .expect("synthesized import");
        assert_eq!(synthesized.visible_name, "named.Tag");
        assert_eq!(synthesized.library_name, "named");
    }

    #[test]
    fn inherited_companion_field_typed_by_an_unexported_dependency_type_warns() {
        let temp_dir = TempDir::new().expect("temp dir");
        write_library(
            &temp_dir.path().join("named"),
            &[(
                "Named.nx",
                "type Tag = a | b
export abstract type Named = { name:string tag:Tag }",
            )],
        );
        let source_path = temp_dir.path().join("people.nx");
        let source = r#"import { Named } from "./named"

export type User extends Named = { email:string }
"#;

        let build = ExportedTypeGraph::from_source_with_warnings(source, &source_path)
            .expect("graph build");

        assert_eq!(build.warnings.len(), 1, "{:?}", build.warnings);
        assert!(
            build.warnings[0].contains("'User_update' inherits field 'tag' typed 'Tag'")
                && build.warnings[0].contains("does not export 'Tag'")
                && build.warnings[0].contains("export it from 'named'"),
            "{}",
            build.warnings[0]
        );
        let user = update_companion(&build.graph, "User_update");
        assert_eq!(
            field_type(user, "tag"),
            &TypeRef::Name(nx_hir::Name::new("Tag"))
        );
        assert_eq!(build.graph.modules[0].imported_types.len(), 1);
    }

    #[test]
    fn update_companion_name_collision_warns_and_skips() {
        let module = analyze_module(
            r#"
            export type User_update = string
            export type User = { name:string }
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");

        assert!(matches!(
            build
                .graph
                .declaration("User_update")
                .map(|declaration| &declaration.item),
            Some(ExportedType::Alias(_))
        ));
        assert_eq!(build.warnings.len(), 1, "{:?}", build.warnings);
        assert!(build.warnings[0].contains("User_update"));
        assert!(build.warnings[0].contains("conflicts with exported declaration"));
    }

    fn property_companion<'a>(graph: &'a ExportedTypeGraph, name: &str) -> &'a ExportedUnion {
        match &graph
            .declaration(name)
            .unwrap_or_else(|| panic!("expected {} companion", name))
            .item
        {
            ExportedType::Union(union_def) => union_def,
            other => panic!("expected {} to be a union companion, got {:?}", name, other),
        }
    }

    fn case_names(union_def: &ExportedUnion) -> Vec<&str> {
        union_def
            .cases
            .iter()
            .map(|case| case.name.as_str())
            .collect()
    }

    #[test]
    fn exports_property_companions_for_records_actions_and_stateful_components() {
        let module = analyze_module(
            r#"
            export type User = { name:string = "anon" email:string? }
            export action Saved = { id:int }
            export component <Counter step:int = 1 /> = { state { count:int = 0 } <Label /> }
            export component <Plain /> = { <Label /> }
            type Hidden = { secret:string }
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");
        assert!(build.warnings.is_empty(), "{:?}", build.warnings);

        let user = property_companion(&build.graph, "User_property");
        assert_eq!(user.property_target.as_deref(), Some("User"));
        assert_eq!(case_names(user), vec!["name", "email"]);
        assert!(
            user.is_constant(),
            "a property companion is a constant union"
        );

        assert_eq!(
            case_names(property_companion(&build.graph, "Saved_property")),
            vec!["id"]
        );
        assert_eq!(
            case_names(property_companion(&build.graph, "Counter_property")),
            vec!["count"],
            "a component's companion names its state and none of its props"
        );

        assert!(build.graph.declaration("Plain_property").is_none());
        assert!(build.graph.declaration("Hidden_property").is_none());
        assert!(
            build.graph.declaration("User.Property").is_none(),
            "a property union is never exported under its dotted name"
        );
    }

    #[test]
    fn property_companions_list_inherited_fields_first() {
        let module = analyze_module(
            r#"
            export abstract type Named = { name:string }
            export type User extends Named = { email:string }
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");
        assert_eq!(
            case_names(property_companion(&build.graph, "User_property")),
            vec!["name", "email"]
        );
        assert_eq!(
            case_names(property_companion(&build.graph, "Named_property")),
            vec!["name"]
        );
    }

    #[test]
    fn property_typed_fields_are_rewritten_to_the_companion() {
        let module = analyze_module(
            r#"
            export type Contact = { title:string }
            export external component <Table sortBy:Contact.Property? columns:Contact.Property[] />
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");
        assert!(build.warnings.is_empty(), "{:?}", build.warnings);
        let ExportedType::Record(table) = &build.graph.declaration("Table").expect("Table").item
        else {
            panic!("expected the Table contract");
        };
        let record_field_type = |name: &str| {
            &table
                .fields
                .iter()
                .find(|field| field.name == name)
                .unwrap_or_else(|| panic!("expected field {}", name))
                .ty
        };
        assert_eq!(
            record_field_type("sortBy"),
            &TypeRef::nullable(TypeRef::Name(nx_hir::Name::new("Contact_property")))
        );
        assert_eq!(
            record_field_type("columns"),
            &TypeRef::array(TypeRef::Name(nx_hir::Name::new("Contact_property")))
        );
    }

    #[test]
    fn property_companion_name_collision_warns_and_skips() {
        let module = analyze_module(
            r#"
            export type User_property = string
            export type User = { name:string }
            "#,
            "types.nx",
        );
        let build = ExportedTypeGraph::from_module_with_warnings(&module, Path::new("types.nx"))
            .expect("graph build");
        assert!(matches!(
            build
                .graph
                .declaration("User_property")
                .map(|declaration| &declaration.item),
            Some(ExportedType::Alias(_))
        ));
        assert!(
            build
                .warnings
                .iter()
                .any(|warning| warning.contains("property union 'User_property' for 'User'")),
            "{:?}",
            build.warnings
        );
    }
}
