//! Pretty printing for NX values in NX syntax format.
//!
//! This module provides formatting of runtime values in NX syntax,
//! which resembles XML with self-closing tags for elements.

use nx_interpreter::Value;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fmt::Write;

/// Pretty print a Value to NX format string.
///
/// # Rules
/// - Literal values (string, number, bool, null) are printed directly
/// - Records and objects are printed as XML-like elements
/// - Arrays are printed as a sequence of their elements
pub fn format_value(value: &Value) -> String {
    let mut output = String::new();
    format_value_inner(value, &mut output, 0);
    output
}

fn format_value_inner(value: &Value, output: &mut String, indent: usize) {
    match value {
        // Literal values - print directly
        Value::Int32(n) => write!(output, "{}", n).unwrap(),
        Value::Int(n) => write!(output, "{}", n).unwrap(),
        Value::Float32(f) => write!(output, "{}", f).unwrap(),
        Value::Float(f) => write!(output, "{}", f).unwrap(),
        Value::String(s) => output.push_str(s.as_str()),
        Value::Boolean(b) => write!(output, "{}", b).unwrap(),
        Value::Null => output.push_str("null"),

        // Enum variant
        Value::EnumVariant { type_name, variant } => {
            write!(output, "{}.{}", type_name, variant).unwrap();
        }

        // Array - print each element
        Value::Array(elements) => {
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                format_value_inner(elem, output, indent);
            }
        }

        Value::Record { type_name, fields } => {
            format_record_with_name(type_name.as_str(), fields, output, indent);
        }
    }
}

fn format_record_with_name(
    tag_name: &str,
    fields: &FxHashMap<SmolStr, Value>,
    output: &mut String,
    indent: usize,
) {
    // Collect and sort fields for deterministic output
    let mut field_vec: Vec<_> = fields.iter().collect();
    field_vec.sort_by_key(|(k, _)| k.as_str());

    // Check if any field contains a complex value (record or non-empty array)
    let has_complex_children = field_vec.iter().any(|(_, v)| is_complex_value(v));

    if has_complex_children {
        // Multi-line format with children
        write!(output, "<{}", tag_name).unwrap();

        // Print simple attributes on the opening tag
        for (key, value) in &field_vec {
            if !is_complex_value(value) {
                output.push(' ');
                output.push_str(key.as_str());
                output.push('=');
                format_attribute_value(value, output);
            }
        }

        output.push_str(">\n");

        // Print complex children as nested elements
        let child_indent = indent + 2;
        for (key, value) in &field_vec {
            if is_complex_value(value) {
                write!(output, "{:indent$}", "", indent = child_indent).unwrap();
                format_nested_element(key.as_str(), value, output, child_indent);
            }
        }

        write!(output, "{:indent$}</{}>", "", tag_name, indent = indent).unwrap();
    } else {
        // Single-line format with all attributes
        write!(output, "<{}", tag_name).unwrap();

        for (key, value) in &field_vec {
            output.push(' ');
            output.push_str(key.as_str());
            output.push('=');
            format_attribute_value(value, output);
        }

        output.push_str(" />");
    }
}

/// Format a nested element with proper opening and closing tags.
fn format_nested_element(tag_name: &str, value: &Value, output: &mut String, indent: usize) {
    match value {
        Value::Record { type_name, fields } => {
            format_nested_record(type_name.as_str(), fields, output, indent);
        }
        Value::Array(elements) => {
            write!(output, "<{}>", tag_name).unwrap();
            output.push('\n');
            let child_indent = indent + 2;
            for elem in elements {
                write!(output, "{:indent$}", "", indent = child_indent).unwrap();
                format_value_inner(elem, output, child_indent);
                output.push('\n');
            }
            write!(output, "{:indent$}</{}>", "", tag_name, indent = indent).unwrap();
            output.push('\n');
        }
        _ => {
            // Simple value - shouldn't happen for complex values but handle gracefully
            write!(output, "<{}", tag_name).unwrap();
            output.push('=');
            format_attribute_value(value, output);
            output.push_str(" />\n");
        }
    }
}

