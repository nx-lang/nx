//! NX IR emitter tests, over the explained text rather than the raw tables.
//!
//! The tables are an encoding; what a test wants to pin is what they mean. `explain_nx_ir` is the
//! one renderer of that meaning, so a test reads its output the way a person would read
//! `nxlang ir explain`, and a change to the encoding that keeps the meaning changes no test here.

use crate::ir::{kinds, NxIrArtifact, NxIrEmitOptions};
use crate::ir_image::{write_nx_ir_image, NxIrImage};
use crate::{
    build_nx_ir_artifacts, emit_nx_ir, explain_nx_ir, explain_nx_ir_image, ExplainError,
    NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1, NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1,
    NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1, NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1,
    NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1, NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION,
};
use nx_api::{
    build_program_artifact_from_source, build_workspace_program_artifact, LibraryRegistry,
    NxWorkspace, NxWorkspaceModule, ProgramArtifact, ProgramBuildContext,
};
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

fn artifact_from_source(source: &str) -> ProgramArtifact {
    build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
        .expect("program artifact should build")
}

fn artifact_from_workspace(files: &[(&str, &str)], entry: &str) -> ProgramArtifact {
    artifact_from_workspace_with(files, entry, &ProgramBuildContext::empty())
}

fn artifact_from_workspace_with(
    files: &[(&str, &str)],
    entry: &str,
    build_context: &ProgramBuildContext,
) -> ProgramArtifact {
    let modules = files
        .iter()
        .map(|(identity, source)| {
            NxWorkspaceModule::from_source(*identity, *source).expect("workspace module")
        })
        .collect::<Vec<_>>();
    let workspace = NxWorkspace::new(modules).expect("workspace");
    build_workspace_program_artifact(&workspace, entry, build_context)
        .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"))
}

/// The entry module's artifact, without debug data.
fn entry_artifact(artifact: &ProgramArtifact) -> NxIrArtifact {
    let program = crate::build_codegen_program(artifact).expect("codegen program");
    build_nx_ir_artifacts(&program, &NxIrEmitOptions::entry_only())
        .expect("nx ir")
        .remove(0)
}

/// Every module's artifact, keyed by identity, without debug data.
fn all_artifacts(artifact: &ProgramArtifact) -> BTreeMap<String, NxIrArtifact> {
    let program = crate::build_codegen_program(artifact).expect("codegen program");
    build_nx_ir_artifacts(
        &program,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            ..NxIrEmitOptions::default()
        },
    )
    .expect("nx ir")
    .into_iter()
    .map(|artifact| (artifact.modules[0].identity.clone(), artifact))
    .collect()
}

fn explain(artifact: &NxIrArtifact) -> String {
    explain_nx_ir(artifact).expect("explain")
}

fn explain_source(source: &str) -> String {
    explain(&entry_artifact(&artifact_from_source(source)))
}

fn explain_entry(files: &[(&str, &str)], entry: &str) -> String {
    explain(&entry_artifact(&artifact_from_workspace(files, entry)))
}

/// The entry module's image, without debug data.
fn image_bytes(artifact: &ProgramArtifact) -> Vec<u8> {
    emit_nx_ir(artifact, &NxIrEmitOptions::entry_only())
        .expect("nx ir")
        .remove(0)
        .bytes
}

/// The model read back from an emitted image.
fn read_back(bytes: &[u8]) -> NxIrArtifact {
    NxIrImage::open(bytes).expect("a valid image").to_artifact()
}

fn assert_line(text: &str, line: &str) {
    assert!(
        text.lines().any(|candidate| candidate.trim_end() == line),
        "expected the line {line:?} in:\n{text}"
    );
}

fn assert_contains(text: &str, needle: &str) {
    assert!(text.contains(needle), "expected {needle:?} in:\n{text}");
}

// ------------------------------------------------------------------------------------------------
// Header, determinism and layout
// ------------------------------------------------------------------------------------------------

#[test]
fn an_artifact_carries_the_header_its_module_and_its_entrypoints() {
    let artifact = artifact_from_source("let root() = { 1 + 2 }");
    let generated = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only()).expect("nx ir");
    assert_eq!(generated.len(), 1);
    let generated = &generated[0];
    let image = NxIrImage::open(&generated.bytes).expect("a valid image");

    assert_eq!(&generated.bytes[..4], b"NXIR");
    assert_eq!(image.schema_version(), NX_IR_SCHEMA_VERSION);
    assert_eq!(image.runtime_abi(), NX_IR_RUNTIME_ABI);
    assert_eq!(
        image.required_features().collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    let modules = image.modules().collect::<Vec<_>>();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].identity, "main.nx");
    assert_eq!(modules[0].version, "");
    assert_ne!(modules[0].fingerprint, 0);
    assert_eq!(image.function_entrypoints().to_vec(), [0]);
    assert!(image.component_entrypoints().is_empty());
    assert!(!image.has_debug());

    assert_eq!(generated.identity, "main.nx");
    assert_eq!(generated.metadata.identity, "main.nx");
    assert_eq!(generated.metadata.fingerprint, modules[0].fingerprint);
    assert_eq!(generated.metadata.schema_version, NX_IR_SCHEMA_VERSION);
    assert_eq!(generated.metadata.runtime_abi, NX_IR_RUNTIME_ABI);
    assert_eq!(
        generated.metadata.function_entrypoints,
        vec!["root".to_string()]
    );
    assert!(generated.metadata.component_entrypoints.is_empty());
}

#[test]
fn an_image_reads_back_as_the_model_it_was_written_from() {
    let source = "external component <B v:int />\nlet root() = { <B v=1 /> }";
    let artifact = entry_artifact(&artifact_from_source(source));
    let bytes = write_nx_ir_image(&artifact).expect("image");
    assert_eq!(bytes.len() % 4, 0, "an image is whole cells");
    assert_eq!(read_back(&bytes), artifact);
}

#[test]
fn nx_ir_output_is_deterministic() {
    let source = r#"
type Theme = light | dark
type User = { name:string score:int = 42 }
let root() = { <User name="Ada" /> }
"#;
    let first = image_bytes(&artifact_from_source(source));
    let second = image_bytes(&artifact_from_source(source));

    assert_eq!(first, second);
}

// ------------------------------------------------------------------------------------------------
// Emit options
// ------------------------------------------------------------------------------------------------

const CATALOG: &str =
    "export external component <SkiaLabel Text:string FontSize:int = 14 />\nexport external component <SkiaButton Text:string />";

/// The catalog at `catalog_version` and a snippet that uses it, with the catalog implicitly imported.
fn versioned_snippet_workspace(catalog_version: Option<&str>) -> ProgramArtifact {
    let mut catalog = NxWorkspaceModule::from_source("drawnui.nx", CATALOG).expect("catalog");
    if let Some(version) = catalog_version {
        catalog = catalog.with_version(version);
    }
    let input =
        NxWorkspaceModule::from_source("input.nx", "let root() = <SkiaLabel Text=\"hi\" />")
            .expect("input");
    let workspace = NxWorkspace::new(vec![catalog, input]).expect("workspace");
    build_workspace_program_artifact(
        &workspace,
        "input.nx",
        &ProgramBuildContext::empty().with_implicit_imports(["drawnui.nx"]),
    )
    .unwrap_or_else(|diagnostics| panic!("workspace artifact: {diagnostics:?}"))
}

fn snippet_workspace() -> ProgramArtifact {
    versioned_snippet_workspace(None)
}

