use rustc_hash::FxHashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// A logical workspace containing NX source modules submitted together for analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxWorkspace {
    modules: Vec<NxWorkspaceModule>,
}

impl NxWorkspace {
    /// Creates a workspace from validated source modules.
    pub fn new(modules: Vec<NxWorkspaceModule>) -> Result<Self, NxWorkspaceInputError> {
        let mut seen = FxHashSet::default();
        for module in &modules {
            if !seen.insert(module.identity.clone()) {
                return Err(NxWorkspaceInputError::DuplicateIdentity {
                    identity: module.identity.clone(),
                });
            }
        }

        Ok(Self { modules })
    }

    /// Creates a workspace from every `.nx` source file under a directory.
    pub fn from_directory(root_path: impl AsRef<Path>) -> Result<Self, NxWorkspaceDirectoryError> {
        let root_path = root_path.as_ref();
        let mut source_paths = Vec::new();
        collect_nx_file_paths(root_path, &mut source_paths)?;
        source_paths.sort();

        let mut modules = Vec::with_capacity(source_paths.len());
        for source_path in source_paths {
            let identity = workspace_identity_for_path(root_path, &source_path)?;
            let source = fs::read_to_string(&source_path).map_err(|error| {
                NxWorkspaceDirectoryError::ReadFile {
                    path: source_path.clone(),
                    message: error.to_string(),
                }
            })?;
            modules.push(NxWorkspaceModule::from_source(identity, source)?);
        }

        Ok(Self::new(modules)?)
    }

    /// Returns the validated modules in this workspace.
    pub fn modules(&self) -> &[NxWorkspaceModule] {
        &self.modules
    }
}

/// One NX module in a logical workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxWorkspaceModule {
    identity: String,
    source: Arc<str>,
}

impl NxWorkspaceModule {
    /// Creates a workspace module from a logical identity and already-decoded UTF-8 source text.
    pub fn from_source(
        identity: impl Into<String>,
        source: impl Into<Arc<str>>,
    ) -> Result<Self, NxWorkspaceInputError> {
        let identity = identity.into();
        let normalized_identity = normalize_workspace_identity(&identity).map_err(|error| {
            NxWorkspaceInputError::InvalidIdentity {
                identity: identity.clone(),
                message: error.to_string(),
            }
        })?;
        Ok(Self {
            identity: normalized_identity,
            source: source.into(),
        })
    }

    /// Creates a workspace module from a logical identity and owned UTF-8 source bytes.
    pub fn from_utf8(
        identity: impl Into<String>,
        source_utf8: Vec<u8>,
    ) -> Result<Self, NxWorkspaceInputError> {
        let identity = identity.into();
        let normalized_identity = normalize_workspace_identity(&identity).map_err(|error| {
            NxWorkspaceInputError::InvalidIdentity {
                identity: identity.clone(),
                message: error.to_string(),
            }
        })?;
        let source = String::from_utf8(source_utf8).map_err(|_| {
            NxWorkspaceInputError::InvalidSourceUtf8 {
                identity: normalized_identity.clone(),
            }
        })?;

        Ok(Self {
            identity: normalized_identity,
            source: Arc::<str>::from(source),
        })
    }

    /// Returns the normalized logical workspace identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the decoded source text.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn source_arc(&self) -> Arc<str> {
        Arc::clone(&self.source)
    }
}

/// Invalid workspace input detected before analysis starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NxWorkspaceInputError {
    InvalidIdentity { identity: String, message: String },
    DuplicateIdentity { identity: String },
    InvalidSourceUtf8 { identity: String },
}

impl fmt::Display for NxWorkspaceInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity { identity, message } => {
                write!(
                    formatter,
                    "Workspace identity '{}' is invalid: {}",
                    identity, message
                )
            }
            Self::DuplicateIdentity { identity } => {
                write!(
                    formatter,
                    "Duplicate workspace identity '{}' after normalization",
                    identity
                )
            }
            Self::InvalidSourceUtf8 { identity } => {
                write!(
                    formatter,
                    "Workspace module '{}' source is not valid UTF-8",
                    identity
                )
            }
        }
    }
}

impl Error for NxWorkspaceInputError {}

/// Error encountered while loading a workspace from a filesystem directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NxWorkspaceDirectoryError {
    ReadDirectory { path: PathBuf, message: String },
    InspectPath { path: PathBuf, message: String },
    ReadFile { path: PathBuf, message: String },
    InvalidSourcePath { path: PathBuf },
    InvalidInput(NxWorkspaceInputError),
}

