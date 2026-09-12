use crate::model::{CodegenProperty, CodegenRecordField};
use crate::runtime::runtime_helper_source;
use crate::{
    build_codegen_program, emit_codegen_program, emit_js_program_module, emit_nx_ir, emit_program,
    javascript_runtime_helper_source, CodegenDeclaration, CodegenDeclarationKind,
    CodegenEntrypoint, CodegenExpression, CodegenExpressionKind, CodegenModule,
    CodegenModuleProvenance, CodegenOptions, CodegenProgram, CodegenReference, CodegenTarget,
    CodegenTypeRef, JsProgramModuleOptions, DEFAULT_JS_PROGRAM_MODULE_NAME,
    DEFAULT_JS_PROGRAM_MODULE_RUNTIME_IMPORT_SPECIFIER, NX_IR_FORMAT_ID, NX_IR_RUNTIME_ABI,
    NX_IR_SCHEMA_VERSION, NX_JS_RUNTIME_ABI,
};
use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact, eval_program_artifact,
    validate_workspace, EvalResult, LibraryRegistry, NxWorkspace, NxWorkspaceModule,
    ProgramArtifact, ProgramBuildContext,
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

fn generated_file<'a>(artifact: &'a ProgramArtifact, target: CodegenTarget, name: &str) -> String {
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
    let output = emit_program(&artifact, &CodegenOptions::javascript()).expect("js output");
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
    match eval_program_artifact(&artifact) {
        EvalResult::Ok(value) => value.to_json_string().expect("interpreter json"),
        EvalResult::Err(diagnostics) => panic!("interpreter diagnostics: {:?}", diagnostics),
    }
}

fn assert_json_values_eq(actual: &str, expected: &str) {
    let actual: Value = serde_json::from_str(actual).expect("actual generated json");
    let expected: Value = serde_json::from_str(expected).expect("expected interpreter json");
    assert_eq!(actual, expected);
}

fn ir_declaration<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["modules"]
        .as_array()
        .expect("modules")
        .iter()
        .flat_map(|module| module["declarations"].as_array().expect("declarations"))
        .find(|declaration| declaration["reference"]["name"] == name)
        .unwrap_or_else(|| panic!("IR declaration '{name}'"))
}

fn ir_record_field_type<'a>(declaration: &'a Value, field_name: &str) -> &'a Value {
    declaration["kind"]["fields"]
        .as_array()
        .expect("record fields")
        .iter()
        .find(|field| field["name"] == field_name)
        .map(|field| &field["ty"])
        .unwrap_or_else(|| panic!("IR record field '{field_name}'"))
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

/// Generated IR must contain no reference that failed to resolve to a declaration.
///
/// `Fit` is not imported by `app.nx`, so there is no visible name to resolve `cover` by. The
/// reference carries the union's declaring origin instead, and the emitted slot names the
/// declaration rather than falling back to `unresolved:`.
#[test]
fn nx_ir_carries_no_unresolved_reference_for_a_case_of_an_unimported_union() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app.nx",
                "import { Img } from \"./widgets.nx\"\nlet root() = { <Img fit=cover /> }",
            ),
            (
                "widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit: Fit = {Fit.fill} /> = <div fit={fit} />",
            ),
        ],
        "app.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");

    assert!(
        !generated.json.contains("unresolved:"),
        "generated IR carries an unresolved reference: {}",
        generated.json
    );
    assert!(
        generated.json.contains("\"caseName\": \"cover\""),
        "expected the case to reach the IR: {}",
        generated.json
    );
}

/// A default naming a field materialized after it cannot be emitted.
///
/// The slot such a name used to emit — `unresolved:b` — is one no runtime can bind, so the program
/// generated cleanly and failed the moment it ran. Analysis reports the name now, and emission
/// carries its diagnostics, so this is what an author sees; the `codegen-unresolved-name` check in
/// the builder stands behind it for whatever analysis might still miss.
#[test]
fn nx_ir_refuses_a_default_naming_a_field_declared_after_it() {
    let artifact = artifact_from_source(
        "abstract component <Node />\n\
         external component <Leaf extends Node />\n\
         component <A extends Node a:int = {b} b:int = 1 /> = { <Leaf /> }\n\
         let root() = { <A /> }",
    );

    let error = emit_nx_ir(&artifact).expect_err("a default with nothing to read should not emit");

    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("'b'")),
        "expected the unreadable name to be reported, got: {:?}",
        error.diagnostics
    );
}

