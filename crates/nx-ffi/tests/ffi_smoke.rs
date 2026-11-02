use nx_api::codegen::read_nx_ir_bundle;
use nx_api::NxDiagnostic;
use nx_ffi::{
    nx_build_program_artifact, nx_build_workspace_program_artifact, nx_codegen_js_program_module,
    nx_codegen_nx_ir, nx_create_library_registry, nx_create_program_build_context,
    nx_ffi_abi_version, nx_free_buffer, nx_free_library_registry, nx_free_program_artifact,
    nx_free_program_build_context, nx_ir_explain, nx_load_library_into_registry,
    nx_validate_workspace, NxBuffer, NxEvalStatus, NxLibraryRegistryHandle,
    NxProgramArtifactHandle, NxProgramBuildContextHandle, NxUtf8Slice, NxWorkspaceModule,
    NX_FFI_ABI_VERSION,
};
use nx_ir::NxIrImage;
use serde::Deserialize;
use tempfile::TempDir;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsProgramModulePayload {
    source_text: String,
    logical_module_name: String,
    runtime_import_specifier: String,
    runtime_abi: String,
    program_fingerprint: u64,
    function_exports: Vec<JsProgramModuleFunctionExport>,
    component_exports: Vec<JsProgramModuleComponentExport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsProgramModuleFunctionExport {
    entrypoint_name: String,
    export_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsProgramModuleComponentExport {
    component_name: String,
    component_export_name: String,
    schema_export_name: String,
    initial_state_export_name: Option<String>,
    render_export_name: Option<String>,
}

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
    unsafe { nx_free_buffer(buffer) };
    bytes
}

fn create_library_registry() -> *mut NxLibraryRegistryHandle {
    let mut out_handle: *mut NxLibraryRegistryHandle = std::ptr::null_mut();
    let status =
        unsafe { nx_create_library_registry(&mut out_handle as *mut *mut NxLibraryRegistryHandle) };
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

    let status = unsafe {
        nx_load_library_into_registry(
            registry as *const NxLibraryRegistryHandle,
            root_path_bytes.as_ptr(),
            root_path_bytes.len(),
            &mut out as *mut NxBuffer,
        )
    };

    (status, copy_and_free_buffer(out))
}

fn create_program_build_context(
    registry: *mut NxLibraryRegistryHandle,
) -> *mut NxProgramBuildContextHandle {
    let mut out_handle: *mut NxProgramBuildContextHandle = std::ptr::null_mut();
    let status = unsafe {
        nx_create_program_build_context(
            registry as *const NxLibraryRegistryHandle,
            &mut out_handle as *mut *mut NxProgramBuildContextHandle,
        )
    };
    assert!(matches!(status, NxEvalStatus::Ok));
    assert!(!out_handle.is_null());
    out_handle
}

fn create_empty_build_context() -> *mut NxProgramBuildContextHandle {
    let registry = create_library_registry();
    let build_context = create_program_build_context(registry);
    unsafe { nx_free_library_registry(registry) };
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

    let status = unsafe {
        nx_build_program_artifact(
            build_context,
            source_bytes.as_ptr(),
            source_bytes.len(),
            file_name_bytes.as_ptr(),
            file_name_bytes.len(),
            &mut out_handle as *mut *mut NxProgramArtifactHandle,
            &mut out as *mut NxBuffer,
        )
    };

    (out_handle, status, copy_and_free_buffer(out))
}

fn workspace_descriptors(
    modules: &[(&[u8], &[u8])],
) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<NxWorkspaceModule>) {
    let identities = modules
        .iter()
        .map(|(identity, _)| identity.to_vec())
        .collect::<Vec<_>>();
    let sources = modules
        .iter()
        .map(|(_, source)| source.to_vec())
        .collect::<Vec<_>>();
    let descriptors = identities
        .iter()
        .zip(sources.iter())
        .map(|(identity, source)| NxWorkspaceModule {
            identity_ptr: identity.as_ptr(),
            identity_len: identity.len(),
            source_utf8_ptr: source.as_ptr(),
            source_utf8_len: source.len(),
            version_ptr: std::ptr::null(),
            version_len: 0,
        })
        .collect::<Vec<_>>();

    (identities, sources, descriptors)
}

fn utf8_slices(values: &[&str]) -> Vec<NxUtf8Slice> {
    values
        .iter()
        .map(|value| NxUtf8Slice {
            ptr: value.as_ptr(),
            len: value.len(),
        })
        .collect()
}

