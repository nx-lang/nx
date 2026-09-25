//! Arithmetic operations evaluation

use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::ast::BinOp;

/// Evaluate an arithmetic binary operation
pub fn eval_arithmetic_op(lhs: Value, op: BinOp, rhs: Value) -> Result<Value, RuntimeError> {
    match op {
        BinOp::Add => eval_add(lhs, rhs),
        BinOp::Sub => eval_sub(lhs, rhs),
        BinOp::Mul => eval_mul(lhs, rhs),
        BinOp::Div => eval_div(lhs, rhs),
        BinOp::Mod => eval_mod(lhs, rhs),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "arithmetic operands".to_string(),
            actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
            operation: format!("{:?}", op),
        })),
    }
}

/// An integer operand and a floating-point operand, both as the `float64` the pair is typed at.
///
/// <para>Analysis types `int` or `int32` with a float at `float64` — the crossing is exact, which
/// is why it is implicit — and this is that widening at run time. The interpreter has no static
/// types to read, so it widens from the operand values. `None` for every other pairing, which the
/// operator handles itself.</para>
fn widened_to_float64(lhs: &Value, rhs: &Value) -> Option<(f64, f64)> {
    fn integer(value: &Value) -> Option<f64> {
        match value {
            Value::Int32(n) => Some(f64::from(*n)),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }
    fn float(value: &Value) -> Option<f64> {
        match value {
            Value::Float32(n) => Some(f64::from(*n)),
            Value::Float(n) => Some(*n),
            _ => None,
        }
    }
    integer(lhs)
        .zip(float(rhs))
        .or_else(|| float(lhs).zip(integer(rhs)))
}

fn eval_add(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let Some((a, b)) = widened_to_float64(&lhs, &rhs) {
        return Ok(Value::Float(a + b));
    }
    match (lhs, rhs) {
        // Same-width integer ops
        (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a.wrapping_add(b))),
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_add(b))),
        // Cross-width integer promotion → i64
        (Value::Int32(a), Value::Int(b)) => Ok(Value::Int((a as i64).wrapping_add(b))),
        (Value::Int(a), Value::Int32(b)) => Ok(Value::Int(a.wrapping_add(b as i64))),
        // Same-width float ops
        (Value::Float32(a), Value::Float32(b)) => Ok(Value::Float32(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        // Cross-width float promotion → f64
        (Value::Float32(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
        (Value::Float(a), Value::Float32(b)) => Ok(Value::Float(a + b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "numeric operands".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "addition".to_string(),
        })),
    }
}

fn eval_sub(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let Some((a, b)) = widened_to_float64(&lhs, &rhs) {
        return Ok(Value::Float(a - b));
    }
    match (lhs, rhs) {
        (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a.wrapping_sub(b))),
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_sub(b))),
        (Value::Int32(a), Value::Int(b)) => Ok(Value::Int((a as i64).wrapping_sub(b))),
        (Value::Int(a), Value::Int32(b)) => Ok(Value::Int(a.wrapping_sub(b as i64))),
        (Value::Float32(a), Value::Float32(b)) => Ok(Value::Float32(a - b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float32(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
        (Value::Float(a), Value::Float32(b)) => Ok(Value::Float(a - b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "numeric operands".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "subtraction".to_string(),
        })),
    }
}

fn eval_mul(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let Some((a, b)) = widened_to_float64(&lhs, &rhs) {
        return Ok(Value::Float(a * b));
    }
    match (lhs, rhs) {
        (Value::Int32(a), Value::Int32(b)) => Ok(Value::Int32(a.wrapping_mul(b))),
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.wrapping_mul(b))),
        (Value::Int32(a), Value::Int(b)) => Ok(Value::Int((a as i64).wrapping_mul(b))),
        (Value::Int(a), Value::Int32(b)) => Ok(Value::Int(a.wrapping_mul(b as i64))),
        (Value::Float32(a), Value::Float32(b)) => Ok(Value::Float32(a * b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float32(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
        (Value::Float(a), Value::Float32(b)) => Ok(Value::Float(a * b as f64)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "numeric operands".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "multiplication".to_string(),
        })),
    }
}

fn eval_div(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let Some((a, b)) = widened_to_float64(&lhs, &rhs) {
        return eval_div(Value::Float(a), Value::Float(b));
    }
    match (lhs, rhs) {
        (Value::Int32(a), Value::Int32(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int32(a / b))
        }
        (Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a / b))
        }
        (Value::Int32(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int((a as i64) / b))
        }
        (Value::Int(a), Value::Int32(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a / (b as i64)))
        }
        (Value::Float32(a), Value::Float32(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float32(a / b))
        }
        (Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a / b))
        }
        (Value::Float32(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a as f64 / b))
        }
        (Value::Float(a), Value::Float32(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a / b as f64))
        }
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "numeric operands".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "division".to_string(),
        })),
    }
}

