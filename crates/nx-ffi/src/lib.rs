//! C ABI wrapper for NX evaluation, intended for P/Invoke and other FFI consumers.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use nx_api::{
    build_workspace_program_artifact, diagnostics_to_api_with_source_entries,
    dispatch_component_actions_program_artifact as api_dispatch_component_actions_program_artifact,
    eval_program_artifact as api_eval_program_artifact, eval_source,
    evaluate_component_program_artifact as api_evaluate_component_program_artifact,
    initialize_component_program_artifact as api_initialize_component_program_artifact,
    load_program_artifact_from_source, validate_workspace, ComponentDispatchEvalResult,
    ComponentDispatchResult, ComponentEvaluateEvalResult, ComponentInitEvalResult,
    ComponentInitResult, EvalResult, LibraryRegistry, NxDiagnostic, NxSeverity, NxWorkspace,
    NxWorkspaceModule as ApiNxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_codegen::{
    emit_js_program_module, emit_nx_ir, explain_nx_ir_image, write_nx_ir_bundle,
    GeneratedJsProgramModule, GeneratedJsProgramModuleComponentExport,
    GeneratedJsProgramModuleFunctionExport, JsProgramModuleOptions, NxIrEmitOptions,
};
use nx_value::NxValue;
use serde::Serialize;
use std::any::Any;
use std::panic;

pub const NX_FFI_ABI_VERSION: u32 = 13;

#[repr(C)]
pub struct NxBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct NxWorkspaceModule {
    pub identity_ptr: *const u8,
    pub identity_len: usize,
    pub source_utf8_ptr: *const u8,
    pub source_utf8_len: usize,
    /// The module's version string as UTF-8; a zero length is no version.
    pub version_ptr: *const u8,
    pub version_len: usize,
}

/// One borrowed UTF-8 string, such as a workspace identity in an implicit-import list.
#[repr(C)]
pub struct NxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

pub struct NxProgramArtifactHandle;

struct ProgramArtifactHandleInner {
    program_artifact: ProgramArtifact,
}

pub struct NxLibraryRegistryHandle;

struct LibraryRegistryHandleInner {
    registry: LibraryRegistry,
}

pub struct NxProgramBuildContextHandle;

struct ProgramBuildContextHandleInner {
    build_context: ProgramBuildContext,
}

impl NxBuffer {
    fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
}

#[repr(u32)]
pub enum NxEvalStatus {
    Ok = 0,
    Error = 1,
    InvalidArgument = 2,
    Panic = 255,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum NxOutputFormat {
    MessagePack = 0,
    Json = 1,
}

impl TryFrom<u32> for NxOutputFormat {
    type Error = NxEvalStatus;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MessagePack),
            1 => Ok(Self::Json),
            _ => Err(NxEvalStatus::InvalidArgument),
        }
    }
}

#[derive(Serialize)]
struct JsonComponentInitResult<'a> {
    rendered: &'a NxValue,
    state_snapshot: String,
}

#[derive(Serialize)]
struct JsonComponentDispatchResult<'a> {
    rendered: &'a NxValue,
    effects: &'a [NxValue],
    state_snapshot: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonGeneratedJsProgramModule {
    source_text: String,
    logical_module_name: String,
    runtime_import_specifier: String,
    runtime_abi: String,
    program_fingerprint: u64,
    function_exports: Vec<JsonGeneratedJsProgramModuleFunctionExport>,
    component_exports: Vec<JsonGeneratedJsProgramModuleComponentExport>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonGeneratedJsProgramModuleFunctionExport {
    entrypoint_name: String,
    export_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonGeneratedJsProgramModuleComponentExport {
    component_name: String,
    component_export_name: String,
    schema_export_name: String,
    initial_state_export_name: Option<String>,
    render_export_name: Option<String>,
}

enum FfiPayload {
    Msgpack(Vec<u8>),
    Json(String),
    /// Bytes in a format the export documents: an NX IR bundle, or explained text.
    Bytes(Vec<u8>),
}

impl FfiPayload {
    fn write(self, out_buffer: *mut NxBuffer) {
        match self {
            Self::Msgpack(payload) | Self::Bytes(payload) => {
                write_msgpack_payload(out_buffer, payload)
            }
            Self::Json(payload) => write_json_payload(out_buffer, payload),
        }
    }
}

#[no_mangle]
pub extern "C" fn nx_ffi_abi_version() -> u32 {
    NX_FFI_ABI_VERSION
}

/// Releases a buffer this library wrote.
///
/// # Safety
///
/// `buffer` must be a buffer one of this library's entry points wrote and has not yet released, or
/// a null buffer. It must not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_free_buffer(buffer: NxBuffer) {
    if buffer.ptr.is_null() {
        return;
    }

    unsafe {
        let _ = Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.cap);
    }
}

