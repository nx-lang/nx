use crate::model::{CodegenProperty, CodegenRecordField};
use crate::runtime::runtime_helper_source;
use crate::{
    build_codegen_program, emit_codegen_program, emit_js_program_module, emit_program,
    javascript_runtime_helper_source, CodegenDeclaration, CodegenDeclarationKind,
    CodegenEntrypoint, CodegenExpression, CodegenExpressionKind, CodegenModule,
    CodegenModuleProvenance, CodegenOptions, CodegenProgram, CodegenReference, CodegenTarget,
    CodegenTypeRef, JsProgramModuleOptions, DEFAULT_JS_PROGRAM_MODULE_NAME,
    DEFAULT_JS_PROGRAM_MODULE_RUNTIME_IMPORT_SPECIFIER, NX_JS_RUNTIME_ABI,
};
use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact, eval_program_artifact,
    EvalResult, NxWorkspace, NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use nx_diagnostics::{Severity, TextSpan};
use nx_hir::{ast, LocalDefinitionId};
use nx_interpreter::{ResolvedItemKind, RuntimeModuleId};
use nx_types::Type;
use serde_json::Value;
use std::fs;
use std::process::Command;
use std::sync::OnceLock;
use tempfile::TempDir;

fn artifact_from_source(source: &str) -> ProgramArtifact {
    build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
        .expect("program artifact should build")
}

fn artifact_from_workspace(files: &[(&str, &str)], entry: &str) -> ProgramArtifact {
    let modules = files
        .iter()
        .map(|(identity, source)| {
            NxWorkspaceModule::from_source(*identity, *source).expect("workspace module")
        })
        .collect::<Vec<_>>();
    let workspace = NxWorkspace::new(modules).expect("workspace");
    build_workspace_program_artifact(&workspace, entry, &ProgramBuildContext::empty())
        .expect("workspace artifact")
}

fn generated_file(artifact: &ProgramArtifact, target: CodegenTarget, name: &str) -> String {
    let options = match target {
        CodegenTarget::TypeScript => CodegenOptions::typescript(),
        CodegenTarget::JavaScript => CodegenOptions::javascript(),
    };
    let output = emit_program(artifact, &options).expect("codegen output");
    output
        .files
        .into_iter()
        .find(|file| file.relative_path.to_string_lossy() == name)
        .map(|file| file.content)
        .expect("generated file")
}

fn generated_module_body(module: &str) -> &str {
    module
        .split_once("\n\n")
        .map(|(_, body)| body)
        .unwrap_or(module)
}

fn execute_generated_javascript_root(source: &str) -> String {
    let artifact = artifact_from_source(source);
    execute_generated_javascript_artifact_root(&artifact, "")
}

fn execute_generated_javascript_artifact_root(artifact: &ProgramArtifact, args: &str) -> String {
    execute_generated_javascript_artifact_script(
        artifact,
        &format!("console.log(JSON.stringify(m.root({})));", args),
    )
}

