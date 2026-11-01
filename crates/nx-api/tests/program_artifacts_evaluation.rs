use nx_api::*;
use nx_diagnostics::{Diagnostic, Severity};
use nx_value::NxValue;
use std::{fs, path::Path, sync::Arc};
use tempfile::TempDir;
fn workspace_module(identity: &str, source: impl Into<Vec<u8>>) -> NxWorkspaceModule {
    NxWorkspaceModule::from_utf8(identity, source.into()).expect("workspace module")
}

fn workspace(modules: Vec<NxWorkspaceModule>) -> NxWorkspace {
    NxWorkspace::new(modules).expect("workspace")
}

/// The program's libraries other than the prelude, which is part of every program.
fn host_libraries(artifact: &ProgramArtifact) -> Vec<&Arc<LibraryArtifact>> {
    artifact
        .libraries
        .iter()
        .filter(|library| library.root_path != Path::new(nx_hir::PRELUDE_MODULE_IDENTITY))
        .collect()
}

/// A property union reached through a workspace import carries its inherited cases: the peer
/// module's unions are completed before its items are bound into the importer. The derived
/// declarations also resolve at runtime, so `changed` on the imported update record orders by
/// the target's effective shape.
#[test]
fn property_union_of_a_workspace_import_includes_inherited_fields() {
    let workspace = workspace(vec![
        workspace_module(
            "data.nx",
            r#"export abstract type Named = { name:string }
export type User extends Named = { email?:string }"#,
        ),
        workspace_module(
            "main.nx",
            r#"import { User } from "./data.nx"
let root() = <Box a={User.Property.name} b={User.Property.email} keys={changed(<User.Update email={} name="Ada" />)} />"#,
        ),
    ]);
    let artifact =
        build_workspace_program_artifact(&workspace, "main.nx", &ProgramBuildContext::empty())
            .expect("Expected workspace artifact");
    assert!(
        !has_error_diagnostics(&artifact.diagnostics),
        "Expected the imported property union to carry its inherited case: {:?}",
        artifact.diagnostics
    );

    let value = match eval_program_artifact(&artifact) {
        EvalResult::Ok(value) => value,
        EvalResult::Err(diagnostics) => panic!("interpreter diagnostics: {:?}", diagnostics),
    };
    let NxValue::Record { properties, .. } = value else {
        panic!("Expected a record value");
    };
    let bare = |name: &str| NxValue::String(name.to_string());
    assert_eq!(properties.get("a"), Some(&bare("name")));
    assert_eq!(properties.get("b"), Some(&bare("email")));
    assert_eq!(
        properties.get("keys"),
        Some(&NxValue::Array(vec![bare("name"), bare("email")]))
    );
}

/// A generic record crosses a library boundary with its type parameters, so an importing
/// module can apply it, construct it, read a substituted field, and take one from a function.
#[test]
fn a_generic_record_is_applied_and_constructed_across_a_library_boundary() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        ui_dir.join("ranges.nx"),
        "export type Range = { T:type start:T end:T }\n\
         export let unit(): <Range T=int/> = {<Range T=int start={0} end={1} />}\n",
    )
    .expect("ranges file");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected ui registry load");
    let build_context = registry.build_context();

    let main_path = app_dir.join("main.nx");
    let source = "import \"../ui\"\n\
         let r:<Range T=int/> = <Range T=int start={1} end={5} />\n\
         let s:int = {r.start}\n\
         let u:<Range T=int/> = {unit()}\n\
         let root() = { s }";
    fs::write(&main_path, source).expect("main file");

    let artifact = build_program_artifact_from_source(
        source,
        &main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected program artifact");
    assert!(
        !has_error_diagnostics(&artifact.diagnostics),
        "expected a clean build, got: {:?}",
        artifact
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message().to_string())
            .collect::<Vec<_>>()
    );
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected the program to evaluate");
    };
    assert_eq!(value, nx_value::NxValue::Int(1));
}

