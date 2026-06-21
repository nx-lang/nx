//! Language Server Protocol adapter for the NX language service.

use nx_language_service::{
    CompletionItemKind as ServiceCompletionItemKind, DiagnosticSeverity, DocumentInput,
    DocumentSymbolKind as ServiceDocumentSymbolKind, DocumentUri, EditorDiagnostic, EditorRange,
    TextPosition, WorkspaceDiagnostic, WorkspaceSnapshot,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity as LspDiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    Location, MarkupContent, MarkupKind, MessageType, NumberOrString, OneOf, Position, Range,
    ServerCapabilities, SymbolKind, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{async_trait, Client, LanguageServer, LspService, Server};

const DIAGNOSTIC_DEBOUNCE: Duration = Duration::from_millis(150);

/// Starts the NX language server over stdio.
pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(NxLanguageServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Returns the MVP capability advertisement for NX.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![":".to_string(), "<".to_string()]),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    }
}

#[derive(Clone)]
struct NxLanguageServer {
    client: Client,
    state: Arc<RwLock<ServerState>>,
    debounce: Duration,
}

impl NxLanguageServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ServerState::default())),
            debounce: DIAGNOSTIC_DEBOUNCE,
        }
    }

    async fn upsert_document(&self, document: OpenDocument) {
        let uri = document.uri.clone();
        {
            let mut state = self.state.write().await;
            state.documents.insert(uri.clone(), document);
        }
        self.schedule_diagnostics(uri).await;
    }

    async fn schedule_diagnostics(&self, uri: Url) {
        let version = {
            let state = self.state.read().await;
            state.documents.get(&uri).map(|document| document.version)
        };
        let Some(version) = version else {
            return;
        };

        let server = self.clone();
        tokio::spawn(async move {
            sleep(server.debounce).await;
            server.publish_diagnostics_if_current(uri, version).await;
        });
    }

    async fn publish_diagnostics_if_current(&self, uri: Url, version: i32) {
        let (workspace_root, documents) = {
            let state = self.state.read().await;
            let Some(document) = state.documents.get(&uri) else {
                return;
            };
            if document.version != version {
                return;
            }

            (state.workspace_root.clone(), state.documents.clone())
        };

        match diagnostics_for_open_documents(workspace_root, &documents) {
            Ok(mut diagnostics) => {
                for diagnostic in diagnostics.workspace {
                    self.client
                        .log_message(
                            to_lsp_message_type(diagnostic.severity),
                            workspace_diagnostic_message(diagnostic),
                        )
                        .await;
                }

                if let Some((_, document_diagnostics, published_version)) =
                    diagnostics.documents.remove(&uri)
                {
                    self.client
                        .publish_diagnostics(uri, document_diagnostics, Some(published_version))
                        .await;
                }
            }
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("NX diagnostics failed: {}", error),
                    )
                    .await;
            }
        }
    }

    async fn snapshot(&self) -> std::result::Result<WorkspaceSnapshot, String> {
        let state = self.state.read().await;
        snapshot_for_open_documents(state.workspace_root.clone(), &state.documents)
    }
}

