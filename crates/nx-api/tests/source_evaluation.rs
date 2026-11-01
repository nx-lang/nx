mod support;
mod tests {
    use super::support::*;
    use nx_api::*;
    use nx_program::LibraryRegistry;
    use nx_value::NxValue;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::TempDir;

    /// Documentation never changes what a program means.
    #[test]
    fn doc_comments_do_not_change_evaluation() {
        let plain = r#"type User = {
  name:string
  score:int = 42
}
let label(user:User): string = {user.name}
let root() = { <User name={label(<User name="Ada" />)} /> }
"#;
        let documented = r#"/// A user.
type User = {
  name:string      /// The name.
  score:int = 42   /// The score.
}
/// A label for [User].
let label(
  user:User   /// Whose label.
): string = {user.name}
/// The root.
let root() = { <User name={label(<User name="Ada" />)} /> }
"#;
        let evaluate = |source: &str| match eval_source(
            source,
            "documented.nx",
            &ProgramBuildContext::empty(),
        ) {
            EvalResult::Ok(value) => value,
            EvalResult::Err(diagnostics) => panic!("{diagnostics:?}"),
        };
        assert_eq!(evaluate(plain), evaluate(documented));
    }

    #[test]
    fn eval_source_evaluates_an_int_literal_at_a_float_site_to_a_float() {
        let written_as_int = eval_source(
            "let root(): float64 = 24",
            "eval-int-literal.nx",
            &ProgramBuildContext::empty(),
        );
        let written_as_float = eval_source(
            "let root(): float64 = 24.0",
            "eval-float-literal.nx",
            &ProgramBuildContext::empty(),
        );

        let EvalResult::Ok(from_int) = written_as_int else {
            panic!("an integer literal at a float64 return type should evaluate");
        };
        let EvalResult::Ok(from_float) = written_as_float else {
            panic!("a real literal at a float64 return type should evaluate");
        };

        // Equality alone would hold if both were integers, so the kind is asserted directly.
        assert!(
            matches!(from_int, NxValue::Float(value) if value == 24.0),
            "expected a float value, got {:?}",
            from_int
        );
        assert_eq!(from_int, from_float);
    }

    #[test]
    fn eval_source_returns_aggregated_static_diagnostics_before_runtime_execution() {
        let source = r#"
            abstract type Entity = {
              id: int
            }

            type User extends Entity = {
              name: string
            }

            type Admin extends User = {
              level: int
            }

            let broken(): int = "oops"
            let root(): int = { 1 / 0 }
        "#;

        let EvalResult::Err(diagnostics) = eval_source(
            source,
            "eval-static-errors.nx",
            &ProgramBuildContext::empty(),
        ) else {
            panic!("Expected evaluation to stop on static analysis diagnostics");
        };

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("lowering-error")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("runtime-error")));
    }

    #[test]
    fn eval_source_reports_missing_import_when_source_is_not_on_disk() {
        let source = r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#;

        let EvalResult::Err(diagnostics) =
            eval_source(source, "virtual/main.nx", &ProgramBuildContext::empty())
        else {
            panic!("Expected virtual import source to fail");
        };

        assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("Missing workspace module or loaded library")));
    }

    #[test]
    fn eval_source_resolves_preloaded_local_imports() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("button.nx"),
            r#"export let <Button /> = <button />"#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import { Button as Layout.Button } from "../ui"
