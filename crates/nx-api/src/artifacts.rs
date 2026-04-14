use nx_diagnostics::{Diagnostic, Label, Severity, TextSize, TextSpan};
use nx_hir::{
    ast::TypeRef, binding_specs_for_item, local_definition_id, lower, Import, ImportKind,
    ImportedRawRef, InterfaceField, InterfaceItem, InterfaceItemKind, InterfaceParam, Item,
    LocalDefinitionId, LoweredModule, LoweringDiagnostic, Name, PreparedBinding,
    PreparedBindingOrigin, PreparedBindingTarget, PreparedModule, PreparedNamespace, RecordField,
    SelectiveImport, SourceId, Visibility,
};
use nx_interpreter::{
    ModuleQualifiedItemRef, ResolvedItemKind, ResolvedModule, ResolvedProgram, RuntimeModuleId,
};
use nx_syntax::parse_str as syntax_parse_str;
use nx_types::{analyze_prepared_module, ModuleArtifact, Type, TypeEnvironment};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Export metadata for one symbol provided by a library artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryExport {
    pub module_file: String,
    pub item_name: String,
    pub definition_id: LocalDefinitionId,
    pub kind: ResolvedItemKind,
}

pub type LibraryInterfaceParam = InterfaceParam;
pub type LibraryInterfaceField = InterfaceField;
pub type LibraryInterfaceKind = InterfaceItemKind;
pub type LibraryInterfaceItem = InterfaceItem;

/// File-preserving artifact for one local NX library directory.
#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub root_path: PathBuf,
    pub modules: Vec<ModuleArtifact>,
    pub exports: FxHashMap<String, LibraryExport>,
    pub interface_items: Vec<LibraryInterfaceItem>,
    pub exported_items: FxHashMap<String, Vec<usize>>,
    pub visible_to_library_items: FxHashMap<String, Vec<usize>>,
    pub dependency_roots: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
}

/// File-preserving artifact for one resolved NX program.
#[derive(Debug, Clone)]
pub struct ProgramArtifact {
    pub root_modules: Vec<ModuleArtifact>,
    pub libraries: Vec<Arc<LibraryArtifact>>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
    pub resolved_program: ResolvedProgram,
}

#[derive(Debug, Default)]
struct LibraryRegistryState {
    libraries: FxHashMap<PathBuf, Arc<LibraryArtifact>>,
    dependency_graph: FxHashMap<PathBuf, Vec<PathBuf>>,
    loading: FxHashSet<PathBuf>,
}

/// Public owner of analyzed library snapshots.
#[derive(Debug, Clone, Default)]
pub struct LibraryRegistry {
    inner: Arc<RwLock<LibraryRegistryState>>,
}

impl LibraryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_library_from_directory(
        &self,
        root_path: impl AsRef<Path>,
    ) -> Result<Arc<LibraryArtifact>, Vec<crate::NxDiagnostic>> {
        let artifact = self
            .load_library_from_directory_internal(root_path.as_ref())
            .map_err(|error| {
                let diagnostic = Diagnostic::error("library-load-error")
                    .with_message(format!(
                        "Failed to load library artifact from '{}': {}",
                        root_path.as_ref().display(),
                        error
                    ))
                    .build();
                crate::diagnostics::diagnostics_to_api(&[diagnostic], "")
            })?;

        let diagnostics = self.closure_diagnostics(&artifact.root_path);
        if has_error_diagnostics(&diagnostics) {
            return Err(crate::diagnostics::diagnostics_to_api(&diagnostics, ""));
        }

        Ok(artifact)
    }

    pub fn build_context(&self) -> ProgramBuildContext {
        ProgramBuildContext {
            registry: self.clone(),
            visible_roots: self.loaded_roots().into_iter().collect(),
        }
    }

    pub fn build_context_with_visible_roots<I, P>(
        &self,
        roots: I,
    ) -> io::Result<ProgramBuildContext>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut visible_roots = FxHashSet::default();
        for root in roots {
            visible_roots.insert(fs::canonicalize(root.as_ref())?);
        }

        Ok(ProgramBuildContext {
            registry: self.clone(),
            visible_roots,
        })
    }

    fn loaded_roots(&self) -> Vec<PathBuf> {
        let state = self.inner.read().expect("library registry lock poisoned");
        let mut roots = state.libraries.keys().cloned().collect::<Vec<_>>();
        roots.sort();
        roots
    }

    fn get_loaded_library(&self, root: &Path) -> Option<Arc<LibraryArtifact>> {
        let state = self.inner.read().expect("library registry lock poisoned");
        state.libraries.get(root).cloned()
    }

    fn load_library_from_directory_internal(
        &self,
        root_path: &Path,
    ) -> io::Result<Arc<LibraryArtifact>> {
        let mut loading_stack = Vec::new();
        self.load_library_from_directory_internal_with_stack(root_path, &mut loading_stack)
    }

    fn load_library_from_directory_internal_with_stack(
        &self,
        root_path: &Path,
        loading_stack: &mut Vec<PathBuf>,
    ) -> io::Result<Arc<LibraryArtifact>> {
        let root_path = fs::canonicalize(root_path)?;
        if let Some(existing) = self.get_loaded_library(&root_path) {
            return Ok(existing);
        }

        if let Some(index) = loading_stack.iter().position(|path| path == &root_path) {
            let mut cycle = loading_stack[index..].to_vec();
            cycle.push(root_path.clone());
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                circular_library_dependency_message(&cycle),
            ));
        }

        {
            let mut state = self.inner.write().expect("library registry lock poisoned");
            if !state.loading.insert(root_path.clone()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Circular library dependency detected while loading '{}'",
                        root_path.display()
                    ),
                ));
            }
        }

        loading_stack.push(root_path.clone());
        let result = (|| {
            let dependency_roots = discover_library_dependency_roots(&root_path)?;
            for dependency_root in &dependency_roots {
                let _ = self.load_library_from_directory_internal_with_stack(
                    dependency_root,
                    loading_stack,
                )?;
            }

            let artifact = Arc::new(build_library_artifact_with_registry(&root_path, self)?);
            let mut state = self.inner.write().expect("library registry lock poisoned");
            let entry = state
                .libraries
                .entry(root_path.clone())
                .or_insert_with(|| artifact.clone())
                .clone();
            state
                .dependency_graph
                .insert(root_path.clone(), artifact.dependency_roots.clone());
            Ok(entry)
        })();

        let popped = loading_stack.pop();
        debug_assert_eq!(popped.as_deref(), Some(root_path.as_path()));

        let mut state = self.inner.write().expect("library registry lock poisoned");
        state.loading.remove(&root_path);
        result
    }

    fn dependency_roots(&self, root: &Path) -> Vec<PathBuf> {
        let state = self.inner.read().expect("library registry lock poisoned");
        state
            .dependency_graph
            .get(root)
            .cloned()
            .unwrap_or_default()
    }

    fn closure_diagnostics(&self, root: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen = FxHashSet::default();
        let mut queue = vec![root.to_path_buf()];

        while let Some(current) = queue.pop() {
            if !seen.insert(current.clone()) {
                continue;
            }

            if let Some(library) = self.get_loaded_library(&current) {
                diagnostics.extend(library.diagnostics.iter().cloned());
            }

            queue.extend(self.dependency_roots(&current));
        }

        diagnostics
    }
}

/// Registry-backed build scope used for one program build or tenant.
#[derive(Debug, Clone)]
pub struct ProgramBuildContext {
    registry: LibraryRegistry,
    visible_roots: FxHashSet<PathBuf>,
}

impl Default for ProgramBuildContext {
    fn default() -> Self {
        Self::empty()
    }
}

impl ProgramBuildContext {
    pub fn empty() -> Self {
        Self {
            registry: LibraryRegistry::new(),
            visible_roots: FxHashSet::default(),
        }
    }