fn execute_generated_javascript_artifact_script(
    artifact: &ProgramArtifact,
    script_body: &str,
) -> String {
    let output = emit_program(artifact, &CodegenOptions::javascript()).expect("js output");
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#).expect("package file");
    for file in output.files {
        fs::write(dir.path().join(file.relative_path), file.content).expect("generated file");
    }

    let index_url = format!("file://{}", dir.path().join("index.js").display());
    let script = format!(
        "import({:?}).then((m) => {{ {} }});",
        index_url, script_body
    );
    let output = node_command()
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .output()
        .expect("node execution");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn execute_generated_js_program_module_script(
    artifact: &ProgramArtifact,
    script_body: &str,
) -> String {
    let options = JsProgramModuleOptions {
        runtime_import_specifier: "./nx-runtime.js".to_string(),
        ..JsProgramModuleOptions::default()
    };
    let module = emit_js_program_module(artifact, &options).expect("program module output");
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#).expect("package file");
    fs::write(
        dir.path().join("nx-runtime.js"),
        javascript_runtime_helper_source(),
    )
    .expect("runtime helper");
    fs::write(dir.path().join("program.js"), module.source_text).expect("program module");

    let program_url = format!("file://{}", dir.path().join("program.js").display());
    let script = format!(
        "import({:?}).then((m) => {{ {} }});",
        program_url, script_body
    );
    let output = node_command()
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .output()
        .expect("node execution");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The TypeScript compiler, from `PATH` or from the repository's own `runtime/typescript`
/// install. `tsc` is a hard requirement of these tests, so a missing one fails the run instead of
/// skipping it.
fn tsc_command() -> Command {
    let workspace_tsc = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../runtime/typescript/node_modules/.bin/tsc");
    for program in [std::path::PathBuf::from("tsc"), workspace_tsc] {
        let probe = Command::new(&program).arg("--version").output();
        if probe.is_ok_and(|output| output.status.success()) {
            return Command::new(program);
        }
    }
    panic!("`tsc` is not available: run `pnpm install` at the repository root");
}

/// Runs an ES module script next to the emitted runtime module for `target`, returning what it
/// printed. The TypeScript runtime is compiled with `tsc` first; a missing tool fails the run.
fn execute_script_against_emitted_runtime(target: CodegenTarget, script: &str) -> String {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#).expect("package file");
    let runtime_name = format!("nx-runtime.{}", target.extension());
    fs::write(
        dir.path().join(&runtime_name),
        runtime_helper_source(target),
    )
    .expect("runtime");
    if target == CodegenTarget::TypeScript {
        let mut tsc = tsc_command();
        let output = tsc
            .current_dir(dir.path())
            .args([
                "--module",
                "ES2020",
                "--target",
                "ES2020",
                "--strict",
                &runtime_name,
            ])
            .output()
            .expect("tsc execution");
        assert!(
            output.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(dir.path().join("script.mjs"), script).expect("script");
    let output = node_command()
        .current_dir(dir.path())
        .arg("script.mjs")
        .output()
        .expect("node execution");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The oldest Node these tests run on, from `engines.node` in the root `package.json`.
const MINIMUM_NODE_VERSION: (u32, u32) = (22, 18);

/// What every `node` probe failure tells the developer to do. The floor is formatted from
/// `MINIMUM_NODE_VERSION`, so raising that constant updates the advice with it. CI installs the
/// Node pinned in `.github/actions/setup-rust-node/action.yaml`.
fn node_requirement() -> String {
    format!(
        "these tests need Node >= {}.{} on `PATH`, as CI uses Node 24",
        MINIMUM_NODE_VERSION.0, MINIMUM_NODE_VERSION.1
    )
}

/// A `node` command for running the generated JavaScript. Node is a hard requirement of these
/// tests, so a missing or too old one fails the run instead of skipping it. The probe runs once
/// per test binary.
fn node_command() -> Command {
    static PROBE: OnceLock<()> = OnceLock::new();
    PROBE.get_or_init(|| {
        let probe = Command::new("node").arg("--version").output();
        let reported = match probe {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => panic!("`node` is not available: {}", node_requirement()),
        };
        let version = parse_node_version(&reported).unwrap_or_else(|| {
            panic!(
                "`node --version` printed {reported:?}: {}",
                node_requirement()
            )
        });
        assert!(
            version >= MINIMUM_NODE_VERSION,
            "`node` is {reported}: {}",
            node_requirement()
        );
    });
    Command::new("node")
}

/// The major and minor of a `node --version` line such as `v24.1.0`.
fn parse_node_version(reported: &str) -> Option<(u32, u32)> {
    let mut parts = reported.trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn assert_generated_typescript_artifact_type_checks(artifact: &ProgramArtifact) {
    let output = emit_program(artifact, &CodegenOptions::typescript()).expect("ts output");
    let mut tsc = tsc_command();

    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#).expect("package file");
    for file in output.files {
        fs::write(dir.path().join(file.relative_path), file.content).expect("generated file");
    }

    let output = tsc
        .current_dir(dir.path())
        .args([
            "--noEmit",
            "--module",
            "NodeNext",
            "--moduleResolution",
            "NodeNext",
            "--target",
            "ES2020",
            "--strict",
            "index.ts",
        ])
        .output()
        .expect("tsc execution");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn interpreter_json_root(source: &str) -> String {
    let artifact = artifact_from_source(source);
    interpreter_json_artifact_root(&artifact)
}

fn interpreter_json_artifact_root(artifact: &ProgramArtifact) -> String {
    match eval_program_artifact(artifact) {
        EvalResult::Ok(value) => value.to_json_string().expect("interpreter json"),
        EvalResult::Err(diagnostics) => panic!("interpreter diagnostics: {:?}", diagnostics),
    }
}

fn assert_json_values_eq(actual: &str, expected: &str) {
    let actual: Value = serde_json::from_str(actual).expect("actual generated json");
    let expected: Value = serde_json::from_str(expected).expect("expected interpreter json");
    assert_eq!(actual, expected);
}

#[test]
fn builds_codegen_program_from_inline_artifact() {
    let artifact = artifact_from_source("let root() = { 1 + 2 }");
    let program = build_codegen_program(&artifact).expect("codegen program");

    assert_eq!(program.fingerprint, artifact.fingerprint);
    assert!(program.entrypoint("root").is_some());
    assert_eq!(program.source_entries.len(), 1);
    assert_eq!(program.source_entries[0].identity, "main.nx");
    assert_eq!(
        artifact.source_text("main.nx"),
        Some("let root() = { 1 + 2 }")
    );
    assert_eq!(
        artifact.source_entries()[0].source,
        "let root() = { 1 + 2 }"
    );
}

/// A record field read inside a component body is typed by the module that declared the field.
///
/// `Swatch.hue` is declared against `model.nx`'s `Hue`. Resolving the field's type in the consuming
/// module would let `main.nx`'s unrelated same-named `Hue` answer for it, which is exactly the
/// capture nominal identity exists to prevent.
#[test]
fn a_record_field_read_resolves_its_type_in_the_declaring_module() {
    let modules = [
        (
            "main.nx",
            "import { Swatch } from \"./model.nx\"\n             type Hue = Blue | Violet\n             external component <Paint colour:Hue? />\n             abstract external component <Node />\n             component <Chip extends Node s:Swatch /> = { <Paint colour={s.hue} /> }\n             let root() = { <Chip s=<Swatch hue={Swatch} /> /> }",
        ),
        (
            "model.nx",
            "export type Hue = Red | Green\nexport type Swatch = { hue:Hue }",
        ),
    ]
    .iter()
    .map(|(identity, source)| {
        NxWorkspaceModule::from_source(*identity, *source).expect("workspace module")
    })
    .collect::<Vec<_>>();
    let workspace = NxWorkspace::new(modules).expect("workspace");
    let diagnostics =
        build_workspace_program_artifact(&workspace, "main.nx", &ProgramBuildContext::empty())
            .expect_err(
                "the local Hue must not answer for a field declared against model.nx's Hue",
            );

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("main.nx:Hue")
                && diagnostic.message.contains("model.nx:Hue")),
        "expected a diagnostic distinguishing the two same-named unions: {diagnostics:?}"
    );
}

/// A written-but-empty body and an absent body mean different things, and the two implementations
/// of that rule must agree.
///
/// <para>The interpreter keys on whether any content expression was written
/// (`normalize_content_values` in `nx-interpreter`); code generation keys on whether the emitted
/// content list is non-empty, in three separate places across two crates. Nothing but this test
/// holds them together, and this is the one shape whose meaning the change altered: `<Box>{}</Box>`
/// now binds the empty list where it previously could not be written at all.</para>
#[test]
fn an_empty_written_body_generates_an_empty_list_and_an_absent_one_takes_the_default() {
    const WRITTEN: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items: object[] = {<A n=9 />} }
        let root() = { <Box>{}</Box> }
    "#;
    const ABSENT: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items: object[] = {<A n=9 />} }
        let root() = { <Box /> }
    "#;

    let written = artifact_from_source(WRITTEN);
    let module = generated_file(&written, CodegenTarget::TypeScript, "m0_main.ts");
    assert!(
        module.contains("items: []"),
        "a written-but-empty body should generate an empty list, got: {module}"
    );
    assert_json_values_eq(
        &interpreter_json_root(WRITTEN),
        r#"{"$type":"Box","items":[]}"#,
    );

    let absent = artifact_from_source(ABSENT);
    let module = generated_file(&absent, CodegenTarget::TypeScript, "m0_main.ts");
    assert!(
        !module.contains("items: []"),
        "an absent body should leave the declared default, got: {module}"
    );
    assert_json_values_eq(
        &interpreter_json_root(ABSENT),
        r#"{"$type":"Box","items":[{"$type":"A","n":9}]}"#,
    );
}

#[test]
fn emits_typescript_with_type_syntax_and_runtime_helpers() {
    let artifact = artifact_from_source("let root() = { \"hello\" }");
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let runtime = generated_file(&artifact, CodegenTarget::TypeScript, "nx-runtime.ts");

    assert!(module.contains("export function root(): string"));
    assert!(module.contains("return \"hello\";"));
    assert!(runtime.contains("export type NxValue"));
    assert!(runtime.contains("| readonly NxValue[]"));
    assert!(runtime.contains("export function nxArraySchema"));
    assert!(runtime.contains("export function nxRecordSchema"));
    assert!(runtime.contains("export function nxEnumSchema"));
    assert!(!runtime.contains("signal"));
    assert!(!runtime.contains("subscription"));
    assert!(!runtime.contains("dispatch"));
}

#[test]
fn emits_javascript_without_typescript_only_syntax() {
    let artifact = artifact_from_source("let root() = { \"hello\" }");
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    let runtime = generated_file(&artifact, CodegenTarget::JavaScript, "nx-runtime.js");

    assert!(module.contains("export function root()"));
    assert!(!module.contains(": unknown"));
    assert!(!runtime.contains("export type"));
    assert!(!runtime.contains(": string"));
    assert!(runtime.contains("export function nxArraySchema"));
    assert!(runtime.contains("export function nxRecordSchema"));
    assert!(runtime.contains("export function nxEnumSchema"));
}

#[test]
fn javascript_output_executes_as_esm() {
    let output = execute_generated_javascript_root("let root() = { 1 + 2 }");
    assert_eq!(output, "3");
}

#[test]
fn string_plus_emits_a_text_conversion_for_each_primitive_operand() {
    let artifact = artifact_from_source(
        "let label(count:int) = { \"Total: \" + count }\n\
         let width(w:float32) = { w + \" px\" }\n\
         let root() = { label(3) }",
    );
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    let runtime = generated_file(&artifact, CodegenTarget::JavaScript, "nx-runtime.js");

    assert!(module.contains("(\"Total: \" + String(count))"), "{module}");
    // A float32 is carried as a number, so `String` would print its float64 expansion.
    assert!(module.contains("(nxFloat32Text(w) + \" px\")"), "{module}");
    assert!(module.contains("nxFloat32Text"), "{module}");
    assert!(
        runtime.contains("export function nxFloat32Text"),
        "{runtime}"
    );
}

#[test]
fn generated_javascript_prints_primitives_as_the_interpreter_does() {
    let cases = [
        "let root() = { \"Total: \" + 3 }",
        "let root() = { 1 + 2 + \" items\" }",
        "let root() = { \"n=\" + 1 + 2 }",
        "let root() = { \"on: \" + true }",
        "let root() = { \"\" + 1.0 + \" \" + 0.1 + \" \" + 1.0e21 + \" \" + 1.0e-7 + \" \" + -0.0 }",
        "let tenth: float32 = 0.1\nlet root() = { \"w=\" + tenth }",
        // A computed float32 is not rounded by JavaScript arithmetic, so the helper must round it.
        "let scaled(w:float32) = { w * 3 + \" px\" }\nlet root() = { scaled(2.3) }",
        "type Item = { title:string }\n\
         let item = <Item title=\"Rust\" />\n\
         let root() = { \"Reorder \" + item.title }",
        "let root() = { 1 + 1.5 }",
        "let root() = { 2 == 2.0 }",
        // A join is a float64, so dividing it by an int divides as floats. Generated JavaScript
        // has no match expressions yet; the conformance corpus covers a match arm.
        "let pick(b:boolean, n:int, x:float64) = { if b { n } else { x } }\n\
         let root() = { pick(true, 3, 1.5) / 2 }",
        // Each float32 operation rounds to a float32, so a widened or compared result agrees.
        "let scaled(w:float32): float64 = { w * 3 }\nlet root() = { scaled(2.3) }",
        "let scaled(w:float32): float64 = { w * 3 }\n\
         let root() = { scaled(2.3) == 6.899999618530273 }",
        "let steps(v:float32, d:float32): float64 = { (v + 0.1 - 0.2) / d }\n\
         let root() = { steps(2.3, 3) }",
        "let half(v:float32): float64 = { v / 2 }\nlet root() = { half(0.1) }",
        // Integer division truncates toward zero.
        "let a(n:int) = { n / 2 }\nlet root() = { a(7) + a(-7) * 10 }",
        "let q(n:int32, m:int) = { n / m }\nlet root() = { q(7, 2) }",
        // A remainder and a float32 quotient go through the same helpers.
        "let r(n:int, m:int) = { n % m }\nlet root() = { r(-7, 2) }",
        "let fr(v:float64, w:float64) = { v % w }\nlet root() = { fr(-7.5, 2.0) }",
        "let fq(a:float32, b:float32): float64 = { a / b }\nlet root() = { fq(0.1, 3) }",
        // A widened branch computes at its own type before the join widens its result.
        "let half(b:boolean, n:int, x:float64) = { if b { n / 2 } else { x } }\n\
         let root() = { half(true, 7, 0.5) + 0.25 }",
        "let pickf(b:boolean, v:float32, x:float64) = { if b { v * 3 } else { x } }\n\
         let scale(v:float32) = { pickf(true, v, 0.5) }\n\
         let root() = { scale(2.3) }",
    ];
    for source in cases {
        assert_json_values_eq(
            &execute_generated_javascript_root(source),
            &interpreter_json_root(source),
        );
    }
}

#[test]
fn emits_cross_module_imports_from_resolved_references() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer } from "../shared/value.nx"
let root(): int = { answer }"#,
            ),
            ("shared/value.nx", r#"export let answer: int = 42"#),
        ],
        "app/main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");

    assert!(module.contains("import { answer as m1_answer } from \"./m1_value.js\";"));
    assert!(module.contains("return m1_answer;"));
}

#[test]
fn generated_javascript_serializes_records_constant_cases_and_elements() {
    let cases = [
        r#"
type User = {
  name: string
  age: int
}
let root() = { <User name="Ada" age=42 /> }
"#,
        r#"
type Theme = light | dark
let root() = { Theme.dark }
"#,
        r#"let root() = { <div class="test" /> }"#,
    ];

    for source in cases {
        let output = execute_generated_javascript_root(source);
        assert_json_values_eq(&output, &interpreter_json_root(source));
    }
}

#[test]
fn generated_javascript_serializes_union_cases_and_defaults() {
    let cases = [
        r#"
type LoadState = idle | loading
let root(): LoadState = { LoadState.idle }
"#,
        r#"
type LoadState =
  | idle
  | failed {
      message: string
      retryable: boolean = true
    }
let root(): LoadState = { <LoadState.failed message={"Offline"} /> }
"#,
        r#"
type User = {
  name: string
  age: int = 30
}
let root() = { <User name="Bob" /> }
"#,
    ];

    for source in cases {
        let output = execute_generated_javascript_root(source);
        assert_json_values_eq(&output, &interpreter_json_root(source));
    }
}

