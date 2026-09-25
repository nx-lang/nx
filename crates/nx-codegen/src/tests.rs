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

/// A JavaScript snippet as TypeScript writes it: each `$type: "Name"` entry followed by `as const`.
fn typescript_type_tags(snippet: &str) -> String {
    let mut output = String::new();
    let mut rest = snippet;
    while let Some(start) = rest.find("$type: \"") {
        let value_start = start + "$type: \"".len();
        let value_end = value_start + rest[value_start..].find('"').expect("closing quote") + 1;
        output.push_str(&rest[..value_end]);
        output.push_str(" as const");
        rest = &rest[value_end..];
    }
    output.push_str(rest);
    output
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

/// Generated output against the interpreter's. A generated function returns an empty `T?` result
/// as `nxEmpty`, as its TypeScript signature says, where the interpreter's entry call returns the
/// host's `null`.
fn assert_json_values_eq(actual: &str, expected: &str) {
    let actual: Value = serde_json::from_str(actual).expect("actual generated json");
    let expected: Value = serde_json::from_str(expected).expect("expected interpreter json");
    if expected == Value::Null && actual == Value::Array(Vec::new()) {
        return;
    }
    assert_eq!(actual, expected);
}

#[test]
fn builds_codegen_program_from_inline_artifact() {
    let artifact = artifact_from_source("let root() = { 1 + 2 }");
    let program = build_codegen_program(&artifact).expect("codegen program");

    assert_eq!(program.fingerprint, artifact.fingerprint);
    assert!(program.entrypoint("root").is_some());
    // The program's own module, beside the prelude, whose source is part of every program's.
    let identities = program
        .source_entries
        .iter()
        .map(|entry| entry.identity.as_str())
        .collect::<Vec<_>>();
    assert_eq!(identities, [nx_hir::PRELUDE_MODULE_IDENTITY, "main.nx"]);
    assert_eq!(
        artifact.source_text("main.nx"),
        Some("let root() = { 1 + 2 }")
    );
    assert_eq!(
        artifact
            .source_entries()
            .iter()
            .find(|entry| entry.identity == "main.nx")
            .map(|entry| entry.source),
        Some("let root() = { 1 + 2 }")
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
            "import { Swatch } from \"./model.nx\"\n             type Hue = Blue | Violet\n             external component <Paint colour?:Hue />\n             abstract external component <Node />\n             component <Chip extends Node s:Swatch /> = { <Paint colour={s.hue} /> }\n             let root() = { <Chip s=<Swatch hue={Swatch} /> /> }",
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

/// A written-but-empty body is the empty value, so on an optional content property it binds what
/// an absent body binds, and on a defaulted one it is refused rather than displacing the default.
/// The interpreter and code generation must agree on both.
///
/// <para>The interpreter keys on whether any content expression was written
/// (`normalize_content_values` in `nx-interpreter`); code generation keys on whether the emitted
/// content list is non-empty, in three separate places across two crates. Nothing but this test
/// holds them together.</para>
#[test]
fn an_empty_written_body_binds_the_empty_value_and_an_absent_one_takes_the_default() {
    const WRITTEN: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items?: A+ }
        let root() = { <Box>{}</Box> }
    "#;
    const OMITTED: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items?: A+ }
        let root() = { <Box /> }
    "#;
    const ABSENT: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items: A+ = {<A n=9 />} }
        let root() = { <Box /> }
    "#;
    const REFUSED: &str = r#"
        type A = { n: int = 1 }
        type Box = { content items: A+ = {<A n=9 />} }
        let root() = { <Box>{}</Box> }
    "#;

    // An empty optional field is an omitted key, whether the body was written empty or left out.
    for source in [WRITTEN, OMITTED] {
        assert_json_values_eq(&interpreter_json_root(source), r#"{"$type":"Box"}"#);
        assert_json_values_eq(
            &execute_generated_javascript_root(source),
            &interpreter_json_root(source),
        );
    }

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
    assert_json_values_eq(
        &execute_generated_javascript_root(ABSENT),
        &interpreter_json_root(ABSENT),
    );

    // The empty value does not satisfy `A+`, so a written-empty body cannot reach a defaulted
    // `+` property: analysis refuses it before either engine sees it.
    let error = emit_program(
        &artifact_from_source(REFUSED),
        &CodegenOptions::javascript(),
    )
    .expect_err("a written-empty body at a defaulted `+` property is refused");
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.message().contains("expects A+") && diagnostic.message().contains("found {}")
        }),
        "{:?}",
        error.diagnostics
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
let root(items: int+): int+ = { for item in items { item + answer } }"#,
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
    // A `for` concatenates what its body yields, so it splices rather than maps, and it iterates
    // through `nxItems`, the one helper this module needs from the runtime.
    assert!(
        module.contains("return nxItems(items).flatMap((item, _index) => (item + m1_answer));"),
        "{module}"
    );
    assert!(
        module.contains("import { nxItems } from \"./nx-runtime.js\";"),
        "{module}"
    );
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
fn a_type_alias_named_by_a_signature_or_field_is_declared() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { Name, Maybe, People } from "../shared/types.nx"
type Ints = int+
type R = { n:Name ps:People xs:Ints }
let count(ps:People, n:Name, xs:Ints): Maybe = {}
let root() = { 1 }"#,
            ),
            (
                "shared/types.nx",
                r#"export type Person = { name:string }
export type Name = string
export type Maybe = string?
export type People = Person+"#,
            ),
        ],
        "app/main.nx",
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    assert!(
        module.contains("type Ints = readonly number[];"),
        "{module}"
    );
}