#[test]
fn consumer_imports_only_explicit_exports() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(ui_dir.join("helpers.nx"), r#"let helper(): int = { 42 }"#).expect("helpers file");
    fs::write(
        ui_dir.join("public.nx"),
        r#"export let answer(): int = { helper() }"#,
    )
    .expect("public file");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected ui registry load");
    let build_context = registry.build_context();

    let visible_main_path = app_dir.join("visible-main.nx");
    let visible_source = r#"import "../ui"
let root() = { answer() }"#;
    fs::write(&visible_main_path, visible_source).expect("visible main file");

    let visible_artifact = build_program_artifact_from_source(
        visible_source,
        &visible_main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected visible program artifact");

    assert!(
        !has_error_diagnostics(&visible_artifact.diagnostics),
        "Expected explicit exports to remain visible to consumers"
    );

    let EvalResult::Ok(value) = eval_program_artifact(&visible_artifact) else {
        panic!("Expected explicit export program artifact evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));

    let hidden_main_path = app_dir.join("hidden-main.nx");
    let hidden_source = r#"import { helper } from "../ui"
let root() = { helper() }"#;
    fs::write(&hidden_main_path, hidden_source).expect("hidden main file");

    let hidden_artifact = build_program_artifact_from_source(
        hidden_source,
        &hidden_main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected hidden program artifact with diagnostics");

    assert!(hidden_artifact
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.message().contains("does not export 'helper'") }));
}

#[test]
fn program_artifact_uses_registry_backed_build_context() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        ui_dir.join("answer.nx"),
        r#"export let answer(): int = { 42 }"#,
    )
    .expect("ui file");
    let registry = LibraryRegistry::new();
    let ui_snapshot = registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected registry load");
    let build_context = registry.build_context();

    let main_path = app_dir.join("main.nx");
    let source = r#"import "../ui"
let root() = { answer() }"#;
    fs::write(&main_path, source).expect("main file");

    let artifact = build_program_artifact_from_source(
        source,
        &main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected program artifact");

    let libraries = host_libraries(&artifact);
    assert_eq!(libraries.len(), 1);
    assert!(Arc::ptr_eq(libraries[0], &ui_snapshot));
    assert!(
        artifact.root_modules[0]
            .lowered_module
            .as_ref()
            .expect("Expected preserved root module")
            .find_item("answer")
            .is_none(),
        "Expected preserved root module to remain file-scoped"
    );

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected registry-backed program artifact evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

#[test]
fn imported_content_bindings_match_local_behavior() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        ui_dir.join("content.nx"),
        r#"export type Wrapper = { content body:object }
export let Wrap(content body:object): object = { body }
export component <Panel content body:object /> = {
  <section>{body}</section>
}"#,
    )
    .expect("content file");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected ui registry load");
    let build_context = registry.build_context();

    let main_path = app_dir.join("main.nx");
    let source = r#"import { Wrapper, Wrap, Panel } from "../ui"
let root(): object+ = {
  <Wrap><Badge /></Wrap>
  <Wrapper><Badge /></Wrapper>
  <Panel><Badge /></Panel>
}"#;
    fs::write(&main_path, source).expect("main file");

    let artifact = build_program_artifact_from_source(
        source,
        &main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected program artifact");

    assert!(
        !has_error_diagnostics(&artifact.diagnostics),
        "Expected imported content bindings to analyze without errors, got {:?}",
        artifact.diagnostics
    );

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected imported content bindings evaluation to succeed");
    };

    let badge = nx_value::NxValue::Record {
        type_name: Some("Badge".to_string()),
        properties: std::collections::BTreeMap::new(),
    };
    let wrapper = nx_value::NxValue::Record {
        type_name: Some("Wrapper".to_string()),
        properties: std::collections::BTreeMap::from([("body".to_string(), badge.clone())]),
    };
    let panel = nx_value::NxValue::Record {
        type_name: Some("Panel".to_string()),
        properties: std::collections::BTreeMap::from([("body".to_string(), badge.clone())]),
    };
    assert_eq!(value, nx_value::NxValue::Array(vec![badge, wrapper, panel]));
}

