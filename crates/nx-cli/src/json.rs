use nx_api::entry_result_to_nx_value;
use nx_interpreter::Value;
use nx_types::Type;

/// Formats an entry call's result as the host reads it: an empty standalone `T?` result, per
/// `result_type`, is `null`.
pub fn format_value_json_pretty(
    value: &Value,
    result_type: Option<&Type>,
) -> Result<String, String> {
    let nx_value = entry_result_to_nx_value(value, result_type);
    nx_value
        .to_json_string_pretty()
        .map_err(|e| format!("Failed to serialize JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_hir::{LoweredModule, Name, SourceId};
    use nx_interpreter::RuntimeModuleId;
    use nx_value::NxValue;
    use rustc_hash::FxHashMap;
    use smol_str::SmolStr;
    use std::collections::BTreeMap;

    #[test]
    fn test_format_value_json_pretty_action_handler() {
        let mut module = LoweredModule::new(SourceId::new(0));
        let body = module.alloc_expr(nx_hir::ast::Expr::Literal(nx_hir::ast::Literal::Int(0)));
        let value = Value::ActionHandler {
            module_id: RuntimeModuleId::new(0),
            component: Name::new("SearchBox"),
            emit: Name::new("SearchSubmitted"),
            action_name: Name::new("SearchSubmitted"),
            action_module_identity: "json-test.nx".to_string(),
            body,
            captured: FxHashMap::default(),
            owner: None,
            owner_state: Vec::new(),
            token: Some(SmolStr::new("h1-1")),
        };

        let formatted =
            format_value_json_pretty(&value, None).expect("Action handler should serialize");
        let parsed = NxValue::from_json_str(&formatted).expect("JSON output should parse");

        assert_eq!(
            parsed,
            NxValue::Record {
                type_name: Some("ActionHandler".to_string()),
                properties: BTreeMap::from([
                    (
                        "action".to_string(),
                        NxValue::String("SearchSubmitted".to_string()),
                    ),
                    ("token".to_string(), NxValue::String("h1-1".to_string())),
                ]),
            }
        );
    }
}
