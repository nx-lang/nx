//! WebAssembly ABI for the NX compiler, NX IR codegen and language service.
//!
//! <para>The module exchanges UTF-8 JSON with its loader: every operation takes at most one JSON
//! argument in the module's own memory and returns a pointer to a result record whose payload is
//! JSON. The loader allocates arguments through [`nx_wasm_alloc`], reads results through
//! [`NxWasmResult`], and releases them through [`nx_wasm_result_free`]; it never guesses at a
//! layout the module owns.</para>
//!
//! <para>The target is `wasm32-wasip1`, where panics abort. There is deliberately no
//! `catch_unwind`: a panic or an out-of-bounds access traps and ends the instance, and the loader
//! reports that as a crashed host rather than hiding it behind a stale instance.</para>

use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

use nx_api::{
    diagnostics_to_api_with_source_entries, load_program_artifact_from_source, LibraryRegistry,
    NxDiagnostic, NxSeverity, ProgramArtifact,
};
use nx_codegen::{emit_nx_ir, NxIrEntrypointMetadata, NxIrFormat, NxIrMetadata};
use nx_language_service::{
    DocumentInput, DocumentUri, SnapshotError, TextPosition, WorkspaceSnapshot,
};
use serde::{Deserialize, Serialize};

/// ABI version the loader checks before it makes any other call. Bump it whenever an export's
/// signature, a status code or a payload shape changes.
pub const ABI_VERSION: u32 = 1;

/// The operation succeeded; the payload is its JSON result.
pub const STATUS_OK: u32 = 0;
/// NX reported diagnostics; the payload is the SDK's JSON diagnostics array.
pub const STATUS_EVALUATION_ERROR: u32 = 1;
/// The module could not carry out the operation; the payload is a JSON string message.
pub const STATUS_INTERNAL_ERROR: u32 = 2;

/// What every operation returns a pointer to: a status and a UTF-8 JSON payload in the module's
/// memory.
#[repr(C)]
pub struct NxWasmResult {
    /// One of the `STATUS_` constants.
    pub status: u32,
    /// Start of the UTF-8 JSON payload, or null when it is empty.
    pub ptr: *mut u8,
    /// Length of the payload in bytes.
    pub len: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildRequest {
    source: String,
    file_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentRequest {
    uri: String,
    source: String,
    identity: Option<String>,
    version: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionRequest {
    uri: String,
    line: u32,
    character: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UriRequest {
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedNxIrPayload {
    json: String,
    metadata: NxIrMetadataPayload,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NxIrMetadataPayload {
    program_fingerprint: String,
    schema_version: u32,
    runtime_abi: String,
    required_features: Vec<String>,
    function_entrypoints: Vec<NxIrEntrypointMetadata>,
    component_entrypoints: Vec<NxIrEntrypointMetadata>,
}

/// The ABI version this module implements.
#[no_mangle]
pub extern "C" fn nx_wasm_abi_version() -> u32 {
    ABI_VERSION
}

/// Reserves `len` bytes in the module's memory for the loader to write an argument into.
///
/// The loader releases the buffer with [`nx_wasm_free`] and the same length.
#[no_mangle]
pub extern "C" fn nx_wasm_alloc(len: usize) -> *mut u8 {
    allocate(len)
}

/// Releases a buffer [`nx_wasm_alloc`] returned.
///
/// # Safety
/// `ptr` must come from [`nx_wasm_alloc`] with the same `len` and must not have been released yet.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_free(ptr: *mut u8, len: usize) {
    release(ptr, len);
}

/// Releases a result record and the payload it points at.
///
/// # Safety
/// `result` must be a record this module returned and must not have been released yet.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_result_free(result: *mut NxWasmResult) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);
    release(result.ptr, result.len);
}

/// Traps on purpose, so the SDK's trap-handling tests have something that ends the instance.
///
/// Built only with the `debug-trap` feature; the shipped module does not export it.
#[cfg(feature = "debug-trap")]
#[no_mangle]
pub extern "C" fn nx_wasm_trap() {
    panic!("nx_wasm_trap was called");
}

/// Builds a program artifact from `{ source, fileName }` JSON and answers with its handle.
///
/// # Safety
/// `ptr` and `len` must describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_build(ptr: *const u8, len: usize) -> *mut NxWasmResult {
    into_result(build_program(argument(ptr, len)))
}

/// Emits NX IR from the artifact `handle` names.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`].
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_nx_ir(handle: *mut ProgramArtifact) -> *mut NxWasmResult {
    into_result(program_nx_ir(&*handle))
}

/// Releases the artifact `handle` names.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_free(handle: *mut ProgramArtifact) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Analyzes a JSON array of `{ uri, source, identity?, version? }` documents and answers with the
/// snapshot's handle.
///
/// # Safety
/// `ptr` and `len` must describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_new(ptr: *const u8, len: usize) -> *mut NxWasmResult {
    into_result(new_snapshot(argument(ptr, len)))
}

/// Hover content at a `{ uri, line, character }` position, as JSON, or `null` when there is none.
///
/// # Safety
/// `handle` must be a live snapshot handle; `ptr` and `len` must describe UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_hover(
    handle: *mut WorkspaceSnapshot,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(snapshot_hover(&*handle, argument(ptr, len)))
}

/// Completion candidates at a `{ uri, line, character }` position, as JSON.
///
/// # Safety
/// `handle` must be a live snapshot handle; `ptr` and `len` must describe UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_completions(
    handle: *mut WorkspaceSnapshot,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(snapshot_completions(&*handle, argument(ptr, len)))
}

