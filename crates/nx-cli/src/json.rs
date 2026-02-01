use nx_interpreter::Value;
use nx_value::NxValue;
use std::collections::BTreeMap;

pub fn format_value_json_pretty(value: &Value) -> Result<String, String> {
    let nx_value = to_nx_value(value);
    nx_value
        .to_json_string_pretty()
        .map_err(|e| format!("Failed to serialize JSON: {}", e))
}

fn to_nx_value(value: &Value) -> NxValue {
    match value {
        Value::Null => NxValue::Null,
        Value::Boolean(value) => NxValue::Bool(*value),
        Value::Int(value) => NxValue::Int(*value),
        Value::Float(value) => NxValue::Float(*value),
        Value::String(value) => NxValue::String(value.to_string()),
        Value::Array(elements) => NxValue::Array(elements.iter().map(to_nx_value).collect()),
        Value::EnumVariant { type_name, variant } => NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([
                (
                    "$enum".to_string(),
                    NxValue::String(type_name.as_str().to_string()),
                ),
                ("$variant".to_string(), NxValue::String(variant.to_string())),
            ]),
        },
        Value::Record { type_name, fields } => NxValue::Record {
            type_name: Some(type_name.as_str().to_string()),
            properties: fields_to_properties(fields),
        },
    }
}

fn fields_to_properties(
    fields: &rustc_hash::FxHashMap<smol_str::SmolStr, Value>,
) -> BTreeMap<String, NxValue> {
    let mut obj = BTreeMap::new();
    for (key, value) in fields {
        obj.insert(key.to_string(), to_nx_value(value));
    }

    obj
}