/// A component body is a value position like any other, so a bare case there reaches the IR.
///
/// Component bodies were skipped by type checking's item loop, so the contextual name inside one
/// was never resolved and code generation refused it outright with `unresolved contextual name
/// cannot be emitted`. The qualified form is the control: both spellings must emit the same case.
#[test]
fn nx_ir_emits_a_bare_case_written_in_a_component_body() {
    let preamble = "type Fit = fill | contain | cover\n                    external component <Img fit:Fit? />\n                    abstract external component <Node />\n";
    let bare = emit_nx_ir(&artifact_from_source(&format!(
        "{preamble}component <A extends Node /> = {{ <Img fit=cover /> }}\nlet root() = {{ <A /> }}"
    )))
    .expect("nx ir for the bare form");
    let qualified = emit_nx_ir(&artifact_from_source(&format!(
        "{preamble}component <A extends Node /> = {{ <Img fit={{Fit.cover}} /> }}\nlet root() = {{ <A /> }}"
    )))
    .expect("nx ir for the qualified form");

    assert!(
        bare.json.contains("\"caseName\": \"cover\""),
        "expected the case to reach the IR: {}",
        bare.json
    );
    assert_eq!(
        bare.json.matches("\"caseName\": \"cover\"").count(),
        qualified.json.matches("\"caseName\": \"cover\"").count(),
        "the bare form emitted a different number of case references than the qualified form"
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

/// The same, for a case reached through the name a selective import alias bound.
///
/// `Fit as ui.Fit` binds the single visible name `ui.Fit`; there is no `ui` to take a member of.
/// Reading only a single-segment base left `ui.Fit.cover` lowered as a member access on a name that
/// reaches nothing — an `unresolved:` slot and a null case reference in output the compiler
/// accepted without a diagnostic.
#[test]
fn nx_ir_carries_no_unresolved_reference_for_a_case_of_an_aliased_union() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app.nx",
                "import { Img, Fit as ui.Fit } from \"./widgets.nx\"\nlet root() = { <Img fit={ui.Fit.cover} /> }",
            ),
            (
                "widgets.nx",
                "export type Fit = fill | contain | cover\nexport let <Img fit: Fit = {Fit.fill} /> = <div fit={fit} />",
            ),
        ],
        "app.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");

    assert!(
        !generated.json.contains("unresolved:"),
        "generated IR carries an unresolved reference: {}",
        generated.json
    );
    assert!(
        !generated.json.contains("\"reference\": null"),
        "generated IR carries a reference that resolved to nothing: {}",
        generated.json
    );
    assert!(
        generated.json.contains("\"caseName\": \"cover\""),
        "expected the case to reach the IR: {}",
        generated.json
    );
}

#[test]
fn nx_ir_emits_metadata_entrypoints_and_source_provenance() {
    let artifact = artifact_from_source("let root() = { 1 + 2 }");
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    assert_eq!(document["format"], NX_IR_FORMAT_ID);
    assert_eq!(document["schemaVersion"], NX_IR_SCHEMA_VERSION);
    assert_eq!(document["runtimeAbi"], NX_IR_RUNTIME_ABI);
    assert_eq!(
        document["programFingerprint"],
        artifact.fingerprint.to_string()
    );
    assert_eq!(document["functionEntrypoints"][0]["name"], "root");
    assert_eq!(
        document["functionEntrypoints"][0]["reference"]["module"],
        "m0"
    );
    assert_eq!(document["sources"][0]["identity"], "main.nx");
    assert_eq!(document["sources"][0]["source"], "let root() = { 1 + 2 }");

    assert_eq!(generated.metadata.program_fingerprint, artifact.fingerprint);
    assert_eq!(generated.metadata.schema_version, NX_IR_SCHEMA_VERSION);
    assert_eq!(generated.metadata.runtime_abi, NX_IR_RUNTIME_ABI);
    assert_eq!(generated.metadata.function_entrypoints[0].name, "root");
}

#[test]
fn nx_ir_output_is_deterministic() {
    let source = r#"
type Theme = light | dark
type User = { name:string score:int = 42 }
let root() = { <User name="Ada" /> }
"#;
    let first = emit_nx_ir(&artifact_from_source(source)).expect("first nx ir");
    let second = emit_nx_ir(&artifact_from_source(source)).expect("second nx ir");

    assert_eq!(first.json, second.json);
}

/// The two spellings of a whole number at a float property, as one program each.
fn float_property_program(literal: &str) -> String {
    format!(
        "external component <B v:float64 />\nlet root() = {{ <B v={} /> }}\n",
        literal
    )
}

/// Strips what two programs cannot share when their source text differs.
///
/// <para>`24` and `24.0` are two different files, two characters apart, so every byte offset after
/// the literal moves, the retained source differs, and the fingerprint over them differs with it.
/// NX IR carries source provenance deliberately — diagnostics and source maps need it — so those
/// differences are correct rather than noise to be designed away. What must match is everything
/// else: the declarations, the types, and the literal itself.</para>
fn without_source_provenance(json: &str) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("span");
                map.remove("programFingerprint");
                map.remove("source");
                map.remove("sources");
                for nested in map.values_mut() {
                    strip(nested);
                }
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }

    let mut value: serde_json::Value = serde_json::from_str(json).expect("nx ir json");
    strip(&mut value);
    value
}

#[test]
fn int_literal_at_a_float_site_emits_the_same_ir_as_a_real_literal() {
    // The guarantee the conversion exists to provide: nothing below type checking can tell which
    // spelling the author used, so a consumer added later cannot depend on the difference.
    let written_as_int = emit_nx_ir(&artifact_from_source(&float_property_program("24")))
        .expect("nx ir for the integer spelling");
    let written_as_float = emit_nx_ir(&artifact_from_source(&float_property_program("24.0")))
        .expect("nx ir for the real spelling");

    assert_eq!(
        without_source_provenance(&written_as_int.json),
        without_source_provenance(&written_as_float.json)
    );
}