#[test]
fn program_artifact_survives_build_context_drop() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        ui_dir.join("answer.nx"),
        r#"export let answer(): int = { 42 }"#,
    )
    .expect("ui file");
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected registry load");
    let build_context = registry.build_context();

    let main_path = app_dir.join("main.nx");
    let source = r#"import "../ui"
let root() = { answer() }"#;
    fs::write(&main_path, source).expect("main file");

    let artifact = build_program_artifact_from_source(
        source,
        &main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected program artifact");
    drop(build_context);

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected program artifact to remain executable after context release");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

#[test]
fn build_context_with_visible_roots_includes_transitive_dependencies_of_visible_library() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let widgets_dir = temp.path().join("widgets");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&widgets_dir).expect("widgets dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        ui_dir.join("answer.nx"),
        r#"export let ui_answer(): int = { 42 }"#,
    )
    .expect("ui file");
    fs::write(
        widgets_dir.join("answer.nx"),
        r#"import "../ui"
export let answer(): int = { ui_answer() }"#,
    )
    .expect("widgets file");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&widgets_dir)
        .expect("Expected widgets registry load");
    let build_context = registry
        .build_context_with_visible_roots([&widgets_dir])
        .expect("Expected filtered build context");

    let main_path = app_dir.join("main.nx");
    let source = r#"import "../widgets"