#[test]
fn local_bindings_shadow_imported_items() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer } from "../shared/value.nx"
let root(answer: int): int = { answer }"#,
            ),
            ("shared/value.nx", r#"export let answer: int = 42"#),
        ],
        "app/main.nx",
    );

    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(!module.contains("m1_answer"));

    let output = execute_generated_javascript_artifact_root(&artifact, "7");
    assert_eq!(output, "7");
}

#[test]
fn typescript_uses_js_import_specifiers_and_only_used_runtime_helpers() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer } from "../shared/value.nx"
let root(items: int[]): int[] = { for item in items { item + answer } }"#,
            ),
            ("shared/value.nx", r#"export let answer: int = 42"#),
        ],
        "app/main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let index = generated_file(&artifact, CodegenTarget::TypeScript, "index.ts");

    assert!(module.contains("from \"./m1_value.js\";"));
    assert!(index.contains("from \"./m0_main.js\";"));
    assert!(!module.contains(".ts\""));
    assert!(module.contains("export function root(items: readonly number[]): readonly number[]"));
    assert!(module.contains("return Array.from(items).map((item, _index) => (item + m1_answer));"));
    assert!(!module.contains("nx-runtime"));
    assert!(!module.contains("nxArray"));
    assert!(!module.contains("nxElement"));
    assert!(!module.contains("nxRuntimeError"));

    let trivial = generated_file(
        &artifact_from_source("let root() = { \"hello\" }"),
        CodegenTarget::TypeScript,
        "m0_main.ts",
    );
    assert!(!trivial.contains("nx-runtime"));
}

#[test]
fn generated_typescript_type_checks() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer } from "../shared/value.nx"
type User = { name: string age: int = 0 }
let root(): User = { <User name="Ada" age={answer} /> }"#,
            ),
            ("shared/value.nx", r#"export let answer: int = 42"#),
        ],
        "app/main.nx",
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

#[test]
fn generated_component_typescript_type_checks() {
    let artifact = artifact_from_source(
        r#"
type Mode = exact | fuzzy
type User = { name:string }
type LoadState =
  | ready { label:string }
external component <Question label:string />
external component <Summary mode:Mode user:User load:LoadState child:Question query:string />
component <SearchBox mode:Mode user:User load:LoadState child:Question /> = {
  state { mode:Mode = {mode} }
  <Summary mode={mode} user={user} load={load} child={child} query="docs" />
}
let root() = { 1 }
"#,
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

#[test]
fn materialized_record_iife_uses_collision_free_field_temps() {
    let module_id = RuntimeModuleId::new(0);
    let root_reference = CodegenReference {
        module_id,
        definition_id: LocalDefinitionId::new(0),
        name: "root".to_string(),
        kind: ResolvedItemKind::Function,
    };
    let program = CodegenProgram {
        fingerprint: 1,
        entry_identity: "main.nx".to_string(),
        modules: vec![CodegenModule {
            id: module_id,
            provenance: CodegenModuleProvenance::SourceProvider {
                identity: "main.nx".to_string(),
            },
            imports: Vec::new(),
            declarations: vec![CodegenDeclaration {
                reference: root_reference.clone(),
                span: TextSpan::default(),
                kind: CodegenDeclarationKind::Function {
                    params: Vec::new(),
                    return_type: None,
                    body: CodegenExpression {
                        expr_id: 0,
                        span: TextSpan::default(),
                        ty: None,
                        kind: CodegenExpressionKind::Record {
                            is_update: false,
                            reference: None,
                            name: "CollisionRecord".to_string(),
                            fields: vec![
                                int_field_with_default("my-field", 1),
                                int_field_with_default("my_field", 2),
                                int_field_with_default("__nx_prop_0", 3),
                            ],
                            properties: vec![CodegenProperty {
                                name: "explicit".to_string(),
                                value: int_expression(4, 4),
                                span: TextSpan::default(),
                            }],
                            content_field: None,
                            content: Vec::new(),
                        },
                    },
                },
            }],
        }],
        entrypoints: vec![CodegenEntrypoint {
            name: "root".to_string(),
            reference: root_reference,
        }],
        component_entrypoints: Vec::new(),
        source_entries: Vec::new(),
    };

    let output = emit_codegen_program(&program, &CodegenOptions::javascript()).expect("js output");
    let module = output
        .files
        .into_iter()
        .find(|file| file.relative_path.to_string_lossy() == "m0_main.js")
        .map(|file| file.content)
        .expect("generated module");

    assert!(module.contains("const __nx_prop_0 = 4;"));
    assert!(module.contains("const __nx_field_0 = 1;"));
    assert!(module.contains("const __nx_field_1 = 2;"));
    assert!(module.contains("const __nx_field_2 = 3;"));
    assert!(module.contains(
        "return { $type: \"CollisionRecord\", \"my-field\": __nx_field_0, my_field: __nx_field_1, __nx_prop_0: __nx_field_2, explicit: __nx_prop_0 };"
    ));
    assert!(!module.contains("const my_field ="));
}

fn int_field_with_default(name: &str, value: i64) -> CodegenRecordField {
    CodegenRecordField {
        name: name.to_string(),
        ty: ast::TypeRef::name("int"),
        resolved_ty: CodegenTypeRef::Primitive {
            name: "int".to_string(),
        },
        is_content: false,
        is_required: false,
        default: Some(int_expression(value as u32, value)),
        owner_module_id: RuntimeModuleId::new(0),
        span: TextSpan::default(),
    }
}

fn int_expression(expr_id: u32, value: i64) -> CodegenExpression {
    CodegenExpression {
        expr_id,
        span: TextSpan::default(),
        ty: Some(Type::int()),
        kind: CodegenExpressionKind::Literal(ast::Literal::Int(value)),
    }
}

#[test]
fn cross_module_record_literals_use_declared_field_order() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { User } from "../shared/model.nx"
let root() = { <User age=42 name="Ada" /> }"#,
            ),
            (
                "shared/model.nx",
                r#"export type User = {
  name: string
  age: int
}"#,
            ),
        ],
        "app/main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");

    assert!(module.contains("return ({ $type: \"User\", name: \"Ada\", age: 42 });"));
}

