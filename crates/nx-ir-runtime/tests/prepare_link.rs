//! Preparation and linking, over corpus images and images altered through the format crate.

mod common;

use common::{canonical_eq, load_corpus, prepare_all, CorpusProgram};
use nx_ir::{kinds, write_nx_ir_image, IrItem, NxIrArtifact, NxIrImage, NxIrModuleEntry};
use nx_ir_runtime::{
    LinkOptions, NxIrRuntimeError, PreparedModule, Program, RuntimeOptions, NX_IR_RUNTIME_ABI,
    NX_IR_SCHEMA_VERSION, NX_PRELUDE_MODULE_IDENTITY,
};
use nx_value::NxValue;
use std::collections::BTreeMap;

fn corpus_program(name: &str) -> CorpusProgram {
    load_corpus()
        .into_iter()
        .find(|program| program.name == name)
        .unwrap_or_else(|| panic!("the corpus has no program '{name}'"))
}

fn image(program: &CorpusProgram, identity: &str) -> Vec<u8> {
    program
        .stripped_images
        .iter()
        .find(|(candidate, _)| candidate == identity)
        .map(|(_, bytes)| bytes.clone())
        .unwrap_or_else(|| panic!("{} emits no image for {identity}", program.name))
}

fn artifact(bytes: &[u8]) -> NxIrArtifact {
    NxIrImage::open(bytes).expect("a valid image").to_artifact()
}

/// Writes an artifact, first adding to its string table any string its header names: the writer
/// finds header strings in the table rather than adding them.
fn write(mut artifact: NxIrArtifact) -> Vec<u8> {
    let named: Vec<String> = std::iter::once(artifact.runtime_abi.clone())
        .chain(artifact.required_features.iter().cloned())
        .chain(
            artifact
                .modules
                .iter()
                .flat_map(|module| [module.identity.clone(), module.version.clone()]),
        )
        .collect();
    for text in named {
        if !artifact.strings.contains(&text) {
            artifact.strings.push(text);
        }
    }
    write_nx_ir_image(&artifact).expect("an image")
}

fn rewritten(bytes: &[u8], change: impl FnOnce(&mut NxIrArtifact)) -> Vec<u8> {
    let mut artifact = artifact(bytes);
    change(&mut artifact);
    write(artifact)
}

fn error_of<T>(result: Result<T, NxIrRuntimeError>) -> NxIrRuntimeError {
    match result {
        Ok(_) => panic!("expected a failure"),
        Err(error) => error,
    }
}

fn codes(error: &NxIrRuntimeError) -> Vec<&'static str> {
    error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn no_modules(_: &str) -> Option<PreparedModule> {
    None
}

/// An artifact with no tables: one module, the declarations and nodes a test adds.
fn bare_artifact() -> NxIrArtifact {
    NxIrArtifact {
        schema_version: NX_IR_SCHEMA_VERSION,
        runtime_abi: NX_IR_RUNTIME_ABI.to_string(),
        required_features: Vec::new(),
        modules: vec![NxIrModuleEntry {
            identity: "main.nx".to_string(),
            version: String::new(),
            fingerprint: 7,
        }],
        function_entrypoints: Vec::new(),
        component_entrypoints: Vec::new(),
        strings: Vec::new(),
        types: Vec::new(),
        constants: Vec::new(),
        nodes: Vec::new(),
        declarations: Vec::new(),
        debug: None,
    }
}

#[test]
fn a_valid_image_prepares_and_lists_its_entrypoints() {
    let program = corpus_program("two-module");
    let module = PreparedModule::prepare(image(&program, "app/main.nx")).expect("prepares");

    assert_eq!(module.identity(), "app/main.nx");
    assert_eq!(
        module.referenced_modules().collect::<Vec<_>>(),
        ["shared/model.nx"]
    );
    let functions: Vec<&str> = module.function_entrypoints().collect();
    for name in ["total", "user", "card", "local", "root"] {
        assert!(functions.contains(&name), "{functions:?} lacks {name}");
    }
    assert!(module.component_entrypoints().any(|name| name == "Panel"));
    assert_eq!(module.image().schema_version(), NX_IR_SCHEMA_VERSION);
}

