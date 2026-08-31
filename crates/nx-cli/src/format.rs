//! Pretty printing for NX values in NX syntax format.
//!
//! Every field of a record is emitted in property position as `key=value`. NX has no
//! property-element syntax — an element body binds to the single field marked `is_content`, which
//! is schema information no [`Value`] carries — so a property name written in body position has
//! nowhere to go and cannot be read back. Rendering is therefore uniform: the simple/complex split
//! decides line layout only, never whether a field becomes body content.

use nx_interpreter::Value;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fmt::Write;

/// Pretty print a value as NX source.
///
/// Fails for a value with no NX spelling rather than emitting output that does not read back.
/// Two values are in that position today: an empty list, because `items={}` is a syntax error, and
/// [`Value::ActionHandler`], which has no source form at all.
pub fn format_value(value: &Value) -> Result<String, String> {
    let mut output = String::new();
    format_value_inner(value, &mut output, 0)?;
    Ok(output)
}

fn format_value_inner(value: &Value, output: &mut String, indent: usize) -> Result<(), String> {
    match value {
        Value::Int32(n) => write!(output, "{}", n).unwrap(),
        Value::Int(n) => write!(output, "{}", n).unwrap(),
        Value::Float32(f) => output.push_str(&format_real_literal(f64::from(*f))),
        Value::Float(f) => output.push_str(&format_real_literal(*f)),
        Value::String(s) => output.push_str(s.as_str()),
        Value::Boolean(b) => write!(output, "{}", b).unwrap(),
        Value::Null => output.push_str("null"),

        // A constant union case names its union, exactly as the qualified source form does.
        Value::UnionCase { union, case } => {
            write!(output, "{}.{}", union, case).unwrap();
        }

        // A top-level sequence is a run of values, one per line.
        Value::Array(elements) => {
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                format_value_inner(elem, output, indent)?;
            }
        }

        Value::Record { type_name, fields } => {
            format_record(type_name.as_str(), fields, output, indent)?;
        }
        Value::ActionHandler { .. } => return Err(unspellable_action_handler()),
    }

    Ok(())
}

/// Emits a record as an element whose every field is a property.
fn format_record(
    tag_name: &str,
    fields: &FxHashMap<SmolStr, Value>,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    // Sorted for deterministic output.
    let mut field_vec: Vec<_> = fields.iter().collect();
    field_vec.sort_by_key(|(k, _)| k.as_str());

    write!(output, "<{}", tag_name).unwrap();

    if field_vec.iter().any(|(_, value)| is_complex_value(value)) {
        // One property per line. This is layout only — every field is still a property.
        let property_indent = indent + 2;
        for (key, value) in &field_vec {
            output.push('\n');
            write!(output, "{:width$}", "", width = property_indent).unwrap();
            output.push_str(key.as_str());
            output.push('=');
            format_property_value(value, output, property_indent)?;
        }
        output.push('\n');
        write!(output, "{:width$}/>", "", width = indent).unwrap();
    } else {
        for (key, value) in &field_vec {
            output.push(' ');
            output.push_str(key.as_str());
            output.push('=');
            format_property_value(value, output, indent)?;
        }
        output.push_str(" />");
    }

    Ok(())
}

/// Emits one value in property position, in a form that reads back at a typed site.
fn format_property_value(value: &Value, output: &mut String, indent: usize) -> Result<(), String> {
    match value {
        Value::String(s) => write!(output, "\"{}\"", escape_string(s.as_str())).unwrap(),
        Value::Int32(n) => write!(output, "{}", n).unwrap(),
        Value::Int(n) => write!(output, "{}", n).unwrap(),
        Value::Float32(f) => output.push_str(&format_real_literal(f64::from(*f))),
        Value::Float(f) => output.push_str(&format_real_literal(*f)),
        Value::Boolean(b) => write!(output, "{}", b).unwrap(),
        Value::Null => output.push_str("null"),
        // A bare case name; the declaring union comes from the target type.
        Value::UnionCase { case, .. } => output.push_str(case.as_str()),
        // `rhs_expression` admits an element, so a record value needs no braces.
        Value::Record { type_name, fields } => {
            format_record(type_name.as_str(), fields, output, indent)?
        }
        // A sequence needs them.
        Value::Array(elements) => {
            if elements.is_empty() {
                return Err(unspellable_empty_list());
            }
            output.push('{');
            for (i, element) in elements.iter().enumerate() {
                if i > 0 {
                    output.push(' ');
                }
                format_property_value(element, output, indent)?;
            }
            output.push('}');
        }
        Value::ActionHandler { .. } => return Err(unspellable_action_handler()),
    }

    Ok(())
}

