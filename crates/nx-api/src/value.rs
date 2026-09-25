use nx_hir::ast::Occurrence;
use nx_hir::{is_update_record_name, Name};
use nx_interpreter::Value;
use nx_types::Type;
use nx_value::NxValue;
use rustc_hash::FxHashMap;
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

    fn unsupported_function(path: &str) -> Self {
        Self {
            path: path.to_string(),
            message: format!(
                "NxValue at {path} encodes a Function record, but a function value names a \
                 declaration of the running program and cannot be provided as host input"
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
/// Scalar types (`Boolean`, `Int`, `Float`, `String`) and arrays map directly; the empty value is
/// the empty array on both sides, and a record stores no entry for an empty optional field, so
/// the encoding omits the key. An entry call's result is converted by [`entry_result_to_nx_value`]
/// instead, which writes an empty standalone `T?` as [`NxValue::Null`].
///
/// Record values become [`NxValue::Record`] with their `type_name` preserved and fields
/// sorted alphabetically (via [`BTreeMap`]). A present empty field of an update record is a
/// cleared field and becomes [`NxValue::Null`], so the host reads the key and its `null`.
///
/// Constant union cases become [`NxValue::String`] carrying the bare authored case name. The
/// declaring union type is not preserved on the wire; consumers recover it from the target
/// schema (declared NX type, typed DTO property, or other type annotation).
///
/// `Value::ActionHandler` is encoded as an `ActionHandler` record carrying the public name of the
/// action it accepts and, when a lifecycle render gave it one, the `token` a host passes back in a
/// dispatch handler invocation. That record names the handler; it is not the handler, so it is
/// intentionally not round-trippable through [`from_nx_value`].
///
/// `Value::Function` is encoded as a `Function` record with the declaring module's identity and
/// the function's name, the same record the TypeScript runtime renders. It names a declaration of
/// the running program, so it is likewise not decoded from host input.
pub fn to_nx_value(value: &Value) -> NxValue {
    match value {
        Value::Boolean(value) => NxValue::Bool(*value),
        Value::Int32(value) => NxValue::Int32(*value),
        Value::Int(value) => NxValue::Int(*value),
        Value::Float32(value) => NxValue::Float32(*value),
        Value::Float(value) => NxValue::Float(*value),
        Value::String(value) => NxValue::String(value.to_string()),
        Value::Array(elements) => NxValue::Array(elements.iter().map(to_nx_value).collect()),
        Value::UnionCase { case, .. } => NxValue::String(case.to_string()),
        Value::Record { type_name, fields } => NxValue::Record {
            type_name: Some(type_name.as_str().to_string()),
            properties: fields_to_properties(fields, is_update_record_name(type_name.as_str())),
        },
        Value::Function { module, name } => NxValue::Record {
            type_name: Some("Function".to_string()),
            properties: BTreeMap::from([
                ("module".to_string(), NxValue::String(module.to_string())),
                ("name".to_string(), NxValue::String(name.to_string())),
            ]),
        },
        Value::ActionHandler {
            action_name, token, ..
        } => {
            let mut properties = BTreeMap::from([(
                "action".to_string(),
                NxValue::String(action_name.as_str().to_string()),
            )]);
            if let Some(token) = token {
                properties.insert("token".to_string(), NxValue::String(token.to_string()));
            }
            NxValue::Record {
                type_name: Some("ActionHandler".to_string()),
                properties,
            }
        }
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
        NxValue::Bool(value) => Ok(Value::Boolean(*value)),
        NxValue::Int32(value) => Ok(Value::Int32(*value)),
        NxValue::Int(value) => Ok(Value::Int(*value)),
        NxValue::Float32(value) => Ok(Value::Float32(*value)),
        NxValue::Float(value) => Ok(Value::Float(*value)),
        NxValue::String(value) => Ok(Value::String(SmolStr::new(value.as_str()))),
        NxValue::Null => Ok(Value::empty()),
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
            if type_name.as_deref() == Some("Function") {
                return Err(FromNxValueError::unsupported_function(path));
            }

            // A present `null` or `[]` is kept as written: construction against the declared type
            // decides what it means there. It is the empty value at a `?` or `*` site, the
            // instruction to clear a field of an update record, and an error where a value is
            // required, even one with a default, which only a missing key takes.
            let mut fields = FxHashMap::default();
            for (key, value) in properties {
                let value = from_nx_value_at_path(value, &format!("{path}.{key}"))?;
                fields.insert(SmolStr::new(key.as_str()), value);
            }
            Ok(Value::Record {
                type_name: Name::new(type_name.as_deref().unwrap_or("object")),
                fields,
            })
        }
    }
}

fn fields_to_properties(
    fields: &rustc_hash::FxHashMap<smol_str::SmolStr, Value>,
    is_update_record: bool,
) -> BTreeMap<String, NxValue> {
    let mut obj = BTreeMap::new();
    for (key, value) in fields {
        let value = if is_update_record && value.is_empty_value() {
            NxValue::Null
        } else {
            to_nx_value(value)
        };
        obj.insert(key.to_string(), value);
    }

    obj
}

/// Converts the result of an entry call — a function or value the host evaluates — for the host.
///
/// <para>A result whose type is a standalone `T?` and that holds nothing is [`NxValue::Null`], the
/// spelling hosts use for an absent single value and the one typegen's `T | null` and C#'s `T?`
/// expect. Every other result converts as [`to_nx_value`] does, so an empty `T*` stays `[]`.
/// `result_type` is the entry's declared or inferred type; without one, the result converts
/// as it is.</para>
pub fn entry_result_to_nx_value(value: &Value, result_type: Option<&Type>) -> NxValue {
    let is_optional = result_type.is_some_and(|ty| ty.occurrence() == Occurrence::OPTIONAL);
    if is_optional && value.is_empty_value() {
        NxValue::Null
    } else {
        to_nx_value(value)
    }
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
    fn interpreter_constant_case_lowers_to_bare_authored_case_string() {
        let runtime = Value::UnionCase {
            union: Name::new("Status"),
            case: SmolStr::new("active"),
        };

        assert_eq!(to_nx_value(&runtime), NxValue::String("active".to_string()));
    }

    /// The list `changed` produces is a list of constant cases, so it encodes as bare strings.
    #[test]
    fn a_list_of_property_cases_lowers_to_bare_field_names() {
        let runtime = Value::Array(vec![
            Value::UnionCase {
                union: Name::new("User.Property"),
                case: SmolStr::new("name"),
            },
            Value::UnionCase {
                union: Name::new("User.Property"),
                case: SmolStr::new("age"),
            },
        ]);
        let value = to_nx_value(&runtime);
        assert_eq!(
            value,
            NxValue::Array(vec![
                NxValue::String("name".to_string()),
                NxValue::String("age".to_string()),
            ])
        );
        assert_eq!(
            value.to_json_string().expect("JSON encodes"),
            r#"["name","age"]"#
        );
        let bytes = value.to_msgpack_vec().expect("MessagePack encodes");
        assert_eq!(
            NxValue::from_msgpack_slice(&bytes).expect("MessagePack decodes"),
            value
        );
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

    #[test]
    fn a_function_value_renders_as_a_function_record() {
        let value = to_nx_value(&Value::Function {
            module: SmolStr::new("app/main.nx"),
            name: SmolStr::new("ContactRow"),
        });
        assert_eq!(
            value.to_json_string().expect("JSON encodes"),
            r#"{"$type":"Function","module":"app/main.nx","name":"ContactRow"}"#
        );
    }

    #[test]
    fn from_nx_value_rejects_function_records() {
        let value = NxValue::from_json_str(
            r#"{ "$type": "Function", "module": "app/main.nx", "name": "ContactRow" }"#,
        )
        .expect("JSON parses");

        let error = from_nx_value(&value).expect_err("Expected Function input to be rejected");
        assert_eq!(error.path(), "$");
        assert!(error.to_string().contains("Function"), "{error}");
    }

    #[test]
    fn update_record_absence_survives_the_public_value_model() {
        let value = NxValue::from_json_str(r#"{ "$type": "User.Update", "name": "Ada" }"#)
            .expect("JSON parses");
        let runtime = from_nx_value(&value).expect("An update record with an absent field decodes");
        let Value::Record { type_name, fields } = &runtime else {
            panic!("Expected a record, got {:?}", runtime);
        };
        assert_eq!(type_name.as_str(), "User.Update");
        assert_eq!(fields.len(), 1, "An absent field must stay absent");

        let cleared = Value::Record {
            type_name: Name::new("User.Update"),
            fields: rustc_hash::FxHashMap::from_iter([(SmolStr::new("email"), Value::empty())]),
        };
        let json = to_nx_value(&cleared)
            .to_json_string()
            .expect("JSON encodes");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON parses");
        assert_eq!(
            parsed,
            serde_json::json!({ "$type": "User.Update", "email": null }),
            "Exactly the discriminator and the present field, with null kept"
        );

        let bytes = to_nx_value(&cleared)
            .to_msgpack_vec()
            .expect("MessagePack encodes");
        let decoded = NxValue::from_msgpack_slice(&bytes).expect("MessagePack decodes");
        assert_eq!(decoded, to_nx_value(&cleared));
    }
}
