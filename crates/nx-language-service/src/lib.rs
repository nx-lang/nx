//! Protocol-independent editor language service for NX.

use nx_api::{
    validate_workspace, NxDiagnostic, NxDiagnosticLabel, NxSeverity, NxWorkspace,
    NxWorkspaceInputError, NxWorkspaceModule, ProgramBuildContext,
};
use nx_hir::{ast::TypeRef, Item, RecordKind};
use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use text_size::TextRange as ByteTextRange;
use url::Url;

const KEYWORD_COMPLETIONS: &[&str] = &[
    "import",
    "from",
    "as",
    "export",
    "private",
    "abstract",
    "type",
    "action",
    "component",
    "let",
    "if",
    "else",
    "for",
    "in",
    "match",
    "true",
    "false",
    "null",
];

const PRIMITIVE_TYPE_COMPLETIONS: &[&str] = &[
    "string", "int", "int32", "int64", "float32", "float64", "boolean", "void", "object",
];

/// Built-in type names that are valid in type position but are not primitives.
const BUILTIN_TYPE_COMPLETIONS: &[&str] = &["Element"];

/// Client-owned document URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentUri(String);

impl DocumentUri {
    /// Creates a URI from an editor-provided string.
    pub fn new(uri: impl Into<String>) -> Self {
        Self(uri.into())
    }

    /// Returns the URI string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DocumentUri {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for DocumentUri {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DocumentUri {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Normalized NX workspace identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NxIdentity(String);

impl NxIdentity {
    /// Creates and normalizes an NX workspace identity.
    pub fn new(identity: impl AsRef<str>) -> Result<Self, SnapshotError> {
        Ok(Self(normalize_identity(identity.as_ref())?))
    }

    /// Returns the normalized identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NxIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Monotonically increasing editor document version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocumentVersion(i32);

impl DocumentVersion {
    /// Creates a document version value.
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    /// Returns the raw version number.
    pub fn value(self) -> i32 {
        self.0
    }
}

/// Zero-based editor text position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPosition {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset in the line.
    pub character: u32,
}

impl TextPosition {
    /// Creates a text position.
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Editor text range with byte offsets preserved for staleness and query checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorRange {
    /// Range start.
    pub start: TextPosition,
    /// Range end.
    pub end: TextPosition,
    /// Start byte offset in the source text.
    pub start_byte: u32,
    /// End byte offset in the source text.
    pub end_byte: u32,
}

impl EditorRange {
    fn contains_byte(self, offset: usize) -> bool {
        let start = self.start_byte as usize;
        let end = self.end_byte as usize;
        start <= offset && offset <= end
    }
}

/// One document submitted to a language-service snapshot.
#[derive(Debug, Clone)]
pub struct DocumentInput {
    uri: DocumentUri,
    identity: Option<NxIdentity>,
    version: Option<DocumentVersion>,
    source: Arc<str>,
}

impl DocumentInput {
    /// Creates a document input whose NX identity will be derived from its URI.
    pub fn new(uri: impl Into<DocumentUri>, source: impl Into<Arc<str>>) -> Self {
        Self {
            uri: uri.into(),
            identity: None,
            version: None,
            source: source.into(),
        }
    }

    /// Sets an explicit normalized NX identity.
    pub fn with_identity(mut self, identity: impl AsRef<str>) -> Result<Self, SnapshotError> {
        self.identity = Some(NxIdentity::new(identity)?);
        Ok(self)
    }

    /// Sets an editor version.
    pub fn with_version(mut self, version: i32) -> Self {
        self.version = Some(DocumentVersion::new(version));
        self
    }
}

/// One immutable document in a workspace snapshot.
#[derive(Debug, Clone)]
pub struct DocumentSnapshot {
    uri: DocumentUri,
    identity: NxIdentity,
    version: Option<DocumentVersion>,
    source: Arc<str>,
}

impl DocumentSnapshot {
    /// Returns the client URI.
    pub fn uri(&self) -> &DocumentUri {
        &self.uri
    }

    /// Returns the normalized NX identity.
    pub fn identity(&self) -> &NxIdentity {
        &self.identity
    }

    /// Returns the editor version, if one was supplied.
    pub fn version(&self) -> Option<DocumentVersion> {
        self.version
    }

    /// Returns the source text.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Immutable logical editor workspace snapshot.
#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    documents: Vec<DocumentSnapshot>,
    by_uri: FxHashMap<DocumentUri, usize>,
    by_identity: FxHashMap<String, usize>,
}

impl WorkspaceSnapshot {
    /// Builds a snapshot from filesystem-backed and virtual documents.
    pub fn from_documents(
        workspace_root: Option<impl AsRef<Path>>,
        documents: Vec<DocumentInput>,
    ) -> Result<Self, SnapshotError> {
        let workspace_root = workspace_root.map(|root| root.as_ref().to_path_buf());
        let mut snapshots = Vec::with_capacity(documents.len());
        let mut by_uri = FxHashMap::default();
        let mut by_identity = FxHashMap::default();

        for input in documents {
            let identity = match input.identity {
                Some(identity) => identity,
                None => identity_from_uri(&input.uri, workspace_root.as_deref())?,
            };
            let snapshot = DocumentSnapshot {
                uri: input.uri,
                identity,
                version: input.version,
                source: input.source,
            };

            if by_uri
                .insert(snapshot.uri.clone(), snapshots.len())
                .is_some()
            {
                return Err(SnapshotError::DuplicateUri(snapshot.uri.to_string()));
            }
            if by_identity
                .insert(snapshot.identity.as_str().to_string(), snapshots.len())
                .is_some()
            {
                return Err(SnapshotError::DuplicateIdentity(
                    snapshot.identity.as_str().to_string(),
                ));
            }

            snapshots.push(snapshot);
        }

        Ok(Self {
            documents: snapshots,
            by_uri,
            by_identity,
        })
    }

    /// Returns all snapshot documents.
    pub fn documents(&self) -> &[DocumentSnapshot] {
        &self.documents
    }

    /// Returns one document by URI.
    pub fn document(&self, uri: &DocumentUri) -> Option<&DocumentSnapshot> {
        self.by_uri.get(uri).map(|index| &self.documents[*index])
    }

    /// Computes editor diagnostics for every submitted document.
    pub fn diagnostics(&self) -> Result<Vec<DocumentDiagnostics>, SnapshotError> {
        Ok(self.diagnostic_report()?.documents)
    }