fn format_nested_record(
    tag_name: &str,
    fields: &FxHashMap<SmolStr, Value>,
    output: &mut String,
    indent: usize,
) {
    let mut field_vec: Vec<_> = fields.iter().collect();
    field_vec.sort_by_key(|(k, _)| k.as_str());

    let has_complex = field_vec.iter().any(|(_, v)| is_complex_value(v));

    write!(output, "<{}", tag_name).unwrap();

    if has_complex {
        // Print simple attrs, then children
        for (key, val) in &field_vec {
            if !is_complex_value(val) {
                output.push(' ');
                output.push_str(key.as_str());
                output.push('=');
                format_attribute_value(val, output);
            }
        }
        output.push_str(">\n");

        let child_indent = indent + 2;
        for (key, val) in &field_vec {
            if is_complex_value(val) {
                write!(output, "{:indent$}", "", indent = child_indent).unwrap();
                format_nested_element(key.as_str(), val, output, child_indent);
            }
        }

        write!(output, "{:indent$}</{}>", "", tag_name, indent = indent).unwrap();
        output.push('\n');
    } else {
        // All simple - inline
        for (key, val) in &field_vec {
            output.push(' ');
            output.push_str(key.as_str());
            output.push('=');
            format_attribute_value(val, output);
        }
        output.push_str(" />\n");
    }
}

fn format_attribute_value(value: &Value, output: &mut String) {
    match value {
        Value::String(s) => write!(output, "\"{}\"", escape_string(s.as_str())).unwrap(),
        Value::Int32(n) => write!(output, "\"{}\"", n).unwrap(),
        Value::Int(n) => write!(output, "\"{}\"", n).unwrap(),
        Value::Float32(f) => write!(output, "\"{}\"", f).unwrap(),
        Value::Float(f) => write!(output, "\"{}\"", f).unwrap(),
        Value::Boolean(b) => write!(output, "\"{}\"", b).unwrap(),
        Value::Null => output.push_str("\"null\""),
        Value::EnumVariant { type_name, variant } => {
            write!(output, "\"{}.{}\"", type_name, variant).unwrap()
        }
        Value::Array(_) | Value::Record { .. } => {
            // Complex values shouldn't be formatted as attributes
            output.push_str("\"...\"");
        }
    }
}

fn is_complex_value(value: &Value) -> bool {
    match value {
        Value::Record { .. } => true,
        Value::Array(elements) => !elements.is_empty(),
        _ => false,
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    #[test]
    fn test_format_int() {
        let value = Value::Int(42);
        assert_eq!(format_value(&value), "42");
    }

    #[test]
    fn test_format_float() {
        let value = Value::Float(3.14);
        assert_eq!(format_value(&value), "3.14");
    }

    #[test]
    fn test_format_string() {
        let value = Value::String(SmolStr::new("hello world"));
        assert_eq!(format_value(&value), "hello world");
    }

    #[test]
    fn test_format_boolean() {
        assert_eq!(format_value(&Value::Boolean(true)), "true");
        assert_eq!(format_value(&Value::Boolean(false)), "false");
    }

    #[test]
    fn test_format_null() {
        assert_eq!(format_value(&Value::Null), "null");
    }

    #[test]
    fn test_format_simple_record() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
        fields.insert(SmolStr::new("age"), Value::Int(30));

        let value = Value::Record {
            type_name: nx_hir::Name::new("result"),
            fields,
        };
        let output = format_value(&value);

        // Should be a self-closing tag with attributes
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        assert!(output.contains("age=\"30\""));
        assert!(output.contains("/>"));
    }

    #[test]
    fn test_format_enum_variant() {
        let value = Value::EnumVariant {
            type_name: nx_hir::Name::new("Status"),
            variant: SmolStr::new("Active"),
        };
        assert_eq!(format_value(&value), "Status.Active");
    }

    #[test]
    fn test_format_array_of_primitives() {
        let value = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(format_value(&value), "1\n2\n3");
    }

    #[test]
    fn test_format_nested_record() {
        let mut inner_fields = FxHashMap::default();
        inner_fields.insert(SmolStr::new("city"), Value::String(SmolStr::new("Boston")));
        inner_fields.insert(SmolStr::new("zip"), Value::String(SmolStr::new("02101")));

        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Alice")));
        fields.insert(
            SmolStr::new("address"),
            Value::Record {
                type_name: nx_hir::Name::new("Address"),
                fields: inner_fields,
            },
        );

        let value = Value::Record {
            type_name: nx_hir::Name::new("result"),
            fields,
        };
        let output = format_value(&value);

        // Should have nested structure
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        assert!(output.contains("<Address"));
        assert!(output.contains("city=\"Boston\""));
        assert!(output.contains("</result>"));
    }

    #[test]
    fn test_format_string_with_special_chars() {
        let value = Value::String(SmolStr::new("Hello \"World\"\nNew line"));
        assert_eq!(format_value(&value), "Hello \"World\"\nNew line");
    }
}