#[test]
fn int_literal_at_a_float_site_is_not_emitted_as_an_integer_literal() {
    let generated = emit_nx_ir(&artifact_from_source(&float_property_program("24")))
        .expect("nx ir for the integer spelling");

    // Asserting only equality with the real spelling would pass if both emitted an integer.
    assert!(
        !generated.json.contains(r#""kind":"int""#),
        "the converted literal should not reach the IR as an int: {}",
        generated.json
    );
}

#[test]
fn int_literal_at_a_float32_site_emits_the_same_ir_as_a_real_literal() {
    let program = |literal: &str| {
        format!(
            "external component <B v:float32 />\nlet root() = {{ <B v={} /> }}\n",
            literal
        )
    };
    let written_as_int =
        emit_nx_ir(&artifact_from_source(&program("24"))).expect("nx ir for the integer spelling");
    let written_as_float =
        emit_nx_ir(&artifact_from_source(&program("24.0"))).expect("nx ir for the real spelling");

    assert_eq!(
        without_source_provenance(&written_as_int.json),
        without_source_provenance(&written_as_float.json)
    );
}

#[test]
fn int_literal_defaults_and_lists_at_float_sites_match_their_real_spellings() {
    let program = |x: &str, items: &str| {
        format!(
            "type Opts = {{ x:float64 = {} }}\nexternal component <B v:float64[] />\nlet root() = {{ <B v={{{}}} /> }}\n",
            x, items
        )
    };
    let written_as_int = emit_nx_ir(&artifact_from_source(&program("0", "1 2 3")))
        .expect("nx ir for the integer spelling");
    let written_as_float = emit_nx_ir(&artifact_from_source(&program("0.0", "1.0 2.0 3.0")))
        .expect("nx ir for the real spelling");

    assert_eq!(
        without_source_provenance(&written_as_int.json),
        without_source_provenance(&written_as_float.json)
    );
}

#[test]
fn nx_ir_preserves_module_qualified_references() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { answer } from "../shared/value.nx"
let root(): int = { answer() }"#,
            ),
            ("shared/value.nx", r#"export let answer(): int = { 42 }"#),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let modules = document["modules"].as_array().expect("modules");
    assert_eq!(modules.len(), 2);
    let root_body = &modules[0]["declarations"][0]["kind"]["body"];
    let callee_reference = &root_body["op"]["callee"]["op"]["reference"];
    assert_eq!(callee_reference["kind"], "function");
    assert_eq!(callee_reference["name"], "answer");
    assert_ne!(callee_reference["module"], "m0");
}

/// Fields are flattened into the IR, so the base chain is what tells a runtime that a value
/// stamped `User` is acceptable where a `Base` was asked for. Names cannot carry that across
/// modules, so the chain is emitted as declaration references.
#[test]
fn nx_ir_records_and_unions_carry_their_base_chain() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { Base } from "../shared/model.nx"
abstract type Named extends Base = { label:string }
type User extends Named = { role:string }
type Shape extends Base =
  | circle { r:int }
  | square { s:int }
type Loose = { free:string }
let root(user: User): User = { user }"#,
            ),
            (
                "shared/model.nx",
                r#"export abstract type Base = { name:string }"#,
            ),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let base = ir_declaration(&document, "Base");
    let user = ir_declaration(&document, "User");
    let shape = ir_declaration(&document, "Shape");

    // Nearest first, and reaching past the immediate base to the one it extends.
    let user_bases = user["kind"]["bases"].as_array().expect("user bases");
    assert_eq!(user_bases.len(), 2);
    assert_eq!(user_bases[0]["name"], "Named");
    assert_eq!(user_bases[1]["name"], "Base");
    assert_eq!(user_bases[1]["declaration"], base["id"]);
    assert_eq!(user_bases[1]["kind"], "record");
    assert_ne!(user_bases[1]["module"], user["reference"]["module"]);

    // A union's cases all inherit the union's base, so the chain sits on the union itself.
    let shape_bases = shape["kind"]["bases"].as_array().expect("shape bases");
    assert_eq!(shape_bases.len(), 1);
    assert_eq!(shape_bases[0]["declaration"], base["id"]);

    assert_eq!(
        ir_declaration(&document, "Loose")["kind"]["bases"]
            .as_array()
            .expect("loose bases")
            .len(),
        0
    );
    assert_eq!(
        base["kind"]["bases"].as_array().expect("base bases").len(),
        0
    );
}