let root() = { answer() }"#;
    fs::write(&main_path, source).expect("main file");

    let artifact = build_program_artifact_from_source(
        source,
        &main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected program artifact");

    assert!(
        !has_error_diagnostics(&artifact.diagnostics),
        "Expected transitive dependency closure to be selected for the visible library"
    );

    let widget_root = fs::canonicalize(&widgets_dir).expect("canonical widgets");
    let ui_root = fs::canonicalize(&ui_dir).expect("canonical ui");
    let library_roots = artifact
        .libraries
        .iter()
        .map(|library| library.root_path.clone())
        .collect::<Vec<_>>();
    assert!(
        library_roots.contains(&widget_root),
        "Expected selected libraries to include the visible library"
    );
    assert!(
        library_roots.contains(&ui_root),
        "Expected selected libraries to include the visible library's transitive dependency"
    );

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected transitive dependency program artifact evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

#[test]
fn build_context_with_visible_roots_limits_visible_libraries() {
    let temp = TempDir::new().expect("temp dir");
    let app_dir = temp.path().join("app");
    let ui_dir = temp.path().join("ui");
    let admin_dir = temp.path().join("admin");
    fs::create_dir_all(&app_dir).expect("app dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");
    fs::create_dir_all(&admin_dir).expect("admin dir");

    fs::write(
        ui_dir.join("answer.nx"),
        r#"export let answer(): int = { 42 }"#,
    )
    .expect("ui file");
    fs::write(
        admin_dir.join("secret.nx"),
        r#"export let secret(): int = { 7 }"#,
    )
    .expect("admin file");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&ui_dir)
        .expect("Expected ui registry load");
    registry
        .load_library_from_directory(&admin_dir)
        .expect("Expected admin registry load");
    let build_context = registry
        .build_context_with_visible_roots([&ui_dir])
        .expect("Expected filtered build context");

    let visible_main_path = app_dir.join("visible-main.nx");
    let visible_source = r#"import "../ui"
let root() = { answer() }"#;
    fs::write(&visible_main_path, visible_source).expect("visible main file");

    let visible_artifact = build_program_artifact_from_source(
        visible_source,
        &visible_main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected visible program artifact");

    assert!(
        !has_error_diagnostics(&visible_artifact.diagnostics),
        "Expected visible library import to succeed"
    );

    let EvalResult::Ok(value) = eval_program_artifact(&visible_artifact) else {
        panic!("Expected visible library program artifact evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));

    let hidden_main_path = app_dir.join("hidden-main.nx");
    let hidden_source = r#"import "../admin"
let root() = { secret() }"#;
    fs::write(&hidden_main_path, hidden_source).expect("hidden main file");

    let hidden_artifact = build_program_artifact_from_source(
        hidden_source,
        &hidden_main_path.display().to_string(),
        &build_context,
    )
    .expect("Expected hidden program artifact with diagnostics");

    assert!(
        host_libraries(&hidden_artifact).is_empty(),
        "Expected hidden libraries to stay out of the selected program library set"
    );
    assert!(hidden_artifact
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic
            .message()
            .contains("Missing workspace module or loaded library")));
}

/// The type argument is consumed by the checker, so the evaluated record and its JSON carry
/// the prop and nothing for the parameter.
#[test]
fn an_evaluated_generic_component_record_serializes_without_its_type_argument() {
    let artifact = build_program_artifact_from_source(
        "type Contact = { name:string }\n\
         external component <SkiaLayout TItem:type itemsSource?:TItem+ />\n\
         let root() = { <SkiaLayout TItem=Contact itemsSource={<Contact name=\"Ada\" />} /> }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .expect("program artifact should build");

    let json = match eval_program_artifact(&artifact) {
        EvalResult::Ok(value) => value.to_json_string().expect("json"),
        EvalResult::Err(diagnostics) => panic!("interpreter diagnostics: {diagnostics:?}"),
    };
    assert!(json.contains("itemsSource"), "got: {json}");
    assert!(!json.contains("TItem"), "got: {json}");
}

/// A bare case of a union the using module cannot name must reach the runtime.
///
/// The bare and qualified spellings differ only in whether the union was imported, which is a
/// fact about the use site's namespace, not about the value. Evaluating both is the end-to-end
/// check that the spelling is not observable below type checking.
#[test]
fn a_bare_case_of_an_unimported_union_evaluates_as_the_qualified_form_does() {
    fn eval_app(app: &str) -> nx_value::NxValue {
        let workspace = workspace(vec![
            workspace_module("app.nx", app.as_bytes().to_vec()),
            workspace_module(
                "widgets.nx",
                br#"export type Fit = fill | contain | cover
export let <Img fit: Fit = {Fit.fill} /> = <div fit={fit} />"#
                    .to_vec(),
            ),
        ]);
        let artifact =
            build_workspace_program_artifact(&workspace, "app.nx", &ProgramBuildContext::empty())
                .unwrap_or_else(|diagnostics| {
                    panic!("workspace artifact: {diagnostics:?}");
                });
        let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
            panic!("expected evaluation to succeed");
        };
        value
    }

    let bare = eval_app(
        r#"import { Img } from "./widgets.nx"
let root() = { <Img fit=cover /> }"#,
    );
    let qualified = eval_app(
        r#"import { Img, Fit } from "./widgets.nx"
let root() = { <Img fit={Fit.cover} /> }"#,
    );

    assert_eq!(
        bare, qualified,
        "importing the union changes what the author may write, not what the value is"
    );
}

/// A visible binding whose name matches must not capture a reference to another definition.
///
/// The consumer declares its own `Fit`, so the name is bound here — to a different union. The
/// bare case resolved against the widgets `Fit`, and it is that declaration the value must come
/// from, at type checking and at evaluation alike.
#[test]
fn a_visible_name_does_not_capture_a_reference_to_a_different_definition() {
    let workspace = workspace(vec![
        workspace_module(
            "app.nx",
            br#"import { Img } from "./widgets.nx"
type Fit = cover | squish
let root() = { <Img fit=cover /> }"#
                .to_vec(),
        ),
        workspace_module(
            "widgets.nx",
            br#"export type Fit = fill | contain | cover
export let <Img fit: Fit = {Fit.fill} /> = <div fit={fit} />"#
                .to_vec(),
        ),
    ]);

    let artifact =
        build_workspace_program_artifact(&workspace, "app.nx", &ProgramBuildContext::empty())
            .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };

    assert_eq!(
        value,
        nx_value::NxValue::Record {
            type_name: Some("div".to_string()),
            properties: std::collections::BTreeMap::from([(
                "fit".to_string(),
                nx_value::NxValue::String("cover".to_string()),
            )]),
        }
    );
}

/// A foreign case that inherits an abstract base is constructed with the base's fields.
///
/// The base is resolved in the declaring module, which is the only module that can see it —
/// the consumer imports neither the union nor the base.
#[test]
fn a_foreign_case_inheriting_an_abstract_base_carries_its_base_fields() {
    let workspace = workspace(vec![
        workspace_module(
            "app.nx",
            br#"import { Draw } from "./widgets.nx"
let root() = { <Draw shape=circle /> }"#
                .to_vec(),
        ),
        workspace_module(
            "widgets.nx",
            br#"export abstract type Drawable = { ink: string = "black" }
export type Shape extends Drawable =
  | circle
  | square
export let <Draw shape: Shape = {Shape.circle} /> = <div ink={shape.ink} />"#
                .to_vec(),
        ),
    ]);

    let artifact =
        build_workspace_program_artifact(&workspace, "app.nx", &ProgramBuildContext::empty())
            .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };

    assert_eq!(
        value,
        nx_value::NxValue::Record {
            type_name: Some("div".to_string()),
            properties: std::collections::BTreeMap::from([(
                "ink".to_string(),
                nx_value::NxValue::String("black".to_string()),
            )]),
        }
    );
}