#[async_trait]
impl LanguageServer for NxLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        {
            let mut state = self.state.write().await;
            state.workspace_root = workspace_root_from_initialize(&params);
        }

        Ok(InitializeResult {
            capabilities: server_capabilities(),
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "NX language server initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if params.text_document.language_id != "nx" {
            return;
        }

        self.upsert_document(OpenDocument {
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: params.text_document.text,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };

        self.upsert_document(OpenDocument {
            uri: params.text_document.uri,
            version: params.text_document.version,
            text: change.text,
        })
        .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        {
            let mut state = self.state.write().await;
            state.documents.remove(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let snapshot = match self.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.client
                    .log_message(MessageType::ERROR, format!("NX symbols failed: {}", error))
                    .await;
                return Ok(None);
            }
        };
        let uri = DocumentUri::new(params.text_document.uri.to_string());
        let symbols = match snapshot.document_symbols(&uri) {
            Ok(symbols) => symbols.into_iter().map(to_lsp_document_symbol).collect(),
            Err(error) => {
                self.client
                    .log_message(MessageType::ERROR, format!("NX symbols failed: {}", error))
                    .await;
                return Ok(None);
            }
        };

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let snapshot = match self.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.client
                    .log_message(MessageType::ERROR, format!("NX hover failed: {}", error))
                    .await;
                return Ok(None);
            }
        };
        let uri = DocumentUri::new(params.text_document_position_params.text_document.uri);
        let position = to_service_position(params.text_document_position_params.position);
        let hover = match snapshot.hover(&uri, position) {
            Ok(hover) => hover,
            Err(error) => {
                self.client
                    .log_message(MessageType::ERROR, format!("NX hover failed: {}", error))
                    .await;
                return Ok(None);
            }
        };

        Ok(hover.map(to_lsp_hover))
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let snapshot = match self.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("NX completion failed: {}", error),
                    )
                    .await;
                return Ok(None);
            }
        };
        let uri = DocumentUri::new(params.text_document_position.text_document.uri);
        let position = to_service_position(params.text_document_position.position);
        let completions = match snapshot.completions(&uri, position) {
            Ok(completions) => completions,
            Err(error) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("NX completion failed: {}", error),
                    )
                    .await;
                return Ok(None);
            }
        };

        Ok(Some(CompletionResponse::Array(
            completions
                .items
                .into_iter()
                .map(to_lsp_completion)
                .collect(),
        )))
    }
}