let root() = { <Layout.Button /> }"#;
        fs::write(&main_path, source).expect("main file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let EvalResult::Ok(value) =
            eval_source(source, &main_path.display().to_string(), &build_context)
        else {
            panic!("Expected import-backed source evaluation to succeed");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("button".to_string()),
                properties: Default::default(),
            }
        );
    }

    #[test]
    fn eval_source_preserves_external_component_identity_props_and_handlers() {
        let source = r#"
            action SearchRequested = { query:string }
            action DoSearch = { query:string }

            external component <SearchBox placeholder:string = "Find docs" emits { SearchRequested } />
            let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }
        "#;

        let EvalResult::Ok(value) = eval_source(
            source,
            "external-component-root.nx",
            &ProgramBuildContext::empty(),
        ) else {
            panic!("Expected external component evaluation to succeed");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([
                    (
                        "onSearchRequested".to_string(),
                        NxValue::Record {
                            type_name: Some("ActionHandler".to_string()),
                            // Pure evaluation has no snapshot to dispatch against, so the
                            // handler carries the action it accepts and no token.
                            properties: BTreeMap::from([(
                                "action".to_string(),
                                NxValue::String("SearchRequested".to_string()),
                            )]),
                        },
                    ),
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                ]),
            }
        );
    }

    #[test]
    fn eval_source_external_component_state_only_body_omits_state_fields() {
        let source = r#"
            external component <SearchBox placeholder:string = "Find docs" /> = {
              state { query:string }
            }
            let root() = { <SearchBox /> }
        "#;

        let EvalResult::Ok(value) = eval_source(
            source,
            "external-component-state-root.nx",
            &ProgramBuildContext::empty(),
        ) else {
            panic!("Expected external component evaluation with state-only body to succeed");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([(
                    "placeholder".to_string(),
                    NxValue::String("Find docs".to_string()),
                )]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_applies_imported_abstract_base_component_defaults() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                export let defaultPlaceholder(): string = { "Find docs" }
                export abstract component <SearchBase prefix:string = {defaultPlaceholder()} placeholder:string = {prefix} />
            "#,
        )
        .expect("ui file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"
            import "../ui"

            let defaultPlaceholder(): string = { "Wrong" }

            component <SearchBox extends SearchBase /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }

            let root() = { <SearchBox /> }
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected imported inherited component defaults to evaluate");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                    (
                        "prefix".to_string(),
                        NxValue::String("Find docs".to_string()),
                    ),
                ]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_applies_imported_abstract_record_defaults() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                export let defaultPlaceholder(): string = { "Find docs" }
                export abstract type SearchConfigBase = {
                  prefix: string = {defaultPlaceholder()}
                  placeholder: string = {prefix}
                }
            "#,
        )
        .expect("ui file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"
            import "../ui"

            let defaultPlaceholder(): string = { "Wrong" }

            type SearchConfig extends SearchConfigBase = {
              showSearchIcon: boolean = true
            }

            let root() = { <SearchConfig /> }
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected imported inherited record defaults to evaluate");
        };

        assert_eq!(
            value,
            NxValue::Record {
                type_name: Some("SearchConfig".to_string()),
                properties: BTreeMap::from([
                    (
                        "placeholder".to_string(),
                        NxValue::String("Find docs".to_string())
                    ),
                    (
                        "prefix".to_string(),
                        NxValue::String("Find docs".to_string())
                    ),
                    ("showSearchIcon".to_string(), NxValue::Bool(true)),
                ]),
            }
        );
    }

    #[test]
    fn eval_program_artifact_executes_root_across_imported_modules() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("answer.nx"),
            r#"export let answer(): int = { 42 }"#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { answer() }"#;
        fs::write(&main_path, source).expect("main file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");

        let EvalResult::Ok(value) = eval_program_artifact(&program) else {
            panic!("Expected program artifact evaluation to succeed");
        };

        assert_eq!(value, NxValue::Int(42));
    }

    #[test]
    fn eval_source_returns_bare_authored_case_string() {
        let source = r#"
            type Status = active | disabled
            let root() = { Status.active }
        "#;

        let EvalResult::Ok(value) =
            eval_source(source, "enum-root.nx", &ProgramBuildContext::empty())
        else {
            panic!("Expected enum source evaluation to succeed");
        };

        assert_eq!(value, NxValue::String("active".to_string()));
        assert_eq!(value.to_json_string().unwrap(), "\"active\"");
    }

    #[test]
    fn eval_source_returns_payload_union_case_as_type_map() {
        let source = r#"
            type LoadState =
              | idle
              | failed {
                  message: string
                  retryable: boolean = true
                }
            let root(): LoadState = { <LoadState.failed message={"Offline"} /> }
        "#;

        let EvalResult::Ok(value) =
            eval_source(source, "union-root.nx", &ProgramBuildContext::empty())
        else {
            panic!("Expected union source evaluation to succeed");
        };

        let expected = NxValue::Record {
            type_name: Some("LoadState.failed".to_string()),
            properties: BTreeMap::from([
                (
                    "message".to_string(),
                    NxValue::String("Offline".to_string()),
                ),
                ("retryable".to_string(), NxValue::Bool(true)),
            ]),
        };
        assert_eq!(value, expected);
        assert_eq!(
            value.to_json_string().unwrap(),
            r#"{"$type":"LoadState.failed","message":"Offline","retryable":true}"#
        );
        let bytes = value.to_msgpack_vec().unwrap();
        assert_eq!(NxValue::from_msgpack_slice(&bytes).unwrap(), expected);
    }

    #[test]
    fn eval_source_returns_constant_union_case_as_a_bare_string() {
        let source = r#"
            type LoadState = idle | loading
            let root(): LoadState = { LoadState.idle }
        "#;

        let EvalResult::Ok(value) = eval_source(
            source,
            "fieldless-union-root.nx",
            &ProgramBuildContext::empty(),
        ) else {
            panic!("Expected fieldless union source evaluation to succeed");
        };

        // A constant case carries nothing beyond its name, so it serializes as that bare string
        // rather than as a `$type` map. **BREAKING** relative to the record representation.
        let expected = NxValue::String("idle".to_string());
        assert_eq!(value, expected);
        assert_eq!(value.to_json_string().unwrap(), r#""idle""#);
        let bytes = value.to_msgpack_vec().unwrap();
        assert_eq!(NxValue::from_msgpack_slice(&bytes).unwrap(), expected);
    }
}