#[test]
fn a_snippet_is_emitted_without_the_catalog_and_names_its_version() {
    let artifact = versioned_snippet_workspace(Some("9"));
    let generated = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only()).expect("nx ir");

    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].identity, "input.nx");
    let document = read_back(&generated[0].bytes);
    assert_eq!(document.modules[0].identity, "input.nx");
    assert_eq!(document.modules[1].identity, "drawnui.nx");
    assert_eq!(document.modules[1].version, "9");
    assert_eq!(document.declarations.len(), 1);
    // Nothing of the catalog rides along: not its other control, not its props.
    let strings = &document.strings;
    assert!(
        !strings.iter().any(|string| string == "SkiaButton"),
        "{strings:?}"
    );
    assert!(
        !strings.iter().any(|string| string == "FontSize"),
        "{strings:?}"
    );

    let text = explain_nx_ir_image(&generated[0].bytes).expect("explain");
    assert_contains(&text, "links drawnui.nx version \"9\" fingerprint ");
    assert_line(&text, "  <drawnui.nx:SkiaLabel Text=\"hi\" />");
}

#[test]
fn every_artifact_of_a_build_records_the_version_the_module_was_given() {
    let artifact = versioned_snippet_workspace(Some("9"));
    let generated = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");

    let versions = generated
        .iter()
        .map(|artifact| {
            let document = read_back(&artifact.bytes);
            let modules = document
                .modules
                .iter()
                .map(|module| (module.identity.clone(), module.version.clone()))
                .collect::<Vec<_>>();
            (artifact.identity.clone(), modules)
        })
        .collect::<BTreeMap<_, _>>();
    // The catalog's own artifact and the snippet's link to it agree, so the one links to the other.
    assert_eq!(
        versions["drawnui.nx"],
        vec![("drawnui.nx".to_string(), "9".to_string())]
    );
    assert_eq!(
        versions["input.nx"],
        vec![
            ("input.nx".to_string(), String::new()),
            ("drawnui.nx".to_string(), "9".to_string()),
        ]
    );
}

#[test]
fn a_program_built_under_another_version_is_another_program() {
    let unversioned = versioned_snippet_workspace(None);
    let nine = versioned_snippet_workspace(Some("9"));
    let ten = versioned_snippet_workspace(Some("10"));

    assert_ne!(unversioned.fingerprint, nine.fingerprint);
    assert_ne!(nine.fingerprint, ten.fingerprint);
    assert_eq!(
        nine.fingerprint,
        versioned_snippet_workspace(Some("9")).fingerprint
    );
}

#[test]
fn a_module_nothing_references_is_not_in_the_table() {
    let artifact = artifact_from_workspace(
        &[
            ("main.nx", "let root() = { 1 }"),
            ("other.nx", "export let answer() = { 2 }"),
        ],
        "main.nx",
    );
    let entry = entry_artifact(&artifact);
    assert_eq!(entry.modules.len(), 1);
    assert_eq!(entry.modules[0].identity, "main.nx");
}

#[test]
fn a_catalog_emits_as_its_own_artifact() {
    let artifact = artifact_from_workspace(&[("drawnui.nx", CATALOG)], "drawnui.nx");
    let generated = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only()).expect("nx ir");
    let document = read_back(&generated[0].bytes);

    assert_eq!(document.modules.len(), 1);
    assert_eq!(
        generated[0].metadata.component_entrypoints,
        vec!["SkiaLabel".to_string(), "SkiaButton".to_string()]
    );
    let text = explain_nx_ir_image(&generated[0].bytes).expect("explain");
    assert_line(&text, "component SkiaLabel external");
    assert_line(&text, "    FontSize: int = 14");
}

#[test]
fn every_module_can_be_emitted_with_the_entry_first() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { answer } from \"../shared/value.nx\"\nlet root() = { answer() }",
            ),
            ("shared/value.nx", "export let answer() = { 42 }"),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(Vec::new()),
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");
    let identities = generated
        .iter()
        .map(|artifact| artifact.identity.as_str())
        .collect::<Vec<_>>();
    assert_eq!(identities, ["app/main.nx", "shared/value.nx"]);

    let named = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(vec!["shared/value.nx".to_string()]),
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].identity, "shared/value.nx");
}

#[test]
fn an_unknown_module_selection_is_refused() {
    let artifact = artifact_from_source("let root() = { 1 }");
    let error = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(vec!["missing.nx".to_string()]),
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect_err("an unknown module cannot be emitted");
    assert!(
        error.diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == Some("nx-ir-unknown-module")
                && diagnostic.message().contains("missing.nx")
        }),
        "{:?}",
        error.diagnostics
    );
}

#[test]
fn the_debug_section_is_the_only_difference_between_stripped_and_debug_artifacts() {
    let artifact = snippet_workspace();
    let stripped = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only()).expect("nx ir");
    let with_debug = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            debug: true,
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");

    let stripped = read_back(&stripped[0].bytes);
    let mut with_debug = read_back(&with_debug[0].bytes);
    assert!(stripped.debug.is_none());
    let debug = with_debug.debug.take().expect("debug section");
    assert_eq!(with_debug, stripped);
    assert_eq!(debug.source, "let root() = <SkiaLabel Text=\"hi\" />");
    assert_eq!(debug.spans.declarations.len(), 1);
    assert_eq!(debug.spans.nodes.len(), stripped.nodes.len());
}