#[test]
fn nx_ir_records_carry_whether_they_are_abstract() {
    let artifact = artifact_from_source(
        r#"abstract type Base = { name:string }
type User extends Base = { role:string }
let root(user: User): User = { user }"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    // A base-typed boundary accepts a value of a record that extends the base, never one of the
    // base itself, and this is the only thing in the IR that says which is which.
    assert_eq!(
        ir_declaration(&document, "Base")["kind"]["isAbstract"],
        true
    );
    assert_eq!(
        ir_declaration(&document, "User")["kind"]["isAbstract"],
        false
    );
}

#[test]
fn nx_ir_nominal_type_refs_are_module_qualified() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { User } from "../shared/model.nx"
let root(user: User): User = { user }"#,
            ),
            ("shared/model.nx", r#"export type User = { name:string }"#),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let modules = document["modules"].as_array().expect("modules");
    let root = modules
        .iter()
        .flat_map(|module| module["declarations"].as_array().expect("declarations"))
        .find(|declaration| declaration["reference"]["name"] == "root")
        .expect("root declaration");
    let param_type = &root["kind"]["params"][0]["ty"];

    assert_eq!(param_type["kind"], "nominal");
    assert_eq!(param_type["display"], "User");
    assert_eq!(param_type["reference"]["name"], "User");
    assert_eq!(param_type["reference"]["kind"], "record");
    assert_ne!(
        param_type["reference"]["module"],
        root["reference"]["module"]
    );
}

/// An expression written inside an intrinsic element carries the same semantic type in the IR as
/// the same expression written outside one.
///
/// <para>The IR's `ty` is not decoration. `@nx-lang/ir-runtime` reads it to pick integer division
/// from float division, so an expression that reached codegen with no type divided as a float:
/// `&lt;div&gt;{7 / 2}&lt;/div&gt;` evaluated to `3.5` while `{ 7 / 2 }` evaluated to `3`. Nothing
/// in codegen caused that — inference never visited an element whose tag resolves to nothing, so
/// the type environment codegen reads had no entry to hand over.</para>
#[test]
fn nx_ir_types_an_expression_written_inside_an_intrinsic_element() {
    let inside = artifact_from_source("let root() = <div>{7 / 2}</div>");
    let generated = emit_nx_ir(&inside).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let divisions = find_typed_divisions(ir_declaration(&document, "root"));
    assert_eq!(
        divisions,
        vec!["int".to_string()],
        "the division inside the element should carry the type it has"
    );
}

/// Every `div` expression's semantic type display, gathered from anywhere under a declaration.
fn find_typed_divisions(node: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_typed_divisions(node, &mut found);
    found
}

fn collect_typed_divisions(node: &Value, found: &mut Vec<String>) {
    match node {
        Value::Object(fields) => {
            // An expression node carries `ty` beside the `op` that spells the operator, so the
            // type belongs to the node the operator is nested in rather than to the operator.
            let is_division = fields
                .get("op")
                .and_then(|op| op.get("operator"))
                .and_then(Value::as_str)
                == Some("div");
            if is_division {
                found.push(
                    fields
                        .get("ty")
                        .and_then(|ty| ty.get("display"))
                        .and_then(Value::as_str)
                        .unwrap_or("<none>")
                        .to_string(),
                );
            }
            for value in fields.values() {
                collect_typed_divisions(value, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_typed_divisions(item, found);
            }
        }
        _ => {}
    }
}

#[test]
fn nx_ir_declared_element_type_is_not_shadowed_by_builtin_element_supertype() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                r#"import { Element } from "../shared/model.nx"
let root(value: Element): Element = { value }"#,
            ),
            ("shared/model.nx", r#"export type Element = { id:string }"#),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let root = ir_declaration(&document, "root");
    let param_type = &root["kind"]["params"][0]["ty"];

    assert_eq!(param_type["kind"], "nominal");
    assert_eq!(param_type["display"], "Element");
    assert_eq!(param_type["reference"]["name"], "Element");
    assert_eq!(param_type["reference"]["kind"], "record");
    assert_ne!(
        param_type["reference"]["module"],
        root["reference"]["module"]
    );
}

