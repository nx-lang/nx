use crate::workspace::{normalize_workspace_identity, NxWorkspace, WorkspaceIdentityError};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogicalSourceModule {
    pub identity: String,
    pub source: Arc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogicalModuleGraph {
    modules: Vec<LogicalSourceModule>,
    identities: FxHashMap<String, usize>,
}

impl LogicalModuleGraph {
    pub fn from_modules(modules: Vec<LogicalSourceModule>) -> Result<Self, SourceProviderError> {
        let mut identities = FxHashMap::default();
        let mut seen = FxHashSet::default();

        for (index, module) in modules.iter().enumerate() {
            if !seen.insert(module.identity.clone()) {
                return Err(SourceProviderError::Identity(
                    WorkspaceIdentityError::Duplicate {
                        identity: module.identity.clone(),
                    },
                ));
            }
            identities.insert(module.identity.clone(), index);
        }

        Ok(Self {
            modules,
            identities,
        })
    }

    pub fn modules(&self) -> &[LogicalSourceModule] {
        &self.modules
    }

    pub fn get(&self, identity: &str) -> Option<&LogicalSourceModule> {
        self.identities
            .get(identity)
            .and_then(|index| self.modules.get(*index))
    }

    pub fn contains_identity(&self, identity: &str) -> bool {
        self.identities.contains_key(identity)
    }

    pub fn source_map(&self) -> FxHashMap<String, Arc<str>> {
        self.modules
            .iter()
            .map(|module| (module.identity.clone(), module.source.clone()))
            .collect()
    }
}

pub(crate) trait SourceProvider {
    fn load_graph(&self) -> Result<LogicalModuleGraph, SourceProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceProviderError {
    Identity(WorkspaceIdentityError),
    Io { path: PathBuf, message: String },
}

impl fmt::Display for SourceProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => error.fmt(formatter),
            Self::Io { path, message } => {
                write!(
                    formatter,
                    "Failed to load '{}': {}",
                    path.display(),
                    message
                )
            }
        }
    }
}

impl From<WorkspaceIdentityError> for SourceProviderError {
    fn from(value: WorkspaceIdentityError) -> Self {
        Self::Identity(value)
    }
}

pub(crate) struct WorkspaceSourceProvider<'a> {
    workspace: &'a NxWorkspace,
}

impl<'a> WorkspaceSourceProvider<'a> {
    pub fn new(workspace: &'a NxWorkspace) -> Self {
        Self { workspace }
    }
}

impl SourceProvider for WorkspaceSourceProvider<'_> {
    fn load_graph(&self) -> Result<LogicalModuleGraph, SourceProviderError> {
        let mut modules = Vec::with_capacity(self.workspace.modules().len());
        for module in self.workspace.modules() {
            modules.push(LogicalSourceModule {
                identity: module.identity().to_string(),
                source: module.source_arc(),
            });
        }

        LogicalModuleGraph::from_modules(modules)
    }
}

pub(crate) struct FilesystemSourceProvider {
    root_path: PathBuf,
    source_paths: Vec<PathBuf>,
}

impl FilesystemSourceProvider {
    pub fn new(root_path: impl Into<PathBuf>, source_paths: Vec<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            source_paths,
        }
    }

    pub fn from_root(root_path: impl Into<PathBuf>) -> Result<Self, SourceProviderError> {
        let root_path = root_path.into();
        let mut source_paths = Vec::new();
        collect_nx_files(&root_path, &mut source_paths)?;
        source_paths.sort();
        Ok(Self {
            root_path,
            source_paths,
        })
    }

    fn identity_for_path(&self, source_path: &Path) -> Result<String, WorkspaceIdentityError> {
        let logical_path = source_path
            .strip_prefix(&self.root_path)
            .unwrap_or(source_path);
        let identity = logical_path
            .components()
            .filter_map(logical_component)
            .collect::<Vec<_>>()
            .join("/");
        normalize_workspace_identity(&identity)
    }
}

