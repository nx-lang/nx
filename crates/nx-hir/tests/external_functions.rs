use nx_hir::{ast::Expr, lower, lower_with_runtime_functions, Item, SourceId};
use nx_value::math::MathOp;

fn trusted(source: &str, operations: &[MathOp]) -> nx_hir::LoweredModule {
    let parsed = nx_syntax::parse_str(source, "external.nx");
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    lower_with_runtime_functions(parsed.tree.unwrap().root(), SourceId::new(0), operations)
}

#[test]
fn external_signature_keeps_docs_and_source_but_body_is_synthetic() {
    let source =
        "/// Cosine in radians.\nexport external cos(\n/// Input angle.\nx:float64):float64\n";
    let module = trusted(source, &[MathOp::Cos]);
    assert!(
        module.diagnostics().is_empty(),
        "{:?}",
        module.diagnostics()
    );
    let Item::Function(function) = module.items().first().unwrap() else {
        panic!("function")
    };
    assert!(function.is_external);
    assert_eq!(function.doc.as_ref().unwrap().text, "Cosine in radians.");
    assert_eq!(
        function.params[0].doc.as_ref().unwrap().text,
        "Input angle."
    );
    assert_eq!(
        &source[usize::from(function.span.start())..usize::from(function.span.end())],
        "export external cos(\n/// Input angle.\nx:float64):float64"
    );
    let Expr::StandardMath { op, args } = module.expr(function.body) else {
        panic!("math body")
    };
    assert_eq!(*op, MathOp::Cos);
    assert!(module.expr_span(function.body).is_empty());
    assert!(module.expr_span(args[0]).is_empty());
}

#[test]
fn external_runtime_bindings_reject_signature_drift_and_missing_declarations() {
    for source in [
        "export external cos(y:float64):float64",
        "export external cos(x:int):float64",
        "export external cos(x:float64):int",
        "export external cos(x:float64,y:float64):float64",
        "export external cos(x?:float64):float64",
        "export external cos(x:float64 = 0.0):float64",
        "export external <cos x:float64 />:float64",
        "private external cos(x:float64):float64",
        "export let cos(x:float64):float64 = 0.0",
        "export let another = 1",
        "export external cos(x:float64):float64\nexport external cos(x:float64):float64",
    ] {
        let module = trusted(source, &[MathOp::Cos]);
        assert!(
            !module.diagnostics().is_empty(),
            "accepted mismatched registry declaration: {source}"
        );
    }
}

#[test]
fn external_function_names_do_not_authorize_user_implementations() {
    let parsed = nx_syntax::parse_str("external cos(x:float64):float64", "user.nx");
    let module = lower(parsed.tree.unwrap().root(), SourceId::new(0));
    assert!(module.diagnostics().iter().any(|d| d
        .message
        .contains("No runtime implementation is registered")));
}

#[test]
fn registry_diagnostics_explain_mismatches_in_nx_at_the_relevant_site() {
    for (source, reason, marked) in [
        (
            "export external cos(y:float64):float64",
            "parameter 1 must be named 'x'",
            "y:float64",
        ),
        (
            "export external cos(x:int):float64",
            "parameter 'x' must have type float64",
            "x:int",
        ),
        (
            "export external cos(x:float64):int",
            "result type must be float64",
            "int",
        ),
        (
            "export external cos(x?:float64):float64",
            "must be required",
            "x?:float64",
        ),
        (
            "export external cos(x:float64 = 0.0):float64",
            "cannot have a default",
            "x:float64 = 0.0",
        ),
    ] {
        let module = trusted(source, &[MathOp::Cos]);
        let diagnostic = module
            .diagnostics()
            .iter()
            .find(|d| d.message.contains(reason))
            .expect(reason);
        assert!(diagnostic
            .message
            .contains("export external cos(x:float64):float64"));
        assert!(!diagnostic.message.contains("<module>"));
        assert!(!diagnostic.message.contains("v1"));
        assert_eq!(
            &source[usize::from(diagnostic.span.start())..usize::from(diagnostic.span.end())],
            marked
        );
    }
    let missing = trusted("export let other = 1", &[MathOp::Cos]);
    assert!(missing.diagnostics()[0].span.is_empty());
    let source = "export let cos(x:float64):float64 = x\nexport let other = 1";
    let ordinary = trusted(source, &[MathOp::Cos]);
    let span = ordinary.diagnostics()[0].span;
    assert_eq!(
        &source[usize::from(span.start())..usize::from(span.end())],
        "export let cos(x:float64):float64 = x"
    );
}

#[test]
fn invalid_external_syntax_does_not_drop_subsequent_hir_functions() {
    let parsed = nx_syntax::parse_str(
        "external cos(x:float64):float64 = { 0.0 }\nlet other() = { 1 }\nlet root() = { other() }",
        "recovery.nx",
    );
    assert!(!parsed.errors.is_empty());
    let module = lower(parsed.tree.unwrap().root(), SourceId::new(0));
    for name in ["other", "root"] {
        assert!(module
            .items()
            .iter()
            .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == name)));
    }
}

#[test]
fn invalid_external_value_shape_does_not_drop_following_hir_declarations() {
    let parsed = nx_syntax::parse_str(
        "external value:float64\nlet other() = { 1 }\nlet root() = { other() }",
        "recovery.nx",
    );
    assert!(!parsed.errors.is_empty());
    let module = lower(parsed.tree.unwrap().root(), SourceId::new(0));
    for name in ["other", "root"] {
        assert!(module
            .items()
            .iter()
            .any(|item| matches!(item, Item::Function(f) if f.name.as_str() == name)));
    }
}

#[test]
fn half_typed_external_recovery_nodes_do_not_produce_runtime_binding_errors() {
    for head in ["external cos(", "external ", "export external cos(x:"] {
        let source = format!("{head}\nlet other() = {{ 1 }}\nlet root() = {{ other() }}\n");
        let parsed = nx_syntax::parse_str(&source, "typing.nx");
        assert!(!parsed.errors.is_empty());
        let module = lower(parsed.tree.unwrap().root(), SourceId::new(0));
        assert!(
            !module
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("runtime implementation")),
            "{:?}",
            module.diagnostics()
        );
    }
    let parsed = nx_syntax::parse_str("external value:float64", "typing.nx");
    let module = lower(parsed.tree.unwrap().root(), SourceId::new(0));
    assert!(
        module.items().is_empty(),
        "invalid external values must not become functions"
    );
    assert!(
        module.diagnostics().is_empty(),
        "syntax already diagnoses invalid external values"
    );
}