fn ffi_error_diagnostics(message: String) -> Vec<NxDiagnostic> {
    vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("ffi-error".to_string()),
        message,
        labels: Vec::new(),
        help: None,
        note: None,
    }]
}

fn write_msgpack_payload(out_buffer: *mut NxBuffer, payload: Vec<u8>) {
    unsafe {
        *out_buffer = vec_to_buffer(payload);
    }
}

fn write_json_payload(out_buffer: *mut NxBuffer, payload: String) {
    unsafe {
        *out_buffer = vec_to_buffer(payload.into_bytes());
    }
}

fn prepare_out_buffer(out_buffer: *mut NxBuffer) -> Result<(), NxEvalStatus> {
    unsafe {
        if out_buffer.is_null() {
            return Err(NxEvalStatus::InvalidArgument);
        }
        *out_buffer = NxBuffer::empty();
    }

    Ok(())
}

fn parse_output_format(output_format: u32) -> Result<NxOutputFormat, NxEvalStatus> {
    NxOutputFormat::try_from(output_format)
}

fn prepare_out_program_artifact_handle(
    out_handle: *mut *mut NxProgramArtifactHandle,
) -> Result<(), NxEvalStatus> {
    unsafe {
        if out_handle.is_null() {
            return Err(NxEvalStatus::InvalidArgument);
        }

        *out_handle = std::ptr::null_mut();
    }

    Ok(())
}

fn prepare_out_library_registry_handle(
    out_handle: *mut *mut NxLibraryRegistryHandle,
) -> Result<(), NxEvalStatus> {
    unsafe {
        if out_handle.is_null() {
            return Err(NxEvalStatus::InvalidArgument);
        }

        *out_handle = std::ptr::null_mut();
    }

    Ok(())
}

fn prepare_out_build_context_handle(
    out_handle: *mut *mut NxProgramBuildContextHandle,
) -> Result<(), NxEvalStatus> {
    unsafe {
        if out_handle.is_null() {
            return Err(NxEvalStatus::InvalidArgument);
        }

        *out_handle = std::ptr::null_mut();
    }

    Ok(())
}

/// What `panic::catch_unwind` hands back from an entry point's body: the payload it produced, the
/// message it failed with, which is reported as diagnostics, or the panic it did not survive.
type CaughtEntry<T> = Result<Result<(NxEvalStatus, T), String>, Box<dyn Any + Send>>;

fn finish_msgpack_entry(out_buffer: *mut NxBuffer, result: CaughtEntry<Vec<u8>>) -> NxEvalStatus {
    match result {
        Ok(Ok((status, payload))) => {
            write_msgpack_payload(out_buffer, payload);
            status
        }
        Ok(Err(message)) => {
            if let Ok(payload) = rmp_serde::to_vec_named(&ffi_error_diagnostics(message)) {
                write_msgpack_payload(out_buffer, payload);
            }
            NxEvalStatus::Error
        }
        Err(_) => NxEvalStatus::Panic,
    }
}

fn finish_output_entry(
    out_buffer: *mut NxBuffer,
    output_format: NxOutputFormat,
    result: CaughtEntry<FfiPayload>,
) -> NxEvalStatus {
    match result {
        Ok(Ok((status, payload))) => {
            payload.write(out_buffer);
            status
        }
        Ok(Err(message)) => {
            if let Ok(payload) =
                serialize_diagnostics_payload(output_format, &ffi_error_diagnostics(message))
            {
                payload.write(out_buffer);
            }
            NxEvalStatus::Error
        }
        Err(_) => NxEvalStatus::Panic,
    }
}

fn parse_file_name(file_name_ptr: *const u8, file_name_len: usize) -> Result<String, String> {
    let file_name = unsafe { slice_to_str(file_name_ptr, file_name_len) }.unwrap_or("input.nx");
    if file_name.is_empty() {
        Ok("input.nx".to_string())
    } else {
        Ok(file_name.to_string())
    }
}

fn parse_workspace_modules(
    modules_ptr: *const NxWorkspaceModule,
    module_count: usize,
) -> Result<NxWorkspace, NxEvalStatus> {
    if module_count > 0 && modules_ptr.is_null() {
        return Err(NxEvalStatus::InvalidArgument);
    }

    let descriptors = if module_count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(modules_ptr, module_count) }
    };
    let mut modules = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        let identity = unsafe {
            slice_to_str(descriptor.identity_ptr, descriptor.identity_len)
                .map_err(|_| NxEvalStatus::InvalidArgument)?
        };
        let source_utf8 = unsafe {
            slice_to_bytes(descriptor.source_utf8_ptr, descriptor.source_utf8_len)
                .map_err(|_| NxEvalStatus::InvalidArgument)?
        };
        let source = std::str::from_utf8(source_utf8).map_err(|_| NxEvalStatus::InvalidArgument)?;
        let version = unsafe {
            slice_to_str(descriptor.version_ptr, descriptor.version_len)
                .map_err(|_| NxEvalStatus::InvalidArgument)?
        };

        modules.push(
            ApiNxWorkspaceModule::from_source(identity, source)
                .map_err(|_| NxEvalStatus::InvalidArgument)?
                .with_version(version),
        );
    }

    NxWorkspace::new(modules).map_err(|_| NxEvalStatus::InvalidArgument)
}