impl SourceProvider for FilesystemSourceProvider {
    fn load_graph(&self) -> Result<LogicalModuleGraph, SourceProviderError> {
        let mut modules = Vec::with_capacity(self.source_paths.len());
        for source_path in &self.source_paths {
            let identity = self.identity_for_path(source_path)?;
            let source =
                fs::read_to_string(source_path).map_err(|error| SourceProviderError::Io {
                    path: source_path.clone(),
                    message: error.to_string(),
                })?;
            modules.push(LogicalSourceModule {
                identity,
                source: Arc::<str>::from(source),
            });
        }

        LogicalModuleGraph::from_modules(modules)
    }
}

fn logical_component(component: Component<'_>) -> Option<String> {
    match component {
        Component::Normal(value) => Some(value.to_string_lossy().to_string()),
        Component::CurDir => Some(".".to_string()),
        Component::ParentDir => Some("..".to_string()),
        Component::RootDir | Component::Prefix(_) => None,
    }
}

fn collect_nx_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), SourceProviderError> {
    let entries = fs::read_dir(dir).map_err(|error| SourceProviderError::Io {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| SourceProviderError::Io {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| SourceProviderError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if file_type.is_dir() {
            collect_nx_files(&path, out)?;
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("nx")
        {
            out.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NxWorkspaceModule;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn workspace_provider_loads_normalized_identities_and_source_text() {
        let workspace = NxWorkspace::new(vec![NxWorkspaceModule::from_utf8(
            "app/./main.nx",
            b"let root() = { 1 }".to_vec(),
        )
        .expect("workspace module")])
        .expect("workspace");

        let graph = WorkspaceSourceProvider::new(&workspace)
            .load_graph()
            .expect("workspace graph");

        assert!(graph.contains_identity("app/main.nx"));
        assert_eq!(
            graph
                .get("app/main.nx")
                .map(|module| module.source.as_ref()),
            Some("let root() = { 1 }")
        );
    }

    #[test]
    fn workspace_and_filesystem_providers_load_equivalent_modules() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        fs::create_dir_all(&app_dir).expect("app dir");
        let main_path = app_dir.join("main.nx");
        fs::write(&main_path, "let root() = { 1 }").expect("main file");

        let workspace = NxWorkspace::new(vec![NxWorkspaceModule::from_utf8(
            "app/main.nx",
            b"let root() = { 1 }".to_vec(),
        )
        .expect("workspace module")])
        .expect("workspace");
        let workspace_graph = WorkspaceSourceProvider::new(&workspace)
            .load_graph()
            .expect("workspace graph");
        let filesystem_graph = FilesystemSourceProvider::from_root(temp.path())
            .expect("filesystem provider")
            .load_graph()
            .expect("filesystem graph");

        assert_eq!(workspace_graph.modules(), filesystem_graph.modules());
    }

    #[test]
    fn graph_rejects_duplicate_normalized_identities() {
        assert_eq!(
            LogicalModuleGraph::from_modules(vec![
                LogicalSourceModule {
                    identity: "shared/config.nx".to_string(),
                    source: Arc::<str>::from("let root() = { 1 }"),
                },
                LogicalSourceModule {
                    identity: "shared/config.nx".to_string(),
                    source: Arc::<str>::from("let root() = { 2 }"),
                },
            ]),
            Err(SourceProviderError::Identity(
                WorkspaceIdentityError::Duplicate {
                    identity: "shared/config.nx".to_string(),
                }
            ))
        );
    }

    #[test]
    fn source_map_preserves_provider_source_text_by_identity() {
        let workspace = NxWorkspace::new(vec![NxWorkspaceModule::from_source(
            "shared/config.nx",
            "let root() = { \"memory\" }",
        )
        .expect("workspace module")])
        .expect("workspace");
        let graph = WorkspaceSourceProvider::new(&workspace)
            .load_graph()
            .expect("workspace graph");

        assert_eq!(
            graph
                .source_map()
                .get("shared/config.nx")
                .map(|source| source.as_ref()),
            Some("let root() = { \"memory\" }")
        );
    }
}
