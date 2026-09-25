//! Runtime value representation for the NX interpreter.

use crate::RuntimeModuleId;
use nx_hir::Name;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

/// Runtime value types supported by the NX interpreter
///
/// Represents all possible runtime values that can be produced or consumed
/// during expression evaluation. Values are used for function arguments,
/// return values, and intermediate computation results.
///
/// # Examples
/// ```
/// use nx_interpreter::Value;
/// use smol_str::SmolStr;
/// let i32_val = Value::Int32(42);
/// let int_val = Value::Int(42);
/// let f32_val = Value::Float32(3.14);
/// let float_val = Value::Float(3.14);
/// let string_val = Value::String(SmolStr::new("hello"));
/// let bool_val = Value::Boolean(true);
/// let empty = Value::empty();
/// let array_val = Value::Array(vec![Value::Int(1), Value::Int(2)]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 32-bit signed integer value (i32)
    Int32(i32),

    /// Default integer value (`int`)
    ///
    /// This is the type integer literals take, and the one NX programs use unless they have a
    /// reason not to. `int` is specified as exact over +/-(2^53-1) — the widest range every
    /// backend represents exactly — and is carried here in an `i64`, which holds that range with
    /// room to spare.
    ///
    /// `int64` has no distinct runtime carrier yet and also evaluates to this variant; the two
    /// are separated once `int64` gets its checked, bigint-backed representation.
    Int(i64),

    /// 32-bit floating-point value (f32)
    Float32(f32),

    /// 64-bit floating-point value (f64)
    ///
    /// This is the default float type (`float64`).
    Float(f64),

    /// String value
    ///
    /// Efficiently stores strings using SmolStr (inline for small strings)
    String(SmolStr),

    /// Boolean value
    ///
    /// Represents true or false logical values
    Boolean(bool),

    /// A sequence of values.
    ///
    /// <para>A `+` or `*` value is always one of these. The empty sequence is also the absent
    /// value — what `{}` evaluates to, what an optional property that was not written holds, and
    /// what an `if` with no `else` produces when it takes no branch. A `?` value that holds an
    /// item is the item itself, not a one-element sequence; coercion at a typed site normalizes
    /// between the two.</para>
    Array(Vec<Value>),

    /// Constant union case.
    ///
    /// A case that declares no fields in a union that declares no base carries nothing beyond its
    /// own name, so it is a scalar rather than an empty record. Every other case is a
    /// [`Value::Record`].
    UnionCase {
        /// Declaring union's name.
        union: Name,
        /// Case name, scoped to that union.
        case: SmolStr,
    },

    /// Record value (always typed).
    ///
    /// Records in NX are strongly typed, so a record always carries the record/element type name.
    Record {
        /// The record/element type name (e.g., "User", "Button").
        type_name: Name,
        /// Field values.
        fields: FxHashMap<SmolStr, Value>,
    },

    /// A function as a value: a reference to a module-level function declaration.
    ///
    /// <para>A bare identifier that names a visible function evaluates to this. It captures
    /// nothing — functions are module-level and a body has no nested declarations — so the
    /// declaring module's identity and the function's name are the whole value, and two function
    /// values are equal exactly when they name the same declaration. Rendered publicly as a
    /// `Function` record; see `nx-api`.</para>
    Function {
        /// Stable identity of the module that declares the function.
        module: SmolStr,
        /// The function's declared name.
        name: SmolStr,
    },

    /// Lazy component action handler callback with captured lexical values.
    ActionHandler {
        /// Owning lowered module for the handler body.
        module_id: RuntimeModuleId,
        /// Component name
        component: Name,
        /// Local emitted action name
        emit: Name,
        /// Public action type name expected for invocation
        action_name: Name,
        /// Stable identity of the module that declared the emit this handler binds to.
        ///
        /// <para>`action_name` is resolved here rather than in the handler's own module. A host
        /// supplies an action value carrying a type name and no origin, so the declaration it is
        /// checked against has to come from the emit, not from whatever that name happens to reach
        /// where the handler was written.</para>
        action_module_identity: String,
        /// Lowered handler body expression
        body: nx_hir::ExprId,
        /// Captured lexical variables from the handler definition site
        captured: FxHashMap<SmolStr, Value>,
        /// Component whose declaration the handler was bound in, or `None` at the root.
        owner: Option<Name>,
        /// The owner's state field names.
        ///
        /// <para>During dispatch these captures are replaced by the owner's working state before
        /// the body runs, so a state read sees every patch applied earlier in the batch. Every
        /// other capture keeps the value it had when the handler was created.</para>
        owner_state: Vec<SmolStr>,
        /// Dispatch token a lifecycle render assigned this handler, if any.
        ///
        /// <para>Set only on handlers in the rendered output of component initialization and
        /// dispatch, where the returned snapshot holds the handler under this token. It is output
        /// annotation: snapshots do not store it.</para>
        token: Option<SmolStr>,
    },
}