/// The diagnostic report for every document in the snapshot, as JSON.
///
/// # Safety
/// `handle` must be a live snapshot handle.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_diagnostics(
    handle: *mut WorkspaceSnapshot,
) -> *mut NxWasmResult {
    into_result(snapshot_diagnostics(&*handle))
}

/// Top-level symbols of the document a `{ uri }` argument names, as JSON.
///
/// # Safety
/// `handle` must be a live snapshot handle; `ptr` and `len` must describe UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_document_symbols(
    handle: *mut WorkspaceSnapshot,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(snapshot_document_symbols(&*handle, argument(ptr, len)))
}

/// Releases the snapshot `handle` names.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_snapshot_new`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_free(handle: *mut WorkspaceSnapshot) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// A finished operation: JSON to hand back, or a status and JSON describing the failure.
type Operation = Result<String, OperationError>;

struct OperationError {
    status: u32,
    payload: String,
}

fn build_program(argument: Result<&str, OperationError>) -> Operation {
    let request: BuildRequest = parse_request(argument?)?;
    let file_name = match request.file_name {
        Some(file_name) if !file_name.is_empty() => file_name,
        _ => "input.nx".to_string(),
    };

    let registry = LibraryRegistry::new();
    let build_context = registry.build_context();
    let program = load_program_artifact_from_source(&request.source, &file_name, &build_context)
        .map_err(evaluation_error)?;

    Ok(handle_json(Box::into_raw(Box::new(program))))
}

fn program_nx_ir(program: &ProgramArtifact) -> Operation {
    let ir =
        emit_nx_ir(program, NxIrFormat::Compact).map_err(|error| codegen_error(error, program))?;
    result_json(&GeneratedNxIrPayload {
        json: ir.json,
        metadata: ir_metadata_payload(ir.metadata),
    })
}

fn new_snapshot(argument: Result<&str, OperationError>) -> Operation {
    let documents: Vec<DocumentRequest> = parse_request(argument?)?;

    let mut inputs = Vec::with_capacity(documents.len());
    for document in documents {
        let mut input = DocumentInput::new(document.uri, document.source);
        if let Some(identity) = document.identity {
            input = input.with_identity(identity).map_err(snapshot_error)?;
        }
        if let Some(version) = document.version {
            input = input.with_version(version);
        }
        inputs.push(input);
    }

    let snapshot = WorkspaceSnapshot::from_documents(Option::<std::path::PathBuf>::None, inputs)
        .map_err(snapshot_error)?;
    Ok(handle_json(Box::into_raw(Box::new(snapshot))))
}

fn snapshot_hover(
    snapshot: &WorkspaceSnapshot,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: PositionRequest = parse_request(argument?)?;
    let hover = snapshot
        .hover(
            &DocumentUri::new(request.uri),
            TextPosition::new(request.line, request.character),
        )
        .map_err(snapshot_error)?;
    result_json(&hover)
}

fn snapshot_completions(
    snapshot: &WorkspaceSnapshot,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: PositionRequest = parse_request(argument?)?;
    let completions = snapshot
        .completions(
            &DocumentUri::new(request.uri),
            TextPosition::new(request.line, request.character),
        )
        .map_err(snapshot_error)?;
    result_json(&completions)
}

