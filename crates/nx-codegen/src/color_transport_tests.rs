//! Color crosses host boundaries as ordinary typed numerical records.
use nx_api::{build_program_artifact_from_source, ProgramBuildContext};
use nx_ir_runtime::RuntimeOptions;
use nx_value::NxValue;

const SOURCE: &str = r##"import "@nx/color" as c
export type Brand = { accent:c.Color }
let root() = <Brand accent=#1234/>
let echo(brand:Brand):Brand = { brand }
let validateBrand(brand:Brand):c.Color = { c.validate(brand.accent) }
"##;

#[test]
fn all_named_color_constants_match_css_parser_channels_exactly() {
    let library = nx_api::standard_library_entry("@nx/color").unwrap();
    let source = library
        .modules
        .iter()
        .find(|m| m.identity == "color.nx")
        .unwrap()
        .source;
    let names: Vec<_> = source
        .lines()
        .filter_map(|line| {
            line.strip_prefix("export let ")
                .and_then(|line| line.split_once(':').map(|(name, _)| name))
        })
        .collect();
    assert_eq!(names.len(), 149); // 148 CSS names plus transparent.
    assert!(names.contains(&"lightSkyBlue"));
    assert!(names.contains(&"darkOliveGreen"));
    assert!(names.contains(&"rebeccaPurple"));
    assert!(!names.contains(&"lightskyblue"));
    let program = crate::ir_runtime_tests::program(&format!(
        "import \"@nx/color\" as c\nlet root() = {{ {} }}",
        names
            .iter()
            .map(|name| format!("c.{name}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    let NxValue::Array(values) = program
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap()
    else {
        panic!("color sequence");
    };
    assert_eq!(values.len(), names.len());
    for (name, value) in names.iter().zip(values) {
        let expected = nx_value::color::parse(name).unwrap();
        let actual = nx_value::color::channels(&value).unwrap();
        let expected = nx_value::color::channels(&expected).unwrap();
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(
                (actual as f32).to_bits(),
                (expected as f32).to_bits(),
                "{name}"
            );
        }
    }
}

#[test]
fn authored_color_type_cannot_shadow_the_literal_type() {
    let program = crate::ir_runtime_tests::program(
        "type Color = { unrelated: string }\nlet root() = #dcdcdc",
    );
    let value = program
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let expected = nx_value::color::parse("gainsboro").unwrap();
    assert_eq!(
        nx_value::color::channels(&value).unwrap(),
        nx_value::color::channels(&expected).unwrap()
    );
}

#[test]
fn authored_color_mismatches_distinguish_the_canonical_literal_type() {
    for body in [
        "let accent: Color = #5b6ef5",
        "type Brand = { accent: Color }\nlet brand = <Brand accent=#5b6ef5 />",
        "let use(value: Color): string = { \"ok\" }\nlet root() = { use(#5b6ef5) }",
    ] {
        let source =
            format!("type Color = {{ r: float32 g: float32 b: float32 a: float32 = 1 }}\n{body}");
        let artifact =
            build_program_artifact_from_source(&source, "main.nx", &ProgramBuildContext::empty())
                .unwrap();
        let messages: Vec<_> = artifact.diagnostics.iter().map(|d| d.message()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("main.nx:Color") && m.contains("@nx/color/color.nx:Color")),
            "{messages:?}"
        );
        assert!(
            messages.iter().all(|m| !m.contains("$nxColor")),
            "{messages:?}"
        );
    }
}

#[test]
fn unknown_type_suggestions_hide_internal_color_imports() {
    for source in [
        "import \"@nx/graphics\" as g\nlet x: colors.Color = #fff",
        "let x: nxColor.Color = #fff",
        "import \"@nx/color\" as c\nlet x: c.Colr = #fff",
    ] {
        let artifact =
            build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
                .unwrap();
        let messages: Vec<_> = artifact.diagnostics.iter().map(|d| d.message()).collect();
        assert!(!messages.is_empty(), "expected an unresolved type");
        assert!(messages.iter().all(|m| !m.contains('$')), "{messages:?}");
        if source.contains("c.Colr") {
            assert!(
                messages
                    .iter()
                    .any(|m| m.contains("did you mean `c.Color`")),
                "{messages:?}"
            );
        }
    }
}

#[test]
fn graphics_color_mismatches_do_not_expose_private_import_aliases() {
    for body in [
        "let value: g.Color = <Color r=1 g=0 b=0 />",
        "let value: Color = <g.Color r=1 g=0 b=0 />",
        "let use(value: g.Color): string = { \"ok\" }\nlet root() = { use(<Color r=1 g=0 b=0 />) }",
    ] {
        let source = format!(
            "import \"@nx/graphics\" as g\ntype Color = {{ r: float32 g: float32 b: float32 a: float32 = 1 }}\n{body}"
        );
        let artifact =
            build_program_artifact_from_source(&source, "main.nx", &ProgramBuildContext::empty())
                .unwrap();
        let messages: Vec<_> = artifact.diagnostics.iter().map(|d| d.message()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("g.Color") || m.contains("@nx/color/color.nx:Color")),
            "{messages:?}"
        );
        assert!(
            messages
                .iter()
                .all(|m| !m.contains("colors.Color") && !m.contains("$nxColor")),
            "{messages:?}"
        );
    }
}

#[test]
fn reexport_mismatch_uses_the_consumers_visible_alias() {
    let modules = [
        ("original.nx", "export type P = { x: int }"),
        (
            "bridge.nx",
            "import \"./original.nx\" as base\nexport type Q = base.P",
        ),
        (
            "chain.nx",
            "import \"./bridge.nx\" as hop\nexport type R = hop.Q",
        ),
        (
            "main.nx",
            "import \"./chain.nx\" as top\ntype P = { x: int }\nlet bad: top.R = <P x=1 />",
        ),
    ]
    .into_iter()
    .map(|(identity, source)| nx_api::NxWorkspaceModule::from_source(identity, source).unwrap())
    .collect();
    let workspace = nx_api::NxWorkspace::new(modules).unwrap();
    let diagnostics = nx_api::validate_workspace(&workspace, &ProgramBuildContext::empty());
    let messages: Vec<_> = diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("top.R") || m.contains("original.nx:P")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|m| !m.contains("base.P") && !m.contains("hop.Q")),
        "{messages:?}"
    );
}