fn validate_workspace_handle(
    build_context: *const NxProgramBuildContextHandle,
    descriptors: &[NxWorkspaceModule],
) -> (NxEvalStatus, Vec<u8>) {
    validate_workspace_handle_with_implicit_imports(build_context, descriptors, &[])
}

fn validate_workspace_handle_with_implicit_imports(
    build_context: *const NxProgramBuildContextHandle,
    descriptors: &[NxWorkspaceModule],
    implicit_imports: &[&str],
) -> (NxEvalStatus, Vec<u8>) {
    let implicit_imports = utf8_slices(implicit_imports);
    let mut out = empty_buffer();
    let status = unsafe {
        nx_validate_workspace(
            build_context,
            descriptors.as_ptr(),
            descriptors.len(),
            implicit_imports.as_ptr(),
            implicit_imports.len(),
            &mut out as *mut NxBuffer,
        )
    };

    (status, copy_and_free_buffer(out))
}

fn build_workspace_artifact_handle(
    build_context: *const NxProgramBuildContextHandle,
    descriptors: &[NxWorkspaceModule],
    entry_identity: &str,
) -> (*mut NxProgramArtifactHandle, NxEvalStatus, Vec<u8>) {
    build_workspace_artifact_handle_with_implicit_imports(
        build_context,
        descriptors,
        entry_identity,
        &[],
    )
}

fn build_workspace_artifact_handle_with_implicit_imports(
    build_context: *const NxProgramBuildContextHandle,
    descriptors: &[NxWorkspaceModule],
    entry_identity: &str,
    implicit_imports: &[&str],
) -> (*mut NxProgramArtifactHandle, NxEvalStatus, Vec<u8>) {
    let entry_bytes = entry_identity.as_bytes();
    let implicit_imports = utf8_slices(implicit_imports);
    let mut out_handle: *mut NxProgramArtifactHandle = std::ptr::null_mut();
    let mut out = empty_buffer();

    let status = unsafe {
        nx_build_workspace_program_artifact(
            build_context,
            descriptors.as_ptr(),
            descriptors.len(),
            entry_bytes.as_ptr(),
            entry_bytes.len(),
            implicit_imports.as_ptr(),
            implicit_imports.len(),
            &mut out_handle as *mut *mut NxProgramArtifactHandle,
            &mut out as *mut NxBuffer,
        )
    };

    (out_handle, status, copy_and_free_buffer(out))
}

fn codegen_js_program_module(
    program_artifact: *mut NxProgramArtifactHandle,
    logical_module_name: &str,
    runtime_import_specifier: &str,
) -> (NxEvalStatus, String) {
    let logical_module_name_bytes = logical_module_name.as_bytes();
    let runtime_import_specifier_bytes = runtime_import_specifier.as_bytes();
    let mut out = empty_buffer();

    let status = unsafe {
        nx_codegen_js_program_module(
            program_artifact as *const NxProgramArtifactHandle,
            logical_module_name_bytes.as_ptr(),
            logical_module_name_bytes.len(),
            runtime_import_specifier_bytes.as_ptr(),
            runtime_import_specifier_bytes.len(),
            &mut out as *mut NxBuffer,
        )
    };

    (
        status,
        String::from_utf8(copy_and_free_buffer(out)).unwrap(),
    )
}

fn codegen_nx_ir(program_artifact: *mut NxProgramArtifactHandle) -> (NxEvalStatus, Vec<u8>) {
    codegen_nx_ir_with_options(program_artifact, "")
}

fn codegen_nx_ir_with_options(
    program_artifact: *mut NxProgramArtifactHandle,
    options: &str,
) -> (NxEvalStatus, Vec<u8>) {
    let mut out = empty_buffer();
    let status = unsafe {
        nx_codegen_nx_ir(
            program_artifact as *const NxProgramArtifactHandle,
            options.as_ptr(),
            options.len(),
            &mut out as *mut NxBuffer,
        )
    };

    (status, copy_and_free_buffer(out))
}