#[test]
fn an_unsupported_schema_abi_or_feature_is_refused_by_name() {
    let program = corpus_program("expressions");
    let bytes = image(&program, "main.nx");

    let mut other_schema = bytes.clone();
    other_schema[4..8].copy_from_slice(&99u32.to_le_bytes());
    let error = error_of(PreparedModule::prepare(other_schema));
    assert_eq!(codes(&error), ["nx-ir-schema-version"]);
    assert!(error.to_string().contains("99"), "{error}");

    let error = error_of(PreparedModule::prepare(rewritten(&bytes, |artifact| {
        artifact.runtime_abi = "nx-ir-runtime-v9".to_string();
    })));
    assert_eq!(codes(&error), ["nx-ir-runtime-abi"]);
    assert!(error.to_string().contains("nx-ir-runtime-v9"), "{error}");

    let error = error_of(PreparedModule::prepare(rewritten(&bytes, |artifact| {
        artifact
            .required_features
            .push("time-travel-v1".to_string());
    })));
    assert_eq!(codes(&error), ["nx-ir-required-feature"]);
    assert!(error.to_string().contains("time-travel-v1"), "{error}");

    let error = error_of(PreparedModule::prepare(b"not an image".to_vec()));
    assert_eq!(codes(&error), ["nx-ir-format"]);
}

/// A module whose type table holds a function reference type lists
/// `function-reference-type-v1`, and prepares; the type's entry is read with its one operand.
#[test]
fn a_module_that_lists_the_function_reference_type_feature_prepares() {
    let program = corpus_program("function-references");
    let bytes = image(&program, "main.nx");
    let features = artifact(&bytes).required_features;
    assert!(
        features.contains(&"function-reference-type-v1".to_string()),
        "{features:?}"
    );
    PreparedModule::prepare(bytes.clone()).expect("the module prepares");

    // The feature is a name the runtime knows, not one it lets through: another is refused.
    let error = error_of(PreparedModule::prepare(rewritten(&bytes, |artifact| {
        artifact
            .required_features
            .push("function-reference-type-v9".to_string());
    })));
    assert_eq!(codes(&error), ["nx-ir-required-feature"]);
    assert!(
        error.to_string().contains("function-reference-type-v9"),
        "{error}"
    );
}

fn function_record(name: &str) -> serde_json::Value {
    serde_json::json!({ "$type": "Function", "module": "main.nx", "name": name })
}

