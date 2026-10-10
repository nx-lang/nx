//! Origins: where the records of a call's value were constructed. The images are the corpus's,
//! with their debug sections, since a span comes from there; the corpus test holds every report
//! of every program to what `origins.json` records.

mod common;

use common::{corpus_root, link, load_corpus, prepare_all, CorpusProgram};
use nx_ir_runtime::{ComponentInit, OriginEntry, Origins, Program, RuntimeOptions};
use nx_value::NxValue;
use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

fn corpus_program(name: &str) -> CorpusProgram {
    load_corpus()
        .into_iter()
        .find(|program| program.name == name)
        .unwrap_or_else(|| panic!("the corpus has no program {name}"))
}

/// A corpus program's entry linked against its other modules, from the images with their debug
/// sections, or without them when `stripped`.
fn linked(name: &str, entry: &str, stripped: bool) -> Program {
    let program = corpus_program(name);
    let images = match stripped {
        true => &program.stripped_images,
        false => &program.images,
    };
    link(&prepare_all(images), entry)
}

/// Options that give a call a fresh origins report, and the report.
fn reporting() -> (RuntimeOptions, Arc<Origins>) {
    let origins = Arc::new(Origins::new());
    let options = RuntimeOptions {
        origins: Some(Arc::clone(&origins)),
        ..RuntimeOptions::default()
    };
    (options, origins)
}

/// The source text an origin entry's span covers, read from the corpus program's module.
fn origin_text(program: &str, entry: &OriginEntry) -> String {
    let source = fs::read(corpus_root().join(program).join(&entry.source.identity))
        .expect("the module's source");
    String::from_utf8(source[entry.source.start as usize..entry.source.end as usize].to_vec())
        .expect("a span of whole characters")
}

/// The record a JSON pointer names in `value`.
fn at_pointer<'a>(value: &'a NxValue, pointer: &str) -> &'a NxValue {
    pointer.split('/').skip(1).fold(value, |held, token| {
        let token = token.replace("~1", "/").replace("~0", "~");
        match held {
            NxValue::Array(items) => &items[token.parse::<usize>().expect("an index")],
            NxValue::Record { properties, .. } => &properties[&token],
            other => panic!("{pointer} goes through {other:?}"),
        }
    })
}

fn type_name(value: &NxValue) -> Option<&str> {
    match value {
        NxValue::Record { type_name, .. } => type_name.as_deref(),
        _ => None,
    }
}

#[test]
fn an_origins_report_names_the_element_that_constructed_each_record_of_the_rendered_output() {
    let program = linked("components", "main.nx", false);
    let (options, origins) = reporting();
    let props = BTreeMap::from([("name".to_string(), NxValue::String("Ada".to_string()))]);
    let initialized = program
        .initialize_component("Greeting", &props, &ComponentInit::default(), &options)
        .expect("initializes");
    let entries = origins.entries();
    let reported: Vec<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.path.as_str(),
                entry.source.identity.as_str(),
                origin_text("components", entry),
            )
        })
        .collect();
    assert_eq!(
        reported,
        [
            (
                "",
                "main.nx",
                "<Stack>\n    <Label Text={\"Hello \" + name} />\n    <Label Text=\"!\" FontSize={count + 20} />\n  </Stack>"
                    .to_string()
            ),
            (
                "/Children/0",
                "main.nx",
                "<Label Text={\"Hello \" + name} />".to_string()
            ),
            (
                "/Children/1",
                "main.nx",
                "<Label Text=\"!\" FontSize={count + 20} />".to_string()
            ),
        ]
    );
    // Each pointer names a record of the value the host received.
    let types: Vec<_> = entries
        .iter()
        .map(|entry| type_name(at_pointer(&initialized.rendered, &entry.path)))
        .collect();
    assert_eq!(types, [Some("Stack"), Some("Label"), Some("Label")]);
}

#[test]
fn each_record_a_loop_builds_has_the_one_element_of_the_loop_body_as_its_origin() {
    let (options, origins) = reporting();
    linked("components", "main.nx", false)
        .evaluate_function("labels", &[], &options)
        .expect("evaluates");
    let reported: Vec<_> = origins
        .entries()
        .iter()
        .map(|entry| (entry.path.clone(), origin_text("components", entry)))
        .collect();
    assert_eq!(
        reported,
        [
            ("/0".to_string(), "<Label Text={text} />".to_string()),
            ("/1".to_string(), "<Label Text={text} />".to_string()),
        ]
    );
}

