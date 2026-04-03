use nx_diagnostics::{Diagnostic, Label, Severity, TextSize, TextSpan};
use nx_hir::{
    build_scopes, lower, resolve_local_library_imports, Import, ImportKind, Item,
    LoweringDiagnostic, SelectiveImport, SourceId, Visibility,
};
use nx_interpreter::{
    ModuleQualifiedItemRef, ResolvedItemKind, ResolvedModule, ResolvedProgram, RuntimeModuleId,
};
use nx_syntax::parse_str as syntax_parse_str;
use nx_types::{InferenceContext, ModuleArtifact, TypeEnvironment};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Export metadata for one symbol provided by a library artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryExport {
    pub module_file: String,
    pub item_name: String,
    pub kind: ResolvedItemKind,
}

/// File-preserving artifact for one local NX library directory.
#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub root_path: PathBuf,
    pub modules: Vec<ModuleArtifact>,
    pub exports: FxHashMap<String, LibraryExport>,
    pub dependency_roots: Vec<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
}

/// File-preserving artifact for one resolved NX program.
#[derive(Debug, Clone)]
pub struct ProgramArtifact {
    pub root_modules: Vec<ModuleArtifact>,
    pub libraries: Vec<LibraryArtifact>,
    pub diagnostics: Vec<Diagnostic>,
    pub fingerprint: u64,
    pub resolved_program: ResolvedProgram,
}

/// Builds a file-preserving library artifact from a local directory.
pub fn build_library_artifact_from_directory(
    root_path: impl AsRef<Path>,
) -> io::Result<LibraryArtifact> {
    let root_path = fs::canonicalize(root_path.as_ref())?;
    let mut source_files = Vec::new();
    collect_nx_files(&root_path, &mut source_files)?;
    source_files.sort();

    let mut modules = Vec::with_capacity(source_files.len());
    let mut exports = FxHashMap::default();
    let mut dependency_roots = FxHashSet::default();
    let mut diagnostics = Vec::new();
    let mut hasher = DefaultHasher::new();
    root_path.hash(&mut hasher);

    for source_file in source_files {
        let source = fs::read_to_string(&source_file)?;
        source_file.hash(&mut hasher);
        source.hash(&mut hasher);

        let file_name = source_file.display().to_string();
        let artifact = analyze_preserved_module(&source, &file_name, Some(&source_file));
        diagnostics.extend(artifact.diagnostics.iter().cloned());
        collect_library_dependencies(&artifact.imports, &source_file, &mut dependency_roots);

        if let Some(module) = artifact.lowered_module.as_ref() {
            for item in module.items() {
                if item.visibility() != Visibility::Public {
                    continue;
                }

                exports
                    .entry(item.name().as_str().to_string())
                    .or_insert_with(|| LibraryExport {
                        module_file: artifact.file_name.clone(),
                        item_name: item.name().as_str().to_string(),
                        kind: resolved_item_kind(item),
                    });
            }
        }

        modules.push(artifact);
    }

    let mut dependency_roots = dependency_roots.into_iter().collect::<Vec<_>>();
    dependency_roots.sort();

    Ok(LibraryArtifact {
        root_path,
        modules,
        exports,
        dependency_roots,
        diagnostics,
        fingerprint: hasher.finish(),
    })
}