fn parse_utf8_slices(
    slices_ptr: *const NxUtf8Slice,
    slice_count: usize,
) -> Result<Vec<String>, NxEvalStatus> {
    if slice_count > 0 && slices_ptr.is_null() {
        return Err(NxEvalStatus::InvalidArgument);
    }

    let slices = if slice_count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(slices_ptr, slice_count) }
    };
    slices
        .iter()
        .map(|slice| parse_required_utf8(slice.ptr, slice.len))
        .collect()
}

/// The build context a workspace call runs against: the handle's own, or a copy of it naming the
/// implicit imports the caller passed.
fn workspace_build_context(
    build_context: &ProgramBuildContext,
    implicit_imports: Vec<String>,
) -> std::borrow::Cow<'_, ProgramBuildContext> {
    if implicit_imports.is_empty() {
        std::borrow::Cow::Borrowed(build_context)
    } else {
        std::borrow::Cow::Owned(
            build_context
                .clone()
                .with_implicit_imports(implicit_imports),
        )
    }
}

fn parse_required_utf8(ptr: *const u8, len: usize) -> Result<String, NxEvalStatus> {
    unsafe { slice_to_str(ptr, len) }
        .map(str::to_string)
        .map_err(|_| NxEvalStatus::InvalidArgument)
}

fn with_program_artifact<T>(
    handle_ptr: *const NxProgramArtifactHandle,
    f: impl FnOnce(&ProgramArtifact) -> Result<T, String>,
) -> Result<T, String> {
    if handle_ptr.is_null() {
        return Err("program artifact handle is null".to_string());
    }

    let handle = unsafe { &*handle_ptr.cast::<ProgramArtifactHandleInner>() };
    f(&handle.program_artifact)
}

fn with_library_registry<T>(
    handle_ptr: *const NxLibraryRegistryHandle,
    f: impl FnOnce(&LibraryRegistry) -> Result<T, String>,
) -> Result<T, String> {
    if handle_ptr.is_null() {
        return Err("library registry handle is null".to_string());
    }

    let handle = unsafe { &*handle_ptr.cast::<LibraryRegistryHandleInner>() };
    f(&handle.registry)
}

fn empty_record() -> NxValue {
    NxValue::Record {
        type_name: None,
        properties: Default::default(),
    }
}

fn parse_msgpack_value(bytes: &[u8]) -> Result<NxValue, String> {
    NxValue::from_msgpack_slice(bytes).map_err(|e| format!("messagepack decode failed: {e}"))
}

fn parse_msgpack_actions(bytes: &[u8]) -> Result<Vec<NxValue>, String> {
    rmp_serde::from_slice(bytes).map_err(|e| format!("messagepack decode failed: {e}"))
}

fn json_component_init_payload(result: &ComponentInitResult) -> Result<String, String> {
    serde_json::to_string(&JsonComponentInitResult {
        rendered: &result.rendered,
        state_snapshot: BASE64_STANDARD.encode(&result.state_snapshot),
    })
    .map_err(|e| format!("json serialize failed: {e}"))
}

fn json_component_dispatch_payload(result: &ComponentDispatchResult) -> Result<String, String> {
    serde_json::to_string(&JsonComponentDispatchResult {
        rendered: &result.rendered,
        effects: &result.effects,
        state_snapshot: BASE64_STANDARD.encode(&result.state_snapshot),
    })
    .map_err(|e| format!("json serialize failed: {e}"))
}

fn serialize_eval_payload(
    output_format: NxOutputFormat,
    value: &NxValue,
) -> Result<FfiPayload, String> {
    match output_format {
        NxOutputFormat::MessagePack => Ok(FfiPayload::Msgpack(
            rmp_serde::to_vec(value).map_err(|e| format!("messagepack serialize failed: {e}"))?,
        )),
        NxOutputFormat::Json => Ok(FfiPayload::Json(
            value
                .to_json_string()
                .map_err(|e| format!("json serialize failed: {e}"))?,
        )),
    }
}

fn serialize_diagnostics_payload(
    output_format: NxOutputFormat,
    diagnostics: &[NxDiagnostic],
) -> Result<FfiPayload, String> {
    match output_format {
        NxOutputFormat::MessagePack => Ok(FfiPayload::Msgpack(
            rmp_serde::to_vec_named(diagnostics)
                .map_err(|e| format!("messagepack serialize failed: {e}"))?,
        )),
        NxOutputFormat::Json => Ok(FfiPayload::Json(
            serde_json::to_string(diagnostics)
                .map_err(|e| format!("json serialize failed: {e}"))?,
        )),
    }
}