    pub fn from_registry(registry: &LibraryRegistry) -> Self {
        registry.build_context()
    }

    fn visible_library(&self, root: &Path) -> Option<Arc<LibraryArtifact>> {
        if !self.visible_roots.contains(root) {
            return None;
        }

        self.registry.get_loaded_library(root)
    }
}

#[derive(Debug, Clone)]
struct LibrarySourceFile {
    file_name: String,
    path: PathBuf,
    source: String,
    source_id: SourceId,
    diagnostics: Vec<Diagnostic>,
    preserved_module: Option<LoweredModule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LocalLibraryItemRef {
    file_index: usize,
    item_index: usize,
}

#[derive(Debug, Clone)]
struct LocalLibraryCandidate {
    source_file: PathBuf,
    item: LocalLibraryItemRef,
}

#[derive(Debug, Clone)]
struct ResolvedBuildContextImport {
    normalized_root: PathBuf,
    library: Arc<LibraryArtifact>,
}

#[derive(Debug, Clone)]
struct PendingProgramLibrary {
    root: PathBuf,
    chain: Vec<PathBuf>,
    library: Option<Arc<LibraryArtifact>>,
}

#[derive(Debug, Default)]
struct ProgramLibrarySelection {
    libraries: Vec<Arc<LibraryArtifact>>,
    diagnostics: Vec<Diagnostic>,
}

struct PreparedRootModule {
    artifact: ModuleArtifact,
    libraries: Vec<Arc<LibraryArtifact>>,
    diagnostics: Vec<Diagnostic>,
}

/// Builds a file-preserving library artifact from a local directory.
pub fn build_library_artifact_from_directory(
    root_path: impl AsRef<Path>,
) -> io::Result<LibraryArtifact> {
    let registry = LibraryRegistry::new();
    let artifact = registry.load_library_from_directory_internal(root_path.as_ref())?;
    Ok((*artifact).clone())
}

fn build_library_artifact_with_registry(
    root_path: &Path,
    registry: &LibraryRegistry,
) -> io::Result<LibraryArtifact> {
    let root_path = fs::canonicalize(root_path)?;
    let source_files = read_library_source_files(&root_path)?;
    let mut hasher = DefaultHasher::new();
    root_path.hash(&mut hasher);

    let mut dependency_roots = FxHashSet::default();
    for source_file in &source_files {
        source_file.path.hash(&mut hasher);
        source_file.source.hash(&mut hasher);

        if let Some(module) = source_file.preserved_module.as_ref() {
            collect_library_dependencies(&module.imports, &source_file.path, &mut dependency_roots);
        }
    }

    let mut dependency_roots = dependency_roots.into_iter().collect::<Vec<_>>();
    dependency_roots.sort();

    let dependency_context = registry.build_context();

    let mut modules = Vec::with_capacity(source_files.len());
    let mut exports = FxHashMap::default();
    let mut interface_items = Vec::new();
    let mut exported_items = FxHashMap::default();
    let mut visible_to_library_items = FxHashMap::default();
    let mut diagnostics = Vec::new();

    for (index, source_file) in source_files.iter().enumerate() {
        let artifact =
            analyze_library_source_file(&root_path, &source_files, &dependency_context, index);
        diagnostics.extend(artifact.diagnostics.iter().cloned());

        if let Some(module) = artifact.lowered_module.as_ref() {
            for (item_index, item) in module.items().iter().enumerate() {
                if let Some(interface_item) = build_interface_item(&artifact, item_index, item) {
                    let interface_index = interface_items.len();
                    if item.visibility() != Visibility::Private {
                        visible_to_library_items
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(Vec::new)
                            .push(interface_index);
                    }
                    if item.visibility() == Visibility::Export {
                        exported_items
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(Vec::new)
                            .push(interface_index);
                        exports
                            .entry(item.name().as_str().to_string())
                            .or_insert_with(|| LibraryExport {
                                module_file: artifact.file_name.clone(),
                                item_name: interface_item.item_name.clone(),
                                definition_id: interface_item.definition_id,
                                kind: resolved_item_kind_from_interface(interface_item.item.kind()),
                            });
                    }
                    interface_items.push(interface_item);
                }
            }
        }

        let _ = source_file;
        modules.push(artifact);
    }

    Ok(LibraryArtifact {
        root_path,
        modules,
        exports,
        interface_items,
        exported_items,
        visible_to_library_items,
        dependency_roots,
        diagnostics,
        fingerprint: hasher.finish(),
    })
}

fn discover_library_dependency_roots(root_path: &Path) -> io::Result<Vec<PathBuf>> {
    let source_files = read_library_source_files(root_path)?;
    let mut dependency_roots = FxHashSet::default();

    for source_file in &source_files {
        if let Some(module) = source_file.preserved_module.as_ref() {
            collect_library_dependencies(&module.imports, &source_file.path, &mut dependency_roots);
        }
    }

    let mut dependency_roots = dependency_roots.into_iter().collect::<Vec<_>>();
    dependency_roots.sort();
    Ok(dependency_roots)
}

fn circular_library_dependency_message(cycle: &[PathBuf]) -> String {
    let chain = cycle
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    format!("Circular library dependency detected: {}", chain)
}

fn read_library_source_files(root_path: &Path) -> io::Result<Vec<LibrarySourceFile>> {
    let mut source_paths = Vec::new();
    collect_nx_files(root_path, &mut source_paths)?;
    source_paths.sort();

    let mut source_files = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        let source = fs::read_to_string(&source_path)?;
        let file_name = source_path.display().to_string();
        let parse_result = syntax_parse_str(&source, &file_name);
        let source_id = SourceId::new(parse_result.source_id.as_u32());
        let diagnostics = normalize_diagnostics_file_name(parse_result.errors, &file_name);
        let preserved_module = parse_result.tree.map(|tree| lower(tree.root(), source_id));

        source_files.push(LibrarySourceFile {
            file_name,
            path: source_path,
            source,
            source_id,
            diagnostics,
            preserved_module,
        });
    }

    Ok(source_files)
}

fn analyze_library_source_file(
    library_root: &Path,
    source_files: &[LibrarySourceFile],
    dependency_context: &ProgramBuildContext,
    current_file_index: usize,
) -> ModuleArtifact {
    let source_file = &source_files[current_file_index];
    let diagnostics = source_file.diagnostics.clone();
    let Some(preserved_module) = source_file.preserved_module.clone() else {
        return parse_failure_artifact(&source_file.file_name, source_file.source_id, diagnostics);
    };
    let mut prepared_module = PreparedModule::new(&source_file.file_name, preserved_module);
    apply_build_context_imports(
        &mut prepared_module,
        &source_file.path,
        dependency_context,
        &source_file.source,
    );
    apply_current_library_items(
        &mut prepared_module,
        library_root,
        source_files,
        current_file_index,
    );
    finalize_module_artifact(&source_file.file_name, prepared_module, diagnostics)
}