#[test]
fn emits_strong_typescript_records_and_constant_unions() {
    let record_artifact = artifact_from_source(
        r#"
type User = { name: string tags: string[] age: int }
let root() = { <User age=42 name="Ada" tags={ "admin" "editor" } /> }
"#,
    );
    let record_module = generated_file(&record_artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(record_module.contains("export type User = {"));
    assert!(!record_module.contains("Readonly<{"));
    assert!(record_module.contains("readonly $type: \"User\";"));
    assert!(record_module.contains("readonly name: string;"));
    assert!(record_module.contains("readonly tags: readonly string[];"));
    assert!(record_module.contains("readonly age: number;"));
    assert!(record_module.contains("export function root(): User"));
    assert!(record_module.contains(
        "return ({ $type: \"User\", name: \"Ada\", tags: [\"admin\", \"editor\"], age: 42 });"
    ));
    assert!(!record_module.contains("export const User"));
    assert!(!record_module.contains("nxArray"));
    assert!(!record_module.contains("nxRecord"));

    let constant_union_artifact = artifact_from_source(
        r#"
type Theme = light | dark
let root() = { Theme.dark }
"#,
    );
    let constant_union_module = generated_file(
        &constant_union_artifact,
        CodegenTarget::TypeScript,
        "m0_main.ts",
    );

    assert!(constant_union_module.contains("export const Theme = {"));
    assert!(constant_union_module.contains("light: \"light\","));
    assert!(constant_union_module.contains("dark: \"dark\","));
    assert!(constant_union_module.contains("} as const;"));
    assert!(constant_union_module.contains("export type Theme = typeof Theme[keyof typeof Theme];"));
    assert!(constant_union_module.contains("export function root(): Theme"));
    assert!(constant_union_module.contains("return Theme.dark;"));
    assert!(!constant_union_module.contains("nxEnum"));
}

#[test]
fn emits_supported_subset_snapshots_in_both_target_modes() {
    let cases = [
        ("primitive", "let root() = { 1 + 2 }", "return (1 + 2);"),
        (
            "call",
            r#"
let add(a:int, b:int) = { a + b }
let root() = { add(1, 2) }
"#,
            "return add(1, 2);",
        ),
        (
            "conditional",
            "let root() = { if true { 1 } else { 2 } }",
            "return (true ? 1 : 2);",
        ),
        (
            "array",
            "let root(): int[] = { 1 2 3 }",
            "return [1, 2, 3];",
        ),
        (
            "loop",
            "let root(items:int[]) = { for item, index in items { item + index } }",
            "Array.from(items).map((item, index) => (item + index))",
        ),
        (
            "record",
            r#"
type User = { name: string age: int }
let root() = { <User age=42 name="Ada" /> }
"#,
            "return ({ $type: \"User\", name: \"Ada\", age: 42 });",
        ),
        (
            "enum",
            r#"
type Theme = light | dark
let root() = { Theme.dark }
"#,
            "return Theme.dark;",
        ),
        (
            // A constant union emits the same frozen value object an enum emitted, so its case
            // is reached the same way.
            "constant union",
            r#"
type LoadState = idle | loading
let root(): LoadState = { LoadState.idle }
"#,
            "return LoadState.idle;",
        ),
        (
            // A constant case of a mixed union has no value object to reach through, so it is
            // emitted as the bare string it is on the wire.
            "constant case of a mixed union",
            r#"
type Shape = point | circle { radius: float64 }
let root(): Shape = { Shape.point }
"#,
            "return \"point\";",
        ),
        (
            // A case that carries fields keeps the record representation.
            "payload case",
            r#"
type Shape = point | circle { radius: float64 }
let root(): Shape = { <Shape.circle radius=1.5 /> }
"#,
            "$type: \"Shape.circle\"",
        ),
        (
            "element",
            r#"let root() = { <div class="test" /> }"#,
            "return nxElement(\"div\", { \"class\": \"test\" }, []);",
        ),
    ];

    for (_name, source, expected) in cases {
        let artifact = artifact_from_source(source);
        let ts_module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
        let js_module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");

        assert!(
            ts_module.contains(expected),
            "missing TS snippet: {expected}"
        );
        assert!(
            js_module.contains(expected),
            "missing JS snippet: {expected}"
        );
        assert!(!ts_module.contains(": unknown"));
        assert!(!js_module.contains(": unknown"));
        assert!(!js_module.contains("export type"));
    }
}

#[test]
fn generated_output_is_deterministic() {
    let artifact = artifact_from_source(
        r#"
type User = {
  name: string
  age: int
}
let root() = { <User name="Ada" age=42 /> }
"#,
    );
    let first = emit_program(&artifact, &CodegenOptions::javascript()).expect("first output");
    let second = emit_program(&artifact, &CodegenOptions::javascript()).expect("second output");

    assert_eq!(first.files, second.files);
}

#[test]
fn js_program_module_defaults_and_metadata_are_host_neutral() {
    let artifact = artifact_from_source("let root() = { 1 + 2 }");
    let module = emit_js_program_module(&artifact, &JsProgramModuleOptions::default())
        .expect("program module");

    assert_eq!(module.logical_module_name, DEFAULT_JS_PROGRAM_MODULE_NAME);
    assert_eq!(
        module.runtime_import_specifier,
        DEFAULT_JS_PROGRAM_MODULE_RUNTIME_IMPORT_SPECIFIER
    );
    assert_eq!(module.runtime_abi, NX_JS_RUNTIME_ABI);
    assert_eq!(module.program_fingerprint, artifact.fingerprint);
    assert_eq!(module.function_exports.len(), 1);
    assert_eq!(module.function_exports[0].entrypoint_name, "root");
    assert_eq!(module.function_exports[0].export_name, "root");
    assert!(module.component_exports.is_empty());
    assert!(module.source_text.contains("export function root()"));
    assert!(module
        .source_text
        .contains("export const nxProgramModuleManifest"));
    assert!(module
        .source_text
        .contains("runtimeAbi: \"nx-js-runtime-v1\""));
    assert!(!module.source_text.contains("nx-runtime.js"));
    assert!(!module.source_text.contains("from \"./m"));
}

#[test]
fn js_program_module_uses_configured_runtime_import_specifier() {
    let artifact = artifact_from_source(r#"let root() = { <div class="test" /> }"#);
    let options = JsProgramModuleOptions {
        logical_module_name: "reachme/cache/main".to_string(),
        runtime_import_specifier: "./nx-runtime.js".to_string(),
    };
    let module = emit_js_program_module(&artifact, &options).expect("program module");

    assert_eq!(module.logical_module_name, "reachme/cache/main");
    assert_eq!(module.runtime_import_specifier, "./nx-runtime.js");
    assert!(module
        .source_text
        .contains("import { nxElement } from \"./nx-runtime.js\";"));
    assert!(!module.source_text.contains("export function nxElement"));
}

#[test]
fn js_program_module_rejects_invalid_artifact_diagnostics() {
    let artifact = artifact_from_source(r#"let root(): int = { "not an int" }"#);
    let error = emit_js_program_module(&artifact, &JsProgramModuleOptions::default())
        .expect_err("invalid artifact should fail program-module codegen");

    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error));
}

#[test]
fn js_program_module_flattens_cross_module_values_types_unions_and_components() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer, add, User, Theme, LoadState, SharedBox } from "../shared/model.nx"
let root() = {
  <SharedBox
    user=<User name="Ada" />
    theme={Theme.dark}
    load=<LoadState.ready count={add(answer(), 1)} />
  />
}"#,
            ),
            (
                "shared/model.nx",
                r#"export let answer(): int = { 42 }
export let bonus: int = 1
export let add(a:int, b:int): int = { a + b }
export type Theme = light | dark
export type User = { name:string score:int = 42 }
export type LoadState = | ready { count:int }
export external component <TextInput value:string />
export component <SharedBox user:User theme:Theme load:LoadState /> = {
  <TextInput value={user.name} />
}"#,
            ),
        ],
        "app/main.nx",
    );
    let options = JsProgramModuleOptions {
        runtime_import_specifier: "./nx-runtime.js".to_string(),
        ..JsProgramModuleOptions::default()
    };
    let module = emit_js_program_module(&artifact, &options).expect("program module");

    assert!(module.source_text.contains("function answer()"));
    assert!(module.source_text.contains("const bonus = 1;"));
    assert!(module.source_text.contains("function add(a, b)"));
    assert!(module.source_text.contains("const Theme = Object.freeze"));
    assert!(module.source_text.contains("export function answer()"));
    assert!(module.source_text.contains("export function add(a, b)"));
    assert!(!module.source_text.contains("export const bonus = 1;"));
    assert!(!module
        .source_text
        .contains("export const Theme = Object.freeze"));
    assert!(module.source_text.contains("export function root()"));
    assert!(module
        .source_text
        .contains("export function SharedBox(props)"));
    assert!(module
        .source_text
        .contains("export const SharedBoxSchema = nxComponentSchema"));
    assert!(!module.source_text.contains("from \"./m"));
    assert!(!module.source_text.contains("from \"../"));

    let output = execute_generated_js_program_module_script(
        &artifact,
        "console.log(JSON.stringify(m.root()));",
    );
    assert_json_values_eq(&output, &interpreter_json_artifact_root(&artifact));
}

#[test]
fn js_program_module_plans_collision_free_names_across_modules() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { value as One.value } from "../one/value.nx"
import { value as Two.value } from "../two/value.nx"
let root(): int = { One.value + Two.value }"#,
            ),
            ("one/value.nx", "export let value: int = 1"),
            ("two/value.nx", "export let value: int = 2"),
        ],
        "app/main.nx",
    );
    let module = emit_js_program_module(
        &artifact,
        &JsProgramModuleOptions {
            runtime_import_specifier: "./nx-runtime.js".to_string(),
            ..JsProgramModuleOptions::default()
        },
    )
    .expect("program module");

    assert!(module.source_text.contains("const value = 1;"));
    assert!(module.source_text.contains("const value_2 = 2;"));
    assert!(!module.source_text.contains("export const value = 1;"));
    assert!(!module.source_text.contains("export const value_2 = 2;"));
    assert!(module.source_text.contains("return (value + value_2);"));
    assert!(!module.source_text.contains(" as m"));

    let output = execute_generated_js_program_module_script(
        &artifact,
        "console.log(JSON.stringify(m.root()));",
    );
    assert_eq!(output, "3");
}

#[test]
fn js_program_module_component_schema_execution_uses_separate_runtime() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { 1 }
"#,
    );
    let module = emit_js_program_module(
        &artifact,
        &JsProgramModuleOptions {
            runtime_import_specifier: "./nx-runtime.js".to_string(),
            ..JsProgramModuleOptions::default()
        },
    )
    .expect("program module");

    let search_box = module
        .component_exports
        .iter()
        .find(|export| export.component_name == "SearchBox")
        .expect("SearchBox component export");
    assert_eq!(search_box.component_export_name, "SearchBox");
    assert_eq!(search_box.schema_export_name, "SearchBoxSchema");
    assert_eq!(
        search_box.initial_state_export_name.as_deref(),
        Some("initialSearchBoxState")
    );
    assert_eq!(
        search_box.render_export_name.as_deref(),
        Some("renderSearchBox")
    );

    let output = execute_generated_js_program_module_script(
        &artifact,
        r#"console.log(JSON.stringify({
  init: m.SearchBoxSchema.initializeJson({}),
  evaluated: m.SearchBoxSchema.evaluateJson({}, { query: "docs" })
}));"#,
    );
    assert_json_values_eq(
        &output,
        r#"{
  "init": {
    "rendered": { "$type": "TextInput", "value": "Find docs" },
    "state": { "query": "Find docs" }
  },
  "evaluated": { "$type": "TextInput", "value": "docs" }
}"#,
    );
}

#[test]
fn js_program_module_cross_module_component_schema_executes_after_flattening() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { TextInput } from "../shared/ui.nx"
component <Host child:TextInput /> = { child }
let root() = { 1 }"#,
            ),
            (
                "shared/ui.nx",
                r#"export external component <TextInput value:string />"#,
            ),
        ],
        "app/main.nx",
    );
    let module = emit_js_program_module(
        &artifact,
        &JsProgramModuleOptions {
            runtime_import_specifier: "./nx-runtime.js".to_string(),
            ..JsProgramModuleOptions::default()
        },
    )
    .expect("program module");

    assert!(
        module
            .source_text
            .find("export const TextInputSchema")
            .expect("TextInput schema")
            < module
                .source_text
                .find("export const HostSchema")
                .expect("Host schema")
    );
    assert!(!module.source_text.contains("from \"./m"));

    let output = execute_generated_js_program_module_script(
        &artifact,
        r#"console.log(JSON.stringify(
  m.HostSchema.evaluateJson({ child: { $type: "TextInput", value: "docs" } })
));"#,
    );
    assert_json_values_eq(&output, r#"{ "$type": "TextInput", "value": "docs" }"#);
}