fn serialize_component_init_payload(
    output_format: NxOutputFormat,
    result: &ComponentInitResult,
) -> Result<FfiPayload, String> {
    match output_format {
        NxOutputFormat::MessagePack => Ok(FfiPayload::Msgpack(
            rmp_serde::to_vec_named(result)
                .map_err(|e| format!("messagepack serialize failed: {e}"))?,
        )),
        NxOutputFormat::Json => Ok(FfiPayload::Json(json_component_init_payload(result)?)),
    }
}

fn serialize_component_dispatch_payload(
    output_format: NxOutputFormat,
    result: &ComponentDispatchResult,
) -> Result<FfiPayload, String> {
    match output_format {
        NxOutputFormat::MessagePack => Ok(FfiPayload::Msgpack(
            rmp_serde::to_vec_named(result)
                .map_err(|e| format!("messagepack serialize failed: {e}"))?,
        )),
        NxOutputFormat::Json => Ok(FfiPayload::Json(json_component_dispatch_payload(result)?)),
    }
}

fn json_generated_js_program_module_payload(
    module: GeneratedJsProgramModule,
) -> Result<String, String> {
    serde_json::to_string(&JsonGeneratedJsProgramModule::from(module))
        .map_err(|e| format!("json serialize failed: {e}"))
}

impl From<GeneratedJsProgramModule> for JsonGeneratedJsProgramModule {
    fn from(module: GeneratedJsProgramModule) -> Self {
        Self {
            source_text: module.source_text,
            logical_module_name: module.logical_module_name,
            runtime_import_specifier: module.runtime_import_specifier,
            runtime_abi: module.runtime_abi,
            program_fingerprint: module.program_fingerprint,
            function_exports: module
                .function_exports
                .into_iter()
                .map(JsonGeneratedJsProgramModuleFunctionExport::from)
                .collect(),
            component_exports: module
                .component_exports
                .into_iter()
                .map(JsonGeneratedJsProgramModuleComponentExport::from)
                .collect(),
        }
    }
}

impl From<GeneratedJsProgramModuleFunctionExport> for JsonGeneratedJsProgramModuleFunctionExport {
    fn from(export: GeneratedJsProgramModuleFunctionExport) -> Self {
        Self {
            entrypoint_name: export.entrypoint_name,
            export_name: export.export_name,
        }
    }
}

impl From<GeneratedJsProgramModuleComponentExport> for JsonGeneratedJsProgramModuleComponentExport {
    fn from(export: GeneratedJsProgramModuleComponentExport) -> Self {
        Self {
            component_name: export.component_name,
            component_export_name: export.component_export_name,
            schema_export_name: export.schema_export_name,
            initial_state_export_name: export.initial_state_export_name,
            render_export_name: export.render_export_name,
        }
    }
}

/// # Safety
///
/// `source_ptr`/`source_len` and `file_name_ptr`/`file_name_len` must each describe one readable
/// region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_eval_source(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    output_format: u32,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let output_format = match parse_output_format(output_format) {
        Ok(output_format) => output_format,
        Err(status) => return status,
    };

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = parse_file_name(file_name_ptr, file_name_len)?;
        let build_context = ProgramBuildContext::empty();

        let payload = match eval_source(source, &file_name, &build_context) {
            EvalResult::Ok(value) => (
                NxEvalStatus::Ok,
                serialize_eval_payload(output_format, &value)?,
            ),
            EvalResult::Err(diagnostics) => (
                NxEvalStatus::Error,
                serialize_diagnostics_payload(output_format, &diagnostics)?,
            ),
        };

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// # Safety
///
/// `build_context_ptr` must be a build context this library returned and has not yet freed.
///
/// `source_ptr`/`source_len` and `file_name_ptr`/`file_name_len` must each describe one readable
/// region of that many bytes.
///
/// `out_handle` must point to a writable slot; a handle written to it is released with
/// [`nx_free_program_artifact`].
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_build_program_artifact(
    build_context_ptr: *const NxProgramBuildContextHandle,
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    out_handle: *mut *mut NxProgramArtifactHandle,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_program_artifact_handle(out_handle) {
        return status;
    }

    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if build_context_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = parse_file_name(file_name_ptr, file_name_len)?;
        let handle = unsafe { &*build_context_ptr.cast::<ProgramBuildContextHandleInner>() };
        let build_context = handle.build_context.clone();

        match load_program_artifact_from_source(source, &file_name, &build_context) {
            Ok(program_artifact) => {
                let handle = Box::new(ProgramArtifactHandleInner { program_artifact });
                unsafe {
                    *out_handle = Box::into_raw(handle).cast::<NxProgramArtifactHandle>();
                }
                Ok((NxEvalStatus::Ok, Vec::new()))
            }
            Err(diagnostics) => {
                let payload = rmp_serde::to_vec_named(&diagnostics)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Error, payload))
            }
        }
    });

    finish_msgpack_entry(out_buffer, result)
}