#[test]
fn emit_options_parse_from_the_sdks_json_form() {
    let options =
        NxIrEmitOptions::from_json(r#"{"modules":["a.nx"],"debug":true}"#).expect("options");
    assert_eq!(options.modules, Some(vec!["a.nx".to_string()]));
    assert!(options.debug);
    assert_eq!(
        NxIrEmitOptions::from_json("").expect("empty"),
        NxIrEmitOptions::default()
    );
    assert_eq!(
        NxIrEmitOptions::from_json("{}").expect("empty object"),
        NxIrEmitOptions::default()
    );
}

#[test]
fn emit_options_refuse_a_key_they_do_not_have() {
    // A version belongs to the module, not the emit; a caller still passing `versions` hears so
    // rather than having its versions silently dropped.
    let error = NxIrEmitOptions::from_json(r#"{"versions":{"drawnui.nx":"9"}}"#)
        .expect_err("versions is not an emit option");
    assert!(error.to_string().contains("versions"), "{error}");
}

// ------------------------------------------------------------------------------------------------
// Explain
// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------
// Primitive-to-text conversion and numeric widening
// ------------------------------------------------------------------------------------------------

#[test]
fn a_string_plus_an_int_is_emitted_as_concat_over_a_text_node() {
    let text = explain_source("let f(count:int) = { \"Total: \" + count }\nlet root() = { f(3) }");
    // The string constant is a bare operand; only the int is wrapped, and the node names `int`.
    assert_line(&text, "  (\"Total: \" concat text<int>(count))");
}

#[test]
fn a_float32_operand_names_its_type() {
    let text = explain_source("let f(w:float32) = { w + \" px\" }\nlet root() = { 1 }");
    assert_line(&text, "  (text<float32>(w) concat \" px\")");
}

#[test]
fn every_stringifiable_primitive_names_its_own_type() {
    let text = explain_source(
        "let f(a:int32, b:int64, c:float64, d:boolean) = { \"\" + a + b + c + d }\n\
         let root() = { 1 }",
    );
    for ty in ["int32", "int64", "float64", "boolean"] {
        assert_contains(&text, &format!("text<{ty}>("));
    }
}

#[test]
fn a_string_field_access_is_emitted_as_concat_with_no_text_node() {
    let text = explain_source(
        "type Item = { title:string }\n\
         let f(item:Item) = { \"Reorder \" + item.title }\n\
         let root() = { 1 }",
    );
    assert_line(&text, "  (\"Reorder \" concat item.title)");
    assert!(
        !text.contains("text<"),
        "no operand needs a conversion:\n{text}"
    );
}

#[test]
fn a_widened_operand_carries_no_conversion_node() {
    let text = explain_source(
        "let f(n:int, x:float64) = { n + x }\n\
         let g(n:int, x:float64) = { n / x }\n\
         let h(n:int, m:int32) = { n / m }\n\
         let root() = { 1 }",
    );
    assert_line(&text, "  (n add x)");
    // The checked type of the division is float64, so it is the floating-point operator even
    // though one operand is an integer.
    assert_line(&text, "  (n div x)");
    assert_line(&text, "  (n idiv m)");
    assert!(!text.contains("text<"), "widening emits nothing:\n{text}");
}

#[test]
fn a_division_of_a_widened_join_is_the_floating_point_operator() {
    // Each join is a float64, so dividing it by an int is `div` though one branch is an int, and
    // the narrower branch is emitted as it is, with no conversion node.
    let text = explain_source(
        "let pick(b:boolean, n:int, x:float64) = { if b { n } else { x } }\n\
         let ratio(b:boolean, n:int, x:float64, d:int) = { pick(b, n, x) / d }\n\
         let arm(k:int, n:int, x:float64) = { if k is { 1 => n  else => x } }\n\
         let armRatio(k:int, n:int, x:float64, d:int) = { arm(k, n, x) / d }\n\
         let root() = { 1 }",
    );
    assert_line(&text, "  (pick(b, n, x) div d)");
    assert_line(&text, "  (arm(k, n, x) div d)");
    assert!(
        !text.contains("idiv"),
        "every division is a float one:\n{text}"
    );
    assert!(!text.contains("text<"), "widening emits nothing:\n{text}");
}

#[test]
fn float32_arithmetic_is_the_float32_operator() {
    // A runtime carries a float32 as a float64, so the operator names the rounding. A remainder is
    // exact and keeps `mod`, and float64 arithmetic over a float32 operand is plain.
    let text = explain_source(
        "let f32(v:float32, d:float32) = { v + 0.1 - 0.2 * d / 2 }\n\
         let rem(v:float32, d:float32) = { v % d }\n\
         let wide(v:float32, x:float64) = { v * x }\n\
         let root() = { 1 }",
    );
    assert_line(
        &text,
        "  ((v fadd32 0.10000000149011612) fsub32 ((0.20000000298023224 fmul32 d) fdiv32 2.0))",
    );
    assert_line(&text, "  (v mod d)");
    assert_line(&text, "  (v mul x)");
}

#[test]
fn an_operator_in_a_widened_branch_is_chosen_by_the_branch_type() {
    // The widening is applied to the branch's result, so the branch computes at its own type:
    // integer division stays `idiv`, and float32 arithmetic stays `fmul32`.
    let text = explain_source(
        "let half(b:boolean, n:int, x:float64) = { if b { n / 2 } else { x } }\n\
         let pickf(b:boolean, v:float32, x:float64) = { if b { v * 3 } else { x } }\n\
         let arm(k:int, n:int, x:float64) = { if k is { 1 => n / 2  else => x } }\n\
         let list(n:int, x:float64) = { (n / 2) x }\n\
         let root() = { 1 }",
    );
    assert_line(&text, "    (n idiv 2)");
    assert_line(&text, "    (v fmul32 3.0)");
    assert!(
        !text.contains(" div "),
        "no branch is a float division:\n{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.contains("(n idiv 2)") && line.contains('x')),
        "the list element is integer division:\n{text}"
    );
}

#[test]
fn a_constant_expression_at_a_narrow_site_is_emitted_as_its_folded_literal() {
    // Folded at the literals' own types, then given the site's width: `7 / 2` is `3`, and a
    // constant operand of a float32 product is one float32 constant.
    let text = explain_source(
        "let half(): float32 = { 7 / 2 }\n\
         let scaled(w:float32) = { w * (1.5 * 2) }\n\
         let wide() = { 7 / 2 }\n\
         let root() = { 1 }",
    );
    assert_line(&text, "  3.0");
    assert_line(&text, "  (w fmul32 3.0)");
    // Without a site to narrow to, the expression is left for evaluation.
    assert_line(&text, "  (7 idiv 2)");
}

#[test]
fn a_text_body_at_a_string_content_property_is_emitted_as_one_concat_chain() {
    let text = explain_source(
        "type Label = { content text:string }\n\
         let f(count:int) = <Label>Total: {count} left</Label>\n\
         let root() = { 1 }",
    );
    // The space before `left` is layout the grammar keeps out of the run, put back as written.
    assert_contains(
        &text,
        "(((\"Total: \" concat text<int>(count)) concat \" \") concat \"left\")",
    );
}

#[test]
fn a_text_node_is_kind_20_and_reads_back_from_the_image() {
    assert_eq!(kinds::node::TEXT, 20);
    assert_eq!(
        kinds::name(kinds::node::NAMES, kinds::node::TEXT),
        Some("text")
    );

    let artifact = artifact_from_source("let root(count:int) = { \"n=\" + count }");
    let model = entry_artifact(&artifact);
    assert_eq!(read_back(&image_bytes(&artifact)), model);
}

#[test]
fn explain_refuses_another_schema_version_naming_both() {
    let artifact = artifact_from_source("let root() = { 1 }");
    let mut bytes = image_bytes(&artifact);
    bytes[4..8].copy_from_slice(&3u32.to_le_bytes());
    let error = explain_nx_ir_image(&bytes).expect_err("schema 3 is refused");
    assert_eq!(
        error,
        ExplainError::SchemaVersion {
            found: 3,
            supported: 4
        }
    );
    assert!(error.to_string().contains("schema version 3"));
    assert!(error.to_string().contains("schema version 4"));
}

#[test]
fn explain_annotates_declarations_with_line_and_column_when_debug_is_present() {
    let artifact = artifact_from_source("let answer() = { 1 }\n\nlet root() = { answer() }");
    let generated = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            debug: true,
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");
    let text = explain_nx_ir_image(&generated[0].bytes).expect("explain");
    assert_line(&text, "function answer() @1:1-1:21 =");
    assert_line(&text, "function root() @3:1-3:26 =");
    assert_line(&text, "  answer()");
}

/// A literal carries no span of its own; its span lives in the module's span map. Reading the node
/// instead left every literal and identifier pointing at offset zero, which is line 1 column 1 for
/// every diagnostic a runtime reports against one.
#[test]
fn explain_annotates_literal_and_identifier_nodes_with_where_they_were_written() {
    let artifact = artifact_from_source("let answer() = { 41 }\n\nlet root() = { answer() }");
    let generated = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            debug: true,
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");
    let document = read_back(&generated[0].bytes);
    let spans = &document.debug.expect("debug section").spans.nodes;
    assert!(
        !spans.is_empty() && spans.iter().all(|span| *span != [0, 0]),
        "every node span should name where it was written, but some are [0, 0]: {spans:?}"
    );
    assert!(
        spans.contains(&[17, 19]),
        "the literal 41 should be located at its own offsets: {spans:?}"
    );
}

/// `ir explain` is the support tool for artifacts another toolchain wrote or a person edited, so a
/// span that does not land on a character boundary must read as text, not panic the CLI.
#[test]
fn explain_reads_a_span_that_falls_inside_a_character() {
    let artifact = artifact_from_source("let root() = { \"héllo\" }");
    let program = crate::build_codegen_program(&artifact).expect("codegen program");
    let mut document = build_nx_ir_artifacts(
        &program,
        &NxIrEmitOptions {
            debug: true,
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir")
    .remove(0);
    // The second byte of the 'é' at offset 17, and an offset past the end of the source.
    document
        .debug
        .as_mut()
        .expect("debug section")
        .spans
        .declarations[0] = [18, 1_000_000];
    let bytes = write_nx_ir_image(&document).expect("image");
    let text = explain_nx_ir_image(&bytes).expect("explain");
    assert_line(&text, "function root() @1:18-1:25 =");
}

// ------------------------------------------------------------------------------------------------
// References
// ------------------------------------------------------------------------------------------------

/// Generated IR must contain no reference that failed to resolve to a declaration.
///
/// `Fit` is not imported by `app.nx`, so there is no visible name to resolve `cover` by. The
/// reference carries the union's declaring origin instead.
#[test]
fn a_case_of_an_unimported_union_is_referenced_by_its_declaring_module() {
    let text = explain_entry(
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
    // `let <Img ... />` declares a function, so the element is a call.
    assert_line(&text, "  widgets.nx:Img(widgets.nx:Fit.cover)");
}

/// The same, for a case reached through the name a selective import alias bound.
#[test]
fn a_case_of_an_aliased_union_is_referenced_by_its_declaring_module() {
    let text = explain_entry(
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
    assert_line(&text, "  widgets.nx:Img(widgets.nx:Fit.cover)");
}

/// A default naming a field materialized after it cannot be emitted.
#[test]
fn a_default_naming_a_field_declared_after_it_is_refused() {
    let artifact = artifact_from_source(
        "abstract component <Node />\n\
         external component <Leaf extends Node />\n\
         component <A extends Node a:int = {b} b:int = 1 /> = { <Leaf /> }\n\
         let root() = { <A /> }",
    );

    let error = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only())
        .expect_err("a default with nothing to read should not emit");

    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("'b'")),
        "expected the unreadable name to be reported, got: {:?}",
        error.diagnostics
    );
}

/// A component body is a value position like any other, so a bare case there reaches the IR, and
/// both spellings of the case emit the same thing.
#[test]
fn a_bare_case_in_a_component_body_emits_like_the_qualified_form() {
    let preamble = "type Fit = fill | contain | cover\nexternal component <Img fit:Fit? />\nabstract external component <Node />\n";
    let bare = explain_source(&format!(
        "{preamble}component <A extends Node /> = {{ <Img fit=cover /> }}\nlet root() = {{ <A /> }}"
    ));
    let qualified = explain_source(&format!(
        "{preamble}component <A extends Node /> = {{ <Img fit={{Fit.cover}} /> }}\nlet root() = {{ <A /> }}"
    ));
    assert_line(&bare, "    <Img fit=Fit.cover />");
    // The two sources differ, so their fingerprints on the first line do; nothing else may.
    let body = |text: &str| text.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(body(&bare), body(&qualified));
}

#[test]
fn a_call_across_modules_names_the_declaring_module() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { answer } from \"../shared/value.nx\"\nlet root(): int = { answer() }",
            ),
            ("shared/value.nx", "export let answer(): int = { 42 }"),
        ],
        "app/main.nx",
    );
    let entry = entry_artifact(&artifact);
    assert_eq!(entry.modules.len(), 2);
    assert_eq!(entry.modules[1].identity, "shared/value.nx");
    assert_line(&explain(&entry), "  shared/value.nx:answer()");
}