impl Value {
    /// The empty value: the empty sequence, which is also the absent value.
    pub fn empty() -> Self {
        Value::Array(Vec::new())
    }

    /// True for the empty value.
    pub fn is_empty_value(&self) -> bool {
        matches!(self, Value::Array(elements) if elements.is_empty())
    }

    /// Check if the value is any integer type (i32 or i64)
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int32(_) | Value::Int(_))
    }

    /// Check if the value is any float type (f32 or f64)
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float32(_) | Value::Float(_))
    }

    /// Check if the value is a number (any integer or float)
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            Value::Int32(_) | Value::Int(_) | Value::Float32(_) | Value::Float(_)
        )
    }

    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if the value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    /// Check if the value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Get the type name as a string
    ///
    /// Returns a static string describing the type of this value.
    /// Useful for error messages and debugging.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int32(_) => "int32",
            Value::Int(_) => "int",
            Value::Float32(_) => "float32",
            Value::Float(_) => "float64",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Array(elements) if elements.is_empty() => "{}",
            Value::Array(_) => "sequence",
            Value::UnionCase { .. } => "union_case",
            Value::Record { .. } => "record",
            Value::Function { .. } => "function",
            Value::ActionHandler { .. } => "action_handler",
        }
    }
}

impl Value {
    /// The canonical text form of a primitive value, or `None` for a value that has none.
    ///
    /// <para>This is the text `+` joins to a string and a text body embeds, and it is the same
    /// text every backend prints: an integer as its decimal digits, a boolean as `true` or
    /// `false`, and a float in the ECMAScript `Number::toString` form. A `float32` prints the
    /// shortest digits that round-trip as a `float32`, so one holding `0.1` prints `0.1` and not
    /// the expansion of its `float64` widening. It is not [`std::fmt::Display`], which renders
    /// lists and records for diagnostics and spells an integral float `1.0`.</para>
    pub fn to_text(&self) -> Option<String> {
        Some(match self {
            Value::Int32(value) => value.to_string(),
            Value::Int(value) => value.to_string(),
            Value::Float32(value) => float_text(
                value.is_nan(),
                value.is_infinite(),
                value.is_sign_negative(),
                *value == 0.0,
                format!("{:e}", value),
            ),
            Value::Float(value) => float_text(
                value.is_nan(),
                value.is_infinite(),
                value.is_sign_negative(),
                *value == 0.0,
                format!("{:e}", value),
            ),
            Value::Boolean(value) => value.to_string(),
            Value::String(value) => value.to_string(),
            _ => return None,
        })
    }
}

