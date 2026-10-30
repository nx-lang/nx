use crate::{build_nx_ir_artifacts, NxIrEmitOptions};
use nx_api::{build_program_artifact_from_source, ProgramBuildContext};
use nx_ir_runtime::RuntimeOptions;

#[test]
fn graphics_ir_links_transitive_math_and_applies_defaults() {
    let source = r#"import "@nx/graphics" as g
let root() = <g.Drawing viewBox=<g.ViewBox x=0 y=0 width=100 height=100/>>
  <g.Rect width=20 height=30 opacity=0.5/>
</g.Drawing>"#;
    let artifact =
        build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let images = build_nx_ir_artifacts(
        &crate::build_codegen_program(&artifact).unwrap(),
        &NxIrEmitOptions {
            modules: Some(vec![]),
            debug: true,
        },
    )
    .unwrap();
    for identity in ["main.nx", "@nx/graphics/graphics.nx", "@nx/math/math.nx"] {
        assert!(images
            .iter()
            .any(|image| image.modules[0].identity == identity));
    }
    let math = images
        .iter()
        .find(|image| image.modules[0].identity == "@nx/math/math.nx")
        .unwrap();
    assert!(math
        .required_features
        .iter()
        .any(|feature| feature == "standard-math-v1"));
    let actual = crate::ir_runtime_tests::program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let value = serde_json::to_value(actual).unwrap();
    assert_eq!(value["$type"], "Drawing");
    assert_eq!(value["fit"], "contain");
    assert_eq!(value["opacity"], 1.0);
    let children = &value["children"];
    let child = if children.is_array() {
        &children[0]
    } else {
        children
    };
    assert_eq!(child["$type"], "Rect");
    assert_eq!(child["x"], 0.0);
    assert_eq!(child["y"], 0.0);
    assert_eq!(child["clipRule"], "nonzero");
    assert_eq!(child["opacity"], 0.5);
}

#[test]
fn graphics_helpers_evaluate_as_primitives_on_rust_ir() {
    let program =
        crate::ir_runtime_tests::program(include_str!("../../../examples/graphics/runtime.nx"));
    let value = |name| {
        serde_json::to_value(
            program
                .evaluate_function(name, &[], &RuntimeOptions::default())
                .unwrap(),
        )
        .unwrap()
    };
    let polygon = value("polygon");
    assert_eq!(polygon["$type"], "Polygon");
    assert_eq!(polygon["points"].as_array().unwrap().len(), 3);
    assert!((polygon["points"][1]["x"].as_f64().unwrap() + 5.0).abs() < 1e-5);
    for (name, count) in [("star", 5), ("revisedStar", 7)] {
        let star = value(name);
        let points = star["points"].as_array().unwrap();
        assert_eq!(points.len(), count * 2);
        for (i, p) in points.iter().enumerate() {
            assert!(
                (p["x"].as_f64().unwrap().hypot(p["y"].as_f64().unwrap())
                    - if i % 2 == 0 { 10.0 } else { 5.0 })
                .abs()
                    < 1e-5
            );
        }
    }
    assert_eq!(value("arrow")["points"][3]["x"], 30.0);
    assert!(value("invalidArrow")["data"]
        .as_str()
        .unwrap()
        .contains("nx-graphics-invalid: Arrow"));
    assert_eq!(value("ring")["fillRule"], "evenodd");
    assert_eq!(value("zeroArc")["data"], "");
    assert_eq!(value("aliasedValue")["width"], 3.0);
    assert_eq!(
        program
            .evaluate_function("numericalFailure", &[], &RuntimeOptions::default())
            .unwrap_err()
            .code(),
        "nx-math-domain"
    );
    let large = value("largeRotation");
    let radians = (1e100_f64 % 360.0).to_radians();
    assert!((large["points"][0]["x"].as_f64().unwrap() - 10.0 * radians.cos()).abs() < 1e-5);
    assert!(
        (value("periodicRotation")["points"][0]["y"]
            .as_f64()
            .unwrap()
            - 10.0)
            .abs()
            < 1e-5
    );
    assert_eq!(
        value("periodicArc")["data"]
            .as_str()
            .unwrap()
            .matches(" A ")
            .count(),
        2
    );
    assert_eq!(
        value("nonfiniteArc")["data"],
        "nx-graphics-invalid: Arc radius must be finite"
    );
    assert_eq!(
        value("nonfiniteRing")["data"],
        "nx-graphics-invalid: RingSector outerRadius must be finite"
    );
}