#[test]
fn graphics_solid_paint_rejects_noncanonical_color_payloads() {
    for color in ["\"red\"", "{ 1 }", "{ \"red\" }", "<Color r=1 g=0 b=0 />"] {
        let source = format!(
            "import \"@nx/graphics\" as g\ntype Color = {{ r: float32 g: float32 b: float32 a: float32 = 1 }}\nlet root() = <g.Paint.color color={color} />"
        );
        let artifact =
            build_program_artifact_from_source(&source, "main.nx", &ProgramBuildContext::empty())
                .unwrap();
        assert!(
            artifact
                .diagnostics
                .iter()
                .any(|d| d.message().contains("color") && d.message().contains("expects")),
            "{color}: {:?}",
            artifact.diagnostics
        );
    }
}

#[test]
fn graphics_only_import_retains_transitive_color_type_context() {
    let program = crate::ir_runtime_tests::program(
        r#"import "@nx/graphics" as g
let shade: g.Color = <g.Color r=1 g=0 b=0 />
let root() = <g.Paint.color color={shade} />"#,
    );
    let NxValue::Record { properties, .. } = program
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap()
    else {
        panic!("paint record");
    };
    assert_eq!(
        nx_value::color::channels(&properties["color"]).unwrap(),
        [1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn foreign_field_and_parameter_diagnostics_do_not_expose_private_imports() {
    for field_type in ["Q", "hidden.P"] {
        let library = format!(
            "import \"./original.nx\" as hidden\nexport type Q = hidden.P\nexport type Holder = {{ p: {field_type} }}\nexport type Shape = none | solid {{ p: {field_type} }}\nexport let take(p: {field_type}): int = {{ p.x }}"
        );
        for value in ["\"x\"", "<P x=1 />"] {
            let source = format!(
                "import \"./library.nx\" as b\ntype P = {{ x: int }}\nlet holder = <b.Holder p={value} />\nlet shape = <b.Shape.solid p={value} />\nlet root() = {{ b.take({value}) }}"
            );
            let modules = [
                ("original.nx", "export type P = { x: int }"),
                ("library.nx", library.as_str()),
                ("main.nx", source.as_str()),
            ]
            .into_iter()
            .map(|(identity, source)| {
                nx_api::NxWorkspaceModule::from_source(identity, source).unwrap()
            })
            .collect();
            let workspace = nx_api::NxWorkspace::new(modules).unwrap();
            let diagnostics = nx_api::validate_workspace(&workspace, &ProgramBuildContext::empty());
            let messages: Vec<_> = diagnostics.iter().map(|d| d.message.as_str()).collect();
            assert!(messages.len() >= 3, "{messages:?}");
            assert!(
                messages.iter().all(|m| !m.contains("hidden.P")),
                "{messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .filter(|m| m.contains("expects"))
                    .all(|m| m.contains("original.nx:P") || m.contains("b.Q")),
                "{messages:?}"
            );
        }
    }
}

#[test]
fn foreign_union_type_names_are_public_and_contextual_cases_still_resolve() {
    for valid in [false, true] {
        let source = if valid {
            "import \"./library.nx\" as b\nlet holder = <b.Holder mode=on />\nlet root() = { holder }"
        } else {
            "import \"./library.nx\" as b\nlet holder = <b.Holder mode=\"on\" />\nlet root() = { b.take(\"on\") }"
        };
        let modules = [
            ("original.nx", "export type Mode = on | off"),
            ("library.nx", "import \"./original.nx\" as hidden\nexport type Holder = { mode: hidden.Mode }\nexport let take(mode: hidden.Mode): string = { \"ok\" }"),
            ("main.nx", source),
        ].into_iter().map(|(identity, source)|
            nx_api::NxWorkspaceModule::from_source(identity, source).unwrap()
        ).collect();
        let workspace = nx_api::NxWorkspace::new(modules).unwrap();
        let diagnostics = nx_api::validate_workspace(&workspace, &ProgramBuildContext::empty());
        let messages: Vec<_> = diagnostics.iter().map(|d| d.message.as_str()).collect();
        if valid {
            assert!(messages.is_empty(), "{messages:?}");
        } else {
            assert!(messages.len() >= 2, "{messages:?}");
            assert!(
                messages.iter().all(|m| !m.contains("hidden.Mode")),
                "{messages:?}"
            );
            assert!(
                messages.iter().any(|m| m.contains("original.nx:Mode")),
                "{messages:?}"
            );
        }
    }
}

#[test]
fn graphics_color_aliases_evaluate_as_canonical_colors() {
    let program = crate::ir_runtime_tests::program(
        r##"import "@nx/graphics" as g
import "@nx/color" as c

let format(value: g.Color): string = { c.toHex(value) }
let root() = {
  format(#5b6ef5)
  format(c.parse("red"))
  c.toHex(<g.Color r=0 g=0 b=1 />)
}"##,
    );
    assert_eq!(
        program
            .evaluate_function("root", &[], &RuntimeOptions::default())
            .unwrap(),
        NxValue::Array(vec![
            NxValue::String("#5b6ef5ff".into()),
            NxValue::String("#ff0000ff".into()),
            NxValue::String("#0000ffff".into()),
        ])
    );
}

#[test]
fn color_transport_json_and_messagepack_restore_float32_against_schema() {
    let program = crate::ir_runtime_tests::program(SOURCE);
    let original = program
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let json = original.to_json_string().unwrap();
    let messagepack = original.to_msgpack_vec().unwrap();
    for restored in [
        NxValue::from_json_str(&json).unwrap(),
        NxValue::from_msgpack_slice(&messagepack).unwrap(),
    ] {
        let echoed = program
            .evaluate_function("echo", &[restored.clone()], &RuntimeOptions::default())
            .unwrap();
        assert_eq!(echoed, original);
        let color = program
            .evaluate_function("validateBrand", &[restored], &RuntimeOptions::default())
            .unwrap();
        let NxValue::Record {
            properties,
            type_name,
        } = color
        else {
            panic!("Color record")
        };
        assert_eq!(type_name.as_deref(), Some("Color"));
        assert_eq!(properties["a"], NxValue::Float(f64::from(68.0_f32 / 255.0)));
        assert_eq!(properties["r"], NxValue::Float(f64::from(17.0_f32 / 255.0)));
    }
}

#[test]
fn color_transport_default_alpha_and_invalid_channels() {
    let program = crate::ir_runtime_tests::program(SOURCE);
    let input = NxValue::from_json_str(r#"{"accent":{"r":0.1,"g":0.2,"b":0.3}}"#).unwrap();
    let result = program
        .evaluate_function("validateBrand", &[input], &RuntimeOptions::default())
        .unwrap();
    let NxValue::Record { properties, .. } = result else {
        panic!("Color")
    };
    assert_eq!(properties["a"], NxValue::Float(1.0));
    assert_eq!(properties["r"], NxValue::Float(f64::from(0.1_f32)));
    let invalid = NxValue::from_json_str(r#"{"accent":{"r":2,"g":0,"b":0}}"#).unwrap();
    let error = program
        .evaluate_function("validateBrand", &[invalid], &RuntimeOptions::default())
        .unwrap_err();
    assert_eq!(error.code(), "color-invalid-channel");
}

#[test]
fn color_schema_exports_numerical_fields_and_alpha_default() {
    let artifact =
        build_program_artifact_from_source(SOURCE, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let schema = nx_api::program_artifact_function_schema(
        &artifact,
        &nx_api::DeclarationRef::in_module("main.nx", "echo"),
        &nx_api::FunctionSchemaOptions::default(),
    )
    .unwrap();
    let schema = serde_json::to_value(schema.input_schema.unwrap()).unwrap();
    let color = schema["$defs"]
        .as_object()
        .unwrap()
        .values()
        .find(|definition| definition["properties"]["r"]["type"] == "number")
        .expect("Color definition");
    for channel in ["r", "g", "b", "a"] {
        assert_eq!(color["properties"][channel]["type"], "number");
    }
    assert_eq!(color["properties"]["a"]["default"], 1.0);
    assert!(!color["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "a"));
    assert!(color["properties"]["r"].get("minimum").is_none());
}

#[test]
fn legacy_interpreter_refuses_color_literals_and_constants_explicitly() {
    for source in [
        "let root() = #f00",
        "import \"@nx/color\" as c\nlet root() = { c.red }",
    ] {
        let artifact =
            build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
                .unwrap();
        match nx_api::eval_program_artifact_function(&artifact, "main.nx", "root") {
            nx_api::EvalResult::Err(diagnostics) => assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains("color requires the IR runtime")),
                "{diagnostics:?}"
            ),
            _ => panic!("expected explicit unsupported-runtime error"),
        }
    }
}