/// A site of a function reference type takes a function of the linked program, of any signature,
/// and nothing else, with the codes the TypeScript runtime reports.
#[test]
fn a_function_reference_site_takes_any_function_of_the_program() {
    let program = corpus_program("function-references");
    let modules = prepare_all(&program.stripped_images);
    let linked = common::link(&modules, "main.nx");
    let options = RuntimeOptions::default();
    let wrap =
        |f: serde_json::Value| linked.evaluate_function("wrap", &[common::value(&f)], &options);

    // A rendered field is the `Function` record, and the default fills `build`.
    let rendered = linked
        .evaluate_function("defaulted", &[], &options)
        .expect("defaulted evaluates");
    assert!(canonical_eq(
        &rendered,
        &common::value(&serde_json::json!({
            "$type": "Tool",
            "fn": function_record("double"),
            "build": function_record("makeArgs"),
        }))
    ));

    // A host record naming a function of any signature is accepted, and so is one whose result
    // is not the site's: no part of a signature is re-checked at the boundary.
    for name in ["double", "greet", "Row", "names", "makeArgs"] {
        let tool = wrap(function_record(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            canonical_eq(
                &tool,
                &common::value(&serde_json::json!({
                    "$type": "Tool",
                    "fn": function_record(name),
                    "build": function_record("makeArgs"),
                }))
            ),
            "{name}"
        );
    }

    let error = error_of(wrap(function_record("Nope")));
    assert_eq!(codes(&error), ["nx-ir-function-value"]);
    assert!(error.to_string().contains("Nope"), "{error}");

    let error = error_of(wrap(serde_json::json!({
        "$type": "Function", "module": "other.nx", "name": "double"
    })));
    assert_eq!(codes(&error), ["nx-ir-function-value"]);
    assert!(error.to_string().contains("other.nx"), "{error}");

    // `Tool` is a record of the module, not a function.
    let error = error_of(wrap(function_record("Tool")));
    assert_eq!(codes(&error), ["nx-ir-function-value"]);

    for not_a_function in [
        serde_json::json!("double"),
        serde_json::json!(1),
        serde_json::json!({ "$type": "Tool" }),
        serde_json::json!({ "$type": "Function", "name": "double" }),
    ] {
        let error = error_of(wrap(not_a_function.clone()));
        assert_eq!(codes(&error), ["nx-ir-boundary-type"], "{not_a_function}");
        assert!(
            error.to_string().contains("to be a function value"),
            "{error}"
        );
    }
}

/// The `rust-ir-runtime` scenario "A host-supplied record is validated against the program": a
/// host supplies a whole `Tool`, and the refusal names the field `fn`.
#[test]
fn a_host_supplied_record_with_a_function_reference_field_is_validated() {
    let program = corpus_program("function-references");
    let modules = prepare_all(&program.stripped_images);
    let linked = common::link(&modules, "main.nx");
    let options = RuntimeOptions::default();
    let pass = |function: serde_json::Value| {
        let tool = serde_json::json!({ "$type": "Tool", "fn": function });
        linked.evaluate_function("passTool", &[common::value(&tool)], &options)
    };

    let accepted = pass(function_record("double")).expect("a record naming double is accepted");
    assert!(canonical_eq(
        &accepted,
        &common::value(&serde_json::json!({
            "$type": "Tool",
            "fn": function_record("double"),
            "build": function_record("makeArgs"),
        }))
    ));

    let error = error_of(pass(function_record("Nope")));
    assert_eq!(codes(&error), ["nx-ir-function-value"]);
    assert!(error.to_string().contains("Nope"), "{error}");

    let error = error_of(pass(serde_json::json!("double")));
    assert_eq!(codes(&error), ["nx-ir-boundary-type"]);
    assert!(
        error.to_string().contains("fn to be a function value"),
        "{error}"
    );
}

#[test]
fn a_presence_operator_without_its_feature_is_malformed() {
    let program = corpus_program("occurrences");
    let bytes = image(&program, "main.nx");
    assert!(PreparedModule::prepare(bytes.clone()).is_ok());

    let error = error_of(PreparedModule::prepare(rewritten(&bytes, |artifact| {
        artifact
            .required_features
            .retain(|feature| feature != "occurrence-v1");
    })));
    assert!(
        codes(&error).iter().all(|code| *code == "nx-ir-malformed"),
        "{error}"
    );
    assert!(error.to_string().contains("occurrence-v1"), "{error}");

    let program = corpus_program("occurrence-patterns");
    let error = error_of(PreparedModule::prepare(rewritten(
        &image(&program, "main.nx"),
        |artifact| {
            artifact
                .required_features
                .retain(|feature| feature != "occurrence-v1");
        },
    )));
    assert!(error.to_string().contains("'{}' pattern"), "{error}");
}

#[test]
fn an_unlinked_module_is_refused_and_a_self_contained_one_is_a_program() {
    let program = corpus_program("two-module");
    let entry = PreparedModule::prepare(image(&program, "app/main.nx")).expect("prepares");
    assert_eq!(codes(&error_of(entry.program())), ["nx-ir-unlinked"]);

    let shared = PreparedModule::prepare(image(&program, "shared/model.nx")).expect("prepares");
    let value = shared
        .program()
        .expect("a self-contained module")
        .evaluate_function("answer", &[], &RuntimeOptions::default())
        .expect("evaluates");
    assert!(canonical_eq(
        &value,
        &common::value(&program.results["shared/model.nx::answer"])
    ));

    let value = Program::prepare(image(&program, "shared/model.nx"))
        .expect("prepares and links")
        .evaluate_function("answer", &[], &RuntimeOptions::default())
        .expect("evaluates");
    assert!(canonical_eq(
        &value,
        &common::value(&program.results["shared/model.nx::answer"])
    ));
}

#[test]
fn linking_reports_a_missing_module_a_version_mismatch_and_a_missing_declaration() {
    let program = corpus_program("two-module");
    let entry = PreparedModule::prepare(image(&program, "app/main.nx")).expect("prepares");
    let shared_bytes = image(&program, "shared/model.nx");

    let error = error_of(Program::link(&entry, no_modules, &LinkOptions::default()));
    assert_eq!(codes(&error), ["nx-ir-link-missing-module"]);
    assert!(error.to_string().contains("shared/model.nx"), "{error}");

    let recorded = artifact(&image(&program, "app/main.nx")).modules[1]
        .version
        .clone();
    let newer = PreparedModule::prepare(rewritten(&shared_bytes, |artifact| {
        artifact.modules[0].version = "next".to_string();
    }))
    .expect("prepares");
    let error = error_of(Program::link(
        &entry,
        |_| Some(newer.clone()),
        &LinkOptions::default(),
    ));
    assert_eq!(codes(&error), ["nx-ir-link-version"]);
    let message = error.to_string();
    assert!(
        message.contains("shared/model.nx")
            && message.contains(&format!("'{recorded}'"))
            && message.contains("'next'"),
        "{message}"
    );
    let linked = Program::link(
        &entry,
        |_| Some(newer.clone()),
        &LinkOptions {
            allow_version_mismatch: true,
        },
    )
    .expect("links across versions");
    assert!(linked
        .evaluate_function("total", &[], &RuntimeOptions::default())
        .is_ok());

    // The shared module with one declaration the entry references renamed away.
    let referenced = artifact(&image(&program, "app/main.nx"));
    let wanted = referenced
        .nodes
        .iter()
        .filter_map(IrItem::as_list)
        .find(|node| {
            node[0].as_int() == Some(kinds::node::REFERENCE) && node[1].as_int() == Some(1)
        })
        .map(|node| referenced.strings[node[2].as_int().unwrap() as usize].clone())
        .expect("a reference into the shared module");
    let lacking = PreparedModule::prepare(rewritten(&shared_bytes, |artifact| {
        let index = artifact
            .strings
            .iter()
            .position(|text| *text == wanted)
            .expect("the name");
        artifact.strings[index] = format!("{wanted}Renamed");
    }))
    .expect("prepares");
    let error = error_of(Program::link(
        &entry,
        |_| Some(lacking.clone()),
        &LinkOptions::default(),
    ));
    assert!(
        codes(&error).contains(&"nx-ir-link-missing-declaration"),
        "{error}"
    );
    assert!(error.to_string().contains(&wanted), "{error}");

    let impostor = PreparedModule::prepare(image(&program, "app/main.nx")).expect("prepares");
    let error = error_of(Program::link(
        &entry,
        |_| Some(impostor.clone()),
        &LinkOptions::default(),
    ));
    assert!(codes(&error).contains(&"nx-ir-link-identity"), "{error}");
}

#[test]
fn the_prelude_is_supplied_unless_the_host_supplies_one() {
    let program = corpus_program("ranges");
    let (identity, bytes) = program
        .stripped_images
        .iter()
        .find(|(_, bytes)| {
            artifact(bytes)
                .modules
                .iter()
                .any(|module| module.identity == NX_PRELUDE_MODULE_IDENTITY)
        })
        .expect("a module that names the prelude");
    let modules = prepare_all(&program.stripped_images);
    let entry = modules.get(identity).expect("prepared");
    assert!(artifact(bytes).modules.len() >= 2);

    // The program's own modules only: the prelude's slot is filled by the built-in image.
    let linked = Program::link(
        entry,
        |wanted| modules.get(wanted).cloned(),
        &LinkOptions::default(),
    )
    .expect("links");
    assert!(linked
        .modules()
        .any(|module| module == NX_PRELUDE_MODULE_IDENTITY));

    // A host that supplies the prelude gets its own: one of another version is refused by the
    // version check, which the built-in one passes.
    let built_in = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/prelude.nxir"))
        .expect("the prelude image");
    let other = PreparedModule::prepare(rewritten(&built_in, |artifact| {
        artifact.modules[0].version = "host-prelude".to_string();
    }))
    .expect("prepares");
    let error = error_of(Program::link(
        entry,
        |wanted| {
            if wanted == NX_PRELUDE_MODULE_IDENTITY {
                Some(other.clone())
            } else {
                modules.get(wanted).cloned()
            }
        },
        &LinkOptions::default(),
    ));
    assert!(
        codes(&error)
            .iter()
            .all(|code| *code == "nx-ir-link-version"),
        "{error}"
    );
    assert!(error.to_string().contains("host-prelude"), "{error}");
}

#[test]
fn one_prepared_module_serves_programs_on_several_threads() {
    let program = corpus_program("two-module");
    let shared = PreparedModule::prepare(image(&program, "shared/model.nx")).expect("prepares");
    let entry_bytes = image(&program, "app/main.nx");
    let expected = common::value(&program.results["app/main.nx::total"]);

    let results: Vec<NxValue> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let shared = shared.clone();
                let entry_bytes = entry_bytes.clone();
                scope.spawn(move || {
                    let entry = PreparedModule::prepare(entry_bytes).expect("prepares");
                    Program::link(&entry, |_| Some(shared.clone()), &LinkOptions::default())
                        .expect("links")
                        .evaluate_function("total", &[], &RuntimeOptions::default())
                        .expect("evaluates")
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("no panic"))
            .collect()
    });
    assert!(results.iter().all(|value| canonical_eq(value, &expected)));
}