#[test]
fn a_question_shown_through_a_step_has_the_origin_of_its_element_in_the_value_that_declares_it() {
    let corpus = corpus_program("question-flow");
    let program = link(&prepare_all(&corpus.images), "main.nx");
    let lifecycle = &corpus.lifecycles[0];
    let source = fs::read_to_string(corpus_root().join("question-flow").join("main.nx"))
        .expect("the source");
    let declared = source.find("let roleQuestion =").expect("the declaration") as u32;
    let (options, origins) = reporting();
    let mut instance = program
        .initialize_component(
            &lifecycle.component,
            &lifecycle.props,
            &ComponentInit::default(),
            &options,
        )
        .expect("initializes")
        .instance;
    let mut shown = 0;
    for batch in &lifecycle.batches {
        let dispatched = program
            .dispatch_component_actions(&instance, batch, &options)
            .expect("dispatches");
        for entry in origins.entries() {
            let NxValue::Record {
                type_name: Some(name),
                properties,
            } = at_pointer(&dispatched.rendered, &entry.path)
            else {
                panic!("{} names no record", entry.path);
            };
            if name == "SingleChoice"
                && properties.get("id") == Some(&NxValue::String("role".to_string()))
            {
                shown += 1;
                let text = origin_text("question-flow", &entry);
                assert!(entry.source.start > declared, "{text}");
                assert!(text.starts_with("<SingleChoice id=\"role\""), "{text}");
                assert!(text.ends_with("</SingleChoice>"), "{text}");
            }
        }
        instance = dispatched.instance;
    }
    assert!(shown > 0, "no batch showed the role question");
}

#[test]
fn a_record_a_library_function_constructs_has_its_origin_in_the_library_module() {
    let (options, origins) = reporting();
    linked("ranges", "app/main.nx", false)
        .evaluate_function("passedAlong", &[], &options)
        .expect("evaluates");
    let reported: Vec<_> = origins
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.path.clone(),
                entry.source.identity.clone(),
                origin_text("ranges", entry),
            )
        })
        .collect();
    assert_eq!(
        reported,
        [(
            String::new(),
            "shared/bounds.nx".to_string(),
            "0.0..1.5".to_string()
        )]
    );
}

#[test]
fn a_record_apply_builds_has_the_origin_of_the_expression_that_applied_it() {
    let (options, origins) = reporting();
    linked("records", "main.nx", false)
        .evaluate_function("applied", &[], &options)
        .expect("evaluates");
    let reported: Vec<_> = origins
        .entries()
        .iter()
        .map(|entry| (entry.path.clone(), origin_text("records", entry)))
        .collect();
    assert_eq!(
        reported,
        [(String::new(), "apply(ada(), patch())".to_string())]
    );
}

#[test]
fn an_origins_report_from_images_without_their_debug_section_is_empty() {
    let (options, origins) = reporting();
    let reported = linked("components", "main.nx", true)
        .evaluate_function("root", &[], &options)
        .expect("evaluates");
    assert_eq!(origins.entries(), []);
    let plain = linked("components", "main.nx", false)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .expect("evaluates");
    assert_eq!(reported, plain);
}

/// The entries of the last call, each with the source text its span covers in `held-records`.
fn held_entries(origins: &Origins) -> Vec<(String, String)> {
    origins
        .entries()
        .iter()
        .map(|entry| (entry.path.clone(), origin_text("held-records", entry)))
        .collect()
}

/// The source text of the origin of the rendered `Panel`'s item, if it has one.
fn panel_item(origins: &Origins) -> Option<String> {
    held_entries(origins)
        .into_iter()
        .find(|(path, _)| path == "/children/0/item")
        .map(|(_, text)| text)
}

/// A batch that taps the button whose handler has `token`.
fn tap(token: &str) -> Vec<NxValue> {
    vec![invocation(
        token,
        NxValue::Record {
            type_name: Some("Button.Tapped".to_string()),
            properties: BTreeMap::new(),
        },
    )]
}

fn invocation(token: &str, action: NxValue) -> NxValue {
    NxValue::Record {
        type_name: Some("ActionHandlerInvocation".to_string()),
        properties: BTreeMap::from([
            ("token".to_string(), NxValue::String(token.to_string())),
            ("action".to_string(), action),
        ]),
    }
}

#[test]
fn a_record_held_in_state_keeps_its_origin_through_a_dispatch_given_no_report() {
    let program = linked("held-records", "main.nx", false);
    let (options, origins) = reporting();
    let initialized = program
        .initialize_component(
            "Holder",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options,
        )
        .expect("initializes");
    assert_eq!(
        panel_item(&origins).as_deref(),
        Some("<Item name=\"first\" />")
    );
    // A dispatch that asks for no report re-normalizes the state, and builds nothing.
    let unreported = program
        .dispatch_component_actions(
            &initialized.instance,
            &tap("h1-1"),
            &RuntimeOptions::default(),
        )
        .expect("dispatches");
    program
        .dispatch_component_actions(&unreported.instance, &tap("h2-1"), &options)
        .expect("dispatches");
    assert_eq!(
        panel_item(&origins).as_deref(),
        Some("<Item name=\"first\" />")
    );
}

