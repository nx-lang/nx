//! Runtime value representation for the NX interpreter.

use smol_str::SmolStr;

/// Runtime value types supported by the NX interpreter
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value (i64)
    Int(i64),
    /// Floating-point value (f64)
    Float(f64),
    /// String value
    String(SmolStr),
    /// Boolean value
    Boolean(bool),
    /// Null/undefined value
    Null,
    /// Array of values
    Array(Vec<Value>),
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
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Null => "null",
            Value::Array(_) => "array",
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
    }

    #[test]
    fn test_type_names() {
        assert_eq!(Value::Int(42).type_name(), "int");
        assert_eq!(Value::Float(2.5).type_name(), "float");
        assert_eq!(Value::String(SmolStr::new("test")).type_name(), "string");
        assert_eq!(Value::Boolean(true).type_name(), "boolean");
        assert_eq!(Value::Null.type_name(), "null");
    }
}