#[derive(Clone, Debug, Default)]
struct ServerState {
    workspace_root: Option<PathBuf>,
    documents: HashMap<Url, OpenDocument>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenDocument {
    uri: Url,
    version: i32,
    text: String,
}

fn workspace_root_from_initialize(params: &InitializeParams) -> Option<PathBuf> {
    params
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        .and_then(|folder| folder.uri.to_file_path().ok())
        .or_else(|| {
            params
                .root_uri
                .as_ref()
                .and_then(|uri| uri.to_file_path().ok())
        })
        .or_else(|| deprecated_root_path(params))
}

#[allow(deprecated)]
fn deprecated_root_path(params: &InitializeParams) -> Option<PathBuf> {
    params.root_path.as_ref().map(PathBuf::from)
}

fn snapshot_for_open_documents(
    workspace_root: Option<PathBuf>,
    documents: &HashMap<Url, OpenDocument>,
) -> std::result::Result<WorkspaceSnapshot, String> {
    let inputs = documents
        .values()
        .map(|document| {
            DocumentInput::new(document.uri.to_string(), document.text.clone())
                .with_version(document.version)
        })
        .collect();
    WorkspaceSnapshot::from_documents(workspace_root, inputs).map_err(|error| error.to_string())
}

fn diagnostics_for_open_documents(
    workspace_root: Option<PathBuf>,
    documents: &HashMap<Url, OpenDocument>,
) -> std::result::Result<OpenDocumentDiagnostics, String> {
    let snapshot = snapshot_for_open_documents(workspace_root, documents)?;
    let report = snapshot
        .diagnostic_report()
        .map_err(|error| error.to_string())?;
    let mut out = HashMap::new();

    for document_diagnostics in report.documents {
        let uri = Url::parse(document_diagnostics.uri.as_str())
            .map_err(|error| format!("Invalid diagnostic URI: {}", error))?;
        let version = document_diagnostics
            .version
            .map(|version| version.value())
            .unwrap_or_default();
        let diagnostics = document_diagnostics
            .diagnostics
            .into_iter()
            .map(to_lsp_diagnostic)
            .collect();
        out.insert(uri.clone(), (uri, diagnostics, version));
    }

    Ok(OpenDocumentDiagnostics {
        documents: out,
        workspace: report.workspace,
    })
}

#[derive(Debug)]
struct OpenDocumentDiagnostics {
    documents: HashMap<Url, (Url, Vec<Diagnostic>, i32)>,
    workspace: Vec<WorkspaceDiagnostic>,
}

fn to_lsp_diagnostic(diagnostic: EditorDiagnostic) -> Diagnostic {
    Diagnostic {
        range: to_lsp_range(diagnostic.range),
        severity: Some(match diagnostic.severity {
            DiagnosticSeverity::Error => LspDiagnosticSeverity::ERROR,
            DiagnosticSeverity::Warning => LspDiagnosticSeverity::WARNING,
            DiagnosticSeverity::Info => LspDiagnosticSeverity::INFORMATION,
            DiagnosticSeverity::Hint => LspDiagnosticSeverity::HINT,
        }),
        code: diagnostic.code.map(NumberOrString::String),
        source: Some("nx".to_string()),
        message: diagnostic.message,
        related_information: related_information(diagnostic.related),
        ..Diagnostic::default()
    }
}

fn to_lsp_message_type(severity: DiagnosticSeverity) -> MessageType {
    match severity {
        DiagnosticSeverity::Error => MessageType::ERROR,
        DiagnosticSeverity::Warning => MessageType::WARNING,
        DiagnosticSeverity::Info => MessageType::INFO,
        DiagnosticSeverity::Hint => MessageType::LOG,
    }
}

fn workspace_diagnostic_message(diagnostic: WorkspaceDiagnostic) -> String {
    let code = diagnostic
        .code
        .as_deref()
        .map(|code| format!("[{}] ", code))
        .unwrap_or_default();
    let labels = if diagnostic.labels.is_empty() {
        String::new()
    } else {
        let labels = diagnostic
            .labels
            .iter()
            .map(|label| {
                label
                    .message
                    .as_ref()
                    .map(|message| format!("{} ({})", label.identity, message))
                    .unwrap_or_else(|| label.identity.clone())
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(" Labels: {}", labels)
    };

    format!(
        "NX workspace diagnostic: {}{}{}",
        code, diagnostic.message, labels
    )
}

fn related_information(
    related: Vec<nx_language_service::RelatedLocation>,
) -> Option<Vec<DiagnosticRelatedInformation>> {
    if related.is_empty() {
        return None;
    }

    let related = related
        .into_iter()
        .filter_map(|location| {
            let uri = Url::parse(location.uri.as_str()).ok()?;
            Some(DiagnosticRelatedInformation {
                location: Location {
                    uri,
                    range: to_lsp_range(location.range),
                },
                message: location.message.unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    (!related.is_empty()).then_some(related)
}

#[allow(deprecated)]
fn to_lsp_document_symbol(symbol: nx_language_service::DocumentSymbol) -> DocumentSymbol {
    DocumentSymbol {
        name: symbol.name,
        detail: None,
        kind: match symbol.kind {
            ServiceDocumentSymbolKind::Function => SymbolKind::FUNCTION,
            ServiceDocumentSymbolKind::Value => SymbolKind::VARIABLE,
            ServiceDocumentSymbolKind::TypeAlias => SymbolKind::TYPE_PARAMETER,
            ServiceDocumentSymbolKind::Record => SymbolKind::STRUCT,
            ServiceDocumentSymbolKind::Action => SymbolKind::STRUCT,
            ServiceDocumentSymbolKind::Enum => SymbolKind::ENUM,
            ServiceDocumentSymbolKind::Union => SymbolKind::ENUM,
            ServiceDocumentSymbolKind::Component => SymbolKind::CLASS,
            ServiceDocumentSymbolKind::Element => SymbolKind::OBJECT,
        },
        tags: None,
        deprecated: None,
        range: to_lsp_range(symbol.range),
        selection_range: to_lsp_range(symbol.selection_range),
        children: None,
    }
}

fn to_lsp_hover(hover: nx_language_service::Hover) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: hover.contents,
        }),
        range: Some(to_lsp_range(hover.range)),
    }
}

fn to_lsp_completion(item: nx_language_service::CompletionItem) -> CompletionItem {
    CompletionItem {
        label: item.label,
        kind: Some(match item.kind {
            ServiceCompletionItemKind::Keyword => CompletionItemKind::KEYWORD,
            ServiceCompletionItemKind::Type => CompletionItemKind::TYPE_PARAMETER,
            ServiceCompletionItemKind::Declaration => CompletionItemKind::VARIABLE,
            ServiceCompletionItemKind::Component => CompletionItemKind::CLASS,
            ServiceCompletionItemKind::Property => CompletionItemKind::PROPERTY,
        }),
        detail: item.detail,
        ..CompletionItem::default()
    }
}

fn to_lsp_range(range: EditorRange) -> Range {
    Range {
        start: Position::new(range.start.line, range.start.character),
        end: Position::new(range.end.line, range.end.character),
    }
}

fn to_service_position(position: Position) -> TextPosition {
    TextPosition::new(position.line, position.character)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use serde_json::json;
    use tokio::time::timeout;
    use tower::{Service, ServiceExt};
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::lsp_types::{
        PublishDiagnosticsParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
        TextDocumentItem, TextDocumentPositionParams, VersionedTextDocumentIdentifier,
    };

    fn open_document(uri: &str, text: &str, version: i32) -> OpenDocument {
        OpenDocument {
            uri: Url::parse(uri).expect("uri"),
            version,
            text: text.to_string(),
        }
    }

    fn one_document(uri: &str, text: &str, version: i32) -> HashMap<Url, OpenDocument> {
        let document = open_document(uri, text, version);
        HashMap::from([(document.uri.clone(), document)])
    }

    fn test_server(debounce: Duration) -> (NxLanguageServer, tower_lsp::ClientSocket) {
        let (service, socket) = test_service(debounce);

        (service.inner().clone(), socket)
    }

    fn test_service(debounce: Duration) -> (LspService<NxLanguageServer>, tower_lsp::ClientSocket) {
        LspService::new(|client| NxLanguageServer {
            client,
            state: Arc::new(RwLock::new(ServerState::default())),
            debounce,
        })
    }

    async fn initialize_service(service: &mut LspService<NxLanguageServer>) {
        let request = Request::build("initialize")
            .params(json!({ "capabilities": {} }))
            .id(1)
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(request)
            .await
            .expect("initialize call")
            .expect("initialize response");

        assert!(response.is_ok(), "{response:?}");
    }

    fn did_open_params(
        uri: Url,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> DidOpenTextDocumentParams {
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: language_id.to_string(),
                version,
                text: text.to_string(),
            },
        }
    }

    fn did_change_params(uri: Url, version: i32, text: &str) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        }
    }