fn apply_current_library_items(
    module: &mut PreparedModule,
    library_root: &Path,
    source_files: &[LibrarySourceFile],
    current_file_index: usize,
) {
    let mut peer_candidates =
        FxHashMap::<(PreparedNamespace, String), Vec<LocalLibraryCandidate>>::default();

    for (file_index, source_file) in source_files.iter().enumerate() {
        if file_index == current_file_index {
            continue;
        }

        let Some(peer_module) = source_file.preserved_module.as_ref() else {
            continue;
        };
        module.add_peer_module(source_file.file_name.clone(), Arc::new(peer_module.clone()));

        for (item_index, item) in peer_module.items().iter().enumerate() {
            if item.visibility() == Visibility::Private {
                continue;
            }

            for (namespace, _) in binding_specs_for_item(item) {
                peer_candidates
                    .entry((namespace, item.name().as_str().to_string()))
                    .or_default()
                    .push(LocalLibraryCandidate {
                        source_file: source_file.path.clone(),
                        item: LocalLibraryItemRef {
                            file_index,
                            item_index,
                        },
                    });
            }
        }
    }

    let mut visible_names = peer_candidates.keys().cloned().collect::<Vec<_>>();
    visible_names.sort_by(|lhs, rhs| {
        lhs.1
            .cmp(&rhs.1)
            .then_with(|| namespace_order(lhs.0).cmp(&namespace_order(rhs.0)))
    });

    for (namespace, visible_name) in visible_names {
        let visible_name_ref = Name::new(&visible_name);
        if module.has_binding(namespace, &visible_name_ref) {
            continue;
        }

        let candidates = peer_candidates
            .remove(&(namespace, visible_name.clone()))
            .expect("visible name was collected from peer candidates");
        if candidates.len() != 1 {
            let sources = candidates
                .iter()
                .map(|candidate| {
                    candidate
                        .source_file
                        .strip_prefix(library_root)
                        .unwrap_or(candidate.source_file.as_path())
                        .display()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Library item '{}' is defined in multiple files ({}). Use unique names within one library.",
                    visible_name, sources
                ),
                span: TextSpan::default(),
            });
            continue;
        }

        let candidate = &candidates[0];
        let peer_module = source_files[candidate.item.file_index]
            .preserved_module
            .as_ref()
            .expect("peer prepared module should exist");
        let item = &peer_module.items()[candidate.item.item_index];
        let kind = binding_specs_for_item(item)
            .into_iter()
            .find(|(candidate_namespace, _)| *candidate_namespace == namespace)
            .map(|(_, kind)| kind)
            .expect("peer binding namespace should match item kind");
        let target_module_identity = source_files[candidate.item.file_index].file_name.clone();
        module.insert_binding(PreparedBinding {
            visible_name: visible_name_ref,
            namespace,
            kind,
            origin: PreparedBindingOrigin::Peer {
                module_identity: target_module_identity.clone(),
            },
            target: PreparedBindingTarget::Peer {
                module_identity: target_module_identity,
                definition_id: local_definition_id(candidate.item.item_index),
            },
        });
    }
}

/// Builds a file-preserving program artifact from source text against a caller-supplied build context.
pub fn build_program_artifact_from_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
) -> io::Result<ProgramArtifact> {
    let source_path = Path::new(file_name);
    let root_path = source_path.exists().then_some(source_path);
    let PreparedRootModule {
        artifact: root_artifact,
        mut libraries,
        diagnostics: mut selection_diagnostics,
    } = prepare_root_module_with_context(source, file_name, root_path, build_context);

    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    source.hash(&mut hasher);

    if root_path.is_some() {
        libraries.sort_by(|lhs, rhs| lhs.root_path.cmp(&rhs.root_path));
        for library in &libraries {
            hasher.write_u64(library.fingerprint);
        }
    }

    let root_modules = vec![root_artifact];
    let mut diagnostics = root_modules[0].diagnostics.clone();
    diagnostics.append(&mut selection_diagnostics);
    for library in &libraries {
        diagnostics.extend(library.diagnostics.iter().cloned());
    }

    let fingerprint = hasher.finish();
    let resolved_program = build_resolved_program(&root_modules, &libraries, fingerprint);

    Ok(ProgramArtifact {
        root_modules,
        libraries,
        diagnostics,
        fingerprint,
        resolved_program,
    })
}

fn prepare_root_module_with_context(
    source: &str,
    file_name: &str,
    source_path: Option<&Path>,
    build_context: &ProgramBuildContext,
) -> PreparedRootModule {
    let parse_result = syntax_parse_str(source, file_name);
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let mut diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return PreparedRootModule {
            artifact: parse_failure_artifact(file_name, source_id, diagnostics),
            libraries: Vec::new(),
            diagnostics: Vec::new(),
        };
    };

    let preserved_module = lower(tree.root(), source_id);
    let mut prepared_module = PreparedModule::new(file_name, preserved_module);
    let mut libraries = Vec::new();
    let mut selection_diagnostics = Vec::new();

    if let Some(path) = source_path {
        let resolved_imports =
            apply_build_context_imports(&mut prepared_module, path, build_context, source);
        let selection =
            selected_program_libraries(&resolved_imports, file_name, source, build_context);
        libraries = selection.libraries;
        selection_diagnostics = selection.diagnostics;
    } else if !prepared_module.raw_module().imports.is_empty() {
        diagnostics.push(library_imports_require_path_diagnostic(source, file_name));
    }

    PreparedRootModule {
        artifact: finalize_module_artifact(file_name, prepared_module, diagnostics),
        libraries,
        diagnostics: selection_diagnostics,
    }
}

fn parse_failure_artifact(
    file_name: &str,
    source_id: SourceId,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    ModuleArtifact {
        file_name: file_name.to_string(),
        source_id,
        parse_succeeded: false,
        lowered_module: None,
        type_env: TypeEnvironment::new(),
        diagnostics,
        imports: Vec::new(),
    }
}

fn finalize_module_artifact(
    file_name: &str,
    prepared_module: PreparedModule,
    diagnostics: Vec<Diagnostic>,
) -> ModuleArtifact {
    analyze_prepared_module(file_name, prepared_module, diagnostics)
}

fn apply_build_context_imports(
    module: &mut PreparedModule,
    root_path: &Path,
    build_context: &ProgramBuildContext,
    source: &str,
) -> Vec<ResolvedBuildContextImport> {
    let root_path = match fs::canonicalize(root_path) {
        Ok(root_path) => root_path,
        Err(error) => {
            if module.raw_module().imports.iter().any(|import| {
                !is_git_library_path(&import.library_path)
                    && !is_http_library_path(&import.library_path)
            }) {
                module.add_diagnostic(LoweringDiagnostic {
                    message: format!(
                        "Local library import resolution was skipped because source file path '{}' could not be resolved: {}",
                        root_path.display(),
                        error
                    ),
                    span: full_source_span(source),
                });
            }
            return Vec::new();
        }
    };

    let mut seen_import_roots = FxHashMap::default();
    let mut imported_visible_names = FxHashMap::default();
    let mut resolved_imports = Vec::new();

    for import in module.raw_module().imports.clone() {
        if is_git_library_path(&import.library_path) {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Git library imports are not yet supported: '{}'",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        if is_http_library_path(&import.library_path) {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "HTTP zip library imports are not yet supported: '{}'",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        let normalized_root = match normalize_local_library_path(&root_path, &import.library_path) {
            Ok(path) => path,
            Err(_) => {
                module.add_diagnostic(LoweringDiagnostic {
                    message: format!(
                        "Local library import '{}' could not be resolved to a directory",
                        import.library_path
                    ),
                    span: import.span,
                });
                continue;
            }
        };

        if !normalized_root.is_dir() {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Local library import '{}' must resolve to a directory",
                    import.library_path
                ),
                span: import.span,
            });
            continue;
        }

        let normalized_key = normalized_root.to_string_lossy().to_string();
        if let Some(first_import_span) = seen_import_roots.insert(normalized_key, import.span) {
            let first_import_location = line_col_for_span(source, first_import_span)
                .map(|(line, column)| {
                    format!("; first imported at line {}, column {}", line, column)
                })
                .unwrap_or_default();
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Library '{}' is imported more than once in this file{}",
                    normalized_root.display(),
                    first_import_location
                ),
                span: import.span,
            });
            continue;
        }

        let Some(library) = build_context.visible_library(&normalized_root) else {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Missing loaded library '{}' in the supplied build context",
                    normalized_root.display()
                ),
                span: import.span,
            });
            continue;
        };

        resolved_imports.push(ResolvedBuildContextImport {
            normalized_root: normalized_root.clone(),
            library: library.clone(),
        });

        match &import.kind {
            ImportKind::Wildcard { alias } => {
                let mut export_names = library.exported_items.keys().cloned().collect::<Vec<_>>();
                export_names.sort();

                for export_name in export_names {
                    let visible_name = alias
                        .as_ref()
                        .map(|prefix| format!("{}.{}", prefix.as_str(), export_name))
                        .unwrap_or_else(|| export_name.clone());

                    let Some(item_indices) = library.exported_items.get(&export_name) else {
                        continue;
                    };
                    add_imported_interface_bindings(
                        module,
                        &visible_name,
                        import.span,
                        &library,
                        item_indices,
                        &mut imported_visible_names,
                    );
                }
            }
            ImportKind::Selective { entries } => {
                for entry in entries {
                    let Some(item_indices) = library.exported_items.get(entry.name.as_str()) else {
                        module.add_diagnostic(LoweringDiagnostic {
                            message: format!(
                                "Library '{}' does not export '{}'",
                                normalized_root.display(),
                                entry.name.as_str()
                            ),
                            span: entry.span,
                        });
                        continue;
                    };

                    let visible_name = visible_name_for_selective(entry);
                    add_imported_interface_bindings(
                        module,
                        &visible_name,
                        entry.span,
                        &library,
                        item_indices,
                        &mut imported_visible_names,
                    );
                }
            }
        }
    }

    resolved_imports
}