#[test]
fn js_program_module_output_is_deterministic() {
    let files = [
        (
            "app/main.nx",
            r#"import { SearchBox } from "../shared/ui.nx"
let root() = { <SearchBox /> }"#,
        ),
        (
            "shared/ui.nx",
            r#"export external component <TextInput value:string />
export component <SearchBox placeholder:string = "Find docs" /> = {
  <TextInput value={placeholder} />
}"#,
        ),
    ];
    let first_artifact = artifact_from_workspace(&files, "app/main.nx");
    let second_artifact = artifact_from_workspace(&files, "app/main.nx");
    let options = JsProgramModuleOptions {
        runtime_import_specifier: "./nx-runtime.js".to_string(),
        ..JsProgramModuleOptions::default()
    };
    let first =
        emit_js_program_module(&first_artifact, &options).expect("first JavaScript program module");
    let second = emit_js_program_module(&second_artifact, &options)
        .expect("second JavaScript program module");

    assert_eq!(first, second);
}

#[test]
fn js_program_module_reserves_runtime_helper_and_manifest_names() {
    let runtime_helper_collision = artifact_from_source(
        r#"
let nxElement() = { 1 }
let root() = { <div /> }
"#,
    );
    let options = JsProgramModuleOptions {
        runtime_import_specifier: "./nx-runtime.js".to_string(),
        ..JsProgramModuleOptions::default()
    };
    let runtime_helper_module =
        emit_js_program_module(&runtime_helper_collision, &options).expect("program module");
    assert!(runtime_helper_module
        .source_text
        .contains("import { nxElement } from \"./nx-runtime.js\";"));
    assert!(runtime_helper_module
        .source_text
        .contains("function nxElement_2()"));
    assert!(!runtime_helper_module
        .source_text
        .contains("function nxElement()"));

    let manifest_collision = artifact_from_source(
        r#"
let nxProgramModuleManifest() = { 1 }
let root() = { nxProgramModuleManifest() }
"#,
    );
    let manifest_module =
        emit_js_program_module(&manifest_collision, &options).expect("program module");
    assert!(manifest_module
        .source_text
        .contains("function nxProgramModuleManifest_2()"));
    assert!(manifest_module
        .source_text
        .contains("return nxProgramModuleManifest_2();"));
    assert!(manifest_module
        .source_text
        .contains("export const nxProgramModuleManifest = Object.freeze"));
}

#[test]
fn js_program_module_source_is_host_neutral() {
    let artifact = artifact_from_source(r#"let root() = { <div class="test" /> }"#);
    let module = emit_js_program_module(
        &artifact,
        &JsProgramModuleOptions {
            runtime_import_specifier: "./nx-runtime.js".to_string(),
            ..JsProgramModuleOptions::default()
        },
    )
    .expect("program module");
    let imports = module
        .source_text
        .lines()
        .filter(|line| line.starts_with("import "))
        .collect::<Vec<_>>();

    assert_eq!(
        imports,
        vec!["import { nxElement } from \"./nx-runtime.js\";"]
    );
    assert!(!module.source_text.contains("export default"));
    assert!(!module.source_text.contains("fetch("));
    assert!(!module.source_text.contains("WorkerCode"));
    assert!(!module.source_text.contains("actor("));
    assert!(!module.source_text.contains("setup("));
    assert!(!module.source_text.contains("require("));
    assert!(!module.source_text.contains("from \"node:"));
    assert!(!module.source_text.contains("from \"fs\""));
    assert!(!module.source_text.contains("from \"path\""));
    assert!(!module.source_text.contains("export function nxElement"));
}

#[test]
fn emits_typed_component_functions_state_helpers_and_schema_boundaries() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { 1 }
"#,
    );
    let ts_module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let js_module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    let index = generated_file(&artifact, CodegenTarget::JavaScript, "index.js");

    for module in [&ts_module, &js_module] {
        assert!(module.contains("export function SearchBox("));
        assert!(module.contains("export function initialSearchBoxState("));
        assert!(module.contains("export function renderSearchBox("));
        assert!(module.contains("export const SearchBoxSchema"));
        assert!(!module.contains("export class SearchBox"));
        assert!(!module.contains("static initialize("));
        assert!(!module.contains("static evaluate("));
    }
    assert!(ts_module.contains("export type SearchBoxProps"));
    assert!(ts_module.contains("export type SearchBoxElement"));
    assert!(ts_module.contains("export type SearchBoxState"));
    assert!(ts_module.contains("export type TextInputElement"));
    assert!(!ts_module.contains("SearchBoxOutput"));
    assert!(index
        .contains("export { SearchBox, SearchBoxSchema, initialSearchBoxState, renderSearchBox }"));
}

#[test]
fn emits_component_module_inline_snapshot() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox placeholder="Manual" /> }
"#,
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let body = generated_module_body(&module);

    assert!(body.contains("export type TextInputProps = {"));
    assert!(body.contains("export type TextInputElement = {"));
    assert!(body.contains("export function TextInput(props: TextInputProps): TextInputElement"));
    assert!(body.contains("export const TextInputSchema = nxExternalComponentSchema"));
    assert!(body.contains("const resolvedProps = resolveTextInputProps(props);"));
    assert!(body.contains("export type SearchBoxProps = {"));
    assert!(body.contains("export type SearchBoxElement = {"));
    assert!(body.contains("export type SearchBoxState = {"));
    assert!(
        body.contains("export function SearchBox(props: SearchBoxProps = {}): SearchBoxElement")
    );
    assert!(
        body.contains("return { $type: \"SearchBox\", placeholder: resolvedProps.placeholder };")
    );
    assert!(body.contains(
        "export function renderSearchBox(props: SearchBoxResolvedProps, state: SearchBoxState): TextInputElement"
    ));
    assert!(body.contains("export const SearchBoxSchema = nxComponentSchema"));
    assert!(body.contains(
        "export const SearchBoxSchema = nxComponentSchema<SearchBoxProps, SearchBoxState, TextInputElement>"
    ));
    assert!(body.contains(
        "return { rendered: renderSearchBox(resolvedProps, initialState), state: initialState };"
    ));
    assert!(body.contains("const resolvedState = state ?? initialSearchBoxState(resolvedProps);"));
    assert!(body.contains("return TextInput({ value: query });"));
    assert!(body.contains("return SearchBox({ placeholder: \"Manual\" });"));
    assert!(!body.contains("export class SearchBox"));
    assert!(!body.contains("normalizeProps"));
    assert!(!body.contains("SearchBoxOutput"));
    assert!(!body.contains("__props"));
    assert!(!body.contains("__state"));
}

#[test]
fn abstract_components_emit_contract_only_surface() {
    let artifact = artifact_from_source(
        r#"
abstract component <SearchBase placeholder:string />
component <SearchBox extends SearchBase /> = { placeholder }
let root() = { 1 }
"#,
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(module.contains("export type SearchBaseProps"));
    assert!(module.contains("type SearchBaseResolvedProps"));
    assert!(module.contains("export function SearchBox"));
    assert!(module.contains("export const SearchBoxSchema"));
    assert!(!module.contains("export function SearchBase"));
    assert!(!module.contains("export class SearchBase"));
    assert!(!module.contains("extends SearchBase"));
    assert!(!module.contains("SearchBaseState"));
}

#[test]
fn generated_javascript_component_descriptors_match_interpreter() {
    let source = r#"
external component <Question label:string />
let root() = { <Question label="Name" /> }
"#;
    let output = execute_generated_javascript_root(source);

    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_component_descriptors_apply_cross_module_inherited_defaults() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { Question } from "../shared/ui.nx"
external component <ShortTextQuestion extends Question placeholder:string? />
let root() = { <ShortTextQuestion /> }"#,
            ),
            (
                "shared/ui.nx",
                r#"export abstract external component <Question label:string = "Untitled" />"#,
            ),
        ],
        "app/main.nx",
    );
    let output = execute_generated_javascript_artifact_root(&artifact, "");

    assert_json_values_eq(
        &output,
        r#"{ "$type": "ShortTextQuestion", "label": "Untitled", "placeholder": null }"#,
    );
}

#[test]
fn generated_component_descriptors_preserve_content() {
    let source = r#"
external component <Panel content body:Element />
let root() = { <Panel><span /></Panel> }
"#;
    let output = execute_generated_javascript_root(source);

    assert_json_values_eq(
        &output,
        r#"{ "$type": "Panel", "body": { "$type": "span" } }"#,
    );
}

#[test]
fn generated_parent_renders_two_external_children_of_same_type() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput id:string label:string value:string = "" />
let textInputs(): TextInput[] = {
  <TextInput id="firstName" label="First name" />
  <TextInput id="lastName" label="Last name" />
}
component <QuestionFlow /> = { textInputs() }
let root() = { 1 }
"#,
    );
    let ts_module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify(m.QuestionFlowSchema.evaluateJson({})));",
    );

    assert!(ts_module.contains("export type TextInputElement"));
    assert!(ts_module.contains("export type QuestionFlowElement"));
    assert!(ts_module.contains(
        "export function QuestionFlow(props: QuestionFlowProps = {}): QuestionFlowElement"
    ));
    assert!(ts_module.contains(
        "function renderQuestionFlow(props: QuestionFlowResolvedProps): readonly TextInputElement[]"
    ));
    assert_json_values_eq(
        &output,
        r#"[
  { "$type": "TextInput", "id": "firstName", "label": "First name", "value": "" },
  { "$type": "TextInput", "id": "lastName", "label": "Last name", "value": "" }
]"#,
    );
}