    /// Computes editor diagnostics plus diagnostics that are not tied to any submitted document.
    pub fn diagnostic_report(&self) -> Result<DiagnosticReport, SnapshotError> {
        let workspace = self.to_workspace()?;
        let diagnostics = validate_workspace(&workspace, &ProgramBuildContext::empty());
        Ok(self.project_diagnostic_report(&diagnostics))
    }

    /// Extracts top-level document symbols for the requested document.
    pub fn document_symbols(
        &self,
        uri: &DocumentUri,
    ) -> Result<Vec<DocumentSymbol>, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        Ok(document_symbols_for_document(document))
    }

    /// Returns conservative hover content for a position in the requested document.
    pub fn hover(
        &self,
        uri: &DocumentUri,
        position: TextPosition,
    ) -> Result<Option<Hover>, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let index = LineIndex::new(document.source());
        let offset = index.position_to_byte_offset(document.source(), position);
        let symbols = document_symbols_for_document(document);

        Ok(symbols
            .into_iter()
            .find(|symbol| symbol.selection_range.contains_byte(offset))
            .map(|symbol| Hover {
                uri: document.uri.clone(),
                identity: document.identity.clone(),
                version: document.version,
                range: symbol.selection_range,
                contents: format!("{} `{}`", symbol.kind.display_name(), symbol.name),
            }))
    }

    /// Returns conservative completion items for a position in the requested document.
    pub fn completions(
        &self,
        uri: &DocumentUri,
        position: TextPosition,
    ) -> Result<CompletionList, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let index = LineIndex::new(document.source());
        let offset = index.position_to_byte_offset(document.source(), position);
        let declarations = self.workspace_declarations();

        let items = if let Some(members) =
            property_value_context(document.source(), offset, &declarations)
        {
            // A bare value resolves against the property's declared type, so only its members are
            // valid here; lexically visible names cannot appear unbraced.
            members
                .into_iter()
                .map(|member| CompletionItem {
                    label: member,
                    kind: CompletionItemKind::Member,
                    detail: None,
                })
                .collect()
        } else if let Some(context) =
            component_property_context(document.source(), offset, &declarations)
        {
            property_completion_items(context)
        } else if is_type_position(document.source(), offset) {
            type_completion_items(&declarations)
        } else {
            general_completion_items(&declarations)
        };

        Ok(CompletionList {
            uri: document.uri.clone(),
            identity: document.identity.clone(),
            version: document.version,
            items,
        })
    }

    fn to_workspace(&self) -> Result<NxWorkspace, SnapshotError> {
        let modules = self
            .documents
            .iter()
            .map(|document| {
                NxWorkspaceModule::from_source(
                    document.identity.as_str(),
                    Arc::clone(&document.source),
                )
            })
            .collect::<Result<Vec<_>, NxWorkspaceInputError>>()?;
        Ok(NxWorkspace::new(modules)?)
    }

    fn project_diagnostic_report(&self, diagnostics: &[NxDiagnostic]) -> DiagnosticReport {
        let mut by_document = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.identity.as_str().to_string(),
                    DocumentDiagnostics {
                        uri: document.uri.clone(),
                        identity: document.identity.clone(),
                        version: document.version,
                        diagnostics: Vec::new(),
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();
        let mut workspace = Vec::new();

        for diagnostic in diagnostics {
            match self.project_diagnostic(diagnostic) {
                ProjectedDiagnostic::Document {
                    identity,
                    diagnostic,
                } => {
                    if let Some(document_diagnostics) = by_document.get_mut(&identity) {
                        document_diagnostics.diagnostics.push(diagnostic);
                    }
                }
                ProjectedDiagnostic::Workspace(diagnostic) => {
                    workspace.push(diagnostic);
                }
            }
        }

        let documents = self
            .documents
            .iter()
            .filter_map(|document| by_document.remove(document.identity.as_str()))
            .collect();

        DiagnosticReport {
            documents,
            workspace,
        }
    }

    fn project_diagnostic(&self, diagnostic: &NxDiagnostic) -> ProjectedDiagnostic {
        let Some(primary) = diagnostic
            .labels
            .iter()
            .find(|label| label.primary)
            .or_else(|| diagnostic.labels.first())
        else {
            return ProjectedDiagnostic::Workspace(workspace_diagnostic(diagnostic));
        };
        let Some(document_index) = self.by_identity.get(&primary.file).copied() else {
            return ProjectedDiagnostic::Workspace(workspace_diagnostic(diagnostic));
        };
        let document = &self.documents[document_index];
        let primary_range = label_range(primary);
        let related = diagnostic
            .labels
            .iter()
            .filter(|label| !label.primary)
            .filter_map(|label| {
                let related_document = self
                    .by_identity
                    .get(&label.file)
                    .map(|index| &self.documents[*index])?;
                Some(RelatedLocation {
                    uri: related_document.uri.clone(),
                    identity: related_document.identity.clone(),
                    range: label_range(label),
                    message: label.message.clone(),
                })
            })
            .collect();

        ProjectedDiagnostic::Document {
            identity: document.identity.as_str().to_string(),
            diagnostic: EditorDiagnostic {
                range: primary_range,
                severity: diagnostic.severity.into(),
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
                related,
            },
        }
    }

    fn workspace_declarations(&self) -> Vec<Declaration> {
        self.documents
            .iter()
            .flat_map(declarations_for_document)
            .collect()
    }
}

enum ProjectedDiagnostic {
    Document {
        identity: String,
        diagnostic: EditorDiagnostic,
    },
    Workspace(WorkspaceDiagnostic),
}

fn workspace_diagnostic(diagnostic: &NxDiagnostic) -> WorkspaceDiagnostic {
    WorkspaceDiagnostic {
        severity: diagnostic.severity.into(),
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        labels: diagnostic
            .labels
            .iter()
            .map(|label| WorkspaceDiagnosticLabel {
                identity: label.file.clone(),
                message: label.message.clone(),
            })
            .collect(),
    }
}

/// Diagnostics report for editor integrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Diagnostics that can be published against submitted documents.
    pub documents: Vec<DocumentDiagnostics>,
    /// Diagnostics that do not map to a submitted document and should be surfaced outside inline
    /// editor ranges.
    pub workspace: Vec<WorkspaceDiagnostic>,
}

/// Diagnostic list for one document and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDiagnostics {
    /// Client URI.
    pub uri: DocumentUri,
    /// Normalized NX identity.
    pub identity: NxIdentity,
    /// Source document version used for analysis.
    pub version: Option<DocumentVersion>,
    /// Projected diagnostics.
    pub diagnostics: Vec<EditorDiagnostic>,
}