#[test]
fn a_name_that_is_not_an_entrypoint_is_not_callable() {
    let program = corpus_program("expressions");
    let linked = Program::prepare(image(&program, "main.nx")).expect("prepares");
    let error =
        error_of(linked.evaluate_function("noSuchFunction", &[], &RuntimeOptions::default()));
    assert_eq!(codes(&error), ["nx-ir-missing-entrypoint"]);
    let error = error_of(linked.initialize_component(
        "NoSuchComponent",
        &BTreeMap::new(),
        &Default::default(),
        &RuntimeOptions::default(),
    ));
    assert_eq!(codes(&error), ["nx-ir-component"]);
}

/// `let root() = { not not … not true }`, nested `depth` deep.
fn nested_program(depth: usize) -> Vec<u8> {
    let mut artifact = bare_artifact();
    artifact.strings = vec!["root".to_string()];
    artifact.nodes.push(IrItem::ints([kinds::node::BOOL, 1]));
    for index in 0..depth as i64 {
        artifact
            .nodes
            .push(IrItem::ints([kinds::node::UNARY, kinds::unary::NOT, index]));
    }
    artifact.declarations.push(IrItem::list([
        IrItem::Int(kinds::declaration::FUNCTION),
        IrItem::Int(0),
        IrItem::list([]),
        IrItem::Int(depth as i64),
        IrItem::Int(-1),
        IrItem::Int(0),
    ]));
    artifact.function_entrypoints = vec![0];
    write(artifact)
}