#[test]
fn generated_schema_boundaries_validate_missing_unknown_defaults_and_state_json() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string = "" />
type User = { name:string role:string = "guest" }
component <ElementHost child:TextInput /> = { child }
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
component <Profile user:User /> = { user }
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
	  externalDefault: m.TextInputSchema.fromJson({}),
  recordDefault: m.ProfileSchema.evaluateJson({ user: { $type: "User", name: "Ada" } }),
  missingProp: m.SearchBoxSchema.tryEvaluateJson({ placeholder: 1 }),
  unknownProp: m.TextInputSchema.tryFromJson({ value: "docs", extra: true }),
  missingElementField: m.ElementHostSchema.tryEvaluateJson({ child: { $type: "TextInput" } }),
  missingState: m.SearchBoxSchema.tryEvaluateJson({}, {})
}));"#,
    );
    let output: Value = serde_json::from_str(&output).expect("schema boundary output");

    assert_eq!(
        output["externalDefault"],
        serde_json::json!({ "$type": "TextInput", "value": "" })
    );
    assert_eq!(
        output["recordDefault"],
        serde_json::json!({ "$type": "User", "name": "Ada", "role": "guest" })
    );
    assert_eq!(output["missingProp"]["ok"], false);
    assert_eq!(
        output["missingProp"]["diagnostics"][0]["code"],
        "invalid-field"
    );
    assert_eq!(output["unknownProp"]["ok"], false);
    assert_eq!(
        output["unknownProp"]["diagnostics"][0]["code"],
        "unknown-field"
    );
    assert_eq!(output["missingElementField"]["ok"], false);
    assert_eq!(
        output["missingElementField"]["diagnostics"][0]["code"],
        "missing-field"
    );
    assert_eq!(output["missingState"]["ok"], false);
    assert_eq!(
        output["missingState"]["diagnostics"][0]["code"],
        "missing-field"
    );
}

#[test]
fn generated_record_schema_defaults_can_reference_previous_fields() {
    let artifact = artifact_from_source(
        r#"
type Pair = { a:int b:int = { a } }
type PairChoice =
  | same { a:int b:int = { a } }
component <Box p:Pair /> = { p }
component <ChoiceBox choice:PairChoice /> = { choice }
let root() = { 1 }
"#,
    );
    let ts_module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    let js_module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  record: m.BoxSchema.evaluateJson({ p: { $type: "Pair", a: 7 } }),
  union: m.ChoiceBoxSchema.evaluateJson({ choice: { $type: "PairChoice.same", a: 3 } })
}));"#,
    );

    assert!(!ts_module.contains("defaultValue: a"));
    assert!(!js_module.contains("defaultValue: a"));
    assert!(ts_module.contains("defaultFactory"));
    assert!(js_module.contains("defaultFactory"));
    assert_json_values_eq(
        &output,
        r#"{
  "record": { "$type": "Pair", "a": 7, "b": 7 },
  "union": { "$type": "PairChoice.same", "a": 3, "b": 3 }
}"#,
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

#[test]
fn generated_javascript_gives_a_json_number_the_width_of_a_float32_prop() {
    let artifact = artifact_from_source(
        r#"
component <Gauge ratio:float32 /> = { ratio * 3 == 0.3 }
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  rounded: m.GaugeSchema.evaluateJson({ ratio: 0.1 }),
  exact: m.GaugeSchema.tryEvaluateJson({ ratio: 16777216 }).ok,
  inexact: m.GaugeSchema.tryEvaluateJson({ ratio: 16777217 }).ok
}));"#,
    );

    // `0.1` is rounded on the way in, and the product on the way out, as the interpreter does.
    assert_json_values_eq(
        &output,
        r#"{ "rounded": true, "exact": true, "inexact": false }"#,
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

#[test]
fn generated_cross_module_component_imports_include_element_and_schema_names() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { TextInput } from "../shared/ui.nx"
component <Host child:TextInput /> = { child }
let root() = { <Host child=<TextInput value="docs" /> /> }"#,
            ),
            (
                "shared/ui.nx",
                r#"export external component <TextInput value:string />"#,
            ),
        ],
        "app/main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(module.contains("TextInput as m1_TextInput"));
    assert!(module.contains("TextInputSchema as m1_TextInputSchema"));
    assert!(module.contains("TextInputElement as m1_TextInputElement"));
    assert!(module.contains("child: m1_TextInputElement"));
    assert!(module.contains("m1_TextInputSchema.element"));
}

#[test]
fn generated_component_return_types_follow_normal_child_components() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string />
component <Child /> = { <TextInput value="docs" /> }
component <Parent /> = { <Child /> }
let root() = { 1 }
"#,
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(module.contains("export type ChildElement = {"));
    assert!(module.contains("export type ParentElement = {"));
    assert!(module.contains("export function Child(props: ChildProps = {}): ChildElement"));
    assert!(module.contains("function renderChild(props: ChildResolvedProps): TextInputElement"));
    assert!(module.contains("export function Parent(props: ParentProps = {}): ParentElement"));
    assert!(module.contains("function renderParent(props: ParentResolvedProps): ChildElement"));
}

#[test]
fn generated_component_return_types_import_external_element_names() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { TextInput } from "../shared/ui.nx"
component <Host /> = { <TextInput value="docs" /> }
let root() = { <Host /> }"#,
            ),
            (
                "shared/ui.nx",
                r#"export external component <TextInput value:string />"#,
            ),
        ],
        "app/main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(module.contains("TextInputElement as m1_TextInputElement"));
    assert!(module.contains("export type HostElement = {"));
    assert!(module.contains("export function Host(props: HostProps = {}): HostElement"));
    assert!(module.contains("function renderHost(props: HostResolvedProps): m1_TextInputElement"));
}

#[test]
fn generated_component_suffix_names_avoid_source_declaration_collisions() {
    let artifact = artifact_from_source(
        r#"
type SearchBoxProps = { label:string }
type SearchBoxElement = { label:string }
let SearchBoxSchema = { 1 }
external component <SearchBox value:string />
let root() = { 1 }
"#,
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(module.contains("export type SearchBoxProps = {"));
    assert!(module.contains("export const SearchBoxSchema: number = 1;"));
    assert!(module.contains("export type SearchBoxProps_2 = {"));
    assert!(module.contains("export type SearchBoxElement_2 = {"));
    assert!(module.contains("export const SearchBoxSchema_2 = nxExternalComponentSchema"));
    assert!(
        module.contains("export function SearchBox(props: SearchBoxProps_2): SearchBoxElement_2")
    );
}

#[test]
fn generated_normal_component_descriptor_matches_interpreter() {
    let source = r#"
component <Child label:string /> = { "rendered child" }
let root() = { <Child label="Name" /> }
"#;
    let output = execute_generated_javascript_root(source);

    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_component_bodies_return_normal_child_descriptors() {
    let artifact = artifact_from_source(
        r#"
component <Child label:string /> = { "rendered child" }
component <Parent /> = { <Child label="Name" /> }
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify(m.ParentSchema.evaluateJson({})));",
    );

    assert_json_values_eq(&output, r#"{ "$type": "Child", "label": "Name" }"#);
}

#[test]
fn generated_component_initialize_and_evaluate_materialize_state() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  init: m.SearchBoxSchema.initializeJson({}),
  evaluated: m.SearchBoxSchema.evaluateJson({}, { query: "docs" })
}));"#,
    );

    assert_json_values_eq(
        &output,
        r#"{
  "init": {
    "rendered": { "$type": "TextInput", "value": "Find docs" },
    "state": { "query": "Find docs" }
  },
  "evaluated": { "$type": "TextInput", "value": "docs" }
}"#,
    );
}

#[test]
fn generated_component_entry_handles_constant_cases_nullable_fields_and_lists() {
    let artifact = artifact_from_source(
        r#"
type Mode = exact | fuzzy
external component <Summary mode:Mode tags:string[] note:string? />
component <SearchBox tags:string[] mode:Mode = { Mode.exact } /> = {
  state {
    query:string?
    tags:string[] = {tags}
    mode:Mode = {mode}
  }
  <Summary mode={mode} tags={tags} note={query} />
}
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  omitted: m.SearchBoxSchema.evaluateJson({ tags: ["nx"] }),
  explicit: m.SearchBoxSchema.evaluateJson(
    { tags: ["nx"], mode: "exact" },
    { query: "docs", tags: ["ui"], mode: "fuzzy" }
  )
}));"#,
    );

    assert_json_values_eq(
        &output,
        r#"{
  "omitted": { "$type": "Summary", "mode": "exact", "tags": ["nx"], "note": null },
  "explicit": { "$type": "Summary", "mode": "fuzzy", "tags": ["ui"], "note": "docs" }
}"#,
    );
}

#[test]
fn generated_component_entry_rejects_invalid_constant_case_host_input() {
    let artifact = artifact_from_source(
        r#"
type Mode = exact | fuzzy
external component <Summary mode:Mode />
component <SearchBox mode:Mode = { Mode.exact } /> = {
  state { mode:Mode = {mode} }
  <Summary mode={mode} />
}
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  prop: m.SearchBoxSchema.tryEvaluateJson({ mode: "bogus" }),
  state: m.SearchBoxSchema.tryEvaluateJson({ mode: "exact" }, { mode: "bogus" })
}));"#,
    );
    let output: Value = serde_json::from_str(&output).expect("tryEvaluate output");

    assert_eq!(output["prop"]["ok"], false);
    assert_eq!(output["state"]["ok"], false);
    assert_eq!(output["prop"]["diagnostics"][0]["code"], "invalid-enum");
    assert_eq!(output["state"]["diagnostics"][0]["code"], "invalid-enum");
    assert!(output["prop"]["diagnostics"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("SearchBox props.mode")));
    assert!(output["state"]["diagnostics"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("SearchBox state.mode")));
}

#[test]
fn generated_component_entry_rejects_invalid_named_host_input_shapes() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string }
type LoadState =
  | ready { label:string }
