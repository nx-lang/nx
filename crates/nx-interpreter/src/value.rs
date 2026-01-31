//! Runtime value representation for the NX interpreter.

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
/// let int_val = Value::Int(42);
/// let float_val = Value::Float(3.14);
/// let string_val = Value::String(SmolStr::new("hello"));
/// let bool_val = Value::Boolean(true);
/// let null_val = Value::Null;
/// let array_val = Value::Array(vec![Value::Int(1), Value::Int(2)]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value (i64)
    ///
    /// Represents whole numbers from -2^63 to 2^63-1
    Int(i64),

    /// Floating-point value (f64)
    ///
    /// Represents decimal numbers with IEEE 754 double precision
    Float(f64),

    /// String value
    ///
    /// Efficiently stores strings using SmolStr (inline for small strings)
    String(SmolStr),

    /// Boolean value
    ///
    /// Represents true or false logical values
    Boolean(bool),

    /// Null/undefined value
    ///
    /// Represents the absence of a value
    Null,

    /// Array of values
    ///
    /// Represents a collection of values, used for iteration and collections
    Array(Vec<Value>),

    /// Enum variant value
    ///
    /// Stores the enum type name and selected member.
    EnumVariant {
        /// Enum type name
        type_name: Name,
        /// Variant name
        variant: SmolStr,
    },

    /// Record value (map of field names to values)
    Record(FxHashMap<SmolStr, Value>),

    /// Typed record value with preserved type name
    ///
    /// Like Record but preserves the original type/element name for pretty printing.
    TypedRecord {
        /// The type/element name (e.g., "User", "Button")
        type_name: Name,
        /// Field values
        fields: FxHashMap<SmolStr, Value>,
    },
}

impl Value {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if the value is an integer
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if the value is a number (int or float)
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
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
    ///
    /// # Returns
    /// - "int" for `Int` values
    /// - "float" for `Float` values
    /// - "string" for `String` values
    /// - "boolean" for `Boolean` values
    /// - "null" for `Null` values
    /// - "array" for `Array` values
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::EnumVariant { .. } => "enum",
            Value::Record(_) => "record",
            Value::TypedRecord { .. } => "record",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
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
            Value::EnumVariant { type_name, variant } => write!(f, "{}.{}", type_name, variant),
            Value::TypedRecord { type_name, fields } => {
                write!(f, "{}{{ ", type_name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            Value::Record(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
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
        assert!(!int_val.is_null());

        let float_val = Value::Float(2.5);
        assert!(float_val.is_float());
        assert!(float_val.is_number());

        let string_val = Value::String(SmolStr::new("hello"));
        assert!(string_val.is_string());

        let bool_val = Value::Boolean(true);
        assert!(bool_val.is_boolean());

        let null_val = Value::Null;
        assert!(null_val.is_null());
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Float(2.5).to_string(), "2.5");
        assert_eq!(Value::String(SmolStr::new("test")).to_string(), "test");
        assert_eq!(Value::Boolean(true).to_string(), "true");
        assert_eq!(Value::Null.to_string(), "null");
        assert_eq!(
            Value::EnumVariant {
                type_name: Name::new("Status"),
                variant: SmolStr::new("Active")
            }
            .to_string(),
            "Status.Active"
        );

        let mut fields = FxHashMap::default();
        fields.insert(SmolStr::new("name"), Value::String(SmolStr::new("Ada")));
        fields.insert(SmolStr::new("age"), Value::Int(42));
        let display = Value::Record(fields).to_string();
        assert!(display.contains("age: 42"));
        assert!(display.contains("name: Ada"));
    }

    #[test]
    fn test_type_names() {
        assert_eq!(Value::Int(42).type_name(), "int");
        assert_eq!(Value::Float(2.5).type_name(), "float");
        assert_eq!(Value::String(SmolStr::new("test")).type_name(), "string");
        assert_eq!(Value::Boolean(true).type_name(), "boolean");
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Record(FxHashMap::default()).type_name(), "record");
    }
}