fn add_ambiguous_interface_diagnostic(
    module: &mut PreparedModule,
    visible_name: &str,
    span: TextSpan,
    library: &LibraryArtifact,
    item_indices: &[usize],
) {
    let sources = item_indices
        .iter()
        .take(2)
        .map(|index| interface_origin(&library.root_path, &library.interface_items[*index]))
        .collect::<Vec<_>>()
        .join(" and ");

    module.add_diagnostic(LoweringDiagnostic {
        message: format!(
            "Ambiguous imported name '{}' could refer to {}. Use a more specific import alias.",
            visible_name, sources
        ),
        span,
    });
}

fn add_imported_interface_bindings(
    module: &mut PreparedModule,
    visible_name: &str,
    span: TextSpan,
    library: &LibraryArtifact,
    item_indices: &[usize],
    imported_visible_names: &mut FxHashMap<(PreparedNamespace, String), String>,
) {
    for artifact in &library.modules {
        if let Some(lowered_module) = artifact.lowered_module.as_ref() {
            module.add_peer_module(artifact.file_name.clone(), lowered_module.clone());
        }
    }

    let mut candidates = FxHashMap::<PreparedNamespace, Vec<usize>>::default();

    for item_index in item_indices {
        let interface_item = &library.interface_items[*item_index];
        let kind = interface_item.item.kind();
        for namespace in kind.namespaces() {
            candidates.entry(*namespace).or_default().push(*item_index);
        }
    }

    let mut namespaces = candidates.keys().copied().collect::<Vec<_>>();
    namespaces.sort_by_key(|namespace| namespace_order(*namespace));

    for namespace in namespaces {
        let Some(indices) = candidates.remove(&namespace) else {
            continue;
        };
        let visible_name_ref = Name::new(visible_name);
        if module.has_binding(namespace, &visible_name_ref) {
            continue;
        }

        if indices.len() != 1 {
            add_ambiguous_interface_diagnostic(module, visible_name, span, library, &indices);
            continue;
        }

        if let Some(previous_origin) =
            imported_visible_names.get(&(namespace, visible_name.to_string()))
        {
            module.add_diagnostic(LoweringDiagnostic {
                message: format!(
                    "Imported name '{}' is provided by both {} and {}. Use aliases to disambiguate.",
                    visible_name,
                    previous_origin,
                    library.root_path.display()
                ),
                span,
            });
            continue;
        }

        let interface_item = library.interface_items[indices[0]].clone();
        let kind = interface_item.item.kind();
        module.insert_binding(PreparedBinding {
            visible_name: visible_name_ref,
            namespace,
            kind,
            origin: PreparedBindingOrigin::Imported {
                module_identity: interface_item.module_identity.clone(),
            },
            target: PreparedBindingTarget::Imported {
                item: interface_item,
                raw: Some(ImportedRawRef {
                    module_identity: library.interface_items[indices[0]].module_identity.clone(),
                    definition_id: library.interface_items[indices[0]].definition_id,
                }),
            },
        });
        imported_visible_names.insert(
            (namespace, visible_name.to_string()),
            library.root_path.display().to_string(),
        );
    }
}

fn namespace_order(namespace: PreparedNamespace) -> u8 {
    match namespace {
        PreparedNamespace::Value => 0,
        PreparedNamespace::Type => 1,
        PreparedNamespace::Element => 2,
    }
}

fn selected_program_libraries(
    direct_imports: &[ResolvedBuildContextImport],
    file_name: &str,
    source: &str,
    build_context: &ProgramBuildContext,
) -> ProgramLibrarySelection {
    let mut selection = ProgramLibrarySelection::default();
    let mut seen_library_roots = FxHashSet::default();
    let mut reported_missing_roots = FxHashSet::default();
    let mut queue = direct_imports
        .iter()
        .map(|import| PendingProgramLibrary {
            root: import.normalized_root.clone(),
            chain: vec![import.normalized_root.clone()],
            library: Some(import.library.clone()),
        })
        .collect::<Vec<_>>();
    queue.sort_by(|lhs, rhs| rhs.root.cmp(&lhs.root));

    while let Some(pending) = queue.pop() {
        if !seen_library_roots.insert(pending.root.clone()) {
            continue;
        }

        let Some(library) = pending
            .library
            .or_else(|| build_context.registry.get_loaded_library(&pending.root))
        else {
            if reported_missing_roots.insert(pending.root.clone()) {
                selection
                    .diagnostics
                    .push(incomplete_library_dependency_closure_diagnostic(
                        file_name,
                        source,
                        &pending.chain,
                    ));
            }
            continue;
        };

        let mut dependency_roots = library.dependency_roots.clone();
        dependency_roots.sort();
        for dependency_root in dependency_roots {
            if !seen_library_roots.contains(&dependency_root) {
                let mut chain = pending.chain.clone();
                chain.push(dependency_root.clone());
                queue.push(PendingProgramLibrary {
                    root: dependency_root,
                    chain,
                    library: None,
                });
            }
        }
        queue.sort_by(|lhs, rhs| rhs.root.cmp(&lhs.root));
        selection.libraries.push(library);
    }

    selection
}