/// Diagnostic that is not tied to a submitted document URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiagnostic {
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Optional diagnostic code.
    pub code: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Original labels, if the diagnostic had labels that could not be mapped to submitted
    /// documents.
    pub labels: Vec<WorkspaceDiagnosticLabel>,
}

/// Original label metadata for a workspace diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDiagnosticLabel {
    /// Original NX identity or file name from the diagnostic label.
    pub identity: String,
    /// Optional label-specific message.
    pub message: Option<String>,
}

/// Editor diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    /// Error severity.
    Error,
    /// Warning severity.
    Warning,
    /// Informational severity.
    Info,
    /// Hint severity.
    Hint,
}

impl From<NxSeverity> for DiagnosticSeverity {
    fn from(value: NxSeverity) -> Self {
        match value {
            NxSeverity::Error => Self::Error,
            NxSeverity::Warning => Self::Warning,
            NxSeverity::Info => Self::Info,
            NxSeverity::Hint => Self::Hint,
        }
    }
}

/// One diagnostic projected for editor clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorDiagnostic {
    /// Primary range.
    pub range: EditorRange,
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Optional diagnostic code.
    pub code: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Secondary locations associated with the diagnostic.
    pub related: Vec<RelatedLocation>,
}

/// Secondary diagnostic location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedLocation {
    /// Client URI for the related location.
    pub uri: DocumentUri,
    /// NX identity for the related location.
    pub identity: NxIdentity,
    /// Related range.
    pub range: EditorRange,
    /// Optional related-location message.
    pub message: Option<String>,
}

/// Document symbol kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentSymbolKind {
    /// Function declaration.
    Function,
    /// Top-level value declaration.
    Value,
    /// Type alias declaration.
    TypeAlias,
    /// Record declaration.
    Record,
    /// Action declaration.
    Action,
    /// Enum declaration.
    Enum,
    /// Union declaration.
    Union,
    /// Component declaration.
    Component,
    /// Top-level element.
    Element,
}

impl DocumentSymbolKind {
    fn display_name(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Value => "value",
            Self::TypeAlias => "type",
            Self::Record => "record",
            Self::Action => "action",
            Self::Enum => "enum",
            Self::Union => "union",
            Self::Component => "component",
            Self::Element => "element",
        }
    }
}

/// Top-level document symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSymbol {
    /// Symbol name.
    pub name: String,
    /// Symbol kind.
    pub kind: DocumentSymbolKind,
    /// Whole declaration range.
    pub range: EditorRange,
    /// Name selection range.
    pub selection_range: EditorRange,
}

/// Hover result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hover {
    /// Client URI.
    pub uri: DocumentUri,
    /// NX identity.
    pub identity: NxIdentity,
    /// Document version used for the result.
    pub version: Option<DocumentVersion>,
    /// Hover range.
    pub range: EditorRange,
    /// Markdown-ish hover content.
    pub contents: String,
}

/// Completion response for one document and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionList {
    /// Client URI.
    pub uri: DocumentUri,
    /// NX identity.
    pub identity: NxIdentity,
    /// Document version used for the result.
    pub version: Option<DocumentVersion>,
    /// Completion candidates.
    pub items: Vec<CompletionItem>,
}

/// Completion item kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionItemKind {
    /// Language keyword.
    Keyword,
    /// Type name.
    Type,
    /// Top-level declaration.
    Declaration,
    /// Component/tag name.
    Component,
    /// Component property.
    Property,
    /// Enum member or payloadless union case, offered at a property value position.
    Member,
}

/// Completion candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    /// Insert/display label.
    pub label: String,
    /// Candidate kind.
    pub kind: CompletionItemKind,
    /// Optional detail text.
    pub detail: Option<String>,
}

/// Snapshot construction and analysis error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// URI string failed to parse.
    InvalidUri(String),
    /// URI could not be converted to a filesystem path.
    InvalidFileUri(String),
    /// Derived identity was invalid.
    InvalidIdentity { identity: String, message: String },
    /// Duplicate client URI.
    DuplicateUri(String),
    /// Duplicate NX identity.
    DuplicateIdentity(String),
    /// Requested document is not present.
    UnknownDocument(String),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUri(uri) => write!(formatter, "Invalid document URI '{}'", uri),
            Self::InvalidFileUri(uri) => {
                write!(formatter, "Document URI '{}' is not a valid file URI", uri)
            }
            Self::InvalidIdentity { identity, message } => {
                write!(formatter, "Invalid NX identity '{}': {}", identity, message)
            }
            Self::DuplicateUri(uri) => write!(formatter, "Duplicate document URI '{}'", uri),
            Self::DuplicateIdentity(identity) => {
                write!(formatter, "Duplicate NX identity '{}'", identity)
            }
            Self::UnknownDocument(uri) => write!(formatter, "Unknown document URI '{}'", uri),
        }
    }
}

impl Error for SnapshotError {}