external component <Question label:string />
component <Host user:User load:LoadState child:Question /> = { "ok" }
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"const valid = {
  user: { $type: "User", name: "Ada" },
  load: { $type: "LoadState.ready", label: "Ready" },
  child: { $type: "Question", label: "Name" }
};
console.log(JSON.stringify({
  record: m.HostSchema.tryEvaluateJson({ ...valid, user: { $type: "User", name: 1 } }),
  union: m.HostSchema.tryEvaluateJson({ ...valid, load: { $type: "LoadState.missing", label: "Ready" } }),
  component: m.HostSchema.tryEvaluateJson({ ...valid, child: { $type: "Question", label: "Name", extra: "nope" } })
}));"#,
    );
    let output: Value = serde_json::from_str(&output).expect("tryEvaluate output");

    assert_eq!(output["record"]["ok"], false);
    assert_eq!(output["union"]["ok"], false);
    assert_eq!(output["component"]["ok"], false);
    assert_eq!(output["record"]["diagnostics"][0]["code"], "invalid-field");
    assert_eq!(output["union"]["diagnostics"][0]["code"], "invalid-union");
    assert_eq!(
        output["component"]["diagnostics"][0]["code"],
        "unknown-field"
    );
    assert!(output["record"]["diagnostics"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("Host props.user.name")));
    assert!(output["union"]["diagnostics"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("LoadState.missing")));
    assert!(output["component"]["diagnostics"][0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("unknown field 'extra'")));
}

#[test]
fn generated_function_element_calls_remain_eager() {
    let source = r#"
external component <Question label:string />
let MakeQuestion(label:string) = { <Question label={label} /> }
let root() = { <MakeQuestion label="Name" /> }
"#;
    let output = execute_generated_javascript_root(source);

    assert_json_values_eq(&output, r#"{ "$type": "Question", "label": "Name" }"#);
}

#[test]
fn component_action_handler_bindings_fail_before_emission() {
    let artifact = artifact_from_source(
        r#"
external component <TextInput />
component <SearchBox emits { SearchSubmitted { query:string } } /> = { <TextInput /> }
action DoSearch = { query:string }
let root() = { <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> /> }
"#,
    );
    let error = emit_program(&artifact, &CodegenOptions::javascript()).expect_err("codegen error");

    assert!(error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error
            && diagnostic.code() == Some("codegen-unsupported-construct")
            && diagnostic
                .message()
                .contains("action-handler codegen is not supported")));
}

/// Generated JavaScript and TypeScript for a program whose only difference is how its closed set
/// of constants is declared. The bodies carry the emitted schema; the NX IR half of this guard
/// lives with the IR tests.
fn constant_set_outputs(declaration: &str) -> (String, String) {
    let source = format!(
        r#"{declaration}
export component <Box fit:Fit /> = {{ <div /> }}
let root() = {{ <Box fit=cover /> }}
"#
    );
    let artifact = artifact_from_source(&source);
    let javascript = generated_module_body(&generated_file(
        &artifact,
        CodegenTarget::JavaScript,
        "m0_main.js",
    ))
    .to_string();
    let typescript = generated_module_body(&generated_file(
        &artifact,
        CodegenTarget::TypeScript,
        "m0_main.ts",
    ))
    .to_string();
    (javascript, typescript)
}

/// The optional leading `|` is purely syntactic: both spellings of the same constant union
/// generate identical output.
///
/// This began as the guard that an `enum` and the equivalent `type` generate the same thing —
/// which is what the unification had to establish. With `enum` removed there is one keyword left,
/// so what remains to guard is D2's two case-list spellings.
#[test]
fn both_case_list_spellings_generate_identical_javascript_and_typescript() {
    let (bare_js, bare_ts) = constant_set_outputs("export type Fit = fill | contain | cover");
    let (piped_js, piped_ts) = constant_set_outputs("export type Fit = fill | contain | cover");

    assert_eq!(bare_js, piped_js, "generated JavaScript and its schema");
    assert_eq!(bare_ts, piped_ts, "generated TypeScript and its schema");
}

// ------------------------------------------------------------------------------------------------
// Derived update records
// ------------------------------------------------------------------------------------------------