#[test]
fn nx_ir_preserves_directory_loaded_cross_library_type_refs() {
    let temp = TempDir::new().expect("temp dir");
    let flow_step_dir = temp.path().join("flow-step");
    let ui_dir = temp.path().join("ui");
    let question_flow_dir = temp.path().join("question-flow");
    let chat_link_dir = temp.path().join("chat-link");
    fs::create_dir_all(&flow_step_dir).expect("flow-step dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");
    fs::create_dir_all(&question_flow_dir).expect("question-flow dir");
    fs::create_dir_all(&chat_link_dir).expect("chat-link dir");

    fs::write(
        flow_step_dir.join("FlowStep.nx"),
        r#"export type FlowStep = { id:string }"#,
    )
    .expect("flow step source");
    fs::write(
        ui_dir.join("TextInput.nx"),
        r#"export external component <TextInput value:string />"#,
    )
    .expect("ui source");
    fs::write(
        question_flow_dir.join("QuestionFlow.nx"),
        r#"import { FlowStep } from "../flow-step"
import { TextInput } from "../ui"
export type QuestionFlow = { firstStep:FlowStep input:TextInput }"#,
    )
    .expect("question flow source");
    fs::write(
        chat_link_dir.join("ChatLinkConfig.nx"),
        r#"import { QuestionFlow } from "../question-flow"
export type ChatLinkConfig = { questionFlow:QuestionFlow }"#,
    )
    .expect("chat link source");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&question_flow_dir)
        .expect("question-flow library");
    registry
        .load_library_from_directory(&chat_link_dir)
        .expect("chat-link library");
    let build_context = registry.build_context();
    let workspace = NxWorkspace::new(vec![NxWorkspaceModule::from_source(
        "app/main.nx",
        r#"import { ChatLinkConfig } from "../chat-link"
let root() = { "ready" }"#,
    )
    .expect("workspace module")])
    .expect("workspace");
    let diagnostics = validate_workspace(&workspace, &build_context);
    assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);

    let artifact = build_workspace_program_artifact(&workspace, "app/main.nx", &build_context)
        .expect("workspace artifact");
    assert_eq!(interpreter_json_artifact_root(&artifact), r#""ready""#);

    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let config = ir_declaration(&document, "ChatLinkConfig");
    let question_flow = ir_declaration(&document, "QuestionFlow");
    let question_flow_ty = ir_record_field_type(config, "questionFlow");
    let flow_step_ty = ir_record_field_type(question_flow, "firstStep");
    let input_ty = ir_record_field_type(question_flow, "input");

    assert_eq!(question_flow_ty["kind"], "nominal");
    assert_eq!(question_flow_ty["display"], "QuestionFlow");
    assert_eq!(question_flow_ty["reference"]["name"], "QuestionFlow");
    assert_eq!(question_flow_ty["reference"]["kind"], "record");
    assert_ne!(
        question_flow_ty["reference"]["module"],
        config["reference"]["module"]
    );

    assert_eq!(flow_step_ty["kind"], "nominal");
    assert_eq!(flow_step_ty["display"], "FlowStep");
    assert_eq!(flow_step_ty["reference"]["name"], "FlowStep");
    assert_eq!(flow_step_ty["reference"]["kind"], "record");
    assert_ne!(
        flow_step_ty["reference"]["module"],
        question_flow["reference"]["module"]
    );

    assert_eq!(input_ty["kind"], "nominal");
    assert_eq!(input_ty["display"], "TextInput");
    assert_eq!(input_ty["reference"]["name"], "TextInput");
    assert_eq!(input_ty["reference"]["kind"], "component");
    assert_ne!(
        input_ty["reference"]["module"],
        question_flow["reference"]["module"]
    );
}

#[test]
fn nx_ir_preserves_nullable_union_and_content_boundary_metadata() {
    let temp = TempDir::new().expect("temp dir");
    let flow_dir = temp.path().join("flow");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&flow_dir).expect("flow dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");

    fs::write(
        flow_dir.join("Flow.nx"),
        r#"export type FlowCompletion = continue | end { message:string }
export type QuestionFlow = {
  completion:FlowCompletion?
  content steps:Element
}"#,
    )
    .expect("flow source");
    fs::write(
        ui_dir.join("Panel.nx"),
        r#"export external component <Panel content body:Element />"#,
    )
    .expect("ui source");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&flow_dir)
        .expect("flow library");
    registry
        .load_library_from_directory(&ui_dir)
        .expect("ui library");
    let build_context = registry.build_context();
    let workspace = NxWorkspace::new(vec![NxWorkspaceModule::from_source(
        "app/main.nx",
        r#"import { QuestionFlow } from "../flow"
import { Panel } from "../ui"
let omitted(): QuestionFlow = { <QuestionFlow><Panel><span /></Panel></QuestionFlow> }
let explicit(): QuestionFlow = { <QuestionFlow completion={null}><Panel><span /></Panel></QuestionFlow> }
let root(): QuestionFlow[] = { omitted() explicit() }"#,
    )
    .expect("workspace module")])
    .expect("workspace");

    let diagnostics = validate_workspace(&workspace, &build_context);
    assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
    let artifact = build_workspace_program_artifact(&workspace, "app/main.nx", &build_context)
        .expect("workspace artifact");
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let question_flow = ir_declaration(&document, "QuestionFlow");
    let completion_ty = ir_record_field_type(question_flow, "completion");
    let steps_field = question_flow["kind"]["fields"]
        .as_array()
        .expect("QuestionFlow fields")
        .iter()
        .find(|field| field["name"] == "steps")
        .expect("steps field");
    let omitted_body = &ir_declaration(&document, "omitted")["kind"]["body"]["op"];
    let explicit_body = &ir_declaration(&document, "explicit")["kind"]["body"]["op"];
    let app_module = &ir_declaration(&document, "omitted")["reference"]["module"];
    let panel_descriptor = &omitted_body["content"][0]["op"];

    assert_eq!(completion_ty["kind"], "nullable");
    assert_eq!(completion_ty["inner"]["kind"], "nominal");
    assert_eq!(completion_ty["inner"]["reference"]["kind"], "union");
    assert_ne!(&completion_ty["inner"]["reference"]["module"], app_module);
    assert_eq!(steps_field["isContent"], true);
    assert_eq!(steps_field["isRequired"], true);

    assert_eq!(omitted_body["tag"], "record");
    assert_eq!(omitted_body["contentField"], "steps");
    assert_eq!(
        omitted_body["content"][0]["op"]["tag"],
        "componentDescriptor"
    );
    assert!(omitted_body["properties"]
        .as_array()
        .expect("omitted properties")
        .iter()
        .all(|property| property["name"] != "completion"));

    assert_eq!(explicit_body["tag"], "record");
    assert_eq!(explicit_body["contentField"], "steps");
    let explicit_completion = explicit_body["properties"]
        .as_array()
        .expect("explicit properties")
        .iter()
        .find(|property| property["name"] == "completion")
        .expect("completion property");
    assert_eq!(explicit_completion["value"]["op"]["value"]["kind"], "null");

    assert_eq!(panel_descriptor["tag"], "componentDescriptor");
    assert_eq!(panel_descriptor["contentField"], "body");
    assert_eq!(
        panel_descriptor["content"][0]["op"]["tag"],
        "intrinsicElement"
    );
    assert_eq!(panel_descriptor["component"]["kind"], "component");
    assert_ne!(&panel_descriptor["component"]["module"], app_module);
}