/// Fields are flattened into the IR, so the base chain is what tells a runtime that a value
/// stamped `User` is acceptable where a `Base` was asked for.
#[test]
fn records_and_unions_carry_their_base_chain() {
    let text = explain_entry(
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
    // Nearest first, and reaching past the immediate base to the one it extends.
    assert_line(&text, "record Named abstract extends shared/model.nx:Base");
    assert_line(&text, "record User extends Named, shared/model.nx:Base");
    // A union's cases all inherit the union's base, so the chain sits on the union itself.
    assert_line(&text, "union Shape extends shared/model.nx:Base");
    assert_line(&text, "record Loose");
    // Inherited fields come first, flattened into the record.
    assert_line(&text, "  name: string required");
    assert_line(&text, "  label: string required");
    assert_line(&text, "  role: string required");
}

#[test]
fn nominal_type_references_name_their_declaring_module() {
    let text = explain_entry(
        &[
            (
                "app/main.nx",
                "import { User } from \"../shared/model.nx\"\nlet root(user: User): User = { user }",
            ),
            ("shared/model.nx", "export type User = { name:string }"),
        ],
        "app/main.nx",
    );
    assert_line(&text, "function root(user: shared/model.nx:User) =");
}

#[test]
fn a_declared_element_type_is_not_shadowed_by_the_builtin_element_supertype() {
    let text = explain_entry(
        &[
            (
                "app/main.nx",
                "import { Element } from \"../shared/model.nx\"\nlet root(value: Element): Element = { value }",
            ),
            ("shared/model.nx", "export type Element = { id:string }"),
        ],
        "app/main.nx",
    );
    assert_line(&text, "function root(value: shared/model.nx:Element) =");
}

#[test]
fn directory_loaded_cross_library_type_references_survive_per_module_emission() {
    let temp = TempDir::new().expect("temp dir");
    let dirs = ["flow-step", "ui", "question-flow", "chat-link"].map(|name| temp.path().join(name));
    for dir in &dirs {
        fs::create_dir_all(dir).expect("dir");
    }
    fs::write(
        dirs[0].join("FlowStep.nx"),
        "export type FlowStep = { id:string }",
    )
    .expect("write");
    fs::write(
        dirs[1].join("TextInput.nx"),
        "export external component <TextInput value:string />",
    )
    .expect("write");
    fs::write(
        dirs[2].join("QuestionFlow.nx"),
        "import { FlowStep } from \"../flow-step\"\nimport { TextInput } from \"../ui\"\nexport type QuestionFlow = { firstStep:FlowStep input:TextInput }",
    )
    .expect("write");
    fs::write(
        dirs[3].join("ChatLinkConfig.nx"),
        "import { QuestionFlow } from \"../question-flow\"\nexport type ChatLinkConfig = { questionFlow:QuestionFlow }",
    )
    .expect("write");

    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&dirs[2])
        .expect("question-flow library");
    registry
        .load_library_from_directory(&dirs[3])
        .expect("chat-link library");
    let artifact = artifact_from_workspace_with(
        &[(
            "app/main.nx",
            "import { ChatLinkConfig } from \"../chat-link\"\nlet root() = { \"ready\" }",
        )],
        "app/main.nx",
        &registry.build_context(),
    );

    let artifacts = all_artifacts(&artifact);
    let chat_link = artifacts
        .iter()
        .find(|(identity, _)| identity.ends_with("ChatLinkConfig.nx"))
        .map(|(_, artifact)| explain(artifact))
        .expect("chat-link artifact");
    let question_flow = artifacts
        .iter()
        .find(|(identity, _)| identity.ends_with("QuestionFlow.nx"))
        .map(|(_, artifact)| explain(artifact))
        .expect("question-flow artifact");

    assert_contains(&chat_link, "QuestionFlow.nx:QuestionFlow required");
    assert_contains(&question_flow, "FlowStep.nx:FlowStep required");
    assert_contains(&question_flow, "TextInput.nx:TextInput required");
}

#[test]
fn nullable_unions_content_fields_and_descriptors_read_as_written() {
    let temp = TempDir::new().expect("temp dir");
    let flow_dir = temp.path().join("flow");
    let ui_dir = temp.path().join("ui");
    fs::create_dir_all(&flow_dir).expect("flow dir");
    fs::create_dir_all(&ui_dir).expect("ui dir");
    fs::write(
        flow_dir.join("Flow.nx"),
        "export type FlowCompletion = continue | end { message:string }\nexport type QuestionFlow = {\n  completion:FlowCompletion?\n  content steps:Element\n}",
    )
    .expect("flow source");
    fs::write(
        ui_dir.join("Panel.nx"),
        "export external component <Panel content body:Element />",
    )
    .expect("ui source");
    let registry = LibraryRegistry::new();
    registry
        .load_library_from_directory(&flow_dir)
        .expect("flow library");
    registry
        .load_library_from_directory(&ui_dir)
        .expect("ui library");
    let artifact = artifact_from_workspace_with(
        &[(
            "app/main.nx",
            "import { QuestionFlow } from \"../flow\"\nimport { Panel } from \"../ui\"\nlet omitted(): QuestionFlow = { <QuestionFlow><Panel><span /></Panel></QuestionFlow> }\nlet explicit(): QuestionFlow = { <QuestionFlow completion={null}><Panel><span /></Panel></QuestionFlow> }\nlet root(): QuestionFlow[] = { omitted() explicit() }",
        )],
        "app/main.nx",
        &registry.build_context(),
    );

    let artifacts = all_artifacts(&artifact);
    let flow = artifacts
        .iter()
        .find(|(identity, _)| identity.ends_with("Flow.nx"))
        .map(|(_, artifact)| explain(artifact))
        .expect("flow artifact");
    assert_line(&flow, "  completion: FlowCompletion?");
    // `Element` is the top type of elements, which the IR spells as `object`.
    assert_line(&flow, "  content steps: object required");

    let main = explain(&artifacts["app/main.nx"]);
    assert_contains(&main, "Flow.nx:QuestionFlow>");
    assert_contains(&main, "Panel.nx:Panel>");
    assert_line(&main, "      <span />");
    assert_contains(&main, "Flow.nx:QuestionFlow completion=null>");
}

