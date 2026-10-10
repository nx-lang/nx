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
    build_workspace_program_artifact, diagnostics_to_api_with_source_entries,
    eval_program_artifact_nx_text_with_limits, load_program_artifact_from_source,
    program_artifact_function_schema_json, program_artifact_type_schema_json, validate_workspace,
    LibraryRegistry, NxDiagnostic, NxLibraryModule, NxLibrarySource, NxSeverity, NxWorkspace,
    NxWorkspaceModule, ProgramArtifact, ProgramBuildContext, ResourceLimits, SchemaQueryError,
};
use nx_codegen::{emit_nx_ir, write_nx_ir_bundle, NxIrEmitOptions};
use nx_ir::explain_nx_ir_image;
use nx_language_service::{
    DocumentInput, DocumentUri, SnapshotError, TextPosition, WorkspaceSnapshot,
};
use serde::{Deserialize, Serialize};

/// ABI version the loader checks before it makes any other call. Bump it whenever an export's
/// signature, a status code or a payload shape changes.
pub const ABI_VERSION: u32 = 7;

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
struct WorkspaceBuildRequest {
    modules: Vec<WorkspaceModuleRequest>,
    entry: String,
    /// Replaces the build context's implicit imports when present, an empty list included.
    #[serde(default)]
    implicit_imports: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceModuleRequest {
    identity: String,
    source: String,
    version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceValidateRequest {
    modules: Vec<WorkspaceModuleRequest>,
    /// Replaces the build context's implicit imports when present, an empty list included.
    #[serde(default)]
    implicit_imports: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibrariesRequest {
    libraries: Vec<LibraryRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryRequest {
    root: String,
    version: Option<String>,
    modules: Vec<LibraryModuleRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryModuleRequest {
    identity: String,
    source: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildContextRequest {
    visible_roots: Option<Vec<String>>,
    #[serde(default)]
    implicit_imports: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRequest {
    documents: Vec<DocumentRequest>,
    #[serde(default)]
    implicit_imports: Vec<String>,
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

/// Builds a program artifact from `{ modules: [{ identity, source, version? }], entry,
/// implicitImports }` JSON against the build context `context` names, or an empty one when it is
/// null, and answers with its handle. Implicit imports the request names replace the context's.
///
/// # Safety
/// `context` must be null or a live handle from [`nx_wasm_build_context_new`]; `ptr` and `len` must
/// describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_workspace_build(
    context: *mut ProgramBuildContext,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(build_workspace(context.as_ref(), argument(ptr, len)))
}

/// Validates `{ modules: [{ identity, source, version? }], implicitImports }` JSON against the
/// build context `context` names, or an empty one when it is null, and answers with every
/// diagnostic — errors, warnings, info and hints — as a JSON array. NX diagnostics, including a
/// workspace the SDK cannot form, are the answer rather than a failure.
///
/// # Safety
/// `context` must be null or a live handle from [`nx_wasm_build_context_new`]; `ptr` and `len` must
/// describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_workspace_validate(
    context: *mut ProgramBuildContext,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(validate(context.as_ref(), argument(ptr, len)))
}

/// Creates an empty library registry and answers with its handle.
#[no_mangle]
pub extern "C" fn nx_wasm_registry_new() -> *mut NxWasmResult {
    into_result(Ok(handle_json(Box::into_raw(Box::new(
        LibraryRegistry::new(),
    )))))
}

/// Loads `{ libraries: [{ root, version?, modules: [{ identity, source }] }] }` JSON into the
/// registry `handle` names, in dependency order, and answers with the libraries' own warnings, info
/// and hints as a JSON diagnostic array, which workspace validation and builds leave out. A library
/// that cannot be loaded answers with diagnostics; libraries of the request loaded before it stay
/// loaded.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_registry_new`]; `ptr` and `len` must describe
/// UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_registry_load(
    handle: *mut LibraryRegistry,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(load_libraries(&*handle, argument(ptr, len)))
}

/// Releases the registry `handle` names. Build contexts created from it stay usable: each holds
/// the libraries it can see.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_registry_new`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_registry_free(handle: *mut LibraryRegistry) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Creates a build context over the registry `registry` names from `{ visibleRoots?,
/// implicitImports }` JSON — every loaded library visible when `visibleRoots` is omitted — and
/// answers with its handle.
///
/// # Safety
/// `registry` must be a live handle from [`nx_wasm_registry_new`]; `ptr` and `len` must describe
/// UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_build_context_new(
    registry: *mut LibraryRegistry,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(new_build_context(&*registry, argument(ptr, len)))
}

/// Releases the build context `handle` names.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_build_context_new`] and must not be used
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_build_context_free(handle: *mut ProgramBuildContext) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Emits NX IR from the artifact `handle` names, for the modules the `{ modules, debug }` JSON
/// options select; the payload is an NX IR bundle: a `u32` header length, a JSON header
/// `[{ identity, metadata, offset, length }]`, padding to four bytes, then the images.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] or [`nx_wasm_workspace_build`];
/// `ptr` and `len` must describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_nx_ir(
    handle: *mut ProgramArtifact,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(program_nx_ir(&*handle, argument(ptr, len)))
}

/// Evaluates the `root` of the artifact `handle` names and answers with its NX text and the nodes
/// that annotate it, `{ text, nodes }`: the text `nxlang run` prints. A missing `root`, a runtime
/// error or a value with no NX spelling answers with diagnostics.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] or [`nx_wasm_workspace_build`].
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_evaluate_nx(
    handle: *mut ProgramArtifact,
) -> *mut NxWasmResult {
    into_result(program_evaluate_nx(&*handle))
}

/// Explains an NX IR image as text with every table index resolved; the payload is a JSON string.
/// A malformed or unsupported image answers with diagnostics rather than trapping.
///
/// # Safety
/// `ptr` and `len` must describe bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_ir_explain(ptr: *const u8, len: usize) -> *mut NxWasmResult {
    into_result(explain_ir(bytes_argument(ptr, len)))
}

/// Answers with the diagnostics the build of the artifact `handle` names reported without failing
/// it — warnings, info and hints — as a JSON array.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] or [`nx_wasm_workspace_build`].
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_diagnostics(
    handle: *mut ProgramArtifact,
) -> *mut NxWasmResult {
    into_result(program_diagnostics(&*handle))
}

/// Answers with the schema of a function of the artifact `handle` names, from `{ reference: {
/// module?, name }, options?: { hostSuppliedTypes } }` JSON: its arguments' and result's JSON
/// Schema, its documentation and its parameters. A type with no JSON form is part of the answer;
/// a reference that names no function answers with diagnostics.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] or [`nx_wasm_workspace_build`];
/// `ptr` and `len` must describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_function_schema(
    handle: *mut ProgramArtifact,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(program_function_schema(&*handle, argument(ptr, len)))
}

/// Answers with the schema of a declared type of the artifact `handle` names, from `{ reference: {
/// module?, name }, options?: { direction } }` JSON. A type with no JSON form is part of the
/// answer; a reference that names no type answers with diagnostics.
///
/// # Safety
/// `handle` must be a live handle from [`nx_wasm_program_build`] or [`nx_wasm_workspace_build`];
/// `ptr` and `len` must describe UTF-8 bytes in the module's memory.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_program_type_schema(
    handle: *mut ProgramArtifact,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(program_type_schema(&*handle, argument(ptr, len)))
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

/// Analyzes `{ documents: [{ uri, source, identity?, version? }], implicitImports }` JSON and
/// answers with the snapshot's handle.
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

/// The source tree of the document a `{ uri }` argument names, as JSON. Unstable.
///
/// # Safety
/// `handle` must be a live snapshot handle; `ptr` and `len` must describe UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn nx_wasm_snapshot_source_tree(
    handle: *mut WorkspaceSnapshot,
    ptr: *const u8,
    len: usize,
) -> *mut NxWasmResult {
    into_result(snapshot_source_tree(&*handle, argument(ptr, len)))
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

/// A finished operation: the payload to hand back, JSON unless the export says otherwise, or a
/// status and JSON describing the failure.
type Operation = Result<Vec<u8>, OperationError>;

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

fn build_workspace(
    context: Option<&ProgramBuildContext>,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: WorkspaceBuildRequest = parse_request(argument?)?;
    let workspace = workspace_from_request(request.modules)?;
    let build_context = workspace_build_context(context, request.implicit_imports);
    let program = build_workspace_program_artifact(&workspace, &request.entry, &build_context)
        .map_err(evaluation_error)?;

    Ok(handle_json(Box::into_raw(Box::new(program))))
}

fn validate(
    context: Option<&ProgramBuildContext>,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: WorkspaceValidateRequest = parse_request(argument?)?;
    let diagnostics = match workspace_from_request(request.modules) {
        Ok(workspace) => validate_workspace(
            &workspace,
            &workspace_build_context(context, request.implicit_imports),
        ),
        Err(error) => serde_json::from_str(&error.payload).unwrap_or_default(),
    };
    result_json(&diagnostics)
}

fn workspace_from_request(
    modules: Vec<WorkspaceModuleRequest>,
) -> Result<NxWorkspace, OperationError> {
    let mut workspace_modules = Vec::with_capacity(modules.len());
    for module in modules {
        let workspace_module = NxWorkspaceModule::from_source(module.identity, module.source)
            .map_err(|error| workspace_input_error(error.to_string()))?;
        workspace_modules.push(match module.version {
            Some(version) => workspace_module.with_version(version),
            None => workspace_module,
        });
    }
    NxWorkspace::new(workspace_modules).map_err(|error| workspace_input_error(error.to_string()))
}

/// The context a workspace call runs against: the caller's, or an empty one, naming the implicit
/// imports the request names when it names a list, even an empty one, so a caller can build a
/// module without the context's implicit imports.
fn workspace_build_context(
    context: Option<&ProgramBuildContext>,
    implicit_imports: Option<Vec<String>>,
) -> ProgramBuildContext {
    let context = context.cloned().unwrap_or_else(ProgramBuildContext::empty);
    match implicit_imports {
        Some(implicit_imports) => context.with_implicit_imports(implicit_imports),
        None => context,
    }
}

fn load_libraries(registry: &LibraryRegistry, argument: Result<&str, OperationError>) -> Operation {
    let request: LibrariesRequest = parse_request(argument?)?;
    let libraries = request
        .libraries
        .into_iter()
        .map(|library| {
            let source = NxLibrarySource::new(
                library.root,
                library
                    .modules
                    .into_iter()
                    .map(|module| NxLibraryModule::new(module.identity, module.source))
                    .collect(),
            );
            match library.version {
                Some(version) => source.with_version(version),
                None => source,
            }
        })
        .collect::<Vec<_>>();
    let loaded = registry
        .load_libraries_from_sources(&libraries)
        .map_err(evaluation_error)?;
    // Workspace validation and builds leave a library's own warnings out, so the load is where the
    // host reads them: each library's once, in the order the libraries were given.
    let mut reported = std::collections::HashSet::new();
    let diagnostics = loaded
        .iter()
        .filter(|library| reported.insert(library.root_path.clone()))
        .flat_map(|library| library.api_diagnostics())
        .collect::<Vec<_>>();
    result_json(&diagnostics)
}

fn new_build_context(
    registry: &LibraryRegistry,
    argument: Result<&str, OperationError>,
) -> Operation {
    let argument = argument?;
    let request: BuildContextRequest = if argument.trim().is_empty() {
        BuildContextRequest {
            visible_roots: None,
            implicit_imports: Vec::new(),
        }
    } else {
        parse_request(argument)?
    };
    let context = match request.visible_roots {
        None => registry.build_context(),
        Some(roots) => registry
            .build_context_with_visible_roots(roots)
            .map_err(|error| workspace_input_error(error.to_string()))?,
    };
    let context = if request.implicit_imports.is_empty() {
        context
    } else {
        context.with_implicit_imports(request.implicit_imports)
    };
    Ok(handle_json(Box::into_raw(Box::new(context))))
}

fn program_nx_ir(program: &ProgramArtifact, argument: Result<&str, OperationError>) -> Operation {
    let options = NxIrEmitOptions::from_json(argument?)
        .map_err(|error| internal_error(format!("NX IR emit options are not valid: {error}")))?;
    let artifacts = emit_nx_ir(program, &options).map_err(|error| codegen_error(error, program))?;
    write_nx_ir_bundle(&artifacts).map_err(internal_error)
}

/// How deep NX calls may nest when the module evaluates.
///
/// <para>The interpreter's own default, 1,000, is more than a browser can run. The engine keeps the
/// module's frames on its native stack, which the module cannot size, and in a Chromium dedicated
/// worker that stack ran out 325 to 517 NX calls deep depending on what each call does (measured
/// through the playground: a `for` in the body 325, `n + f(n - 1)` 371, an element function 411, a
/// plain countdown 517). A limit below all of them, with room for heavier calls and for engines with
/// smaller stacks, makes runaway recursion the interpreter's readable error instead of a trap.</para>
pub const MAX_RECURSION_DEPTH: usize = 200;

fn program_evaluate_nx(program: &ProgramArtifact) -> Operation {
    let limits = ResourceLimits {
        max_recursion_depth: MAX_RECURSION_DEPTH,
        ..ResourceLimits::default()
    };
    let text =
        eval_program_artifact_nx_text_with_limits(program, limits).map_err(evaluation_error)?;
    result_json(&text)
}

fn program_function_schema(
    program: &ProgramArtifact,
    argument: Result<&str, OperationError>,
) -> Operation {
    program_artifact_function_schema_json(program, argument?)
        .map(String::into_bytes)
        .map_err(schema_query_error)
}

fn program_type_schema(
    program: &ProgramArtifact,
    argument: Result<&str, OperationError>,
) -> Operation {
    program_artifact_type_schema_json(program, argument?)
        .map(String::into_bytes)
        .map_err(schema_query_error)
}

fn schema_query_error(error: SchemaQueryError) -> OperationError {
    match error {
        SchemaQueryError::Request(message) => internal_error(message),
        SchemaQueryError::Declaration(diagnostics) => evaluation_error(diagnostics),
    }
}

fn explain_ir(argument: Result<&[u8], OperationError>) -> Operation {
    let text = explain_nx_ir_image(argument?).map_err(|error| {
        evaluation_error(vec![NxDiagnostic {
            severity: NxSeverity::Error,
            code: Some(
                match error {
                    nx_ir::ExplainError::SchemaVersion { .. } => "nx-ir-schema-version",
                    nx_ir::ExplainError::Malformed(_) => "nx-ir-malformed",
                }
                .to_string(),
            ),
            message: error.to_string(),
            labels: Vec::new(),
            help: None,
            note: None,
        }])
    })?;
    result_json(&text)
}

fn new_snapshot(argument: Result<&str, OperationError>) -> Operation {
    let request: SnapshotRequest = parse_request(argument?)?;

    let mut inputs = Vec::with_capacity(request.documents.len());
    for document in request.documents {
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
        .map_err(snapshot_error)?
        .with_build_context(
            ProgramBuildContext::empty().with_implicit_imports(request.implicit_imports),
        );
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

fn snapshot_source_tree(
    snapshot: &WorkspaceSnapshot,
    argument: Result<&str, OperationError>,
) -> Operation {
    let request: UriRequest = parse_request(argument?)?;
    let tree = snapshot
        .source_tree(&DocumentUri::new(request.uri))
        .map_err(snapshot_error)?;
    result_json(&tree)
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

/// The bytes of an argument that is not text.
///
/// # Safety
/// `ptr` and `len` must describe bytes in the module's memory.
unsafe fn bytes_argument<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], OperationError> {
    if ptr.is_null() || len == 0 {
        return Ok(&[]);
    }
    Ok(std::slice::from_raw_parts(ptr, len))
}

fn parse_request<T: for<'de> Deserialize<'de>>(argument: &str) -> Result<T, OperationError> {
    serde_json::from_str(argument)
        .map_err(|error| internal_error(format!("NX wasm argument is not valid JSON: {error}")))
}

fn result_json<T: Serialize>(value: &T) -> Operation {
    serde_json::to_vec(value)
        .map_err(|error| internal_error(format!("Failed to serialize NX wasm result: {error}")))
}

fn handle_json<T>(handle: *mut T) -> Vec<u8> {
    (handle as usize).to_string().into_bytes()
}

fn evaluation_error(diagnostics: Vec<NxDiagnostic>) -> OperationError {
    OperationError {
        status: STATUS_EVALUATION_ERROR,
        payload: serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string()),
    }
}

fn program_diagnostics(program: &ProgramArtifact) -> Operation {
    result_json(&program_diagnostics_to_api(program, &program.diagnostics))
}

fn codegen_error(error: nx_codegen::CodegenError, program: &ProgramArtifact) -> OperationError {
    evaluation_error(program_diagnostics_to_api(program, &error.diagnostics))
}

/// `diagnostics`, reported against `program`, with positions counted in the module each names.
fn program_diagnostics_to_api(
    program: &ProgramArtifact,
    diagnostics: &[nx_diagnostics::Diagnostic],
) -> Vec<NxDiagnostic> {
    let fallback_source = program
        .source_text(&program.entry_identity)
        .unwrap_or_default();
    let sources = program
        .source_entries()
        .into_iter()
        .map(|entry| (entry.identity, entry.source));
    diagnostics_to_api_with_source_entries(diagnostics, fallback_source, sources)
}

fn workspace_input_error(message: String) -> OperationError {
    evaluation_error(vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("workspace-input-error".to_string()),
        message,
        labels: Vec::new(),
        help: None,
        note: None,
    }])
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
    let (status, bytes) = match operation {
        Ok(payload) => (STATUS_OK, payload),
        Err(error) => (error.status, error.payload.into_bytes()),
    };

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
    use nx_codegen::read_nx_ir_bundle;
    use nx_ir::NxIrImage;

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

    unsafe fn read_payload_bytes(result: *mut NxWasmResult) -> Vec<u8> {
        let record = &*result;
        if record.ptr.is_null() || record.len == 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(record.ptr, record.len).to_vec()
    }

    unsafe fn read_payload(result: *mut NxWasmResult) -> String {
        String::from_utf8(read_payload_bytes(result)).expect("payload is UTF-8")
    }

    fn handle_from(payload: &str) -> usize {
        payload.parse().expect("handle is a decimal pointer")
    }

    /// Runs an operation on a handle the way the loader does.
    fn call_on<T>(
        handle: *mut T,
        operation: unsafe extern "C" fn(*mut T, *const u8, usize) -> *mut NxWasmResult,
        argument: &str,
    ) -> (u32, String) {
        let bytes = argument.as_bytes();
        let input = nx_wasm_alloc(bytes.len());
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), input, bytes.len());
            let result = operation(handle, input, bytes.len());
            nx_wasm_free(input, bytes.len());

            let status = (*result).status;
            let payload = read_payload(result);
            nx_wasm_result_free(result);
            (status, payload)
        }
    }

    fn take_handle(result: *mut NxWasmResult) -> usize {
        unsafe {
            assert_eq!((*result).status, STATUS_OK);
            let payload = read_payload(result);
            nx_wasm_result_free(result);
            handle_from(&payload)
        }
    }

    /// Every image of the program `handle` names, keyed by identity.
    fn all_images(handle: *mut ProgramArtifact) -> Vec<(String, Vec<u8>)> {
        let options = r#"{"modules":[]}"#;
        let bytes = options.as_bytes();
        unsafe {
            let input = nx_wasm_alloc(bytes.len());
            ptr::copy_nonoverlapping(bytes.as_ptr(), input, bytes.len());
            let result = nx_wasm_program_nx_ir(handle, input, bytes.len());
            nx_wasm_free(input, bytes.len());
            assert_eq!((*result).status, STATUS_OK);
            let artifacts =
                read_nx_ir_bundle(&read_payload_bytes(result)).expect("IR payload is a bundle");
            nx_wasm_result_free(result);
            artifacts
                .into_iter()
                .map(|artifact| (artifact.identity, artifact.bytes))
                .collect()
        }
    }

    fn libraries_request() -> String {
        serde_json::json!({
            "libraries": [
                {
                    "root": "libraries/chat-link",
                    "modules": [{
                        "identity": "ChatLinkConfig.nx",
                        "source": "import \"../question-flow\"\nexport type ChatLinkConfig = { title:string questionFlow:QuestionFlow }"
                    }]
                },
                {
                    "root": "libraries/question-flow",
                    "version": "3",
                    "modules": [
                        { "identity": "Step.nx", "source": "export type Step = { id:string }" },
                        { "identity": "QuestionFlow.nx", "source": "export type QuestionFlow = { firstStep:Step }" }
                    ]
                }
            ]
        })
        .to_string()
    }

    fn tenant_request(title: &str) -> String {
        serde_json::json!({
            "modules": [{
                "identity": "chat-link.nx",
                "source": format!("let root() = <ChatLinkConfig title=\"{title}\" questionFlow={{<QuestionFlow firstStep={{<Step id=\"a\" />}} />}} />")
            }],
            "entry": "chat-link.nx"
        })
        .to_string()
    }

    #[test]
    fn libraries_load_once_and_every_build_in_a_context_reuses_them() {
        let registry = take_handle(nx_wasm_registry_new()) as *mut LibraryRegistry;
        let (status, payload) = call_on(registry, nx_wasm_registry_load, &libraries_request());
        assert_eq!(status, STATUS_OK, "{payload}");
        // The libraries have no warnings of their own to report.
        let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&payload).unwrap();
        assert!(diagnostics.is_empty(), "{payload}");

        let (status, payload) = call_on(
            registry,
            nx_wasm_build_context_new,
            r#"{"implicitImports":["libraries/chat-link","libraries/question-flow"]}"#,
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let context = handle_from(&payload) as *mut ProgramBuildContext;
        // The context holds what it can see, so it outlives the registry's handle.
        unsafe { nx_wasm_registry_free(registry) };

        let (status, payload) = call_on(context, nx_wasm_workspace_build, &tenant_request("One"));
        assert_eq!(status, STATUS_OK, "{payload}");
        let first = handle_from(&payload) as *mut ProgramArtifact;
        let (status, payload) = call_on(context, nx_wasm_workspace_build, &tenant_request("Two"));
        assert_eq!(status, STATUS_OK, "{payload}");
        let second = handle_from(&payload) as *mut ProgramArtifact;

        unsafe {
            // Both builds read the one analyzed snapshot of each library rather than analyzing it.
            for library in &(*first).libraries {
                assert!(
                    (*second)
                        .libraries
                        .iter()
                        .any(|other| std::sync::Arc::ptr_eq(library, other)),
                    "{} was analyzed again",
                    library.root_path.display()
                );
            }

            let first_images = all_images(first);
            let second_images = all_images(second);
            let library_images = |images: &[(String, Vec<u8>)]| {
                images
                    .iter()
                    .filter(|(identity, _)| identity.starts_with("libraries/"))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            assert_eq!(library_images(&first_images).len(), 3);
            assert_eq!(
                library_images(&first_images),
                library_images(&second_images)
            );
            let step = first_images
                .iter()
                .find(|(identity, _)| identity == "libraries/question-flow/Step.nx")
                .expect("the step module's image");
            let image = NxIrImage::open(&step.1).expect("a valid image");
            assert_eq!(image.modules().next().unwrap().version, "3");

            nx_wasm_program_free(first);
            nx_wasm_program_free(second);
            nx_wasm_build_context_free(context);
        }
    }

    #[test]
    fn a_library_that_cannot_load_answers_with_diagnostics() {
        let registry = take_handle(nx_wasm_registry_new()) as *mut LibraryRegistry;
        let (status, payload) = call_on(
            registry,
            nx_wasm_registry_load,
            &serde_json::json!({
                "libraries": [{
                    "root": "libraries/chat-link",
                    "modules": [{ "identity": "ChatLinkConfig.nx", "source": "import \"../question-flow\"" }]
                }]
            })
            .to_string(),
        );
        assert_eq!(status, STATUS_EVALUATION_ERROR, "{payload}");
        assert!(payload.contains("library-dependency-missing"), "{payload}");
        unsafe { nx_wasm_registry_free(registry) };
    }

    #[test]
    fn validation_answers_with_every_diagnostic_as_data() {
        let registry = take_handle(nx_wasm_registry_new()) as *mut LibraryRegistry;
        let (status, payload) = call_on(registry, nx_wasm_registry_load, &libraries_request());
        assert_eq!(status, STATUS_OK, "{payload}");
        let context = take_handle(unsafe { nx_wasm_build_context_new(registry, ptr::null(), 0) })
            as *mut ProgramBuildContext;

        let (status, payload) = call_on(
            context,
            nx_wasm_workspace_validate,
            &serde_json::json!({
                "modules": [{ "identity": "chat-link.nx", "source": "let root() = <ChatLinkConfig title={1} />" }],
                "implicitImports": ["libraries/chat-link", "libraries/question-flow"]
            })
            .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&payload).unwrap();
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0]["severity"], "error");
        assert_eq!(diagnostics[0]["labels"][0]["file"], "chat-link.nx");

        let (status, payload) = call_on(
            context,
            nx_wasm_workspace_validate,
            &serde_json::json!({
                "modules": [
                    { "identity": "a.nx", "source": "let root() = 1" },
                    { "identity": "a.nx", "source": "let root() = 2" }
                ]
            })
            .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        assert!(payload.contains("workspace-input-error"), "{payload}");

        unsafe {
            nx_wasm_build_context_free(context);
            nx_wasm_registry_free(registry);
        }
    }

    #[test]
    fn an_empty_implicit_import_list_replaces_the_contexts_own() {
        let registry = take_handle(nx_wasm_registry_new()) as *mut LibraryRegistry;
        let (status, payload) = call_on(registry, nx_wasm_registry_load, &libraries_request());
        assert_eq!(status, STATUS_OK, "{payload}");
        let (status, payload) = call_on(
            registry,
            nx_wasm_build_context_new,
            r#"{"implicitImports":["libraries/question-flow"]}"#,
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let context = handle_from(&payload) as *mut ProgramBuildContext;
        let request = |implicit_imports: Option<&[&str]>| {
            let mut request = serde_json::json!({
                "modules": [{ "identity": "step.nx", "source": "let root() = <Step id={1} />" }],
                "entry": "step.nx"
            });
            if let Some(implicit_imports) = implicit_imports {
                request["implicitImports"] = serde_json::json!(implicit_imports);
            }
            request.to_string()
        };

        // Omitted, the context's implicit imports apply: `Step` is the library's record, whose
        // `id` is a string.
        let (status, payload) = call_on(context, nx_wasm_workspace_build, &request(None));
        assert_eq!(status, STATUS_EVALUATION_ERROR, "{payload}");
        assert!(payload.contains("Step"), "{payload}");

        // An empty list replaces them, so nothing is imported and `Step` is an element NX does
        // not check.
        let (status, payload) = call_on(context, nx_wasm_workspace_build, &request(Some(&[])));
        assert_eq!(status, STATUS_OK, "{payload}");
        unsafe { nx_wasm_program_free(handle_from(&payload) as *mut ProgramArtifact) };

        unsafe {
            nx_wasm_build_context_free(context);
            nx_wasm_registry_free(registry);
        }
    }

    #[test]
    fn a_visible_root_that_is_not_loaded_is_named_in_the_error() {
        let registry = take_handle(nx_wasm_registry_new()) as *mut LibraryRegistry;
        let (status, payload) = call_on(
            registry,
            nx_wasm_build_context_new,
            r#"{"visibleRoots":["libraries/nope"]}"#,
        );
        assert_ne!(status, STATUS_OK, "{payload}");
        assert!(
            payload.contains("Visible root 'libraries/nope' is not a loaded library"),
            "{payload}"
        );
        unsafe { nx_wasm_registry_free(registry) };
    }

    #[test]
    fn a_program_answers_with_function_and_type_schemas() {
        let (status, payload) = call(
            nx_wasm_program_build,
            r#"{"source":"/// A plan.\ntype Plan = { name:string }\n/// Finds plans.\nlet findPlans(teamSize:int): Plan* = { {} }","fileName":"tools.nx"}"#,
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let program = handle_from(&payload) as *mut ProgramArtifact;

        let (status, payload) = call_on(
            program,
            nx_wasm_program_function_schema,
            r#"{"reference":{"name":"findPlans"}}"#,
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let answer: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(answer["module"], "tools.nx");
        assert_eq!(answer["description"], "Finds plans.");
        assert_eq!(answer["parameters"][0]["name"], "teamSize");
        assert_eq!(answer["inputSchema"]["required"][0], "teamSize");
        assert_eq!(answer["outputSchema"]["items"]["$ref"], "#/$defs/Plan");
        assert_eq!(answer["declaration"]["start_line"], 4);
        // Key order is part of the answer.
        assert!(
            payload.contains(r#""inputSchema":{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object""#),
            "{payload}"
        );

        let (status, payload) = call_on(
            program,
            nx_wasm_program_type_schema,
            r#"{"reference":{"module":"tools.nx","name":"Plan"},"options":{"direction":"input"}}"#,
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        let answer: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(answer["description"], "A plan.");
        assert!(answer["schema"]["$defs"]["Plan"]["properties"]
            .get("$type")
            .is_none());

        let (status, payload) = call_on(
            program,
            nx_wasm_program_function_schema,
            r#"{"reference":{"name":"missing"}}"#,
        );
        assert_eq!(status, STATUS_EVALUATION_ERROR, "{payload}");
        assert!(payload.contains("schema-unknown-declaration"), "{payload}");

        unsafe { nx_wasm_program_free(program) };
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
        let result = into_result(Ok(b"{\"value\":1}".to_vec()));
        unsafe {
            assert_eq!((*result).status, STATUS_OK);
            assert_eq!(read_payload(result), "{\"value\":1}");
            nx_wasm_result_free(result);
        }
    }

    #[test]
    fn an_empty_payload_round_trips_as_a_null_pointer() {
        let result = into_result(Ok(Vec::new()));
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
    fn a_program_that_builds_reports_its_warnings() {
        let (status, payload) = call(
            nx_wasm_program_build,
            &serde_json::json!({
                "source": "/// See [Missing].\nlet root() = { 42 }",
                "fileName": "input.nx",
            })
            .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");

        let handle = handle_from(&payload) as *mut ProgramArtifact;
        unsafe {
            let result = nx_wasm_program_diagnostics(handle);
            assert_eq!((*result).status, STATUS_OK);
            let diagnostics: Vec<serde_json::Value> =
                serde_json::from_str(&read_payload(result)).unwrap();
            assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
            assert_eq!(diagnostics[0]["severity"], "warning");
            assert_eq!(diagnostics[0]["code"], "unresolved-doc-link");
            assert_eq!(diagnostics[0]["labels"][0]["span"]["start_line"], 1);
            nx_wasm_result_free(result);
            nx_wasm_program_free(handle);
        }
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
            let result = nx_wasm_program_nx_ir(handle, ptr::null(), 0);
            assert_eq!((*result).status, STATUS_OK);
            let artifacts =
                read_nx_ir_bundle(&read_payload_bytes(result)).expect("IR payload is a bundle");
            assert_eq!(artifacts.len(), 1);
            assert_eq!(&artifacts[0].bytes[..4], b"NXIR");
            assert_eq!(artifacts[0].metadata.schema_version, 5);
            nx_wasm_result_free(result);

            let image = artifacts[0].bytes.clone();
            let input = nx_wasm_alloc(image.len());
            ptr::copy_nonoverlapping(image.as_ptr(), input, image.len());
            let result = nx_wasm_ir_explain(input, image.len());
            nx_wasm_free(input, image.len());
            assert_eq!((*result).status, STATUS_OK);
            let text: String = serde_json::from_str(&read_payload(result))
                .expect("explained text is a JSON string");
            assert!(text.contains("function root() ="), "{text}");
            nx_wasm_result_free(result);

            let result = nx_wasm_ir_explain(image.as_ptr(), 8);
            assert_eq!((*result).status, STATUS_EVALUATION_ERROR);
            let diagnostics: Vec<serde_json::Value> =
                serde_json::from_str(&read_payload(result)).unwrap();
            assert_eq!(diagnostics[0]["code"], "nx-ir-malformed");
            nx_wasm_result_free(result);
            nx_wasm_program_free(handle);
        }
    }

    /// Builds `source` and returns its handle.
    fn build(source: &str) -> *mut ProgramArtifact {
        let (status, payload) = call(
            nx_wasm_program_build,
            &serde_json::json!({ "source": source, "fileName": "input.nx" }).to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");
        handle_from(&payload) as *mut ProgramArtifact
    }

    #[test]
    fn a_program_evaluates_to_annotated_nx_text() {
        let handle = build("type User = { id:string name:string }\n<User id=\"1\" name=\"Ada\" />");
        unsafe {
            let result = nx_wasm_program_evaluate_nx(handle);
            assert_eq!((*result).status, STATUS_OK);
            let value: serde_json::Value = serde_json::from_str(&read_payload(result)).unwrap();
            assert_eq!(value["text"], r#"<User id="1" name="Ada" />"#);
            assert_eq!(value["nodes"][0]["role"], "record");
            assert_eq!(value["nodes"][0]["type"], "User");
            assert_eq!(value["nodes"][0]["declaration"]["start_line"], 1);
            assert_eq!(value["nodes"][1]["name"], "id");
            nx_wasm_result_free(result);

            // The artifact stays usable after an evaluation.
            let result = nx_wasm_program_evaluate_nx(handle);
            assert_eq!((*result).status, STATUS_OK);
            nx_wasm_result_free(result);
            nx_wasm_program_free(handle);
        }
    }

    #[test]
    fn an_evaluation_failure_answers_with_diagnostics() {
        let handle = build("let root() = { 1 / 0 }");
        unsafe {
            let result = nx_wasm_program_evaluate_nx(handle);
            assert_eq!((*result).status, STATUS_EVALUATION_ERROR);
            let diagnostics: Vec<serde_json::Value> =
                serde_json::from_str(&read_payload(result)).unwrap();
            assert_eq!(diagnostics[0]["code"], "runtime-error");
            assert_eq!(diagnostics[0]["labels"][0]["span"]["start_byte"], 15);
            nx_wasm_result_free(result);
            nx_wasm_program_free(handle);
        }
    }

    #[test]
    fn a_workspace_builds_with_implicit_imports_and_emits_the_entry_alone() {
        let (status, payload) = call_on(
            ptr::null_mut::<ProgramBuildContext>(),
            nx_wasm_workspace_build,
            &serde_json::json!({
                "modules": [
                    { "identity": "drawnui.nx", "source": "export external component <SkiaLabel Text:string />", "version": "9" },
                    { "identity": "input.nx", "source": "let root() = <SkiaLabel Text=\"hi\" />" }
                ],
                "entry": "input.nx",
                "implicitImports": ["drawnui.nx"]
            })
            .to_string(),
        );
        assert_eq!(status, STATUS_OK, "{payload}");

        let handle = handle_from(&payload) as *mut ProgramArtifact;
        let options = "{}";
        let bytes = options.as_bytes();
        unsafe {
            let input = nx_wasm_alloc(bytes.len());
            ptr::copy_nonoverlapping(bytes.as_ptr(), input, bytes.len());
            let result = nx_wasm_program_nx_ir(handle, input, bytes.len());
            nx_wasm_free(input, bytes.len());
            assert_eq!((*result).status, STATUS_OK);
            let artifacts =
                read_nx_ir_bundle(&read_payload_bytes(result)).expect("IR payload is a bundle");
            assert_eq!(artifacts.len(), 1);
            assert_eq!(artifacts[0].identity, "input.nx");
            let image = NxIrImage::open(&artifacts[0].bytes).expect("a valid image");
            let modules = image.modules().collect::<Vec<_>>();
            assert_eq!(modules[1].identity, "drawnui.nx");
            assert_eq!(modules[1].version, "9");
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
            &serde_json::json!({ "documents": [{ "uri": "nx://demo/input.nx", "source": "let root() = { 42 }" }] })
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
            &serde_json::json!({ "documents": [{ "uri": ":::", "source": "" }] }).to_string(),
        );
        assert_eq!(status, STATUS_EVALUATION_ERROR, "{payload}");
        assert!(
            payload.contains("language-snapshot-input-error"),
            "{payload}"
        );
    }
}