/// The ECMAScript `Number::toString` text of a float, given the facts about it and its shortest
/// round-trip scientific form.
///
/// <para>Rust's `{:e}` already prints the shortest digits that round-trip at the value's own
/// width, which is the hard part and is why a `float32` is formatted as a `float32` before it
/// gets here. What is left is ECMAScript's layout of those digits: plain decimal when the decimal
/// point falls within 21 digits to the right or 6 to the left, the exponent form `1e+21` or
/// `1e-7` beyond that, `0` for either zero, and `NaN`, `Infinity` and `-Infinity` spelled
/// out.</para>
fn float_text(
    is_nan: bool,
    is_infinite: bool,
    is_negative: bool,
    is_zero: bool,
    scientific: String,
) -> String {
    if is_nan {
        return "NaN".to_string();
    }
    if is_infinite {
        return if is_negative { "-Infinity" } else { "Infinity" }.to_string();
    }
    if is_zero {
        return "0".to_string();
    }

    let unsigned = scientific.trim_start_matches('-');
    let (mantissa, exponent) = unsigned.split_once('e').unwrap_or((unsigned, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exponent: i32 = exponent.parse().unwrap_or(0);

    // `k` digits, with the decimal point after the `n`th.
    let k = digits.len() as i32;
    let n = exponent + 1;

    let body = if k <= n && n <= 21 {
        format!("{}{}", digits, "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (whole, fraction) = digits.split_at(n as usize);
        format!("{}.{}", whole, fraction)
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let exponent = n - 1;
        let sign = if exponent < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{}e{}{}", first, sign, exponent.abs())
        } else {
            format!("{}.{}e{}{}", first, rest, sign, exponent.abs())
        }
    };

    if is_negative {
        format!("-{}", body)
    } else {
        body
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int32(n) => write!(f, "{}", n),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float32(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Array(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Value::UnionCase { union, case } => write!(f, "{}.{}", union, case),
            Value::Record { type_name, fields } => {
                write!(f, "{}{{ ", type_name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            Value::Function { module, name } => write!(f, "<function {}:{}>", module, name),
            Value::ActionHandler {
                component,
                emit,
                action_name,
                ..
            } => write!(f, "<action-handler {} {} {}>", component, emit, action_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        let int_val = Value::Int(42);
        assert!(int_val.is_int());
        assert!(int_val.is_number());
        assert!(!int_val.is_empty_value());

        let i32_val = Value::Int32(42);
        assert!(i32_val.is_int());
        assert!(i32_val.is_number());

        let float_val = Value::Float(2.5);
        assert!(float_val.is_float());
        assert!(float_val.is_number());

        let f32_val = Value::Float32(2.5);
        assert!(f32_val.is_float());
        assert!(f32_val.is_number());

        let string_val = Value::String(SmolStr::new("hello"));
        assert!(string_val.is_string());

        let bool_val = Value::Boolean(true);
        assert!(bool_val.is_boolean());

        let empty = Value::empty();
        assert!(empty.is_empty_value());
    }

    #[test]
    fn test_to_text_prints_floats_in_the_ecmascript_form() {
        let text = |value: f64| Value::Float(value).to_text().unwrap();

        assert_eq!(text(1.0), "1");
        assert_eq!(text(0.1), "0.1");
        assert_eq!(text(1.5), "1.5");
        assert_eq!(text(-2.25), "-2.25");
        assert_eq!(text(100.0), "100");
        assert_eq!(text(123.456), "123.456");
        assert_eq!(text(1e21), "1e+21");
        assert_eq!(text(1.5e21), "1.5e+21");
        assert_eq!(text(1e-7), "1e-7");
        assert_eq!(text(1.5e-7), "1.5e-7");
        assert_eq!(text(0.000001), "0.000001");
        assert_eq!(text(-0.0), "0");
        assert_eq!(text(0.0), "0");
        assert_eq!(text(123456789012345680000.0), "123456789012345680000");
        assert_eq!(text(9007199254740993.0), "9007199254740992");
        assert_eq!(text(f64::NAN), "NaN");
        assert_eq!(text(f64::INFINITY), "Infinity");
        assert_eq!(text(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(text(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(text(f64::MIN_POSITIVE), "2.2250738585072014e-308");
    }

    #[test]
    fn test_to_text_prints_a_float32_as_a_float32() {
        assert_eq!(Value::Float32(0.1).to_text().unwrap(), "0.1");
        assert_eq!(Value::Float32(1.0).to_text().unwrap(), "1");
        assert_eq!(Value::Float32(16777216.0).to_text().unwrap(), "16777216");
        assert_eq!(Value::Float32(1e-7).to_text().unwrap(), "1e-7");
        // The same number carried as a float64 is a different value and prints as one.
        assert_eq!(
            Value::Float(f64::from(0.1f32)).to_text().unwrap(),
            "0.10000000149011612"
        );
    }

    #[test]
    fn test_to_text_prints_integers_booleans_and_strings() {
        assert_eq!(Value::Int(3).to_text().unwrap(), "3");
        assert_eq!(Value::Int(-42).to_text().unwrap(), "-42");
        assert_eq!(Value::Int32(7).to_text().unwrap(), "7");
        assert_eq!(
            Value::Int(9007199254740993).to_text().unwrap(),
            "9007199254740993"
        );
        assert_eq!(Value::Boolean(true).to_text().unwrap(), "true");
        assert_eq!(Value::Boolean(false).to_text().unwrap(), "false");
        assert_eq!(Value::String(SmolStr::new("hi")).to_text().unwrap(), "hi");
    }

    #[test]
    fn test_to_text_declines_values_with_no_text_form() {
        assert_eq!(Value::empty().to_text(), None);
        assert_eq!(Value::Array(vec![Value::Int(1)]).to_text(), None);
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Int32(42).to_string(), "42");
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Float32(2.5).to_string(), "2.5");
        assert_eq!(Value::Float(2.5).to_string(), "2.5");
        assert_eq!(Value::String(SmolStr::new("test")).to_string(), "test");
        assert_eq!(Value::Boolean(true).to_string(), "true");
        assert_eq!(Value::empty().to_string(), "[]");
        assert_eq!(
            Value::UnionCase {
                union: Name::new("Status"),
                case: SmolStr::new("active")
            }
            .to_string(),
            "Status.active"
        );

        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));
        fields.insert(SmolStr::new("age"), Value::Int(42));
        let display = Value::Record {
            type_name: Name::new("result"),
            fields,
        }
        .to_string();
        assert!(display.contains("age: 42"));
        assert!(display.contains("name: Ada"));
    }

    #[test]
    fn test_type_names() {
        assert_eq!(Value::Int32(42).type_name(), "int32");
        assert_eq!(Value::Int(42).type_name(), "int");
        assert_eq!(Value::Float32(2.5).type_name(), "float32");
        assert_eq!(Value::Float(2.5).type_name(), "float64");
        assert_eq!(Value::String(SmolStr::new("test")).type_name(), "string");
        assert_eq!(Value::Boolean(true).type_name(), "boolean");
        assert_eq!(Value::empty().type_name(), "{}");
        assert_eq!(
            Value::Record {
                type_name: Name::new("result"),
                fields: FxHashMap::default(),
            }
            .type_name(),
            "record"
        );
    }
}
