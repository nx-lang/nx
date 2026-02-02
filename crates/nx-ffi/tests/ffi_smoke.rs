use nx_api::NxDiagnostic;
use nx_ffi::{nx_eval_source_json, nx_eval_source_msgpack, nx_free_buffer, NxBuffer, NxEvalStatus};
use nx_value::NxValue;

fn eval_msgpack(source: &str) -> (NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name = b"test.nx";
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let status = nx_eval_source_msgpack(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name.as_ptr(),
        file_name.len(),
        &mut out as *mut NxBuffer,
    );

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, bytes)
}

fn eval_json(source: &str) -> (NxEvalStatus, String) {
    let source_bytes = source.as_bytes();
    let file_name = b"test.nx";
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let status = nx_eval_source_json(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name.as_ptr(),
        file_name.len(),
        &mut out as *mut NxBuffer,
    );

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, String::from_utf8(bytes).unwrap())
}

#[test]
fn ffi_msgpack_success_round_trip() {
    let (status, bytes) = eval_msgpack("let root() = { 42 }");
    assert!(matches!(status, NxEvalStatus::Ok));

    let decoded = NxValue::from_msgpack_slice(&bytes).unwrap();
    assert_eq!(decoded, NxValue::Int(42));
}

#[test]
fn ffi_json_success_returns_utf8_json() {
    let (status, json) = eval_json("let root() = { 42 }");
    assert!(matches!(status, NxEvalStatus::Ok));
    assert_eq!(json, "42");
}

#[test]
fn ffi_msgpack_error_returns_diagnostics() {
    let (status, bytes) = eval_msgpack("let x = ");
    assert!(matches!(status, NxEvalStatus::Error));

    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0].severity, nx_api::NxSeverity::Error);
}

