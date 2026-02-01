//! NX value model for JSON-like object hierarchies.
//!
//! `NxValue` is intended to be a stable, serde-compatible data IR that can be used as input/output
//! across the NX API surface.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::de::Error as _;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A JSON-like tree value used as the stable NX API value type.
#[derive(Debug, Clone, PartialEq)]
pub enum NxValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<NxValue>),
    /// Record value (ordered properties).
    ///
    /// When serialized to JSON, `type_name` is encoded as a `"$type"` string property if present.
    Record {
        type_name: Option<String>,
        properties: BTreeMap<String, NxValue>,
    },
}

impl NxValue {
    /// Deserialize a value from a JSON string.
    pub fn from_json_str(source: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(source)
    }

    /// Deserialize a value from a JSON reader.
    pub fn from_json_reader<R: Read>(reader: R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    /// Deserialize a value from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, NxValueIoError> {
        let file = File::open(path.as_ref())?;
        Ok(Self::from_json_reader(file)?)
    }

    /// Serialize a value to a compact JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize a value to a pretty JSON string.
    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize a value to JSON using the provided writer.
    pub fn to_json_writer<W: Write>(&self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer(writer, self)
    }

    /// Serialize a value to a JSON file (compact form).
    pub fn to_json_file(&self, path: impl AsRef<Path>) -> Result<(), NxValueIoError> {
        let mut file = File::create(path.as_ref())?;
        self.to_json_writer(&mut file)?;
        file.flush()?;
        Ok(())
    }

    /// Serialize a value to a JSON file (pretty form).
    pub fn to_json_file_pretty(&self, path: impl AsRef<Path>) -> Result<(), NxValueIoError> {
        let mut file = File::create(path.as_ref())?;
        let json = self.to_json_string_pretty()?;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

impl Serialize for NxValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            NxValue::Null => serializer.serialize_unit(),
            NxValue::Bool(value) => serializer.serialize_bool(*value),
            NxValue::Int(value) => serializer.serialize_i64(*value),
            NxValue::Float(value) => serializer.serialize_f64(*value),
            NxValue::String(value) => serializer.serialize_str(value),
            NxValue::Array(elements) => {
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for element in elements {
                    seq.serialize_element(element)?;
                }
                seq.end()
            }
            NxValue::Record {
                type_name,
                properties,
            } => {
                let len = properties.len() + usize::from(type_name.is_some());
                let mut map = serializer.serialize_map(Some(len))?;

                if let Some(type_name) = type_name {
                    map.serialize_entry("$type", type_name)?;
                }

                for (key, value) in properties {
                    map.serialize_entry(key, value)?;
                }

                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for NxValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NxValueVisitor;

        impl<'de> Visitor<'de> for NxValueVisitor {
            type Value = NxValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON-like value")
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(NxValue::Null)
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(NxValue::Null)
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(NxValue::Bool(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(NxValue::Int(v))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                if v <= i64::MAX as u64 {
                    Ok(NxValue::Int(v as i64))
                } else {
                    Ok(NxValue::Float(v as f64))
                }
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(NxValue::Float(v))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(NxValue::String(v.to_owned()))
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(NxValue::String(v))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<NxValue>()? {
                    values.push(value);
                }
                Ok(NxValue::Array(values))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut type_name = None;
                let mut properties = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, NxValue>()? {
                    if key == "$type" {
                        match value {
                            NxValue::String(name) => {
                                type_name = Some(name);
                            }
                            other => {
                                return Err(A::Error::custom(format!(
                                    "expected \"$type\" to be a string, got {:?}",
                                    other
                                )));
                            }
                        }
                        continue;
                    }

                    properties.insert(key, value);
                }
                Ok(NxValue::Record {
                    type_name,
                    properties,
                })
            }
        }

        deserializer.deserialize_any(NxValueVisitor)
    }
}

/// Errors for JSON file IO helpers.
#[derive(Debug)]
pub enum NxValueIoError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for NxValueIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NxValueIoError::Io(e) => write!(f, "io error: {}", e),
            NxValueIoError::Json(e) => write!(f, "json error: {}", e),
        }
    }
}

impl std::error::Error for NxValueIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NxValueIoError::Io(e) => Some(e),
            NxValueIoError::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for NxValueIoError {
    fn from(value: std::io::Error) -> Self {
        NxValueIoError::Io(value)
    }
}