#[test]
fn expression_nesting_ends_in_a_diagnostic_not_a_stack_overflow() {
    let options = RuntimeOptions::default();
    let shallow = Program::prepare(nested_program(100)).expect("prepares");
    assert_eq!(
        shallow.evaluate_function("root", &[], &options),
        Ok(NxValue::Bool(true))
    );

    let deep = Program::prepare(nested_program(200_000)).expect("prepares");
    let error = error_of(deep.evaluate_function("root", &[], &options));
    assert_eq!(codes(&error), ["nx-ir-resource-limit"]);
    assert_eq!(
        error.diagnostics[0].declaration.as_deref(),
        Some("main.nx::root")
    );
}

// ------------------------------------------------------------------------------------------------
// The agent library: a standard library's image is supplied by the host like any library module's
// ------------------------------------------------------------------------------------------------

const AGENT_MODULE: &str = "@nx/agent/agent.nx";

fn json(text: &str) -> NxValue {
    NxValue::from_json_str(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn arguments(text: &str) -> BTreeMap<String, NxValue> {
    match json(text) {
        NxValue::Record { properties, .. } => properties,
        other => panic!("expected an object, got {other:?}"),
    }
}

fn at<'a>(value: &'a NxValue, path: &[&str]) -> &'a NxValue {
    path.iter().fold(value, |value, step| match value {
        NxValue::Record { properties, .. } => properties
            .get(*step)
            .unwrap_or_else(|| panic!("no '{step}' in {value:?}")),
        NxValue::Array(items) => &items[step.parse::<usize>().expect("an index")],
        other => panic!("cannot read '{step}' of {other:?}"),
    })
}