/// # Safety
///
/// `build_context_ptr` must be a build context this library returned and has not yet freed.
///
/// `modules_ptr` must point to `module_count` readable `NxWorkspaceModule` values, and each one's
/// own pointer/length pairs must describe readable regions; `implicit_imports_ptr` must likewise
/// point to `implicit_import_count` readable `NxUtf8Slice` values.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_validate_workspace(
    build_context_ptr: *const NxProgramBuildContextHandle,
    modules_ptr: *const NxWorkspaceModule,
    module_count: usize,
    implicit_imports_ptr: *const NxUtf8Slice,
    implicit_import_count: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if build_context_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let workspace = match parse_workspace_modules(modules_ptr, module_count) {
        Ok(workspace) => workspace,
        Err(status) => return status,
    };
    let implicit_imports = match parse_utf8_slices(implicit_imports_ptr, implicit_import_count) {
        Ok(implicit_imports) => implicit_imports,
        Err(status) => return status,
    };

    let result = panic::catch_unwind(|| {
        let handle = unsafe { &*build_context_ptr.cast::<ProgramBuildContextHandleInner>() };
        let build_context = workspace_build_context(&handle.build_context, implicit_imports);
        let diagnostics = validate_workspace(&workspace, &build_context);
        let payload = rmp_serde::to_vec_named(&diagnostics)
            .map_err(|e| format!("messagepack serialize failed: {e}"))?;
        Ok((NxEvalStatus::Ok, payload))
    });

    finish_msgpack_entry(out_buffer, result)
}

/// # Safety
///
/// `build_context_ptr` must be a build context this library returned and has not yet freed.
///
/// `modules_ptr` must point to `module_count` readable `NxWorkspaceModule` values, and each one's
/// own pointer/length pairs must describe readable regions; `implicit_imports_ptr` must likewise
/// point to `implicit_import_count` readable `NxUtf8Slice` values.
///
/// `entry_identity_ptr`/`entry_identity_len` must describe one readable region of that many bytes.
///
/// `out_handle` must point to a writable slot; a handle written to it is released with
/// [`nx_free_program_artifact`].
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_build_workspace_program_artifact(
    build_context_ptr: *const NxProgramBuildContextHandle,
    modules_ptr: *const NxWorkspaceModule,
    module_count: usize,
    entry_identity_ptr: *const u8,
    entry_identity_len: usize,
    implicit_imports_ptr: *const NxUtf8Slice,
    implicit_import_count: usize,
    out_handle: *mut *mut NxProgramArtifactHandle,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_program_artifact_handle(out_handle) {
        return status;
    }

    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if build_context_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let workspace = match parse_workspace_modules(modules_ptr, module_count) {
        Ok(workspace) => workspace,
        Err(status) => return status,
    };
    let entry_identity = match parse_required_utf8(entry_identity_ptr, entry_identity_len) {
        Ok(entry_identity) => entry_identity,
        Err(status) => return status,
    };
    let implicit_imports = match parse_utf8_slices(implicit_imports_ptr, implicit_import_count) {
        Ok(implicit_imports) => implicit_imports,
        Err(status) => return status,
    };

    let result = panic::catch_unwind(|| {
        let handle = unsafe { &*build_context_ptr.cast::<ProgramBuildContextHandleInner>() };
        let build_context = workspace_build_context(&handle.build_context, implicit_imports);
        match build_workspace_program_artifact(&workspace, &entry_identity, &build_context) {
            Ok(program_artifact) => {
                let handle = Box::new(ProgramArtifactHandleInner { program_artifact });
                unsafe {
                    *out_handle = Box::into_raw(handle).cast::<NxProgramArtifactHandle>();
                }
                Ok((NxEvalStatus::Ok, Vec::new()))
            }
            Err(diagnostics) => {
                let payload = rmp_serde::to_vec_named(&diagnostics)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Error, payload))
            }
        }
    });

    finish_msgpack_entry(out_buffer, result)
}

/// # Safety
///
/// `out_handle` must point to a writable slot; a handle written to it is released with
/// [`nx_free_library_registry`].
#[no_mangle]
pub unsafe extern "C" fn nx_create_library_registry(
    out_handle: *mut *mut NxLibraryRegistryHandle,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_library_registry_handle(out_handle) {
        return status;
    }

    let handle = Box::new(LibraryRegistryHandleInner {
        registry: LibraryRegistry::new(),
    });
    unsafe {
        *out_handle = Box::into_raw(handle).cast::<NxLibraryRegistryHandle>();
    }
    NxEvalStatus::Ok
}

/// # Safety
///
/// `handle` must be a registry this library returned and has not yet freed, or null. It must not
/// be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_free_library_registry(handle: *mut NxLibraryRegistryHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<LibraryRegistryHandleInner>());
    }
}