/// Builds a file-preserving program artifact from source text and an optional on-disk file path.
pub fn build_program_artifact_from_source(
    source: &str,
    file_name: &str,
) -> io::Result<ProgramArtifact> {
    let source_path = Path::new(file_name);
    let root_path = source_path.exists().then_some(source_path);
    let root_artifact = analyze_preserved_module(source, file_name, root_path);

    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    source.hash(&mut hasher);

    let mut libraries = Vec::new();
    let mut seen_library_roots = FxHashSet::default();
    if let Some(root_path) = root_path {
        let mut queue = root_artifact
            .imports
            .iter()
            .filter_map(|import| normalize_supported_library_path(root_path, &import.library_path))
            .collect::<Vec<_>>();
        queue.sort();

        while let Some(root) = queue.pop() {
            if !seen_library_roots.insert(root.clone()) {
                continue;
            }

            let library = build_library_artifact_from_directory(&root)?;
            for dependency_root in &library.dependency_roots {
                if !seen_library_roots.contains(dependency_root) {
                    queue.push(dependency_root.clone());
                }
            }
            hasher.write_u64(library.fingerprint);
            libraries.push(library);
        }
    }

    libraries.sort_by(|lhs, rhs| lhs.root_path.cmp(&rhs.root_path));
    let root_modules = vec![root_artifact];
    let mut diagnostics = root_modules[0].diagnostics.clone();
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

fn analyze_preserved_module(
    source: &str,
    file_name: &str,
    source_path: Option<&Path>,
) -> ModuleArtifact {
    let parse_result = syntax_parse_str(source, file_name);
    let source_id = SourceId::new(parse_result.source_id.as_u32());
    let mut diagnostics = normalize_diagnostics_file_name(parse_result.errors, file_name);

    let Some(tree) = parse_result.tree else {
        return ModuleArtifact {
            file_name: file_name.to_string(),
            source_id,
            parse_succeeded: false,
            lowered_module: None,
            type_env: TypeEnvironment::new(),
            diagnostics,
            imports: Vec::new(),
        };
    };

    let preserved_module = lower(tree.root(), source_id);
    let mut analysis_module = preserved_module.clone();

    if let Some(path) = source_path {
        analysis_module = match resolve_local_library_imports(analysis_module, path) {
            Ok(module) => module,
            Err(error) => {
                diagnostics.push(library_load_error_diagnostic(source, file_name, &error));
                preserved_module.clone()
            }
        };
    } else if !preserved_module.imports.is_empty() {
        diagnostics.push(library_imports_require_path_diagnostic(source, file_name));
    }

    diagnostics.extend(lowering_diagnostics(
        analysis_module.diagnostics(),
        file_name,
    ));

    let (_scope_manager, scope_diagnostics) = build_scopes(&analysis_module);
    diagnostics.extend(normalize_diagnostics_file_name(
        scope_diagnostics,
        file_name,
    ));

    let mut ctx = InferenceContext::with_file_name(&analysis_module, file_name);
    for item in analysis_module.items() {
        match item {
            Item::Function(function) => ctx.infer_function(function),
            Item::Value(_)
            | Item::Component(_)
            | Item::TypeAlias(_)
            | Item::Record(_)
            | Item::Enum(_) => {}
        }
    }

    let (type_env, type_diagnostics) = ctx.finish();
    diagnostics.extend(normalize_diagnostics_file_name(type_diagnostics, file_name));

    ModuleArtifact {
        file_name: file_name.to_string(),
        source_id,
        parse_succeeded: true,
        lowered_module: Some(Arc::new(preserved_module.clone())),
        type_env,
        diagnostics,
        imports: preserved_module.imports.clone(),
    }
}

fn build_resolved_program(
    root_modules: &[ModuleArtifact],
    libraries: &[LibraryArtifact],
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

        for item in module.items() {
            insert_entry_symbol(
                &mut entry_functions,
                &mut entry_components,
                &mut entry_records,
                &mut entry_enums,
                item.name().as_str(),
                ModuleQualifiedItemRef {
                    module_id,
                    item_name: item.name().as_str().to_string(),
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
                    item_name: export.item_name.clone(),
                    kind: export.kind,
                },
            );
        }
    }

    let library_by_root = libraries
        .iter()
        .map(|library| (library.root_path.clone(), library))
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
                                item_name: export.item_name.clone(),
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
                                item_name: export.item_name.clone(),
                                kind: export.kind,
                            });
                    }
                }
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

fn lowering_diagnostics(diagnostics: &[LoweringDiagnostic], file_name: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            Diagnostic::error("lowering-error")
                .with_message(diagnostic.message.clone())
                .with_label(Label::primary(file_name, diagnostic.span))
                .build()
        })
        .collect()
}