#[test]
fn generated_javascript_keeps_absent_update_fields_absent() {
    let output = execute_generated_javascript_root(
        r#"
type User = { name:string = "anon" email:string? }
let root() = <User.Update email={null} />
"#,
    );

    assert_json_values_eq(&output, r#"{ "$type": "User.Update", "email": null }"#);
}

#[test]
fn generated_javascript_constructs_a_component_update_record_from_its_state() {
    let output = execute_generated_javascript_root(
        r#"
component <Counter /> = { state { count:int = 0 label:string = "x" } <Label /> }
let root() = <Counter.Update count=1 />
"#,
    );

    assert_json_values_eq(&output, r#"{ "$type": "Counter.Update", "count": 1 }"#);
}

// ------------------------------------------------------------------------------------------------
// Derived property unions and update intrinsics
// ------------------------------------------------------------------------------------------------

#[test]
fn generated_javascript_serializes_a_property_case_as_its_field_name() {
    let source = r#"
type User = { name:string email:string? }
let root() = <Box key={User.Property.email} />
"#;
    let output = execute_generated_javascript_root(source);
    assert!(output.contains(r#""key":"email""#), "{}", output);
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_applies_an_update_through_the_runtime() {
    let source = r#"
type User = { name:string email:string? }
let root() = {apply(<User name="Ada" email="x@y" />, <User.Update email={null} />)}
"#;
    let artifact = artifact_from_source(source);
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(module.contains("nxApplyUpdate("), "{}", module);
    assert!(
        module.contains("nxApplyUpdate") && module.contains("./nx-runtime.js"),
        "the operation is reached through the supplied runtime: {}",
        module
    );

    let output = execute_generated_javascript_artifact_root(&artifact, "");
    assert_json_values_eq(
        &output,
        r#"{ "$type": "User", "name": "Ada", "email": null }"#,
    );
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_lists_changed_fields_in_declaration_order() {
    let source = r#"
type User = { name:string email:string? age:int? }
let root() = {changed(<User.Update age={null} name="Ada" />)}
"#;
    let output = execute_generated_javascript_root(source);
    assert_json_values_eq(&output, r#"["name", "age"]"#);
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_merges_and_diffs_like_the_interpreter() {
    let cases = [
        r#"
type User = { name:string email:string? age:int? }
let root() = {changed(merge(<User.Update age={null} />, <User.Update name="Ada" />))}
"#,
        r#"
type Address = { city:string }
type User = { name:string tags:string[] home:Address }
let root() = {diff(<User name="Ada" tags={ "x" "y" } home=<Address city="Paris" /> />, <User name="Ada" tags={ "y" "x" } home=<Address city="Rome" /> />)}
"#,
        r#"
type User = { name:string email:string? }
let root() = {changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))}
"#,
    ];
    for source in cases {
        let output = execute_generated_javascript_root(source);
        assert_json_values_eq(&output, &interpreter_json_root(source));
    }
}

/// An intrinsic call nested in an element property still imports its runtime helper.
#[test]
fn generated_javascript_calls_an_intrinsic_inside_an_element_property() {
    let source = r#"
type User = { name:string email:string? age:int? }
let root() = <Box keys={changed(<User.Update age={null} name="Ada" />)} />
"#;
    let artifact = artifact_from_source(source);
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(
        module.contains("import { nxChangedFields, nxElement }"),
        "{}",
        module
    );

    let output = execute_generated_javascript_artifact_root(&artifact, "");
    assert_json_values_eq(&output, r#"{ "$type": "Box", "keys": ["name", "age"] }"#);
    assert_json_values_eq(&output, &interpreter_json_artifact_root(&artifact));
}

/// A property union and an update record reached through a workspace import resolve to the
/// declaring module's declarations, and `changed` carries the target's field order with it.
#[test]
fn generated_javascript_reaches_property_unions_and_intrinsics_across_workspace_modules() {
    let artifact = artifact_from_workspace(
        &[
            (
                "data.nx",
                r#"
export abstract type Named = { name:string }
export type User extends Named = { email:string? age:int? }
"#,
            ),
            (
                "main.nx",
                r#"
import { User } from "./data.nx"
let root() = <Box first={User.Property.name} keys={changed(<User.Update age={null} name="Ada" />)} />
"#,
            ),
        ],
        "main.nx",
    );
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m1_main.js");
    assert!(
        module.contains(r#"["name", "email", "age"]"#),
        "the declared field order travels with the call: {}",
        module
    );
    assert!(
        module.contains("import { User_Property as m0_User_Property }"),
        "{}",
        module
    );

    let output = execute_generated_javascript_artifact_root(&artifact, "");
    assert_json_values_eq(
        &output,
        r#"{ "$type": "Box", "first": "name", "keys": ["name", "age"] }"#,
    );
    assert_json_values_eq(&output, &interpreter_json_artifact_root(&artifact));
}

/// The JavaScript and TypeScript runtime modules are two hand-maintained copies, and nothing in
/// generated-program execution reaches every branch of their intrinsic helpers. One script run
/// against both pins each helper to the interpreter's answer for the same values — a record
/// missing an optional field, a merged `null`, an order-less `changed` — and fails the moment the
/// copies disagree.
/// Dividing by zero fails on every backend. Generated JavaScript would otherwise return `Infinity`
/// or `NaN`, where the interpreter and the IR runtime raise a run-time error.
#[test]
fn generated_javascript_fails_on_a_division_by_zero_like_the_interpreter() {
    let artifact = artifact_from_source(
        "let intDiv(n:int, m:int) = { n / m }\n\
         let intMod(n:int, m:int) = { n % m }\n\
         let div(x:float64, y:float64) = { x / y }\n\
         let f32Div(a:float32, b:float32) = { a / b }\n\
         let root() = { 1 }",
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"
const out = [];
for (const call of [() => m.intDiv(1, 0), () => m.intMod(1, 0), () => m.div(1.0, 0.0), () => m.f32Div(1.0, 0.0)]) {
  try {
    out.push(call());
  } catch (error) {
    out.push(error.message ?? String(error));
  }
}
console.log(JSON.stringify(out));
"#,
    );
    assert_json_values_eq(
        &output,
        r#"["Division by zero", "Division by zero", "Division by zero", "Division by zero"]"#,
    );
}

#[test]
fn emitted_runtime_intrinsic_helpers_agree_across_targets() {
    let script = r#"
import * as rt from "./nx-runtime.js";
const user = { $type: "User", name: "Ada", email: "x@y" };
const partial = { $type: "User", name: "Ada" };
const out = [];
out.push(rt.nxApplyUpdate(user, { $type: "User.Update", email: null }));
out.push(rt.nxMergeUpdates({ $type: "User.Update", name: "Ada", email: "x@y" }, { $type: "User.Update", email: null }));
out.push(rt.nxDiffRecords(user, partial));
out.push(rt.nxDiffRecords(partial, user));
out.push(rt.nxDiffRecords(user, { $type: "User", name: "Ada", email: "x@y" }));
out.push(rt.nxChangedFields({ $type: "User.Update", email: null, name: "Ada" }, ["name", "email", "age"]));
try {
  rt.nxChangedFields({ $type: "User.Update", name: "Ada" });
  out.push("no error");
} catch (error) {
  out.push(error instanceof rt.NxRuntimeError ? error.message : String(error));
}
console.log(JSON.stringify(out));
"#;
    let expected = r#"[
        { "$type": "User", "name": "Ada", "email": null },
        { "$type": "User.Update", "name": "Ada", "email": null },
        { "$type": "User.Update", "email": null },
        { "$type": "User.Update", "email": "x@y" },
        { "$type": "User.Update" },
        ["name", "email"],
        "nxChangedFields needs the update record's declared field order"
    ]"#;

    let mut outputs = Vec::new();
    for target in [CodegenTarget::JavaScript, CodegenTarget::TypeScript] {
        let output = execute_script_against_emitted_runtime(target, script);
        assert_json_values_eq(&output, expected);
        outputs.push(output);
    }
    assert_json_values_eq(&outputs[0], &outputs[1]);
}

#[test]
fn generated_typescript_with_property_references_and_intrinsics_type_checks() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string email:string? age:int? }
let key(): User.Property = {User.Property.email}
let applied(): User = {apply(<User name="Ada" />, <User.Update email={null} />)}
let merged(): User.Update = {merge(<User.Update name="Ada" />, <User.Update age=1 />)}
let changedKeys(): User.Property[] = {changed(diff(<User name="Ada" />, <User name="Bo" />))}
"#,
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    assert!(module.contains("nxChangedFields("), "{}", module);
    assert_generated_typescript_artifact_type_checks(&artifact);
}

// ---------------------------------------------------------------------------------------------
// Component type parameters
// ---------------------------------------------------------------------------------------------

const GENERIC_LAYOUT: &str = "type Contact = { name:string }\n\
    external component <SkiaLayout TItem:type itemsSource:TItem[]? />\n\
    let v = <SkiaLayout TItem=Contact itemsSource={} />\n\
    let root() = { v }";

/// The executable TypeScript carries the parameter where a caller names the instantiation — the
/// `Props` type and the factory — and erases it on the serializable element type.
#[test]
fn generated_typescript_carries_a_type_parameter_generically_and_erases_it_on_the_element() {
    let artifact = artifact_from_source(GENERIC_LAYOUT);
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(
        module.contains("type SkiaLayoutProps<TItem = unknown> = {\n  itemsSource?: readonly TItem[] | null;\n};"),
        "{module}"
    );
    assert!(
        module.contains("function SkiaLayout<TItem = unknown>(props: SkiaLayoutProps<TItem> = {}): SkiaLayoutElement {"),
        "{module}"
    );
    assert!(
        module.contains("type SkiaLayoutElement = {\n  readonly $type: \"SkiaLayout\";\n  readonly itemsSource: readonly unknown[] | null;\n};"),
        "{module}"
    );
    assert!(
        !module
            .lines()
            .any(|line| line.contains("TItem:") || line.contains("TItem?:")),
        "neither type may have a member for the parameter:\n{module}"
    );

    assert_generated_typescript_artifact_type_checks(&artifact);
}

/// A declared generic record is a real generic in the executable TypeScript, and an applied type
/// is its instantiation. Unlike a component, nothing about it is erased here: the record's own
/// values are NX values the generated module builds and reads.
#[test]
fn generated_typescript_emits_a_generic_record_and_its_instantiation() {
    let artifact = artifact_from_source(
        "type Range = { T:type start:T end:T }\n\
         type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }\n\
         let s = <Slider range={<Range T=float64 start={0} end={1} />} />\n\
         let root() = { s }",
    );
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(
        module.contains(
            "type Range<T> = {\n  readonly $type: \"Range\";\n  readonly start: T;\n  readonly end: T;\n};"
        ),
        "{module}"
    );
    assert!(
        module.contains("readonly range: Range<number>;"),
        "{module}"
    );
    assert!(
        module.contains("readonly marks?: readonly Range<number>[] | null;"),
        "{module}"
    );
    assert!(
        !module.lines().any(|line| line.contains("readonly T:")),
        "the record must have no member for its parameter:\n{module}"
    );

    assert_generated_typescript_artifact_type_checks(&artifact);
}

/// A TypeScript caller gets the parameter inferred from the props it passes, and pays nothing at
/// runtime: the factory returns the erased element.
#[test]
fn generated_typescript_infers_a_type_argument_from_the_factory_call() {
    let artifact = artifact_from_source(GENERIC_LAYOUT);
    let output = emit_program(&artifact, &CodegenOptions::typescript()).expect("ts output");
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("package.json"), r#"{ "type": "module" }"#).expect("package file");
    for file in output.files {
        fs::write(dir.path().join(file.relative_path), file.content).expect("generated file");
    }
    fs::write(
        dir.path().join("usage.ts"),
        r#"import { SkiaLayout, type SkiaLayoutProps } from "./m0_main.js";

const props: SkiaLayoutProps<{ name: string }> = { itemsSource: [{ name: "a" }] };
const typed = SkiaLayout(props);
const inferred = SkiaLayout({ itemsSource: [{ name: "a" }] });
const bare = SkiaLayout({});
const erasedProps: SkiaLayoutProps = { itemsSource: [1, "two"] };
const types: ["SkiaLayout", "SkiaLayout", "SkiaLayout"] = [typed.$type, inferred.$type, bare.$type];
const items: readonly unknown[] | null = inferred.itemsSource;
// A wrong item type does not fit the named instantiation.
// @ts-expect-error
const mismatched: SkiaLayoutProps<{ name: string }> = { itemsSource: [1] };
export { types, items, erasedProps, mismatched };
"#,
    )
    .expect("usage file");

    let output = tsc_command()
        .current_dir(dir.path())
        .args([
            "--noEmit",
            "--module",
            "NodeNext",
            "--moduleResolution",
            "NodeNext",
            "--target",
            "ES2020",
            "--strict",
            "usage.ts",
        ])
        .output()
        .expect("tsc execution");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rendered = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify([m.SkiaLayout({}).$type, m.root()]));",
    );
    assert_json_values_eq(
        &rendered,
        r#"["SkiaLayout", { "$type": "SkiaLayout", "itemsSource": [] }]"#,
    );
}

const GENERIC_STATEFUL_LIST: &str = "external component <Label text:string? />\n\
    component <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }\n";

/// State is a snapshot the host holds as data, and its instantiation was fixed at an NX use site
/// the host never sees, so the state type erases the parameter the way the element type does.
#[test]
fn generated_typescript_erases_a_type_parameter_on_the_state_type() {
    let artifact = artifact_from_source(&format!(
        "{GENERIC_STATEFUL_LIST}let root() = {{ <List items={{}} /> }}"
    ));
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(
        module.contains("type ListState = {\n  readonly sel: unknown | null;\n};"),
        "{module}"
    );
    assert!(
        !module
            .lines()
            .any(|line| line.contains("TItem:") || line.contains("TItem?:")),
        "no type may have a member for the parameter:\n{module}"
    );

    assert_generated_typescript_artifact_type_checks(&artifact);
}

/// The update record copies the state annotations, and outside the component the parameter is not
/// a type, so the record erases it to the top type in every target rather than failing to build.
#[test]
fn an_update_record_of_a_generic_component_erases_the_parameter() {
    let artifact = artifact_from_source(&format!(
        "{GENERIC_STATEFUL_LIST}let u = <List.Update sel=null />\nlet root() = {{ u }}"
    ));
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(
        module.contains(
            "type List_Update = {\n  readonly $type: \"List.Update\";\n  readonly sel?: object | null;\n};"
        ),
        "{module}"
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

/// An optional nullable prop is `T | null | undefined` on the input and `T | null` once resolved:
/// a key present with no value resolves to `null`, the same as an absent key. The rule is the
/// resolver's, not a generic component's, so it is pinned on an ordinary component.
#[test]
fn a_nullable_prop_present_with_no_value_resolves_to_null() {
    let artifact =
        artifact_from_source("external component <Box label:string? />\nlet root() = { <Box /> }");
    let rendered = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify([m.Box({ label: undefined }), m.Box({})]));",
    );
    assert_json_values_eq(
        &rendered,
        r#"[{ "$type": "Box", "label": null }, { "$type": "Box", "label": null }]"#,
    );
}