impl From<NxWorkspaceInputError> for SnapshotError {
    fn from(value: NxWorkspaceInputError) -> Self {
        match value {
            NxWorkspaceInputError::InvalidIdentity { identity, message } => {
                SnapshotError::InvalidIdentity { identity, message }
            }
            NxWorkspaceInputError::DuplicateIdentity { identity } => {
                SnapshotError::DuplicateIdentity(identity)
            }
            NxWorkspaceInputError::InvalidSourceUtf8 { identity } => {
                SnapshotError::InvalidIdentity {
                    identity,
                    message: "source is not valid UTF-8".to_string(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Declaration {
    name: String,
    kind: DocumentSymbolKind,
    detail: String,
    properties: Vec<String>,
    /// Property name paired with the base name of its declared type, for contextual completions.
    property_types: Vec<(String, String)>,
    /// Enum members or union case names, for contextual completions at a value position.
    members: Vec<String>,
}

fn identity_from_uri(
    uri: &DocumentUri,
    workspace_root: Option<&Path>,
) -> Result<NxIdentity, SnapshotError> {
    let parsed =
        Url::parse(uri.as_str()).map_err(|_| SnapshotError::InvalidUri(uri.to_string()))?;
    if parsed.scheme() == "file" {
        let path = parsed
            .to_file_path()
            .map_err(|_| SnapshotError::InvalidFileUri(uri.to_string()))?;
        let identity = file_identity(&path, workspace_root);
        return NxIdentity::new(identity);
    }

    let mut segments = Vec::new();
    if let Some(host) = parsed.host_str() {
        if !host.is_empty() {
            segments.push(host.to_string());
        }
    }
    if let Some(path_segments) = parsed.path_segments() {
        segments.extend(
            path_segments
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string),
        );
    }
    if segments.is_empty() {
        segments.push(parsed.scheme().to_string());
    }

    NxIdentity::new(segments.join("/"))
}

fn file_identity(path: &Path, workspace_root: Option<&Path>) -> String {
    let relative = workspace_root
        .and_then(|root| path.strip_prefix(root).ok())
        .unwrap_or_else(|| path.file_name().map(Path::new).unwrap_or(path));
    path_to_identity(relative)
}

fn path_to_identity(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            std::path::Component::CurDir => Some(".".to_string()),
            std::path::Component::ParentDir => Some("..".to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_identity(identity: &str) -> Result<String, SnapshotError> {
    let identity = identity.trim().replace('\\', "/");
    if identity.is_empty() {
        return Err(SnapshotError::InvalidIdentity {
            identity,
            message: "identity must not be empty".to_string(),
        });
    }
    if identity.starts_with('/') {
        return Err(SnapshotError::InvalidIdentity {
            identity,
            message: "identity must not be absolute".to_string(),
        });
    }

    let mut normalized = Vec::new();
    for raw_segment in identity.split('/') {
        let segment = raw_segment.trim();
        if segment.is_empty() {
            return Err(SnapshotError::InvalidIdentity {
                identity: identity.clone(),
                message: "identity must not contain empty segments".to_string(),
            });
        }
        match segment {
            "." => {}
            ".." => {
                if normalized.pop().is_none() {
                    return Err(SnapshotError::InvalidIdentity {
                        identity: identity.clone(),
                        message: "identity escapes the workspace root".to_string(),
                    });
                }
            }
            _ => normalized.push(segment.to_string()),
        }
    }

    if normalized.is_empty() {
        return Err(SnapshotError::InvalidIdentity {
            identity,
            message: "identity must not be empty".to_string(),
        });
    }

    Ok(normalized.join("/"))
}

fn label_range(label: &NxDiagnosticLabel) -> EditorRange {
    EditorRange {
        start: TextPosition::new(
            label.span.start_line.saturating_sub(1) as u32,
            label.span.start_column.saturating_sub(1) as u32,
        ),
        end: TextPosition::new(
            label.span.end_line.saturating_sub(1) as u32,
            label.span.end_column.saturating_sub(1) as u32,
        ),
        start_byte: label.span.start_byte.max(0) as u32,
        end_byte: label.span.end_byte.max(0) as u32,
    }
}

fn document_symbols_for_document(document: &DocumentSnapshot) -> Vec<DocumentSymbol> {
    let parse_result = parse_str(document.source(), document.identity.as_str());
    let Some(tree) = parse_result.tree else {
        return Vec::new();
    };
    let index = LineIndex::new(document.source());
    let mut symbols = Vec::new();

    for child in tree.root().children() {
        if let Some((kind, name_node)) = symbol_name_node(child) {
            let name = clean_symbol_name(name_node.text());
            if name.is_empty() {
                continue;
            }

            symbols.push(DocumentSymbol {
                name,
                kind,
                range: index.range_from_bytes(child.span()),
                selection_range: index.range_from_bytes(name_node.span()),
            });
        }
    }
    collect_declaration_symbols(tree.root(), &index, &mut symbols);

    for symbol in hir_document_symbols(document) {
        if !symbols
            .iter()
            .any(|existing| existing.name == symbol.name && existing.kind == symbol.kind)
        {
            symbols.push(symbol);
        }
    }

    symbols
}

fn collect_declaration_symbols(
    node: SyntaxNode<'_>,
    index: &LineIndex,
    symbols: &mut Vec<DocumentSymbol>,
) {
    for child in node.children() {
        if !matches!(
            child.kind(),
            SyntaxKind::ELEMENT | SyntaxKind::SELF_CLOSING_ELEMENT
        ) {
            if let Some((kind, name_node)) = symbol_name_node(child) {
                let name = clean_symbol_name(name_node.text());
                if !name.is_empty()
                    && !symbols
                        .iter()
                        .any(|symbol| symbol.name == name && symbol.kind == kind)
                {
                    symbols.push(DocumentSymbol {
                        name,
                        kind,
                        range: index.range_from_bytes(child.span()),
                        selection_range: index.range_from_bytes(name_node.span()),
                    });
                }
            }
        }

        collect_declaration_symbols(child, index, symbols);
    }
}

fn hir_document_symbols(document: &DocumentSnapshot) -> Vec<DocumentSymbol> {
    let artifact = nx_types::analyze_str(document.source(), document.identity.as_str());
    let Some(module) = artifact.lowered_module else {
        return Vec::new();
    };
    let index = LineIndex::new(document.source());

    module
        .items()
        .iter()
        .map(|item| {
            let declaration = declaration_from_item(item, document.source());
            let range = item_span(item);
            DocumentSymbol {
                name: declaration.name,
                kind: declaration.kind,
                range: index.range_from_bytes(range),
                selection_range: selection_range_for_name(document.source(), &index, range, item),
            }
        })
        .collect()
}

fn item_span(item: &Item) -> ByteTextRange {
    match item {
        Item::Function(function) => function.span,
        Item::Value(value) => value.span,
        Item::Component(component) => component.span,
        Item::TypeAlias(alias) => alias.span,
        Item::Enum(enum_def) => enum_def.span,
        Item::Union(union_def) => union_def.span,
        Item::Record(record) => record.span,
    }
}

fn selection_range_for_name(
    source: &str,
    index: &LineIndex,
    range: ByteTextRange,
    item: &Item,
) -> EditorRange {
    let name = item.name().as_str();
    let start: usize = range.start().into();
    let end: usize = range.end().into();
    let Some(slice) = source.get(start..end) else {
        return index.range_from_bytes(range);
    };
    if let Some(relative_start) = slice.find(name) {
        let selection_start = start + relative_start;
        let selection_end = selection_start + name.len();
        return index.range_from_bytes(ByteTextRange::new(
            (selection_start as u32).into(),
            (selection_end as u32).into(),
        ));
    }

    index.range_from_bytes(range)
}

fn symbol_name_node(node: SyntaxNode<'_>) -> Option<(DocumentSymbolKind, SyntaxNode<'_>)> {
    match node.kind() {
        SyntaxKind::FUNCTION_DEFINITION => {
            if node.text().trim_start().starts_with("let <") {
                component_name(node).map(|name| (DocumentSymbolKind::Component, name))
            } else {
                declaration_name(node).map(|name| (DocumentSymbolKind::Function, name))
            }
        }
        SyntaxKind::VALUE_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::Value, name))
        }
        SyntaxKind::TYPE_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::TypeAlias, name))
        }
        SyntaxKind::RECORD_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::Record, name))
        }
        SyntaxKind::ACTION_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::Action, name))
        }
        SyntaxKind::ENUM_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::Enum, name))
        }
        SyntaxKind::UNION_DEFINITION => {
            declaration_name(node).map(|name| (DocumentSymbolKind::Union, name))
        }
        SyntaxKind::COMPONENT_DEFINITION => component_name(node)
            .or_else(|| declaration_name(node))
            .map(|name| (DocumentSymbolKind::Component, name)),
        SyntaxKind::ELEMENT | SyntaxKind::SELF_CLOSING_ELEMENT => {
            element_name(node).map(|name| (DocumentSymbolKind::Element, name))
        }
        _ => None,
    }
}