#[test]
fn missing_semantic_data_fails_without_a_partial_artifact() {
    let mut artifact = artifact_from_source("let root() = { 42 }");
    artifact.root_modules[0].lowered_module = None;

    let error = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only())
        .expect_err("missing semantic data should fail IR emission");
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == Some("codegen-missing-semantic-data")),
        "diagnostics: {:?}",
        error.diagnostics
    );
}

/// A default inherited from a base component declared in another module was written in that
/// module's text, so it carries no span into this artifact's source.
#[test]
fn an_inherited_default_carries_no_span_into_the_artifact() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { Question } from \"../shared/ui.nx\"\nexternal component <ShortTextQuestion extends Question placeholder:string? />\nlet root() = { <ShortTextQuestion /> }",
            ),
            (
                "shared/ui.nx",
                "export abstract external component <Question label:string = \"Untitled\" />",
            ),
        ],
        "app/main.nx",
    );
    let generated = emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            debug: true,
            ..NxIrEmitOptions::entry_only()
        },
    )
    .expect("nx ir");
    let document = read_back(&generated[0].bytes);
    let debug = document.debug.clone().expect("debug section");
    let spans = &debug.spans.nodes;
    assert!(spans.contains(&[-1, -1]), "{spans:?}");
    assert!(spans.iter().any(|span| span[0] >= 0), "{spans:?}");
    assert_eq!(spans.len(), document.nodes.len());
    let text = explain(&document);
    assert_line(&text, "    label: string = \"Untitled\"");
    assert_line(&text, "    placeholder: string?");
}

// ------------------------------------------------------------------------------------------------
// Nodes and literals
// ------------------------------------------------------------------------------------------------

#[test]
fn loops_matches_union_cases_and_big_integers_read_as_written() {
    let loops =
        explain_source("let <Labels items:int[] /> = {for item, index in items { item + index }}");
    assert_line(&loops, "  for item, index in items yield");
    assert_line(&loops, "    (item add index)");

    let matches = explain_source(
        "let root(x:int): string = {\n    if x is {\n        0 => \"zero\"\n        1, 2 => \"small\"\n        else => \"many\"\n    }\n}",
    );
    assert_line(&matches, "  if x is");
    assert_line(&matches, "    0 =>");
    assert_line(&matches, "      \"zero\"");
    assert_line(&matches, "    1 | 2 =>");
    assert_line(&matches, "    else =>");
    assert_line(&matches, "      \"many\"");

    let union = explain_source(
        "type LoadState = idle | failed { message:string }\nlet root(): LoadState = { <LoadState.failed message={\"Offline\"} /> }",
    );
    assert_line(&union, "  <LoadState.failed message=\"Offline\" />");

    let big = entry_artifact(&artifact_from_source("let root() = { 9007199254740993 }"));
    let digits = big
        .strings
        .iter()
        .position(|string| string == "9007199254740993")
        .expect("the digits are interned") as i64;
    assert_eq!(
        big.constants,
        vec![crate::IrItem::ints([kinds::constant::BIGINT, digits])]
    );
    assert_line(&explain(&big), "  9007199254740993");
}

/// An expression written inside an intrinsic element carries the same operator as the same
/// expression written outside one: the division of two ints is integer division either way.
#[test]
fn a_division_inside_an_intrinsic_element_is_integer_division() {
    let inside = explain_source("let root() = <div>{7 / 2}</div>");
    assert_line(&inside, "    (7 idiv 2)");
    let outside = explain_source("let root() = { 7 / 2 }");
    assert_line(&outside, "  (7 idiv 2)");
    let real = explain_source("let root() = { 7.0 / 2.0 }");
    assert_line(&real, "  (7.0 div 2.0)");
}

fn float_property_program(literal: &str) -> String {
    format!("external component <B v:float64 />\nlet root() = {{ <B v={literal} /> }}\n")
}

/// Strips what two programs cannot share when their source text differs: the fingerprint over it.
fn without_fingerprint(artifact: &ProgramArtifact) -> NxIrArtifact {
    let mut artifact = entry_artifact(artifact);
    for module in &mut artifact.modules {
        module.fingerprint = 0;
    }
    artifact
}

#[test]
fn an_int_literal_at_a_float_site_emits_the_same_ir_as_a_real_literal() {
    assert_eq!(
        without_fingerprint(&artifact_from_source(&float_property_program("24"))),
        without_fingerprint(&artifact_from_source(&float_property_program("24.0")))
    );
    let float32 = |literal: &str| {
        format!("external component <B v:float32 />\nlet root() = {{ <B v={literal} /> }}\n")
    };
    assert_eq!(
        without_fingerprint(&artifact_from_source(&float32("24"))),
        without_fingerprint(&artifact_from_source(&float32("24.0")))
    );
    let lists = |x: &str, items: &str| {
        format!(
            "type Opts = {{ x:float64 = {x} }}\nexternal component <B v:float64[] />\nlet root() = {{ <B v={{{items}}} /> }}\n"
        )
    };
    assert_eq!(
        without_fingerprint(&artifact_from_source(&lists("0", "1 2 3"))),
        without_fingerprint(&artifact_from_source(&lists("0.0", "1.0 2.0 3.0")))
    );
}

#[test]
fn an_int_literal_at_a_float_site_is_not_emitted_as_an_integer_constant() {
    let artifact = entry_artifact(&artifact_from_source(&float_property_program("24")));
    // Asserting only equality with the real spelling would pass if both emitted an integer.
    assert_eq!(
        artifact.constants,
        vec![crate::IrItem::list([
            crate::IrItem::Int(kinds::constant::FLOAT),
            crate::IrItem::Float(24.0)
        ])]
    );
    assert_line(&explain(&artifact), "  <B v=24.0 />");
}

#[test]
fn a_component_reads_with_its_props_state_and_body() {
    let text = explain_source(
        "external component <TextInput value:string />\ncomponent <SearchBox placeholder:string = \"Find docs\" /> = {\n  state { query:string = { placeholder } }\n  <TextInput value={query} />\n}\nlet root() = { <SearchBox /> }",
    );
    assert_line(&text, "component TextInput external");
    assert_line(&text, "component SearchBox");
    assert_line(&text, "  props");
    assert_line(&text, "    placeholder: string = \"Find docs\"");
    assert_line(&text, "  state");
    assert_line(&text, "    query: string = placeholder");
    assert_line(&text, "  body =");
    assert_line(&text, "    <TextInput value=query />");
}