#[test]
fn nx_ir_missing_semantic_data_fails_without_partial_document() {
    let mut artifact = artifact_from_source("let root() = { 42 }");
    artifact.root_modules[0].lowered_module = None;

    let error = emit_nx_ir(&artifact).expect_err("missing semantic data should fail IR emission");
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == Some("codegen-missing-semantic-data")),
        "diagnostics: {:?}",
        error.diagnostics
    );
}

#[test]
fn nx_ir_component_inherited_field_spans_use_owner_source() {
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
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let modules = document["modules"].as_array().expect("modules");
    let short_text_question = modules
        .iter()
        .flat_map(|module| module["declarations"].as_array().expect("declarations"))
        .find(|declaration| declaration["reference"]["name"] == "ShortTextQuestion")
        .expect("ShortTextQuestion declaration");
    let props = short_text_question["kind"]["props"]
        .as_array()
        .expect("props");
    let label = props
        .iter()
        .find(|field| field["name"] == "label")
        .expect("label prop");
    let placeholder = props
        .iter()
        .find(|field| field["name"] == "placeholder")
        .expect("placeholder prop");

    assert_eq!(label["span"]["source"], "shared/ui.nx");
    assert_eq!(placeholder["span"]["source"], "app/main.nx");
}

#[test]
fn nx_ir_encodes_eager_expressions_and_canonical_value_metadata() {
    let loop_ir = emit_nx_ir(&artifact_from_source(
        "let <Labels items:int[] /> = {for item, index in items { item + index }}",
    ))
    .expect("loop nx ir");
    let loop_document: Value = serde_json::from_str(&loop_ir.json).expect("loop nx ir json");
    assert_eq!(
        loop_document["modules"][0]["declarations"][0]["kind"]["body"]["op"]["tag"],
        "for"
    );

    let match_ir = emit_nx_ir(&artifact_from_source(
        r#"let root(x:int): string = {
    if x is {
        0 => "zero"
        1, 2 => "small"
        else => "many"
    }
}"#,
    ))
    .expect("match nx ir");
    let match_document: Value = serde_json::from_str(&match_ir.json).expect("match nx ir json");
    let root_body = &match_document["modules"][0]["declarations"][0]["kind"]["body"];
    assert_eq!(root_body["op"]["tag"], "ifIs");
    assert_eq!(
        root_body["op"]["arms"][0]["patterns"][0]["op"]["tag"],
        "literal"
    );

    let union_ir = emit_nx_ir(&artifact_from_source(
        r#"type LoadState = idle | failed { message:string }
let root(): LoadState = { <LoadState.failed message={"Offline"} /> }"#,
    ))
    .expect("union nx ir");
    let union_document: Value = serde_json::from_str(&union_ir.json).expect("union nx ir json");
    let union_body = &union_document["modules"][0]["declarations"][1]["kind"]["body"];
    assert_eq!(union_body["op"]["tag"], "unionCase");

    let big_ir = emit_nx_ir(&artifact_from_source("let root() = { 9007199254740993 }"))
        .expect("big int nx ir");
    let big_document: Value = serde_json::from_str(&big_ir.json).expect("big int nx ir json");
    let big_body = &big_document["modules"][0]["declarations"][0]["kind"]["body"];
    assert_eq!(big_body["op"]["value"]["kind"], "int");
    assert_eq!(big_body["op"]["value"]["value"], "9007199254740993");
    assert!(big_body["op"]["value"]["number"].is_null());
}

#[test]
fn nx_ir_encodes_component_state_metadata() {
    let source = r#"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox /> }
"#;
    let generated = emit_nx_ir(&artifact_from_source(source)).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");
    let declarations = document["modules"][0]["declarations"]
        .as_array()
        .expect("declarations");
    let search_box = declarations
        .iter()
        .find(|declaration| declaration["reference"]["name"] == "SearchBox")
        .expect("SearchBox declaration");

    assert_eq!(search_box["kind"]["tag"], "component");
    assert_eq!(search_box["kind"]["props"][0]["name"], "placeholder");
    assert_eq!(
        search_box["kind"]["props"][0]["default"]["op"]["tag"],
        "literal"
    );
    assert_eq!(search_box["kind"]["state"][0]["name"], "query");
    assert_eq!(
        search_box["kind"]["state"][0]["default"]["op"]["tag"],
        "slot"
    );
    assert_eq!(
        search_box["kind"]["body"]["op"]["tag"],
        "componentDescriptor"
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

/// Strips every `span` object from an NX IR document so two spellings of the same declaration can
/// be compared on content rather than on source offsets.
fn strip_spans(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("span");
            map.remove("sources");
            for entry in map.values_mut() {
                strip_spans(entry);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_spans(item);
            }
        }
        _ => {}
    }
}

