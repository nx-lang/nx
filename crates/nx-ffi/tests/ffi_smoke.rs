use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use nx_api::{ComponentDispatchResult, ComponentInitResult, NxDiagnostic};
use nx_ffi::{
    nx_component_dispatch_actions, nx_component_dispatch_result_msgpack_to_json, nx_component_init,
    nx_component_init_result_msgpack_to_json, nx_diagnostics_msgpack_to_json, nx_eval_source,
    nx_ffi_abi_version, nx_free_buffer, nx_value_msgpack_to_json, NxBuffer, NxEvalStatus,
    NX_FFI_ABI_VERSION,
};
use nx_interpreter::Interpreter;
use nx_types::analyze_str;
use nx_value::NxValue;
use serde::Deserialize;

fn eval_msgpack_with_file_name(source: &str, file_name: &str) -> (NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name_bytes = file_name.as_bytes();
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let status = nx_eval_source(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name_bytes.as_ptr(),
        file_name_bytes.len(),
        &mut out as *mut NxBuffer,
    );

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, bytes)
}

fn eval_msgpack(source: &str) -> (NxEvalStatus, Vec<u8>) {
    eval_msgpack_with_file_name(source, "test.nx")
}

fn component_init_msgpack(
    source: &str,
    component_name: &str,
    props: Option<&[u8]>,
) -> (NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name = b"test.nx";
    let component_name_bytes = component_name.as_bytes();
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let (props_ptr, props_len) = props
        .map(|bytes| (bytes.as_ptr(), bytes.len()))
        .unwrap_or((std::ptr::null(), 0));

    let status = nx_component_init(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name.as_ptr(),
        file_name.len(),
        component_name_bytes.as_ptr(),
        component_name_bytes.len(),
        props_ptr,
        props_len,
        &mut out as *mut NxBuffer,
    );

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, bytes)
}

fn component_dispatch_msgpack(
    source: &str,
    state_snapshot: &[u8],
    actions_msgpack: &[u8],
) -> (NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name = b"test.nx";
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let status = nx_component_dispatch_actions(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name.as_ptr(),
        file_name.len(),
        state_snapshot.as_ptr(),
        state_snapshot.len(),
        actions_msgpack.as_ptr(),
        actions_msgpack.len(),
        &mut out as *mut NxBuffer,
    );

    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, bytes)
}

fn json_from_msgpack(
    payload: &[u8],
    converter: unsafe extern "C" fn(*const u8, usize, *mut NxBuffer) -> NxEvalStatus,
) -> (NxEvalStatus, String) {
    let mut out = NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    };

    let status = unsafe { converter(payload.as_ptr(), payload.len(), &mut out as *mut NxBuffer) };
    let bytes = unsafe { std::slice::from_raw_parts(out.ptr, out.len) }.to_vec();
    nx_free_buffer(out);
    (status, String::from_utf8(bytes).unwrap())
}

fn analyze_module(source: &str) -> std::sync::Arc<nx_hir::Module> {
    let analysis = analyze_str(source, "ffi-smoke.nx");
    assert!(
        analysis.is_ok(),
        "Expected FFI fixture analysis to succeed, got {:?}",
        analysis.diagnostics
    );
    analysis.module.expect("Expected analyzed module")
}

#[derive(Deserialize)]
struct JsonComponentInitResult {
    rendered: NxValue,
    state_snapshot: String,
}

#[derive(Deserialize)]
struct JsonComponentDispatchResult {
    effects: Vec<NxValue>,
    state_snapshot: String,
}

#[test]
fn ffi_msgpack_success_round_trip() {
    let (status, bytes) = eval_msgpack("let root() = { 42 }");
    assert!(matches!(status, NxEvalStatus::Ok));

    let decoded = NxValue::from_msgpack_slice(&bytes).unwrap();
    assert_eq!(decoded, NxValue::Int(42));
}

#[test]
fn ffi_value_msgpack_to_json_converts_success_payloads() {
    let (status, bytes) = eval_msgpack("let root() = { 42 }");
    assert!(matches!(status, NxEvalStatus::Ok));

    let (json_status, json) = json_from_msgpack(&bytes, nx_value_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    assert_eq!(json, "42");
}

#[test]
fn ffi_msgpack_error_returns_diagnostics() {
    let (status, bytes) = eval_msgpack("let x = ");
    assert!(matches!(status, NxEvalStatus::Error));

    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0].severity, nx_api::NxSeverity::Error);

    let (json_status, json) = json_from_msgpack(&bytes, nx_diagnostics_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = serde_json::from_str(&json).unwrap();
    assert!(!diagnostics.is_empty());
}