fn full_source_span(source: &str) -> TextSpan {
    let source_len = u32::try_from(source.len())
        .expect("NX source size should be validated before creating source diagnostics");
    TextSpan::new(TextSize::from(0), TextSize::from(source_len))
}

fn library_load_error_diagnostic(source: &str, file_name: &str, error: &io::Error) -> Diagnostic {
    Diagnostic::error("library-load-error")
        .with_message(format!("Failed to load library imports: {}", error))
        .with_label(Label::primary(file_name, full_source_span(source)))
        .build()
}

fn library_imports_require_path_diagnostic(source: &str, file_name: &str) -> Diagnostic {
    Diagnostic::error("library-imports-require-path")
        .with_message("Library imports require an on-disk source path")
        .with_label(Label::primary(file_name, full_source_span(source)))
        .with_help("Pass a real file path as file_name or use a file-based entry point.")
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn library_artifact_preserves_modules_and_exports_separately() {
        let temp = TempDir::new().expect("temp dir");
        let ui_dir = temp.path().join("ui");
        let forms_dir = ui_dir.join("forms");
        fs::create_dir_all(&forms_dir).expect("forms dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("button");
        fs::write(forms_dir.join("input.nx"), r#"let <Input /> = <input />"#).expect("input");

        let artifact =
            build_library_artifact_from_directory(&ui_dir).expect("Expected library artifact");

        assert_eq!(artifact.modules.len(), 2);
        assert_eq!(
            artifact
                .exports
                .get("Button")
                .expect("Expected Button export")
                .module_file
                .ends_with("button.nx"),
            true
        );
        assert_eq!(
            artifact
                .exports
                .get("Input")
                .expect("Expected Input export")
                .module_file
                .ends_with("forms/input.nx"),
            true
        );
        assert!(
            artifact
                .modules
                .iter()
                .all(|module| module.lowered_module.is_some()),
            "Expected one preserved lowered module per source file"
        );
    }

    #[test]
    fn library_artifact_records_library_level_dependency_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let widgets_dir = temp.path().join("widgets");
        let ui_dir = temp.path().join("ui");
        let core_dir = temp.path().join("core");
        fs::create_dir_all(&widgets_dir).expect("widgets dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");
        fs::create_dir_all(&core_dir).expect("core dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        fs::write(core_dir.join("theme.nx"), r#"let theme() = "base""#).expect("core file");
        fs::write(
            widgets_dir.join("search-box.nx"),
            r#"import "../ui"
let root() = { 1 }"#,
        )
        .expect("search box");
        fs::write(
            widgets_dir.join("indexing.nx"),
            r#"import "../core"
let indexName() = "docs""#,
        )
        .expect("indexing");

        let artifact = build_library_artifact_from_directory(&widgets_dir)
            .expect("Expected dependency-aware library artifact");

        let dependency_roots = artifact
            .dependency_roots
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
            })
            .collect::<Vec<_>>();
        assert!(dependency_roots.contains(&"ui"));
        assert!(dependency_roots.contains(&"core"));
    }

    #[test]
    fn program_artifact_preserves_root_modules_and_resolved_libraries() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(ui_dir.join("button.nx"), r#"let <Button /> = <button />"#).expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { <Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let artifact = build_program_artifact_from_source(source, &main_path.display().to_string())
            .expect("Expected program artifact");

        assert_eq!(artifact.root_modules.len(), 1);
        assert_eq!(artifact.libraries.len(), 1);
        assert_ne!(artifact.fingerprint, 0);
        assert!(
            artifact.root_modules[0]
                .lowered_module
                .as_ref()
                .expect("Expected preserved root module")
                .find_item("Button")
                .is_none(),
            "Expected preserved root module to remain file-scoped"
        );
        assert!(
            artifact.resolved_program.entry_function("root").is_some(),
            "Expected resolved program entrypoint lookup to include root()"
        );
        let root_module_id = artifact.resolved_program.root_modules()[0];
        assert!(
            artifact
                .resolved_program
                .imported_item(root_module_id, "Button")
                .is_some(),
            "Expected resolved program imports to preserve the library export lookup"
        );
    }
}