fn explain_nx_ir(image: &[u8]) -> (NxEvalStatus, Vec<u8>) {
    let mut out = empty_buffer();
    let status = unsafe { nx_ir_explain(image.as_ptr(), image.len(), &mut out as *mut NxBuffer) };
    (status, copy_and_free_buffer(out))
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
        r#"export let answer() = { 42 }"#,
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

    unsafe { nx_free_program_build_context(build_context) };
    unsafe { nx_free_library_registry(registry) };

    let (eval_status, eval_bytes) = ir_after_build(program);
    unsafe { nx_free_program_artifact(program) };

    assert!(matches!(eval_status, NxEvalStatus::Ok));
    assert!(!read_nx_ir_bundle(&eval_bytes).unwrap().is_empty());
}

#[test]
fn ffi_codegen_js_program_module_returns_source_and_metadata() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context,
        r#"
external component <TextInput value:string />
component <SearchBox /> = { <TextInput value="docs" /> }
let root() = { <SearchBox /> }
"#,
        "root.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (status, json) = codegen_js_program_module(program, "managed/main", "./nx-runtime.js");
    unsafe { nx_free_program_artifact(program) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let payload: JsProgramModulePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(payload.logical_module_name, "managed/main");
    assert_eq!(payload.runtime_import_specifier, "./nx-runtime.js");
    assert_eq!(payload.runtime_abi, "nx-js-runtime-v1");
    assert!(payload.program_fingerprint > 0);
    assert_eq!(payload.function_exports.len(), 1);
    assert_eq!(payload.function_exports[0].entrypoint_name, "root");
    assert_eq!(payload.function_exports[0].export_name, "root");
    assert_eq!(payload.component_exports.len(), 2);
    let search_box = payload
        .component_exports
        .iter()
        .find(|export| export.component_name == "SearchBox")
        .expect("SearchBox export metadata");
    assert_eq!(search_box.component_export_name, "SearchBox");
    assert_eq!(search_box.schema_export_name, "SearchBoxSchema");
    assert!(search_box.initial_state_export_name.is_none());
    assert!(search_box.render_export_name.is_none());
    assert!(payload.source_text.contains("from \"./nx-runtime.js\";"));
    assert!(payload.source_text.contains("export function root()"));
    assert!(!payload
        .source_text
        .contains("nx-runtime.js\";\nexport function nxElement"));
}