#[test]
fn slots_are_declaration_local_integers() {
    let artifact = entry_artifact(&artifact_from_source(
        "let f(a:int[], b:int) = { for x in a { x + b } }\nlet g(c:int) = { c }",
    ));
    let slots = artifact
        .nodes
        .iter()
        .filter_map(|node| {
            let entry = node.as_list()?;
            (entry[0].as_int()? == kinds::node::SLOT).then(|| {
                (
                    entry[1].as_int().unwrap(),
                    artifact.strings[entry[2].as_int().unwrap() as usize].clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    // `a` and `b` are parameters 0 and 1 of `f`; `x` is the next local, 2; `c` is 0 again in `g`.
    assert_eq!(
        slots,
        vec![
            (0, "a".to_string()),
            (2, "x".to_string()),
            (1, "b".to_string()),
            (0, "c".to_string()),
        ]
    );
}

#[test]
fn a_name_and_a_type_are_written_once() {
    let source = "external component <Label Text:string? />\nlet root() = { <Label Text=\"a\" /> <Label Text=\"b\" /> <Label Text=\"c\" /> }";
    let artifact = entry_artifact(&artifact_from_source(source));
    assert_eq!(
        artifact
            .strings
            .iter()
            .filter(|string| string.as_str() == "Text")
            .count(),
        1
    );
    let nullable_strings = artifact
        .types
        .iter()
        .filter(|ty| ty.as_list().and_then(|entry| entry[0].as_int()) == Some(kinds::ty::NULLABLE))
        .count();
    assert_eq!(nullable_strings, 1);
}

// ------------------------------------------------------------------------------------------------
// Derived declarations and intrinsics
// ------------------------------------------------------------------------------------------------

#[test]
fn a_referenced_update_record_is_declared_with_its_own_schema() {
    let artifact = artifact_from_source(
        "type User = { name:string = \"anon\" email:string? }\nlet patch() = <User.Update email={null} />",
    );
    let entry = entry_artifact(&artifact);
    let text = explain(&entry);
    assert_line(&text, "record User.Update update of User");
    // Every update field is optional and none has a default, so the lines carry neither.
    let update_lines = text
        .lines()
        .skip_while(|line| *line != "record User.Update update of User")
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .collect::<Vec<_>>();
    assert_eq!(update_lines, vec!["  name: string", "  email: string?"]);
    assert_line(&text, "  <User.Update email=null />");
    assert!(entry
        .required_features
        .contains(&NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1.to_string()));
}

#[test]
fn derived_declarations_a_program_never_references_are_omitted() {
    let entry = entry_artifact(&artifact_from_source(
        "type User = { name:string }\nlet user() = <User name=\"Ada\" />",
    ));
    let text = explain(&entry);
    assert!(!text.contains("User.Update"), "{text}");
    assert!(!text.contains("User.Property"), "{text}");
    assert!(
        entry.required_features.is_empty(),
        "{:?}",
        entry.required_features
    );
}

#[test]
fn an_update_construction_names_the_update_record() {
    let text = explain_source(
        "component <Counter /> = { state { count:int = 0 label:string = \"x\" } <Label /> }\nlet reset() = <Counter.Update count=1 />",
    );
    assert_line(&text, "record Counter.Update update of Counter");
    assert_line(&text, "  <Counter.Update count=1 />");
}

#[test]
fn a_referenced_property_union_is_declared_with_its_target_and_cases() {
    let entry = entry_artifact(&artifact_from_source(
        "abstract type Named = { name:string }\ntype User extends Named = { email:string? }\nlet key() = {User.Property.email}",
    ));
    let text = explain(&entry);
    assert_line(&text, "union User.Property property of User");
    // Inherited fields first, in declaration order; every case is constant.
    let cases = text
        .lines()
        .skip_while(|line| *line != "union User.Property property of User")
        .skip(1)
        .take_while(|line| line.starts_with("  case"))
        .collect::<Vec<_>>();
    assert_eq!(cases, vec!["  case name constant", "  case email constant"]);
    assert_line(&text, "  User.Property.email");
    assert!(entry
        .required_features
        .contains(&NX_IR_REQUIRED_FEATURE_PROPERTY_UNIONS_V1.to_string()));
    assert!(
        !entry
            .required_features
            .contains(&NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1.to_string()),
        "naming a field is not calling an intrinsic"
    );
}

#[test]
fn an_intrinsic_call_is_distinct_from_a_function_call() {
    let entry = entry_artifact(&artifact_from_source(
        "type User = { name:string }\nlet same(user:User) = {user}\nlet v() = {apply(<User name=\"Ada\" />, <User.Update name=\"Bo\" />)}\nlet w() = {same(<User name=\"Ada\" />)}\nlet c() = {changed(<User.Update name=\"Bo\" />)}",
    ));
    let text = explain(&entry);
    assert_line(
        &text,
        "  apply(<User name=\"Ada\" />, <User.Update name=\"Bo\" />)",
    );
    assert_line(&text, "  same(<User name=\"Ada\" />)");
    // `changed` carries the target's declared field order with it.
    assert_line(
        &text,
        "  changed(<User.Update name=\"Bo\" />) ordering name",
    );
    assert!(entry
        .required_features
        .contains(&NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1.to_string()));
    assert!(entry
        .required_features
        .contains(&NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1.to_string()));
}

// ------------------------------------------------------------------------------------------------
// Component type parameters
// ------------------------------------------------------------------------------------------------

/// NX IR carries no component type parameters: the prop schema is erased to the top type, and
/// the descriptor carries no property for the argument the source bound.
#[test]
fn a_component_type_parameter_is_erased() {
    let text = explain_source(
        "type Contact = { name:string }\nexternal component <SkiaLayout TItem:type itemsSource:TItem[]? />\nlet v = <SkiaLayout TItem=Contact itemsSource={} />\nlet root() = { v }",
    );
    assert_line(&text, "    itemsSource: object[]?");
    assert_line(&text, "  <SkiaLayout itemsSource=[] />");
    assert!(!text.contains("TItem"), "{text}");
}

#[test]
fn an_update_record_of_a_generic_component_erases_the_parameter() {
    let text = explain_source(
        "external component <Label text:string? />\ncomponent <List TItem:type items:TItem[]? /> = { state { sel:TItem? = null } <Label /> }\nlet u = <List.Update sel=null />\nlet root() = { u }",
    );
    assert_line(&text, "record List.Update update of List");
    assert_line(&text, "  sel: object?");
    assert!(!text.contains("TItem"), "{text}");
}

// ------------------------------------------------------------------------------------------------
// Record type parameters
// ------------------------------------------------------------------------------------------------

/// NX IR carries no record type parameters: a parameter-typed field is the top type, and the
/// declaration has no entry for the parameter itself.
#[test]
fn a_record_type_parameter_is_erased_from_the_field_schema() {
    let text = explain_source(
        "type Range = { T:type start:T end:T endInclusive:boolean }\n\
         let r = <Range T=int start={1} end={5} endInclusive={false} />\n\
         let root() = { r }",
    );
    assert_line(&text, "  start: object required");
    assert_line(&text, "  end: object required");
    assert_line(&text, "  endInclusive: boolean required");
    assert!(!text.contains("T:"), "{text}");
}

/// An applied type is the nominal reference to its record, under every wrapper.
#[test]
fn an_applied_type_is_a_nominal_reference() {
    let text = explain_source(
        "type Range = { T:type start:T end:T }\n\
         type Slider = { range:<Range T=float64/> marks:<Range T=int/>[]? }\n\
         let s = <Slider range={<Range T=float64 start={0} end={1} />} />\n\
         let root() = { s }",
    );
    assert_line(&text, "  range: Range required");
    assert_line(&text, "  marks: Range[]?");
}

/// A construction carries its fields and nothing for the type argument the source bound.
#[test]
fn a_generic_record_construction_omits_the_type_argument() {
    let text = explain_source(
        "type Range = { T:type start:T end:T }\n\
         let r = <Range T=int start={1} end={5} />\n\
         let root() = { r }",
    );
    assert_contains(&text, "start=");
    assert_contains(&text, "end=");
    assert!(!text.contains("T="), "{text}");
}

/// The update companion of a generic record erases the parameter the same way its record does.
#[test]
fn the_update_companion_of_a_generic_record_erases_the_parameter() {
    let text = explain_source(
        "type Range = { T:type start:T end:T }\n\
         let u = <Range.Update T=int end={9} />\n\
         let root() = { u }",
    );
    assert_line(&text, "record Range.Update update of Range");
    assert_line(&text, "  end: object");
    assert!(!text.contains("T="), "{text}");
}

/// Adding generic records does not change the schema version, and an image for a program with one
/// is as deterministic as any other.
#[test]
fn a_program_with_generic_records_keeps_the_schema_version() {
    let source = "type Range = { T:type start:T end:T }\n\
         let r = <Range T=int start={1} end={5} />\n\
         let root() = { r }";
    let artifact = artifact_from_source(source);
    let generated = emit_nx_ir(&artifact, &NxIrEmitOptions::entry_only()).expect("nx ir");
    let image = NxIrImage::open(&generated[0].bytes).expect("a valid image");
    assert_eq!(image.schema_version(), NX_IR_SCHEMA_VERSION);
    assert_eq!(image.runtime_abi(), NX_IR_RUNTIME_ABI);
    assert_eq!(
        image.required_features().collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "a generic record needs no feature an older consumer would not know"
    );
    assert_eq!(
        image_bytes(&artifact_from_source(source)),
        image_bytes(&artifact_from_source(source))
    );
}

// ------------------------------------------------------------------------------------------------
// Action handlers and component emits
// ------------------------------------------------------------------------------------------------

const COUNTER_SOURCE: &str = r#"
action Reset = { }
external component <Button label:string emits { Tapped { } } />
component <Counter emits { Reset ValueChanged { value:int } } /> = {
  state { count:int = 0 }
  <Button label="Add" onTapped=<Update count={count + 1} /> />
}
component <Plain /> = { <Button label="x" /> }
"#;

const SEARCH_SOURCE: &str = r#"
external component <TextInput />
component <SearchBox emits { SearchSubmitted { searchString:string } } /> = { <TextInput /> }
action DoSearch = { search:string }
let root() = { <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> /> }
"#;

/// The node with `kind`, and its entry, from an artifact's node table.
fn node_of_kind(artifact: &NxIrArtifact, kind: i64) -> &[crate::IrItem] {
    artifact
        .nodes
        .iter()
        .filter_map(|node| node.as_list())
        .find(|node| node[0].as_int() == Some(kind))
        .unwrap_or_else(|| panic!("no node of kind {kind}"))
}

#[test]
fn component_declarations_carry_their_emits_in_declaration_order() {
    let text = explain_source(COUNTER_SOURCE);
    let counter = text
        .split("\ncomponent Counter\n")
        .nth(1)
        .and_then(|rest| rest.split("\n\n").next())
        .expect("the Counter declaration");
    assert_contains(
        counter,
        "  emits\n    Reset = Reset\n    ValueChanged = Counter.ValueChanged\n  body =",
    );
    // An inline emit's record is an ordinary record of the module.
    assert_line(&text, "record Counter.ValueChanged");
    let plain = text
        .split("\ncomponent Plain\n")
        .nth(1)
        .and_then(|rest| rest.split("\n\n").next())
        .expect("the Plain declaration");
    assert!(!plain.contains("emits"), "{plain}");
}

#[test]
fn an_inherited_emit_references_its_declaring_module() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { Base } from \"../ui/base.nx\"\nexternal component <Button extends Base />\nlet root() = { <Button /> }",
            ),
            (
                "ui/base.nx",
                "export abstract external component <Base emits { Tapped { } } />",
            ),
        ],
        "app/main.nx",
    );
    let text = explain(&entry_artifact(&artifact));
    assert_line(&text, "    Tapped = ui/base.nx:Base.Tapped");
}

#[test]
fn a_handler_bound_to_an_inherited_emit_references_the_emit_module_action() {
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { Base } from \"../ui/base.nx\"\naction Log = { }\nexternal component <Button extends Base />\nlet root() = { <Button onTapped=<Log /> /> }",
            ),
            (
                "ui/base.nx",
                "export abstract external component <Base emits { Tapped { } } />",
            ),
        ],
        "app/main.nx",
    );
    let text = explain(&entry_artifact(&artifact));
    assert_line(
        &text,
        "    onTapped=handler Button.Tapped action@0:ui/base.nx:Base.Tapped =>",
    );
}