#[test]
fn eval_workspace_artifact_uses_selected_entry_root() {
    let workspace = workspace(vec![
        workspace_module("a.nx", br#"let root() = { "a" }"#.to_vec()),
        workspace_module("b.nx", br#"let root() = { "b" }"#.to_vec()),
    ]);

    let artifact =
        build_workspace_program_artifact(&workspace, "b.nx", &ProgramBuildContext::empty())
            .expect("workspace artifact");

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected selected root evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::String("b".to_string()));
}

#[test]
fn eval_workspace_artifact_reports_no_root_for_selected_entry() {
    let workspace = workspace(vec![
        workspace_module("entry.nx", br#"let helper() = { 1 }"#.to_vec()),
        workspace_module("other.nx", br#"let root() = { 2 }"#.to_vec()),
    ]);

    let artifact =
        build_workspace_program_artifact(&workspace, "entry.nx", &ProgramBuildContext::empty())
            .expect("workspace artifact");

    let EvalResult::Err(diagnostics) = eval_program_artifact(&artifact) else {
        panic!("Expected missing root diagnostic");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_deref() == Some("no-root")
            && diagnostic
                .labels
                .iter()
                .any(|label| label.file == "entry.nx" && label.span.end_column > 1)
    }));
}

#[test]
fn eval_workspace_artifact_executes_across_workspace_imports() {
    let workspace = workspace(vec![
        workspace_module(
            "app/main.nx",
            br#"import { answer } from "../shared/value.nx"
let root(): int = { answer() }"#
                .to_vec(),
        ),
        workspace_module(
            "shared/value.nx",
            br#"export let answer(): int = { 42 }"#.to_vec(),
        ),
    ]);

    let artifact =
        build_workspace_program_artifact(&workspace, "app/main.nx", &ProgramBuildContext::empty())
            .expect("workspace artifact");

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected workspace import evaluation to succeed");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

#[test]
fn workspace_program_artifact_survives_workspace_drop() {
    let artifact = {
        let workspace = workspace(vec![workspace_module(
            "main.nx",
            br#"let root() = { 42 }"#.to_vec(),
        )]);
        build_workspace_program_artifact(&workspace, "main.nx", &ProgramBuildContext::empty())
            .expect("workspace artifact")
    };

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected artifact evaluation to succeed after workspace drop");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

#[test]
fn workspace_program_artifact_survives_build_context_drop() {
    let temp = TempDir::new().expect("temp dir");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&ui_dir).expect("ui dir");
    fs::write(
        ui_dir.join("answer.nx"),
        r#"export let answer(): int = { 42 }"#,
    )
    .expect("ui file");

    let artifact = {
        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("library preload");
        let build_context = registry.build_context();
        let workspace = workspace(vec![workspace_module(
            "app/main.nx",
            br#"import { answer } from "../ui"
let root(): int = { answer() }"#
                .to_vec(),
        )]);
        build_workspace_program_artifact(&workspace, "app/main.nx", &build_context)
            .expect("workspace artifact")
    };

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("Expected artifact evaluation to succeed after build context drop");
    };
    assert_eq!(value, nx_value::NxValue::Int(42));
}

fn catalog_source() -> &'static [u8] {
    b"export external component <SkiaLabel Text:string FontSize:int = 14 />\n"
}

