use nx_hir::Name;
use nx_interpreter::Value;
use nx_value::NxValue;
use smol_str::SmolStr;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Error returned when converting a public [`NxValue`] into an interpreter [`Value`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromNxValueError {
    path: String,
    message: String,
}

impl FromNxValueError {
    fn unsupported_action_handler(path: &str) -> Self {
        Self {
            path: path.to_string(),
            message: format!(
                "NxValue at {path} encodes an ActionHandler record, but ActionHandler values are \
                 runtime-only and cannot be provided as host input"
            ),
        }
    }

    /// Returns the path to the invalid value within the input tree.
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

impl fmt::Display for FromNxValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for FromNxValueError {}

/// Converts an interpreter [`Value`] into the serializable [`NxValue`] representation.
///
/// Scalar types (`Null`, `Boolean`, `Int`, `Float`, `String`) and arrays map directly.
///
/// Record values become [`NxValue::Record`] with their `type_name` preserved and fields
/// sorted alphabetically (via [`BTreeMap`]).
///
/// Enum variants are encoded as a record with two special fields:
/// - `$enum` — the enum type name
/// - `$variant` — the variant name
///
/// For example, `Color::Red` becomes `{ "$enum": "Color", "$variant": "Red" }`.
///
/// `Value::ActionHandler` is encoded as a record for display and inspection only. That shape is
/// intentionally not round-trippable through [`from_nx_value`].
pub fn to_nx_value(value: &Value) -> NxValue {
    match value {
        Value::Null => NxValue::Null,
        Value::Boolean(value) => NxValue::Bool(*value),
        Value::Int32(value) => NxValue::Int32(*value),
        Value::Int(value) => NxValue::Int(*value),
        Value::Float32(value) => NxValue::Float32(*value),
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
        Value::ActionHandler {
            component,
            emit,
            action_name,
            ..
        } => NxValue::Record {
            type_name: Some("ActionHandler".to_string()),
            properties: BTreeMap::from([
                (
                    "component".to_string(),
                    NxValue::String(component.as_str().to_string()),
                ),
                (
                    "emit".to_string(),
                    NxValue::String(emit.as_str().to_string()),
                ),
                (
                    "action".to_string(),
                    NxValue::String(action_name.as_str().to_string()),
                ),
            ]),
        },
    }
}

/// Converts a serializable [`NxValue`] into the interpreter [`Value`] representation.
///
/// This reverse conversion rejects runtime-only values that do not have a faithful public
/// encoding, such as `ActionHandler`.
pub fn from_nx_value(value: &NxValue) -> Result<Value, FromNxValueError> {
    from_nx_value_at_path(value, "$")
}

fn from_nx_value_at_path(value: &NxValue, path: &str) -> Result<Value, FromNxValueError> {
    match value {
        NxValue::Null => Ok(Value::Null),
        NxValue::Bool(value) => Ok(Value::Boolean(*value)),
        NxValue::Int32(value) => Ok(Value::Int32(*value)),
        NxValue::Int(value) => Ok(Value::Int(*value)),
        NxValue::Float32(value) => Ok(Value::Float32(*value)),
        NxValue::Float(value) => Ok(Value::Float(*value)),
        NxValue::String(value) => Ok(Value::String(SmolStr::new(value.as_str()))),
        NxValue::Array(elements) => Ok(Value::Array(
            elements
                .iter()
                .enumerate()
                .map(|(index, element)| from_nx_value_at_path(element, &format!("{path}[{index}]")))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        NxValue::Record {
            type_name,
            properties,
        } => {
            if type_name.as_deref() == Some("ActionHandler") {
                return Err(FromNxValueError::unsupported_action_handler(path));
            }

            Ok(Value::Record {
                type_name: Name::new(type_name.as_deref().unwrap_or("object")),
                fields: properties
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            SmolStr::new(key.as_str()),
                            from_nx_value_at_path(value, &format!("{path}.{key}"))?,
                        ))
                    })
                    .collect::<Result<_, _>>()?,
            })
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nx_value_round_trips_through_interpreter_value() {
        let value = NxValue::Record {
            type_name: Some("SearchSubmitted".to_string()),
            properties: BTreeMap::from([
                (
                    "searchString".to_string(),
                    NxValue::String("docs".to_string()),
                ),
                (
                    "metadata".to_string(),
                    NxValue::Record {
                        type_name: Some("SearchMetadata".to_string()),
                        properties: BTreeMap::from([("attempt".to_string(), NxValue::Int(1))]),
                    },
                ),
            ]),
        };

        let runtime = from_nx_value(&value).expect("Expected NxValue conversion to succeed");
        assert_eq!(to_nx_value(&runtime), value);
    }

    #[test]
    fn from_nx_value_rejects_action_handler_records() {
        let value = NxValue::Record {
            type_name: Some("ActionHandler".to_string()),
            properties: BTreeMap::from([(
                "component".to_string(),
                NxValue::String("SearchBox".to_string()),
            )]),
        };

        let error = from_nx_value(&value).expect_err("Expected ActionHandler input to be rejected");
        assert_eq!(error.path(), "$");
        assert!(error.to_string().contains("ActionHandler"));
    }
}
