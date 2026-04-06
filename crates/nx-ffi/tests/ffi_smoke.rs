use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use nx_api::{
    load_program_artifact_from_source, ComponentDispatchResult, ComponentInitResult, NxDiagnostic,
    ProgramBuildContext,
};
use nx_ffi::{
    nx_build_program_artifact, nx_component_dispatch_actions_program_artifact,
    nx_component_dispatch_result_msgpack_to_json, nx_component_init_program_artifact,
    nx_component_init_result_msgpack_to_json, nx_create_library_registry,
    nx_create_program_build_context, nx_diagnostics_msgpack_to_json, nx_eval_program_artifact,
    nx_eval_source, nx_ffi_abi_version, nx_free_buffer, nx_free_library_registry,
    nx_free_program_artifact, nx_free_program_build_context, nx_load_library_into_registry,
    nx_value_msgpack_to_json, NxBuffer, NxEvalStatus, NxLibraryRegistryHandle,
    NxProgramArtifactHandle, NxProgramBuildContextHandle, NX_FFI_ABI_VERSION,
};
use nx_interpreter::Interpreter;
use nx_value::NxValue;
use serde::Deserialize;
use tempfile::TempDir;

fn empty_buffer() -> NxBuffer {
    NxBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        cap: 0,
    }
}

fn copy_and_free_buffer(buffer: NxBuffer) -> Vec<u8> {
    let bytes = if buffer.ptr.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) }.to_vec()
    };
    nx_free_buffer(buffer);
    bytes
}

fn eval_msgpack_with_file_name(source: &str, file_name: &str) -> (NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name_bytes = file_name.as_bytes();
    let mut out = empty_buffer();

    let status = nx_eval_source(
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name_bytes.as_ptr(),
        file_name_bytes.len(),
        &mut out as *mut NxBuffer,
    );

    (status, copy_and_free_buffer(out))
}

fn eval_msgpack(source: &str) -> (NxEvalStatus, Vec<u8>) {
    eval_msgpack_with_file_name(source, "test.nx")
}

fn create_library_registry() -> *mut NxLibraryRegistryHandle {
    let mut out_handle: *mut NxLibraryRegistryHandle = std::ptr::null_mut();
    let status = nx_create_library_registry(&mut out_handle as *mut *mut NxLibraryRegistryHandle);
    assert!(matches!(status, NxEvalStatus::Ok));
    assert!(!out_handle.is_null());
    out_handle
}

fn load_library_into_registry(
    registry: *mut NxLibraryRegistryHandle,
    root_path: &str,
) -> (NxEvalStatus, Vec<u8>) {
    let root_path_bytes = root_path.as_bytes();
    let mut out = empty_buffer();

    let status = nx_load_library_into_registry(
        registry as *const NxLibraryRegistryHandle,
        root_path_bytes.as_ptr(),
        root_path_bytes.len(),
        &mut out as *mut NxBuffer,
    );

    (status, copy_and_free_buffer(out))
}

fn create_program_build_context(
    registry: *mut NxLibraryRegistryHandle,
) -> *mut NxProgramBuildContextHandle {
    let mut out_handle: *mut NxProgramBuildContextHandle = std::ptr::null_mut();
    let status = nx_create_program_build_context(
        registry as *const NxLibraryRegistryHandle,
        &mut out_handle as *mut *mut NxProgramBuildContextHandle,
    );
    assert!(matches!(status, NxEvalStatus::Ok));
    assert!(!out_handle.is_null());
    out_handle
}

fn create_empty_build_context() -> *mut NxProgramBuildContextHandle {
    let registry = create_library_registry();
    let build_context = create_program_build_context(registry);
    nx_free_library_registry(registry);
    build_context
}

fn build_program_artifact_handle(
    build_context: *const NxProgramBuildContextHandle,
    source: &str,
    file_name: &str,
) -> (*mut NxProgramArtifactHandle, NxEvalStatus, Vec<u8>) {
    let source_bytes = source.as_bytes();
    let file_name_bytes = file_name.as_bytes();
    let mut out_handle: *mut NxProgramArtifactHandle = std::ptr::null_mut();
    let mut out = empty_buffer();

    let status = nx_build_program_artifact(
        build_context,
        source_bytes.as_ptr(),
        source_bytes.len(),
        file_name_bytes.as_ptr(),
        file_name_bytes.len(),
        &mut out_handle as *mut *mut NxProgramArtifactHandle,
        &mut out as *mut NxBuffer,
    );

    (out_handle, status, copy_and_free_buffer(out))
}

