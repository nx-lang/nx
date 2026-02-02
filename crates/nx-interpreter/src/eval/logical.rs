//! Logical and comparison operations evaluation

use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::ast::{BinOp, UnOp};

/// Evaluate a comparison binary operation (T036)
pub fn eval_comparison_op(lhs: Value, op: BinOp, rhs: Value) -> Result<Value, RuntimeError> {
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
        BinOp::Eq => eval_eq(lhs, rhs),
        BinOp::Ne => eval_ne(lhs, rhs),
        BinOp::Lt => eval_lt(lhs, rhs),
        BinOp::Le => eval_le(lhs, rhs),
        BinOp::Gt => eval_gt(lhs, rhs),
        BinOp::Ge => eval_ge(lhs, rhs),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "comparison operands".to_string(),
            actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
            operation: format!("{:?}", op),
        })),
    }
}

/// Evaluate a logical binary operation (T038)
pub fn eval_logical_op(lhs: Value, op: BinOp, rhs: Value) -> Result<Value, RuntimeError> {
    match op {
        BinOp::And => eval_and(lhs, rhs),
        BinOp::Or => eval_or(lhs, rhs),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "logical operands".to_string(),
            actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
            operation: format!("{:?}", op),
        })),
    }
}

/// Evaluate a logical unary operation (T038)
pub fn eval_logical_unary(op: UnOp, operand: Value) -> Result<Value, RuntimeError> {
    match op {
        UnOp::Not => eval_not(operand),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "bool".to_string(),
            actual: operand.type_name().to_string(),
            operation: format!("{:?}", op),
        })),
    }
}

/// Extract an i64 from any integer Value variant, for comparison.
fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Int32(n) => Some(*n as i64),
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

/// Extract an f64 from any float Value variant, for comparison.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float32(n) => Some(*n as f64),
        Value::Float(n) => Some(*n),
        _ => None,
    }
}

// Comparison operators

fn eval_eq(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let result = match (&lhs, &rhs) {
        (Value::Int32(_) | Value::Int(_), Value::Int32(_) | Value::Int(_)) => {
            as_i64(&lhs).unwrap() == as_i64(&rhs).unwrap()
        }
        (Value::Float32(_) | Value::Float(_), Value::Float32(_) | Value::Float(_)) => {
            as_f64(&lhs).unwrap() == as_f64(&rhs).unwrap()
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (
            Value::EnumVariant {
                type_name: a_type,
                variant: a_variant,
            },
            Value::EnumVariant {
                type_name: b_type,
                variant: b_variant,
            },
        ) => a_type == b_type && a_variant == b_variant,
        (Value::Null, Value::Null) => true,
        _ => false,
    };
    Ok(Value::Boolean(result))
}

fn eval_ne(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let eq_result = eval_eq(lhs, rhs)?;
    match eq_result {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        _ => unreachable!(),
    }
}

fn eval_lt(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let result = match (&lhs, &rhs) {
        (Value::Int32(_) | Value::Int(_), Value::Int32(_) | Value::Int(_)) => {
            as_i64(&lhs).unwrap() < as_i64(&rhs).unwrap()
        }
        (Value::Float32(_) | Value::Float(_), Value::Float32(_) | Value::Float(_)) => {
            as_f64(&lhs).unwrap() < as_f64(&rhs).unwrap()
        }
        (Value::String(a), Value::String(b)) => a < b,
        _ => {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "comparable types within same category".to_string(),
                actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
                operation: "less than".to_string(),
            }))
        }
    };
    Ok(Value::Boolean(result))
}

fn eval_le(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let result = match (&lhs, &rhs) {
        (Value::Int32(_) | Value::Int(_), Value::Int32(_) | Value::Int(_)) => {
            as_i64(&lhs).unwrap() <= as_i64(&rhs).unwrap()
        }
        (Value::Float32(_) | Value::Float(_), Value::Float32(_) | Value::Float(_)) => {
            as_f64(&lhs).unwrap() <= as_f64(&rhs).unwrap()
        }
        (Value::String(a), Value::String(b)) => a <= b,
        _ => {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "comparable types within same category".to_string(),
                actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
                operation: "less than or equal".to_string(),
            }))
        }
    };
    Ok(Value::Boolean(result))
}

