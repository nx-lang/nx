use nx_api::to_nx_value;
use nx_interpreter::Value;

pub fn format_value_json_pretty(value: &Value) -> Result<String, String> {
    let nx_value = to_nx_value(value);
    nx_value
        .to_json_string_pretty()
        .map_err(|e| format!("Failed to serialize JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nx_hir::{Module, Name, SourceId};
    use nx_value::NxValue;
    use rustc_hash::FxHashMap;
    use std::collections::BTreeMap;

    #[test]
    fn test_format_value_json_pretty_action_handler() {
        let mut module = Module::new(SourceId::new(0));
        let body = module.alloc_expr(nx_hir::ast::Expr::Literal(nx_hir::ast::Literal::Null));
        let value = Value::ActionHandler {
            component: Name::new("SearchBox"),
            emit: Name::new("SearchSubmitted"),
            action_name: Name::new("SearchSubmitted"),
            body,
            captured: FxHashMap::default(),
        };

        let formatted = format_value_json_pretty(&value).expect("Action handler should serialize");
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
                    (
                        "component".to_string(),
                        NxValue::String("SearchBox".to_string()),
                    ),
                    (
                        "emit".to_string(),
                        NxValue::String("SearchSubmitted".to_string()),
                    ),
                ]),
            }
        );
    }
}