impl fmt::Display for NxWorkspaceDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { path, message } => {
                write!(
                    formatter,
                    "Failed to read directory '{}': {}",
                    path.display(),
                    message
                )
            }
            Self::InspectPath { path, message } => {
                write!(
                    formatter,
                    "Failed to inspect '{}': {}",
                    path.display(),
                    message
                )
            }
            Self::ReadFile { path, message } => {
                write!(
                    formatter,
                    "Failed to read '{}': {}",
                    path.display(),
                    message
                )
            }
            Self::InvalidSourcePath { path } => {
                write!(
                    formatter,
                    "Workspace source path '{}' cannot be converted to a logical identity",
                    path.display()
                )
            }
            Self::InvalidInput(error) => write!(formatter, "Invalid workspace input: {}", error),
        }
    }
}

impl Error for NxWorkspaceDirectoryError {}

impl From<NxWorkspaceInputError> for NxWorkspaceDirectoryError {
    fn from(value: NxWorkspaceInputError) -> Self {
        Self::InvalidInput(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceIdentityError {
    Empty,
    Absolute,
    EmptySegment,
    EscapesRoot,
    Duplicate { identity: String },
}

impl fmt::Display for WorkspaceIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "Workspace identity must not be empty"),
            Self::Absolute => write!(formatter, "Workspace identity must not be absolute"),
            Self::EmptySegment => {
                write!(
                    formatter,
                    "Workspace identity must not contain empty segments"
                )
            }
            Self::EscapesRoot => {
                write!(formatter, "Workspace identity escapes the workspace root")
            }
            Self::Duplicate { identity } => {
                write!(
                    formatter,
                    "Duplicate workspace identity '{}' after normalization",
                    identity
                )
            }
        }
    }
}

pub(crate) fn normalize_workspace_identity(
    identity: &str,
) -> Result<String, WorkspaceIdentityError> {
    let identity = identity.trim();
    if identity.is_empty() {
        return Err(WorkspaceIdentityError::Empty);
    }
    if identity.starts_with('/') {
        return Err(WorkspaceIdentityError::Absolute);
    }

    normalize_workspace_identity_from_segments(identity.split('/'))
}

pub(crate) fn normalize_workspace_import_identity(
    importer_identity: &str,
    import_identity: &str,
) -> Result<String, WorkspaceIdentityError> {
    let import_identity = import_identity.trim();
    if import_identity.starts_with('/') {
        return normalize_workspace_identity(import_identity);
    }

    let importer_identity = normalize_workspace_identity(importer_identity)?;
    let mut segments = importer_identity.split('/').collect::<Vec<_>>();
    let _ = segments.pop();
    segments.extend(import_identity.split('/'));
    normalize_workspace_identity_from_segments(segments)
}

fn normalize_workspace_identity_from_segments<'a>(
    segments: impl IntoIterator<Item = &'a str>,
) -> Result<String, WorkspaceIdentityError> {
    let mut normalized = Vec::new();
    let mut saw_segment = false;

    for raw_segment in segments {
        let segment = raw_segment.trim();
        if segment.is_empty() {
            return Err(WorkspaceIdentityError::EmptySegment);
        }

        saw_segment = true;
        match segment {
            "." => {}
            ".." => {
                if normalized.pop().is_none() {
                    return Err(WorkspaceIdentityError::EscapesRoot);
                }
            }
            _ => normalized.push(segment),
        }
    }

    if !saw_segment || normalized.is_empty() {
        return Err(WorkspaceIdentityError::Empty);
    }

    Ok(normalized.join("/"))
}

