//! Standard math across compiled IR, legacy Rust evaluation, and executable targets.
use crate::ir_runtime_tests::program;
use crate::{build_nx_ir_artifacts, emit_nx_ir, NxIrEmitOptions};
use nx_api::{build_program_artifact_from_source, ProgramBuildContext};
use nx_ir::{write_nx_ir_image, IrItem};
use nx_ir_runtime::{PreparedModule, RuntimeOptions};
use nx_value::{math::MathOp, NxValue};

fn probes() -> String {
    let mut source = "import \"@nx/math\" as math\n".to_string();
    for op in MathOp::ALL {
        source.push_str(&format!(
            "let {}({}):float64 = {{ math.{}({}) }}\n",
            op.name(),
            op.params()
                .iter()
                .map(|p| format!("{p}:float64"))
                .collect::<Vec<_>>()
                .join(","),
            op.name(),
            op.params().join(",")
        ));
    }
    source
}

#[test]
fn standard_math_reference_corpus_in_rust_runtimes() {
    let source = probes();
    let program = program(&source);
    let artifact =
        build_program_artifact_from_source(&source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    let legacy =
        nx_interpreter::Interpreter::from_resolved_program(artifact.resolved_program.clone());
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../crates/nx-value/src/math-cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        let op = case["op"].as_str().unwrap();
        let result = program.evaluate_function(
            op,
            &args.iter().copied().map(NxValue::Float).collect::<Vec<_>>(),
            &RuntimeOptions::default(),
        );
        let legacy_result = legacy.execute_resolved_program_function(
            op,
            args.iter()
                .copied()
                .map(nx_interpreter::Value::Float)
                .collect(),
        );
        if let Some(code) = case["error"].as_str() {
            let error = result.unwrap_err();
            assert_eq!(error.code(), code, "{}", case["name"]);
            assert!(error.diagnostics[0].message.contains(op));
            assert!(
                matches!(legacy_result.unwrap_err().kind(),nx_interpreter::RuntimeErrorKind::Math(e) if e.code == code)
            );
        } else {
            let NxValue::Float(actual) = result.unwrap() else {
                panic!("expected float")
            };
            let nx_interpreter::Value::Float(legacy_actual) = legacy_result.unwrap() else {
                panic!("expected legacy float")
            };
            let expected = case["expected"].as_str().unwrap().parse::<f64>().unwrap();
            for value in [actual, legacy_actual] {
                if case["exact"] == true {
                    assert_eq!(value.to_bits(), expected.to_bits(), "{}", case["name"]);
                } else {
                    assert!(
                        (value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                        "{}: {value} vs {expected}",
                        case["name"]
                    );
                }
            }
        }
    }
}

#[test]
fn standard_math_function_values_and_shadowing() {
    let source = r#"import "@nx/math" as math
let sin(x:float64):float64 = { x + 10.0 }
type Numeric = <function x:float64/>:float64
let pass(f:Numeric):Numeric = { f }
let f:Numeric = { pass(math.sin) }
let root():float64 = <f x=0.0/>
let authored():float64 = { sin(1.0) }
"#;
    let result = program(source)
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    assert_eq!(result, NxValue::Float(0.0));
    assert_eq!(
        program(source)
            .evaluate_function("authored", &[], &RuntimeOptions::default())
            .unwrap(),
        NxValue::Float(11.0)
    );
    let artifact =
        build_program_artifact_from_source(source, "main.nx", &ProgramBuildContext::empty())
            .unwrap();
    match nx_api::eval_program_artifact_function(&artifact, "main.nx", "root") {
        nx_api::EvalResult::Ok(value) => assert_eq!(value, NxValue::Float(0.0)),
        nx_api::EvalResult::Err(e) => panic!("{e:?}"),
    }
}

#[test]
fn standard_math_images_are_gated_validated_and_independent() {
    let artifact = build_program_artifact_from_source(
        "import \"@nx/math\"\nlet root() = { sin(0.0) }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .unwrap();
    let options = NxIrEmitOptions {
        modules: Some(vec![]),
        debug: true,
    };
    let images =
        build_nx_ir_artifacts(&crate::build_codegen_program(&artifact).unwrap(), &options).unwrap();
    let math = images
        .iter()
        .find(|m| m.modules[0].identity == "@nx/math/math.nx")
        .unwrap();
    assert!(math
        .required_features
        .iter()
        .any(|f| f == "standard-math-v1"));
    assert!(!images
        .iter()
        .find(|m| m.modules[0].identity == "main.nx")
        .unwrap()
        .required_features
        .iter()
        .any(|f| f == "standard-math-v1"));
    let position = math
        .nodes
        .iter()
        .position(|node| matches!(node,IrItem::List(c) if c[0] == IrItem::Int(26)))
        .unwrap();
    for bad in [
        IrItem::list([IrItem::Int(26), IrItem::Int(999), IrItem::ints([0])]),
        IrItem::list([IrItem::Int(26), IrItem::Int(0), IrItem::ints([])]),
        IrItem::list([IrItem::Int(26), IrItem::Int(0), IrItem::ints([999999])]),
    ] {
        let mut malformed = math.clone();
        malformed.nodes[position] = bad;
        // Writers validate the same structure as readers; either stage must reject it.
        if let Ok(bytes) = write_nx_ir_image(&malformed) {
            assert!(PreparedModule::prepare(bytes).is_err());
        }
    }
    let mut no_feature = math.clone();
    no_feature
        .required_features
        .retain(|f| f != "standard-math-v1");
    if let Ok(bytes) = write_nx_ir_image(&no_feature) {
        assert!(PreparedModule::prepare(bytes).is_err());
    }
    let mut future = math.clone();
    future.required_features.push("standard-math-v999".into());
    future.strings.push("standard-math-v999".into());
    assert_eq!(
        PreparedModule::prepare(write_nx_ir_image(&future).unwrap())
            .unwrap_err()
            .code(),
        "nx-ir-required-feature"
    );
    let other = build_program_artifact_from_source(
        "import \"@nx/math\" as m\nlet root() = { m.cos(1.0) }",
        "main.nx",
        &ProgramBuildContext::empty(),
    )
    .unwrap();
    let bytes = |artifact| {
        emit_nx_ir(
            artifact,
            &NxIrEmitOptions {
                modules: Some(vec!["@nx/math/math.nx".into()]),
                debug: false,
            },
        )
        .unwrap()[0]
            .bytes
            .clone()
    };
    assert_eq!(bytes(&artifact), bytes(&other));
}

#[test]
fn standard_math_respects_operation_budget() {
    let program = program("import \"@nx/math\"\nlet root() = { sin(0.0) }");
    let options = RuntimeOptions {
        max_operations: Some(1),
        ..RuntimeOptions::default()
    };
    assert_eq!(
        program
            .evaluate_function("root", &[], &options)
            .unwrap_err()
            .code(),
        "nx-ir-resource-limit"
    );
}

#[test]
fn standard_math_error_keeps_authored_call_origin() {
    let p = program("import \"@nx/math\"\nlet root() = { sqrt(-1.0) }");
    let error = p
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap_err();
    assert_eq!(error.code(), "nx-math-domain");
    assert!(error.diagnostics[0]
        .declaration
        .as_ref()
        .unwrap()
        .contains("math.nx::sqrt"));
    assert_eq!(
        error.diagnostics[0].source.as_ref().unwrap().identity,
        "main.nx"
    );
}

#[test]
fn standard_math_geometry_example() {
    let p = program(include_str!("../../../examples/math-graphics/main.nx"));
    let value = p
        .evaluate_function("root", &[], &RuntimeOptions::default())
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&value.to_json_string().unwrap()).unwrap();
    assert_eq!(json["polygon"].as_array().unwrap().len(), 3);
    assert_eq!(json["star"].as_array().unwrap().len(), 6);
    assert_eq!(json["arrow"].as_array().unwrap().len(), 5);
    for (index, x, y) in [
        (0, 10.0, 0.0),
        (1, -5.0, 8.660254037844386),
        (2, -5.0, -8.660254037844386),
    ] {
        assert!((json["polygon"][index]["x"].as_f64().unwrap() - x).abs() < 1e-12);
        assert!((json["polygon"][index]["y"].as_f64().unwrap() - y).abs() < 1e-12);
    }
    assert_eq!(json["arrow"][2]["x"], 1.0);
    assert_eq!(json["arrow"][2]["y"], 3.0);
}

#[test]
fn standard_math_operation_tables_agree() {
    let ts = include_str!("../../../runtime/typescript/src/math.ts");
    let ts_arities = ts
        .split_once("export const nxStandardMathArities = [")
        .unwrap()
        .1
        .split_once(']')
        .unwrap()
        .0
        .split(',')
        .map(|n| n.trim().parse::<u32>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(MathOp::ALL.len(), nx_ir::kinds::standard_math::NAMES.len());
    assert_eq!(
        MathOp::ALL.len(),
        nx_ir::kinds::standard_math::ARITIES.len()
    );
    assert_eq!(ts_arities, nx_ir::kinds::standard_math::ARITIES);
    for (op, (code, name)) in MathOp::ALL
        .into_iter()
        .zip(nx_ir::kinds::standard_math::NAMES)
    {
        assert_eq!(op as i64, *code);
        assert_eq!(op.name(), *name);
        assert_eq!(
            op.arity() as u32,
            nx_ir::kinds::standard_math::ARITIES[*code as usize]
        );
        assert_eq!(MathOp::from_code(*code as u32), Some(op));
    }
    assert_eq!(MathOp::from_code(999), None);
}