#[test]
fn ffi_codegen_nx_ir_returns_a_bundle_of_images_and_metadata() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) =
        build_program_artifact_handle(build_context, "let root() = { 1 + 2 }", "root.nx");
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (status, bundle) = codegen_nx_ir(program);
    let (debug_status, debug_bundle) = codegen_nx_ir_with_options(program, r#"{"debug":true}"#);
    unsafe { nx_free_program_artifact(program) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let artifacts = read_nx_ir_bundle(&bundle).expect("the payload is a bundle");
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    let image = NxIrImage::open(&artifact.bytes).expect("the image opens");

    assert_eq!(artifact.identity, "root.nx");
    assert!(!image.has_debug());
    assert_eq!(artifact.metadata.identity, "root.nx");
    assert_eq!(artifact.metadata.schema_version, 6);
    assert_eq!(artifact.metadata.runtime_abi, "nx-ir-runtime-v2");
    assert_eq!(
        artifact.metadata.fingerprint,
        image.modules().next().unwrap().fingerprint
    );
    assert_eq!(
        artifact.metadata.function_entrypoints,
        vec!["root".to_string()]
    );
    assert!(artifact.metadata.component_entrypoints.is_empty());

    assert!(matches!(debug_status, NxEvalStatus::Ok));
    let debug_artifacts = read_nx_ir_bundle(&debug_bundle).expect("the payload is a bundle");
    let debug_image = NxIrImage::open(&debug_artifacts[0].bytes).expect("the image opens");
    assert_eq!(debug_image.source(), Some("let root() = { 1 + 2 }"));

    let (status, text) = explain_nx_ir(&artifact.bytes);
    assert!(matches!(status, NxEvalStatus::Ok));
    let text = String::from_utf8(text).unwrap();
    assert!(text.contains("function root() ="), "{text}");

    let (status, payload) = explain_nx_ir(&artifact.bytes[..8]);
    assert!(matches!(status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = serde_json::from_slice(&payload).unwrap();
    assert_eq!(diagnostics[0].code.as_deref(), Some("nx-ir-malformed"));
}

/// Conditional property fragments emit the shared versioned representation.
#[test]
fn ffi_codegen_nx_ir_supports_conditional_property_fragments() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context,
        r#"
external component <Notice density:string />
let root(compact:boolean) = { <Notice if compact { density="tight" } else { density="normal" } /> }
"#,
        "root.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (status, bundle) = codegen_nx_ir(program);
    unsafe { nx_free_program_artifact(program) };
    assert!(matches!(status, NxEvalStatus::Ok));
    let artifacts = read_nx_ir_bundle(&bundle).expect("IR bundle");
    assert!(artifacts[0]
        .metadata
        .required_features
        .iter()
        .any(|feature| feature == "conditional-properties-v1"));
}

/// An action handler is an IR node, so the source the JavaScript target refuses below emits an
/// image whose explained text names the handler and whose feature list names handler support.
#[test]
fn ffi_codegen_nx_ir_carries_action_handlers() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context,
        r#"
external component <TextInput />
component <SearchBox emits { SearchSubmitted { query:string } } /> = { <TextInput /> }
action DoSearch = { query:string }
let root() = { <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> /> }
"#,
        "root.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (status, bundle) = codegen_nx_ir(program);
    unsafe { nx_free_program_artifact(program) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let artifacts = read_nx_ir_bundle(&bundle).expect("the payload is a bundle");
    assert_eq!(
        artifacts[0].metadata.required_features,
        vec!["action-handlers-v1".to_string()]
    );
    let (status, text) = explain_nx_ir(&artifacts[0].bytes);
    assert!(matches!(status, NxEvalStatus::Ok));
    let text = String::from_utf8(text).unwrap();
    assert!(
        text.contains(
            "onSearchSubmitted=handler SearchBox.SearchSubmitted action@0:SearchBox.SearchSubmitted =>"
        ),
        "{text}"
    );
}

#[test]
fn ffi_codegen_js_program_module_returns_json_diagnostics_for_codegen_errors() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) = build_program_artifact_handle(
        build_context,
        r#"
external component <TextInput />
component <SearchBox emits { SearchSubmitted { query:string } } /> = { <TextInput /> }
action DoSearch = { query:string }
let root() = { <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> /> }
"#,
        "root.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (status, json) = codegen_js_program_module(program, "", "");
    unsafe { nx_free_program_artifact(program) };

    assert!(matches!(status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = serde_json::from_str(&json).unwrap();
    assert!(diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("action-handler codegen is not supported")));
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
    assert!(diagnostics.iter().any(|diagnostic| diagnostic
        .message
        .contains("Missing workspace module or loaded library")));

    unsafe { nx_free_program_build_context(build_context) };
    unsafe { nx_free_library_registry(registry) };
}

#[test]
fn ffi_validate_workspace_returns_ok_with_diagnostics_payload() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) =
        workspace_descriptors(&[(b"main.nx", br#"let broken(): int = { "oops" }"#)]);

    let (status, bytes) = validate_workspace_handle(build_context, &descriptors);
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
}

#[test]
fn ffi_validate_workspace_returns_empty_diagnostics_payload_for_valid_workspace() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) =
        workspace_descriptors(&[(b"main.nx", br#"let root(): int = { 42 }"#)]);

    let (status, bytes) = validate_workspace_handle(build_context, &descriptors);
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics.is_empty());
}

#[test]
fn ffi_workspace_calls_accept_implicit_imports() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) = workspace_descriptors(&[
        (
            b"drawnui.nx",
            b"export external component <SkiaLabel Text:string />",
        ),
        (b"input.nx", b"let root() = <SkiaLabel Text=\"hi\" />"),
    ]);

    let (status, bytes) = validate_workspace_handle_with_implicit_imports(
        build_context,
        &descriptors,
        &["drawnui.nx"],
    );
    assert!(matches!(status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let (handle, status, bytes) = build_workspace_artifact_handle_with_implicit_imports(
        build_context,
        &descriptors,
        "input.nx",
        &["drawnui.nx"],
    );
    assert!(matches!(status, NxEvalStatus::Ok), "{bytes:?}");
    assert!(!handle.is_null());
    unsafe { nx_free_program_artifact(handle) };

    let (status, bytes) = validate_workspace_handle_with_implicit_imports(
        build_context,
        &descriptors,
        &["missing.nx"],
    );
    assert!(matches!(status, NxEvalStatus::Ok));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("implicit-import-not-found")),
        "{diagnostics:?}"
    );
    unsafe { nx_free_program_build_context(build_context) };
}

#[test]
fn ffi_workspace_module_version_reaches_the_module_table() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, mut descriptors) = workspace_descriptors(&[
        (
            b"drawnui.nx",
            b"export external component <SkiaLabel Text:string />",
        ),
        (b"input.nx", b"let root() = <SkiaLabel Text=\"hi\" />"),
    ]);
    let version = b"9";
    descriptors[0].version_ptr = version.as_ptr();
    descriptors[0].version_len = version.len();

    let (handle, status, bytes) = build_workspace_artifact_handle_with_implicit_imports(
        build_context,
        &descriptors,
        "input.nx",
        &["drawnui.nx"],
    );
    unsafe { nx_free_program_build_context(build_context) };
    assert!(matches!(status, NxEvalStatus::Ok), "{bytes:?}");

    let (status, bundle) = codegen_nx_ir(handle);
    let (versions_status, versions_bytes) =
        codegen_nx_ir_with_options(handle, r#"{"versions":{"drawnui.nx":"9"}}"#);
    unsafe { nx_free_program_artifact(handle) };

    assert!(matches!(status, NxEvalStatus::Ok));
    let artifacts = read_nx_ir_bundle(&bundle).expect("the payload is a bundle");
    let image = NxIrImage::open(&artifacts[0].bytes).expect("the image opens");
    let modules = image
        .modules()
        .map(|module| (module.identity, module.version))
        .collect::<Vec<_>>();
    assert_eq!(modules, vec![("input.nx", ""), ("drawnui.nx", "9")]);

    // Versions are not an emit option any more; passing them is an error, not a silent no-op.
    assert!(matches!(versions_status, NxEvalStatus::InvalidArgument));
    assert!(versions_bytes.is_empty());
}