fn eval_gt(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let result = match (&lhs, &rhs) {
        (Value::Int32(_) | Value::Int(_), Value::Int32(_) | Value::Int(_)) => {
            as_i64(&lhs).unwrap() > as_i64(&rhs).unwrap()
        }
        (Value::Float32(_) | Value::Float(_), Value::Float32(_) | Value::Float(_)) => {
            as_f64(&lhs).unwrap() > as_f64(&rhs).unwrap()
        }
        (Value::String(a), Value::String(b)) => a > b,
        _ => {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "comparable types within same category".to_string(),
                actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
                operation: "greater than".to_string(),
            }))
        }
    };
    Ok(Value::Boolean(result))
}

fn eval_ge(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let result = match (&lhs, &rhs) {
        (Value::Int32(_) | Value::Int(_), Value::Int32(_) | Value::Int(_)) => {
            as_i64(&lhs).unwrap() >= as_i64(&rhs).unwrap()
        }
        (Value::Float32(_) | Value::Float(_), Value::Float32(_) | Value::Float(_)) => {
            as_f64(&lhs).unwrap() >= as_f64(&rhs).unwrap()
        }
        (Value::String(a), Value::String(b)) => a >= b,
        _ => {
            return Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
                expected: "comparable types within same category".to_string(),
                actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
                operation: "greater than or equal".to_string(),
            }))
        }
    };
    Ok(Value::Boolean(result))
}

// Logical operators

fn eval_and(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "bool".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "logical and".to_string(),
        })),
    }
}

fn eval_or(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "bool".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "logical or".to_string(),
        })),
    }
}

fn eval_not(operand: Value) -> Result<Value, RuntimeError> {
    match operand {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "bool".to_string(),
            actual: v.type_name().to_string(),
            operation: "logical not".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;

    // Comparison tests
    #[test]
    fn test_eq_int() {
        let result = eval_eq(Value::Int(5), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_eq(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_eq_int32() {
        let result = eval_eq(Value::Int32(5), Value::Int32(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_eq_int32_int_cross_width() {
        let result = eval_eq(Value::Int32(5), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_eq_cross_category_false() {
        // Int vs Float returns false (not an error for eq/ne)
        let result = eval_eq(Value::Int(5), Value::Float(5.0)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_ne() {
        let result = eval_ne(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_lt() {
        let result = eval_lt(Value::Int(3), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_lt(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_lt_int32_int_cross_width() {
        let result = eval_lt(Value::Int32(3), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_lt_cross_category_error() {
        let result = eval_lt(Value::Int(3), Value::Float(5.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_le() {
        let result = eval_le(Value::Int(3), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_le(Value::Int(5), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_gt() {
        let result = eval_gt(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_ge() {
        let result = eval_ge(Value::Int(5), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_ge(Value::Int(5), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_string_comparison() {
        let result = eval_lt(
            Value::String(SmolStr::new("abc")),
            Value::String(SmolStr::new("def")),
        )
        .unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    // Logical operator tests
    #[test]
    fn test_and() {
        let result = eval_and(Value::Boolean(true), Value::Boolean(true)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_and(Value::Boolean(true), Value::Boolean(false)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_or() {
        let result = eval_or(Value::Boolean(true), Value::Boolean(false)).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_or(Value::Boolean(false), Value::Boolean(false)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_not() {
        let result = eval_not(Value::Boolean(true)).unwrap();
        assert_eq!(result, Value::Boolean(false));

        let result = eval_not(Value::Boolean(false)).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_null_comparison() {
        let result = eval_eq(Value::Null, Value::Null).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_null_operation_error() {
        let result = eval_lt(Value::Null, Value::Int(5));
        assert!(result.is_err());
    }

    #[test]
    fn test_type_mismatch_logical() {
        let result = eval_and(Value::Int(1), Value::Boolean(true));
        assert!(result.is_err());
    }
}
