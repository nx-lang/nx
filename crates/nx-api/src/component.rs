use crate::eval::{lower_source_module, runtime_error_diagnostics};
use crate::value::{from_nx_value, to_nx_value};
use crate::{NxDiagnostic, NxSeverity};
use nx_hir::{Item, Module};
use nx_interpreter::Interpreter;
use nx_value::NxValue;
use serde::{Deserialize, Serialize};

/// The result of initializing a component from source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentInitResult {
    /// Rendered component body converted to the public value model.
    pub rendered: NxValue,
    /// Opaque host-owned component state snapshot.
    #[serde(with = "serde_bytes")]
    pub state_snapshot: Vec<u8>,
}

/// The result of dispatching actions against a component state snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDispatchResult {
    /// Effect actions returned in dispatch order.
    pub effects: Vec<NxValue>,
    /// Opaque host-owned component state snapshot.
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

/// Result of dispatching actions from source text.
pub enum ComponentDispatchEvalResult {
    /// Dispatch succeeded.
    Ok(ComponentDispatchResult),
    /// Dispatch failed with diagnostics.
    Err(Vec<NxDiagnostic>),
}

/// Parses, lowers, and initializes a named component from source text.
pub fn initialize_component_source(
    source: &str,
    file_name: &str,
    component_name: &str,
    props: &NxValue,
) -> ComponentInitEvalResult {
    let module = match lower_source_module(source, file_name) {
        Ok(module) => module,
        Err(diagnostics) => return ComponentInitEvalResult::Err(diagnostics),
    };

    if let Err(message) = validate_host_input_value(&module, props) {
        return ComponentInitEvalResult::Err(invalid_input_diagnostics(message));
    }

    let props = match from_nx_value(props) {
        Ok(props) => props,
        Err(error) => return ComponentInitEvalResult::Err(invalid_input_diagnostics(error)),
    };

    let interpreter = Interpreter::new();
    match interpreter.initialize_component(&module, component_name, props) {
        Ok(result) => ComponentInitEvalResult::Ok(ComponentInitResult {
            rendered: to_nx_value(&result.rendered),
            state_snapshot: result.state_snapshot,
        }),
        Err(error) => ComponentInitEvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

/// Parses, lowers, and dispatches a batch of actions against a component state snapshot.
pub fn dispatch_component_actions_source(
    source: &str,
    file_name: &str,
    state_snapshot: &[u8],
    actions: &[NxValue],
) -> ComponentDispatchEvalResult {
    let module = match lower_source_module(source, file_name) {
        Ok(module) => module,
        Err(diagnostics) => return ComponentDispatchEvalResult::Err(diagnostics),
    };

    for (index, action) in actions.iter().enumerate() {
        if let Err(message) =
            validate_host_input_value_at_path(&module, action, &format!("$[{index}]"))
        {
            return ComponentDispatchEvalResult::Err(invalid_input_diagnostics(message));
        }
    }

    let interpreter = Interpreter::new();
    let actions = match actions
        .iter()
        .map(from_nx_value)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(actions) => actions,
        Err(error) => return ComponentDispatchEvalResult::Err(invalid_input_diagnostics(error)),
    };
    match interpreter.dispatch_component_actions(&module, state_snapshot, actions) {
        Ok(result) => ComponentDispatchEvalResult::Ok(ComponentDispatchResult {
            effects: result.effects.iter().map(to_nx_value).collect(),
            state_snapshot: result.state_snapshot,
        }),
        Err(error) => ComponentDispatchEvalResult::Err(runtime_error_diagnostics(source, error)),
    }
}

fn invalid_input_diagnostics(message: impl ToString) -> Vec<NxDiagnostic> {
    vec![NxDiagnostic {
        severity: NxSeverity::Error,
        code: Some("invalid-input".to_string()),
        message: message.to_string(),
        labels: Vec::new(),
        help: None,
        note: None,
    }]
}

fn validate_host_input_value(module: &Module, value: &NxValue) -> Result<(), String> {
    validate_host_input_value_at_path(module, value, "$")
}

fn validate_host_input_value_at_path(
    module: &Module,
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
                validate_host_input_value_at_path(module, value, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        NxValue::Record {
            type_name,
            properties,
        } => {
            if let Some(type_name) = type_name {
                if matches!(module.find_item(type_name), Some(Item::Component(_))) {
                    return Err(format!(
                        "NxValue at {path} uses component type '{type_name}', but component values \
                         are output-only and cannot be provided as host input"
                    ));
                }
            }

            for (key, value) in properties {
                validate_host_input_value_at_path(module, value, &format!("{path}.{key}"))?;
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_hir::{lower as lower_hir, SourceId};
    use std::collections::BTreeMap;

    fn empty_record() -> NxValue {
        NxValue::Record {
            type_name: None,
            properties: BTreeMap::new(),
        }
    }

    #[test]
    fn initialize_component_source_returns_rendered_output_and_state_snapshot() {
        let source = r#"
            component <SearchBox placeholder:string = "Find docs" /> = {
              state { query:string = {placeholder} }
              <TextInput value={query} placeholder={placeholder} />
            }
        "#;

        let result =
            initialize_component_source(source, "component-init.nx", "SearchBox", &empty_record());
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
    fn dispatch_component_actions_source_round_trips_effects_and_snapshot() {
        let source = r#"
            action SearchSubmitted = { searchString:string }
            action DoSearch = { search:string }

            component <SearchBox emits { SearchSubmitted } /> = {
              <TextInput />
            }

            let withHandler() = <SearchBox onSearchSubmitted=<DoSearch search={action.searchString} /> />
        "#;

        let parse_result = nx_syntax::parse_str(source, "component-dispatch.nx");
        let tree = parse_result
            .tree
            .expect("Expected dispatch source to parse");
        let module = lower_hir(tree.root(), SourceId::new(0));
        let interpreter = Interpreter::new();
        let props = interpreter
            .execute_function(&module, "withHandler", vec![])
            .expect("Expected props function to succeed");
        let init = interpreter
            .initialize_component(&module, "SearchBox", props)
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
    fn dispatch_component_actions_source_returns_runtime_diagnostics_for_invalid_snapshot() {
        let source = r#"
            component <Button text:string /> = {
              <button>{text}</button>
            }
        "#;

        let result = dispatch_component_actions_source(source, "invalid-snapshot.nx", b"nope", &[]);
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

        let result = initialize_component_source(source, "component-props.nx", "Wrapper", &props);
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
}