#[test]
fn a_record_the_host_passes_back_in_a_dispatch_has_no_origin() {
    let program = linked("held-records", "main.nx", false);
    let (options, origins) = reporting();
    let initialized = program
        .initialize_component(
            "Holder",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options,
        )
        .expect("initializes");
    // The state the runtime returned holds a record the program built.
    let spare = initialized.state["spare"].clone();
    let chosen = NxValue::Record {
        type_name: Some("Chooser.Chosen".to_string()),
        properties: BTreeMap::from([("item".to_string(), spare)]),
    };
    program
        .dispatch_component_actions(
            &initialized.instance,
            &[invocation("h1-4", chosen)],
            &options,
        )
        .expect("dispatches");
    assert_eq!(panel_item(&origins), None);
    assert_eq!(held_entries(&origins).len(), 6);
}

#[test]
fn a_function_value_the_host_passes_back_from_the_state_is_read_as_it_is() {
    let program = linked("held-records", "main.nx", false);
    for (initializing, dispatching) in [(false, false), (true, false), (false, true), (true, true)]
    {
        let options = |reports: bool| match reports {
            true => reporting().0,
            false => RuntimeOptions::default(),
        };
        let initialized = program
            .initialize_component(
                "Tool",
                &BTreeMap::new(),
                &ComponentInit::default(),
                &options(initializing),
            )
            .expect("initializes");
        let picked = NxValue::Record {
            type_name: Some("Picker.Picked".to_string()),
            properties: BTreeMap::from([("op".to_string(), initialized.state["spare"].clone())]),
        };
        let dispatched = program
            .dispatch_component_actions(
                &initialized.instance,
                &[invocation("h1-1", picked)],
                &options(dispatching),
            )
            .expect("dispatches");
        let value = at_pointer(&dispatched.rendered, "/children/0/value");
        assert!(
            matches!(value, NxValue::Int32(6) | NxValue::Int(6)),
            "{value:?}"
        );
    }
}

#[test]
fn call_function_reports_the_origins_of_its_value() {
    let (options, origins) = reporting();
    let function = NxValue::Record {
        type_name: Some("Function".to_string()),
        properties: BTreeMap::from([
            ("module".to_string(), NxValue::String("main.nx".to_string())),
            ("name".to_string(), NxValue::String("picked".to_string())),
        ]),
    };
    linked("held-records", "main.nx", false)
        .call_function(&function, &BTreeMap::new(), &options)
        .expect("calls");
    assert_eq!(
        held_entries(&origins),
        [(String::new(), "<Item name=\"picked\" />".to_string())]
    );
}

#[test]
fn evaluate_component_reports_the_origins_of_its_rendered_output_and_none_for_the_state_given() {
    let (options, origins) = reporting();
    let item = |name: &str| NxValue::Record {
        type_name: Some("Item".to_string()),
        properties: BTreeMap::from([("name".to_string(), NxValue::String(name.to_string()))]),
    };
    let state = BTreeMap::from([
        ("held".to_string(), item("given")),
        ("spare".to_string(), item("other")),
        ("count".to_string(), NxValue::Int32(3)),
    ]);
    linked("held-records", "main.nx", false)
        .evaluate_component("Holder", &BTreeMap::new(), &state, &options)
        .expect("evaluates");
    let paths: Vec<_> = held_entries(&origins)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            "",
            "/children/0",
            "/children/1",
            "/children/2",
            "/children/3",
            "/children/4"
        ]
    );
}

#[test]
fn a_restored_instance_holds_no_origins() {
    let program = linked("held-records", "main.nx", false);
    let (options, origins) = reporting();
    let initialized = program
        .initialize_component(
            "Holder",
            &BTreeMap::new(),
            &ComponentInit::default(),
            &options,
        )
        .expect("initializes");
    let stored = serde_json::to_value(&initialized.instance).expect("an instance serializes");
    let restored = program
        .restore_component_instance(stored)
        .expect("restores");
    program
        .dispatch_component_actions(&restored, &tap("h1-1"), &options)
        .expect("dispatches");
    assert_eq!(panel_item(&origins), None);
    // The instance itself keeps them.
    program
        .dispatch_component_actions(&initialized.instance, &tap("h1-1"), &options)
        .expect("dispatches");
    assert_eq!(
        panel_item(&origins).as_deref(),
        Some("<Item name=\"first\" />")
    );
}

#[test]
fn a_call_that_fails_and_a_call_that_renders_nothing_leave_the_origins_report_empty() {
    let program = linked("components", "main.nx", false);
    let (options, origins) = reporting();
    program
        .evaluate_function("root", &[], &options)
        .expect("evaluates");
    assert_eq!(origins.entries().len(), 3);
    let error = program
        .evaluate_function("missing", &[], &options)
        .expect_err("fails");
    assert_eq!(error.diagnostics[0].code, "nx-ir-missing-entrypoint");
    assert_eq!(origins.entries(), []);
    program
        .evaluate_function("root", &[], &options)
        .expect("evaluates");
    let state = BTreeMap::from([("count".to_string(), NxValue::Int32(1))]);
    program
        .normalize_component_state("Greeting", &state, &options)
        .expect("normalizes");
    assert_eq!(origins.entries(), []);
}