/// Generated JavaScript, TypeScript, and NX IR for a program whose only difference is how its
/// closed set of constants is declared. The JS and TS bodies carry the emitted schema.
fn constant_set_outputs(declaration: &str) -> (String, String, String) {
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
    let mut ir: Value = serde_json::from_str(&emit_nx_ir(&artifact).expect("nx ir output").json)
        .expect("nx ir json");
    strip_spans(&mut ir);
    let ir = serde_json::to_string_pretty(&ir["modules"]).expect("ir modules");

    (javascript, typescript, ir)
}

/// The optional leading `|` is purely syntactic: both spellings of the same constant union
/// generate identical output.
///
/// This began as the guard that an `enum` and the equivalent `type` generate the same thing —
/// which is what the unification had to establish. With `enum` removed there is one keyword left,
/// so what remains to guard is D2's two case-list spellings.
#[test]
fn both_case_list_spellings_generate_identical_javascript_typescript_ir_and_schema() {
    let (bare_js, bare_ts, bare_ir) =
        constant_set_outputs("export type Fit = fill | contain | cover");
    let (piped_js, piped_ts, piped_ir) =
        constant_set_outputs("export type Fit = fill | contain | cover");

    assert_eq!(bare_js, piped_js, "generated JavaScript and its schema");
    assert_eq!(bare_ts, piped_ts, "generated TypeScript and its schema");
    assert_eq!(bare_ir, piped_ir, "generated NX IR");
}

// ------------------------------------------------------------------------------------------------
// Derived update records
// ------------------------------------------------------------------------------------------------

fn ir_declaration_names(document: &Value) -> Vec<String> {
    document["modules"]
        .as_array()
        .expect("modules")
        .iter()
        .flat_map(|module| module["declarations"].as_array().expect("declarations"))
        .map(|declaration| {
            declaration["reference"]["name"]
                .as_str()
                .expect("declaration name")
                .to_string()
        })
        .collect()
}

/// Finds the first IR expression op anywhere in `value` whose tag and name match.
fn find_ir_op<'a>(value: &'a Value, tag: &str, name: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            if object.get("tag").and_then(Value::as_str) == Some(tag)
                && object.get("name").and_then(Value::as_str) == Some(name)
            {
                return Some(value);
            }
            object
                .values()
                .find_map(|child| find_ir_op(child, tag, name))
        }
        Value::Array(items) => items.iter().find_map(|child| find_ir_op(child, tag, name)),
        _ => None,
    }
}

#[test]
fn nx_ir_declares_a_referenced_update_record_with_its_own_schema() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string = "anon" email:string? }
let patch() = <User.Update email={null} />
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let update = ir_declaration(&document, "User.Update");
    assert_eq!(update["kind"]["tag"], "record");
    assert_eq!(update["kind"]["updateTarget"]["name"], "User");
    assert_eq!(update["kind"]["updateTarget"]["kind"], "record");

    let fields = update["kind"]["fields"].as_array().expect("update fields");
    let names = fields
        .iter()
        .map(|field| field["name"].as_str().expect("field name"))
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["name", "email"]);
    for field in fields {
        assert_eq!(field["isRequired"], false, "every update field is optional");
        assert!(
            field.get("default").is_none(),
            "no update field has a default"
        );
    }
    assert_eq!(ir_record_field_type(update, "email")["kind"], "nullable");
    assert_eq!(ir_record_field_type(update, "name")["kind"], "primitive");

    assert!(generated
        .metadata
        .required_features
        .iter()
        .any(|feature| feature == crate::NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1));
    assert_eq!(
        document["requiredFeatures"],
        serde_json::json!(generated.metadata.required_features)
    );
}

#[test]
fn nx_ir_omits_update_records_a_program_never_references() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string }
let user() = <User name="Ada" />
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    assert_eq!(ir_declaration_names(&document), vec!["User", "user"]);
    assert_eq!(
        generated.metadata.required_features,
        vec![crate::NX_IR_REQUIRED_FEATURE_EAGER_V1.to_string()]
    );
    let user = ir_declaration(&document, "User");
    assert!(
        user["kind"].get("updateTarget").is_none(),
        "an ordinary record's declaration is unchanged"
    );
    assert_eq!(
        user["reference"]["declaration"], "m0:d0",
        "the update record is declared after every written item, so ids do not shift"
    );
}