fn declaration_name(node: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
    node.child_by_field("name")
        .or_else(|| {
            node.child_by_field("signature")
                .and_then(|signature| signature.child_by_field("name"))
        })
        .or_else(|| {
            first_descendant_matching(
                node,
                &[
                    SyntaxKind::IDENTIFIER,
                    SyntaxKind::QUALIFIED_NAME,
                    SyntaxKind::MARKUP_IDENTIFIER,
                    SyntaxKind::QUALIFIED_MARKUP_NAME,
                    SyntaxKind::ELEMENT_NAME,
                ],
            )
        })
}

fn component_name(node: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
    node.child_by_field("signature")
        .and_then(|signature| signature.child_by_field("name"))
        .or_else(|| {
            first_descendant_matching(
                node,
                &[
                    SyntaxKind::MARKUP_IDENTIFIER,
                    SyntaxKind::QUALIFIED_MARKUP_NAME,
                ],
            )
        })
}

fn element_name(node: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
    node.child_by_field("open_tag")
        .and_then(|open_tag| open_tag.child_by_field("name"))
        .or_else(|| {
            first_descendant_matching(
                node,
                &[SyntaxKind::ELEMENT_NAME, SyntaxKind::QUALIFIED_MARKUP_NAME],
            )
        })
}

fn first_descendant_matching<'tree>(
    node: SyntaxNode<'tree>,
    kinds: &[SyntaxKind],
) -> Option<SyntaxNode<'tree>> {
    for child in node.children() {
        if kinds.contains(&child.kind()) {
            return Some(child);
        }
        if let Some(descendant) = first_descendant_matching(child, kinds) {
            return Some(descendant);
        }
    }
    None
}

fn clean_symbol_name(text: &str) -> String {
    let cleaned = text
        .trim()
        .trim_start_matches('<')
        .trim_start_matches('/')
        .trim_end_matches("/>")
        .trim_end_matches('>')
        .trim();
    cleaned
        .chars()
        .take_while(|ch| !ch.is_whitespace() && !matches!(ch, '/' | '>'))
        .collect()
}

fn declarations_for_document(document: &DocumentSnapshot) -> Vec<Declaration> {
    let artifact = nx_types::analyze_str(document.source(), document.identity.as_str());
    let Some(module) = artifact.lowered_module else {
        return Vec::new();
    };

    module
        .items()
        .iter()
        .map(|item| declaration_from_item(item, document.source()))
        .collect::<Vec<_>>()
}

