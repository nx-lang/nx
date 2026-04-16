//! C ABI wrapper for NX evaluation, intended for P/Invoke and other FFI consumers.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use nx_api::{
    dispatch_component_actions_program_artifact as api_dispatch_component_actions_program_artifact,
    eval_program_artifact as api_eval_program_artifact, eval_source,
    initialize_component_program_artifact as api_initialize_component_program_artifact,
    load_program_artifact_from_source, ComponentDispatchEvalResult, ComponentDispatchResult,
    ComponentInitEvalResult, ComponentInitResult, EvalResult, LibraryRegistry, NxDiagnostic,
    NxSeverity, ProgramArtifact, ProgramBuildContext,
};
use nx_value::NxValue;
use serde::Serialize;
use std::any::Any;
use std::panic;

pub const NX_FFI_ABI_VERSION: u32 = 8;

#[repr(C)]
pub struct NxBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
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
    effects: &'a [NxValue],
    state_snapshot: String,
}

enum FfiPayload {
    Msgpack(Vec<u8>),
    Json(String),
}

impl FfiPayload {
    fn write(self, out_buffer: *mut NxBuffer) {
        match self {
            Self::Msgpack(payload) => write_msgpack_payload(out_buffer, payload),
            Self::Json(payload) => write_json_payload(out_buffer, payload),
        }
    }
}

#[no_mangle]
pub extern "C" fn nx_ffi_abi_version() -> u32 {
    NX_FFI_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn nx_free_buffer(buffer: NxBuffer) {
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

fn finish_msgpack_entry(
    out_buffer: *mut NxBuffer,
    result: Result<Result<(NxEvalStatus, Vec<u8>), String>, Box<dyn Any + Send>>,
) -> NxEvalStatus {
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
    result: Result<Result<(NxEvalStatus, FfiPayload), String>, Box<dyn Any + Send>>,
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

#[no_mangle]
pub extern "C" fn nx_eval_source(
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

#[no_mangle]
pub extern "C" fn nx_build_program_artifact(
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

#[no_mangle]
pub extern "C" fn nx_create_library_registry(
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

#[no_mangle]
pub extern "C" fn nx_free_library_registry(handle: *mut NxLibraryRegistryHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<LibraryRegistryHandleInner>());
    }
}

#[no_mangle]
pub extern "C" fn nx_load_library_into_registry(
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

#[no_mangle]
pub extern "C" fn nx_create_program_build_context(
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

#[no_mangle]
pub extern "C" fn nx_free_program_build_context(handle: *mut NxProgramBuildContextHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<ProgramBuildContextHandleInner>());
    }
}

#[no_mangle]
pub extern "C" fn nx_free_program_artifact(handle: *mut NxProgramArtifactHandle) {
    if handle.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(handle.cast::<ProgramArtifactHandleInner>());
    }
}

#[no_mangle]
pub extern "C" fn nx_eval_program_artifact(
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

#[no_mangle]
pub extern "C" fn nx_component_init_program_artifact(
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

#[no_mangle]
pub extern "C" fn nx_component_dispatch_actions_program_artifact(
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