fn snapshot_diagnostics(snapshot: &WorkspaceSnapshot) -> Operation {
    let report = snapshot.diagnostic_report().map_err(snapshot_error)?;
    result_json(&report)
}

fn snapshot_document_symbols(
    snapshot: &WorkspaceSnapshot,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: UriRequest = parse_request(argument?)?;
    let symbols = snapshot
        .document_symbols(&DocumentUri::new(request.uri))
        .map_err(snapshot_error)?;
    result_json(&symbols)
}

/// Reads the loader's argument buffer as UTF-8.
///
/// # Safety
/// `ptr` and `len` must describe initialized bytes in the module's memory.
unsafe fn argument<'a>(ptr: *const u8, len: usize) -> Result<&'a str, OperationError> {
    if ptr.is_null() || len == 0 {
        return Ok("");
    }

    std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).map_err(|error| {
        internal_error(format!(
            "NX wasm argument bytes are not valid UTF-8: {error}"
        ))
    })
}

fn parse_request<T: for<'de> Deserialize<'de>>(argument: &str) -> Result<T, OperationError> {
    serde_json::from_str(argument)
        .map_err(|error| internal_error(format!("NX wasm argument is not valid JSON: {error}")))
}

fn result_json<T: Serialize>(value: &T) -> Operation {
    serde_json::to_string(value)
        .map_err(|error| internal_error(format!("Failed to serialize NX wasm result: {error}")))
}

fn handle_json<T>(handle: *mut T) -> String {
    (handle as usize).to_string()
}

fn ir_metadata_payload(metadata: NxIrMetadata) -> NxIrMetadataPayload {
    NxIrMetadataPayload {
        program_fingerprint: metadata.program_fingerprint.to_string(),
        schema_version: metadata.schema_version,
        runtime_abi: metadata.runtime_abi,
        required_features: metadata.required_features,
        function_entrypoints: metadata.function_entrypoints,
        component_entrypoints: metadata.component_entrypoints,
    }
}

fn evaluation_error(diagnostics: Vec<NxDiagnostic>) -> OperationError {
    OperationError {
        status: STATUS_EVALUATION_ERROR,
        payload: serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string()),
    }
}

fn codegen_error(error: nx_codegen::CodegenError, program: &ProgramArtifact) -> OperationError {
    let fallback_source = program
        .source_text(&program.entry_identity)
        .unwrap_or_default();
    let sources = program
        .source_entries()
        .into_iter()
        .map(|entry| (entry.identity, entry.source));
    evaluation_error(diagnostics_to_api_with_source_entries(
        &error.diagnostics,
        fallback_source,
        sources,
    ))
}

fn snapshot_error(error: SnapshotError) -> OperationError {
    evaluation_error(vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("language-snapshot-input-error".to_string()),
        message: error.to_string(),
        labels: Vec::new(),
        help: None,
        note: None,
    }])
}

fn internal_error(message: impl Into<String>) -> OperationError {
    OperationError {
        status: STATUS_INTERNAL_ERROR,
        payload: serde_json::to_string(&message.into()).unwrap_or_else(|_| "\"\"".to_string()),
    }
}

fn into_result(operation: Operation) -> *mut NxWasmResult {
    let (status, payload) = match operation {
        Ok(payload) => (STATUS_OK, payload),
        Err(error) => (error.status, error.payload),
    };

    let bytes = payload.into_bytes();
    let len = bytes.len();
    let ptr = allocate(len);
    if len > 0 {
        // SAFETY: `allocate` reserved exactly `len` bytes and nothing else refers to them yet.
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
        }
    }

    Box::into_raw(Box::new(NxWasmResult { status, ptr, len }))
}

fn allocate(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }

    let layout = match Layout::from_size_align(len, 1) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    // SAFETY: `len` is non-zero, so the layout has a non-zero size.
    unsafe { alloc(layout) }
}