fn implicit_context() -> ProgramBuildContext {
    ProgramBuildContext::empty().with_implicit_imports(["drawnui.nx"])
}

#[test]
fn implicit_import_puts_a_control_in_scope_without_an_import_line() {
    let workspace = workspace(vec![
        workspace_module("drawnui.nx", catalog_source().to_vec()),
        workspace_module(
            "input.nx",
            b"let root() = <SkiaLabel Text=\"hi\" />\n".to_vec(),
        ),
    ]);

    let artifact = build_workspace_program_artifact(&workspace, "input.nx", &implicit_context())
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));

    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };
    assert!(
        matches!(&value, NxValue::Record { type_name: Some(name), .. } if name == "SkiaLabel"),
        "{value:?}"
    );
}

/// A document that declares no `root` and is one element expression is accepted exactly as the
/// same text with a written wildcard import would be.
#[test]
fn a_document_that_is_one_trailing_element_compiles_with_an_implicit_import() {
    let workspace = workspace(vec![
        workspace_module("drawnui.nx", catalog_source().to_vec()),
        workspace_module("input.nx", b"<SkiaLabel Text=\"hi\" />\n".to_vec()),
    ]);

    let artifact = build_workspace_program_artifact(&workspace, "input.nx", &implicit_context())
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));

    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };
    assert!(
        matches!(&value, NxValue::Record { type_name: Some(name), .. } if name == "SkiaLabel"),
        "{value:?}"
    );
}

/// A catalog of three names — two controls and a function — so a test can import one of them
/// explicitly and ask what became of the others.
fn two_name_catalog_source() -> &'static [u8] {
    b"export external component <SkiaLabel Text:string />\n\
      export external component <SkiaButton Text:string />\n\
      export let hint(): string = { \"tap\" }\n"
}

/// A written import of the implicitly imported module is not a repeated import of it: the
/// synthesized wildcard still joins it rather than tripping the one-import-per-module rule,
/// so naming one declaration does not hide the rest. The name the author imported is bound
/// once, by the written import, and the wildcard binds only what it did not name.
#[test]
fn a_named_import_of_the_implicit_module_leaves_its_other_names_in_scope() {
    let workspace = workspace(vec![
        workspace_module("drawnui.nx", two_name_catalog_source().to_vec()),
        workspace_module(
            "input.nx",
            b"import { SkiaLabel } from \"./drawnui.nx\"\nlet root() = <SkiaButton Text={hint()} />\n"
                .to_vec(),
        ),
    ]);

    let artifact = build_workspace_program_artifact(&workspace, "input.nx", &implicit_context())
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };
    assert!(
        matches!(&value, NxValue::Record { type_name: Some(name), .. } if name == "SkiaButton"),
        "{value:?}"
    );
}

/// The same for an aliased wildcard, which puts the names behind a qualifier.
#[test]
fn an_aliased_wildcard_import_of_the_implicit_module_leaves_its_other_names_in_scope() {
    let workspace = workspace(vec![
        workspace_module("drawnui.nx", two_name_catalog_source().to_vec()),
        workspace_module(
            "input.nx",
            b"import \"./drawnui.nx\" as Ui\nlet root() = <SkiaButton Text={hint()} />\n".to_vec(),
        ),
    ]);

    let artifact = build_workspace_program_artifact(&workspace, "input.nx", &implicit_context())
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"));
    let EvalResult::Ok(value) = eval_program_artifact(&artifact) else {
        panic!("expected evaluation to succeed");
    };
    assert!(
        matches!(&value, NxValue::Record { type_name: Some(name), .. } if name == "SkiaButton"),
        "{value:?}"
    );
}

fn has_error_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity() == Severity::Error)
}
