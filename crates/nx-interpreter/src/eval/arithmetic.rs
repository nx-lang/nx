//! Arithmetic operations evaluation

use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::ast::BinOp;

/// Evaluate an arithmetic binary operation
pub fn eval_arithmetic_op(lhs: Value, op: BinOp, rhs: Value) -> Result<Value, RuntimeError> {
    // Check for null operands
    if lhs.is_null() {
        return Err(RuntimeError::new(RuntimeErrorKind::NullOperation {
            operation: format!("{:?}", op),
        }));
    }
    if rhs.is_null() {
        return Err(RuntimeError::new(RuntimeErrorKind::NullOperation {
            operation: format!("{:?}", op),
        }));
    }

    match op {
        BinOp::Add => eval_add(lhs, rhs),
        BinOp::Sub => eval_sub(lhs, rhs),
        BinOp::Mul => eval_mul(lhs, rhs),
        BinOp::Div => eval_div(lhs, rhs),
        BinOp::Mod => eval_mod(lhs, rhs),
        BinOp::Concat => eval_concat(lhs, rhs),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "arithmetic operands".to_string(),
            actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
            operation: format!("{:?}", op),
        })),
    }
}

fn eval_add(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "addition".to_string(),
        })),
    }
}

fn eval_sub(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "subtraction".to_string(),
        })),
    }
}

fn eval_mul(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "multiplication".to_string(),
        })),
    }
}

fn eval_div(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a / b))
        }
        (Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a / b))
        }
        (Value::Int(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a as f64 / b))
        }
        (Value::Float(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a / b as f64))
        }
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "division".to_string(),
        })),
    }
}

fn eval_mod(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a % b))
        }
        (Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a % b))
        }
        (Value::Int(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a as f64 % b))
        }
        (Value::Float(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a % b as f64))
        }
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "number".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "modulo".to_string(),
        })),
    }
}

fn eval_concat(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::String(a), Value::String(b)) => {
            let mut result = a.to_string();
            result.push_str(&b);
            Ok(Value::String(result.into()))
        }
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "string".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "concatenation".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;

    #[test]
    fn test_add_int() {
        let result = eval_add(Value::Int(2), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_add_float() {
        let result = eval_add(Value::Float(2.5), Value::Float(1.5)).unwrap();
        assert_eq!(result, Value::Float(4.0));
    }

    #[test]
    fn test_add_mixed() {
        let result = eval_add(Value::Int(2), Value::Float(3.5)).unwrap();
        assert_eq!(result, Value::Float(5.5));
    }

    #[test]
    fn test_sub() {
        let result = eval_sub(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_mul() {
        let result = eval_mul(Value::Int(4), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(12));
    }

    #[test]
    fn test_div() {
        let result = eval_div(Value::Int(10), Value::Int(2)).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_div_by_zero() {
        let result = eval_div(Value::Int(10), Value::Int(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_mod() {
        let result = eval_mod(Value::Int(10), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn test_mod_float() {
        let result = eval_mod(Value::Float(10.5), Value::Float(4.0)).unwrap();
        assert_eq!(result, Value::Float(10.5 % 4.0));
    }

    #[test]
    fn test_mod_mixed_int_float() {
        let result = eval_mod(Value::Int(10), Value::Float(4.0)).unwrap();
        assert_eq!(result, Value::Float(10.0 % 4.0));
    }

    #[test]
    fn test_mod_mixed_float_int() {
        let result = eval_mod(Value::Float(10.5), Value::Int(4)).unwrap();
        assert_eq!(result, Value::Float(10.5 % 4.0));
    }

    #[test]
    fn test_mod_by_zero_float() {
        let result = eval_mod(Value::Float(10.0), Value::Float(0.0));
        assert!(matches!(result, Err(RuntimeError { .. })));
    }

    #[test]
    fn test_concat() {
        let result = eval_concat(
            Value::String(SmolStr::new("hello")),
            Value::String(SmolStr::new(" world")),
        )
        .unwrap();
        assert_eq!(result, Value::String(SmolStr::new("hello world")));
    }

    #[test]
    fn test_null_operand() {
        let result = eval_add(Value::Null, Value::Int(5));
        assert!(result.is_err());
    }
}