fn declaration_from_item(item: &Item, source: &str) -> Declaration {
    match item {
        Item::Function(function) => {
            let is_markup_function = source_text_for_range(source, function.span)
                .trim_start()
                .starts_with("let <");
            Declaration {
                name: function.name.as_str().to_string(),
                kind: if is_markup_function {
                    DocumentSymbolKind::Component
                } else {
                    DocumentSymbolKind::Function
                },
                detail: if is_markup_function {
                    format!("component <{} />", function.name.as_str())
                } else {
                    format!("function {}", function_signature(function))
                },
                properties: if is_markup_function {
                    function
                        .params
                        .iter()
                        .map(|param| param.name.as_str().to_string())
                        .collect()
                } else {
                    Vec::new()
                },
                property_types: if is_markup_function {
                    function
                        .params
                        .iter()
                        .map(|param| {
                            (
                                param.name.as_str().to_string(),
                                base_type_name(&param.ty),
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                members: Vec::new(),
            }
        }
        Item::Value(value) => Declaration {
            name: value.name.as_str().to_string(),
            kind: DocumentSymbolKind::Value,
            detail: value
                .ty
                .as_ref()
                .map(|ty| format!("value: {}", type_ref_display(ty)))
                .unwrap_or_else(|| "value".to_string()),
            properties: Vec::new(),
            property_types: Vec::new(),
            members: Vec::new(),
        },
        Item::Component(component) => Declaration {
            name: component.name.as_str().to_string(),
            kind: DocumentSymbolKind::Component,
            detail: format!("component <{} />", component.name.as_str()),
            properties: component
                .props
                .iter()
                .map(|property| property.name.as_str().to_string())
                .collect(),
            property_types: component
                .props
                .iter()
                .map(|property| {
                    (
                        property.name.as_str().to_string(),
                        base_type_name(&property.ty),
                    )
                })
                .collect(),
            members: Vec::new(),
        },
        Item::TypeAlias(alias) => Declaration {
            name: alias.name.as_str().to_string(),
            kind: DocumentSymbolKind::TypeAlias,
            detail: format!("type = {}", type_ref_display(&alias.ty)),
            properties: Vec::new(),
            property_types: Vec::new(),
            members: Vec::new(),
        },
        Item::Enum(enum_def) => Declaration {
            name: enum_def.name.as_str().to_string(),
            kind: DocumentSymbolKind::Enum,
            detail: "enum".to_string(),
            properties: Vec::new(),
            property_types: Vec::new(),
            members: enum_def
                .members
                .iter()
                .map(|member| member.name.as_str().to_string())
                .collect(),
        },
        Item::Union(union_def) => Declaration {
            name: union_def.name.as_str().to_string(),
            kind: DocumentSymbolKind::Union,
            detail: "union".to_string(),
            // Only payloadless cases have a bare spelling; a payload case needs element-style
            // construction and must not be offered here.
            properties: Vec::new(),
            property_types: Vec::new(),
            members: union_def
                .cases
                .iter()
                .filter(|case| case.fields.is_empty())
                .map(|case| case.name.as_str().to_string())
                .collect(),
        },
        Item::Record(record) => Declaration {
            name: record.name.as_str().to_string(),
            kind: if record.kind == RecordKind::Action {
                DocumentSymbolKind::Action
            } else {
                DocumentSymbolKind::Record
            },
            detail: if record.kind == RecordKind::Action {
                "action".to_string()
            } else {
                "record".to_string()
            },
            properties: record
                .properties
                .iter()
                .map(|property| property.name.as_str().to_string())
                .collect(),
            property_types: record
                .properties
                .iter()
                .map(|property| {
                    (
                        property.name.as_str().to_string(),
                        base_type_name(&property.ty),
                    )
                })
                .collect(),
            members: Vec::new(),
        },
    }
}

fn source_text_for_range(source: &str, range: ByteTextRange) -> &str {
    let start: usize = range.start().into();
    let end: usize = range.end().into();
    source.get(start..end).unwrap_or_default()
}

fn function_signature(function: &nx_hir::Function) -> String {
    let params = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name.as_str(), type_ref_display(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = function
        .return_type
        .as_ref()
        .map(|ty| format!(": {}", type_ref_display(ty)))
        .unwrap_or_default();
    format!("{}({}){}", function.name.as_str(), params, return_type)
}

/// Strips nullability and one list level to reach the type a bare value would resolve against.
///
/// Mirrors the checker's normalization, so completions offer members exactly where the compiler
/// would accept a bare name.
fn base_type_name(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Name(name) => name.as_str().to_string(),
        TypeRef::Nullable(inner) | TypeRef::Array(inner) => base_type_name(inner),
        TypeRef::Function { .. } => String::new(),
    }
}

fn type_ref_display(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Name(name) => name.as_str().to_string(),
        TypeRef::Array(inner) => format!("{}[]", type_ref_display(inner)),
        TypeRef::Nullable(inner) => format!("{}?", type_ref_display(inner)),
        TypeRef::Function {
            params,
            return_type,
        } => {
            let params = params
                .iter()
                .map(type_ref_display)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) => {}", params, type_ref_display(return_type))
        }
    }
}

/// Detects a property *value* position — immediately after `prop=` inside an opening tag — and
/// returns the members a bare name could resolve to there.
///
/// Returns `None` when the element or property is unknown, or when the property's declared type is
/// not an enum or a union, because a bare name is not accepted at those sites either.
fn property_value_context(
    source: &str,
    offset: usize,
    declarations: &[Declaration],
) -> Option<Vec<String>> {
    let prefix = source.get(..offset)?;
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_prefix = &prefix[line_start..];
    let tag_start = line_prefix.rfind('<')?;
    if line_prefix[tag_start..].starts_with("</") || line_prefix[tag_start..].starts_with("<:") {
        return None;
    }
    if line_prefix[tag_start..].contains('>') {
        return None;
    }

    // The cursor must sit in the value of `name=`, with nothing typed yet or a partial bare word.
    let mut cursor = line_prefix.len();
    let bytes = line_prefix.as_bytes();
    while cursor > 0 && is_identifier_continue(bytes[cursor - 1] as char) {
        cursor -= 1;
    }
    if cursor == 0 || bytes[cursor - 1] != b'=' {
        return None;
    }
    // A quoted or braced value is not a contextual-name position.
    let name_end = cursor - 1;
    let mut name_start = name_end;
    while name_start > 0 && is_identifier_continue(bytes[name_start - 1] as char) {
        name_start -= 1;
    }
    if name_start == name_end {
        return None;
    }
    let property_name = &line_prefix[name_start..name_end];

    let after_lt = &line_prefix[tag_start + 1..];
    let tag_name = after_lt
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect::<String>();
    if tag_name.is_empty() {
        return None;
    }

    let element = declarations.iter().find(|declaration| {
        matches!(
            declaration.kind,
            DocumentSymbolKind::Component | DocumentSymbolKind::Record
        ) && declaration.name == tag_name
    })?;

    let type_name = element
        .property_types
        .iter()
        .find(|(name, _)| name == property_name)
        .map(|(_, type_name)| type_name.clone())?;

    let target = declarations.iter().find(|declaration| {
        matches!(
            declaration.kind,
            DocumentSymbolKind::Enum | DocumentSymbolKind::Union
        ) && declaration.name == type_name
    })?;

    if target.members.is_empty() {
        return None;
    }
    Some(target.members.clone())
}

fn component_property_context(
    source: &str,
    offset: usize,
    declarations: &[Declaration],
) -> Option<PropertyCompletionContext> {
    let prefix = source.get(..offset)?;
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_prefix = &prefix[line_start..];
    let tag_start = line_prefix.rfind('<')?;
    if line_prefix[tag_start..].starts_with("</") || line_prefix[tag_start..].starts_with("<:") {
        return None;
    }
    if line_prefix[tag_start..].contains('>') {
        return None;
    }

    let after_lt = &line_prefix[tag_start + 1..];
    let tag_name = after_lt
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect::<String>();
    if tag_name.is_empty() {
        return None;
    }

    let declaration = declarations.iter().find(|declaration| {
        declaration.kind == DocumentSymbolKind::Component && declaration.name == tag_name
    })?;
    let supplied = supplied_properties(after_lt.get(tag_name.len()..).unwrap_or_default());

    Some(PropertyCompletionContext {
        properties: declaration.properties.clone(),
        supplied,
    })
}

fn supplied_properties(text: &str) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        while index < bytes.len() && !is_identifier_start(bytes[index] as char) {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && is_identifier_continue(bytes[index] as char) {
            index += 1;
        }
        if start == index {
            continue;
        }

        let name = &text[start..index];
        let mut lookahead = index;
        while lookahead < bytes.len() && (bytes[lookahead] as char).is_whitespace() {
            lookahead += 1;
        }
        if lookahead < bytes.len() && bytes[lookahead] as char == '=' {
            out.insert(name.to_string());
        }
    }
    out
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch == '-' || ch.is_ascii_alphanumeric()
}

#[derive(Debug, Clone)]
struct PropertyCompletionContext {
    properties: Vec<String>,
    supplied: FxHashSet<String>,
}

fn property_completion_items(context: PropertyCompletionContext) -> Vec<CompletionItem> {
    context
        .properties
        .into_iter()
        .filter(|property| !context.supplied.contains(property))
        .map(|property| CompletionItem {
            label: property,
            kind: CompletionItemKind::Property,
            detail: Some("component property".to_string()),
        })
        .collect()
}

fn is_type_position(source: &str, offset: usize) -> bool {
    let prefix = source.get(..offset).unwrap_or(source);
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_prefix = &prefix[line_start..];
    let colon = line_prefix.rfind(':');
    let equals = line_prefix.rfind('=');
    matches!((colon, equals), (Some(colon), None) if colon < line_prefix.len())
        || matches!((colon, equals), (Some(colon), Some(equals)) if colon > equals)
}

fn type_completion_items(declarations: &[Declaration]) -> Vec<CompletionItem> {
    let mut items = PRIMITIVE_TYPE_COMPLETIONS
        .iter()
        .map(|label| CompletionItem {
            label: (*label).to_string(),
            kind: CompletionItemKind::Type,
            detail: Some("primitive type".to_string()),
        })
        .collect::<Vec<_>>();

    items.extend(BUILTIN_TYPE_COMPLETIONS.iter().map(|label| CompletionItem {
        label: (*label).to_string(),
        kind: CompletionItemKind::Type,
        detail: Some("built-in type".to_string()),
    }));

    items.extend(declarations.iter().filter_map(|declaration| {
        matches!(
            declaration.kind,
            DocumentSymbolKind::TypeAlias
                | DocumentSymbolKind::Record
                | DocumentSymbolKind::Action
                | DocumentSymbolKind::Enum
                | DocumentSymbolKind::Union
                | DocumentSymbolKind::Component
        )
        .then(|| CompletionItem {
            label: declaration.name.clone(),
            kind: CompletionItemKind::Type,
            detail: Some(declaration.detail.clone()),
        })
    }));

    dedupe_completions(items)
}

fn general_completion_items(declarations: &[Declaration]) -> Vec<CompletionItem> {
    let mut items = KEYWORD_COMPLETIONS
        .iter()
        .map(|label| CompletionItem {
            label: (*label).to_string(),
            kind: CompletionItemKind::Keyword,
            detail: None,
        })
        .collect::<Vec<_>>();

    items.extend(declarations.iter().map(|declaration| CompletionItem {
        label: declaration.name.clone(),
        kind: if declaration.kind == DocumentSymbolKind::Component {
            CompletionItemKind::Component
        } else {
            CompletionItemKind::Declaration
        },
        detail: Some(declaration.detail.clone()),
    }));

    dedupe_completions(items)
}

fn dedupe_completions(items: Vec<CompletionItem>) -> Vec<CompletionItem> {
    let mut seen = FxHashSet::default();
    items
        .into_iter()
        .filter(|item| seen.insert(item.label.clone()))
        .collect()
}

struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (index, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }

        Self { line_starts }
    }

    fn range_from_bytes(&self, range: ByteTextRange) -> EditorRange {
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        EditorRange {
            start: self.byte_offset_to_position(start),
            end: self.byte_offset_to_position(end),
            start_byte: start as u32,
            end_byte: end as u32,
        }
    }

    fn position_to_byte_offset(&self, text: &str, position: TextPosition) -> usize {
        let line_start = self
            .line_starts
            .get(position.line as usize)
            .copied()
            .unwrap_or_else(|| text.len());
        let line_end = self
            .line_starts
            .get(position.line as usize + 1)
            .copied()
            .unwrap_or_else(|| text.len());
        let line = &text[line_start..line_end];
        let character = position.character as usize;
        line.char_indices()
            .nth(character)
            .map(|(index, _)| line_start + index)
            .unwrap_or(line_end)
    }

    fn byte_offset_to_position(&self, offset: usize) -> TextPosition {
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        TextPosition::new(
            line_index as u32,
            (offset.saturating_sub(line_start)) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_api::NxTextSpan;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn snapshot_for(uri: &str, source: &str, version: i32) -> WorkspaceSnapshot {
        WorkspaceSnapshot::from_documents(
            Option::<PathBuf>::None,
            vec![DocumentInput::new(uri, source).with_version(version)],
        )
        .expect("snapshot")
    }

    #[test]
    fn filesystem_document_maps_to_workspace_identity() {
        let temp = TempDir::new().expect("temp dir");
        let source_path = temp.path().join("forms").join("signup.nx");
        let uri = Url::from_file_path(&source_path).expect("file uri");

        let snapshot = WorkspaceSnapshot::from_documents(
            Some(temp.path()),
            vec![DocumentInput::new(uri.to_string(), "let value = 1").with_version(7)],
        )
        .expect("snapshot");

        let document = snapshot.documents().first().expect("document");
        assert_eq!(document.identity().as_str(), "forms/signup.nx");
        assert_eq!(document.version(), Some(DocumentVersion::new(7)));
    }

    #[test]
    fn virtual_document_maps_to_logical_identity_without_filesystem() {
        let snapshot = snapshot_for("nx://tenant/form.nx", "let value = 1", 1);

        let document = snapshot.documents().first().expect("document");
        assert_eq!(document.identity().as_str(), "tenant/form.nx");
        assert_eq!(document.uri().as_str(), "nx://tenant/form.nx");
    }

    #[test]
    fn diagnostics_clear_when_document_becomes_valid() {
        let invalid = snapshot_for("nx://tenant/form.nx", "let count: string = 1", 1);
        let invalid_diagnostics = invalid.diagnostics().expect("invalid diagnostics");
        assert!(
            invalid_diagnostics[0]
                .diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.severity == DiagnosticSeverity::Error
                        && diagnostic
                            .code
                            .as_deref()
                            .is_some_and(|code| code.contains("type-mismatch"))
                ),
            "expected type mismatch diagnostics: {invalid_diagnostics:#?}"
        );

        let valid = snapshot_for("nx://tenant/form.nx", "let count: string = \"one\"", 2);
        let valid_diagnostics = valid.diagnostics().expect("valid diagnostics");
        assert_eq!(valid_diagnostics[0].version, Some(DocumentVersion::new(2)));
        assert!(valid_diagnostics[0].diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_preserve_stale_version_metadata() {
        let snapshot = snapshot_for("nx://tenant/form.nx", "let count: string = 1", 3);

        let diagnostics = snapshot.diagnostics().expect("diagnostics");

        assert_eq!(diagnostics[0].version, Some(DocumentVersion::new(3)));
    }

    #[test]
    fn diagnostics_separate_unmapped_or_label_less_results() {
        let snapshot = snapshot_for("nx://tenant/form.nx", "let value = 1", 1);
        let report = snapshot.project_diagnostic_report(&[
            NxDiagnostic {
                severity: NxSeverity::Error,
                code: Some("workspace".to_string()),
                message: "workspace diagnostic".to_string(),
                labels: Vec::new(),
                help: None,
                note: None,
            },
            NxDiagnostic {
                severity: NxSeverity::Error,
                code: Some("other".to_string()),
                message: "other diagnostic".to_string(),
                labels: vec![NxDiagnosticLabel {
                    file: "other.nx".to_string(),
                    span: NxTextSpan {
                        start_byte: 0,
                        end_byte: 1,
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 2,
                    },
                    message: None,
                    primary: true,
                }],
                help: None,
                note: None,
            },
        ]);

        assert_eq!(report.documents.len(), 1);
        assert!(report.documents[0].diagnostics.is_empty());
        assert_eq!(report.workspace.len(), 2);
        assert_eq!(report.workspace[0].code.as_deref(), Some("workspace"));
        assert_eq!(report.workspace[1].labels[0].identity, "other.nx");
    }

    #[test]
    fn document_symbols_include_top_level_declarations() {
        let source = r#"
type User = { name: string }
action Save = { id: int }
let title = "Hello"
"#;
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let symbols = snapshot.document_symbols(&uri).expect("symbols");
        let names = symbols
            .iter()
            .map(|symbol| (symbol.name.as_str(), symbol.kind))
            .collect::<Vec<_>>();

        assert!(
            names.contains(&("User", DocumentSymbolKind::Record)),
            "{names:#?}"
        );
        assert!(
            names.contains(&("Save", DocumentSymbolKind::Action)),
            "{names:#?}"
        );
        assert!(
            names.contains(&("title", DocumentSymbolKind::Value)),
            "{names:#?}"
        );
    }

    #[test]
    fn document_symbols_include_functions() {
        let source = "let root() = 1";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let symbols = snapshot.document_symbols(&uri).expect("symbols");
        let names = symbols
            .iter()
            .map(|symbol| (symbol.name.as_str(), symbol.kind))
            .collect::<Vec<_>>();

        assert!(
            names.contains(&("root", DocumentSymbolKind::Function)),
            "{names:#?}"
        );
    }

    #[test]
    fn document_symbols_include_unions() {
        let source = "type LoadState = | idle";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let symbols = snapshot.document_symbols(&uri).expect("symbols");
        let names = symbols
            .iter()
            .map(|symbol| (symbol.name.as_str(), symbol.kind))
            .collect::<Vec<_>>();

        assert!(
            names.contains(&("LoadState", DocumentSymbolKind::Union)),
            "{names:#?}"
        );
    }

    #[test]
    fn document_symbols_include_components_and_root_elements() {
        let source = r#"
component <SearchBox placeholder:string /> = {
  <input value={placeholder}/>
}

<SearchBox placeholder="Find docs" />
"#;
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let symbols = snapshot.document_symbols(&uri).expect("symbols");
        let names = symbols
            .iter()
            .map(|symbol| (symbol.name.as_str(), symbol.kind))
            .collect::<Vec<_>>();

        assert!(
            names.contains(&("SearchBox", DocumentSymbolKind::Component)),
            "{names:#?}"
        );
        assert!(
            names.contains(&("SearchBox", DocumentSymbolKind::Element)),
            "{names:#?}"
        );
    }

    #[test]
    fn declaration_hover_returns_conservative_content() {
        let source = "let root() = 1";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let hover = snapshot
            .hover(&uri, TextPosition::new(0, 5))
            .expect("hover")
            .expect("hover content");

        assert_eq!(hover.contents, "function `root`");
        assert_eq!(hover.version, Some(DocumentVersion::new(1)));
    }

    #[test]
    fn hover_returns_none_for_unknown_syntax() {
        let source = "let root() = 1";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let hover = snapshot
            .hover(&uri, TextPosition::new(0, 3))
            .expect("hover");

        assert!(hover.is_none());
    }

    #[test]
    fn type_position_completions_include_primitives_and_visible_types() {
        let source = "type User = { name: string }\nlet value:  = 1";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(1, 11))
            .expect("completions");
        let labels = completions
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"string"));
        assert!(labels.contains(&"boolean"));
        assert!(labels.contains(&"int"));
        assert!(labels.contains(&"int32"));
        assert!(labels.contains(&"int64"));
        assert!(labels.contains(&"User"));
        assert!(!labels.contains(&"bool"));
        assert!(!labels.contains(&"long"));
        assert!(!labels.contains(&"double"));
    }

    #[test]
    fn property_value_completions_offer_members_of_the_declared_type() {
        let source = "enum Fit = fill | contain | cover\nlet <Img fit:Fit /> = <img />\n<Img fit= />\n";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(2, 9))
            .expect("completions");
        let labels = completions
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"fill"), "got: {labels:?}");
        assert!(labels.contains(&"contain"), "got: {labels:?}");
        assert!(labels.contains(&"cover"), "got: {labels:?}");
        // Only members are valid unbraced, so nothing lexical may be offered here.
        assert!(!labels.contains(&"Img"), "got: {labels:?}");
        assert!(!labels.contains(&"Fit"), "got: {labels:?}");
        assert!(
            completions
                .items
                .iter()
                .all(|item| item.kind == CompletionItemKind::Member),
            "every item should be a member: {:?}",
            completions.items
        );
    }

    #[test]
    fn property_value_completions_offer_only_payloadless_union_cases() {
        let source = concat!(
            "type LoadState = | idle | failed { message:string }\n",
            "let <View state:LoadState /> = <div />\n",
            "<View state= />\n"
        );
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(2, 12))
            .expect("completions");
        let labels = completions
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"idle"), "got: {labels:?}");
        // `failed` carries a payload, so it has no bare spelling.
        assert!(!labels.contains(&"failed"), "got: {labels:?}");
    }

    #[test]
    fn property_value_completions_absent_without_a_nominal_type() {
        let source = "let <Img alt:string /> = <img />\n<Img alt= />\n";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(1, 9))
            .expect("completions");

        assert!(
            completions
                .items
                .iter()
                .all(|item| item.kind != CompletionItemKind::Member),
            "a string-typed property has no contextual members: {:?}",
            completions.items
        );
    }

    #[test]
    fn component_property_completions_omit_supplied_properties() {
        let source = r#"
let <Card title:string subtitle:string /> = <div>{title}</div>
<Card title="Hello" />
"#;
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(2, 20))
            .expect("completions");
        let labels = completions
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(!labels.contains(&"title"));
        assert!(labels.contains(&"subtitle"));
    }
}