#[test]
fn ffi_validate_workspace_rejects_null_module_array_with_count() {
    let build_context = create_empty_build_context();
    let mut out = empty_buffer();

    let status = unsafe {
        nx_validate_workspace(
            build_context as *const NxProgramBuildContextHandle,
            std::ptr::null(),
            1,
            std::ptr::null(),
            0,
            &mut out as *mut NxBuffer,
        )
    };
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(status, NxEvalStatus::InvalidArgument));
}

#[test]
fn ffi_validate_workspace_rejects_non_empty_null_module_fields() {
    let build_context = create_empty_build_context();
    let identity = b"main.nx";
    let source = br#"let root(): int = { 42 }"#;

    let null_identity = NxWorkspaceModule {
        identity_ptr: std::ptr::null(),
        identity_len: identity.len(),
        source_utf8_ptr: source.as_ptr(),
        source_utf8_len: source.len(),
        version_ptr: std::ptr::null(),
        version_len: 0,
    };
    let (identity_status, identity_bytes) =
        validate_workspace_handle(build_context, &[null_identity]);

    let null_source = NxWorkspaceModule {
        identity_ptr: identity.as_ptr(),
        identity_len: identity.len(),
        source_utf8_ptr: std::ptr::null(),
        source_utf8_len: source.len(),
        version_ptr: std::ptr::null(),
        version_len: 0,
    };
    let (source_status, source_bytes) = validate_workspace_handle(build_context, &[null_source]);
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(identity_status, NxEvalStatus::InvalidArgument));
    assert!(identity_bytes.is_empty());
    assert!(matches!(source_status, NxEvalStatus::InvalidArgument));
    assert!(source_bytes.is_empty());
}

#[test]
fn ffi_validate_workspace_rejects_invalid_utf8() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) = workspace_descriptors(&[(b"main.nx", &[0xff])]);

    let (status, bytes) = validate_workspace_handle(build_context, &descriptors);
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(bytes.is_empty());
}