#[test]
fn a_record_extending_an_abstract_base_is_accepted_where_the_base_is_expected() {
    let artifact = artifact_from_source(
        r#"abstract type Shape = { id:int }
type Circle extends Shape = { r:int }
type Square extends Shape = { side:int = 1 }
type Badge extends Shape = | icon { glyph:string } | dot
let idOf(s:Shape): int = { s.id }
let radius(c:Circle): int = { c.r }
let root() = {
  idOf(<Circle id={1} r={2} />) + idOf(<Square id={2} />) + idOf(<Badge.icon id={3} glyph="x" />)
    + radius(<Circle id={4} r={5} />)
}"#,
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    for expected in [
        "export interface Shape {\n  readonly $type: string;",
        "export interface Circle extends Shape {\n  readonly $type: \"Circle\";",
        "export interface Badge_icon extends Shape {",
        "satisfies Circle as Circle",
        "satisfies Badge_icon as Badge_icon",
    ] {
        assert!(
            module.contains(expected),
            "expected {expected:?} in:\n{module}"
        );
    }
    // A plain literal is pinned only in TypeScript.
    let javascript = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(!javascript.contains("satisfies"), "{javascript}");
}

#[test]
fn a_record_extending_a_base_from_another_module_is_accepted_there() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { Shape, idOf, Badge } from "../lib/shapes.nx"
type Circle extends Shape = { r:int }
let local(s:Shape): int = { s.id }
let root() = { idOf(<Circle id={1} r={2} />) + local(<Badge.icon id={3} glyph="x" />) }"#,
            ),
            (
                "lib/shapes.nx",
                r#"export abstract type Shape = { id:int }
export let idOf(s:Shape): int = { s.id }
export type Badge extends Shape = | icon { glyph:string } | dot"#,
            ),
        ],
        "app/main.nx",
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    for expected in [
        "import type { Shape as m1_Shape }",
        "export interface Circle extends m1_Shape {",
        "satisfies Extract<m1_Badge, { readonly $type: \"Badge.icon\" }>",
    ] {
        assert!(
            module.contains(expected),
            "expected {expected:?} in:\n{module}"
        );
    }
}

