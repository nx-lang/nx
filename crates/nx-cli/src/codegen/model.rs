use nx_api::{build_library_artifact_from_directory, LibraryArtifact};
use nx_hir::{
    ast::TypeRef, Component, EnumDef, ImportKind, Item, LoweredModule, PreparedItemKind, RecordDef,
    RecordKind, SelectiveImport, TypeAlias, Visibility,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedAlias {
    pub name: String,
    pub target: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedEnum {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedRecordField {
    pub name: String,
    pub ty: TypeRef,
    pub has_default: bool,
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
pub struct ExportedExternalState {
    pub component_name: String,
    pub name: String,
    pub fields: Vec<ExportedRecordField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedType {
    pub visible_name: String,
    pub exported_name: String,
    pub library_name: String,
    pub is_reference: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportedType {
    Alias(ExportedAlias),
    Enum(ExportedEnum),
    Record(ExportedRecord),
    ExternalState(ExportedExternalState),
}

impl ExportedType {
    pub fn name(&self) -> &str {
        match self {
            Self::Alias(alias) => &alias.name,
            Self::Enum(enum_def) => &enum_def.name,
            Self::Record(record) => &record.name,
            Self::ExternalState(state) => &state.name,
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
    dependency_cache: FxHashMap<PathBuf, Result<CachedImportedLibrary, String>>,
}

struct ImportedTypesBuild {
    imported_types: Vec<ImportedType>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CachedImportedLibrary {
    library_name: String,
    export_kinds: FxHashMap<String, PreparedItemKind>,
    wildcard_importable_export_names: Vec<String>,
}

#[derive(Clone)]
struct ImportedVisibleNameOrigin {
    library_path: String,
    exported_name: String,
}

impl ExportedTypeGraph {
    #[allow(dead_code)]
    pub fn from_module(module: &LoweredModule, source_path: &Path) -> Result<Self, String> {
        Ok(Self::from_module_with_warnings(module, source_path)?.graph)
    }

    pub fn from_module_with_warnings(
        module: &LoweredModule,
        source_path: &Path,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let mut imported_type_collector = ImportedTypeCollector::default();
        let imported_build = imported_type_collector.collect_for_module(module, source_path);
        let file_name = source_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("types.nx"));
        let module_path = module_output_stem(&file_name)?;
        let mut build = Self::build_from_exported_modules(vec![ExportedModule {
            module_path: module_path.clone(),
            imported_types: imported_build.imported_types.clone(),
            declarations: collect_exported_declarations(module),
        }])?;

        if build.graph.modules.is_empty() {
            build.graph.modules.push(ExportedModule {
                module_path,
                imported_types: imported_build.imported_types,
                declarations: Vec::new(),
            });
        }

        build.warnings.extend(imported_build.warnings);
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
        let mut modules = Vec::new();
        let mut warnings = Vec::new();

        for artifact in &library.modules {
            let Some(module) = artifact.lowered_module.as_ref() else {
                continue;
            };

            let declarations = collect_exported_declarations(module);
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

    fn build_from_exported_modules(
        modules: Vec<ExportedModule>,
    ) -> Result<ExportedTypeGraphBuild, String> {
        let mut explicit_owners = FxHashMap::default();
        let mut generated_candidates = FxHashMap::<String, Vec<(usize, usize)>>::default();
        let mut warnings = Vec::new();

        for (module_index, module) in modules.iter().enumerate() {
            for (declaration_index, declaration) in module.declarations.iter().enumerate() {
                let name = declaration.name().to_string();
                match &declaration.item {
                    ExportedType::ExternalState(_) => {
                        generated_candidates
                            .entry(name)
                            .or_default()
                            .push((module_index, declaration_index));
                    }
                    _ => {
                        explicit_owners
                            .entry(name)
                            .or_insert_with(|| module.module_path.clone());
                    }
                }
            }
        }

        let mut generated_names_to_skip = FxHashSet::default();
        for (name, candidates) in &generated_candidates {
            if explicit_owners.contains_key(name) {
                generated_names_to_skip.insert(name.clone());
                for (module_index, declaration_index) in candidates {
                    let Some(ExportedType::ExternalState(state)) = modules[*module_index]
                        .declarations
                        .get(*declaration_index)
                        .map(|declaration| &declaration.item)
                    else {
                        continue;
                    };
                    warnings.push(format!(
                        "Skipping generated component state contract '{}' for external component '{}' because it conflicts with exported declaration '{}'",
                        state.name, state.component_name, state.name
                    ));
                }
                continue;
            }

            if candidates.len() > 1 {
                generated_names_to_skip.insert(name.clone());
                for (module_index, declaration_index) in candidates {
                    let Some(ExportedType::ExternalState(state)) = modules[*module_index]
                        .declarations
                        .get(*declaration_index)
                        .map(|declaration| &declaration.item)
                    else {
                        continue;
                    };
                    warnings.push(format!(
                        "Skipping generated component state contract '{}' for external component '{}' because another exported external component would generate the same companion name",
                        state.name, state.component_name
                    ));
                }
            }
        }

        let mut owners = FxHashMap::default();
        let mut filtered_modules = Vec::with_capacity(modules.len());

        for module in modules {
            let declarations = module
                .declarations
                .into_iter()
                .filter(|declaration| match &declaration.item {
                    ExportedType::ExternalState(state) => {
                        !generated_names_to_skip.contains(&state.name)
                    }
                    _ => true,
                })
                .collect::<Vec<_>>();

            if declarations.is_empty() {
                continue;
            }

            for declaration in &declarations {
                if !matches!(declaration.item, ExportedType::ExternalState(_)) {
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
                if matches!(declaration.item, ExportedType::ExternalState(_)) {
                    owners
                        .entry(declaration.name().to_string())
                        .or_insert_with(|| module.module_path.clone());
                }
            }
        }

        Ok(ExportedTypeGraphBuild {
            graph: Self {
                modules: filtered_modules,
                owners,
            },
            warnings,
        })
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

    fn records(&self) -> impl Iterator<Item = &ExportedRecord> {
        self.modules
            .iter()
            .flat_map(|module| module.declarations.iter())
            .filter_map(|declaration| match &declaration.item {
                ExportedType::Record(record) => Some(record),
                _ => None,
            })
    }
}

fn collect_exported_declarations(module: &LoweredModule) -> Vec<ExportedTypeDecl> {
    let mut declarations = Vec::new();

    for item in module.items() {
        if item.visibility() != Visibility::Export {
            continue;
        }

        match item {
            Item::TypeAlias(alias) => declarations.push(ExportedTypeDecl {
                visibility: alias.visibility,
                item: ExportedType::Alias(export_alias(alias)),
            }),
            Item::Enum(enum_def) => declarations.push(ExportedTypeDecl {
                visibility: enum_def.visibility,
                item: ExportedType::Enum(export_enum(enum_def)),
            }),
            Item::Record(record) => declarations.push(ExportedTypeDecl {
                visibility: record.visibility,
                item: ExportedType::Record(export_record(record)),
            }),
            Item::Component(component) => {
                if let Some(record) = export_external_component_contract(component) {
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
                            Some(imported_type) => imported_types.push(imported_type),
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

        ImportedTypesBuild {
            imported_types,
            warnings,
        }
    }

    fn load_dependency(&mut self, dependency_root: &Path) -> Result<CachedImportedLibrary, String> {
        if let Some(cached) = self.dependency_cache.get(dependency_root) {
            return cached.clone();
        }

        let loaded = build_cached_imported_library(dependency_root);
        self.dependency_cache
            .insert(dependency_root.to_path_buf(), loaded.clone());
        loaded
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
        let is_reference = match self.export_kinds.get(exported_name)? {
            PreparedItemKind::Enum => false,
            PreparedItemKind::Record | PreparedItemKind::Component => true,
            _ => return None,
        };

        Some(ImportedType {
            visible_name: visible_name.to_string(),
            exported_name: exported_name.to_string(),
            library_name: self.library_name.clone(),
            is_reference,
        })
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

fn build_cached_imported_library(dependency_root: &Path) -> Result<CachedImportedLibrary, String> {
    let dependency = build_library_artifact_from_directory(dependency_root).map_err(|error| {
        format!(
            "failed to build library artifact for '{}': {}",
            dependency_root.display(),
            error
        )
    })?;
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
    let mut wildcard_importable_export_names = Vec::new();

    for (exported_name, item_indices) in &dependency.exported_items {
        let Some(index) = item_indices.first() else {
            continue;
        };
        let Some(interface_item) = dependency.interface_items.get(*index) else {
            continue;
        };

        let kind = interface_item.item.kind();
        // Wildcard imports intentionally collect only the exports codegen can resolve as
        // cross-library type references; unsupported kinds are ignored instead of warned to
        // avoid noise from unrelated value/function exports.
        if matches!(
            kind,
            PreparedItemKind::Enum | PreparedItemKind::Record | PreparedItemKind::Component
        ) {
            wildcard_importable_export_names.push(exported_name.clone());
        }
        export_kinds.insert(exported_name.clone(), kind);
    }

    wildcard_importable_export_names.sort();

    Ok(CachedImportedLibrary {
        library_name,
        export_kinds,
        wildcard_importable_export_names,
    })
}

fn export_alias(def: &TypeAlias) -> ExportedAlias {
    ExportedAlias {
        name: def.name.as_str().to_string(),
        target: def.ty.clone(),
    }
}

fn export_enum(def: &EnumDef) -> ExportedEnum {
    ExportedEnum {
        name: def.name.as_str().to_string(),
        members: def
            .members
            .iter()
            .map(|member| member.name.as_str().to_string())
            .collect(),
    }
}

fn export_record(def: &RecordDef) -> ExportedRecord {
    ExportedRecord {
        name: def.name.as_str().to_string(),
        kind: def.kind,
        is_abstract: def.is_abstract,
        base: def.base.as_ref().map(|name| name.as_str().to_string()),
        fields: def
            .properties
            .iter()
            .map(|field| ExportedRecordField {
                name: field.name.as_str().to_string(),
                ty: field.ty.clone(),
                has_default: field.default.is_some(),
            })
            .collect(),
    }
}

fn export_external_component_contract(component: &Component) -> Option<ExportedRecord> {
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
            .map(|field| ExportedRecordField {
                name: field.name.as_str().to_string(),
                ty: field.ty.clone(),
                has_default: field.default.is_some(),
            })
            .collect(),
    })
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
                has_default: false,
            })
            .collect(),
    })
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

    #[test]
    fn resolves_abstract_record_alias_bases() {
        let source = r#"
            export abstract type Question = { label:string }
            export type QuestionBaseAlias = Question
            export type ShortTextQuestion extends QuestionBaseAlias = { placeholder:string? }
        "#;
        let module = lower_module(source, "types.nx");
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
        let module = lower_module(source, "components.nx");
        let graph = ExportedTypeGraph::from_module(&module, Path::new("components.nx")).unwrap();

        let declaration = graph
            .declaration("SearchBox_state")
            .expect("SearchBox_state declaration");
        match &declaration.item {
            ExportedType::ExternalState(state) => {
                assert_eq!(state.component_name, "SearchBox");
                assert_eq!(state.fields.len(), 1);
                assert_eq!(state.fields[0].name, "query");
                assert!(!state.fields[0].has_default);
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
        let module = lower_module(source, "components.nx");
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
        assert_eq!(build.warnings.len(), 2);
        assert!(build
            .warnings
            .iter()
            .all(|warning| warning.contains("SearchBox_state")));
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
}