#[test]
fn ffi_validate_workspace_rejects_duplicate_normalized_identities() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) = workspace_descriptors(&[
        (b"shared/value.nx", br#"let root() = { 1 }"#),
        (b"shared/./value.nx", br#"let root() = { 2 }"#),
    ]);

    let (status, bytes) = validate_workspace_handle(build_context, &descriptors);
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(bytes.is_empty());
}

#[test]
fn ffi_build_workspace_program_artifact_reports_missing_entry_identity() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) =
        workspace_descriptors(&[(b"main.nx", br#"let root(): int = { 42 }"#)]);

    let (program, status, bytes) = build_workspace_artifact_handle(
        build_context as *const NxProgramBuildContextHandle,
        &descriptors,
        "missing.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(program.is_null());
    assert!(matches!(status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("workspace-entry-not-found")));
}

#[test]
fn ffi_build_workspace_program_artifact_emits_after_buffers_are_dropped() {
    let build_context = create_empty_build_context();
    let (program, build_status, build_bytes) = {
        let (_identities, _sources, descriptors) = workspace_descriptors(&[
            (
                b"app/main.nx",
                br#"import { answer } from "../shared/value.nx"
let root(): int = { answer() }"#,
            ),
            (b"shared/value.nx", br#"export let answer(): int = { 42 }"#),
        ]);
        build_workspace_artifact_handle(
            build_context as *const NxProgramBuildContextHandle,
            &descriptors,
            "app/main.nx",
        )
    };
    unsafe { nx_free_program_build_context(build_context) };

    assert!(matches!(build_status, NxEvalStatus::Ok));
    assert!(build_bytes.is_empty());
    assert!(!program.is_null());

    let (eval_status, eval_bytes) = ir_after_build(program);
    unsafe { nx_free_program_artifact(program) };
    assert!(matches!(eval_status, NxEvalStatus::Ok));
    assert!(!read_nx_ir_bundle(&eval_bytes).unwrap().is_empty());
}

#[test]
fn ffi_build_workspace_program_artifact_returns_static_diagnostics() {
    let build_context = create_empty_build_context();
    let (_identities, _sources, descriptors) =
        workspace_descriptors(&[(b"main.nx", br#"let root(): int = { "oops" }"#)]);

    let (program, status, bytes) = build_workspace_artifact_handle(
        build_context as *const NxProgramBuildContextHandle,
        &descriptors,
        "main.nx",
    );
    unsafe { nx_free_program_build_context(build_context) };

    assert!(program.is_null());
    assert!(matches!(status, NxEvalStatus::Error));
    let diagnostics: Vec<NxDiagnostic> = rmp_serde::from_slice(&bytes).unwrap();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
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
    unsafe { nx_free_library_registry(registry) };

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
    let status = unsafe {
        nx_load_library_into_registry(
            std::ptr::null(),
            b"/tmp".as_ptr(),
            4,
            &mut out as *mut NxBuffer,
        )
    };
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    let _ = copy_and_free_buffer(out);

    let mut registry = std::ptr::null_mut();
    let status =
        unsafe { nx_create_library_registry(&mut registry as *mut *mut NxLibraryRegistryHandle) };
    assert!(matches!(status, NxEvalStatus::Ok));

    let status = unsafe {
        nx_load_library_into_registry(
            registry as *const NxLibraryRegistryHandle,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        )
    };
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    unsafe { nx_free_library_registry(registry) };
}

#[test]
fn ffi_program_artifact_entry_points_reject_null_handles() {
    let mut out_handle: *mut NxProgramArtifactHandle = std::ptr::null_mut();
    let mut out = empty_buffer();
    let status = unsafe {
        nx_build_program_artifact(
            std::ptr::null(),
            b"let root() = { 42 }".as_ptr(),
            "let root() = { 42 }".len(),
            b"input.nx".as_ptr(),
            "input.nx".len(),
            &mut out_handle as *mut *mut NxProgramArtifactHandle,
            &mut out as *mut NxBuffer,
        )
    };
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(out_handle.is_null());
    let _ = copy_and_free_buffer(out);
}

#[test]
fn ffi_create_program_build_context_rejects_null_registry_handle() {
    let mut out_handle: *mut NxProgramBuildContextHandle = std::ptr::null_mut();
    let status = unsafe {
        nx_create_program_build_context(
            std::ptr::null(),
            &mut out_handle as *mut *mut NxProgramBuildContextHandle,
        )
    };
    assert!(matches!(status, NxEvalStatus::InvalidArgument));
    assert!(out_handle.is_null());
}

#[test]
fn ffi_exposes_abi_version() {
    assert_eq!(nx_ffi_abi_version(), NX_FFI_ABI_VERSION);
}

fn ir_after_build(program: *mut NxProgramArtifactHandle) -> (NxEvalStatus, Vec<u8>) {
    let options = b"{\"modules\":[]}";
    let mut out = empty_buffer();
    let status = unsafe { nx_codegen_nx_ir(program, options.as_ptr(), options.len(), &mut out) };
    (status, copy_and_free_buffer(out))
}