/// # Safety
///
/// `registry_ptr` must be a registry this library returned and has not yet freed.
///
/// `root_path_ptr`/`root_path_len` must describe one readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_load_library_into_registry(
    registry_ptr: *const NxLibraryRegistryHandle,
    root_path_ptr: *const u8,
    root_path_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if registry_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let root_path = unsafe { slice_to_str(root_path_ptr, root_path_len) }?;
        if root_path.is_empty() {
            return Err("library root path is empty".to_string());
        }

        let bytes = with_library_registry(registry_ptr, |registry| {
            match registry.load_library_from_directory(root_path) {
                Ok(_) => Ok((NxEvalStatus::Ok, Vec::new())),
                Err(diagnostics) => {
                    let payload = rmp_serde::to_vec_named(&diagnostics)
                        .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                    Ok((NxEvalStatus::Error, payload))
                }
            }
        })?;

        Ok(bytes)
    });

    finish_msgpack_entry(out_buffer, result)
}

/// # Safety
///
/// `registry_ptr` must be a registry this library returned and has not yet freed; it must outlive
/// the build context.
///
/// `out_handle` must point to a writable slot; a handle written to it is released with
/// [`nx_free_program_build_context`].
#[no_mangle]
pub unsafe extern "C" fn nx_create_program_build_context(
    registry_ptr: *const NxLibraryRegistryHandle,
    out_handle: *mut *mut NxProgramBuildContextHandle,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_build_context_handle(out_handle) {
        return status;
    }

    if registry_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let build_context =
            with_library_registry(registry_ptr, |registry| Ok(registry.build_context()))?;
        let handle = Box::new(ProgramBuildContextHandleInner { build_context });
        unsafe {
            *out_handle = Box::into_raw(handle).cast::<NxProgramBuildContextHandle>();
        }
        Ok::<(), String>(())
    });

    match result {
        Ok(Ok(())) => NxEvalStatus::Ok,
        Ok(Err(_)) => NxEvalStatus::Error,
        Err(_) => NxEvalStatus::Panic,
    }
}

/// # Safety
///
/// `handle` must be a build context this library returned and has not yet freed, or null. It must
/// not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_free_program_build_context(handle: *mut NxProgramBuildContextHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<ProgramBuildContextHandleInner>());
    }
}

/// # Safety
///
/// `handle` must be a program artifact this library returned and has not yet freed, or null. It
/// must not be used again afterwards.
#[no_mangle]
pub unsafe extern "C" fn nx_free_program_artifact(handle: *mut NxProgramArtifactHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<ProgramArtifactHandleInner>());
    }
}

