use nx_syntax::{parse_str, AstNode, FunctionDef};

#[test]
fn external_declarations_parse_without_a_body() {
    for source in [
        "/// Cosine.\nexport external cos(x:float64):float64",
        "external <Cos x:float64 />:float64",
        "private external cos(x:float64):float64",
        "export external first(x:float64):float64\nexport external second(x:float64):float64",
        "let external(x:int):int = x",
        "external letter(x:float64):float64",
        "external external(x:float64):float64",
    ] {
        let parsed = parse_str(source, "external.nx");
        assert!(parsed.errors.is_empty(), "{source}: {:?}", parsed.errors);
        if source.starts_with("///") {
            let tree = parsed.tree.unwrap();
            let function = tree.root().children().find_map(FunctionDef::cast).unwrap();
            assert!(function.is_external());
            assert!(function.body().is_none());
        }
    }
}

#[test]
fn external_declarations_require_return_types_and_forbid_authored_bodies() {
    for source in [
        "external cos(x:float64)",
        "external cos(x:float64):float64 = 0.0",
        "external cos(x:float64) = 0.0",
        "external value:float64",
        "let cos(x:float64):float64",
    ] {
        let parsed = parse_str(source, "external.nx");
        assert!(
            !parsed.errors.is_empty(),
            "accepted invalid declaration: {source}"
        );
    }
}

#[test]
fn invalid_external_shapes_have_targeted_diagnostics_and_keep_following_declarations() {
    for (declaration, message) in [
        (
            "external cos(x:float64):float64 = { 0.0 }",
            "cannot have a body",
        ),
        ("external cos(x:float64)", "requires a result type"),
        (
            "external <Cos x:float64 /> = { 0.0 }",
            "requires a result type",
        ),
    ] {
        let source = format!("{declaration}\nlet other() = {{ 1 }}\nlet root() = {{ other() }}\n");
        let parsed = parse_str(&source, "recovery.nx");
        let diagnostic = parsed
            .errors
            .iter()
            .find(|d| d.message().contains(message))
            .expect(message);
        assert_eq!(
            diagnostic.code(),
            Some("invalid-external-function-definition")
        );
        for label in diagnostic.labels() {
            assert!(
                usize::from(label.range.end()) <= declaration.len(),
                "diagnostic swallows following declarations"
            );
        }
        assert!(
            !parsed
                .errors
                .iter()
                .any(|d| d.code() == Some("syntax-error")),
            "{:?}",
            parsed.errors
        );
        let tree = parsed.tree.unwrap();
        let names = tree
            .root()
            .children()
            .filter_map(FunctionDef::cast)
            .map(|function| function.signature().unwrap().text().to_string())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == "other"));
        assert!(names.iter().any(|name| name == "root"));
    }
}

#[test]
fn external_value_shapes_report_locally_and_preserve_following_functions() {
    for declaration in [
        "external value:float64",
        "external value:float64 = 0.0",
        "external value",
    ] {
        let source = format!("{declaration}\nlet other() = {{ 1 }}\nlet root() = {{ other() }}\n");
        let parsed = parse_str(&source, "recovery.nx");
        assert_eq!(parsed.errors.len(), 1, "{:?}", parsed.errors);
        let diagnostic = &parsed.errors[0];
        assert_eq!(
            diagnostic.code(),
            Some("invalid-external-function-definition")
        );
        assert_eq!(
            diagnostic.message(),
            "External declarations must be functions"
        );
        assert!(diagnostic
            .labels()
            .iter()
            .all(|label| usize::from(label.range.end()) <= declaration.len()));
        let tree = parsed.tree.unwrap();
        let names = tree
            .root()
            .children()
            .filter_map(FunctionDef::cast)
            .map(|function| function.signature().unwrap().text().to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, ["value", "other", "root"]);
    }
}

#[test]
fn half_typed_external_declarations_do_not_misdiagnose_the_next_function() {
    for head in ["external cos(", "external ", "export external cos(x:"] {
        let source = format!("{head}\nlet other() = {{ 1 }}\nlet root() = {{ other() }}\n");
        let parsed = parse_str(&source, "typing.nx");
        assert!(!parsed.errors.is_empty());
        assert!(!parsed.errors.iter().any(|diagnostic| diagnostic.code() == Some("invalid-external-function-definition")), "{:?}", parsed.errors);
    }
}

#[test]
fn external_let_is_incorrect_and_suggests_removing_let() {
    for declaration in [
        "export external let sin(x:float64):float64",
        "external let <Cos x:float64 />:float64",
        "external let",
    ] {
        let source = format!("{declaration}\nlet other() = {{ 1 }}\nlet root() = {{ other() }}\n");
        let parsed = parse_str(&source, "incorrect-external.nx");
        let error = parsed
            .errors
            .iter()
            .find(|d| d.message() == "External functions are declared without `let`")
            .expect("targeted incorrect syntax diagnostic");
        assert_eq!(error.code(), Some("invalid-external-function-definition"));
        assert_eq!(&source[error.labels()[0].range], "let");
        assert_eq!(error.severity(), nx_diagnostics::Severity::Error);
        assert_eq!(
            error.labels()[0].message.as_deref(),
            Some("remove `let`; write `external name(params):Result`")
        );
        assert!(
            !parsed
                .errors
                .iter()
                .any(|d| d.code() == Some("syntax-error")),
            "{:?}",
            parsed.errors
        );
        let tree = parsed.tree.unwrap();
        let names = tree
            .root()
            .children()
            .filter_map(FunctionDef::cast)
            .filter_map(|f| f.signature())
            .map(|n| n.text().to_string())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|n| n == "other"));
        assert!(names.iter().any(|n| n == "root"));
    }
}