fn build_resolved_program(
    root_modules: &[ModuleArtifact],
    libraries: &[Arc<LibraryArtifact>],
    fingerprint: u64,
) -> ResolvedProgram {
    let mut modules = Vec::new();
    let mut module_ids = FxHashMap::default();
    let mut root_module_ids = Vec::new();

    for artifact in root_modules {
        let Some(module) = artifact.lowered_module.clone() else {
            continue;
        };

        let module_id = RuntimeModuleId::new(modules.len() as u32);
        module_ids.insert(artifact.file_name.clone(), module_id);
        root_module_ids.push(module_id);
        modules.push(ResolvedModule {
            id: module_id,
            identity: artifact.file_name.clone(),
            lowered_module: module,
        });
    }

    for library in libraries {
        for artifact in &library.modules {
            let Some(module) = artifact.lowered_module.clone() else {
                continue;
            };

            let module_id = RuntimeModuleId::new(modules.len() as u32);
            module_ids.insert(artifact.file_name.clone(), module_id);
            modules.push(ResolvedModule {
                id: module_id,
                identity: artifact.file_name.clone(),
                lowered_module: module,
            });
        }
    }

    let mut entry_functions = FxHashMap::default();
    let mut entry_components = FxHashMap::default();
    let mut entry_records = FxHashMap::default();
    let mut entry_enums = FxHashMap::default();
    let mut imports = FxHashMap::default();

    for artifact in root_modules {
        let Some(module) = artifact.lowered_module.as_ref() else {
            continue;
        };
        let Some(&module_id) = module_ids.get(&artifact.file_name) else {
            continue;
        };

        for (item_index, item) in module.items().iter().enumerate() {
            insert_entry_symbol(
                &mut entry_functions,
                &mut entry_components,
                &mut entry_records,
                &mut entry_enums,
                item.name().as_str(),
                ModuleQualifiedItemRef {
                    module_id,
                    definition_id: local_definition_id(item_index),
                    kind: resolved_item_kind(item),
                },
            );
        }
    }

    for library in libraries {
        for (export_name, export) in &library.exports {
            let Some(&module_id) = module_ids.get(&export.module_file) else {
                continue;
            };

            insert_entry_symbol(
                &mut entry_functions,
                &mut entry_components,
                &mut entry_records,
                &mut entry_enums,
                export_name,
                ModuleQualifiedItemRef {
                    module_id,
                    definition_id: export.definition_id,
                    kind: export.kind,
                },
            );
        }
    }

    let library_by_root = libraries
        .iter()
        .map(|library| (library.root_path.clone(), library.clone()))
        .collect::<FxHashMap<_, _>>();
    let library_by_module_file = libraries
        .iter()
        .flat_map(|library| {
            library
                .modules
                .iter()
                .map(move |artifact| (artifact.file_name.clone(), library.clone()))
        })
        .collect::<FxHashMap<_, _>>();

    for artifact in root_modules
        .iter()
        .chain(libraries.iter().flat_map(|library| library.modules.iter()))
    {
        let Some(&module_id) = module_ids.get(&artifact.file_name) else {
            continue;
        };
        let module_file = Path::new(&artifact.file_name);
        let mut visible_imports = FxHashMap::default();

        for import in &artifact.imports {
            let Some(normalized_root) =
                normalize_supported_library_path(module_file, &import.library_path)
            else {
                continue;
            };
            let Some(library) = library_by_root.get(&normalized_root) else {
                continue;
            };

            match &import.kind {
                ImportKind::Wildcard { alias } => {
                    for (export_name, export) in &library.exports {
                        let visible_name = alias
                            .as_ref()
                            .map(|prefix| format!("{}.{}", prefix.as_str(), export_name))
                            .unwrap_or_else(|| export_name.clone());
                        let Some(&target_module_id) = module_ids.get(&export.module_file) else {
                            continue;
                        };

                        visible_imports.entry(visible_name).or_insert_with(|| {
                            ModuleQualifiedItemRef {
                                module_id: target_module_id,
                                definition_id: export.definition_id,
                                kind: export.kind,
                            }
                        });
                    }
                }
                ImportKind::Selective { entries } => {
                    for entry in entries {
                        let Some(export) = library.exports.get(entry.name.as_str()) else {
                            continue;
                        };
                        let Some(&target_module_id) = module_ids.get(&export.module_file) else {
                            continue;
                        };

                        visible_imports
                            .entry(visible_name_for_selective(entry))
                            .or_insert_with(|| ModuleQualifiedItemRef {
                                module_id: target_module_id,
                                definition_id: export.definition_id,
                                kind: export.kind,
                            });
                    }
                }
            }
        }

        if let Some(library) = library_by_module_file.get(&artifact.file_name) {
            let mut local_item_names = artifact
                .lowered_module
                .as_ref()
                .map(|module| {
                    module
                        .items()
                        .iter()
                        .map(|item| item.name().as_str().to_string())
                        .collect::<FxHashSet<_>>()
                })
                .unwrap_or_default();
            let mut visible_names = library
                .visible_to_library_items
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            visible_names.sort();

            for visible_name in visible_names {
                if local_item_names.contains(&visible_name) {
                    continue;
                }

                let Some(item_indices) = library.visible_to_library_items.get(&visible_name) else {
                    continue;
                };
                if item_indices.len() != 1 {
                    continue;
                }

                let interface_item = &library.interface_items[item_indices[0]];
                let Some(&target_module_id) = module_ids.get(&interface_item.module_identity)
                else {
                    continue;
                };

                visible_imports
                    .entry(visible_name.clone())
                    .or_insert_with(|| ModuleQualifiedItemRef {
                        module_id: target_module_id,
                        definition_id: interface_item.definition_id,
                        kind: resolved_item_kind_from_interface(interface_item.item.kind()),
                    });
                local_item_names.insert(visible_name);
            }
        }

        if !visible_imports.is_empty() {
            imports.insert(module_id, visible_imports);
        }
    }

    ResolvedProgram::new(
        fingerprint,
        root_module_ids,
        modules,
        entry_functions,
        entry_components,
        entry_records,
        entry_enums,
        imports,
    )
}

fn build_interface_item(
    artifact: &ModuleArtifact,
    item_index: usize,
    item: &Item,
) -> Option<LibraryInterfaceItem> {
    let item_name = item.name().as_str().to_string();
    let visibility = item.visibility();
    let item = match item {
        Item::Function(function) => {
            let return_type = function.return_type.clone().or_else(|| {
                artifact
                    .type_env
                    .lookup(&function.name)
                    .and_then(type_from_function_binding)
            })?;

            LibraryInterfaceKind::Function {
                params: function
                    .params
                    .iter()
                    .map(|param| LibraryInterfaceParam {
                        name: param.name.clone(),
                        ty: param.ty.clone(),
                        is_content: param.is_content,
                        span: param.span,
                    })
                    .collect(),
                return_type,
                span: function.span,
            }
        }
        Item::Value(value) => {
            let ty = value.ty.clone().or_else(|| {
                artifact
                    .type_env
                    .lookup(&value.name)
                    .and_then(type_to_type_ref)
            })?;

            LibraryInterfaceKind::Value {
                ty,
                span: value.span,
            }
        }
        Item::Component(component) => LibraryInterfaceKind::Component {
            is_abstract: component.is_abstract,
            is_external: component.is_external,
            base: component.base.clone(),
            props: component
                .props
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            emits: component.emits.clone(),
            state: component
                .state
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            span: component.span,
        },
        Item::TypeAlias(type_alias) => LibraryInterfaceKind::TypeAlias {
            ty: type_alias.ty.clone(),
            span: type_alias.span,
        },
        Item::Enum(enum_def) => LibraryInterfaceKind::Enum {
            members: enum_def.members.clone(),
            span: enum_def.span,
        },
        Item::Record(record_def) => LibraryInterfaceKind::Record {
            kind: record_def.kind,
            is_abstract: record_def.is_abstract,
            base: record_def.base.clone(),
            properties: record_def
                .properties
                .iter()
                .map(record_field_to_interface_field)
                .collect(),
            span: record_def.span,
        },
    };

    Some(LibraryInterfaceItem {
        module_identity: artifact.file_name.clone(),
        item_name,
        definition_id: local_definition_id(item_index),
        visibility,
        item,
    })
}

fn record_field_to_interface_field(field: &RecordField) -> LibraryInterfaceField {
    LibraryInterfaceField {
        name: field.name.clone(),
        ty: field.ty.clone(),
        is_content: field.is_content,
        is_required: field.default.is_none() && !matches!(field.ty, TypeRef::Nullable(_)),
        span: field.span,
    }
}

fn type_from_function_binding(ty: &Type) -> Option<TypeRef> {
    match ty {
        Type::Function { ret, .. } => type_to_type_ref(ret),
        _ => None,
    }
}