impl From<serde_json::Error> for NxValueIoError {
    fn from(value: serde_json::Error) -> Self {
        NxValueIoError::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn json_round_trip_in_memory() {
        let mut obj = BTreeMap::new();
        obj.insert("a".to_string(), NxValue::Int(1));
        obj.insert("b".to_string(), NxValue::Bool(true));
        obj.insert(
            "c".to_string(),
            NxValue::Array(vec![NxValue::Null, NxValue::String("x".to_string())]),
        );

        let value = NxValue::Record {
            type_name: None,
            properties: obj,
        };
        let json = value.to_json_string().unwrap();
        let decoded = NxValue::from_json_str(&json).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn json_round_trip_primitives() {
        let cases = [
            ("null", NxValue::Null),
            ("true", NxValue::Bool(true)),
            ("false", NxValue::Bool(false)),
            ("0", NxValue::Int(0)),
            ("-1", NxValue::Int(-1)),
            ("3.14", NxValue::Float(3.14)),
            ("\"hello\"", NxValue::String("hello".to_string())),
            ("[]", NxValue::Array(vec![])),
            (
                "{}",
                NxValue::Record {
                    type_name: None,
                    properties: BTreeMap::new(),
                },
            ),
        ];

        for (json, expected) in cases {
            let decoded = NxValue::from_json_str(json).unwrap();
            assert_eq!(decoded, expected);
            let encoded = decoded.to_json_string().unwrap();
            let decoded_again = NxValue::from_json_str(&encoded).unwrap();
            assert_eq!(decoded_again, expected);
        }
    }

    #[test]
    fn json_round_trip_nested_structures() {
        let value = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([
                (
                    "a".to_string(),
                    NxValue::Array(vec![NxValue::Int(1), NxValue::Int(2)]),
                ),
                (
                    "b".to_string(),
                    NxValue::Record {
                        type_name: None,
                        properties: BTreeMap::from([
                            ("x".to_string(), NxValue::Bool(false)),
                            ("y".to_string(), NxValue::Null),
                        ]),
                    },
                ),
            ]),
        };

        let json = value.to_json_string().unwrap();
        let decoded = NxValue::from_json_str(&json).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn json_parses_large_u64_as_float() {
        let value = NxValue::from_json_str("18446744073709551615").unwrap();
        assert!(matches!(value, NxValue::Float(_)));
    }

    #[test]
    fn json_number_boundaries() {
        let max_i64 = NxValue::from_json_str("9223372036854775807").unwrap();
        assert_eq!(max_i64, NxValue::Int(i64::MAX));

        let min_i64 = NxValue::from_json_str("-9223372036854775808").unwrap();
        assert_eq!(min_i64, NxValue::Int(i64::MIN));

        let just_over_max_i64 = NxValue::from_json_str("9223372036854775808").unwrap();
        assert!(matches!(just_over_max_i64, NxValue::Float(_)));
    }

    #[test]
    fn json_supports_exponent_notation() {
        let value = NxValue::from_json_str("1e3").unwrap();
        assert_eq!(value, NxValue::Float(1000.0));
    }

    #[test]
    fn json_preserves_negative_zero_sign() {
        let value = NxValue::from_json_str("-0.0").unwrap();
        match value {
            NxValue::Float(f) => {
                assert_eq!(f.to_bits(), (-0.0f64).to_bits());
            }
            _ => panic!("Expected float"),
        }
    }

    #[test]
    fn json_object_serialization_is_deterministic() {
        let mut obj = BTreeMap::new();
        obj.insert("b".to_string(), NxValue::Int(2));
        obj.insert("a".to_string(), NxValue::Int(1));

        let value = NxValue::Record {
            type_name: None,
            properties: obj,
        };
        let json = value.to_json_string().unwrap();
        assert_eq!(json, "{\"a\":1,\"b\":2}");
    }

    #[test]
    fn json_reader_writer_round_trip() {
        let value = NxValue::Array(vec![
            NxValue::String("x".to_string()),
            NxValue::Int(1),
            NxValue::Bool(false),
        ]);

        let mut buffer = Vec::new();
        value.to_json_writer(&mut buffer).unwrap();

        let decoded = NxValue::from_json_reader(Cursor::new(buffer)).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn json_pretty_string_round_trip() {
        let value = NxValue::Record {
            type_name: None,
            properties: BTreeMap::from([(
                "a".to_string(),
                NxValue::Array(vec![NxValue::Int(1), NxValue::Int(2)]),
            )]),
        };

        let json = value.to_json_string_pretty().unwrap();
        let decoded = NxValue::from_json_str(&json).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn json_invalid_input_errors() {
        assert!(NxValue::from_json_str("{]").is_err());
        assert!(NxValue::from_json_str("[1,]").is_err());
    }

    #[test]
    fn json_missing_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("missing.json");

        let err = NxValue::from_json_file(&missing).unwrap_err();
        assert!(matches!(err, NxValueIoError::Io(_)));
    }

    #[test]
    fn json_file_helpers_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("value.json");

        let mut obj = BTreeMap::new();
        obj.insert("name".to_string(), NxValue::String("Ada".to_string()));
        obj.insert("age".to_string(), NxValue::Int(42));
        let value = NxValue::Record {
            type_name: None,
            properties: obj,
        };

        value.to_json_file_pretty(&path).unwrap();
        let decoded = NxValue::from_json_file(&path).unwrap();
        assert_eq!(decoded, value);
    }
}
