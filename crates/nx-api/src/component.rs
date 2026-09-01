use crate::artifacts::{ProgramArtifact, ProgramBuildContext};
use crate::eval::{
    build_source_program_artifact, program_artifact_error_diagnostics, program_root_source,
    runtime_error_diagnostics,
};
use crate::value::{from_nx_value, to_nx_value};
use crate::{NxDiagnostic, NxSeverity};
use nx_interpreter::Interpreter;
use nx_value::NxValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
enum ComponentLookup<'a> {
    Program(&'a ProgramArtifact),
}

/// The result of initializing a component from source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInitResult {
    /// Rendered component body converted to the public value model.
    pub rendered: NxValue,
    /// Opaque host-owned component state snapshot.
    ///
    /// Reuse this snapshot only with the exact same source text revision that produced it.
    #[serde(with = "serde_bytes")]
    pub state_snapshot: Vec<u8>,
}

/// The result of evaluating a component from explicit props and host-owned explicit state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentEvaluateResult {
    /// Rendered component body converted to the public value model.
    ///
    /// Evaluation returns this value directly through raw FFI formats; it does not include
    /// lifecycle state snapshots, effects, or wrapper fields.
    pub rendered: NxValue,
}

/// The result of dispatching actions against a component state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDispatchResult {
    /// Effect actions returned in dispatch order.
    pub effects: Vec<NxValue>,
    /// Opaque host-owned component state snapshot.
    ///
    /// Reuse this snapshot only with the exact same source text revision that produced it.
    #[serde(with = "serde_bytes")]
    pub state_snapshot: Vec<u8>,
}

/// Result of initializing a component from source text.
pub enum ComponentInitEvalResult {
    /// Initialization succeeded.
    Ok(ComponentInitResult),
    /// Initialization failed with diagnostics.
    Err(Vec<NxDiagnostic>),
}

/// Result of evaluating a component from explicit props and explicit state.
pub enum ComponentEvaluateEvalResult {
    /// Evaluation succeeded.
    Ok(ComponentEvaluateResult),
    /// Evaluation failed with diagnostics.
    Err(Vec<NxDiagnostic>),
}

/// Result of dispatching actions from source text.
pub enum ComponentDispatchEvalResult {
    /// Dispatch succeeded.
    Ok(ComponentDispatchResult),
    /// Dispatch failed with diagnostics.
    Err(Vec<NxDiagnostic>),
}

/// Runs shared static analysis and then initializes a named component from source text using a
/// caller-supplied build context.
///
/// The returned state snapshot is opaque host-owned data and is only valid with the exact same
/// source text revision that produced it.
pub fn initialize_component_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
    component_name: &str,
    props: &NxValue,
) -> ComponentInitEvalResult {
    let program = match build_source_program_artifact(source, file_name, build_context) {
        Ok(program) => program,
        Err(diagnostics) => return ComponentInitEvalResult::Err(diagnostics),
    };

    initialize_component_program_artifact_with_source(&program, source, component_name, props)
}

/// Initializes a named component from a resolved [`ProgramArtifact`].
pub fn initialize_component_program_artifact(
    program: &ProgramArtifact,
    component_name: &str,
    props: &NxValue,
) -> ComponentInitEvalResult {
    let source = program_root_source(program);
    initialize_component_program_artifact_with_source(program, &source, component_name, props)
}