    fn did_close_params(uri: Url) -> DidCloseTextDocumentParams {
        DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        }
    }

    async fn next_published_diagnostics(
        socket: &mut tower_lsp::ClientSocket,
    ) -> PublishDiagnosticsParams {
        loop {
            let request = timeout(Duration::from_secs(1), socket.next())
                .await
                .expect("publish diagnostics notification")
                .expect("client notification");

            if request.method() == "textDocument/publishDiagnostics" {
                return serde_json::from_value(
                    request
                        .params()
                        .cloned()
                        .expect("publish diagnostics params"),
                )
                .expect("publish diagnostics params");
            }
        }
    }

    fn document_position(uri: Url, line: u32, character: u32) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position::new(line, character),
        }
    }

    #[test]
    fn initialize_capabilities_advertise_mvp_surface() {
        let capabilities = server_capabilities();

        assert_eq!(
            capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
        );
        assert!(matches!(
            capabilities.document_symbol_provider,
            Some(OneOf::Left(true))
        ));
        assert!(matches!(
            capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true))
        ));
        let completion_provider = capabilities
            .completion_provider
            .expect("completion provider");
        assert_eq!(
            completion_provider.trigger_characters,
            Some(vec![":".to_string(), "<".to_string()])
        );
    }

    #[test]
    fn diagnostics_publish_and_clear_with_versions() {
        let uri = "file:///workspace/form.nx";
        let invalid = one_document(uri, "let count: string = 1", 2);
        let invalid_diagnostics =
            diagnostics_for_open_documents(Some(PathBuf::from("/workspace")), &invalid)
                .expect("diagnostics");
        let (_, diagnostics, version) = invalid_diagnostics
            .documents
            .get(&Url::parse(uri).expect("uri"))
            .expect("document diagnostics");

        assert_eq!(*version, 2);
        assert!(!diagnostics.is_empty());

        let valid = one_document(uri, "let count: string = \"one\"", 3);
        let valid_diagnostics =
            diagnostics_for_open_documents(Some(PathBuf::from("/workspace")), &valid)
                .expect("diagnostics");
        let (_, diagnostics, version) = valid_diagnostics
            .documents
            .get(&Url::parse(uri).expect("uri"))
            .expect("document diagnostics");

        assert_eq!(*version, 3);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn virtual_uri_document_is_analyzed_without_filesystem_access() {
        let uri = "nx://tenant/form.nx";
        let documents = one_document(uri, "let count: string = 1", 4);
        let diagnostics =
            diagnostics_for_open_documents(None, &documents).expect("virtual diagnostics");
        let (published_uri, document_diagnostics, version) = diagnostics
            .documents
            .get(&Url::parse(uri).expect("uri"))
            .expect("document diagnostics");

        assert_eq!(published_uri.as_str(), uri);
        assert_eq!(*version, 4);
        assert!(!document_diagnostics.is_empty());
    }

    #[test]
    fn workspace_diagnostic_messages_include_code_and_unmapped_labels() {
        let message = workspace_diagnostic_message(WorkspaceDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("workspace-identity-error".to_string()),
            message: "identity is invalid".to_string(),
            labels: vec![nx_language_service::WorkspaceDiagnosticLabel {
                identity: "other.nx".to_string(),
                message: Some("unmapped location".to_string()),
            }],
        });

        assert!(message.contains("[workspace-identity-error]"));
        assert!(message.contains("identity is invalid"));
        assert!(message.contains("other.nx (unmapped location)"));
    }

    #[test]
    fn document_symbol_hover_and_completion_adapters_use_language_service_results() {
        let uri = "nx://tenant/form.nx";
        let source = r#"
type User = { name: string }
let root() = 1
component <Card title:string subtitle:string /> = {
  <div>{title}</div>
}
<Card title="Hello" />
"#;
        let snapshot =
            snapshot_for_open_documents(None, &one_document(uri, source, 5)).expect("snapshot");
        let service_uri = DocumentUri::new(uri);

        let symbols = snapshot
            .document_symbols(&service_uri)
            .expect("symbols")
            .into_iter()
            .map(to_lsp_document_symbol)
            .collect::<Vec<_>>();
        assert!(symbols.iter().any(|symbol| symbol.name == "User"));

        let hover = snapshot
            .hover(&service_uri, TextPosition::new(2, 5))
            .expect("hover")
            .map(to_lsp_hover)
            .expect("hover content");
        assert!(matches!(hover.contents, HoverContents::Markup(_)));

        let completions = snapshot
            .completions(&service_uri, TextPosition::new(6, 20))
            .expect("completions")
            .items
            .into_iter()
            .map(to_lsp_completion)
            .collect::<Vec<_>>();
        assert!(completions
            .iter()
            .any(|completion| completion.label == "subtitle"));
    }

    #[tokio::test]
    async fn initialize_handler_advertises_mvp_surface() {
        let (server, _) = test_server(Duration::from_millis(1));

        let result = server
            .initialize(InitializeParams::default())
            .await
            .expect("initialize");

        assert_eq!(
            result.capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
        );
        assert!(result.capabilities.document_symbol_provider.is_some());
        assert!(result.capabilities.hover_provider.is_some());
        assert!(result.capabilities.completion_provider.is_some());
    }

    #[tokio::test]
    async fn handlers_publish_current_diagnostics_and_clear_on_close() {
        let (mut service, mut socket) = test_service(Duration::from_millis(1));
        initialize_service(&mut service).await;
        let server = service.inner().clone();
        let uri = Url::parse("nx://tenant/form.nx").expect("uri");

        server
            .did_open(did_open_params(
                uri.clone(),
                "nx",
                1,
                "let count: string = 1",
            ))
            .await;

        let published = next_published_diagnostics(&mut socket).await;
        assert_eq!(published.uri, uri);
        assert_eq!(published.version, Some(1));
        assert!(!published.diagnostics.is_empty());

        server.did_close(did_close_params(uri.clone())).await;

        let cleared = next_published_diagnostics(&mut socket).await;
        assert_eq!(cleared.uri, uri);
        assert_eq!(cleared.version, None);
        assert!(cleared.diagnostics.is_empty());
        assert!(!server.state.read().await.documents.contains_key(&uri));
    }

    #[tokio::test]
    async fn did_open_ignores_non_nx_documents() {
        let (server, _) = test_server(Duration::from_millis(1));
        let uri = Url::parse("file:///workspace/readme.txt").expect("uri");

        server
            .did_open(did_open_params(
                uri.clone(),
                "plaintext",
                1,
                "not an nx document",
            ))
            .await;

        assert!(!server.state.read().await.documents.contains_key(&uri));
    }

    #[tokio::test]
    async fn debounced_diagnostics_drop_stale_versions() {
        let (mut service, mut socket) = test_service(Duration::from_millis(20));
        initialize_service(&mut service).await;
        let server = service.inner().clone();
        let uri = Url::parse("nx://tenant/form.nx").expect("uri");

        server
            .did_open(did_open_params(
                uri.clone(),
                "nx",
                1,
                "let count: string = 1",
            ))
            .await;
        server
            .did_change(did_change_params(
                uri.clone(),
                2,
                "let count: string = \"one\"",
            ))
            .await;

        let published = next_published_diagnostics(&mut socket).await;
        assert_eq!(published.uri, uri);
        assert_eq!(published.version, Some(2));
        assert!(published.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn request_handlers_delegate_to_language_service() {
        let (server, _) = test_server(Duration::from_millis(1));
        let uri = Url::parse("nx://tenant/form.nx").expect("uri");
        let source = r#"
type User = { name: string }
let root() = 1
component <Card title:string subtitle:string /> = {
  <div>{title}</div>
}
<Card title="Hello" />
"#;

        server
            .did_open(did_open_params(uri.clone(), "nx", 5, source))
            .await;

        let symbols = server
            .document_symbol(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            })
            .await
            .expect("document symbol")
            .expect("document symbol response");
        let DocumentSymbolResponse::Nested(symbols) = symbols else {
            panic!("expected nested symbols");
        };
        assert!(symbols.iter().any(|symbol| symbol.name == "User"));

        let hover = server
            .hover(HoverParams {
                text_document_position_params: document_position(uri.clone(), 2, 5),
                work_done_progress_params: Default::default(),
            })
            .await
            .expect("hover")
            .expect("hover response");
        assert!(matches!(hover.contents, HoverContents::Markup(_)));

        let completions = server
            .completion(CompletionParams {
                text_document_position: document_position(uri, 6, 20),
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
                context: None,
            })
            .await
            .expect("completion")
            .expect("completion response");
        let CompletionResponse::Array(completions) = completions else {
            panic!("expected completion array");
        };
        assert!(completions
            .iter()
            .any(|completion| completion.label == "subtitle"));
    }
}