/// # Safety
/// `ptr` must come from [`allocate`] with the same `len` and must not have been released yet.
unsafe fn release(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }

    if let Ok(layout) = Layout::from_size_align(len, 1) {
        dealloc(ptr, layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs an operation the way the loader does: JSON in through `nx_wasm_alloc`, a result record
    /// out, read and released.
    fn call(
        operation: unsafe extern "C" fn(*const u8, usize) -> *mut NxWasmResult,
        argument: &str,
    ) -> (u32, String) {
        let bytes = argument.as_bytes();
        let input = nx_wasm_alloc(bytes.len());
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), input, bytes.len());
            let result = operation(input, bytes.len());
            nx_wasm_free(input, bytes.len());

            let status = (*result).status;
            let payload = read_payload(result);
            nx_wasm_result_free(result);
            (status, payload)
        }
    }

    unsafe fn read_payload(result: *mut NxWasmResult) -> String {
        let record = &*result;
        if record.ptr.is_null() || record.len == 0 {
            return String::new();
        }
        String::from_utf8(std::slice::from_raw_parts(record.ptr, record.len).to_vec())
            .expect("payload is UTF-8")
    }

    fn handle_from(payload: &str) -> usize {
        payload.parse().expect("handle is a decimal pointer")
    }

    #[test]
    fn abi_version_is_the_one_the_loader_checks() {
        assert_eq!(nx_wasm_abi_version(), ABI_VERSION);
    }

    #[test]
    fn empty_allocations_are_null_and_free_is_a_no_op() {
        assert!(nx_wasm_alloc(0).is_null());
        unsafe {
            nx_wasm_free(ptr::null_mut(), 0);
            nx_wasm_result_free(ptr::null_mut());
        }
    }

    #[test]
    fn a_result_record_round_trips_its_payload() {
        let result = into_result(Ok("{\"value\":1}".to_string()));
        unsafe {
            assert_eq!((*result).status, STATUS_OK);
            assert_eq!(read_payload(result), "{\"value\":1}");
            nx_wasm_result_free(result);
        }
    }

    #[test]
    fn an_empty_payload_round_trips_as_a_null_pointer() {
        let result = into_result(Ok(String::new()));
        unsafe {
            assert_eq!((*result).len, 0);
            assert!((*result).ptr.is_null());
            assert_eq!(read_payload(result), "");
            nx_wasm_result_free(result);
        }
    }

    #[test]
    fn malformed_json_is_an_internal_error() {
        let (status, payload) = call(nx_wasm_program_build, "not json");
        assert_eq!(status, STATUS_INTERNAL_ERROR);
        let message: String = serde_json::from_str(&payload).expect("payload is a JSON string");
        assert!(message.contains("not valid JSON"), "{message}");
    }

    #[test]
    fn a_program_builds_and_emits_nx_ir() {
        let (status, payload) = call(
            nx_wasm_program_build,
            &serde_json::json!({ "source": "let root() = { 42 }", "fileName": "input.nx" })
                .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");

        let handle = handle_from(&payload) as *mut ProgramArtifact;
        unsafe {
            let result = nx_wasm_program_nx_ir(handle);
            assert_eq!((*result).status, STATUS_OK);
            let ir: serde_json::Value =
                serde_json::from_str(&read_payload(result)).expect("IR payload is JSON");
            assert!(ir["json"].is_string());
            assert!(ir["metadata"]["schemaVersion"].is_number());
            nx_wasm_result_free(result);
            nx_wasm_program_free(handle);
        }
    }

    #[test]
    fn a_build_failure_answers_with_diagnostics() {
        let (status, payload) = call(
            nx_wasm_program_build,
            &serde_json::json!({ "source": "let broken(): int = { \"oops\" }", "fileName": "input.nx" })
                .to_string(),
        );
        assert_eq!(status, STATUS_EVALUATION_ERROR);
        let diagnostics: Vec<serde_json::Value> =
            serde_json::from_str(&payload).expect("payload is a diagnostics array");
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0]["severity"], "error");
    }

    #[test]
    fn a_snapshot_answers_queries_and_reports_an_unparseable_uri() {
        let (status, payload) = call(
            nx_wasm_snapshot_new,
            &serde_json::json!([{ "uri": "nx://demo/input.nx", "source": "let root() = { 42 }" }])
                .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");

        let handle = handle_from(&payload) as *mut WorkspaceSnapshot;
        unsafe {
            let result = nx_wasm_snapshot_diagnostics(handle);
            assert_eq!((*result).status, STATUS_OK);
            assert!(read_payload(result).starts_with('{'));
            nx_wasm_result_free(result);
            nx_wasm_snapshot_free(handle);
        }

        let (status, payload) = call(
            nx_wasm_snapshot_new,
            &serde_json::json!([{ "uri": ":::", "source": "" }]).to_string(),
        );
        assert_eq!(status, STATUS_EVALUATION_ERROR, "{payload}");
        assert!(
            payload.contains("language-snapshot-input-error"),
            "{payload}"
        );
    }
}
