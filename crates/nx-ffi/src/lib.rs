//! C ABI wrapper for NX evaluation, intended for P/Invoke and other FFI consumers.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use nx_api::{
    dispatch_component_actions_source, eval_source, initialize_component_source,
    ComponentDispatchEvalResult, ComponentDispatchResult, ComponentInitEvalResult,
    ComponentInitResult, EvalResult, NxDiagnostic, NxSeverity,
};
use nx_value::NxValue;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::Any;
use std::panic;

pub const NX_FFI_ABI_VERSION: u32 = 2;

#[repr(C)]
pub struct NxBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
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

fn finish_json_entry(
    out_buffer: *mut NxBuffer,
    result: Result<Result<(NxEvalStatus, String), String>, Box<dyn Any + Send>>,
) -> NxEvalStatus {
    match result {
        Ok(Ok((status, payload))) => {
            write_json_payload(out_buffer, payload);
            status
        }
        Ok(Err(message)) => {
            if let Ok(payload) = serde_json::to_string(&ffi_error_diagnostics(message)) {
                write_json_payload(out_buffer, payload);
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

fn decode_msgpack_payload<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(bytes).map_err(|e| format!("messagepack decode failed: {e}"))
}

fn json_component_dispatch_payload(result: &ComponentDispatchResult) -> Result<String, String> {
    serde_json::to_string(&JsonComponentDispatchResult {
        effects: &result.effects,
        state_snapshot: BASE64_STANDARD.encode(&result.state_snapshot),
    })
    .map_err(|e| format!("json serialize failed: {e}"))
}

#[no_mangle]
pub extern "C" fn nx_eval_source(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = parse_file_name(file_name_ptr, file_name_len)?;

        let bytes = match eval_source(source, &file_name) {
            EvalResult::Ok(value) => {
                let payload = rmp_serde::to_vec(&value)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                (NxEvalStatus::Ok, payload)
            }
            EvalResult::Err(diagnostics) => {
                let payload = rmp_serde::to_vec_named(&diagnostics)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                (NxEvalStatus::Error, payload)
            }
        };

        Ok(bytes)
    });

    finish_msgpack_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_component_init(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    component_name_ptr: *const u8,
    component_name_len: usize,
    props_ptr: *const u8,
    props_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = parse_file_name(file_name_ptr, file_name_len)?;
        let component_name = unsafe { slice_to_str(component_name_ptr, component_name_len) }?;
        let props = if props_len == 0 {
            empty_record()
        } else {
            let bytes = unsafe { slice_to_bytes(props_ptr, props_len) }?;
            parse_msgpack_value(bytes)?
        };

        match initialize_component_source(source, &file_name, component_name, &props) {
            ComponentInitEvalResult::Ok(result) => {
                let payload = rmp_serde::to_vec_named(&result)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Ok, payload))
            }
            ComponentInitEvalResult::Err(diagnostics) => {
                let payload = rmp_serde::to_vec_named(&diagnostics)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Error, payload))
            }
        }
    });

    finish_msgpack_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_component_dispatch_actions(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    state_snapshot_ptr: *const u8,
    state_snapshot_len: usize,
    actions_ptr: *const u8,
    actions_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = parse_file_name(file_name_ptr, file_name_len)?;
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

        match dispatch_component_actions_source(source, &file_name, state_snapshot, &actions) {
            ComponentDispatchEvalResult::Ok(result) => {
                let payload = rmp_serde::to_vec_named(&result)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Ok, payload))
            }
            ComponentDispatchEvalResult::Err(diagnostics) => {
                let payload = rmp_serde::to_vec_named(&diagnostics)
                    .map_err(|e| format!("messagepack serialize failed: {e}"))?;
                Ok((NxEvalStatus::Error, payload))
            }
        }
    });

    finish_msgpack_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_value_msgpack_to_json(
    payload_ptr: *const u8,
    payload_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let payload = unsafe { slice_to_bytes(payload_ptr, payload_len) }?;
        let value: NxValue = decode_msgpack_payload(payload)?;
        Ok((
            NxEvalStatus::Ok,
            value
                .to_json_string()
                .map_err(|e| format!("json serialize failed: {e}"))?,
        ))
    });

    finish_json_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_diagnostics_msgpack_to_json(
    payload_ptr: *const u8,
    payload_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let payload = unsafe { slice_to_bytes(payload_ptr, payload_len) }?;
        let diagnostics: Vec<NxDiagnostic> = decode_msgpack_payload(payload)?;
        Ok((
            NxEvalStatus::Ok,
            serde_json::to_string(&diagnostics)
                .map_err(|e| format!("json serialize failed: {e}"))?,
        ))
    });

    finish_json_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_component_init_result_msgpack_to_json(
    payload_ptr: *const u8,
    payload_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let payload = unsafe { slice_to_bytes(payload_ptr, payload_len) }?;
        let result: ComponentInitResult = decode_msgpack_payload(payload)?;
        Ok((NxEvalStatus::Ok, json_component_init_payload(&result)?))
    });

    finish_json_entry(out_buffer, result)
}

#[no_mangle]
pub extern "C" fn nx_component_dispatch_result_msgpack_to_json(
    payload_ptr: *const u8,
    payload_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    if let Err(status) = prepare_out_buffer(out_buffer) {
        return status;
    }

    let result = panic::catch_unwind(|| {
        let payload = unsafe { slice_to_bytes(payload_ptr, payload_len) }?;
        let result: ComponentDispatchResult = decode_msgpack_payload(payload)?;
        Ok((NxEvalStatus::Ok, json_component_dispatch_payload(&result)?))
    });

    finish_json_entry(out_buffer, result)
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