#[test]
fn ffi_static_analysis_diagnostics_preserve_file_name_and_phase_coverage() {
    let file_name = "ffi/widgets/search-box.nx";
    let source = r#"
        abstract type Entity = {
          id: int
        }

        type User extends Entity = {
          name: string
        }

        type Admin extends User = {
          level: int
        }

        let broken(): int = "oops"
        let root(): int = { 1 / 0 }
    "#;

    let (status, bytes) = eval_msgpack_with_file_name(source, file_name);
    assert!(matches!(status, NxEvalStatus::Error));

    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("lowering-error")));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
    assert!(!diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("runtime-error")));

    let labeled = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code.as_deref(),
                Some("lowering-error") | Some("return-type-mismatch")
            )
        })
        .collect::<Vec<_>>();
    assert!(!labeled.is_empty());
    assert!(labeled.iter().all(|diagnostic| diagnostic
        .labels
        .first()
        .is_some_and(|label| label.file == file_name)));

    let (json_status, json) = json_from_msgpack(&bytes, nx_diagnostics_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = serde_json::from_str(&json).unwrap();
    assert!(diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code.as_deref(),
                Some("lowering-error") | Some("return-type-mismatch")
            )
        })
        .all(|diagnostic| diagnostic
            .labels
            .first()
            .is_some_and(|label| label.file == file_name)));
}

#[test]
fn ffi_exposes_abi_version() {
    assert_eq!(nx_ffi_abi_version(), NX_FFI_ABI_VERSION);
}

#[test]
fn ffi_component_init_round_trips_state_snapshot_in_msgpack_and_debug_json() {
    let source = r#"
        component <SearchBox placeholder:string = "Find docs" /> = {
          state { query:string = {placeholder} }
          <TextInput value={query} placeholder={placeholder} />
        }
    "#;

    let props = NxValue::Record {
        type_name: None,
        properties: Default::default(),
    };
    let props_msgpack = props.to_msgpack_vec().unwrap();
    let (msgpack_status, msgpack_bytes) =
        component_init_msgpack(source, "SearchBox", Some(&props_msgpack));
    assert!(matches!(msgpack_status, NxEvalStatus::Ok));
    let init_result: ComponentInitResult = rmp_serde::from_slice(&msgpack_bytes).unwrap();
    assert!(!init_result.state_snapshot.is_empty());
    assert_eq!(
        init_result.rendered,
        NxValue::Record {
            type_name: Some("TextInput".to_string()),
            properties: std::collections::BTreeMap::from([
                (
                    "placeholder".to_string(),
                    NxValue::String("Find docs".to_string())
                ),
                (
                    "value".to_string(),
                    NxValue::String("Find docs".to_string())
                ),
            ]),
        }
    );

    let (json_status, json_payload) =
        json_from_msgpack(&msgpack_bytes, nx_component_init_result_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let init_result: JsonComponentInitResult = serde_json::from_str(&json_payload).unwrap();
    assert!(!init_result.state_snapshot.is_empty());
    assert_eq!(
        init_result.rendered,
        NxValue::Record {
            type_name: Some("TextInput".to_string()),
            properties: std::collections::BTreeMap::from([
                (
                    "placeholder".to_string(),
                    NxValue::String("Find docs".to_string())
                ),
                (
                    "value".to_string(),
                    NxValue::String("Find docs".to_string())
                ),
            ]),
        }
    );
}

#[test]
fn ffi_component_dispatch_round_trips_effect_payloads_in_msgpack_and_debug_json() {
    let source = r#"
        action SearchSubmitted = { searchString:string }
        action DoSearch = { search:string }

        component <SearchBox emits { SearchSubmitted } /> = {
          <TextInput />
        }

        let withHandler() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
    "#;

    let module = analyze_module(source);
    let interpreter = Interpreter::new();
    let props = interpreter
        .execute_function(&module, "withHandler", vec![])
        .expect("Expected props function to succeed");
    let init = interpreter
        .initialize_component(&module, "SearchBox", props)
        .expect("Expected component initialization to succeed");

    let actions = vec![NxValue::Record {
        type_name: Some("SearchSubmitted".to_string()),
        properties: std::collections::BTreeMap::from([(
            "searchString".to_string(),
            NxValue::String("docs".to_string()),
        )]),
    }];
    let actions_msgpack = rmp_serde::to_vec_named(&actions).unwrap();
    let (msgpack_status, msgpack_bytes) =
        component_dispatch_msgpack(source, &init.state_snapshot, &actions_msgpack);
    assert!(matches!(msgpack_status, NxEvalStatus::Ok));
    let dispatch_result: ComponentDispatchResult = rmp_serde::from_slice(&msgpack_bytes).unwrap();
    assert_eq!(
        dispatch_result.effects,
        vec![NxValue::Record {
            type_name: Some("DoSearch".to_string()),
            properties: std::collections::BTreeMap::from([(
                "search".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        }]
    );
    assert!(!dispatch_result.state_snapshot.is_empty());

    let (json_status, json_payload) =
        json_from_msgpack(&msgpack_bytes, nx_component_dispatch_result_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let dispatch_result: JsonComponentDispatchResult = serde_json::from_str(&json_payload).unwrap();
    assert_eq!(
        dispatch_result.effects,
        vec![NxValue::Record {
            type_name: Some("DoSearch".to_string()),
            properties: std::collections::BTreeMap::from([(
                "search".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        }]
    );
    assert_eq!(
        BASE64_STANDARD
            .decode(dispatch_result.state_snapshot)
            .unwrap(),
        init.state_snapshot
    );
}