#[test]
fn graphics_helpers_report_overflow_and_solid_rings_have_no_central_subpath() {
    let program = crate::ir_runtime_tests::program(
        r#"import "@nx/graphics" as g
let arrow() = { <g.Arrow x1=-1.0e308 y1=0 x2=1.0e308 y2=0/> }
let polygon() = { <g.RegularPolygon cx=1.0e308 cy=0 radius=1.0e308 pointCount=3/> }
let star() = { <g.Star cx=1.0e308 cy=0 outerRadius=1.0e308 innerRadius=1 pointCount=5/> }
let solid() = { <g.RingSector cx=50 cy=50 outerRadius=40 sweepAngle=360/> }
let arrowPoints() = { <g.Arrow x1=0 y1=0 x2=30 y2=0 shaftWidth=4 headWidth=10 headLength=8/> }
"#,
    );
    for name in ["arrow", "polygon", "star"] {
        let value = serde_json::to_value(
            program
                .evaluate_function(name, &[], &RuntimeOptions::default())
                .unwrap(),
        )
        .unwrap();
        assert!(value["data"]
            .as_str()
            .unwrap()
            .starts_with("nx-graphics-invalid: "));
        assert!(value["data"].as_str().unwrap().contains("must be finite"));
    }
    let solid = serde_json::to_value(
        program
            .evaluate_function("solid", &[], &RuntimeOptions::default())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(solid["data"].as_str().unwrap().matches("M ").count(), 1);
    let arrow = serde_json::to_value(
        program
            .evaluate_function("arrowPoints", &[], &RuntimeOptions::default())
            .unwrap(),
    )
    .unwrap();
    let expected = [
        (0.0, 2.0),
        (22.0, 2.0),
        (22.0, 5.0),
        (30.0, 0.0),
        (22.0, -5.0),
        (22.0, -2.0),
        (0.0, -2.0),
    ];
    for (point, (x, y)) in arrow["points"].as_array().unwrap().iter().zip(expected) {
        assert!((point["x"].as_f64().unwrap() - x).abs() < 1e-10);
        assert!((point["y"].as_f64().unwrap() - y).abs() < 1e-10);
    }
}

#[test]
fn graphics_arc_helper_endpoints_follow_signed_clockwise_degrees() {
    let program = crate::ir_runtime_tests::program(
        r#"import "@nx/graphics" as g
let positive() = { <g.Arc cx=5 cy=6 radius=10 sweepAngle=90/> }
let negative() = { <g.Arc cx=5 cy=6 radius=10 sweepAngle=-90/> }
"#,
    );
    for (name, sign) in [("positive", 1.0), ("negative", -1.0)] {
        let value = serde_json::to_value(
            program
                .evaluate_function(name, &[], &RuntimeOptions::default())
                .unwrap(),
        )
        .unwrap();
        let coordinates = value["data"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect::<Vec<_>>();
        let expected = [
            (coordinates[0], 15.0),
            (coordinates[1], 6.0),
            (coordinates[7], 5.0 + 10.0 * std::f64::consts::FRAC_1_SQRT_2),
            (
                coordinates[8],
                6.0 + sign * 10.0 * std::f64::consts::FRAC_1_SQRT_2,
            ),
            (coordinates[14], 5.0),
            (coordinates[15], 6.0 + sign * 10.0),
        ];
        for (actual, expected) in expected {
            assert!(
                (actual - expected).abs() < 1e-5,
                "{name}: {actual} != {expected}"
            );
        }
    }
}

#[test]
fn graphics_aliased_records_defaults_and_payload_cases_evaluate_on_ir() {
    let evaluate = |body: &str| {
        let source = format!("import \"@nx/graphics\" as g\ntype ViewBox = {{ unrelated:string }}\nlet root() = {{ {body} }}");
        serde_json::to_value(
            crate::ir_runtime_tests::program(&source)
                .evaluate_function("root", &[], &RuntimeOptions::default())
                .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(
        evaluate("<g.ViewBox x=1 y=2 width=3 height=4/>"),
        serde_json::json!({"$type":"ViewBox", "x":1.0, "y":2.0, "width":3.0, "height":4.0}),
    );
    assert_eq!(
        evaluate("<g.ContentAlignment/>"),
        serde_json::json!({"$type":"ContentAlignment", "x":"center", "y":"center"}),
    );
    let rect = evaluate("<g.Rect width=2 height=3 transform={<g.Transform.translate x=1 y=2/>} fill=<g.Paint.color color=#ff0000/> stroke=<g.Stroke paint={g.Paint.none}/>/>");
    assert_eq!(rect["$type"], "Rect");
    assert_eq!(rect["width"], 2.0);
    assert_eq!(rect["height"], 3.0);
    assert_eq!(rect["opacity"], 1.0);
    let transform = match rect["transform"].as_array() {
        Some(transforms) => {
            assert_eq!(transforms.len(), 1);
            &transforms[0]
        }
        None => &rect["transform"],
    };
    assert_eq!(
        transform,
        &serde_json::json!({"$type":"Transform.translate", "x":1.0, "y":2.0})
    );
    assert_eq!(
        rect["fill"],
        serde_json::json!({"$type":"Paint.color", "color":{"$type":"Color", "r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0}})
    );
    assert_eq!(rect["stroke"]["$type"], "Stroke");
    assert_eq!(rect["stroke"]["paint"], "none");
    assert_eq!(rect["stroke"]["width"], 1.0);
}

#[test]
fn graphics_consumers_accept_aliased_shared_types_on_ir() {
    let source = r#"import "@nx/graphics" as g
type ViewBox = { unrelated:string }
type Holder = { box:g.ViewBox transform:g.Transform }
let id(t:g.Transform):g.Transform = { t }
let root() = <Holder box=<g.ViewBox x=1 y=2 width=3 height=4/> transform={id(<g.Transform.translate x=5 y=6/>)}/>
"#;
    let value = crate::ir_runtime_tests::program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let value = serde_json::to_value(value).unwrap();
    assert_eq!(value["$type"], "Holder");
    assert_eq!(
        value["box"],
        serde_json::json!({"$type":"ViewBox", "x":1.0, "y":2.0, "width":3.0, "height":4.0})
    );
    assert_eq!(
        value["transform"],
        serde_json::json!({"$type":"Transform.translate", "x":5.0, "y":6.0})
    );
}

#[test]
fn element_props_accept_local_components_and_intrinsic_children_on_ir() {
    let source = r#"external component <Box content children?:Element+ />
external component <Shape />
let root() = <Box><Shape/><div/></Box>"#;
    let value = crate::ir_runtime_tests::program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let value = serde_json::to_value(value).unwrap();
    assert_eq!(value["$type"], "Box");
    assert_eq!(value["children"][0]["$type"], "Shape");
    assert_eq!(value["children"][1]["$type"], "div");
}

#[test]
fn color_literal_ir_infers_canonical_import_free_records() {
    let source = "type Color = { decoy:string }\nlet root() = { #f008 #FF000088 #fff }";
    let actual = crate::ir_runtime_tests::program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let value = serde_json::to_value(actual).unwrap();
    assert_eq!(value[0], value[1]);
    assert_eq!(value[0]["$type"], "Color");
    assert_eq!(
        value[0]["a"].as_f64().unwrap(),
        (136.0_f64 / 255.0) as f32 as f64
    );
    assert_eq!(value[2]["r"], 1.0);
    assert_eq!(value[2]["a"], 1.0);
}

#[test]
fn color_literals_bind_defaults_and_unbraced_properties() {
    let source = "import \"@nx/color\" as c\ntype Brand = { accent:c.Color = #fff }\nlet root() = { <Brand/> <Brand accent=#1234/> }";
    let actual = crate::ir_runtime_tests::program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let value = serde_json::to_value(actual).unwrap();
    assert_eq!(value[0]["accent"]["r"], 1.0);
    assert_eq!(
        value[1]["accent"]["a"].as_f64().unwrap(),
        (68.0_f64 / 255.0) as f32 as f64
    );
}

#[test]
fn standard_color_runtime_construction_and_parse_result() {
    let program = crate::ir_runtime_tests::program(
        r##"import "@nx/color" as c
let root() = { c.srgb(0.2,0.4,0.8,0.5) }
let failure() = { c.tryParse("invalid") }
let contrast() = { c.contrastRatio(#000,#fff) }
let textColor() = { c.bestTextColor(#fff) }
let defaultAlpha() = { c.srgb(0.2,0.4,0.8) }
"##,
    );
    let value = program
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let color = serde_json::to_value(value).unwrap();
    assert_eq!(color["r"], serde_json::json!(0.2f32 as f64));
    assert_eq!(color["a"], serde_json::json!(0.5));
    assert_eq!(
        program
            .evaluate_function("contrast", &[], &RuntimeOptions::default())
            .unwrap(),
        nx_value::NxValue::Float(21.0)
    );
    let text_color = program
        .evaluate_function("textColor", &[], &RuntimeOptions::default())
        .unwrap();
    assert_eq!(
        nx_value::color::channels(&text_color).unwrap(),
        [0.0, 0.0, 0.0, 1.0]
    );
    let default_alpha = program
        .evaluate_function("defaultAlpha", &[], &RuntimeOptions::default())
        .unwrap();
    assert_eq!(nx_value::color::channels(&default_alpha).unwrap()[3], 1.0);
    let failed = serde_json::to_value(
        program
            .evaluate_function("failure", &[], &RuntimeOptions::default())
            .unwrap(),
    )
    .unwrap();
    assert!(failed["message"].as_str().unwrap().contains("invalid"));
}

#[test]
fn color_literal_links_without_color_operation_module() {
    use nx_ir_runtime::{LinkOptions, PreparedModule, Program};
    use std::collections::BTreeMap;
    let source = "let root() = #33669980";
    let artifact =
        build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let images = crate::emit_nx_ir(
        &artifact,
        &NxIrEmitOptions {
            modules: Some(vec![]),
            debug: true,
        },
    )
    .unwrap();
    let modules: BTreeMap<_, _> = images
        .into_iter()
        .filter(|image| image.identity != "@nx/color/operations.nx")
        .inspect(|image| {
            assert!(!image
                .metadata
                .required_features
                .iter()
                .any(|f| f == "standard-color-v1"))
        })
        .map(|image| {
            let module = PreparedModule::prepare(image.bytes).unwrap();
            (image.identity, module)
        })
        .collect();
    let program = Program::link(
        modules.get("main.nx").unwrap(),
        |identity| modules.get(identity).cloned(),
        &LinkOptions::default(),
    )
    .unwrap();
    let value = serde_json::to_value(
        program
            .evaluate_function("root", &[], &RuntimeOptions::default())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        value["a"].as_f64().unwrap(),
        (128.0_f64 / 255.0) as f32 as f64
    );
}

#[test]
fn graphics_gradients_evaluate_as_ordinary_reusable_paints() {
    let program =
        crate::ir_runtime_tests::program(include_str!("../../../examples/graphics/runtime.nx"));
    let value = serde_json::to_value(
        program
            .evaluate_function("gradientShape", &[], &RuntimeOptions::default())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["fill"]["$type"], "Paint.linearGradient");
    assert_eq!(value["fill"], value["stroke"]["paint"]);
    assert_eq!(value["fill"]["stops"].as_array().unwrap().len(), 2);
}

#[test]
fn graphics_gradients_cover_every_paint_location_on_ir() {
    let program =
        crate::ir_runtime_tests::program(include_str!("../../../examples/graphics/runtime.nx"));
    let value = |name| {
        serde_json::to_value(
            program
                .evaluate_function(name, &[], &RuntimeOptions::default())
                .unwrap(),
        )
        .unwrap()
    };
    let shared = value("gradientPaint");
    assert_eq!(value("gradientRect")["fill"], shared);
    assert_eq!(value("gradientLine")["stroke"]["paint"], shared);
    assert_eq!(value("gradientText")["fill"], shared);
    assert_eq!(
        value("gradientBackground")["background"],
        value("radialPaint")
    );
    assert_eq!(value("radialPaint")["$type"], "Paint.radialGradient");
}
