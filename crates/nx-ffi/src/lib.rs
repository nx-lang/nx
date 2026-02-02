//! C ABI wrapper for NX evaluation, intended for P/Invoke and other FFI consumers.

use nx_api::{eval_source, EvalResult, NxDiagnostic, NxSeverity};
use std::panic;

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

#[no_mangle]
pub extern "C" fn nx_free_buffer(buffer: NxBuffer) {
    if buffer.ptr.is_null() {
        return;
    }

    unsafe {
        let _ = Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.cap);
    }
}

#[no_mangle]
pub extern "C" fn nx_eval_source_msgpack(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    unsafe {
        if out_buffer.is_null() {
            return NxEvalStatus::InvalidArgument;
        }
        *out_buffer = NxBuffer::empty();
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = unsafe { slice_to_str(file_name_ptr, file_name_len) }.unwrap_or("input.nx");
        let file_name = if file_name.is_empty() {
            "input.nx"
        } else {
            file_name
        };

        let bytes = match eval_source(source, file_name) {
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

    match result {
        Ok(Ok((status, bytes))) => {
            unsafe {
                *out_buffer = vec_to_buffer(bytes);
            }
            status
        }
        Ok(Err(message)) => {
            let diagnostics = vec![NxDiagnostic {
                severity: NxSeverity::Error,
                code: Some("ffi-error".to_string()),
                message,
                labels: Vec::new(),
                help: None,
                note: None,
            }];

            if let Ok(payload) = rmp_serde::to_vec_named(&diagnostics) {
                unsafe {
                    *out_buffer = vec_to_buffer(payload);
                }
            }

            NxEvalStatus::Error
        }
        Err(_) => NxEvalStatus::Panic,
    }
}

#[no_mangle]
pub extern "C" fn nx_eval_source_json(
    source_ptr: *const u8,
    source_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
    out_buffer: *mut NxBuffer,
) -> NxEvalStatus {
    unsafe {
        if out_buffer.is_null() {
            return NxEvalStatus::InvalidArgument;
        }
        *out_buffer = NxBuffer::empty();
    }

    let result = panic::catch_unwind(|| {
        let source = unsafe { slice_to_str(source_ptr, source_len) }?;
        let file_name = unsafe { slice_to_str(file_name_ptr, file_name_len) }.unwrap_or("input.nx");
        let file_name = if file_name.is_empty() {
            "input.nx"
        } else {
            file_name
        };

        match eval_source(source, file_name) {
            EvalResult::Ok(value) => {
                let json = value
                    .to_json_string()
                    .map_err(|e| format!("json serialize failed: {e}"))?;
                Ok((NxEvalStatus::Ok, json.into_bytes()))
            }
            EvalResult::Err(diagnostics) => {
                let json = serde_json::to_string(&diagnostics)
                    .map_err(|e| format!("json serialize failed: {e}"))?;
                Ok((NxEvalStatus::Error, json.into_bytes()))
            }
        }
    });

    match result {
        Ok(Ok((status, bytes))) => {
            unsafe {
                *out_buffer = vec_to_buffer(bytes);
            }
            status
        }
        Ok(Err(message)) => {
            let diagnostics = vec![NxDiagnostic {
                severity: NxSeverity::Error,
                code: Some("ffi-error".to_string()),
                message,
                labels: Vec::new(),
                help: None,
                note: None,
            }];

            if let Ok(json) = serde_json::to_string(&diagnostics) {
                unsafe {
                    *out_buffer = vec_to_buffer(json.into_bytes());
                }
            }

            NxEvalStatus::Error
        }
        Err(_) => NxEvalStatus::Panic,
    }
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

fn vec_to_buffer(vec: Vec<u8>) -> NxBuffer {
    let mut vec = std::mem::ManuallyDrop::new(vec);
    NxBuffer {
        ptr: vec.as_mut_ptr(),
        len: vec.len(),
        cap: vec.capacity(),
    }
}