fn eval_msgpack_with_program_artifact(
    program_artifact: *mut NxProgramArtifactHandle,
) -> (NxEvalStatus, Vec<u8>) {
    let mut out = empty_buffer();

    let status = nx_eval_program_artifact(
        program_artifact as *const NxProgramArtifactHandle,
        &mut out as *mut NxBuffer,
    );

    (status, copy_and_free_buffer(out))
}

fn component_init_msgpack_with_program_artifact(
    program_artifact: *mut NxProgramArtifactHandle,
    component_name: &str,
    props: Option<&[u8]>,
) -> (NxEvalStatus, Vec<u8>) {
    let component_name_bytes = component_name.as_bytes();
    let mut out = empty_buffer();

    let (props_ptr, props_len) = props
        .map(|bytes| (bytes.as_ptr(), bytes.len()))
        .unwrap_or((std::ptr::null(), 0));

    let status = nx_component_init_program_artifact(
        program_artifact as *const NxProgramArtifactHandle,
        component_name_bytes.as_ptr(),
        component_name_bytes.len(),
        props_ptr,
        props_len,
        &mut out as *mut NxBuffer,
    );

    (status, copy_and_free_buffer(out))
}

fn component_dispatch_msgpack_with_program_artifact(
    program_artifact: *mut NxProgramArtifactHandle,
    state_snapshot: &[u8],
    actions_msgpack: &[u8],
) -> (NxEvalStatus, Vec<u8>) {
    let mut out = empty_buffer();

    let status = nx_component_dispatch_actions_program_artifact(
        program_artifact as *const NxProgramArtifactHandle,
        state_snapshot.as_ptr(),
        state_snapshot.len(),
        actions_msgpack.as_ptr(),
        actions_msgpack.len(),
        &mut out as *mut NxBuffer,
    );

    (status, copy_and_free_buffer(out))
}

fn json_from_msgpack(
    payload: &[u8],
    converter: unsafe extern "C" fn(*const u8, usize, *mut NxBuffer) -> NxEvalStatus,
) -> (NxEvalStatus, String) {
    let mut out = empty_buffer();
    let status = unsafe { converter(payload.as_ptr(), payload.len(), &mut out as *mut NxBuffer) };
    let json = String::from_utf8(copy_and_free_buffer(out)).unwrap();
    (status, json)
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
    assert_eq!(
        NxValue::from_msgpack_slice(&bytes).unwrap(),
        NxValue::Int(42)
    );
}

#[test]
fn ffi_registry_backed_program_build_reuses_preloaded_library() {
    let temp = TempDir::new().expect("temp dir");
    let app_root = temp.path().join("app");
    let library_root = temp.path().join("question-flow");
    std::fs::create_dir_all(&app_root).expect("app root");
    std::fs::create_dir_all(&library_root).expect("library root");
    std::fs::write(
        library_root.join("QuestionFlow.nx"),
        r#"let answer() = { 42 }"#,
    )
    .expect("library file");

    let registry = create_library_registry();
    let (load_status, load_bytes) =
        load_library_into_registry(registry, &library_root.display().to_string());
    assert!(matches!(load_status, NxEvalStatus::Ok));
    assert!(load_bytes.is_empty());

    let build_context = create_program_build_context(registry);
    let main_path = app_root.join("main.nx");
    let source = r#"import "../question-flow"
let root() = { answer() }"#;
    std::fs::write(&main_path, source).expect("main file");

    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context as *const NxProgramBuildContextHandle,
        source,
        &main_path.display().to_string(),
    );
    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    nx_free_program_build_context(build_context);
    nx_free_library_registry(registry);

    let (eval_status, eval_bytes) = eval_msgpack_with_program_artifact(program);
    nx_free_program_artifact(program);

    assert!(matches!(eval_status, NxEvalStatus::Ok));
    assert_eq!(
        NxValue::from_msgpack_slice(&eval_bytes).unwrap(),
        NxValue::Int(42)
    );
}

#[test]
fn ffi_build_program_artifact_reports_missing_library_from_context() {
    let temp = TempDir::new().expect("temp dir");
    let app_root = temp.path().join("app");
    let library_root = temp.path().join("question-flow");
    std::fs::create_dir_all(&app_root).expect("app root");
    std::fs::create_dir_all(&library_root).expect("library root");
    std::fs::write(
        library_root.join("QuestionFlow.nx"),
        r#"let answer() = { 42 }"#,
    )
    .expect("library file");

    let registry = create_library_registry();
    let build_context = create_program_build_context(registry);
    let main_path = app_root.join("main.nx");
    let source = r#"import "../question-flow"
let root() = { answer() }"#;
    std::fs::write(&main_path, source).expect("main file");

    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context as *const NxProgramBuildContextHandle,
        source,
        &main_path.display().to_string(),
    );

    assert!(program.is_null());
    assert!(matches!(build_status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&build_bytes).unwrap();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Missing loaded library")));

    nx_free_program_build_context(build_context);
    nx_free_library_registry(registry);
}