#[test]
fn a_handler_names_the_component_its_descriptor_names() {
    // An aliased import beside a local component of the same name: the handler on `ui.Button`
    // answers the imported component, as the descriptor does, not the local one.
    let artifact = artifact_from_workspace(
        &[
            (
                "app/main.nx",
                "import { Button as ui.Button } from \"../ui/button.nx\"\naction Log = { }\nexternal component <Button emits { Tapped { } } />\nlet root() = { <ui.Button onTapped=<Log /> /> }",
            ),
            (
                "ui/button.nx",
                "export external component <Button emits { Tapped { } } />",
            ),
        ],
        "app/main.nx",
    );
    let text = explain(&entry_artifact(&artifact));
    assert_line(&text, "  <ui/button.nx:Button");
    assert_line(
        &text,
        "    onTapped=handler ui/button.nx:Button.Tapped action@0:ui/button.nx:Button.Tapped =>",
    );
}

#[test]
fn an_intrinsic_call_inside_a_handler_lists_the_intrinsic_feature() {
    let artifact = entry_artifact(&artifact_from_source(
        r#"
type User = { name:string }
external component <Button emits { Tapped { } } />
component <Editor /> = {
  state { user:User = <User name="a" /> }
  <Button onTapped=<Update user={apply(user, <User.Update name="b" />)} /> />
}
"#,
    ));
    assert!(
        artifact
            .required_features
            .iter()
            .any(|feature| feature == NX_IR_REQUIRED_FEATURE_UPDATE_INTRINSICS_V1),
        "{:?}",
        artifact.required_features
    );
}

#[test]
fn a_handler_bound_inside_a_component_body_is_encoded_with_its_owner() {
    let artifact = entry_artifact(&artifact_from_source(COUNTER_SOURCE));
    assert_eq!(
        artifact.required_features,
        [
            NX_IR_REQUIRED_FEATURE_UPDATE_RECORDS_V1,
            NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1
        ]
    );
    let text = explain(&artifact);
    assert_line(
        &text,
        "      onTapped=handler Button.Tapped action@1:Button.Tapped owner Counter =>",
    );
    assert_line(&text, "        <Counter.Update count=(count add 1) />");

    // `[19, ref, str, ref, slot, ref?, node]`: `Counter`'s frame is its one state field at slot
    // 0, so `action` takes slot 1 and the body reads `count` through slot 0.
    let handler = node_of_kind(&artifact, kinds::node::ACTION_HANDLER);
    assert_eq!(handler.len(), 9);
    assert_eq!(
        artifact.strings[handler[2].as_int().unwrap() as usize],
        "Button"
    );
    assert_eq!(
        artifact.strings[handler[3].as_int().unwrap() as usize],
        "Tapped"
    );
    assert_eq!(
        artifact.strings[handler[5].as_int().unwrap() as usize],
        "Button.Tapped"
    );
    assert_eq!(handler[6].as_int(), Some(1), "the action slot");
    let owner = handler[7].as_list().expect("owner reference");
    assert_eq!(
        artifact.strings[owner[1].as_int().unwrap() as usize],
        "Counter"
    );
    let count = node_of_kind(&artifact, kinds::node::SLOT);
    assert_eq!(count[1].as_int(), Some(0));
    assert_eq!(
        artifact.strings[count[2].as_int().unwrap() as usize],
        "count"
    );
}