#[test]
fn the_agent_example_links_its_library_image_and_its_tools_are_callable() {
    let corpus = corpus_program("agent-library");
    let entry = PreparedModule::prepare(image(&corpus, "main.nx")).expect("prepares");
    let library = PreparedModule::prepare(image(&corpus, AGENT_MODULE)).expect("prepares");
    assert!(artifact(&image(&corpus, AGENT_MODULE))
        .required_features
        .iter()
        .any(|feature| feature == "function-reference-type-v1"));

    let program = Program::link(
        &entry,
        |identity| (identity == AGENT_MODULE).then(|| library.clone()),
        &LinkOptions::default(),
    )
    .expect("links");
    let options = RuntimeOptions::default();
    let agent = program
        .evaluate_function("root", &[], &options)
        .expect("root evaluates");

    assert!(canonical_eq(
        at(&agent, &["documents", "0"]),
        &json(
            r#"{ "$type": "Document", "title": "Refund policy",
                 "text": "Refunds are available within 30 days of purchase." }"#
        )
    ));
    assert!(matches!(
        at(&agent, &["instructions"]),
        NxValue::String(text) if text.starts_with("You are the support assistant for Example.\n")
    ));

    let find_plans = at(&agent, &["tools", "1", "function"]);
    assert!(canonical_eq(
        find_plans,
        &json(r#"{ "$type": "Function", "module": "main.nx", "name": "findPlans" }"#)
    ));
    let plans = program
        .call_function(find_plans, &arguments(r#"{ "teamSize": 4 }"#), &options)
        .expect("findPlans runs");
    assert!(canonical_eq(
        &plans,
        &json(r#"[{ "$type": "Plan", "name": "Team", "seats": 4, "monthlyPrice": 20 }]"#)
    ));

    let lookup = program
        .call_function(
            at(&agent, &["tools", "2", "arguments"]),
            &arguments(r#"{ "orderId": "A-1" }"#),
            &options,
        )
        .expect("lookupOrder runs");
    assert!(canonical_eq(
        &lookup,
        &json(
            r#"{ "$type": "HttpArguments",
                 "pathParams": [{ "$type": "HttpParam", "name": "orderId", "value": "A-1" }] }"#
        )
    ));

    let ticket = program
        .call_function(
            at(&agent, &["tools", "3", "arguments"]),
            &arguments(r#"{ "subject": "Late order" }"#),
            &options,
        )
        .expect("openTicket runs");
    assert!(canonical_eq(
        &ticket,
        &json(
            r#"{ "$type": "HttpArguments",
                 "body": { "$type": "NewTicket", "subject": "Late order", "priority": 2 } }"#
        )
    ));
}

#[test]
fn a_missing_or_mismatched_agent_library_image_fails_the_link() {
    let corpus = corpus_program("agent-library");
    let entry = PreparedModule::prepare(image(&corpus, "main.nx")).expect("prepares");

    // No runtime carries a standard library's image: nothing stands in for the one the host did
    // not supply.
    let error = error_of(Program::link(&entry, no_modules, &LinkOptions::default()));
    assert_eq!(codes(&error), ["nx-ir-link-missing-module"]);
    assert!(error.to_string().contains(AGENT_MODULE), "{error}");

    let other = PreparedModule::prepare(rewritten(&image(&corpus, AGENT_MODULE), |artifact| {
        artifact.modules[0].version = "0000000000000000".to_string();
    }))
    .expect("prepares");
    let error = error_of(Program::link(
        &entry,
        |_| Some(other.clone()),
        &LinkOptions::default(),
    ));
    assert_eq!(codes(&error), ["nx-ir-link-version"]);
    assert!(error.to_string().contains(AGENT_MODULE), "{error}");

    Program::link(
        &entry,
        |_| Some(other.clone()),
        &LinkOptions {
            allow_version_mismatch: true,
        },
    )
    .expect("links across versions when the host allows it")
    .evaluate_function("root", &[], &RuntimeOptions::default())
    .expect("root evaluates");
}

#[test]
fn a_tool_function_takes_the_host_context_by_its_subtype_or_its_base() {
    let corpus = corpus_program("agent-tool-context");
    let modules = prepare_all(&corpus.stripped_images);
    let program = common::link(&modules, "main.nx");
    let options = RuntimeOptions::default();
    let agent = program
        .evaluate_function("root", &[], &options)
        .expect("root evaluates");
    assert_eq!(
        at(&agent, &["model"]),
        &NxValue::String("any-model-name-at-all".into())
    );

    // The host builds the context record with the `$type` of its concrete subtype, declared in a
    // module other than the abstract base.
    let context = r#"{ "context": { "$type": "ChatToolContext", "callId": "call-1",
                                    "conversationId": "conv-7" } }"#;
    let who = at(&agent, &["tools", "0", "function"]);
    assert_eq!(
        program.call_function(who, &arguments(context), &options),
        Ok(NxValue::String("conv-7".into()))
    );
    // A parameter typed by the abstract base accepts the host's subtype.
    let call_id = at(&agent, &["tools", "1", "function"]);
    assert_eq!(
        program.call_function(call_id, &arguments(context), &options),
        Ok(NxValue::String("call-1".into()))
    );
    // A function tool of another signature, with its optional parameter left out.
    let seats = at(&agent, &["tools", "2", "function"]);
    assert!(canonical_eq(
        &program
            .call_function(seats, &arguments(r#"{ "teamSize": 4 }"#), &options)
            .expect("seatCount runs"),
        &NxValue::Int(4)
    ));
}

#[test]
fn the_agent_library_image_is_the_same_from_either_corpus_program() {
    assert_eq!(
        image(&corpus_program("agent-library"), AGENT_MODULE),
        image(&corpus_program("agent-tool-context"), AGENT_MODULE)
    );
}