#[test]
fn ffi_load_library_into_registry_reports_module_diagnostics_with_file_context() {
    let temp = TempDir::new().expect("temp dir");
    let library_root = temp.path().join("question-flow");
    std::fs::create_dir_all(&library_root).expect("library root");
    let library_file = library_root.join("QuestionFlow.nx");
    std::fs::write(
        &library_file,
        "let answer() = { 42 }\nlet broken(): int = \"oops\"\n",
    )
    .expect("library file");

    let registry = create_library_registry();
    let (status, bytes) = load_library_into_registry(registry, &library_root.display().to_string());
    nx_free_library_registry(registry);

    assert!(matches!(status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch"))
        .expect("Expected return-type-mismatch diagnostic");
    let label = mismatch.labels.first().expect("Expected diagnostic label");
    assert_eq!(label.file, library_file.display().to_string());
}

#[test]
fn ffi_load_library_into_registry_validates_arguments() {
    let mut out = empty_buffer();
    let status = nx_load_library_into_registry(
        std::ptr::null(),
        b"/tmp".as_ptr(),
        4,
        &mut out as *mut NxBuffer,
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    let _ = copy_and_free_buffer(out);

    let mut registry = std::ptr::null_mut();
    let status = nx_create_library_registry(&mut registry as *mut *mut NxLibraryRegistryHandle);
    assert!(matches!(status, NxEvalStatus::Ok));

    let status = nx_load_library_into_registry(
        registry as *const NxLibraryRegistryHandle,
        std::ptr::null(),
        0,
        std::ptr::null_mut(),
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    nx_free_library_registry(registry);
}

#[test]
fn ffi_program_artifact_entry_points_reject_null_handles() {
    let mut out_handle: *mut NxProgramArtifactHandle = std::ptr::null_mut();
    let mut out = empty_buffer();
    let status = nx_build_program_artifact(
        std::ptr::null(),
        b"let root() = { 42 }".as_ptr(),
        "let root() = { 42 }".len(),
        b"input.nx".as_ptr(),
        "input.nx".len(),
        &mut out_handle as *mut *mut NxProgramArtifactHandle,
        &mut out as *mut NxBuffer,
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(out_handle.is_null());
    let _ = copy_and_free_buffer(out);

    let mut out = empty_buffer();
    let status = nx_eval_program_artifact(std::ptr::null(), &mut out as *mut NxBuffer);
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    let _ = copy_and_free_buffer(out);

    let mut out = empty_buffer();
    let status = nx_component_init_program_artifact(
        std::ptr::null(),
        b"SearchBox".as_ptr(),
        "SearchBox".len(),
        std::ptr::null(),
        0,
        &mut out as *mut NxBuffer,
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    let _ = copy_and_free_buffer(out);

    let mut out = empty_buffer();
    let status = nx_component_dispatch_actions_program_artifact(
        std::ptr::null(),
        std::ptr::null(),
        0,
        std::ptr::null(),
        0,
        &mut out as *mut NxBuffer,
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    let _ = copy_and_free_buffer(out);
}

#[test]
fn ffi_create_program_build_context_rejects_null_registry_handle() {
    let mut out_handle: *mut NxProgramBuildContextHandle = std::ptr::null_mut();
    let status = nx_create_program_build_context(
        std::ptr::null(),
        &mut out_handle as *mut *mut NxProgramBuildContextHandle,
    );
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(out_handle.is_null());
}

#[test]
fn ffi_component_init_with_program_artifact_reuses_preloaded_library_component() {
    let temp = TempDir::new().expect("temp dir");
    let app_root = temp.path().join("app");
    let library_root = temp.path().join("question-flow");
    std::fs::create_dir_all(&app_root).expect("app root");
    std::fs::create_dir_all(&library_root).expect("library root");
    std::fs::write(
        library_root.join("QuestionFlow.nx"),
        r#"
            action SearchSubmitted = { searchString:string }

            component <SearchBox placeholder:string = "Find docs" emits { SearchSubmitted } /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#,
    )
    .expect("library file");

    let registry = create_library_registry();
    let (load_status, load_bytes) =
        load_library_into_registry(registry, &library_root.display().to_string());
    assert!(matches!(load_status, NxEvalStatus::Ok));
    assert!(load_bytes.is_empty());
    let build_context = create_program_build_context(registry);

    let main_path = app_root.join("main.nx");
    let source = r#"import "../question-flow"
let root() = { 0 }"#;
    std::fs::write(&main_path, source).expect("main file");

    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context as *const NxProgramBuildContextHandle,
        source,
        &main_path.display().to_string(),
    );
    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());

    nx_free_program_build_context(build_context);
    nx_free_library_registry(registry);

    let props = NxValue::Record {
        type_name: None,
        properties: std::collections::BTreeMap::from([(
            "placeholder".to_string(),
            NxValue::String("From library".to_string()),
        )]),
    };
    let props_msgpack = props.to_msgpack_vec().unwrap();
    let (init_status, init_payload) =
        component_init_msgpack_with_program_artifact(program, "SearchBox", Some(&props_msgpack));
    nx_free_program_artifact(program);

    assert!(matches!(init_status, NxEvalStatus::Ok));
    let init_result: ComponentInitResult = rmp_serde::from_slice(&init_payload).unwrap();
    assert!(!init_result.state_snapshot.is_empty());
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

    let program =
        load_program_artifact_from_source(source, "ffi-dispatch.nx", &ProgramBuildContext::empty())
            .expect("Expected program artifact");
    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    let props = interpreter
        .execute_resolved_program_function("withHandler", vec![])
        .expect("Expected props function to succeed");
    let init = interpreter
        .initialize_resolved_component("SearchBox", props)
        .expect("Expected component initialization to succeed");

    let build_context = create_empty_build_context();
    let (handle, status, bytes) =
        build_program_artifact_handle(build_context, source, "ffi-dispatch.nx");
    nx_free_program_build_context(build_context);
    assert!(matches!(status, NxEvalStatus::Ok));
    assert!(bytes.is_empty());
    assert!(!handle.is_null());

    let actions = vec![NxValue::Record {
        type_name: Some("SearchSubmitted".to_string()),
        properties: std::collections::BTreeMap::from([(
            "searchString".to_string(),
            NxValue::String("docs".to_string()),
        )]),
    }];
    let actions_msgpack = rmp_serde::to_vec_named(&actions).unwrap();
    let (msgpack_status, msgpack_bytes) = component_dispatch_msgpack_with_program_artifact(
        handle,
        &init.state_snapshot,
        &actions_msgpack,
    );
    nx_free_program_artifact(handle);
    assert!(matches!(msgpack_status, NxEvalStatus::Ok));

    let dispatch_result: ComponentDispatchResult = rmp_serde::from_slice(&msgpack_bytes).unwrap();
    assert_eq!(dispatch_result.effects.len(), 1);

    let (json_status, json_payload) =
        json_from_msgpack(&msgpack_bytes, nx_component_dispatch_result_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let dispatch_result: JsonComponentDispatchResult = serde_json::from_str(&json_payload).unwrap();
    assert_eq!(dispatch_result.effects.len(), 1);
    assert_eq!(
        BASE64_STANDARD
            .decode(dispatch_result.state_snapshot)
            .unwrap(),
        init.state_snapshot
    );
}

#[test]
fn ffi_value_and_diagnostics_json_converters_round_trip() {
    let (status, bytes) = eval_msgpack("let root() = { 42 }");
    assert!(matches!(status, NxEvalStatus::Ok));

    let (json_status, json) = json_from_msgpack(&bytes, nx_value_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    assert_eq!(json, "42");

    let (status, bytes) = eval_msgpack("let x = ");
    assert!(matches!(status, NxEvalStatus::Error));
    let (json_status, json) = json_from_msgpack(&bytes, nx_diagnostics_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = serde_json::from_str(&json).unwrap();
    assert!(!diagnostics.is_empty());
}

#[test]
fn ffi_component_init_round_trips_state_snapshot_debug_json() {
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
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) =
        build_program_artifact_handle(build_context, source, "ffi-component-init.nx");
    nx_free_program_build_context(build_context);

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (msgpack_status, msgpack_bytes) =
        component_init_msgpack_with_program_artifact(program, "SearchBox", Some(&props_msgpack));
    nx_free_program_artifact(program);
    assert!(matches!(msgpack_status, NxEvalStatus::Ok));

    let init_result: ComponentInitResult = rmp_serde::from_slice(&msgpack_bytes).unwrap();
    assert!(!init_result.state_snapshot.is_empty());

    let (json_status, json_payload) =
        json_from_msgpack(&msgpack_bytes, nx_component_init_result_msgpack_to_json);
    assert!(matches!(json_status, NxEvalStatus::Ok));
    let init_result: JsonComponentInitResult = serde_json::from_str(&json_payload).unwrap();
    assert!(!init_result.state_snapshot.is_empty());
    assert!(matches!(init_result.rendered, NxValue::Record { .. }));
    assert!(!BASE64_STANDARD
        .decode(init_result.state_snapshot)
        .unwrap()
        .is_empty());
}

#[test]
fn ffi_exposes_abi_version() {
    assert_eq!(nx_ffi_abi_version(), NX_FFI_ABI_VERSION);
}
