use rustc_hash::FxHashSet;
use std::error::Error;
use std::fmt;
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

#[cfg(test)]
mod tests {
    use super::*;

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