fn type_to_type_ref(ty: &Type) -> Option<TypeRef> {
    match ty {
        Type::Primitive(primitive) => Some(TypeRef::name(primitive.as_str())),
        Type::Array(inner) => Some(TypeRef::array(type_to_type_ref(inner)?)),
        Type::Nullable(inner) => Some(TypeRef::nullable(type_to_type_ref(inner)?)),
        Type::Function { params, ret } => Some(TypeRef::function(
            params
                .iter()
                .map(type_to_type_ref)
                .collect::<Option<Vec<_>>>()?,
            type_to_type_ref(ret)?,
        )),
        Type::Named(name) => Some(TypeRef::name(name.clone())),
        Type::Enum(enum_type) => Some(TypeRef::name(enum_type.name.clone())),
        Type::Variable(_) | Type::Unknown | Type::Error => None,
    }
}

fn insert_entry_symbol(
    entry_functions: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_components: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_records: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    entry_enums: &mut FxHashMap<String, ModuleQualifiedItemRef>,
    visible_name: &str,
    item_ref: ModuleQualifiedItemRef,
) {
    match item_ref.kind {
        ResolvedItemKind::Function => {
            entry_functions
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Component => {
            entry_components
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Record | ResolvedItemKind::TypeAlias | ResolvedItemKind::Value => {
            entry_records
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
        ResolvedItemKind::Enum => {
            entry_enums
                .entry(visible_name.to_string())
                .or_insert(item_ref);
        }
    }
}

fn resolved_item_kind_from_interface(kind: nx_hir::PreparedItemKind) -> ResolvedItemKind {
    match kind {
        nx_hir::PreparedItemKind::Function => ResolvedItemKind::Function,
        nx_hir::PreparedItemKind::Value => ResolvedItemKind::Value,
        nx_hir::PreparedItemKind::Component => ResolvedItemKind::Component,
        nx_hir::PreparedItemKind::TypeAlias => ResolvedItemKind::TypeAlias,
        nx_hir::PreparedItemKind::Enum => ResolvedItemKind::Enum,
        nx_hir::PreparedItemKind::Record => ResolvedItemKind::Record,
    }
}

fn resolved_item_kind(item: &Item) -> ResolvedItemKind {
    match item {
        Item::Function(_) => ResolvedItemKind::Function,
        Item::Value(_) => ResolvedItemKind::Value,
        Item::Component(_) => ResolvedItemKind::Component,
        Item::TypeAlias(_) => ResolvedItemKind::TypeAlias,
        Item::Enum(_) => ResolvedItemKind::Enum,
        Item::Record(_) => ResolvedItemKind::Record,
    }
}

fn interface_origin(root_path: &Path, item: &LibraryInterfaceItem) -> String {
    let item_path = Path::new(&item.module_identity);
    let relative = item_path.strip_prefix(root_path).unwrap_or(item_path);
    format!("{} ({})", item.item_name, relative.display())
}

fn collect_library_dependencies(
    imports: &[Import],
    source_file: &Path,
    dependency_roots: &mut FxHashSet<PathBuf>,
) {
    for import in imports {
        let Some(root) = normalize_supported_library_path(source_file, &import.library_path) else {
            continue;
        };
        dependency_roots.insert(root);
    }
}

fn collect_nx_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_nx_files(&path, out)?;
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("nx")
        {
            out.push(path);
        }
    }

    Ok(())
}

fn normalize_supported_library_path(base_file: &Path, library_path: &str) -> Option<PathBuf> {
    if is_http_library_path(library_path) || is_git_library_path(library_path) {
        return None;
    }

    normalize_local_library_path(base_file, library_path).ok()
}

fn is_http_library_path(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

fn is_git_library_path(path: &str) -> bool {
    path.starts_with("git://")
}

fn normalize_local_library_path(base_file: &Path, library_path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(library_path).is_absolute() {
        PathBuf::from(library_path)
    } else {
        base_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(library_path)
    };

    fs::canonicalize(candidate)
}

fn visible_name_for_selective(entry: &SelectiveImport) -> String {
    match entry.qualifier.as_ref() {
        Some(prefix) => format!("{}.{}", prefix.as_str(), entry.name.as_str()),
        None => entry.name.as_str().to_string(),
    }
}

fn line_col_for_span(source: &str, span: TextSpan) -> Option<(usize, usize)> {
    let start: usize = span.start().into();
    if start > source.len() {
        return None;
    }

    let mut line = 1usize;
    let mut column = 1usize;
    for (byte_index, ch) in source.char_indices() {
        if byte_index >= start {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    Some((line, column))
}

fn full_source_span(source: &str) -> TextSpan {
    let source_len = u32::try_from(source.len())
        .expect("NX source size should be validated before creating source diagnostics");
    TextSpan::new(TextSize::from(0), TextSize::from(source_len))
}

fn library_imports_require_path_diagnostic(source: &str, file_name: &str) -> Diagnostic {
    Diagnostic::error("library-imports-require-path")
        .with_message("Library imports require an on-disk source path")
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help("Pass a real file path as file_name or use a file-based entry point.")
        .build()
}

fn incomplete_library_dependency_closure_diagnostic(
    file_name: &str,
    source: &str,
    chain: &[PathBuf],
) -> Diagnostic {
    let chain_text = chain
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let missing_root = chain
        .last()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    Diagnostic::error("library-dependency-closure-incomplete")
        .with_message(format!(
            "Loaded library dependency closure is incomplete: {}",
            chain_text
        ))
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help(format!(
            "Reload '{}' and its dependency closure into the supplied library registry before building the program.",
            missing_root
        ))
        .build()
}

fn normalize_diagnostics_file_name(
    diagnostics: Vec<Diagnostic>,
    file_name: &str,
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| {
            let labels = diagnostic
                .labels()
                .iter()
                .cloned()
                .map(|mut label| {
                    if label.file.is_empty() {
                        label.file = file_name.to_string();
                    }
                    label
                })
                .collect::<Vec<_>>();

            let code = diagnostic.code().unwrap_or("diagnostic");
            let mut builder = match diagnostic.severity() {
                Severity::Error => Diagnostic::error(code),
                Severity::Warning => Diagnostic::warning(code),
                Severity::Info => Diagnostic::info(code),
                Severity::Hint => Diagnostic::hint(code),
            }
            .with_message(diagnostic.message())
            .with_labels(labels);

            if let Some(help) = diagnostic.help() {
                builder = builder.with_help(help);
            }

            if let Some(note) = diagnostic.note() {
                builder = builder.with_note(note);
            }

            builder.build()
        })
        .collect()
}

pub(crate) fn has_error_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval_program_artifact;
    use crate::EvalResult;
    use tempfile::TempDir;

    #[test]
    fn library_artifact_preserves_modules_and_exports_separately() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        let forms_dir = ui_dir.join("forms");
        fs::create_dir_all(&forms_dir).expect("forms dir");

        fs::write(
            ui_dir.join("button.nx"),
            r#"export let <Button /> = <button />"#,
        )
        .expect("button");
        fs::write(
            forms_dir.join("input.nx"),
            r#"export let <Input /> = <input />"#,
        )
        .expect("input");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert_eq!(artifact.modules.len(), 2);
        assert_eq!(artifact.exported_items.get("Button").map(Vec::len), Some(1));
        assert_eq!(artifact.exported_items.get("Input").map(Vec::len), Some(1));
        assert!(
            artifact
                .modules
                .iter()
                .all(|module| module.lowered_module.is_some()),
            "Expected one preserved lowered module per source file"
        );
    }

    #[test]
    fn library_artifact_default_internal_items_remain_library_local() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("helpers.nx"), r#"let helper(): int = { 42 }"#)
            .expect("helpers file");
        fs::write(
            ui_dir.join("public.nx"),
            r#"export let answer(): int = { helper() }"#,
        )
        .expect("public file");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected default-internal declarations to resolve across library files"
        );
        assert_eq!(
            artifact
                .visible_to_library_items
                .get("helper")
                .map(Vec::len),
            Some(1)
        );
        assert!(artifact.exported_items.get("helper").is_none());
        assert!(artifact.exports.get("helper").is_none());
        assert_eq!(artifact.exported_items.get("answer").map(Vec::len), Some(1));
    }

    #[test]
    fn library_artifact_record_inheritance_resolves_across_library_files() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"export abstract type Field = { label:string }"#,
        )
        .expect("base file");
        fs::write(
            ui_dir.join("derived.nx"),
            r#"export type TextField extends Field = { placeholder:string? }"#,
        )
        .expect("derived file");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected abstract record bases from peer files to resolve within one library"
        );
        assert_eq!(
            artifact.exported_items.get("TextField").map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn library_artifact_private_items_stay_file_local() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("helpers.nx"),
            r#"private let secret(): int = { 7 }"#,
        )
        .expect("helpers file");
        fs::write(
            ui_dir.join("public.nx"),
            r#"export let answer(): int = { secret() }"#,
        )
        .expect("public file");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert!(artifact.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity() == Severity::Error && diagnostic.message().contains("secret")
        }));
    }

    #[test]
    fn library_artifact_interface_items_preserve_content_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("content.nx"),
            r#"export type Wrapper = { content body:object }