#[test]
fn an_inferred_type_naming_a_derived_declaration_emits_it() {
    // Neither `Person.Property` nor `Person.Update` is written: each is only the inferred result
    // type of an update intrinsic, which the signature prints.
    let artifact = artifact_from_source(
        r#"type Person = { name:string age?:int }
let edited(a:Person, b:Person) = { changed(diff(a, b)) }
let delta(a:Person, b:Person) = { diff(a, b) }
let root() = { edited(<Person name="a" />, <Person name="b" />) }"#,
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
                    declared_return_type: None,
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
        optional: false,
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
type User = { name: string tags: string+ age: int }
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
        "return ({ $type: \"User\" as const, name: \"Ada\", tags: [\"admin\", \"editor\"], age: 42 });"
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
            // Items splice: the flattening is what a sequence-valued item is spliced by, and it
            // is written for every sequence so that one rule covers them all.
            "array",
            "let root(): int+ = { 1 2 3 }",
            "return [1, 2, 3];",
        ),
        (
            "array that may be empty",
            "let root(): int* = { 1 2 3 }",
            "return [1, 2, 3];",
        ),
        (
            "loop",
            "let root(items:int+) = { for item, index in items { item + index } }",
            "nxItems(items).flatMap((item, index) => (item + index))",
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

        // TypeScript writes an object literal's `$type` `as const`, so it keeps its literal type
        // wherever the object is not contextually typed.
        let ts_expected = typescript_type_tags(expected);
        assert!(
            ts_module.contains(&ts_expected),
            "missing TS snippet: {ts_expected}"
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

/// The flat sequence model in generated JavaScript: items splice, and a conditional with no
/// `else` contributes nothing where it is not taken.
#[test]
fn generated_javascript_splices_items_and_drops_an_untaken_conditional() {
    let cases = [
        // Two sequences in a braced value concatenate.
        (
            r#"let xs:string+ = {"a" "b"}
let ys:string+ = {"c"}
let root(): string+ = { xs ys }"#,
            "[\"a\",\"b\",\"c\"]",
        ),
        // A sequence and an item share one sequence.
        (
            r#"let xs:string+ = {"a" "b"}
let root(): string+ = { xs "c" }"#,
            "[\"a\",\"b\",\"c\"]",
        ),
        // A `for` concatenates what its body yields.
        (
            r#"type Row = { cells:int+ }
let rows:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }
let root(): int+ = { for r in rows { r.cells } }"#,
            "[1,2,3,4]",
        ),
        // A conditional body contributes on the iterations it is taken, and nothing on the rest.
        (
            r#"let ns:int+ = {1 2 3 4}
let root(): int* = { for n in ns { if (n % 2 == 0) { n } } }"#,
            "[2,4]",
        ),
        // An empty item is the empty sequence, so the splice drops it.
        (
            r#"let none(): string? = {}
let root(): string* = { "a" none() }"#,
            "[\"a\"]",
        ),
        // An optional binding that holds an item is that item, and one that is empty splices
        // to nothing.
        (
            r#"let o:string? = "b"
let e:string? = {}
let root(): string* = { "a" o e }"#,
            "[\"a\",\"b\"]",
        ),
    ];

    for (source, expected) in cases {
        let artifact = artifact_from_source(source);
        assert_eq!(
            execute_generated_javascript_artifact_root(&artifact, ""),
            expected,
            "for source: {source}"
        );
    }
}

#[test]
fn an_untaken_conditional_child_emits_no_content_item() {
    let source = r#"type A = { n:int = 1 }
type Box = { content items:A+ }
let c = false
let root(): Box = { <Box><A/>{if c { <A/> }}</Box> }"#;
    let artifact = artifact_from_source(source);

    // The untaken branch is the empty list, so the splice drops it with no element form of its own.
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(
        module.contains(": nxEmpty)"),
        "expected an empty-list branch: {module}"
    );
    assert!(
        !module.contains(": null)"),
        "no null element form: {module}"
    );

    let output = execute_generated_javascript_artifact_root(&artifact, "");
    assert!(
        !output.contains("null"),
        "an untaken conditional contributes no items: {output}"
    );
    assert_eq!(
        output.matches("\"$type\"").count(),
        2,
        "one Box and one A: {output}"
    );
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
external component <ShortTextQuestion extends Question placeholder?:string />
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
        r#"{ "$type": "ShortTextQuestion", "label": "Untitled" }"#,
    );
    assert_json_values_eq(&output, &interpreter_json_artifact_root(&artifact));
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
let textInputs(): TextInput+ = {
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
fn generated_component_entry_handles_constant_cases_optional_fields_and_lists() {
    let artifact = artifact_from_source(
        r#"
type Mode = exact | fuzzy
external component <Summary mode:Mode tags:string+ note?:string />
component <SearchBox tags:string+ mode:Mode = { Mode.exact } /> = {
  state {
    query?:string
    tags:string+ = {tags}
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

    // An empty optional state field reaches the descriptor as an omitted key.
    assert_json_values_eq(
        &output,
        r#"{
  "omitted": { "$type": "Summary", "mode": "exact", "tags": ["nx"] },
  "explicit": { "$type": "Summary", "mode": "fuzzy", "tags": ["ui"], "note": "docs" }
}"#,
    );
}

/// The schema of a `+` prop refuses the host's empty array and `null`, where a `?:T+` prop reads
/// both as the empty value, as every decoder does at a `?` or `*` site, and an empty optional
/// prop is an omitted key.
#[test]
fn generated_schema_boundaries_decode_host_absence_by_occurrence() {
    let artifact = artifact_from_source(
        r#"
external component <Summary items:string+ tags?:string+ subtitle?:string />
let root() = { 1 }
"#,
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"console.log(JSON.stringify({
  absent: m.SummarySchema.fromJson({ items: ["a"] }),
  written: m.SummarySchema.fromJson({ items: ["a"], tags: [], subtitle: null }),
  emptyItems: m.SummarySchema.tryFromJson({ items: [] }),
  nullItems: m.SummarySchema.tryFromJson({ items: null })
}));"#,
    );
    let output: Value = serde_json::from_str(&output).expect("schema boundary output");

    assert_eq!(
        output["absent"],
        serde_json::json!({ "$type": "Summary", "items": ["a"] })
    );
    assert_eq!(output["written"], output["absent"]);
    assert_eq!(output["emptyItems"]["ok"], false);
    assert!(
        output["emptyItems"]["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("items")),
        "{output}"
    );
    assert_eq!(output["nullItems"]["ok"], false);
    assert!(
        output["nullItems"]["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("items")),
        "{output}"
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
type User = { name:string = "anon" email?:string }
let root() = <User.Update email={} />
"#,
    );

    // A present empty field is `null`, the one place the canonical encoding writes it.
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
type User = { name:string email?:string }
let root() = <Box key={User.Property.email} />
"#;
    let output = execute_generated_javascript_root(source);
    assert!(output.contains(r#""key":"email""#), "{}", output);
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_applies_an_update_through_the_runtime() {
    let source = r#"
type User = { name:string email?:string }
let root() = {apply(<User name="Ada" email="x@y" />, <User.Update email={} />)}
"#;
    let artifact = artifact_from_source(source);
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(module.contains("nxApplyUpdate("), "{}", module);
    assert!(
        module.contains("nxApplyUpdate") && module.contains("./nx-runtime.js"),
        "the operation is reached through the supplied runtime: {}",
        module
    );

    // The cleared field is absent from the result's keys.
    let output = execute_generated_javascript_artifact_root(&artifact, "");
    assert_json_values_eq(&output, r#"{ "$type": "User", "name": "Ada" }"#);
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_lists_changed_fields_in_declaration_order() {
    let source = r#"
type User = { name:string email?:string age?:int }
let root() = {changed(<User.Update age={} name="Ada" />)}
"#;
    let output = execute_generated_javascript_root(source);
    assert_json_values_eq(&output, r#"["name", "age"]"#);
    assert_json_values_eq(&output, &interpreter_json_root(source));
}

#[test]
fn generated_javascript_merges_and_diffs_like_the_interpreter() {
    let cases = [
        r#"
type User = { name:string email?:string age?:int }
let root() = {changed(merge(<User.Update age={} />, <User.Update name="Ada" />))}
"#,
        r#"
type Address = { city:string }
type User = { name:string tags:string+ home:Address }
let root() = {diff(<User name="Ada" tags={ "x" "y" } home=<Address city="Paris" /> />, <User name="Ada" tags={ "y" "x" } home=<Address city="Rome" /> />)}
"#,
        r#"
type User = { name:string email?:string }
let root() = {changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))}
"#,
        // A field cleared between the two records is a present empty field of the diff.
        r#"
type User = { name:string email?:string }
let root() = {diff(<User name="Ada" email="x@y" />, <User name="Ada" />)}
"#,
        // A merge keeps a present empty field present, so it still clears when applied.
        r#"
type User = { name:string email?:string }
let root() = {apply(<User name="Ada" email="x@y" />, merge(<User.Update name="Bo" />, <User.Update email={} />))}
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
type User = { name:string email?:string age?:int }
let root() = <Box keys={changed(<User.Update age={} name="Ada" />)} />
"#;
    let artifact = artifact_from_source(source);
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    // `nxCleared` writes the update record's present fields, `null` for the cleared one.
    assert!(
        module.contains("import { nxChangedFields, nxCleared, nxElement }"),
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
export type User extends Named = { email?:string age?:int }
"#,
            ),
            (
                "main.nx",
                r#"
import { User } from "./data.nx"
let root() = <Box first={User.Property.name} keys={changed(<User.Update age={} name="Ada" />)} />
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

/// The JavaScript and TypeScript runtime modules are two hand-maintained copies, and nothing in
/// generated-program execution reaches every branch of their intrinsic helpers. One script run
/// against both pins each helper to the interpreter's answer for the same values — a record
/// missing an optional field, a merged `null`, an order-less `changed` — and fails the moment the
/// copies disagree.
#[test]
fn emitted_runtime_intrinsic_helpers_agree_across_targets() {
    let script = r#"
import * as rt from "./nx-runtime.js";
const user = { $type: "User", name: "Ada", email: "x@y" };
const partial = { $type: "User", name: "Ada" };
const out = [];
out.push(rt.nxApplyUpdate(user, { $type: "User.Update", email: null }));
out.push(rt.nxApplyUpdate(user, { $type: "User.Update", email: [] }));
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
// The presence operators over the one empty representation and its host synonyms.
out.push([rt.nxExists("x"), rt.nxExists([]), rt.nxExists(null), rt.nxExists(undefined), rt.nxExists(["x"])]);
out.push([rt.nxStep([], "name"), rt.nxStep({ name: "Ada" }, "name"), rt.nxStep({ $type: "User" }, "email")]);
let ran = false;
out.push([rt.nxCoalesce("x", () => { ran = true; return "y"; }), ran, rt.nxCoalesce([], () => "y")]);
out.push([rt.nxItems([]), rt.nxItems("x"), rt.nxItems(["x", "y"])]);
out.push([rt.nxOptional("k", []), rt.nxOptional("k", "v"), rt.nxOptional("k", ["v"])]);
out.push([rt.nxCleared([]), rt.nxCleared("v")]);
console.log(JSON.stringify(out));
"#;
    // Applying a present empty field leaves the field absent from the result's keys; a merge and
    // a diff carry it as present and `null`.
    let expected = r#"[
        { "$type": "User", "name": "Ada" },
        { "$type": "User", "name": "Ada" },
        { "$type": "User.Update", "name": "Ada", "email": null },
        { "$type": "User.Update", "email": null },
        { "$type": "User.Update", "email": "x@y" },
        { "$type": "User.Update" },
        ["name", "email"],
        "nxChangedFields needs the update record's declared field order",
        [true, false, false, false, true],
        [[], "Ada", []],
        ["x", false, "y"],
        [[], ["x"], ["x", "y"]],
        [{}, { "k": "v" }, { "k": "v" }],
        [null, "v"]
    ]"#;

    let mut outputs = Vec::new();
    for target in [CodegenTarget::JavaScript, CodegenTarget::TypeScript] {
        let output = execute_script_against_emitted_runtime(target, script);
        assert_json_values_eq(&output, expected);
        outputs.push(output);
    }
    assert_json_values_eq(&outputs[0], &outputs[1]);
}

/// A range loop counts through the `nxRangeMap` helper rather than `Array.from`, because a range is
/// a record and not a JavaScript iterable. The values are the interpreter's.
#[test]
fn generated_code_counts_over_a_range_through_the_runtime_helper() {
    // A `for` over a range may yield nothing, so each is `int*`.
    let source = "let squares(): int* = { for i, n in 0..4 { i * i + n } }\n\
                  let pages(): int* = { for page in 1..=3 { page } }\n\
                  let none(): int* = { for i in 5..2 { i } }\n\
                  let root(): int* = { squares() }";
    let artifact = artifact_from_source(source);

    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");
    assert!(
        module.contains(
            "nxRangeMap(({ $type: \"Range\" as const, start: 0, end: 4, endInclusive: false }), (i, n) =>"
        ),
        "{module}"
    );
    assert!(
        !module.contains("Array.from({ $type: \"Range\""),
        "a range is not passed to Array.from:\n{module}"
    );
    assert_generated_typescript_artifact_type_checks(&artifact);

    let printed = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify([m.squares(), m.pages(), m.none()]));",
    );
    assert_json_values_eq(&printed, "[[0, 2, 6, 12], [1, 2, 3], []]");

    // The interpreter's answer for the same program, so the two backends are compared rather than
    // each pinned to a literal.
    let EvalResult::Ok(interpreted) = nx_api::eval_program_artifact(&artifact) else {
        panic!("the interpreter should evaluate the program");
    };
    assert_json_values_eq(
        &serde_json::to_string(&interpreted).expect("json"),
        "[0, 2, 6, 12]",
    );
}

#[test]
fn generated_typescript_with_property_references_and_intrinsics_type_checks() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string email?:string age?:int }
let key(): User.Property = {User.Property.email}
let applied(): User = {apply(<User name="Ada" />, <User.Update email={} />)}
let merged(): User.Update = {merge(<User.Update name="Ada" />, <User.Update age=1 />)}
let changedKeys(): User.Property* = {changed(diff(<User name="Ada" />, <User name="Bo" />))}
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
    external component <SkiaLayout TItem:type itemsSource?:TItem+ />\n\
    let v = <SkiaLayout TItem=Contact itemsSource={} />\n\
    let root() = { v }";

/// The executable TypeScript carries the parameter where a caller names the instantiation — the
/// `Props` type and the factory — and erases it on the serializable element type.
#[test]
fn generated_typescript_carries_a_type_parameter_generically_and_erases_it_on_the_element() {
    let artifact = artifact_from_source(GENERIC_LAYOUT);
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    // `?:TItem+` is an optional property whose value is an array; its read type `TItem*` is an
    // array that may be empty, so neither side mentions `null`.
    assert!(
        module.contains(
            "type SkiaLayoutProps<TItem = unknown> = {\n  itemsSource?: readonly TItem[];\n};"
        ),
        "{module}"
    );
    assert!(
        module.contains("function SkiaLayout<TItem = unknown>(props: SkiaLayoutProps<TItem> = {}): SkiaLayoutElement {"),
        "{module}"
    );
    assert!(
        module.contains("type SkiaLayoutElement = {\n  readonly $type: \"SkiaLayout\";\n  readonly itemsSource?: readonly unknown[];\n};"),
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
         type Slider = { range:<Range T=float64/> marks?:<Range T=int/>+ }\n\
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
        module.contains("readonly marks?: readonly Range<number>[];"),
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
const items: readonly unknown[] = inferred.itemsSource ?? [];
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
    assert_json_values_eq(&rendered, r#"["SkiaLayout", { "$type": "SkiaLayout" }]"#);
}

const GENERIC_STATEFUL_LIST: &str = "external component <Label text?:string />\n\
    component <List TItem:type items?:TItem+ /> = { state { sel?:TItem } <Label /> }\n";

/// State is a snapshot the host holds as data, and its instantiation was fixed at an NX use site
/// the host never sees, so the state type erases the parameter the way the element type does.
#[test]
fn generated_typescript_erases_a_type_parameter_on_the_state_type() {
    let artifact = artifact_from_source(&format!(
        "{GENERIC_STATEFUL_LIST}let root() = {{ <List items={{}} /> }}"
    ));
    let module = generated_file(&artifact, CodegenTarget::TypeScript, "m0_main.ts");

    assert!(
        module.contains("type ListState = {\n  readonly sel?: unknown;\n};"),
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
        "{GENERIC_STATEFUL_LIST}let u = <List.Update sel={{}} />\nlet root() = {{ u }}"
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

/// An optional prop's key present with no value, present with `null`, or present with an empty
/// array resolves to the empty value, the same as an absent key, and the descriptor carries no key
/// for it. The rule is the resolver's, not a generic component's, so it is pinned on an ordinary
/// component.
#[test]
fn an_optional_prop_present_with_no_value_resolves_to_the_empty_value() {
    let artifact =
        artifact_from_source("external component <Box label?:string />\nlet root() = { <Box /> }");
    let rendered = execute_generated_javascript_artifact_script(
        &artifact,
        "console.log(JSON.stringify([m.Box({ label: undefined }), m.Box({ label: null }), m.Box({ label: [] }), m.Box({}), m.root()]));",
    );
    assert_json_values_eq(
        &rendered,
        r#"[{ "$type": "Box" }, { "$type": "Box" }, { "$type": "Box" }, { "$type": "Box" }, { "$type": "Box" }]"#,
    );
    assert_json_values_eq(
        &execute_generated_javascript_artifact_root(&artifact, ""),
        &interpreter_json_artifact_root(&artifact),
    );
}

// ------------------------------------------------------------------------------------------------
// Presence operators
// ------------------------------------------------------------------------------------------------

/// The presence test, the step, the fallback and the `{}` pattern evaluate in generated JavaScript
/// to what the interpreter evaluates them to, over the one empty representation.
#[test]
fn generated_javascript_evaluates_the_presence_operators_like_the_interpreter() {
    let cases = [
        (
            r#"type Person = { name:string }
type Book = { title:string author?:Person }
let root() = { <Box has={<Book title="A" />.author?} /> }"#,
            r#"{ "$type": "Box", "has": false }"#,
        ),
        (
            r#"type Person = { name:string }
type Book = { title:string author?:Person }
let root() = { <Box has={<Book title="A" author=<Person name="Ada" /> />.author?} /> }"#,
            r#"{ "$type": "Box", "has": true }"#,
        ),
        (
            r#"type Person = { name:string }
type Book = { author?:Person }
let root() = { <Box name={<Book />.author?.name ?? "anonymous"} /> }"#,
            r#"{ "$type": "Box", "name": "anonymous" }"#,
        ),
        (
            r#"type Person = { name:string }
type Book = { author?:Person }
let root() = { <Box name={<Book author=<Person name="Ada" /> />.author?.name ?? "anonymous"} /> }"#,
            r#"{ "$type": "Box", "name": "Ada" }"#,
        ),
        // A `?:T+` field reads as `T*`: empty when omitted, the array when written.
        (
            r#"type Book = { tags?:string+ }
let root() = { <Box none={<Book />.tags?} some={<Book tags={"x" "y"} />.tags?} /> }"#,
            r#"{ "$type": "Box", "none": false, "some": true }"#,
        ),
        // `??` binds above arithmetic.
        (
            r#"let f(o?:int): int = { 10 + o ?? 5 }
let root() = { f({}) + f(1) }"#,
            "26",
        ),
    ];
    // The `{}` match pattern has no case here: executable source codegen does not support match
    // expressions, so the conformance corpus covers it through the IR runtime instead.
    for (source, expected) in cases {
        let output = execute_generated_javascript_root(source);
        assert_json_values_eq(&output, expected);
        assert_json_values_eq(&output, &interpreter_json_root(source));
    }
}

/// The fallback runs only when the left operand is empty.
#[test]
fn generated_javascript_evaluates_the_fallback_lazily() {
    let artifact = artifact_from_source(
        "let fail(): int = { 1 / 0 }\n\
         let f(o?:int): int = { o ?? fail() }\n\
         let root() = { f(1) }",
    );
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(
        module.contains("nxCoalesce(o, () => fail())"),
        "the fallback is a thunk: {module}"
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"const out = [m.root()];
try {
  out.push(m.f([]));
} catch (error) {
  out.push(error.message ?? String(error));
}
console.log(JSON.stringify(out));"#,
    );
    assert_json_values_eq(&output, r#"[1, "Division by zero"]"#);
    assert_json_values_eq(
        &execute_generated_javascript_artifact_root(&artifact, ""),
        &interpreter_json_artifact_root(&artifact),
    );
}

/// `==` compares by value in every engine: two empty optionals are equal, records compare by
/// their fields, and a sequence by its items.
#[test]
fn generated_javascript_equality_is_structural_like_the_interpreter() {
    let prelude = "type P = { name:string nick?:string }\n\
         let same(a:P, b:P) = { a.nick == b.nick }\n";
    let cases = [
        (
            "let root() = { same(<P name=\"x\" />, <P name=\"y\" />) }",
            "true",
        ),
        (
            "let root() = { same(<P name=\"x\" nick=\"n\" />, <P name=\"y\" />) }",
            "false",
        ),
        (
            "let root() = { <P name=\"x\" nick=\"n\" /> == <P name=\"x\" nick=\"n\" /> }",
            "true",
        ),
        (
            "let root() = { <P name=\"x\" /> != <P name=\"y\" /> }",
            "true",
        ),
        (
            "let xs:int+ = { 1 2 }\nlet ys:int+ = { 1 2 }\nlet root() = { xs == ys }",
            "true",
        ),
    ];
    for (body, expected) in cases {
        let source = format!("{prelude}{body}");
        let output = execute_generated_javascript_root(&source);
        assert_json_values_eq(&output, expected);
        assert_json_values_eq(&output, &interpreter_json_root(&source));
    }

    // Two exactly-one primitives still compare with `===`.
    let artifact =
        artifact_from_source("let f(a:int, b:int) = { a == b }\nlet root() = { f(1, 1) }");
    let module = generated_file(&artifact, CodegenTarget::JavaScript, "m0_main.js");
    assert!(module.contains("(a === b)"), "{module}");
    assert!(!module.contains("nxValuesEqual"), "{module}");
}

/// The runtime's equality treats every value as a sequence: an item equals a one-element array
/// holding an equal item, and every spelling of the empty value is the one empty.
#[test]
fn emitted_runtime_equality_treats_an_item_as_a_sequence_of_one() {
    for target in [CodegenTarget::JavaScript, CodegenTarget::TypeScript] {
        let output = execute_script_against_emitted_runtime(
            target,
            "import { nxValuesEqual as eq } from './nx-runtime.js';\n\
             console.log(JSON.stringify([eq(1, [1]), eq([1], 1), eq(1, [1, 1]), eq([], null), \
             eq(undefined, []), eq([], [[]]), eq({ $type: 'P', a: 1 }, { $type: 'P', a: [1] }), \
             eq({ $type: 'P' }, { $type: 'P', a: 1 })]));",
        );
        assert_json_values_eq(
            &output,
            "[true, true, false, true, true, false, true, false]",
        );
    }
}

/// An occurrence decision reads the alias-resolved type, so `type Ints = int+` lifts exactly as
/// `int+` does, and a call argument and a `??` fallback are lifted to the sequence their site
/// binds.
#[test]
fn generated_javascript_lifts_through_aliases_arguments_and_fallbacks() {
    let prelude = "type Ints = int+\n\
         type R = { xs:Ints }\n\
         type O = { xs?:Ints }\n\
         type Item = { n:int }\n\
         type Items = Item+\n\
         type Box = { content items:Items }\n\
         external component <Ext xs?:Ints />\n\
         let f(xs?:int+) = { xs ?? 5 }\n\
         let p(xs:int+) = { xs }\n\
         type S = { xs:int+ }\n\
         let k(xs?:int+) = <S xs={xs ?? 5} />\n";
    for body in [
        "let root() = <R xs={3} />",
        "let root() = <O xs={3} />",
        "let root() = <Box><Item n=1 /></Box>",
        "let root() = <Ext xs={3} />",
        "let root() = { f(3) }",
        "let root() = { f({}) }",
        "let root() = { p(5) }",
        "let root() = { k({}) }",
    ] {
        let source = format!("{prelude}{body}");
        assert_json_values_eq(
            &execute_generated_javascript_root(&source),
            &interpreter_json_root(&source),
        );
    }
}

/// A `for` over an optional yields its body's value unchanged, so an item stays an item in
/// generated JavaScript exactly as in the interpreter, and an empty optional yields `{}`.
#[test]
fn generated_javascript_for_over_an_optional_yields_its_item() {
    let prelude = "type Person = { name:string }\n\
         let name(p?:Person) = { for x in p { x.name } }\n\
         let names(ps:Person+) = { for x in ps { x.name } }\n";
    for body in [
        "let root() = { name(<Person name=\"Ada\" />) }",
        "let root() = { name({}) }",
        "let root() = { name(<Person name=\"Ada\" />) == \"Ada\" }",
        "let root() = { names(<Person name=\"Ada\" />) }",
    ] {
        let source = format!("{prelude}{body}");
        assert_json_values_eq(
            &execute_generated_javascript_root(&source),
            &interpreter_json_root(&source),
        );
    }
}

/// A record reached through a component's props is validated at each field's read type, so an
/// optional `+` field admits `[]` and `null`, and either is dropped as an empty optional field is.
#[test]
fn generated_record_schemas_validate_optional_fields_at_the_read_type() {
    let artifact = artifact_from_source(
        "type Book = { title:string tags?:string+ sub?:string }\n\
         external component <Ext b:Book />\n\
         component <Show book:Book /> = { <Ext b={book} /> }\n\
         let root() = { <Show book={<Book title=\"x\" />} /> }",
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"const out = [];
for (const book of [
  { $type: "Book", title: "x", tags: [] },
  { $type: "Book", title: "x", sub: null },
  { $type: "Book", title: "x", tags: "a" },
]) {
  out.push(m.ShowSchema.initializeJson({ book }).rendered);
}
console.log(JSON.stringify(out));"#,
    );
    assert_json_values_eq(
        &output,
        r#"[
          { "$type": "Ext", "b": { "$type": "Book", "title": "x" } },
          { "$type": "Ext", "b": { "$type": "Book", "title": "x" } },
          { "$type": "Ext", "b": { "$type": "Book", "title": "x", "tags": ["a"] } }
        ]"#,
    );
}

/// An empty optional state field is an omitted key in generated code, as it is in the IR runtime,
/// and reads as the empty value in the render.
#[test]
fn generated_initial_state_omits_an_empty_optional_field() {
    let artifact = artifact_from_source(
        "external component <Ext label?:string />\n\
         component <Card title?:string /> = { state { sel?:string tags?:string+ n:int = 1 } \
         <Ext label={sel ?? title} /> }\n\
         let root() = { <Card /> }",
    );
    let output = execute_generated_javascript_artifact_script(
        &artifact,
        r#"const a = m.CardSchema.initializeJson({ title: "t" });
const b = m.CardSchema.evaluateJson({}, { n: 2, sel: "s" });
console.log(JSON.stringify([a.state, a.rendered, b]));"#,
    );
    assert_json_values_eq(
        &output,
        r#"[{ "n": 1 }, { "$type": "Ext", "label": "t" }, { "$type": "Ext", "label": "s" }]"#,
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
}

/// Loops, `??` and `?.` type-check under `--strict`: the runtime helpers carry the item and member
/// types through rather than widening them to `NxValue`.
#[test]
fn generated_typescript_type_checks_loops_and_occurrence_operators() {
    let artifact = artifact_from_source(
        "let inc(xs:int+): int+ = { for x in xs { x + 1 } }\n\
         type P = { nick?:string }\n\
         let n(p:P): string = { p.nick ?? \"none\" }\n\
         let s(p?:P): string? = { p?.nick }\n\
         let m(p?:P): string = { p?.nick ?? \"d\" }\n\
         let e(a:P, b:P): boolean = { a.nick == b.nick }\n\
         let root() = { inc(1) }",
    );
    assert_generated_typescript_artifact_type_checks(&artifact);

    // `?.` into a `*` member, a `for` over an optional member, chained fallbacks, an optional
    // member read into an optional field, and a conditional record at a `?` and a `*` site.
    let artifact = artifact_from_source(
        "type Person = { name:string nick?:string tags?:string+ }\n\
         type Book = { title:string author?:Person subtitle?:string }\n\
         type Item = { n:int }\n\
         type Opt = { content one?:Item }\n\
         type Star = { content items?:Item+ }\n\
         let bk() = <Book title=\"T\" author={<Person name=\"Ada\" />} />\n\
         let tags(): string* = { bk().author?.tags }\n\
         let authorName(): string? = { for t in bk().author { t.name } }\n\
         let subtitle(): string = { bk().subtitle ?? bk().subtitle ?? \"x\" }\n\
         let copied(): Person = <Person name=\"A\" nick={bk().subtitle} />\n\
         let one(c:boolean): Opt = <Opt>{if c { <Item n=1 /> }}</Opt>\n\
         let star(c:boolean): Star = <Star>{if c { <Item n=1 /> }}</Star>\n\
         let root() = { tags() }",
    );
    assert_generated_typescript_artifact_type_checks(&artifact);
    assert_json_values_eq(
        &execute_generated_javascript_artifact_script(
            &artifact,
            "console.log(JSON.stringify([m.tags(), m.authorName(), m.subtitle(), m.copied(), \
             m.one(true), m.one(false), m.star(true), m.star(false)]));",
        ),
        r#"[[], "Ada", "x", { "$type": "Person", "name": "A" },
            { "$type": "Opt", "one": { "$type": "Item", "n": 1 } }, { "$type": "Opt" },
            { "$type": "Star", "items": [{ "$type": "Item", "n": 1 }] }, { "$type": "Star" }]"#,
    );
}

/// An element call may leave out an optional parameter, which binds the empty value.
#[test]
fn generated_javascript_element_call_omits_an_optional_parameter() {
    let prelude = "let <Row Item:string Note?:string />: string = { Item + (Note ?? \"-none\") }\n";
    for body in [
        "let root() = { <Row Item=\"x\" /> }",
        "let root() = { <Row Item=\"x\" Note=\"y\" /> }",
    ] {
        let source = format!("{prelude}{body}");
        assert_json_values_eq(
            &execute_generated_javascript_root(&source),
            &interpreter_json_root(&source),
        );
    }
}

/// Executable source codegen does not support match expressions, so a `{}` pattern is refused with
/// a diagnostic rather than emitted; the IR runtime evaluates it.
#[test]
fn generated_javascript_refuses_a_match_on_the_empty_pattern() {
    let error = emit_program(
        &artifact_from_source(
            "type Person = { name:string }\n\
             type Book = { author?:Person }\n\
             let root() = { if <Book />.author is { {} => \"anonymous\" else => \"named\" } }",
        ),
        &CodegenOptions::javascript(),
    )
    .expect_err("a match is refused");
    assert!(
        error.diagnostics.iter().any(|diagnostic| diagnostic
            .message()
            .contains("match expressions are not supported by executable source codegen")),
        "{:?}",
        error.diagnostics
    );
}