/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_eval_program_artifact(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    output_format: u32,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let output_format = match parse_output_format(output_format) {
        Ok(output_format) => output_format,
        Err(status) => return status,
    };

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match api_eval_program_artifact(program_artifact) {
                EvalResult::Ok(value) => Ok((
                    NxEvalStatus::Ok,
                    serialize_eval_payload(output_format, &value)?,
                )),
                EvalResult::Err(diagnostics) => Ok((
                    NxEvalStatus::Error,
                    serialize_diagnostics_payload(output_format, &diagnostics)?,
                )),
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `logical_module_name_ptr`/`logical_module_name_len` and
/// `runtime_import_specifier_ptr`/`runtime_import_specifier_len` must each describe one readable
/// region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_codegen_js_program_module(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    logical_module_name_ptr: *const u8,
    logical_module_name_len: usize,
    runtime_import_specifier_ptr: *const u8,
    runtime_import_specifier_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let output_format = NxOutputFormat::Json;
    let result = panic::catch_unwind(|| {
        let logical_module_name =
            unsafe { slice_to_str(logical_module_name_ptr, logical_module_name_len) }?;
        let runtime_import_specifier =
            unsafe { slice_to_str(runtime_import_specifier_ptr, runtime_import_specifier_len) }?;
        let mut options = JsProgramModuleOptions::javascript();
        if !logical_module_name.is_empty() {
            options.logical_module_name = logical_module_name.to_string();
        }
        if !runtime_import_specifier.is_empty() {
            options.runtime_import_specifier = runtime_import_specifier.to_string();
        }

        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match emit_js_program_module(program_artifact, &options) {
                Ok(module) => Ok((
                    NxEvalStatus::Ok,
                    FfiPayload::Json(json_generated_js_program_module_payload(module)?),
                )),
                Err(error) => {
                    let diagnostics = error
                        .diagnostics
                        .iter()
                        .map(|diagnostic| NxDiagnostic {
                            severity: diagnostic.severity().into(),
                            code: diagnostic.code().map(str::to_string),
                            message: diagnostic.message().to_string(),
                            labels: Vec::new(),
                            help: None,
                            note: None,
                        })
                        .collect::<Vec<_>>();
                    Ok((
                        NxEvalStatus::Error,
                        serialize_diagnostics_payload(output_format, &diagnostics)?,
                    ))
                }
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// Explains an NX IR image as text with every table index resolved.
///
/// `image_ptr` and `image_len` describe the image. The payload is the UTF-8 text on success, or
/// the JSON diagnostics with [`NxEvalStatus::Error`] when the image is malformed, truncated or of
/// a schema version this build does not read. The image is validated before it is read, so no
/// input traps.
///
/// # Safety
///
/// `image_ptr`/`image_len` must describe one readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_ir_explain(
    image_ptr: *const u8,
    image_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }
    if image_ptr.is_null() && image_len > 0 {
        return NxEvalStatus::InvalidArgument;
    }
    let image: &[u8] = if image_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(image_ptr, image_len) }
    };

    let output_format = NxOutputFormat::Json;
    let result = panic::catch_unwind(|| match explain_nx_ir_image(image) {
        Ok(text) => Ok((NxEvalStatus::Ok, FfiPayload::Bytes(text.into_bytes()))),
        Err(error) => {
            let code = match error {
                nx_codegen::ExplainError::SchemaVersion { .. } => "nx-ir-schema-version",
                nx_codegen::ExplainError::Malformed(_) => "nx-ir-malformed",
            };
            let diagnostics = vec![NxDiagnostic {
                severity: NxSeverity::Error,
                code: Some(code.to_string()),
                message: error.to_string(),
                labels: Vec::new(),
                help: None,
                note: None,
            }];
            Ok((
                NxEvalStatus::Error,
                serialize_diagnostics_payload(output_format, &diagnostics)?,
            ))
        }
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// Emits NX IR artifacts from a program artifact.
///
/// `options_ptr` and `options_len` describe the emit options as UTF-8 JSON, `{ "modules": [...],
/// "debug": false }` with every key optional; an empty text is the default, which emits the entry
/// module alone without its debug section. Each module's version comes from the workspace the
/// program was built from.
///
/// The payload is an NX IR bundle: a little-endian `u32` header length, a JSON header
/// `[{ identity, metadata, offset, length }]`, zero padding to four bytes, then the images at the
/// offsets the header gives, measured from the start of the payload. On error the payload is the
/// JSON diagnostics.
///
/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `options_ptr`/`options_len` must describe one readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_codegen_nx_ir(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    options_ptr: *const u8,
    options_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }
    let options_json = if options_len == 0 {
        String::new()
    } else {
        match parse_required_utf8(options_ptr, options_len) {
            Ok(options) => options,
            Err(status) => return status,
        }
    };
    let options = match NxIrEmitOptions::from_json(&options_json) {
        Ok(options) => options,
        Err(_) => return NxEvalStatus::InvalidArgument,
    };

    let output_format = NxOutputFormat::Json;
    let result = panic::catch_unwind(|| {
        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match emit_nx_ir(program_artifact, &options) {
                Ok(artifacts) => Ok((
                    NxEvalStatus::Ok,
                    FfiPayload::Bytes(
                        write_nx_ir_bundle(&artifacts)
                            .map_err(|e| format!("bundle serialize failed: {e}"))?,
                    ),
                )),
                Err(error) => {
                    let fallback_source = program_artifact
                        .source_text(&program_artifact.entry_identity)
                        .unwrap_or_default();
                    let sources = program_artifact
                        .source_entries()
                        .into_iter()
                        .map(|entry| (entry.identity, entry.source));
                    let diagnostics = diagnostics_to_api_with_source_entries(
                        &error.diagnostics,
                        fallback_source,
                        sources,
                    );
                    Ok((
                        NxEvalStatus::Error,
                        serialize_diagnostics_payload(output_format, &diagnostics)?,
                    ))
                }
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `component_name_ptr`/`component_name_len` and `props_ptr`/`props_len` must each describe one
/// readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_component_init_program_artifact(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    component_name_ptr: *const u8,
    component_name_len: usize,
    props_ptr: *const u8,
    props_len: usize,
    output_format: u32,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let output_format = match parse_output_format(output_format) {
        Ok(output_format) => output_format,
        Err(status) => return status,
    };

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let component_name = unsafe { slice_to_str(component_name_ptr, component_name_len) }?;
        let props = if props_len == 0 {
            empty_record()
        } else {
            let bytes = unsafe { slice_to_bytes(props_ptr, props_len) }?;
            parse_msgpack_value(bytes)?
        };

        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match api_initialize_component_program_artifact(
                program_artifact,
                component_name,
                &props,
            ) {
                ComponentInitEvalResult::Ok(result) => Ok((
                    NxEvalStatus::Ok,
                    serialize_component_init_payload(output_format, &result)?,
                )),
                ComponentInitEvalResult::Err(diagnostics) => Ok((
                    NxEvalStatus::Error,
                    serialize_diagnostics_payload(output_format, &diagnostics)?,
                )),
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// Evaluates a component from a program artifact using MessagePack props/state inputs.
///
/// Successful output is the rendered value directly in the selected format, without lifecycle
/// state snapshot, effects, or wrapper fields.
///
/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `component_name_ptr`/`component_name_len`, `props_ptr`/`props_len` and `state_ptr`/`state_len`
/// must each describe one readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_component_evaluate_program_artifact(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    component_name_ptr: *const u8,
    component_name_len: usize,
    props_ptr: *const u8,
    props_len: usize,
    state_ptr: *const u8,
    state_len: usize,
    output_format: u32,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let output_format = match parse_output_format(output_format) {
        Ok(output_format) => output_format,
        Err(status) => return status,
    };

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let component_name = unsafe { slice_to_str(component_name_ptr, component_name_len) }?;
        let props = if props_len == 0 {
            empty_record()
        } else {
            let bytes = unsafe { slice_to_bytes(props_ptr, props_len) }?;
            parse_msgpack_value(bytes)?
        };
        let state = if state_len == 0 {
            empty_record()
        } else {
            let bytes = unsafe { slice_to_bytes(state_ptr, state_len) }?;
            parse_msgpack_value(bytes)?
        };

        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match api_evaluate_component_program_artifact(
                program_artifact,
                component_name,
                &props,
                &state,
            ) {
                ComponentEvaluateEvalResult::Ok(result) => Ok((
                    NxEvalStatus::Ok,
                    serialize_eval_payload(output_format, &result.rendered)?,
                )),
                ComponentEvaluateEvalResult::Err(diagnostics) => Ok((
                    NxEvalStatus::Error,
                    serialize_diagnostics_payload(output_format, &diagnostics)?,
                )),
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

/// Dispatches a MessagePack batch against a component snapshot from a program artifact.
///
/// Each batch entry is either an action record the component emits, which runs the handler its
/// parent bound, or an `ActionHandlerInvocation` record `{ token, action }` that runs a handler
/// from the snapshot's rendered output, identified by the `token` its `ActionHandler` record
/// carried. The whole batch succeeds or fails together.
///
/// Successful output carries `rendered` (the body re-rendered against the final state, with fresh
/// handler tokens), `effects`, and `state_snapshot` in the selected format.
///
/// # Safety
///
/// `program_artifact_ptr` must be a program artifact this library returned and has not yet freed.
///
/// `state_snapshot_ptr`/`state_snapshot_len` and `actions_ptr`/`actions_len` must each describe one
/// readable region of that many bytes.
///
/// `out_buffer` must point to a writable `NxBuffer` the caller owns; the payload written to it
/// is released with [`nx_free_buffer`].
#[no_mangle]
pub unsafe extern "C" fn nx_component_dispatch_actions_program_artifact(
    program_artifact_ptr: *const NxProgramArtifactHandle,
    state_snapshot_ptr: *const u8,
    state_snapshot_len: usize,
    actions_ptr: *const u8,
    actions_len: usize,
    output_format: u32,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let output_format = match parse_output_format(output_format) {
        Ok(output_format) => output_format,
        Err(status) => return status,
    };

    if program_artifact_ptr.is_null() {
        return NxEvalStatus::InvalidArgument;
    }

    let result = panic::catch_unwind(|| {
        let state_snapshot = if state_snapshot_len == 0 {
            &[][..]
        } else {
            unsafe { slice_to_bytes(state_snapshot_ptr, state_snapshot_len) }?
        };
        let actions = if actions_len == 0 {
            Vec::new()
        } else {
            let bytes = unsafe { slice_to_bytes(actions_ptr, actions_len) }?;
            parse_msgpack_actions(bytes)?
        };

        let payload = with_program_artifact(program_artifact_ptr, |program_artifact| {
            match api_dispatch_component_actions_program_artifact(
                program_artifact,
                state_snapshot,
                &actions,
            ) {
                ComponentDispatchEvalResult::Ok(result) => Ok((
                    NxEvalStatus::Ok,
                    serialize_component_dispatch_payload(output_format, &result)?,
                )),
                ComponentDispatchEvalResult::Err(diagnostics) => Ok((
                    NxEvalStatus::Error,
                    serialize_diagnostics_payload(output_format, &diagnostics)?,
                )),
            }
        })?;

        Ok(payload)
    });

    finish_output_entry(out_buffer, output_format, result)
}

unsafe fn slice_to_str<'a>(ptr: *const u8, len: usize) -> Result<&'a str, String> {
    if len == 0 {
        return Ok("");
    }
    if ptr.is_null() {
        return Err("null pointer".to_string());
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).map_err(|e| format!("invalid utf-8: {e}"))
}

unsafe fn slice_to_bytes<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], String> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err("null pointer".to_string());
    }

    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

fn vec_to_buffer(vec: Vec<u8>) -> NxBuffer {
    let mut vec = std::mem::ManuallyDrop::new(vec);
    NxBuffer {
        ptr: vec.as_mut_ptr(),
        len: vec.len(),
        cap: vec.capacity(),
    }
}