fn collect_nx_file_paths(
    dir: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), NxWorkspaceDirectoryError> {
    let entries = fs::read_dir(dir).map_err(|error| NxWorkspaceDirectoryError::ReadDirectory {
        path: dir.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| NxWorkspaceDirectoryError::ReadDirectory {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let file_type =
            entry
                .file_type()
                .map_err(|error| NxWorkspaceDirectoryError::InspectPath {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
        if file_type.is_dir() {
            collect_nx_file_paths(&path, output)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("nx")
        {
            output.push(path);
        }
    }
    Ok(())
}

fn workspace_identity_for_path(
    root_path: &Path,
    source_path: &Path,
) -> Result<String, NxWorkspaceDirectoryError> {
    let relative_path = source_path.strip_prefix(root_path).unwrap_or(source_path);
    let mut parts = Vec::new();
    for component in relative_path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(NxWorkspaceDirectoryError::InvalidSourcePath {
                    path: source_path.to_path_buf(),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn normalizes_workspace_identity_dot_segments() {
        assert_eq!(
            normalize_workspace_identity("tenant/./shared/../config.nx"),
            Ok("tenant/config.nx".to_string())
        );
    }

    #[test]
    fn rejects_empty_workspace_identity() {
        assert_eq!(
            normalize_workspace_identity(""),
            Err(WorkspaceIdentityError::Empty)
        );
        assert_eq!(
            normalize_workspace_identity("."),
            Err(WorkspaceIdentityError::Empty)
        );
    }

    #[test]
    fn rejects_absolute_workspace_identity() {
        assert_eq!(
            normalize_workspace_identity("/tenant/config.nx"),
            Err(WorkspaceIdentityError::Absolute)
        );
    }

    #[test]
    fn rejects_empty_workspace_identity_segments() {
        assert_eq!(
            normalize_workspace_identity("tenant//config.nx"),
            Err(WorkspaceIdentityError::EmptySegment)
        );
        assert_eq!(
            normalize_workspace_identity("tenant/config.nx/"),
            Err(WorkspaceIdentityError::EmptySegment)
        );
    }

    #[test]
    fn rejects_root_escaping_workspace_identity() {
        assert_eq!(
            normalize_workspace_identity("../outside.nx"),
            Err(WorkspaceIdentityError::EscapesRoot)
        );
        assert_eq!(
            normalize_workspace_identity("tenant/../../outside.nx"),
            Err(WorkspaceIdentityError::EscapesRoot)
        );
    }

    #[test]
    fn loads_workspace_from_directory() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::write(app_dir.join("main.nx"), "let root() = { 1 }").expect("main file");
        fs::write(temp.path().join("notes.txt"), "ignored").expect("non-nx file");

        let workspace = NxWorkspace::from_directory(temp.path()).expect("workspace");

        assert_eq!(workspace.modules().len(), 1);
        assert_eq!(workspace.modules()[0].identity(), "app/main.nx");
        assert_eq!(workspace.modules()[0].source(), "let root() = { 1 }");
    }

    #[test]
    fn rejects_duplicate_normalized_workspace_identities() {
        let modules = vec![
            NxWorkspaceModule::from_source("shared/config.nx", "let root() = { 1 }")
                .expect("first module"),
            NxWorkspaceModule::from_source("shared/./config.nx", "let root() = { 2 }")
                .expect("second module"),
        ];

        assert_eq!(
            NxWorkspace::new(modules),
            Err(NxWorkspaceInputError::DuplicateIdentity {
                identity: "shared/config.nx".to_string(),
            })
        );
    }

    #[test]
    fn rejects_invalid_utf8_source_bytes() {
        assert_eq!(
            NxWorkspaceModule::from_utf8("main.nx", vec![0xff]),
            Err(NxWorkspaceInputError::InvalidSourceUtf8 {
                identity: "main.nx".to_string(),
            })
        );
    }

    #[test]
    fn workspace_module_stores_normalized_identity_and_decoded_source() {
        let module = NxWorkspaceModule::from_utf8(
            "tenant/./shared/../config.nx",
            b"let root() = { 1 }".to_vec(),
        )
        .expect("workspace module");

        assert_eq!(module.identity(), "tenant/config.nx");
        assert_eq!(module.source(), "let root() = { 1 }");
    }

    #[test]
    fn normalizes_path_like_identity_without_filesystem_access() {
        assert_eq!(
            normalize_workspace_identity("does/not/exist/../module.nx"),
            Ok("does/not/module.nx".to_string())
        );
    }

    #[test]
    fn normalizes_relative_import_against_importer_parent() {
        assert_eq!(
            normalize_workspace_import_identity("app/main.nx", "../shared/questions.nx"),
            Ok("shared/questions.nx".to_string())
        );
    }

    #[test]
    fn rejects_relative_imports_that_escape_workspace_root() {
        assert_eq!(
            normalize_workspace_import_identity("app/main.nx", "../../outside.nx"),
            Err(WorkspaceIdentityError::EscapesRoot)
        );
    }
}