fn unspellable_empty_list() -> String {
    "Cannot format an empty list: NX has no spelling for one (`items={}` is a syntax error), so \
     any output here would read back as a different value"
        .to_string()
}

fn unspellable_action_handler() -> String {
    "Cannot format an action handler: it has no NX source spelling".to_string()
}

/// Renders a float so it reads back as a real literal rather than an integer one.
///
/// `1.0` formats as `1` by default, which would bind as an integer literal at a float-typed site.
fn format_real_literal(value: f64) -> String {
    let rendered = format!("{}", value);
    if rendered.contains(['.', 'e', 'E']) || !value.is_finite() {
        rendered
    } else {
        format!("{}.0", rendered)
    }
}

/// Whether a value forces the one-property-per-line layout.
fn is_complex_value(value: &Value) -> bool {
    match value {
        Value::Record { .. } | Value::ActionHandler { .. } => true,
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
    use smol_str::SmolStr;

    /// Formats a value that is expected to have an NX spelling.
    fn formatted(value: &Value) -> String {
        format_value(value).expect("value should have an NX spelling")
    }

    /// Every scalar in attribute position must be emitted in a form that reads back.
    #[test]
    fn test_format_attribute_scalars_are_unquoted() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("w"), Value::Float(1.5));
        fields.insert(SmolStr::new("flag"), Value::Boolean(true));
        fields.insert(SmolStr::new("opt"), Value::Null);
        fields.insert(
            SmolStr::new("fit"),
            Value::UnionCase {
                union: nx_hir::Name::new("Fit"),
                case: SmolStr::new("cover"),
            },
        );
        // A payloadless case is a `Value::UnionCase` now, not an empty dotted record, so there is
        // nothing left for a heuristic to guess at.
        fields.insert(
            SmolStr::new("state"),
            Value::UnionCase {
                union: nx_hir::Name::new("LoadState"),
                case: SmolStr::new("loading"),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let formatted = formatted(&value);
        assert_eq!(
            formatted.trim(),
            "<Box fit=cover flag=true opt=null state=loading w=1.5 />"
        );
        assert!(
            !formatted.contains('"'),
            "no scalar should be quoted: {formatted}"
        );
    }

    /// A float must keep a real-literal spelling, or it binds as an integer at a float site.
    #[test]
    fn test_format_attribute_negative_float_keeps_real_spelling() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("neg"), Value::Float(-1.0));
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let formatted = formatted(&value);
        assert!(formatted.contains("neg=-1.0"), "got: {formatted}");
        assert!(!formatted.contains("neg=-1 "), "got: {formatted}");
        assert!(!formatted.contains("neg=\"-1\""), "got: {formatted}");
    }

    /// Formatted output must re-parse and type check against the types it came from.
    #[test]
    fn test_format_attribute_output_round_trips() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("w"), Value::Float(1.5));
        fields.insert(SmolStr::new("neg"), Value::Float(-1.0));
        fields.insert(SmolStr::new("n"), Value::Int(42));
        fields.insert(SmolStr::new("flag"), Value::Boolean(true));
        fields.insert(SmolStr::new("opt"), Value::Null);
        fields.insert(
            SmolStr::new("fit"),
            Value::UnionCase {
                union: nx_hir::Name::new("Fit"),
                case: SmolStr::new("cover"),
            },
        );
        fields.insert(
            SmolStr::new("state"),
            Value::Record {
                type_name: nx_hir::Name::new("LoadState.loading"),
                fields: FxHashMap::default(),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("Box"),
            fields,
        };

        let source = format!(
            "type Fit = fill | contain | cover\n\
             type LoadState = idle | loading\n\
             type Box = {{ w: float64 neg: float64 n: int flag: boolean opt: string? \
             fit: Fit state: LoadState }}\n{}",
            formatted(&value)
        );

        let result = nx_types::check_str(&source, "roundtrip.nx");
        assert!(
            result.errors().is_empty(),
            "formatted output should type check, got: {:?}\nsource:\n{}",
            result.errors(),
            source
        );
    }

    #[test]
    fn test_format_int() {
        let value = Value::Int(42);
        assert_eq!(formatted(&value), "42");
    }

    #[test]
    fn test_format_float() {
        let value = Value::Float(3.14);
        assert_eq!(formatted(&value), "3.14");
    }

    #[test]
    fn test_format_string() {
        let value = Value::String(SmolStr::new("hello world"));
        assert_eq!(formatted(&value), "hello world");
    }

    #[test]
    fn test_format_boolean() {
        assert_eq!(formatted(&Value::Boolean(true)), "true");
        assert_eq!(formatted(&Value::Boolean(false)), "false");
    }

    #[test]
    fn test_format_null() {
        assert_eq!(formatted(&Value::Null), "null");
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
        let output = formatted(&value);

        // Should be a self-closing tag with attributes
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        // A number is emitted unquoted: `age="30"` is a string at an int-typed site.
        assert!(output.contains("age=30"));
        assert!(output.contains("/>"));
    }

    #[test]
    fn test_format_enum_value() {
        let value = Value::UnionCase {
            union: nx_hir::Name::new("Status"),
            case: SmolStr::new("active"),
        };
        assert_eq!(formatted(&value), "Status.active");
    }

    #[test]
    fn test_format_array_of_primitives() {
        let value = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(formatted(&value), "1\n2\n3");
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
        let output = formatted(&value);

        // The nested record is a property value, not body content: the property name `address`
        // has nowhere to go in body position, so emitting it there loses which field it bound to.
        assert!(output.contains("<result"));
        assert!(output.contains("name=\"Alice\""));
        assert!(
            output.contains("address=<Address"),
            "a record-valued property is an unbraced element, got: {output}"
        );
        assert!(output.contains("city=\"Boston\""));
        assert!(
            !output.contains("</result>"),
            "no field becomes body content, got: {output}"
        );
    }

    #[test]
    fn test_format_string_with_special_chars() {
        let value = Value::String(SmolStr::new("Hello \"World\"\nNew line"));
        assert_eq!(formatted(&value), "Hello \"World\"\nNew line");
    }

    #[test]
    fn test_format_action_handler() {
        let mut module = nx_hir::LoweredModule::new(nx_hir::SourceId::new(0));
        let body = module.alloc_expr(nx_hir::ast::Expr::Literal(nx_hir::ast::Literal::Null));
        let value = Value::ActionHandler {
            module_id: nx_interpreter::RuntimeModuleId::new(0),
            component: nx_hir::Name::new("SearchBox"),
            emit: nx_hir::Name::new("SearchSubmitted"),
            action_name: nx_hir::Name::new("SearchSubmitted"),
            body,
            captured: FxHashMap::default(),
        };

        // `<ActionHandler ... />` is not a real element, so printing one produced output that
        // could never be read back. It fails explicitly instead.
        let error = format_value(&value).expect_err("an action handler has no NX spelling");
        assert!(error.contains("action handler"), "got: {error}");
    }

    /// An empty qualified record is not a union case. Formatting must not rewrite it into one.
    ///
    /// This is RF2 in `contextual-literal-binding`'s `review.md`. It is asserted on the value
    /// rather than through a source round-trip because the interpreter cannot yet construct a
    /// record imported under a module alias (`RecordTypeNotFound`), so `<div data={<foo.bar />} />`
    /// has no end-to-end spelling today. The defect is entirely in this module regardless.
    #[test]
    fn test_format_empty_qualified_record_is_not_rendered_as_a_union_case() {
        let mut fields = FxHashMap::default();
        fields.insert(
            SmolStr::new("data"),
            Value::Record {
                type_name: nx_hir::Name::new("foo.bar"),
                fields: FxHashMap::default(),
            },
        );
        let value = Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields,
        };

        let formatted = formatted(&value);

        assert_ne!(
            formatted.trim(),
            "<div data=bar />",
            "an empty qualified record must not be rewritten as a bare union case name"
        );
        assert!(
            formatted.contains("data="),
            "the property name must survive, got `{}`",
            formatted.trim()
        );
        assert!(
            formatted.contains("foo.bar"),
            "the record's own type name must survive, got `{}`",
            formatted.trim()
        );
    }

    /// An empty list has no NX spelling — `items={}` is a syntax error — so it must fail rather
    /// than emit `items="..."`, which is a `string` where a list was meant.
    #[test]
    fn test_format_empty_list_property_has_no_readable_spelling() {
        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("items"), Value::Array(Vec::new()));
        let value = Value::Record {
            type_name: nx_hir::Name::new("div"),
            fields,
        };

        let error = format_value(&value).expect_err("an empty list has no NX spelling");
        assert!(error.contains("empty list"), "got: {error}");
    }
}
