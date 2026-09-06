//! Protocol-independent editor language service for NX.

mod positions;

use nx_api::{
    analyze_workspace_modules, validate_workspace, NxDiagnostic, NxDiagnosticLabel, NxSeverity,
    NxWorkspace, NxWorkspaceInputError, NxWorkspaceModule, ProgramBuildContext,
};
use nx_hir::{ast::TypeRef, Item, LocalDefinitionId, LoweredModule, PreparedNamespace, RecordKind};
use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};
use nx_types::{ModuleArtifact, TypeEnvironment};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, OnceLock};
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
    "string", "int", "int32", "int64", "float32", "float64", "boolean", "object",
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
    /// The workspace analysis every completion request reads, computed at most once.
    ///
    /// <para>Building it type checks every module in the workspace. A snapshot is immutable, so
    /// the result is too, and a keystroke cannot afford to derive it again — the plain keyword
    /// path pays for it as much as the ones that use it.</para>
    declarations: OnceLock<Arc<WorkspaceDeclarations>>,
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
            declarations: OnceLock::new(),
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
    ///
    /// <para>The position is resolved against the analyzed document, so hover answers at a
    /// reference and at an expression, not only at the name of a top-level declaration. Where the
    /// resolved position carries nothing the analysis knows about, hover returns nothing rather
    /// than a fabricated result.</para>
    pub fn hover(
        &self,
        uri: &DocumentUri,
        position: TextPosition,
    ) -> Result<Option<Hover>, SnapshotError> {
        let resolved = self.resolve_position(uri, position)?;

        let Some(contents) = self.hover_contents(&resolved) else {
            return Ok(None);
        };
        let Some(span) = resolved.context.span() else {
            return Ok(None);
        };

        Ok(Some(Hover {
            uri: resolved.document.uri.clone(),
            identity: resolved.document.identity.clone(),
            version: resolved.document.version,
            range: resolved.index.range_from_bytes(span),
            contents,
        }))
    }

    /// Turns a position query into the construct it names, once, for every query that asks.
    ///
    /// <para>Hover and completions must agree about what the cursor is on. Two hand-written copies
    /// of this preamble is where that agreement would drift, so there is one. A document that does
    /// not parse resolves to no construct, which both entry points already answer conservatively.
    /// </para>
    fn resolve_position(
        &self,
        uri: &DocumentUri,
        position: TextPosition,
    ) -> Result<ResolvedPosition<'_>, SnapshotError> {
        let document = self
            .document(uri)
            .ok_or_else(|| SnapshotError::UnknownDocument(uri.to_string()))?;
        let index = LineIndex::new(document.source());
        let offset = index.position_to_byte_offset(document.source(), position);
        let context = parse_str(document.source(), document.identity.as_str())
            .tree
            .map(|tree| positions::resolve(&tree, offset))
            .unwrap_or(positions::PositionContext::Unresolved);
        let scope = self.document_scope(document.identity.as_str());

        Ok(ResolvedPosition {
            document,
            index,
            offset,
            context,
            scope,
        })
    }

    /// The text a resolved position reports, or nothing where the analysis has nothing to say.
    fn hover_contents(&self, resolved: &ResolvedPosition<'_>) -> Option<String> {
        let uri = &resolved.document.uri;
        let offset = resolved.offset;
        let scope = &resolved.scope;
        match &resolved.context {
            // A declaration and a reference both report the declaration: its kind and the
            // signature `declaration_from_item` already computes for completion detail.
            positions::PositionContext::Declaration { name, .. } => {
                let declaration = scope.visible.get(name)?;
                Some(declaration_hover(declaration))
            }
            positions::PositionContext::ComponentTag { tag, .. } => {
                let declaration = scope.visible.get(tag)?;
                Some(declaration_hover(declaration))
            }
            // A name is an expression first. Looking the spelling up among the top-level
            // declarations first would answer for the wrong binding wherever a local shadows one,
            // because that map is keyed by name and knows nothing of locals; the type environment
            // reports what the name actually resolved to. The declaration is the answer only
            // where the name reaches no expression, as in an import clause.
            positions::PositionContext::Reference { name, span } => self
                .inferred_type_hover(uri, offset, *span)
                .or_else(|| scope.visible.get(name).map(declaration_hover)),
            positions::PositionContext::Expression { span } => {
                self.inferred_type_hover(uri, offset, *span)
            }
            // A property name is not an expression and has no type of its own, but the component
            // it is supplied to declares one, and that declaration is the metadata the position
            // has. An empty slot in a tag names no property, so it still reports nothing.
            positions::PositionContext::PropertyName { tag, property, .. } => {
                let property = property.as_deref()?;
                let declared = scope
                    .element(tag)?
                    .properties
                    .iter()
                    .find(|declared| declared.name == property)?;
                Some(property_hover(declared))
            }
            // A written type annotation names a type, and a name has a declaration behind it. An
            // annotation with nothing written in it yet does not.
            positions::PositionContext::TypeAnnotation { name, .. } => {
                let name = name.as_deref()?;
                scope
                    .visible
                    .get(name)
                    .map(declaration_hover)
                    .or_else(|| builtin_type_hover(name))
            }
            // The value slot of `name=` is a place a value goes. Until one is written there is
            // nothing to report, and once one is it resolves as an expression.
            positions::PositionContext::PropertyValue { .. }
            | positions::PositionContext::Unresolved => None,
        }
    }

    /// The inferred type of the innermost expression the resolved construct covers.
    ///
    /// <para>`LoweredModule::innermost_expr_at` does the locating, because the arena and the spans
    /// are its own. What is decided here is what to do with the answer: an expression the analysis
    /// reached but has no type for reports nothing, which is the conservative answer the contract
    /// permits rather than the fabricated one it rules out.</para>
    fn inferred_type_hover(
        &self,
        uri: &DocumentUri,
        offset: usize,
        within: ByteTextRange,
    ) -> Option<String> {
        let analysis = self.module_analysis(uri)?;
        let id = analysis
            .lowered_module()
            .innermost_expr_at(within, (offset as u32).into())?;

        let ty = analysis.type_env().get_expr_type(id)?;
        if is_unresolved_type(ty) {
            return None;
        }
        Some(format!("`{}`", ty))
    }

    /// Returns conservative completion items for a position in the requested document.
    pub fn completions(
        &self,
        uri: &DocumentUri,
        position: TextPosition,
    ) -> Result<CompletionList, SnapshotError> {
        let resolved = self.resolve_position(uri, position)?;
        let scope = &resolved.scope;

        // A context the resolver classified but the scope cannot fill in — an unknown element, a
        // property with no union type — offers no contextual completions, and falls back to the
        // general set exactly as an unrecognized position does.
        let items = match &resolved.context {
            positions::PositionContext::PropertyValue { tag, property, .. } => {
                // A bare value resolves against the property's declared type, so only its members
                // are valid here; lexically visible names cannot appear unbraced.
                match property_value_members(tag, property, scope) {
                    Some(members) => members
                        .into_iter()
                        .map(|member| CompletionItem {
                            label: member,
                            kind: CompletionItemKind::Member,
                            detail: None,
                        })
                        .collect(),
                    None => general_completion_items(scope),
                }
            }
            positions::PositionContext::PropertyName { tag, supplied, .. } => {
                match scope
                    .visible
                    .get(tag)
                    .filter(|declaration| declaration.kind == DocumentSymbolKind::Component)
                {
                    Some(declaration) => property_completion_items(PropertyCompletionContext {
                        properties: declaration.properties.clone(),
                        supplied: supplied.clone(),
                    }),
                    None => general_completion_items(scope),
                }
            }
            positions::PositionContext::TypeAnnotation { .. } => type_completion_items(scope),
            _ => general_completion_items(scope),
        };

        Ok(CompletionList {
            uri: resolved.document.uri.clone(),
            identity: resolved.document.identity.clone(),
            version: resolved.document.version,
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

    /// Resolves what one document can see, through the same import graph the compiler uses.
    fn document_scope(&self, identity: &str) -> DocumentScope {
        let workspace = self.workspace_declarations();

        // Only what the edited document itself can name. A declaration in a document it does not
        // import is not a completion candidate, however its name is spelled.
        let mut visible = FxHashMap::default();
        if let Some(bindings) = workspace.visible_bindings.get(identity) {
            for (name, target) in bindings {
                if let Some(declaration) = workspace.by_origin.get(target) {
                    visible.insert(
                        name.clone(),
                        Declaration {
                            name: name.clone(),
                            ..declaration.clone()
                        },
                    );
                }
            }
        }

        DocumentScope { visible, workspace }
    }

    /// The analysis of one document, as the position resolver reads it.
    ///
    /// <para>Returns `None` for a document the snapshot does not hold, and for one whose analysis
    /// produced no lowered module — a document that failed to parse has no arena to resolve a
    /// position against.</para>
    fn module_analysis(&self, uri: &DocumentUri) -> Option<ModuleAnalysis> {
        let identity = self.document(uri)?.identity.as_str().to_string();
        let workspace = self.workspace_declarations();
        workspace.artifact(&identity)?.lowered_module.as_ref()?;
        Some(ModuleAnalysis {
            workspace,
            identity,
        })
    }

    /// Analyzes the workspace once and keeps the result for the snapshot's lifetime.
    fn workspace_declarations(&self) -> Arc<WorkspaceDeclarations> {
        Arc::clone(
            self.declarations
                .get_or_init(|| Arc::new(self.build_workspace_declarations())),
        )
    }

    fn build_workspace_declarations(&self) -> WorkspaceDeclarations {
        let Ok(workspace) = self.to_workspace() else {
            return WorkspaceDeclarations::default();
        };
        let modules = analyze_workspace_modules(&workspace, &ProgramBuildContext::empty());

        let mut declarations = WorkspaceDeclarations::default();
        for module in modules {
            declarations.visible_bindings.insert(
                module.file_name.clone(),
                module
                    .prepared_bindings
                    .iter()
                    .map(|binding| {
                        (
                            binding.visible_name.as_str().to_string(),
                            (
                                binding.module_identity(&module.file_name).to_string(),
                                binding.definition_id(),
                            ),
                        )
                    })
                    .collect(),
            );

            declarations.type_namespaces.insert(
                module.file_name.clone(),
                module
                    .prepared_bindings
                    .iter()
                    .filter(|binding| binding.namespace == PreparedNamespace::Type)
                    .map(|binding| {
                        (
                            binding.visible_name.as_str().to_string(),
                            (
                                binding.module_identity(&module.file_name).to_string(),
                                binding.definition_id(),
                            ),
                        )
                    })
                    .collect(),
            );

            // The artifact is the analysis this loop was about to drop, so it is moved in rather
            // than copied, once, whether or not the module lowered. The lowered module is behind
            // an `Arc`, so keeping a handle to it across the move costs a refcount.
            let identity = module.file_name.clone();
            let lowered = module.lowered_module.clone();
            declarations.artifacts.insert(identity.clone(), module);

            let Some(lowered) = lowered else {
                continue;
            };
            let source = self
                .documents
                .iter()
                .find(|document| document.identity.as_str() == identity)
                .map(|document| document.source())
                .unwrap_or("");
            for (item_index, item) in lowered.items().iter().enumerate() {
                let origin = (identity.clone(), LocalDefinitionId::new(item_index as u32));
                declarations
                    .by_origin
                    .insert(origin.clone(), declaration_from_item(item, source, origin));
            }
        }

        declarations
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
    /// Constant union case, offered at a property value position.
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
    /// The properties this declaration accepts, as its declaring module wrote them.
    properties: Vec<PropertyDeclaration>,
    /// Union case names, for contextual completions at a value position.
    members: Vec<String>,
    /// The declaration this came from, as `(module identity, definition id)`.
    origin: DeclarationOrigin,
}

type DeclarationOrigin = (String, LocalDefinitionId);

/// One property of a component, record, or markup function.
///
/// <para>The two spellings of the type are both needed and neither derives from the other here.
/// `base_type` is what a namespace lookup takes — the declaring module's name for the type, with
/// nullability and arity stripped, so `Fit`, `Fit?`, and `Fit[]` all reach the `Fit` declaration.
/// `display_type` is what a reader is shown, where that stripping would be a lie.</para>
#[derive(Debug, Clone, PartialEq, Eq)]
struct PropertyDeclaration {
    /// The name the property is supplied under.
    name: String,
    /// The base name of the declared type, resolved in the declaring module's namespace.
    base_type: String,
    /// The declared type as it is written.
    display_type: String,
}

/// Hover content for a type the language declares rather than the workspace.
///
/// <para>A primitive has no declaration to report, but the editor still knows what it is, and the
/// same word is what type completions offer at this position. Anything else is a name the document
/// cannot resolve, which reports nothing.</para>
fn builtin_type_hover(name: &str) -> Option<String> {
    if PRIMITIVE_TYPE_COMPLETIONS.contains(&name) {
        return Some(format!("primitive type `{}`", name));
    }
    BUILTIN_TYPE_COMPLETIONS
        .contains(&name)
        .then(|| format!("built-in type `{}`", name))
}

/// Hover content for a property: the name it is supplied under and the type it accepts.
fn property_hover(property: &PropertyDeclaration) -> String {
    format!("property `{}`\n\n{}", property.name, property.display_type)
}

/// Hover content for a declaration: its kind and the signature completions already show as detail.
///
/// <para>`detail` is where the signature already lives — `component <Panel />` for a markup
/// function, `function add(count: int): int` for a plain one. Hover reuses it rather than growing
/// a second way to spell the same declaration.</para>
fn declaration_hover(declaration: &Declaration) -> String {
    let kind = declaration.kind.display_name();
    // A union's detail is the bare word `union`, and an unannotated value's is `value`. Repeating
    // the kind under itself is neither a signature nor type information, so the second line is
    // written only where it says something the first does not.
    if declaration.detail == kind {
        return format!("{} `{}`", kind, declaration.name);
    }
    format!("{} `{}`\n\n{}", kind, declaration.name, declaration.detail)
}

/// A position query resolved to the construct the cursor is on, shared by hover and completions.
struct ResolvedPosition<'a> {
    /// The document the position is in.
    document: &'a DocumentSnapshot,
    /// Line and character offsets for that document's source.
    index: LineIndex,
    /// The queried position as a byte offset into the source.
    offset: usize,
    /// What the cursor is on.
    context: positions::PositionContext,
    /// Everything the document can name.
    scope: DocumentScope,
}

/// Everything one document can see, resolved through its own import graph.
///
/// Joining a flat list of every workspace declaration by name is what made an element resolve to
/// whichever document happened to declare that spelling, alias or no alias, imported or not. Names
/// here are resolved where they are written: a tag in the document being edited against that
/// document's namespace, and a property's declared type against the namespace of the module that
/// declared the property.
#[derive(Default)]
struct DocumentScope {
    /// Declarations the edited document can name, keyed by the name it writes.
    visible: FxHashMap<String, Declaration>,
    /// The workspace analysis, shared by every document in the snapshot.
    workspace: Arc<WorkspaceDeclarations>,
}

/// Everything a document scope reads that does not depend on which document is being edited.
#[derive(Debug, Default)]
struct WorkspaceDeclarations {
    /// Every workspace declaration, keyed by where it is declared.
    by_origin: FxHashMap<DeclarationOrigin, Declaration>,
    /// The type namespace of each module, as `module identity → (visible name → origin)`.
    type_namespaces: FxHashMap<String, FxHashMap<String, DeclarationOrigin>>,
    /// Every visible binding of each module, as `module identity → (visible name, origin)`.
    visible_bindings: FxHashMap<String, Vec<(String, DeclarationOrigin)>>,
    /// The analysis artifact of each module, keyed by module identity.
    ///
    /// <para>The names and kinds above are derived from these and were all that survived. A
    /// position query needs the rest — the expression arena the spans live in and the type
    /// environment the inferred types live in — so the artifacts are kept rather than consumed and
    /// dropped. They are computed once per snapshot under the same `OnceLock`, so this changes
    /// what the snapshot holds, not how often analysis runs.</para>
    artifacts: FxHashMap<String, ModuleArtifact>,
}

impl WorkspaceDeclarations {
    /// The analysis artifact for one module identity.
    fn artifact(&self, identity: &str) -> Option<&ModuleArtifact> {
        self.artifacts.get(identity)
    }
}

/// One document's analysis, held open for the length of a position request.
///
/// <para>The artifacts live behind the snapshot's `OnceLock`, which hands out an `Arc` rather than
/// a borrow, so a resolver that wants to read them has to hold that `Arc` while it does. This is
/// the handle that holds it.</para>
struct ModuleAnalysis {
    workspace: Arc<WorkspaceDeclarations>,
    identity: String,
}

impl ModuleAnalysis {
    /// The lowered module every expression span is measured against.
    fn lowered_module(&self) -> &LoweredModule {
        self.artifact()
            .lowered_module
            .as_deref()
            .expect("module analysis is only constructed for a module that lowered")
    }

    /// The inferred type of every expression the checker reached.
    fn type_env(&self) -> &TypeEnvironment {
        &self.artifact().type_env
    }

    fn artifact(&self) -> &ModuleArtifact {
        self.workspace
            .artifact(&self.identity)
            .expect("module analysis is only constructed for a retained artifact")
    }
}

impl DocumentScope {
    /// The element or record a tag written in the edited document names.
    fn element(&self, tag: &str) -> Option<&Declaration> {
        self.visible.get(tag).filter(|declaration| {
            matches!(
                declaration.kind,
                DocumentSymbolKind::Component | DocumentSymbolKind::Record
            )
        })
    }

    /// The declaration a type name written by the module at `origin` denotes.
    fn type_in_module(&self, origin: &DeclarationOrigin, name: &str) -> Option<&Declaration> {
        let target = self.workspace.type_namespaces.get(&origin.0)?.get(name)?;
        self.workspace.by_origin.get(target)
    }

    /// Every declaration the edited document can name, in a stable order.
    fn visible_declarations(&self) -> Vec<&Declaration> {
        let mut declarations = self.visible.values().collect::<Vec<_>>();
        declarations.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        declarations
    }
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
        .enumerate()
        .map(|(item_index, item)| {
            let origin = (
                document.identity.as_str().to_string(),
                LocalDefinitionId::new(item_index as u32),
            );
            let declaration = declaration_from_item(item, document.source(), origin);
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

/// True where a type still carries something inference has not resolved.
///
/// <para>`?` and `<error>` are the two the checker spells as failures, but not the only two it
/// produces where it has nothing. An unsolved variable renders as `T0` and a bare
/// `ContextualName` renders as the name itself, which reads as a type of that name; `null` infers
/// as `T0?`, so the unresolved part need not be at the top of the type. Rendering any of them
/// would be the fabricated hover the conservative contract rules out.</para>
fn is_unresolved_type(ty: &nx_types::Type) -> bool {
    match ty {
        nx_types::Type::Unknown
        | nx_types::Type::Error
        | nx_types::Type::Variable(_)
        | nx_types::Type::ContextualName(_) => true,
        nx_types::Type::Array(inner) | nx_types::Type::Nullable(inner) => is_unresolved_type(inner),
        nx_types::Type::Function { params, ret } => {
            params.iter().any(is_unresolved_type) || is_unresolved_type(ret)
        }
        nx_types::Type::Primitive(_)
        | nx_types::Type::Named(_)
        | nx_types::Type::Union(_)
        | nx_types::Type::UnionCase(_) => false,
    }
}

fn item_span(item: &Item) -> ByteTextRange {
    match item {
        Item::Function(function) => function.span,
        Item::Value(value) => value.span,
        Item::Component(component) => component.span,
        Item::TypeAlias(alias) => alias.span,
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

fn declaration_from_item(item: &Item, source: &str, origin: DeclarationOrigin) -> Declaration {
    match item {
        Item::Function(function) => {
            // A markup function is `let <Tag ... />`, optionally behind a visibility keyword. The
            // span covers the whole declaration, so the keywords have to be skipped rather than
            // matched away.
            let declaration_text = source_text_for_range(source, function.span);
            let is_markup_function = declaration_text
                .split_whitespace()
                .find(|word| !matches!(*word, "export" | "private"))
                .is_some_and(|word| word == "let" || word.starts_with("let<"))
                && declaration_text
                    .split_once("let")
                    .is_some_and(|(_, rest)| rest.trim_start().starts_with('<'));
            Declaration {
                name: function.name.as_str().to_string(),
                kind: if is_markup_function {
                    DocumentSymbolKind::Component
                } else {
                    DocumentSymbolKind::Function
                },
                detail: if is_markup_function {
                    markup_signature(
                        function.name.as_str(),
                        function
                            .params
                            .iter()
                            .map(|param| (param.name.as_str(), &param.ty)),
                    )
                } else {
                    format!("function {}", function_signature(function))
                },
                properties: if is_markup_function {
                    function
                        .params
                        .iter()
                        .map(|param| property_declaration(param.name.as_str(), &param.ty))
                        .collect()
                } else {
                    Vec::new()
                },
                members: Vec::new(),
                origin: origin.clone(),
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
            members: Vec::new(),
            origin: origin.clone(),
        },
        Item::Component(component) => Declaration {
            name: component.name.as_str().to_string(),
            kind: DocumentSymbolKind::Component,
            detail: markup_signature(
                component.name.as_str(),
                component
                    .props
                    .iter()
                    .map(|property| (property.name.as_str(), &property.ty)),
            ),
            properties: component
                .props
                .iter()
                .map(|property| property_declaration(property.name.as_str(), &property.ty))
                .collect(),
            members: Vec::new(),
            origin: origin.clone(),
        },
        Item::TypeAlias(alias) => Declaration {
            name: alias.name.as_str().to_string(),
            kind: DocumentSymbolKind::TypeAlias,
            detail: format!("type = {}", type_ref_display(&alias.ty)),
            properties: Vec::new(),
            members: Vec::new(),
            origin: origin.clone(),
        },
        Item::Union(union_def) => Declaration {
            name: union_def.name.as_str().to_string(),
            kind: DocumentSymbolKind::Union,
            detail: "union".to_string(),
            // Only payloadless cases have a bare spelling; a payload case needs element-style
            // construction and must not be offered here.
            properties: Vec::new(),
            members: union_def
                .cases
                .iter()
                .filter(|case| case.fields.is_empty())
                .map(|case| case.name.as_str().to_string())
                .collect(),
            origin: origin.clone(),
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
                .map(|property| property_declaration(property.name.as_str(), &property.ty))
                .collect(),
            members: Vec::new(),
            origin: origin.clone(),
        },
    }
}

fn property_declaration(name: &str, ty: &TypeRef) -> PropertyDeclaration {
    PropertyDeclaration {
        name: name.to_string(),
        base_type: base_type_name(ty),
        display_type: type_ref_display(ty),
    }
}

fn source_text_for_range(source: &str, range: ByteTextRange) -> &str {
    let start: usize = range.start().into();
    let end: usize = range.end().into();
    source.get(start..end).unwrap_or_default()
}

/// A component's signature as it is written: the tag with its properties and their types.
///
/// <para>`component <Panel />` says only that a component exists. What the reader wants at a tag
/// is what the tag accepts, which is the same list completions offer one name at a time.</para>
fn markup_signature<'a>(
    name: &str,
    properties: impl Iterator<Item = (&'a str, &'a TypeRef)>,
) -> String {
    let properties = properties
        .map(|(property, ty)| format!("{}:{}", property, type_ref_display(ty)))
        .collect::<Vec<_>>()
        .join(" ");
    if properties.is_empty() {
        return format!("component <{} />", name);
    }
    format!("component <{} {} />", name, properties)
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

/// The members a bare name could resolve to in one property's value slot.
///
/// <para>Returns `None` when the element or property is unknown, or when the property's declared
/// type is not a union, because a bare name is not accepted at those sites either.</para>
fn property_value_members(
    tag: &str,
    property_name: &str,
    scope: &DocumentScope,
) -> Option<Vec<String>> {
    let element = scope.element(tag)?;

    let type_name = element
        .properties
        .iter()
        .find(|property| property.name == property_name)
        .map(|property| property.base_type.clone())?;

    // The property's type was written in the declaring module's namespace, so it is resolved
    // there. Looking it up by name among everything in the workspace is what let an unrelated
    // same-named declaration supply the members.
    let target = scope.type_in_module(&element.origin, &type_name)?;
    if !matches!(target.kind, DocumentSymbolKind::Union) || target.members.is_empty() {
        return None;
    }
    Some(target.members.clone())
}

#[derive(Debug, Clone)]
struct PropertyCompletionContext {
    properties: Vec<PropertyDeclaration>,
    supplied: FxHashSet<String>,
}

fn property_completion_items(context: PropertyCompletionContext) -> Vec<CompletionItem> {
    context
        .properties
        .into_iter()
        .filter(|property| !context.supplied.contains(&property.name))
        .map(|property| CompletionItem {
            // The detail is the property's declared type, which is what hover reports at the same
            // position. One fact, spelled once, so the two cannot drift apart.
            detail: Some(property.display_type),
            label: property.name,
            kind: CompletionItemKind::Property,
        })
        .collect()
}

fn type_completion_items(scope: &DocumentScope) -> Vec<CompletionItem> {
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

    items.extend(
        scope
            .visible_declarations()
            .into_iter()
            .filter_map(|declaration| {
                matches!(
                    declaration.kind,
                    DocumentSymbolKind::TypeAlias
                        | DocumentSymbolKind::Record
                        | DocumentSymbolKind::Action
                        | DocumentSymbolKind::Union
                        | DocumentSymbolKind::Component
                )
                .then(|| CompletionItem {
                    label: declaration.name.clone(),
                    kind: CompletionItemKind::Type,
                    detail: Some(declaration.detail.clone()),
                })
            }),
    );

    dedupe_completions(items)
}

fn general_completion_items(scope: &DocumentScope) -> Vec<CompletionItem> {
    let mut items = KEYWORD_COMPLETIONS
        .iter()
        .map(|label| CompletionItem {
            label: (*label).to_string(),
            kind: CompletionItemKind::Keyword,
            detail: None,
        })
        .collect::<Vec<_>>();

    items.extend(
        scope
            .visible_declarations()
            .into_iter()
            .map(|declaration| CompletionItem {
                label: declaration.name.clone(),
                kind: if declaration.kind == DocumentSymbolKind::Component {
                    CompletionItemKind::Component
                } else {
                    CompletionItemKind::Declaration
                },
                detail: Some(declaration.detail.clone()),
            }),
    );

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

    /// The marker fixtures write where the cursor sits.
    const CURSOR: &str = "⟨cursor⟩";

    /// Splits a marked fixture into the source the editor holds and the position of the cursor.
    ///
    /// <para>Scenarios about multi-line constructs are unreadable when the cursor is a pair of
    /// literal numbers: the reader has to count columns to know what is being asked. Writing the
    /// marker into the fixture puts the question where it can be seen.</para>
    fn position_for(source: &str, marker: &str) -> (String, TextPosition) {
        let offset = source
            .find(marker)
            .unwrap_or_else(|| panic!("fixture has no `{marker}` marker"));
        let prefix = &source[..offset];
        let line = prefix.matches('\n').count() as u32;
        let line_start = prefix.rfind('\n').map(|index| index + 1).unwrap_or(0);
        let character = prefix[line_start..].chars().count() as u32;

        let mut stripped = String::with_capacity(source.len() - marker.len());
        stripped.push_str(prefix);
        stripped.push_str(&source[offset + marker.len()..]);

        (stripped, TextPosition::new(line, character))
    }

    /// Resolves completion labels for a fixture that deliberately does not parse — the half-typed
    /// states an editor asks from, which are what this change is about.
    ///
    /// The exception to `assert_fixture_parses`, stated at each site rather than implied.
    fn labels_at_incomplete(source: &str) -> Vec<String> {
        let (source, position) = position_for(source, CURSOR);
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        completion_labels(&snapshot, "nx://tenant/form.nx", position)
    }

    /// Resolves completion labels for a marked fixture in a single-document snapshot.
    fn labels_at(source: &str) -> Vec<String> {
        let (source, position) = position_for(source, CURSOR);
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        assert_fixture_parses(&snapshot, &source);
        completion_labels(&snapshot, "nx://tenant/form.nx", position)
    }

    /// Resolves hover for a marked fixture in a single-document snapshot.
    fn hover_at(source: &str) -> Option<Hover> {
        let (source, position) = position_for(source, CURSOR);
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        assert_fixture_parses(&snapshot, &source);
        snapshot
            .hover(&DocumentUri::from("nx://tenant/form.nx"), position)
            .expect("hover")
    }

    /// Resolves hover for a fixture that deliberately does not parse — half-typed code, or a
    /// syntax error that is itself under test.
    ///
    /// The exception to `assert_fixture_parses`, stated at each site rather than implied.
    fn hover_at_incomplete(source: &str) -> Option<Hover> {
        let (source, position) = position_for(source, CURSOR);
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        snapshot
            .hover(&DocumentUri::from("nx://tenant/form.nx"), position)
            .expect("hover")
    }

    /// Fails where a fixture is not valid NX.
    ///
    /// <para>Hover declines inside a syntax error deliberately — pinned by
    /// `hover_inside_a_declaration_with_a_syntax_error_returns_no_result` — and the `None` it
    /// returns there is indistinguishable from the `None` a position hover genuinely cannot answer
    /// returns. So a fixture with a typo in it reads as a discovered gap, and the cost is
    /// asymmetric: a fixture that should hover and does not fails its assertion, while one that
    /// cannot parse looks exactly like the finding its author went looking for. Three wrong
    /// conclusions were drawn from that in this change's review cycle, one of which reached
    /// `specs/future.md`.</para>
    fn assert_fixture_parses(snapshot: &WorkspaceSnapshot, source: &str) {
        let errors: Vec<String> = snapshot
            .diagnostics()
            .expect("diagnostics")
            .into_iter()
            .flat_map(|document| document.diagnostics)
            .filter(|diagnostic| diagnostic.code.as_deref() == Some("syntax-error"))
            .map(|diagnostic| diagnostic.message)
            .collect();

        assert!(
            errors.is_empty(),
            "fixture is not valid NX: {source:?} — {errors:?}"
        );
    }

    /// Asserts two spellings of one construct offer identical completions.
    ///
    /// <para>The single-line spelling is the behavior that works today, so it is the standard the
    /// multi-line spelling is held to — the specified guarantee is that layout does not change the
    /// answer, not that some particular list comes back.</para>
    ///
    /// <para>Exempt from `assert_fixture_parses` by construction rather than by omission: a parity
    /// fixture is half-typed on both sides, because the positions where layout could change the
    /// answer are the ones mid-edit. Naming the exemption here is what `hover_at_incomplete` and
    /// `labels_at_incomplete` do at their call sites.</para>
    fn assert_same_completions(single_line: &str, multi_line: &str, marker: &str) {
        let (single_source, single_position) = position_for(single_line, marker);
        let (multi_source, multi_position) = position_for(multi_line, marker);
        let single = snapshot_for("nx://tenant/form.nx", &single_source, 1);
        let multi = snapshot_for("nx://tenant/form.nx", &multi_source, 1);

        let expected = completion_labels(&single, "nx://tenant/form.nx", single_position);
        let actual = completion_labels(&multi, "nx://tenant/form.nx", multi_position);

        assert_eq!(
            expected, actual,
            "layout changed the completions\n  single-line: {expected:?}\n   multi-line: {actual:?}"
        );
    }

    /// The completion list is the primitive set, and `void` is no longer in it.
    #[test]
    fn primitive_type_completions_are_exactly_the_canonical_names() {
        assert_eq!(
            PRIMITIVE_TYPE_COMPLETIONS,
            &["string", "int", "int32", "int64", "float32", "float64", "boolean", "object",]
        );
        for absent in ["void", "long", "double", "bool", "f64", "float"] {
            assert!(
                !PRIMITIVE_TYPE_COMPLETIONS.contains(&absent),
                "{absent} is not an NX primitive type name"
            );
        }
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
    fn diagnostics_accept_an_int_literal_at_a_float_site_and_still_reject_an_inexact_one() {
        // The editor runs the same analysis, so the notation has to be as quiet here as it is at
        // the command line — a marker under `24` would make the language look like it disagreed
        // with itself.
        let accepted = snapshot_for("nx://tenant/ui.nx", "let width: float64 = 24", 1);
        assert!(
            accepted.diagnostics().expect("diagnostics")[0]
                .diagnostics
                .is_empty(),
            "an integer literal at a float site should not be marked"
        );

        // The property binding is the site the notation exists for, so it gets its own check
        // rather than riding on the annotated `let` above.
        let property = snapshot_for(
            "nx://tenant/ui.nx",
            "external component <B v:float64 />\nlet root() = { <B v=1 /> }",
            2,
        );
        assert!(
            property.diagnostics().expect("diagnostics")[0]
                .diagnostics
                .is_empty(),
            "an integer literal at a float property binding should not be marked"
        );

        let inexact = snapshot_for(
            "nx://tenant/ui.nx",
            "let width: float64 = 9007199254740993",
            3,
        );
        assert!(
            inexact.diagnostics().expect("diagnostics")[0]
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic
                    .code
                    .as_deref()
                    .is_some_and(|code| code == "float-literal-not-exact")),
            "a literal that cannot be represented should still be marked"
        );
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

        assert_eq!(hover.contents, "function `root`\n\nfunction root()");
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

    /// `enum` was removed from the language, so it must not be offered as a declaration keyword.
    #[test]
    fn declaration_completions_do_not_offer_the_removed_enum_keyword() {
        let source = "\n";
        let snapshot = snapshot_for("nx://tenant/form.nx", source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot
            .completions(&uri, TextPosition::new(0, 0))
            .expect("completions");
        let labels = completions
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert!(labels.contains(&"type"), "got: {labels:?}");
        assert!(!labels.contains(&"enum"), "got: {labels:?}");
    }

    #[test]
    fn property_value_completions_offer_members_of_the_declared_type() {
        let (source, position) = position_for(
            "type Fit = fill | contain | cover\nlet <Img fit:Fit /> = <img />\n<Img fit=⟨cursor⟩ />\n",
            CURSOR,
        );
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        let uri = DocumentUri::from("nx://tenant/form.nx");

        let completions = snapshot.completions(&uri, position).expect("completions");
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
            "type LoadState = idle | failed { message:string }\n",
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

    fn snapshot_of(documents: &[(&str, &str)]) -> WorkspaceSnapshot {
        WorkspaceSnapshot::from_documents(
            Option::<PathBuf>::None,
            documents
                .iter()
                .map(|(uri, source)| DocumentInput::new(*uri, *source).with_version(1))
                .collect(),
        )
        .expect("snapshot")
    }

    fn completion_labels(
        snapshot: &WorkspaceSnapshot,
        uri: &str,
        position: TextPosition,
    ) -> Vec<String> {
        snapshot
            .completions(&DocumentUri::from(uri), position)
            .expect("completions")
            .items
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    /// An element reached through a selective import resolves through that import, not by finding
    /// some document that happens to declare the tag.
    #[test]
    fn member_completions_follow_a_selective_import() {
        let snapshot = snapshot_of(&[
            (
                "nx://tenant/app.nx",
                "import { Img } from \"./widgets.nx\"\n<Img fit= />\n",
            ),
            (
                "nx://tenant/widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n",
            ),
        ]);

        let labels = completion_labels(&snapshot, "nx://tenant/app.nx", TextPosition::new(1, 9));

        assert!(labels.contains(&"cover".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"fill".to_string()), "got: {labels:?}");
    }

    /// The alias is the name the element is written under, so the lookup has to go through it.
    #[test]
    fn member_completions_follow_a_wildcard_import_alias() {
        let snapshot = snapshot_of(&[
            (
                "nx://tenant/app.nx",
                "import \"./widgets.nx\" as ui\n<ui.Img fit= />\n",
            ),
            (
                "nx://tenant/widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n",
            ),
        ]);

        let labels = completion_labels(&snapshot, "nx://tenant/app.nx", TextPosition::new(1, 12));

        assert!(labels.contains(&"cover".to_string()), "got: {labels:?}");
    }

    /// A declaration the document does not import is not a completion candidate.
    #[test]
    fn completions_are_not_drawn_from_a_document_that_is_not_imported() {
        let snapshot = snapshot_of(&[
            ("nx://tenant/app.nx", "<Img fit= />\n"),
            (
                "nx://tenant/widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n",
            ),
        ]);

        let labels = completion_labels(&snapshot, "nx://tenant/app.nx", TextPosition::new(0, 9));

        assert!(!labels.contains(&"cover".to_string()), "got: {labels:?}");
        assert!(!labels.contains(&"Img".to_string()), "got: {labels:?}");
    }

    /// Two documents declaring `Fit` are two types; the members offered are the declaring
    /// module's, not whichever document the flat list reached first.
    #[test]
    fn member_completions_come_from_the_declaring_module_not_a_same_named_local_type() {
        let snapshot = snapshot_of(&[
            (
                "nx://tenant/app.nx",
                "import { Img } from \"./widgets.nx\"\ntype Fit = stretch | squish\n<Img fit= />\n",
            ),
            (
                "nx://tenant/widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n",
            ),
        ]);

        let labels = completion_labels(&snapshot, "nx://tenant/app.nx", TextPosition::new(2, 9));

        assert!(labels.contains(&"cover".to_string()), "got: {labels:?}");
        assert!(!labels.contains(&"stretch".to_string()), "got: {labels:?}");
    }

    // ---------------------------------------------------------------------------------------
    // The single-line behavior the measurement table in `proposal.md` recorded as working. These
    // are the standard the resolver replaces the line-scanning heuristics against: a regression on
    // any of them is attributable to the replacement rather than to the change as a whole.
    // ---------------------------------------------------------------------------------------

    const PANEL: &str = concat!(
        "type Mode = light | dark\n",
        "let <Panel mode:Mode title:string caption:string /> = <div />\n"
    );

    #[test]
    fn baseline_single_line_tag_offers_property_names() {
        let labels = labels_at(&format!("{PANEL}<Panel ⟨cursor⟩/>\n"));

        assert_eq!(
            labels,
            vec![
                "mode".to_string(),
                "title".to_string(),
                "caption".to_string()
            ],
            "got: {labels:?}"
        );
    }

    #[test]
    fn baseline_single_line_tag_offers_property_value_members() {
        let labels = labels_at_incomplete(&format!("{PANEL}<Panel mode=⟨cursor⟩ />\n"));

        assert_eq!(
            labels,
            vec!["light".to_string(), "dark".to_string()],
            "got: {labels:?}"
        );
    }

    #[test]
    fn baseline_single_line_tag_omits_a_supplied_property() {
        let labels = labels_at(&format!("{PANEL}<Panel mode=\"light\" ⟨cursor⟩/>\n"));

        assert!(!labels.contains(&"mode".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"title".to_string()), "got: {labels:?}");
    }

    #[test]
    fn baseline_hover_on_a_declaration_name_reports_its_kind() {
        let hover = hover_at(concat!(
            "type Mode = light | dark\n",
            "let <P⟨cursor⟩anel mode:Mode /> = <div />\n"
        ))
        .expect("hover content");

        assert!(
            hover.contents.contains("component") && hover.contents.contains("Panel"),
            "got: {}",
            hover.contents
        );
    }

    /// The retained artifacts are what a position query reads, so the snapshot has to hand back an
    /// environment that actually has types in it — not an empty one that would make every hover
    /// conservative for the wrong reason.
    ///
    /// <para>The reference is taken in a function body. A parameter interpolated into a markup
    /// body — `let &lt;Panel width:int /&gt; = &lt;div&gt;{width}&lt;/div&gt;` — lowers with a span
    /// but reaches the type environment with no entry. That is a gap in checking, not in what the
    /// snapshot retains, and closing it would mean changing an analysis crate, which this change
    /// does not do. Hover is conservative there, which the specified contract permits.</para>
    #[test]
    fn retained_analysis_carries_inferred_types_for_a_parameter_reference() {
        let snapshot = snapshot_for(
            "nx://tenant/form.nx",
            "let add(count:int) = { count + 1 }\n",
            1,
        );

        let analysis = snapshot
            .module_analysis(&DocumentUri::from("nx://tenant/form.nx"))
            .expect("retained analysis");
        let module = analysis.lowered_module();
        let types = analysis.type_env();

        let counts = module
            .exprs()
            .filter(|(_, expr)| {
                matches!(expr, nx_hir::ast::Expr::Ident(name) if name.as_str() == "count")
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        assert!(!counts.is_empty(), "the body reference should have lowered");
        assert!(
            counts.iter().any(|id| types
                .get_expr_type(*id)
                .is_some_and(|ty| ty.to_string() == "int")),
            "no `count` reference carried its declared type"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The specified scenarios. Layout must not change what a position means, hover must answer at
    // references and expressions, and neither may invent an answer where analysis has none.
    // ---------------------------------------------------------------------------------------

    /// Spec: "Multi-line opening tag offers the same property completions as a single-line one".
    #[test]
    fn multi_line_opening_tag_offers_the_same_property_completions() {
        assert_same_completions(
            &format!("{PANEL}<Panel ⟨cursor⟩/>\n"),
            &format!("{PANEL}<Panel\n  ⟨cursor⟩\n/>\n"),
            CURSOR,
        );
    }

    /// Spec: "Multi-line opening tag offers contextual member completions".
    #[test]
    fn multi_line_opening_tag_offers_contextual_member_completions() {
        assert_same_completions(
            &format!("{PANEL}<Panel mode=⟨cursor⟩ />\n"),
            &format!("{PANEL}<Panel\n  mode=⟨cursor⟩\n/>\n"),
            CURSOR,
        );

        let labels = labels_at_incomplete(&format!("{PANEL}<Panel\n  mode=⟨cursor⟩\n/>\n"));
        assert!(labels.contains(&"light".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"dark".to_string()), "got: {labels:?}");
    }

    /// Spec: "Supplied properties are recognized anywhere in the opening tag".
    #[test]
    fn a_property_supplied_on_another_line_is_not_offered_again() {
        let labels = labels_at(&format!(
            "{PANEL}<Panel\n  mode=\"light\"\n  ⟨cursor⟩\n  caption=\"c\"\n/>\n"
        ));

        assert!(!labels.contains(&"mode".to_string()), "got: {labels:?}");
        assert!(!labels.contains(&"caption".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"title".to_string()), "got: {labels:?}");
    }

    /// Spec: "Type completions are offered in a multi-line signature".
    #[test]
    fn type_completions_are_offered_in_a_multi_line_signature() {
        let labels = labels_at_incomplete(concat!(
            "type Mode = light | dark\n",
            "let <Panel\n",
            "  title:string\n",
            "  mode:⟨cursor⟩\n",
            "/> = <div />\n"
        ));

        assert!(labels.contains(&"string".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"boolean".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"Mode".to_string()), "got: {labels:?}");
        // A type position is not a declaration position, so the keyword list must be absent.
        assert!(!labels.contains(&"import".to_string()), "got: {labels:?}");
        assert!(
            !labels.contains(&"component".to_string()),
            "got: {labels:?}"
        );
        assert!(!labels.contains(&"let".to_string()), "got: {labels:?}");
    }

    /// The same requirement read the other way: a colon that is not an annotation must not make a
    /// property-name position look like a type position just because it shares the cursor's line.
    #[test]
    fn a_colon_inside_a_property_value_does_not_make_a_type_position() {
        let labels = labels_at(&format!("{PANEL}<Panel\n  title=\"a:b\" ⟨cursor⟩\n/>\n"));

        assert!(labels.contains(&"mode".to_string()), "got: {labels:?}");
        assert!(!labels.contains(&"string".to_string()), "got: {labels:?}");
    }

    /// Spec: "Hover over a reference reports the referenced declaration".
    #[test]
    fn hover_over_a_component_tag_reports_the_declaration_it_resolves_to() {
        let hover = hover_at(&format!("{PANEL}<Pa⟨cursor⟩nel />\n")).expect("hover content");

        assert!(
            hover.contents.contains("component") && hover.contents.contains("Panel"),
            "got: {}",
            hover.contents
        );
    }

    /// Spec: "Hover over an expression reports its inferred type".
    #[test]
    fn hover_over_a_parameter_reference_reports_its_inferred_type() {
        let hover =
            hover_at("let add(count:int) = { c⟨cursor⟩ount + 1 }\n").expect("hover content");

        assert!(hover.contents.contains("int"), "got: {}", hover.contents);
    }

    /// A hover fixture resolved against a multi-document snapshot.
    fn hover_in(documents: &[(&str, &str)], uri: &str, marked: &str) -> Option<Hover> {
        let (source, position) = position_for(marked, CURSOR);
        let mut all = vec![(uri, source.as_str())];
        all.extend_from_slice(documents);
        let snapshot = snapshot_of(&all);
        assert_fixture_parses(&snapshot, &source);
        snapshot
            .hover(&DocumentUri::from(uri), position)
            .expect("hover")
    }

    const WIDGETS: (&str, &str) = (
        "nx://tenant/widgets.nx",
        "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n",
    );

    /// The name being edited is not a name already supplied. Counting it would offer every
    /// property of the tag except the one the cursor is in the middle of typing.
    #[test]
    fn the_property_name_under_the_cursor_is_still_offered() {
        for spelling in [
            format!("{PANEL}<Panel mo⟨cursor⟩de=\"light\" />\n"),
            format!("{PANEL}<Panel\n  mo⟨cursor⟩de=\"light\"\n/>\n"),
        ] {
            let labels = labels_at(&spelling);

            assert!(labels.contains(&"mode".to_string()), "got: {labels:?}");
            assert!(labels.contains(&"title".to_string()), "got: {labels:?}");
        }
    }

    /// A cursor in a quoted property value is on the string, so the string's type is the answer.
    /// The element the string is written inside is not: answering with the type the tag evaluates
    /// to would be the fabricated hover the contract rules out, and it is what a lookup by offset
    /// alone reported before the resolved construct bounded it.
    #[test]
    fn hover_inside_a_string_property_value_reports_the_string_not_its_element() {
        let hover = hover_at(&format!("{PANEL}<Panel title=\"he⟨cursor⟩llo\" />\n"))
            .expect("hover content");

        assert_eq!(hover.contents, "`string`", "got: {}", hover.contents);
    }

    /// Design D6 recorded hover on a literal as out of reach, because lowering located no literal
    /// by offset. It records their spans now, so the type a literal already had reaches the reader.
    #[test]
    fn hover_over_a_literal_reports_its_type() {
        for (fixture, expected) in [
            ("let value = \"he⟨cursor⟩llo\"\n", "`string`"),
            ("let value = 4⟨cursor⟩2\n", "`int`"),
            ("let value = 1.⟨cursor⟩5\n", "`float64`"),
            ("let value = tr⟨cursor⟩ue\n", "`boolean`"),
            // The `-` and the digits are one literal, so the cursor is on it wherever it sits —
            // and however it parsed. Unbraced it is one signed-numeric-literal node; braced or
            // nested it is a prefix `-` over the digits, which lowering folds into the same
            // literal.
            ("let value = -4⟨cursor⟩2\n", "`int`"),
            ("let value = ⟨cursor⟩-42\n", "`int`"),
            ("let value = {-4⟨cursor⟩2}\n", "`int`"),
            ("let value = {-⟨cursor⟩42}\n", "`int`"),
            ("let value = {-1.⟨cursor⟩5}\n", "`float64`"),
            ("let add(x:int) = {x + -1⟨cursor⟩0}\n", "`int`"),
            ("let add(x:int) = {x + ⟨cursor⟩-10}\n", "`int`"),
        ] {
            let hover = hover_at(fixture).unwrap_or_else(|| panic!("no hover for: {fixture:?}"));

            assert_eq!(hover.contents, expected, "for: {fixture:?}");
        }
    }

    /// A list is written `{a b}`, and its items are ordinary expressions — a literal answers there,
    /// and so does a reference. Pinned because the opposite was measured from `[1, 2]`, which is
    /// not NX and so reported nothing for the reason every syntax error does.
    #[test]
    fn hover_inside_a_list_reports_the_item_it_is_on() {
        for (fixture, expected) in [
            ("let value = {1⟨cursor⟩ 2}\n", "`int`"),
            ("let value = {1 2⟨cursor⟩}\n", "`int`"),
            ("let value = {\"a⟨cursor⟩b\" \"c\"}\n", "`string`"),
            ("let value = {1 -4⟨cursor⟩2}\n", "`int`"),
            ("let ab = 1\nlet value = {a⟨cursor⟩b 2}\n", "value `ab`"),
        ] {
            let hover = hover_at(fixture).unwrap_or_else(|| panic!("no hover for: {fixture:?}"));

            assert_eq!(hover.contents, expected, "for: {fixture:?}");
        }
    }

    /// `null` is the one literal inference has no type for on its own: it infers as `T0?`, an
    /// unsolved variable. The variable's id is not a type a reader can act on, so the conservative
    /// answer is no hover rather than one naming it.
    #[test]
    fn hover_over_a_null_literal_reports_no_type() {
        for fixture in [
            "let value = nu⟨cursor⟩ll\n",
            "let value:string? = nu⟨cursor⟩ll\n",
        ] {
            let hover = hover_at(fixture);

            assert!(
                hover.is_none(),
                "for {fixture:?}, got: {:?}",
                hover.map(|hover| hover.contents)
            );
        }
    }

    /// A local shadows the top-level declaration of the same name, so the name the cursor is on is
    /// not the one a lookup by spelling finds. The type environment knows which binding it is.
    #[test]
    fn hover_on_a_shadowing_parameter_reports_the_parameter_not_the_top_level_name() {
        let hover = hover_at(concat!(
            "let count = \"top\"\n",
            "let add(count:int) = { c⟨cursor⟩ount + 1 }\n"
        ))
        .expect("hover content");

        assert!(hover.contents.contains("int"), "got: {}", hover.contents);
        assert!(!hover.contents.contains("value"), "got: {}", hover.contents);
    }

    /// The second line of a hover is a signature or a type. Repeating the kind under itself is
    /// neither, so a declaration whose detail says only what the kind already said omits it.
    #[test]
    fn hover_on_a_union_declaration_does_not_repeat_its_kind() {
        let hover = hover_at("type M⟨cursor⟩ode = light | dark\n").expect("hover content");

        assert_eq!(hover.contents, "union `Mode`", "got: {}", hover.contents);
    }

    /// Spec: the referenced declaration may be "declared elsewhere in the workspace snapshot".
    #[test]
    fn hover_over_a_selectively_imported_tag_reports_the_declaration() {
        let hover = hover_in(
            &[WIDGETS],
            "nx://tenant/app.nx",
            "import { Img } from \"./widgets.nx\"\n<I⟨cursor⟩mg />\n",
        )
        .expect("hover content");

        assert!(
            hover.contents.contains("component") && hover.contents.contains("Img"),
            "got: {}",
            hover.contents
        );
        assert!(hover.contents.contains("fit"), "got: {}", hover.contents);
    }

    /// The alias is the name the tag is written under, so hover has to go through it too.
    #[test]
    fn hover_over_a_wildcard_aliased_tag_reports_the_declaration() {
        let hover = hover_in(
            &[WIDGETS],
            "nx://tenant/app.nx",
            "import \"./widgets.nx\" as ui\n<ui.I⟨cursor⟩mg />\n",
        )
        .expect("hover content");

        assert!(
            hover.contents.contains("component") && hover.contents.contains("Img"),
            "got: {}",
            hover.contents
        );
    }

    /// Spec: "Hover over a component declaration reports its signature".
    #[test]
    fn hover_over_a_component_declaration_reports_its_properties() {
        let hover = hover_at(concat!(
            "type Mode = light | dark\n",
            "let <Pa⟨cursor⟩nel mode:Mode title:string /> = <div />\n"
        ))
        .expect("hover content");

        assert!(
            hover.contents.contains("component"),
            "got: {}",
            hover.contents
        );
        assert!(hover.contents.contains("mode"), "got: {}", hover.contents);
        assert!(hover.contents.contains("Mode"), "got: {}", hover.contents);
        assert!(hover.contents.contains("title"), "got: {}", hover.contents);
        assert!(hover.contents.contains("string"), "got: {}", hover.contents);
    }

    /// Spec: "Hover over a property name reports the property's declared type".
    #[test]
    fn hover_over_a_property_name_reports_its_declared_type() {
        for spelling in [
            format!("{PANEL}<Panel ti⟨cursor⟩tle=\"x\" />\n"),
            format!("{PANEL}<Panel\n  ti⟨cursor⟩tle=\"x\"\n/>\n"),
        ] {
            let hover = hover_at(&spelling).expect("hover content");

            assert!(
                hover.contents.contains("property"),
                "got: {}",
                hover.contents
            );
            assert!(hover.contents.contains("title"), "got: {}", hover.contents);
            assert!(hover.contents.contains("string"), "got: {}", hover.contents);
        }
    }

    /// An empty slot names no property, so there is nothing to report there.
    #[test]
    fn hover_over_an_empty_property_slot_returns_no_result() {
        assert!(
            hover_at(&format!("{PANEL}<Panel ⟨cursor⟩/>\n")).is_none(),
            "an empty property slot reported hover"
        );
    }

    /// Spec: "Hover over a type annotation reports the type it names".
    #[test]
    fn hover_over_a_type_annotation_reports_the_type_it_names() {
        let hover = hover_at(concat!(
            "type Mode = light | dark\n",
            "let <Panel mode:M⟨cursor⟩ode /> = <div />\n"
        ))
        .expect("hover content");

        assert_eq!(hover.contents, "union `Mode`", "got: {}", hover.contents);
    }

    /// A primitive has no declaration to report, but it is still what the editor knows is there.
    #[test]
    fn hover_over_a_primitive_type_annotation_reports_the_primitive() {
        let hover = hover_at("let count:i⟨cursor⟩nt = 1\n").expect("hover content");

        assert_eq!(
            hover.contents, "primitive type `int`",
            "got: {}",
            hover.contents
        );
    }

    /// An annotation is an annotation wherever it is written, so every declaration that can carry
    /// one answers at it — the resolver keys on the annotation, not on what encloses it.
    #[test]
    fn hover_over_a_type_annotation_reports_it_in_every_declaration_that_can_carry_one() {
        for (fixture, expected) in [
            (
                "action Go = {\n  query:str⟨cursor⟩ing\n}\n",
                "primitive type `string`",
            ),
            ("type R = {\n  a:in⟨cursor⟩t\n}\n", "primitive type `int`"),
            (
                "let <Panel mode:str⟨cursor⟩ing /> = <div />\n",
                "primitive type `string`",
            ),
            (
                "let add(count:in⟨cursor⟩t) = count\n",
                "primitive type `int`",
            ),
            ("let value:in⟨cursor⟩t = 1\n", "primitive type `int`"),
        ] {
            let hover = hover_at(fixture).unwrap_or_else(|| panic!("no hover for: {fixture:?}"));

            assert_eq!(hover.contents, expected, "for: {fixture:?}");
        }
    }

    /// An annotation with nothing written in it names no type.
    #[test]
    fn hover_over_an_empty_type_annotation_returns_no_result() {
        assert!(
            hover_at_incomplete("let value:⟨cursor⟩ = 1\n").is_none(),
            "an empty annotation reported hover"
        );
    }

    /// Spec: "A position inside no identifiable construct yields no contextual result".
    ///
    /// A position in ordinary whitespace is not a property, value, or type position, so it falls
    /// back to the general completion set rather than to a contextual one.
    #[test]
    fn a_position_in_no_construct_offers_no_contextual_completions() {
        let (source, position) = position_for(&format!("{PANEL}⟨cursor⟩\n"), CURSOR);
        let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
        let completions = snapshot
            .completions(&DocumentUri::from("nx://tenant/form.nx"), position)
            .expect("completions");

        assert!(
            completions.items.iter().all(|item| !matches!(
                item.kind,
                CompletionItemKind::Member | CompletionItemKind::Property
            )),
            "got: {:?}",
            completions.items
        );
    }

    /// Spec: "Hover on unknown syntax returns no result".
    #[test]
    fn hover_on_a_keyword_returns_no_result() {
        assert!(hover_at("l⟨cursor⟩et root() = 1\n").is_none());
    }

    /// Spec: "Hover on an expression with no inferred type returns no result".
    #[test]
    fn hover_inside_a_declaration_with_a_syntax_error_returns_no_result() {
        assert!(hover_at_incomplete("let broken( = { mis⟨cursor⟩sing }\n").is_none());
    }

    // Malformed states the design's first Risk enumerates. These assert only that a position query
    // answers at all — tree-sitter's recovery is what is under test, not any particular answer.

    #[test]
    fn malformed_documents_answer_position_queries_without_panicking() {
        for fixture in [
            // An unterminated tag.
            "let <Panel mode:string /> = <div />\n<Panel ⟨cursor⟩\n",
            // An empty property slot on its own line.
            "let <Panel mode:string /> = <div />\n<Panel\n  ⟨cursor⟩\n/>\n",
            // `name=` with no value.
            "let <Panel mode:string /> = <div />\n<Panel mode=⟨cursor⟩ />\n",
            // An annotation with no type.
            "let value:⟨cursor⟩\n",
            // An annotation with no type, in a signature.
            "let <Panel mode:⟨cursor⟩ /> = <div />\n",
        ] {
            let (source, position) = position_for(fixture, CURSOR);
            let snapshot = snapshot_for("nx://tenant/form.nx", &source, 1);
            let uri = DocumentUri::from("nx://tenant/form.nx");

            snapshot.completions(&uri, position).expect("completions");
            snapshot.hover(&uri, position).expect("hover");
        }
    }

    /// The parity helper has to be able to fail, or the parity tests written with it prove
    /// nothing. Two single-line spellings of the same tag agree; two genuinely different
    /// positions do not.
    #[test]
    fn parity_helper_distinguishes_agreeing_and_disagreeing_positions() {
        const DECLARATIONS: &str =
            "type Fit = fill | contain | cover\nlet <Img fit:Fit /> = <img />\n";

        assert_same_completions(
            &format!("{DECLARATIONS}<Img fit=⟨cursor⟩ />\n"),
            &format!("{DECLARATIONS}<Img  fit=⟨cursor⟩ />\n"),
            CURSOR,
        );

        let disagreement = std::panic::catch_unwind(|| {
            assert_same_completions(
                &format!("{DECLARATIONS}<Img fit=⟨cursor⟩ />\n"),
                &format!("{DECLARATIONS}⟨cursor⟩\n"),
                CURSOR,
            );
        });
        assert!(
            disagreement.is_err(),
            "a property value and a top-level position must not offer the same completions"
        );
    }

    #[test]
    fn component_property_completions_omit_supplied_properties() {
        let labels = labels_at(
            "\nlet <Card title:string subtitle:string /> = <div>{title}</div>\n<Card title=\"Hello\" ⟨cursor⟩/>\n",
        );

        assert!(!labels.contains(&"title".to_string()), "got: {labels:?}");
        assert!(labels.contains(&"subtitle".to_string()), "got: {labels:?}");
    }
}