#[test]
fn nx_ir_encodes_update_construction_as_record_construction() {
    let artifact = artifact_from_source(
        r#"
component <Counter /> = { state { count:int = 0 label:string = "x" } <Label /> }
let reset() = <Counter.Update count=1 />
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let update = ir_declaration(&document, "Counter.Update");
    assert_eq!(update["kind"]["updateTarget"]["kind"], "component");

    let reset = ir_declaration(&document, "reset");
    let construction =
        find_ir_op(reset, "record", "Counter.Update").expect("record construction op");
    assert_eq!(construction["isUpdate"], true);
    let properties = construction["properties"].as_array().expect("properties");
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0]["name"], "count");

    // Ordinary record construction carries no update flag at all.
    let plain = artifact_from_source(
        r#"
type User = { name:string }
let user() = <User name="Ada" />
"#,
    );
    let plain: Value =
        serde_json::from_str(&emit_nx_ir(&plain).expect("nx ir").json).expect("nx ir json");
    let construction = find_ir_op(&plain, "record", "User").expect("record op");
    assert!(construction.get("isUpdate").is_none());
}

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

/// Finds the first IR expression op anywhere in `value` with the given tag that `matches`.
fn find_ir_op_where<'a>(
    value: &'a Value,
    tag: &str,
    matches: &dyn Fn(&Value) -> bool,
) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            if object.get("tag").and_then(Value::as_str) == Some(tag) && matches(value) {
                return Some(value);
            }
            object
                .values()
                .find_map(|child| find_ir_op_where(child, tag, matches))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|child| find_ir_op_where(child, tag, matches)),
        _ => None,
    }
}

#[test]
fn nx_ir_declares_a_referenced_property_union_with_its_target_and_cases() {
    let artifact = artifact_from_source(
        r#"
abstract type Named = { name:string }
type User extends Named = { email:string? }
let key() = {User.Property.email}
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let union = ir_declaration(&document, "User.Property");
    assert_eq!(union["kind"]["tag"], "union");
    assert_eq!(union["kind"]["propertyTarget"]["name"], "User");
    assert_eq!(union["kind"]["propertyTarget"]["kind"], "record");
    assert_eq!(union["kind"]["bases"].as_array().map(Vec::len), Some(0));
    let cases = union["kind"]["cases"].as_array().expect("cases");
    assert_eq!(
        cases
            .iter()
            .map(|case| case["name"].as_str().expect("case name"))
            .collect::<Vec<_>>(),
        ["name", "email"],
        "inherited fields first, in declaration order"
    );
    assert!(cases.iter().all(|case| case["isConstant"] == true));

    let features = document["requiredFeatures"].as_array().expect("features");
    assert!(features.contains(&Value::from("property-unions-v1")));
    assert!(
        !features.contains(&Value::from("update-intrinsics-v1")),
        "naming a field is not calling an intrinsic"
    );
}

#[test]
fn nx_ir_omits_property_unions_a_program_never_references() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string }
let user() = <User name="Ada" />
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let names = ir_declaration_names(&document);
    assert!(!names.iter().any(|name| name == "User.Property"));
    assert!(!names.iter().any(|name| name == "User.Update"));
    assert_eq!(
        document["requiredFeatures"],
        serde_json::json!(["eager-v1"]),
        "a program without property references lists what it did before they existed"
    );
    let user = ir_declaration(&document, "User");
    assert!(user["kind"].get("propertyTarget").is_none());
}

#[test]
fn nx_ir_encodes_a_property_case_as_a_constant_union_case() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string }
let key() = {User.Property.name}
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let key = ir_declaration(&document, "key");
    let case = find_ir_op_where(key, "unionCase", &|op| {
        op["union"]["name"] == "User.Property"
    })
    .expect("union case op");
    assert_eq!(case["caseName"], "name");
    assert_eq!(case["isConstant"], true);
    assert_eq!(
        ir_declaration(&document, "User.Property")["id"],
        case["union"]["declaration"]
    );
}

#[test]
fn nx_ir_encodes_an_intrinsic_call_distinctly_from_a_function_call() {
    let artifact = artifact_from_source(
        r#"
type User = { name:string }
let same(user:User) = {user}
let v() = {apply(<User name="Ada" />, <User.Update name="Bo" />)}
let w() = {same(<User name="Ada" />)}
let c() = {changed(<User.Update name="Bo" />)}
"#,
    );
    let generated = emit_nx_ir(&artifact).expect("nx ir output");
    let document: Value = serde_json::from_str(&generated.json).expect("nx ir json");

    let v = ir_declaration(&document, "v");
    let call = find_ir_op_where(v, "intrinsicCall", &|_| true).expect("intrinsic call op");
    assert_eq!(call["intrinsic"], "apply");
    assert_eq!(call["args"].as_array().map(Vec::len), Some(2));
    assert!(call.get("fieldOrder").is_none());
    assert!(find_ir_op_where(v, "call", &|_| true).is_none());

    // `changed` carries the target's declared field order with it.
    let c = ir_declaration(&document, "c");
    let call = find_ir_op_where(c, "intrinsicCall", &|_| true).expect("changed call op");
    assert_eq!(call["intrinsic"], "changed");
    assert_eq!(call["fieldOrder"], Value::Array(vec![Value::from("name")]));

    let w = ir_declaration(&document, "w");
    assert!(find_ir_op_where(w, "call", &|_| true).is_some());
    assert!(find_ir_op_where(w, "intrinsicCall", &|_| true).is_none());

    let features = document["requiredFeatures"].as_array().expect("features");
    assert!(features.contains(&Value::from("update-intrinsics-v1")));
    assert!(features.contains(&Value::from("update-records-v1")));
}

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