export let wrap(content body:object): object = { body }
export component <Panel content body:object /> = {
  state {
    content cached:string = ""
  }
  <section>{body}</section>
}"#,
        )
        .expect("content file");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected content metadata fixture to analyze without errors"
        );

        let wrap_item = artifact
            .interface_items
            .iter()
            .find(|item| item.item_name == "wrap")
            .expect("Expected exported function interface item");
        match &wrap_item.item {
            LibraryInterfaceKind::Function { params, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name.as_str(), "body");
                assert!(params[0].is_content);
            }
            other => panic!("Expected function interface item, got {other:?}"),
        }

        let wrapper_item = artifact
            .interface_items
            .iter()
            .find(|item| item.item_name == "Wrapper")
            .expect("Expected exported record interface item");
        match &wrapper_item.item {
            LibraryInterfaceKind::Record { properties, .. } => {
                assert_eq!(properties.len(), 1);
                assert_eq!(properties[0].name.as_str(), "body");
                assert!(properties[0].is_content);
                assert!(properties[0].is_required);
            }
            other => panic!("Expected record interface item, got {other:?}"),
        }

        let panel_item = artifact
            .interface_items
            .iter()
            .find(|item| item.item_name == "Panel")
            .expect("Expected exported component interface item");
        match &panel_item.item {
            LibraryInterfaceKind::Component {
                is_abstract,
                is_external,
                base,
                props,
                state,
                ..
            } => {
                assert!(!is_abstract);
                assert!(!is_external);
                assert!(base.is_none());
                assert_eq!(props.len(), 1);
                assert_eq!(props[0].name.as_str(), "body");
                assert!(props[0].is_content);
                assert!(props[0].is_required);
                assert_eq!(state.len(), 1);
                assert_eq!(state[0].name.as_str(), "cached");
                assert!(state[0].is_content);
                assert!(!state[0].is_required);
            }
            other => panic!("Expected component interface item, got {other:?}"),
        }
    }

    #[test]
    fn library_artifact_interface_items_preserve_external_component_state() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("search-box.nx"),
            r#"export external component <SearchBox placeholder:string /> = {
  state {
    query:string
  }
}"#,
        )
        .expect("search-box file");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected external component state fixture to analyze without errors"
        );

        let search_box_item = artifact
            .interface_items
            .iter()
            .find(|item| item.item_name == "SearchBox")
            .expect("Expected exported SearchBox component interface item");
        match &search_box_item.item {
            LibraryInterfaceKind::Component {
                is_abstract,
                is_external,
                props,
                state,
                ..
            } => {
                assert!(!is_abstract);
                assert!(*is_external);
                assert_eq!(props.len(), 1);
                assert_eq!(props[0].name.as_str(), "placeholder");
                assert_eq!(state.len(), 1);
                assert_eq!(state[0].name.as_str(), "query");
            }
            other => panic!("Expected component interface item, got {other:?}"),
        }
    }

    #[test]
    fn consumer_imports_only_explicit_exports() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("helpers.nx"), r#"let helper(): int = { 42 }"#)
            .expect("helpers file");
        fs::write(
            ui_dir.join("public.nx"),
            r#"export let answer(): int = { helper() }"#,
        )
        .expect("public file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let visible_main_path = app_dir.join("visible-main.nx");
        let visible_source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&visible_main_path, visible_source).expect("visible main file");

        let visible_artifact = build_program_artifact_from_source(
            visible_source,
            &visible_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected visible program artifact");

        assert!(
            !has_error_diagnostics(&visible_artifact.diagnostics),
            "Expected explicit exports to remain visible to consumers"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&visible_artifact) else {
            panic!("Expected explicit export program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));

        let hidden_main_path = app_dir.join("hidden-main.nx");
        let hidden_source = r#"import { helper } from "../ui"
let root() = { helper() }"#;
        fs::write(&hidden_main_path, hidden_source).expect("hidden main file");

        let hidden_artifact = build_program_artifact_from_source(
            hidden_source,
            &hidden_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected hidden program artifact with diagnostics");

        assert!(hidden_artifact
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.message().contains("does not export 'helper'") }));
    }

    #[test]
    fn program_artifact_record_inheritance_resolves_imported_abstract_base() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"export abstract type Field = { label:string }"#,
        )
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
type TextField extends Field = { placeholder:string? }
let root() = { 0 }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected imported abstract record bases to resolve through the build context"
        );
    }

    #[test]
    fn program_artifact_component_inheritance_resolves_imported_abstract_base() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"export abstract component <SearchBase placeholder:string content body:Element />"#,
        )
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
component <SearchBox extends SearchBase showSearchIcon:bool = true /> = {
  <section>{body}</section>
}
let root() = { <SearchBox placeholder={"Docs"}><Badge /></SearchBox> }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected imported abstract component bases to resolve through the build context"
        );
    }

    #[test]
    fn library_registry_loads_library_closure_without_program_build() {
        let temp = TempDir::new().expect("temp dir");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("button.nx"),
            r#"export let <Button /> = <button />"#,
        )
        .expect("ui file");
        fs::write(
            widgets_dir.join("search-box.nx"),
            r#"import "../ui"
let root() = { <Button /> }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        let artifact = registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected registry load to succeed");

        assert_eq!(artifact.root_path, fs::canonicalize(&widgets_dir).unwrap());
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&ui_dir).unwrap())
            .is_some());
    }

    #[test]
    fn library_registry_rejects_circular_library_dependencies() {
        let temp = TempDir::new().expect("temp dir");
        let a_dir = temp.path().join("a");
        let b_dir = temp.path().join("b");
        fs::create_dir_all(&a_dir).expect("a dir");
        fs::create_dir_all(&b_dir).expect("b dir");

        fs::write(
            a_dir.join("entry.nx"),
            r#"import "../b"
let from_a() = { from_b() }"#,
        )
        .expect("a file");
        fs::write(
            b_dir.join("entry.nx"),
            r#"import "../a"
let from_b() = { from_a() }"#,
        )
        .expect("b file");

        let registry = LibraryRegistry::new();
        let error = registry
            .load_library_from_directory(&a_dir)
            .expect_err("Expected circular library dependency to fail");
        let a_root = fs::canonicalize(&a_dir).expect("canonical a");
        let b_root = fs::canonicalize(&b_dir).expect("canonical b");
        let expected_chain = format!(
            "{} -> {} -> {}",
            a_root.display(),
            b_root.display(),
            a_root.display()
        );

        assert_eq!(
            error.len(),
            1,
            "Expected one diagnostic for the circular dependency"
        );
        assert!(
            error[0]
                .message
                .contains("Circular library dependency detected"),
            "Expected circular dependency diagnostic, got {:?}",
            error
        );
        assert!(
            error[0].message.contains(&expected_chain),
            "Expected circular dependency chain '{}', got {:?}",
            expected_chain,
            error
        );
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&a_dir).unwrap())
            .is_none());
        assert!(registry
            .get_loaded_library(&fs::canonicalize(&b_dir).unwrap())
            .is_none());
    }

    #[test]
    fn program_artifact_uses_registry_backed_build_context() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let answer(): int = { 42 }"#,
        )
        .expect("ui file");
        let registry = LibraryRegistry::new();
        let ui_snapshot = registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert_eq!(artifact.libraries.len(), 1);
        assert!(Arc::ptr_eq(&artifact.libraries[0], &ui_snapshot));
        assert!(
            artifact.root_modules[0]
                .lowered_module
                .as_ref()
                .expect("Expected preserved root module")
                .find_item("answer")
                .is_none(),
            "Expected preserved root module to remain file-scoped"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected registry-backed program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn imported_content_bindings_match_local_behavior() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("content.nx"),
            r#"export type Wrapper = { content body:object }
export let Wrap(content body:object): object = { body }
export component <Panel content body:object /> = {
  <section>{body}</section>
}"#,
        )
        .expect("content file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import { Wrapper, Wrap, Panel } from "../ui"
let root(): object[] = {
  <Wrap><Badge /></Wrap>
  <Wrapper><Badge /></Wrapper>
  <Panel><Badge /></Panel>
}"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected imported content bindings to analyze without errors, got {:?}",
            artifact.diagnostics
        );

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected imported content bindings evaluation to succeed");
        };

        let badge = nx_value::NxValue::Record {
            type_name: Some("Badge".to_string()),
            properties: std::collections::BTreeMap::new(),
        };
        let wrapper = nx_value::NxValue::Record {
            type_name: Some("Wrapper".to_string()),
            properties: std::collections::BTreeMap::from([("body".to_string(), badge.clone())]),
        };
        let panel = nx_value::NxValue::Record {
            type_name: Some("Panel".to_string()),
            properties: std::collections::BTreeMap::from([("body".to_string(), badge.clone())]),
        };
        assert_eq!(value, nx_value::NxValue::Array(vec![badge, wrapper, panel]));
    }

    #[test]
    fn program_build_reports_missing_library_from_context() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::write(
            ui_dir.join("button.nx"),
            r#"export let <Button /> = <button />"#,
        )
        .expect("ui file");

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { <Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact with diagnostics");

        assert!(artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("Missing loaded library")));
    }

    #[test]
    fn apply_build_context_imports_reports_unresolved_source_path_for_local_imports() {
        let source = r#"import "../ui"
let root() = { answer() }"#;
        let parse_result = syntax_parse_str(source, "virtual.nx");
        let source_id = SourceId::new(parse_result.source_id.as_u32());
        let tree = parse_result.tree.expect("Expected syntax tree");
        let mut module =
            PreparedModule::standalone("virtual.nx", nx_hir::lower(tree.root(), source_id));

        let resolved_imports = apply_build_context_imports(
            &mut module,
            Path::new("/path/that/does/not/exist/main.nx"),
            &ProgramBuildContext::empty(),
            source,
        );

        assert!(resolved_imports.is_empty());
        assert!(module.diagnostics().iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("Local library import resolution was skipped because source file path")
                && diagnostic.message.contains("could not be resolved")
        }));
    }

    #[test]
    fn program_artifact_survives_build_context_drop() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let answer(): int = { 42 }"#,
        )
        .expect("ui file");
        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        drop(build_context);

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected program artifact to remain executable after context release");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn build_context_with_visible_roots_includes_transitive_dependencies_of_visible_library() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let ui_answer(): int = { 42 }"#,
        )
        .expect("ui file");
        fs::write(
            widgets_dir.join("answer.nx"),
            r#"import "../ui"
export let answer(): int = { ui_answer() }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected widgets registry load");
        let build_context = registry
            .build_context_with_visible_roots([&widgets_dir])
            .expect("Expected filtered build context");

        let main_path = app_dir.join("main.nx");
        let source = r#"import "../widgets"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        assert!(
            !has_error_diagnostics(&artifact.diagnostics),
            "Expected transitive dependency closure to be selected for the visible library"
        );

        let widget_root = fs::canonicalize(&widgets_dir).expect("canonical widgets");
        let ui_root = fs::canonicalize(&ui_dir).expect("canonical ui");
        let library_roots = artifact
            .libraries
            .iter()
            .map(|library| library.root_path.clone())
            .collect::<Vec<_>>();
        assert!(
            library_roots.contains(&widget_root),
            "Expected selected libraries to include the visible library"
        );
        assert!(
            library_roots.contains(&ui_root),
            "Expected selected libraries to include the visible library's transitive dependency"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("Expected transitive dependency program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));
    }

    #[test]
    fn build_context_with_visible_roots_limits_visible_libraries() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        let admin_dir = temp.path().join("admin");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::create_dir_all(&admin_dir).expect("admin dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let answer(): int = { 42 }"#,
        )
        .expect("ui file");
        fs::write(
            admin_dir.join("secret.nx"),
            r#"export let secret(): int = { 7 }"#,
        )
        .expect("admin file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        registry
            .load_library_from_directory(&admin_dir)
            .expect("Expected admin registry load");
        let build_context = registry
            .build_context_with_visible_roots([&ui_dir])
            .expect("Expected filtered build context");

        let visible_main_path = app_dir.join("visible-main.nx");
        let visible_source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&visible_main_path, visible_source).expect("visible main file");

        let visible_artifact = build_program_artifact_from_source(
            visible_source,
            &visible_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected visible program artifact");

        assert!(
            !has_error_diagnostics(&visible_artifact.diagnostics),
            "Expected visible library import to succeed"
        );

        let EvalResult::Ok(value) = eval_program_artifact(&visible_artifact) else {
            panic!("Expected visible library program artifact evaluation to succeed");
        };
        assert_eq!(value, nx_value::NxValue::Int(42));

        let hidden_main_path = app_dir.join("hidden-main.nx");
        let hidden_source = r#"import "../admin"
let root() = { secret() }"#;
        fs::write(&hidden_main_path, hidden_source).expect("hidden main file");

        let hidden_artifact = build_program_artifact_from_source(
            hidden_source,
            &hidden_main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected hidden program artifact with diagnostics");

        assert!(
            hidden_artifact.libraries.is_empty(),
            "Expected hidden libraries to stay out of the selected program library set"
        );
        assert!(hidden_artifact
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("Missing loaded library")));
    }

    #[test]
    fn program_build_reports_incomplete_loaded_library_dependency_closure() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let ui_answer(): int = { 42 }"#,
        )
        .expect("ui file");
        fs::write(
            widgets_dir.join("answer.nx"),
            r#"import "../ui"
export let answer(): int = { ui_answer() }"#,
        )
        .expect("widgets file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&widgets_dir)
            .expect("Expected widgets registry load");
        let ui_root = fs::canonicalize(&ui_dir).expect("canonical ui");
        {
            let mut state = registry
                .inner
                .write()
                .expect("library registry lock poisoned");
            let removed = state.libraries.remove(&ui_root);
            assert!(
                removed.is_some(),
                "Expected inconsistent registry setup to remove the transitive dependency snapshot"
            );
        }

        let build_context = registry.build_context();
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../widgets"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact with diagnostics");

        let widgets_root = fs::canonicalize(&widgets_dir).expect("canonical widgets");
        let expected_chain = format!("{} -> {}", widgets_root.display(), ui_root.display());
        assert!(artifact.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == Some("library-dependency-closure-incomplete")
                && diagnostic.message().contains(&expected_chain)
        }));
    }
}