fn initialize_component_program_artifact_with_source(
    program: &ProgramArtifact,
    source: &str,
    component_name: &str,
    props: &NxValue,
) -> ComponentInitEvalResult {
    if let Some(diagnostics) = program_artifact_error_diagnostics(program, source) {
        return ComponentInitEvalResult::Err(diagnostics);
    }

    if let Err(message) = validate_host_input_value(ComponentLookup::Program(program), props) {
        return ComponentInitEvalResult::Err(invalid_input_diagnostics(message));
    }

    let props = match from_nx_value(props) {
        Ok(props) => props,
        Err(error) => return ComponentInitEvalResult::Err(invalid_input_diagnostics(error)),
    };

    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    match interpreter.initialize_resolved_component(component_name, props) {
        Ok(result) => ComponentInitEvalResult::Ok(ComponentInitResult {
            rendered: to_nx_value(&result.rendered),
            state_snapshot: result.state_snapshot,
        }),
        Err(error) => ComponentInitEvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

/// Runs shared static analysis and then evaluates a named component from source text using a
/// caller-supplied build context.
///
/// The caller owns the supplied current state. Evaluation is pure: it does not create snapshots,
/// dispatch actions, invoke handlers, or return effects.
pub fn evaluate_component_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
    component_name: &str,
    props: &NxValue,
    state: &NxValue,
) -> ComponentEvaluateEvalResult {
    let program = match build_source_program_artifact(source, file_name, build_context) {
        Ok(program) => program,
        Err(diagnostics) => return ComponentEvaluateEvalResult::Err(diagnostics),
    };

    evaluate_component_program_artifact_with_source(&program, source, component_name, props, state)
}

/// Evaluates a named component from a resolved [`ProgramArtifact`] using explicit props and
/// host-owned current state.
///
/// Successful evaluation returns the rendered component body only.
pub fn evaluate_component_program_artifact(
    program: &ProgramArtifact,
    component_name: &str,
    props: &NxValue,
    state: &NxValue,
) -> ComponentEvaluateEvalResult {
    let source = program_root_source(program);
    evaluate_component_program_artifact_with_source(program, &source, component_name, props, state)
}

fn evaluate_component_program_artifact_with_source(
    program: &ProgramArtifact,
    source: &str,
    component_name: &str,
    props: &NxValue,
    state: &NxValue,
) -> ComponentEvaluateEvalResult {
    if let Some(diagnostics) = program_artifact_error_diagnostics(program, source) {
        return ComponentEvaluateEvalResult::Err(diagnostics);
    }

    if let Err(message) = validate_host_input_value(ComponentLookup::Program(program), props) {
        return ComponentEvaluateEvalResult::Err(invalid_input_diagnostics(message));
    }
    if let Err(message) = validate_host_input_value(ComponentLookup::Program(program), state) {
        return ComponentEvaluateEvalResult::Err(invalid_input_diagnostics(message));
    }

    let props = match from_nx_value(props) {
        Ok(props) => props,
        Err(error) => return ComponentEvaluateEvalResult::Err(invalid_input_diagnostics(error)),
    };
    let state = match from_nx_value(state) {
        Ok(state) => state,
        Err(error) => return ComponentEvaluateEvalResult::Err(invalid_input_diagnostics(error)),
    };

    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    match interpreter.evaluate_resolved_component(component_name, props, state) {
        Ok(result) => ComponentEvaluateEvalResult::Ok(ComponentEvaluateResult {
            rendered: to_nx_value(&result.rendered),
        }),
        Err(error) => ComponentEvaluateEvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

/// Runs shared static analysis and then dispatches a batch of actions against a component state
/// snapshot.
///
/// The provided snapshot must come from [`initialize_component_source`] or an earlier dispatch
/// against the exact same source text revision.
pub fn dispatch_component_actions_source(
    source: &str,
    file_name: &str,
    build_context: &ProgramBuildContext,
    state_snapshot: &[u8],
    actions: &[NxValue],
) -> ComponentDispatchEvalResult {
    let program = match build_source_program_artifact(source, file_name, build_context) {
        Ok(program) => program,
        Err(diagnostics) => return ComponentDispatchEvalResult::Err(diagnostics),
    };

    dispatch_component_actions_program_artifact_with_source(
        &program,
        source,
        state_snapshot,
        actions,
    )
}

/// Dispatches actions against a component snapshot produced by a resolved [`ProgramArtifact`].
pub fn dispatch_component_actions_program_artifact(
    program: &ProgramArtifact,
    state_snapshot: &[u8],
    actions: &[NxValue],
) -> ComponentDispatchEvalResult {
    let source = program_root_source(program);
    dispatch_component_actions_program_artifact_with_source(
        program,
        &source,
        state_snapshot,
        actions,
    )
}

fn dispatch_component_actions_program_artifact_with_source(
    program: &ProgramArtifact,
    source: &str,
    state_snapshot: &[u8],
    actions: &[NxValue],
) -> ComponentDispatchEvalResult {
    if let Some(diagnostics) = program_artifact_error_diagnostics(program, source) {
        return ComponentDispatchEvalResult::Err(diagnostics);
    }

    for (index, action) in actions.iter().enumerate() {
        if let Err(message) = validate_dispatch_action_input(
            ComponentLookup::Program(program),
            action,
            &format!("$[{index}]"),
        ) {
            return ComponentDispatchEvalResult::Err(invalid_input_diagnostics(message));
        }
    }

    let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
    let actions = match actions
        .iter()
        .map(from_nx_value)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(actions) => actions,
        Err(error) => return ComponentDispatchEvalResult::Err(invalid_input_diagnostics(error)),
    };

    match interpreter.dispatch_resolved_component_actions(state_snapshot, actions) {
        Ok(result) => ComponentDispatchEvalResult::Ok(ComponentDispatchResult {
            effects: result.effects.iter().map(to_nx_value).collect(),
            state_snapshot: result.state_snapshot,
        }),
        Err(error) => ComponentDispatchEvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

pub(crate) fn invalid_input_diagnostics(message: impl ToString) -> Vec<NxDiagnostic> {
    vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("invalid-input".to_string()),
        message: message.to_string(),
        labels: Vec::new(),
        help: None,
        note: None,
    }]
}

fn validate_host_input_value(lookup: ComponentLookup<'_>, value: &NxValue) -> Result<(), String> {
    validate_host_input_value_at_path(lookup, value, "$")
}

fn validate_dispatch_action_input(
    lookup: ComponentLookup<'_>,
    value: &NxValue,
    path: &str,
) -> Result<(), String> {
    if let NxValue::Record {
        type_name: None, ..
    } = value
    {
        return Err(format!("action record at {path} must have a '$type' field"));
    }

    validate_host_input_value_at_path(lookup, value, path)
}

fn validate_host_input_value_at_path(
    lookup: ComponentLookup<'_>,
    value: &NxValue,
    path: &str,
) -> Result<(), String> {
    match value {
        NxValue::Null
        | NxValue::Bool(_)
        | NxValue::Int32(_)
        | NxValue::Int(_)
        | NxValue::Float32(_)
        | NxValue::Float(_)
        | NxValue::String(_) => Ok(()),
        NxValue::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_host_input_value_at_path(lookup, value, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        NxValue::Record {
            type_name,
            properties,
        } => {
            if let Some(type_name) = type_name {
                if lookup_contains_component(lookup, type_name) {
                    return Err(format!(
                        "NxValue at {path} uses component type '{type_name}', but component values \
                         are output-only and cannot be provided as host input"
                    ));
                }
            }

            for (key, value) in properties {
                validate_host_input_value_at_path(lookup, value, &format!("{path}.{key}"))?;
            }

            Ok(())
        }
    }
}

fn lookup_contains_component(lookup: ComponentLookup<'_>, type_name: &str) -> bool {
    match lookup {
        ComponentLookup::Program(program) => program
            .resolved_program
            .entry_component(type_name)
            .is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{
        build_program_artifact_from_source, build_workspace_program_artifact, LibraryRegistry,
        ProgramBuildContext,
    };
    use crate::{NxWorkspace, NxWorkspaceModule};
    use nx_hir::{ast::Expr, Item, LoweredModule};
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn empty_record() -> NxValue {
        NxValue::Record {
            type_name: None,
            properties: BTreeMap::new(),
        }
    }

    fn static_analysis_failure_source() -> &'static str {
        r#"
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

            component <SearchBox /> = {
              <TextInput />
            }
        "#
    }

    fn assert_static_analysis_diagnostics(diagnostics: &[NxDiagnostic]) {
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("lowering-error")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("return-type-mismatch")));
    }

    #[test]
    fn initialize_component_source_returns_rendered_output_and_state_snapshot() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#;

        let result = initialize_component_source(
            source,
            "component-init.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected source-based component initialization to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered element record");
        };
        assert_eq!(type_name.as_deref(), Some("TextInput"));
        assert_eq!(
            properties.get("value"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert_eq!(
            properties.get("placeholder"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn evaluate_component_program_artifact_returns_rendered_output() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" /> = {
              state { query:string }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#;
        let program = build_program_artifact_from_source(
            source,
            "component-evaluate-artifact.nx",
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact");
        let state = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };

        let result =
            evaluate_component_program_artifact(&program, "SearchBox", &empty_record(), &state);
        let ComponentEvaluateEvalResult::Ok(result) = result else {
            panic!("Expected artifact component evaluation to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered element record");
        };
        assert_eq!(type_name.as_deref(), Some("TextInput"));
        assert_eq!(
            properties.get("value"),
            Some(&NxValue::String("docs".to_string()))
        );
        assert_eq!(
            properties.get("placeholder"),
            Some(&NxValue::String("Find docs".to_string()))
        );
    }

    #[test]
    fn evaluate_component_source_returns_static_diagnostics_before_runtime_work() {
        let result = evaluate_component_source(
            static_analysis_failure_source(),
            "component-evaluate-static-errors.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
            &empty_record(),
        );
        let ComponentEvaluateEvalResult::Err(diagnostics) = result else {
            panic!("Expected evaluation to stop on static analysis diagnostics");
        };

        assert_static_analysis_diagnostics(&diagnostics);
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("runtime-error")));
    }

    #[test]
    fn evaluate_component_program_artifact_resolves_imported_library_component() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("search-box.nx"),
            r#"
                export component <SearchBox placeholder:string = "Find docs" /> = {
                  state { query:string }
                  <TextInput value={query} placeholder={placeholder} />
                }
            "#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { 0 }"#;
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
        let state = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };

        let result =
            evaluate_component_program_artifact(&program, "SearchBox", &empty_record(), &state);
        let ComponentEvaluateEvalResult::Ok(result) = result else {
            panic!("Expected imported component evaluation to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered element record");
        };
        assert_eq!(type_name.as_deref(), Some("TextInput"));
        assert_eq!(
            properties.get("value"),
            Some(&NxValue::String("docs".to_string()))
        );
    }

    #[test]
    fn evaluate_component_source_returns_json_compatible_constant_case_output() {
        let source = r#"
            type ThemeMode = light | dark

            component <ThemeView /> = {
              state { theme:ThemeMode }
              <ThemePanel mode={theme} />
            }
        "#;
        let state = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "theme".to_string(),
                NxValue::String("dark".to_string()),
            )]),
        };

        let result = evaluate_component_source(
            source,
            "component-evaluate-enum.nx",
            &ProgramBuildContext::empty(),
            "ThemeView",
            &empty_record(),
            &state,
        );
        let ComponentEvaluateEvalResult::Ok(result) = result else {
            panic!("Expected source component evaluation to succeed");
        };
        assert_eq!(
            result.rendered,
            NxValue::Record {
                type_name: Some("ThemePanel".to_string()),
                properties: BTreeMap::from([(
                    "mode".to_string(),
                    NxValue::String("dark".to_string()),
                )]),
            }
        );

        let json = result
            .rendered
            .to_json_string()
            .expect("Expected rendered value to serialize as JSON");
        assert!(json.contains(r#""$type":"ThemePanel""#));
        assert!(json.contains(r#""mode":"dark""#));
        assert!(
            !json.contains(r#""rendered""#),
            "Evaluation JSON must be the rendered value directly"
        );
    }

    #[test]
    fn evaluate_component_source_preserves_polymorphic_record_output() {
        let source = r#"
            abstract type Question = {
              label:string
            }

            type ShortTextQuestion extends Question = {
              placeholder:string
            }

            component <Flow /> = {
              state { label:string }
              <ShortTextQuestion label={label} placeholder="Enter your name" />
            }
        "#;
        let state = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "label".to_string(),
                NxValue::String("Name".to_string()),
            )]),
        };

        let result = evaluate_component_source(
            source,
            "component-evaluate-polymorphic.nx",
            &ProgramBuildContext::empty(),
            "Flow",
            &empty_record(),
            &state,
        );
        let ComponentEvaluateEvalResult::Ok(result) = result else {
            panic!("Expected polymorphic component evaluation to succeed");
        };

        assert_eq!(
            result.rendered,
            NxValue::Record {
                type_name: Some("ShortTextQuestion".to_string()),
                properties: BTreeMap::from([
                    ("label".to_string(), NxValue::String("Name".to_string()),),
                    (
                        "placeholder".to_string(),
                        NxValue::String("Enter your name".to_string()),
                    ),
                ]),
            }
        );
    }

    #[test]
    fn initialize_component_source_supports_external_component_entrypoint() {
        let source = r#"
            external component <SearchBox placeholder:string = "Find docs" showSearchIcon:boolean = true />
        "#;

        let result = initialize_component_source(
            source,
            "external-component-init.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected external component initialization to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered external component record");
        };
        assert_eq!(type_name.as_deref(), Some("SearchBox"));
        assert_eq!(
            properties.get("placeholder"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert_eq!(properties.get("showSearchIcon"), Some(&NxValue::Bool(true)));
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn initialize_component_source_supports_external_component_state_only_entrypoint() {
        let source = r#"
            external component <SearchBox placeholder:string = "Find docs" /> = {
              state { query:string }
            }
        "#;

        let result = initialize_component_source(
            source,
            "external-component-state-init.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected external component state-only initialization to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered external component record");
        };
        assert_eq!(type_name.as_deref(), Some("SearchBox"));
        assert_eq!(
            properties.get("placeholder"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert!(
            !properties.contains_key("query"),
            "Declared external state must not be returned as an invocable prop"
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn initialize_component_source_rejects_external_state_as_host_prop() {
        let source = r#"
            external component <SearchBox /> = {
              state { query:string }
            }
        "#;

        let result = initialize_component_source(
            source,
            "external-component-state-prop.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &NxValue::Record {
                type_name: None,
                properties: BTreeMap::from([(
                    "query".to_string(),
                    NxValue::String("docs".to_string()),
                )]),
            },
        );
        let ComponentInitEvalResult::Err(diagnostics) = result else {
            panic!("Expected external state-as-prop initialization to fail");
        };

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown prop 'query'")),
            "Expected unknown-prop runtime diagnostic, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn initialize_component_source_rejects_abstract_component_entrypoint() {
        let source = r#"
            abstract component <SearchBase placeholder:string />
        "#;

        let result = initialize_component_source(
            source,
            "abstract-component-init.nx",
            &ProgramBuildContext::empty(),
            "SearchBase",
            &empty_record(),
        );
        let ComponentInitEvalResult::Err(diagnostics) = result else {
            panic!("Expected abstract component initialization to fail");
        };

        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("Cannot instantiate abstract component 'SearchBase'")),
            "Expected abstract component runtime diagnostic, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn dispatch_component_actions_source_round_trips_effects_and_snapshot() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let withHandler() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#;

        let program = build_program_artifact_from_source(
            source,
            "component-dispatch.nx",
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let props = interpreter
            .execute_resolved_program_function("withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(root_module.as_ref(), "SearchBox", props)
            .expect("Expected interpreter initialization to succeed");

        let action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([(
                "searchString".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };
        let result = dispatch_component_actions_source(
            source,
            "component-dispatch.nx",
            &ProgramBuildContext::empty(),
            &init.state_snapshot,
            &[action],
        );
        let ComponentDispatchEvalResult::Ok(result) = result else {
            panic!("Expected source-based dispatch to succeed");
        };

        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0],
            NxValue::Record {
                type_name: Some("DoSearch".to_string()),
                properties: BTreeMap::from([(
                    "search".to_string(),
                    NxValue::String("docs".to_string()),
                )]),
            }
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn initialize_component_source_round_trips_constant_case_props_in_rendered_output() {
        let source = r#"
            type ThemeMode = light | dark

            external component <SearchBox theme:ThemeMode />
        "#;

        let props = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "theme".to_string(),
                NxValue::String("light".to_string()),
            )]),
        };

        let result = initialize_component_source(
            source,
            "component-enum-props.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &props,
        );
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected enum-bearing component initialization to succeed");
        };

        assert_eq!(
            result.rendered,
            NxValue::Record {
                type_name: Some("SearchBox".to_string()),
                properties: BTreeMap::from([(
                    "theme".to_string(),
                    NxValue::String("light".to_string()),
                )]),
            }
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn initialize_component_source_rejects_unknown_union_case_in_prop() {
        let source = r#"
            type ThemeMode = light | dark

            external component <SearchBox theme:ThemeMode />
        "#;

        let props = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "theme".to_string(),
                NxValue::String("sparkly".to_string()),
            )]),
        };

        let result = initialize_component_source(
            source,
            "component-enum-unknown.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &props,
        );
        let ComponentInitEvalResult::Err(diagnostics) = result else {
            panic!("Expected an unknown union case to be rejected");
        };

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("unknown union case 'sparkly'")),
            "Expected unknown-union-case diagnostic, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn dispatch_component_actions_source_round_trips_constant_case_effects_and_snapshot() {
        let source = r#"
            type ThemeMode = light | dark

            action SearchSubmitted = { theme:ThemeMode }
            action DoSearch = { theme:ThemeMode }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let withHandler() = { <SearchBox onSearchSubmitted=<DoSearch theme={action.theme} /> /> }
        "#;

        let program = build_program_artifact_from_source(
            source,
            "component-enum-dispatch.nx",
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let props = interpreter
            .execute_resolved_program_function("withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(root_module.as_ref(), "SearchBox", props)
            .expect("Expected interpreter initialization to succeed");

        let action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([(
                "theme".to_string(),
                NxValue::String("dark".to_string()),
            )]),
        };
        let result = dispatch_component_actions_source(
            source,
            "component-enum-dispatch.nx",
            &ProgramBuildContext::empty(),
            &init.state_snapshot,
            &[action],
        );
        let ComponentDispatchEvalResult::Ok(result) = result else {
            panic!("Expected enum-bearing source-based dispatch to succeed");
        };

        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0],
            NxValue::Record {
                type_name: Some("DoSearch".to_string()),
                properties: BTreeMap::from([(
                    "theme".to_string(),
                    NxValue::String("dark".to_string()),
                )]),
            }
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn dispatch_component_actions_source_supports_external_component_handlers() {
        let source = r#"
            action SearchRequested = { query:string }
            action DoSearch = { query:string }

            external component <SearchBox emits { SearchRequested } /> = {
              state { query:string }
            }

            let withHandler() = <SearchBox onSearchRequested=<DoSearch query={action.query} /> />
        "#;

        let program = build_program_artifact_from_source(
            source,
            "external-component-dispatch.nx",
            &ProgramBuildContext::empty(),
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let props = interpreter
            .execute_resolved_program_function("withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(root_module.as_ref(), "SearchBox", props)
            .expect("Expected external interpreter initialization to succeed");

        let action = NxValue::Record {
            type_name: Some("SearchRequested".to_string()),
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };
        let result = dispatch_component_actions_source(
            source,
            "external-component-dispatch.nx",
            &ProgramBuildContext::empty(),
            &init.state_snapshot,
            &[action],
        );
        let ComponentDispatchEvalResult::Ok(result) = result else {
            panic!("Expected external source-based dispatch to succeed");
        };

        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0],
            NxValue::Record {
                type_name: Some("DoSearch".to_string()),
                properties: BTreeMap::from([(
                    "query".to_string(),
                    NxValue::String("docs".to_string()),
                )]),
            }
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn dispatch_component_actions_source_returns_runtime_diagnostics_for_invalid_snapshot() {
        let source = r#"
            component <Button text:string /> = {
              <button>{text}</button>
            }
        "#;

        let result = dispatch_component_actions_source(
            source,
            "invalid-snapshot.nx",
            &ProgramBuildContext::empty(),
            b"nope",
            &[],
        );
        let ComponentDispatchEvalResult::Err(diagnostics) = result else {
            panic!("Expected invalid snapshot dispatch to fail");
        };

        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("Invalid component state snapshot")),
            "Expected invalid snapshot diagnostic, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn initialize_component_source_returns_aggregated_static_diagnostics_before_runtime_work() {
        let result = initialize_component_source(
            static_analysis_failure_source(),
            "component-static-errors.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Err(diagnostics) = result else {
            panic!("Expected initialization to stop on static analysis diagnostics");
        };

        assert_static_analysis_diagnostics(&diagnostics);
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_deref() == Some("runtime-error")));
    }

    #[test]
    fn dispatch_component_actions_source_returns_static_diagnostics_before_snapshot_validation() {
        let result = dispatch_component_actions_source(
            static_analysis_failure_source(),
            "component-static-errors.nx",
            &ProgramBuildContext::empty(),
            b"nope",
            &[],
        );
        let ComponentDispatchEvalResult::Err(diagnostics) = result else {
            panic!("Expected dispatch to stop on static analysis diagnostics");
        };

        assert_static_analysis_diagnostics(&diagnostics);
        assert!(!diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("Invalid component state snapshot")));
    }

    #[test]
    fn initialize_component_source_rejects_component_records_in_host_props() {
        let source = r#"
            component <SearchBox /> = {
              <TextInput />
            }

            component <Wrapper child:object /> = {
              child
            }
        "#;

        let props = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "child".to_string(),
                NxValue::Record {
                    type_name: Some("SearchBox".to_string()),
                    properties: BTreeMap::new(),
                },
            )]),
        };

        let result = initialize_component_source(
            source,
            "component-props.nx",
            &ProgramBuildContext::empty(),
            "Wrapper",
            &props,
        );
        let ComponentInitEvalResult::Err(diagnostics) = result else {
            panic!("Expected component-shaped host props to be rejected");
        };

        assert_eq!(diagnostics[0].code.as_deref(), Some("invalid-input"));
        assert!(diagnostics[0].message.contains("SearchBox"));
        assert!(diagnostics[0].message.contains("$.child"));
    }

    #[test]
    fn dispatch_component_actions_source_rejects_component_records_in_action_payloads() {
        let source = r#"
            action SearchSubmitted = { payload:object }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let init = initialize_component_source(
            source,
            "component-actions.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(init) = init else {
            panic!("Expected component initialization to succeed");
        };

        let action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([(
                "payload".to_string(),
                NxValue::Record {
                    type_name: Some("SearchBox".to_string()),
                    properties: BTreeMap::new(),
                },
            )]),
        };

        let result = dispatch_component_actions_source(
            source,
            "component-actions.nx",
            &ProgramBuildContext::empty(),
            &init.state_snapshot,
            &[action],
        );
        let ComponentDispatchEvalResult::Err(diagnostics) = result else {
            panic!("Expected component-shaped action payloads to be rejected");
        };

        assert_eq!(diagnostics[0].code.as_deref(), Some("invalid-input"));
        assert!(diagnostics[0].message.contains("SearchBox"));
        assert!(diagnostics[0].message.contains("$[0].payload"));
    }

    #[test]
    fn dispatch_component_actions_source_rejects_untyped_action_records() {
        let source = r#"
            action SearchSubmitted = { searchString:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }
        "#;

        let init = initialize_component_source(
            source,
            "untyped-action.nx",
            &ProgramBuildContext::empty(),
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(init) = init else {
            panic!("Expected component initialization to succeed");
        };

        let action = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "searchString".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };

        let result = dispatch_component_actions_source(
            source,
            "untyped-action.nx",
            &ProgramBuildContext::empty(),
            &init.state_snapshot,
            &[action],
        );
        let ComponentDispatchEvalResult::Err(diagnostics) = result else {
            panic!("Expected untyped action records to be rejected");
        };

        assert_eq!(diagnostics[0].code.as_deref(), Some("invalid-input"));
        assert!(diagnostics[0].message.contains("$[0]"));
        assert!(diagnostics[0].message.contains("$type"));
    }

    #[test]
    fn initialize_component_program_artifact_resolves_imported_library_component() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("search-box.nx"),
            r#"
                export component <SearchBox placeholder:string = "Find docs" /> = {
                  state { query:string = {placeholder} }
                  <TextInput value={query} placeholder={placeholder} />
                }
            "#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { 0 }"#;
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
        let result = initialize_component_program_artifact(&program, "SearchBox", &empty_record());
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected imported component initialization to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered element record");
        };
        assert_eq!(type_name.as_deref(), Some("TextInput"));
        assert_eq!(
            properties.get("value"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn imported_component_handler_bindings_from_abstract_base_lower_and_dispatch() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                export abstract component <SearchBase emits { SearchRequested { query:string } } />
            "#,
        )
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        let source = r#"
            import "../ui"

            action DoSearch = { query:string }

            component <SearchBox extends SearchBase /> = {
              <TextInput />
            }

            let withHandler() = <SearchBox onSearchRequested=<DoSearch query={action.query} /> />
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");

        let handler_property = root_module
            .items()
            .iter()
            .find_map(|item| match item {
                Item::Function(function) if function.name.as_str() == "withHandler" => {
                    match root_module.expr(function.body) {
                        Expr::Element { element, .. } => {
                            root_module.element(*element).properties.first()
                        }
                        _ => None,
                    }
                }
                _ => None,
            })
            .expect("Expected withHandler property");
        match root_module.expr(handler_property.value) {
            Expr::ActionHandler {
                component,
                emit,
                action_name,
                ..
            } => {
                assert_eq!(component.as_str(), "SearchBox");
                assert_eq!(emit.as_str(), "SearchRequested");
                assert_eq!(action_name.as_str(), "SearchBase.SearchRequested");
            }
            other => panic!(
                "Expected imported handler binding to lower as ActionHandler, got {:?}",
                other
            ),
        }

        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let props = interpreter
            .execute_resolved_program_function("withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(root_module.as_ref(), "SearchBox", props)
            .expect("Expected component initialization to succeed");

        let action = NxValue::Record {
            type_name: Some("SearchBase.SearchRequested".to_string()),
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };
        let result =
            dispatch_component_actions_program_artifact(&program, &init.state_snapshot, &[action]);
        let ComponentDispatchEvalResult::Ok(result) = result else {
            match result {
                ComponentDispatchEvalResult::Err(diagnostics) => {
                    panic!(
                        "Expected imported inherited handler dispatch to succeed, got {:?}",
                        diagnostics
                            .iter()
                            .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                            .collect::<Vec<_>>()
                    );
                }
                ComponentDispatchEvalResult::Ok(_) => unreachable!(),
            }
        };

        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0],
            NxValue::Record {
                type_name: Some("DoSearch".to_string()),
                properties: BTreeMap::from([(
                    "query".to_string(),
                    NxValue::String("docs".to_string()),
                )]),
            }
        );
    }

    #[test]
    fn imported_abstract_base_component_defaults_apply_during_initialization() {
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
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
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
        let result = initialize_component_program_artifact(&program, "SearchBox", &empty_record());
        let ComponentInitEvalResult::Ok(result) = result else {
            panic!("Expected imported inherited default initialization to succeed");
        };

        let NxValue::Record {
            type_name,
            properties,
        } = result.rendered
        else {
            panic!("Expected rendered element record");
        };
        assert_eq!(type_name.as_deref(), Some("TextInput"));
        assert_eq!(
            properties.get("placeholder"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert_eq!(
            properties.get("value"),
            Some(&NxValue::String("Find docs".to_string()))
        );
        assert!(!result.state_snapshot.is_empty());
    }

    #[test]
    fn dispatch_component_actions_source_keeps_loaded_snapshot_after_disk_changes() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        let library_path = ui_dir.join("search-box.nx");
        fs::write(
            &library_path,
            r#"
                action SearchSubmitted = { searchString:string }

                export component <SearchBox emits { SearchSubmitted } /> = {
                  <TextInput />
                }
            "#,
        )
        .expect("ui file");
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { 0 }"#;
        fs::write(&main_path, source).expect("main file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected registry preload");
        let build_context = registry.build_context();

        let init = initialize_component_source(
            source,
            &main_path.display().to_string(),
            &build_context,
            "SearchBox",
            &empty_record(),
        );
        let ComponentInitEvalResult::Ok(init) = init else {
            panic!("Expected imported component initialization to succeed");
        };

        fs::write(
            &library_path,
            r#"
                action SearchSubmitted = { searchString:string }

                export component <SearchBox placeholder:string = "Updated" emits { SearchSubmitted } /> = {
                  <TextInput placeholder={placeholder} />
                }
            "#,
        )
        .expect("ui file update");

        let result = dispatch_component_actions_source(
            source,
            &main_path.display().to_string(),
            &build_context,
            &init.state_snapshot,
            &[],
        );
        let ComponentDispatchEvalResult::Ok(result) = result else {
            panic!("Expected dispatch to keep using the loaded snapshot");
        };
        assert!(!result.state_snapshot.is_empty());
    }

    /// Builds a workspace whose library declares an action the app also declares under the same
    /// name, with a handler bound in the app for the library's emit.
    fn handler_collision_workspace(
        temp: &TempDir,
        library_action: &str,
        app_source: &str,
    ) -> (ProgramArtifact, Arc<LoweredModule>) {
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            format!(
                r#"
                {library_action}

                export abstract component <SearchBase emits {{ SearchSubmitted }} />
            "#
            ),
        )
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        fs::write(&main_path, app_source).expect("main file");

        let program = build_program_artifact_from_source(
            app_source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");
        (program, root_module)
    }

    fn dispatch_host_action(
        program: &ProgramArtifact,
        root_module: &LoweredModule,
        action: NxValue,
    ) -> ComponentDispatchEvalResult {
        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let props = interpreter
            .execute_resolved_program_function("withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(root_module, "SearchBox", props)
            .expect("Expected component initialization to succeed");
        dispatch_component_actions_program_artifact(program, &init.state_snapshot, &[action])
    }

    const HANDLER_APP_SOURCE: &str = r#"
            import "../ui"

            action SearchSubmitted = { query:int }
            action DoSearch = { query:string }

            component <SearchBox extends SearchBase /> = {
              <TextInput />
            }

            let withHandler() = <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> />
        "#;

    #[test]
    fn a_host_action_is_validated_against_the_declaration_the_component_emits() {
        let temp = TempDir::new().expect("temp dir");
        let (program, root_module) = handler_collision_workspace(
            &temp,
            "action SearchSubmitted = { query:string }",
            HANDLER_APP_SOURCE,
        );

        // The app declares its own `SearchSubmitted` with an `int` field. Host input must be
        // checked against the library's declaration — the one `SearchBase` actually emits — so an
        // `int` reaching a field the library typed `string` is rejected.
        let action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([("query".to_string(), NxValue::Int(7))]),
        };
        let ComponentDispatchEvalResult::Err(diagnostics) =
            dispatch_host_action(&program, root_module.as_ref(), action)
        else {
            panic!("Expected host input to be rejected against the emitted action declaration");
        };

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected string, got int")),
            "Expected a field type mismatch against the library declaration, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_host_action_matching_the_emitted_declaration_is_still_accepted() {
        let temp = TempDir::new().expect("temp dir");
        let (program, root_module) = handler_collision_workspace(
            &temp,
            "action SearchSubmitted = { query:string }",
            HANDLER_APP_SOURCE,
        );

        // The same collision, with host input that matches the library's declaration. Resolving the
        // expectation in the declaring module must not reject what that declaration accepts.
        let action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };
        let result = dispatch_host_action(&program, root_module.as_ref(), action);
        let ComponentDispatchEvalResult::Ok(result) = result else {
            let ComponentDispatchEvalResult::Err(diagnostics) = result else {
                unreachable!()
            };
            panic!(
                "Expected host input matching the emitted declaration to be accepted, got {:?}",
                diagnostics
                    .iter()
                    .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                    .collect::<Vec<_>>()
            );
        };

        assert_eq!(result.effects.len(), 1);
    }

    /// Builds a workspace whose `widgets.nx` declares a component prop typed by an abstract base
    /// that a third module extends.
    ///
    /// `shapes` decides whether `Circle` extends the base `Draw` expects or a same-named local one,
    /// which is the only difference between the two lineage cases below.
    fn prop_lineage_workspace(shapes: &str) -> ProgramArtifact {
        let workspace = NxWorkspace::new(vec![
            NxWorkspaceModule::from_utf8("shapes.nx", shapes.as_bytes().to_vec())
                .expect("shapes module"),
            NxWorkspaceModule::from_utf8(
                "widgets.nx",
                br#"import { Circle } from "./shapes.nx"

export abstract type Shape = { label: string? }

export component <Draw s: Shape /> = { <div /> }"#
                    .to_vec(),
            )
            .expect("widgets module"),
            NxWorkspaceModule::from_utf8(
                "app.nx",
                br#"import { Draw } from "./widgets.nx"

let root() = { 0 }"#
                    .to_vec(),
            )
            .expect("app module"),
        ])
        .expect("workspace");

        build_workspace_program_artifact(&workspace, "app.nx", &ProgramBuildContext::empty())
            .expect("Expected workspace program artifact")
    }

    fn circle_props() -> NxValue {
        NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "s".to_string(),
                NxValue::Record {
                    type_name: Some("Circle".to_string()),
                    properties: BTreeMap::from([("r".to_string(), NxValue::Int(1))]),
                },
            )]),
        }
    }

    /// A host-supplied type name is a label, and the declaration it reaches decides the match.
    ///
    /// `shapes.nx` declares its own `Shape` and extends that. The lineage therefore reaches a
    /// `Shape` sharing nothing but a spelling with the one `Draw` expects — the defect the static
    /// half of this change closes, at the one boundary analysis cannot see. Both bases declare the
    /// same field, so only declaring origin distinguishes them.
    #[test]
    fn a_host_record_whose_lineage_only_shares_a_name_with_the_expected_base_is_rejected() {
        let program = prop_lineage_workspace(
            r#"abstract type Shape = { label: string? }

export type Circle extends Shape = { r: int }"#,
        );

        let ComponentInitEvalResult::Err(diagnostics) =
            initialize_component_program_artifact(&program, "Draw", &circle_props())
        else {
            panic!(
                "Expected a lineage that only shares a name with the expected base to be rejected"
            );
        };

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Type mismatch")),
            "Expected a type mismatch for the same-named ancestor, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                .collect::<Vec<_>>()
        );
    }

    /// The positive half: walking the chain by declaration must still cross a module boundary.
    #[test]
    fn a_host_record_extending_the_expected_base_in_another_module_is_accepted() {
        let program = prop_lineage_workspace(
            r#"import { Shape } from "./widgets.nx"

export type Circle extends Shape = { r: int }"#,
        );

        let result = initialize_component_program_artifact(&program, "Draw", &circle_props());
        let ComponentInitEvalResult::Ok(_) = result else {
            let ComponentInitEvalResult::Err(diagnostics) = result else {
                unreachable!()
            };
            panic!(
                "Expected a descendant of the expected base to be accepted, got {:?}",
                diagnostics
                    .iter()
                    .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                    .collect::<Vec<_>>()
            );
        };
    }

    /// The same rule where the component is imported outright rather than extended locally.
    ///
    /// This is the ordinary embedding shape: an app binds a handler on a library component it did
    /// not declare. The emit's declaring module reaches the handler through the imported contract
    /// rather than through a locally written `extends`, so it is a distinct path to the same rule.
    #[test]
    fn a_host_action_for_a_fully_imported_component_is_checked_against_the_library_declaration() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("base.nx"),
            r#"
                action SearchSubmitted = { query:string }

                export component <SearchBox emits { SearchSubmitted } /> = {
                  <TextInput />
                }
            "#,
        )
        .expect("base file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        let main_path = app_dir.join("main.nx");
        // The app declares its own `SearchSubmitted` typing `query` as an int.
        let source = r#"
            import "../ui"

            action SearchSubmitted = { query:int }
            action DoSearch = { query:string }

            let withHandler() = <SearchBox onSearchSubmitted=<DoSearch query={action.query} /> />
        "#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");

        let int_action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([("query".to_string(), NxValue::Int(7))]),
        };
        let ComponentDispatchEvalResult::Err(diagnostics) =
            dispatch_host_action(&program, root_module.as_ref(), int_action)
        else {
            panic!("Expected host input to be checked against the library's declaration");
        };
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected string, got int")),
            "Expected a mismatch against the library declaration, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                .collect::<Vec<_>>()
        );

        let string_action = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([(
                "query".to_string(),
                NxValue::String("docs".to_string()),
            )]),
        };
        let result = dispatch_host_action(&program, root_module.as_ref(), string_action);
        let ComponentDispatchEvalResult::Ok(result) = result else {
            let ComponentDispatchEvalResult::Err(diagnostics) = result else {
                unreachable!()
            };
            panic!(
                "Expected input matching the library declaration to be accepted, got {:?}",
                diagnostics
                    .iter()
                    .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                    .collect::<Vec<_>>()
            );
        };
        assert_eq!(result.effects.len(), 1);
    }

    /// A component is evaluated in the module that declares it, not the one that named it.
    ///
    /// `Component::body` is an `ExprId` into its own module's arena, so evaluating it against the
    /// importing module reads a different expression at the same index. The program-entrypoint API
    /// resolves the declaring module already; the module-taking API did not.
    #[test]
    fn an_imported_component_is_evaluated_in_the_module_that_declares_it() {
        let temp = TempDir::new().expect("temp dir");
        let app_dir = temp.path().join("app");
        let ui_dir = temp.path().join("ui");
        fs::create_dir_all(&app_dir).expect("app dir");
        fs::create_dir_all(&ui_dir).expect("ui dir");

        fs::write(
            ui_dir.join("card.nx"),
            r#"
                export component <Card title:string = "Untitled" /> = {
                  <TextInput value={title} />
                }
            "#,
        )
        .expect("ui file");

        let registry = LibraryRegistry::new();
        registry
            .load_library_from_directory(&ui_dir)
            .expect("Expected ui registry load");
        let build_context = registry.build_context();

        // The app's own arena holds unrelated expressions at the indices the library body uses.
        let main_path = app_dir.join("main.nx");
        let source = r#"import "../ui"
let root() = { 1 + 2 + 3 }"#;
        fs::write(&main_path, source).expect("main file");

        let program = build_program_artifact_from_source(
            source,
            &main_path.display().to_string(),
            &build_context,
        )
        .expect("Expected program artifact");
        let root_module = program
            .root_modules
            .first()
            .and_then(|artifact| artifact.lowered_module.clone())
            .expect("Expected preserved root module");

        let interpreter = Interpreter::from_resolved_program(program.resolved_program.clone());
        let result = interpreter
            .initialize_component(
                root_module.as_ref(),
                "Card",
                from_nx_value(&empty_record()).expect("props"),
            )
            .expect("Expected imported component initialization to succeed");

        let nx_interpreter::Value::Record { type_name, fields } = &result.rendered else {
            panic!(
                "Expected the library's body to render, got {:?}",
                result.rendered
            );
        };
        assert_eq!(type_name.as_str(), "TextInput");
        assert_eq!(
            fields.get("value"),
            Some(&nx_interpreter::Value::String("Untitled".into()))
        );
    }
}