fn eval_mod(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    if let Some((a, b)) = widened_to_float64(&lhs, &rhs) {
        return eval_mod(Value::Float(a), Value::Float(b));
    }
    match (lhs, rhs) {
        (Value::Int32(a), Value::Int32(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int32(a % b))
        }
        (Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a % b))
        }
        (Value::Int32(a), Value::Int(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int((a as i64) % b))
        }
        (Value::Int(a), Value::Int32(b)) => {
            if b == 0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Int(a % (b as i64)))
        }
        (Value::Float32(a), Value::Float32(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float32(a % b))
        }
        (Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a % b))
        }
        (Value::Float32(a), Value::Float(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a as f64 % b))
        }
        (Value::Float(a), Value::Float32(b)) => {
            if b == 0.0 {
                return Err(RuntimeError::new(RuntimeErrorKind::DivisionByZero));
            }
            Ok(Value::Float(a % b as f64))
        }
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "numeric operands".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "modulo".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_int() {
        let result = eval_add(Value::Int(2), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_add_int32() {
        let result = eval_add(Value::Int32(2), Value::Int32(3)).unwrap();
        assert_eq!(result, Value::Int32(5));
    }

    #[test]
    fn test_add_int32_int_promotes() {
        let result = eval_add(Value::Int32(2), Value::Int(3)).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_add_float() {
        let result = eval_add(Value::Float(2.5), Value::Float(1.5)).unwrap();
        assert_eq!(result, Value::Float(4.0));
    }

    #[test]
    fn test_add_float32() {
        let result = eval_add(Value::Float32(2.5), Value::Float32(1.5)).unwrap();
        assert_eq!(result, Value::Float32(4.0));
    }

    #[test]
    fn test_add_float32_float_promotes() {
        let result = eval_add(Value::Float32(2.5), Value::Float(1.5)).unwrap();
        assert_eq!(result, Value::Float(4.0));
    }

    #[test]
    fn test_add_int_and_float_widens_to_float64() {
        assert_eq!(
            eval_add(Value::Int(2), Value::Float(3.5)).unwrap(),
            Value::Float(5.5)
        );
        assert_eq!(
            eval_add(Value::Float(3.5), Value::Int32(2)).unwrap(),
            Value::Float(5.5)
        );
    }

    #[test]
    fn test_add_int32_and_float32_widens_to_float64() {
        // Neither widens to the other; both widen exactly to float64.
        assert_eq!(
            eval_add(Value::Int32(2), Value::Float32(3.5)).unwrap(),
            Value::Float(5.5)
        );
    }

    #[test]
    fn test_mixed_category_arithmetic_is_float64() {
        assert_eq!(
            eval_sub(Value::Int(5), Value::Float(0.5)).unwrap(),
            Value::Float(4.5)
        );
        assert_eq!(
            eval_mul(Value::Float(1.5), Value::Int(2)).unwrap(),
            Value::Float(3.0)
        );
        assert_eq!(
            eval_div(Value::Int(7), Value::Float(2.0)).unwrap(),
            Value::Float(3.5)
        );
        assert!(eval_div(Value::Float(7.0), Value::Int(0)).is_err());
    }

    #[test]
    fn test_add_string_is_not_arithmetic() {
        // A `+` with a string operand is an `Expr::Concat` by the time it is evaluated; an
        // addition that still reaches here with a string was never type checked.
        assert!(eval_add(Value::String("a".into()), Value::Int(1)).is_err());
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
    fn test_div_i32_by_zero() {
        let result = eval_div(Value::Int32(10), Value::Int32(0));
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
    fn test_mod_int_and_float_widens_to_float64() {
        assert_eq!(
            eval_mod(Value::Int(10), Value::Float(4.0)).unwrap(),
            Value::Float(2.0)
        );
    }

    #[test]
    fn test_mod_by_zero_float() {
        let result = eval_mod(Value::Float(10.0), Value::Float(0.0));
        assert!(matches!(result, Err(RuntimeError { .. })));
    }

    #[test]
    fn test_empty_operand_is_a_type_mismatch() {
        let result = eval_add(Value::empty(), Value::Int(5)).unwrap_err();
        assert!(matches!(
            result.kind(),
            RuntimeErrorKind::TypeMismatch { .. }
        ));

        let result = eval_mul(Value::Int(5), Value::empty()).unwrap_err();
        assert!(matches!(
            result.kind(),
            RuntimeErrorKind::TypeMismatch { .. }
        ));
    }
}