#[test]
fn a_handler_bound_outside_a_component_body_has_no_owner() {
    let artifact = entry_artifact(&artifact_from_source(SEARCH_SOURCE));
    let text = explain(&artifact);
    assert_line(
        &text,
        "    onSearchSubmitted=handler SearchBox.SearchSubmitted action@0:SearchBox.SearchSubmitted =>",
    );
    assert_line(&text, "      <DoSearch search=action.searchString />");
    let handler = node_of_kind(&artifact, kinds::node::ACTION_HANDLER);
    // `root` has no parameters, so `action` is the frame's first slot.
    assert_eq!(handler[6].as_int(), Some(0));
    assert_eq!(handler[7].as_list(), Some(&[][..]), "no owner");
    let action = node_of_kind(&artifact, kinds::node::SLOT);
    assert_eq!(action[1].as_int(), Some(0));
    assert_eq!(
        artifact.strings[action[2].as_int().unwrap() as usize],
        "action"
    );
}

#[test]
fn a_program_without_handlers_lists_no_handler_feature() {
    let artifact = entry_artifact(&artifact_from_source(
        "external component <Button label:string emits { Tapped { } } />\nlet root() = { <Button label=\"x\" /> }",
    ));
    assert!(
        artifact.required_features.is_empty(),
        "{:?}",
        artifact.required_features
    );
}

/// A handler of the shape `component_action_handler_bindings_fail_before_emission` in `tests.rs`
/// refuses for the JavaScript target emits an image here.
#[test]
fn a_handler_emits_an_image_that_reads_back() {
    let artifact = artifact_from_source(SEARCH_SOURCE);
    let bytes = image_bytes(&artifact);
    let image = NxIrImage::open(&bytes).expect("a valid image");
    assert_eq!(
        image.required_features().collect::<Vec<_>>(),
        [NX_IR_REQUIRED_FEATURE_ACTION_HANDLERS_V1]
    );
    assert_eq!(read_back(&bytes), entry_artifact(&artifact));
    assert_eq!(
        explain_nx_ir_image(&bytes).expect("explain"),
        explain(&entry_artifact(&artifact))
    );
}

// ------------------------------------------------------------------------------------------------
// Function types and function values
// ------------------------------------------------------------------------------------------------

const TEMPLATE_LIST: &str =
    "external component <List ItemTemplate:(<function Item:object Index:int />: string)? />\n";

#[test]
fn a_function_typed_prop_is_a_function_type_in_nx_spelling() {
    assert_eq!(kinds::ty::FUNCTION, 4);
    assert_eq!(
        kinds::name(kinds::ty::NAMES, kinds::ty::FUNCTION),
        Some("function")
    );
    let text = explain_source(&format!("{TEMPLATE_LIST}let root() = {{ 1 }}"));
    assert_contains(
        &text,
        "ItemTemplate: (<function Item:object Index:int />: string)?",
    );
    assert!(!text.contains("=>"), "{text}");

    // One function type, one spelling: the explained artifact reads a type table and the checker
    // reads a `Type`, and both assemble the text through `nx_hir::ast::spell_function_type`.
    let checked = nx_types::Type::nullable(nx_types::Type::function(
        vec![
            nx_types::FunctionParam::new("Item", nx_types::Type::named("object")),
            nx_types::FunctionParam::new("Index", nx_types::Type::int()),
        ],
        nx_types::Type::string(),
    ));
    assert_contains(&text, &format!("ItemTemplate: {checked}"));
}

#[test]
fn two_identical_function_types_share_one_table_entry() {
    let artifact = artifact_from_source(
        "external component <List RowTemplate:<function Item:object />: string HeaderTemplate:<function Item:object />: string />\n\
         let root() = { 1 }",
    );
    let model = entry_artifact(&artifact);
    let function_types = model
        .types
        .iter()
        .filter(|entry| {
            entry
                .as_list()
                .and_then(|entry| entry.first())
                .and_then(|k| k.as_int())
                == Some(kinds::ty::FUNCTION)
        })
        .count();
    assert_eq!(function_types, 1, "{:?}", model.types);
    assert_eq!(read_back(&image_bytes(&artifact)), model);
}

#[test]
fn a_type_parameter_inside_a_function_type_is_erased() {
    let text = explain_source(
        "external component <SkiaLayout TItem:type ItemTemplate:(<function Item:TItem Index:int />: object)? />\n\
         let root() = { 1 }",
    );
    assert_contains(
        &text,
        "ItemTemplate: (<function Item:object Index:int />: object)?",
    );
    assert!(!text.contains("TItem"), "{text}");
}

#[test]
fn a_function_bound_to_a_prop_is_a_reference_and_needs_the_feature() {
    let source = "let <Row Item:object />: string = \"r\"\n\
         external component <List ItemTemplate:(<function Item:object />: string)? />\n\
         let root() = <List ItemTemplate={Row} />"
        .to_string();
    let model = entry_artifact(&artifact_from_source(&source));
    let text = explain(&model);
    // A same-module reference renders unqualified, as every reference does.
    assert_contains(&text, "<List ItemTemplate=Row />");
    assert!(
        model
            .required_features
            .contains(&NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1.to_string()),
        "{:?}",
        model.required_features
    );
}

#[test]
fn a_call_of_a_function_typed_binding_is_a_named_call() {
    assert_eq!(kinds::node::NAMED_CALL, 21);
    assert_eq!(
        kinds::name(kinds::node::NAMES, kinds::node::NAMED_CALL),
        Some("namedCall")
    );
    let artifact = artifact_from_source(
        "component <Section Row:<function Item:object Index:int />: string /> = { <Row Item=\"a\" Index=1 /> }\n\
         let root() = { 1 }",
    );
    let model = entry_artifact(&artifact);
    let text = explain(&model);
    assert_contains(&text, "<Row Index=1 Item=\"a\" />");
    let named_calls = model
        .nodes
        .iter()
        .filter(|entry| {
            entry
                .as_list()
                .and_then(|entry| entry.first())
                .and_then(|k| k.as_int())
                == Some(kinds::node::NAMED_CALL)
        })
        .count();
    assert_eq!(named_calls, 1, "{text}");
    assert!(model
        .required_features
        .contains(&NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1.to_string()));
    assert_eq!(read_back(&image_bytes(&artifact)), model);
}

#[test]
fn a_call_of_a_top_level_let_of_function_type_is_a_named_call_on_a_reference() {
    // The callee is a declaration, not a lexical binding, so it emits the same reference node a
    // bare identifier would.
    let artifact = artifact_from_source(
        "external component <Box Label:string? />\n\
         let <Wrap Item:object />: string = \"w\"\n\
         let F: <function Item:object />: string = {Wrap}\n\
         let root() = <Box Label=<F Item=\"x\" /> />",
    );
    let model = entry_artifact(&artifact);
    let text = explain(&model);
    assert_contains(&text, "<F Item=\"x\" />");
    let named_calls = model
        .nodes
        .iter()
        .filter(|entry| {
            entry
                .as_list()
                .and_then(|entry| entry.first())
                .and_then(|k| k.as_int())
                == Some(kinds::node::NAMED_CALL)
        })
        .count();
    assert_eq!(named_calls, 1, "{text}");
    assert_eq!(read_back(&image_bytes(&artifact)), model);
}

#[test]
fn a_program_without_function_values_lists_no_new_feature() {
    // Functions declared and called, one of them by element, but never named as a value.
    let model = entry_artifact(&artifact_from_source(
        "let <Row Item:object />: string = \"r\"\nlet double(n:int): int = {n * 2}\n\
         let root() = { <Row Item=1 /> double(2) }",
    ));
    assert!(
        !model
            .required_features
            .iter()
            .any(|feature| feature == NX_IR_REQUIRED_FEATURE_FUNCTION_VALUES_V1),
        "{:?}",
        model.required_features
    );
    assert!(
        model.required_features.is_empty(),
        "{:?}",
        model.required_features
    );
}
