//! Logical and comparison operations evaluation

use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Value;
use nx_hir::ast::{BinOp, UnOp};

/// Evaluate a comparison binary operation (T036)
pub fn eval_comparison_op(lhs: Value, op: BinOp, rhs: Value) -> Result<Value, RuntimeError> {
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
            expected: "boolean".to_string(),
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

/// Extract an f64 from any numeric Value variant, for comparison.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Int32(n) => Some(f64::from(*n)),
        Value::Int(n) => Some(*n as f64),
        Value::Float32(n) => Some(f64::from(*n)),
        Value::Float(n) => Some(*n),
        _ => None,
    }
}

/// Two numeric operands at the type they compare at.
///
/// <para>Two integers compare as integers. Any pair involving a float compares as `float64`, the
/// type analysis gives a mixed pair: `int` and `int32` widen to it exactly, which is why the
/// comparison is implicit at all, so `2 == 2.0` is true and `2 < 2.5` compares numerically.</para>
enum NumericPair {
    Ints(i64, i64),
    Floats(f64, f64),
}

fn numeric_pair(lhs: &Value, rhs: &Value) -> Option<NumericPair> {
    if let (Some(a), Some(b)) = (as_i64(lhs), as_i64(rhs)) {
        return Some(NumericPair::Ints(a, b));
    }
    Some(NumericPair::Floats(as_f64(lhs)?, as_f64(rhs)?))
}

/// How two ordered operands compare, or `None` when they do not: `NaN` with anything.
///
/// <para>`Err` is a pair with no order at all. The four relational operators differ only in which
/// orderings they accept, so they share this.</para>
fn ordering(
    lhs: &Value,
    rhs: &Value,
    operation: &str,
) -> Result<Option<std::cmp::Ordering>, RuntimeError> {
    if let Some(pair) = numeric_pair(lhs, rhs) {
        return Ok(match pair {
            NumericPair::Ints(a, b) => Some(a.cmp(&b)),
            NumericPair::Floats(a, b) => a.partial_cmp(&b),
        });
    }
    match (lhs, rhs) {
        (Value::String(a), Value::String(b)) => Ok(Some(a.cmp(b))),
        _ => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "two numbers or two strings".to_string(),
            actual: format!("{} and {}", lhs.type_name(), rhs.type_name()),
            operation: operation.to_string(),
        })),
    }
}

// Comparison operators

fn eval_eq(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(values_equal(&lhs, &rhs)))
}

/// The language's equality: scalars by value, records and lists structurally, a constant case by
/// its union and name, a function by the declaration it names.
///
/// <para>A record equals another of the same type whose every field is equal, and a list equals
/// another of the same length whose elements are equal in order. A function value equals another
/// exactly when both name the same declaration, which is its whole identity. Every value compares
/// as a sequence, so an item equals a one-element list of an equal item, and the empty value
/// equals only the empty value, being the empty sequence. This is the one equality `==`, match
/// patterns, and `diff` share, so what an author can test by hand is what every other comparison
/// sees.</para>
pub fn values_equal(lhs: &Value, rhs: &Value) -> bool {
    if let Some(pair) = numeric_pair(lhs, rhs) {
        return match pair {
            NumericPair::Ints(a, b) => a == b,
            NumericPair::Floats(a, b) => a == b,
        };
    }
    match (lhs, rhs) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (
            Value::UnionCase {
                union: a_union,
                case: a_case,
            },
            Value::UnionCase {
                union: b_union,
                case: b_case,
            },
        ) => a_union == b_union && a_case == b_case,
        (
            Value::Function {
                module: a_module,
                name: a_name,
            },
            Value::Function {
                module: b_module,
                name: b_name,
            },
        ) => a_module == b_module && a_name == b_name,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| values_equal(a, b))
        }
        // Every value compares as a sequence: an item is a sequence of one.
        (Value::Array(items), item) | (item, Value::Array(items)) => {
            items.len() == 1 && values_equal(&items[0], item)
        }
        (
            Value::Record {
                type_name: a_type,
                fields: a_fields,
            },
            Value::Record {
                type_name: b_type,
                fields: b_fields,
            },
        ) => {
            a_type == b_type
                && a_fields.len() == b_fields.len()
                && a_fields
                    .iter()
                    .all(|(name, a)| b_fields.get(name).is_some_and(|b| values_equal(a, b)))
        }
        _ => false,
    }
}

fn eval_ne(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    let eq_result = eval_eq(lhs, rhs)?;
    match eq_result {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        _ => unreachable!(),
    }
}

fn eval_lt(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use std::cmp::Ordering::Less;
    let ordering = ordering(&lhs, &rhs, "less than")?;
    Ok(Value::Boolean(matches!(ordering, Some(Less))))
}

fn eval_le(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use std::cmp::Ordering::{Equal, Less};
    let ordering = ordering(&lhs, &rhs, "less than or equal")?;
    Ok(Value::Boolean(matches!(ordering, Some(Less | Equal))))
}

fn eval_gt(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use std::cmp::Ordering::Greater;
    let ordering = ordering(&lhs, &rhs, "greater than")?;
    Ok(Value::Boolean(matches!(ordering, Some(Greater))))
}

fn eval_ge(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    use std::cmp::Ordering::{Equal, Greater};
    let ordering = ordering(&lhs, &rhs, "greater than or equal")?;
    Ok(Value::Boolean(matches!(ordering, Some(Greater | Equal))))
}

// Logical operators

fn eval_and(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "boolean".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "logical and".to_string(),
        })),
    }
}

fn eval_or(lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
    match (lhs, rhs) {
        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
        (a, b) => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "boolean".to_string(),
            actual: format!("{} and {}", a.type_name(), b.type_name()),
            operation: "logical or".to_string(),
        })),
    }
}

fn eval_not(operand: Value) -> Result<Value, RuntimeError> {
    match operand {
        Value::Boolean(b) => Ok(Value::Boolean(!b)),
        v => Err(RuntimeError::new(RuntimeErrorKind::TypeMismatch {
            expected: "boolean".to_string(),
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
    fn test_eq_int_and_float_compares_numerically() {
        assert_eq!(
            eval_eq(Value::Int(5), Value::Float(5.0)).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_eq(Value::Float(2.5), Value::Int32(2)).unwrap(),
            Value::Boolean(false)
        );
        assert_eq!(
            eval_ne(Value::Int(2), Value::Float(2.0)).unwrap(),
            Value::Boolean(false)
        );
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
    fn test_ordering_int_and_float_compares_numerically() {
        assert_eq!(
            eval_lt(Value::Int(2), Value::Float(2.5)).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_le(Value::Float(2.0), Value::Int32(2)).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_gt(Value::Int(3), Value::Float32(2.5)).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            eval_ge(Value::Int(2), Value::Float(2.5)).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_ordering_with_nan_is_false_every_way() {
        for compare in [eval_lt, eval_le, eval_gt, eval_ge] {
            assert_eq!(
                compare(Value::Float(f64::NAN), Value::Float(1.0)).unwrap(),
                Value::Boolean(false)
            );
        }
    }

    #[test]
    fn test_ordering_a_number_with_a_string_is_an_error() {
        assert!(eval_lt(Value::Int(3), Value::String("a".into())).is_err());
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
    fn test_type_mismatch_logical() {
        let result = eval_and(Value::Int(1), Value::Boolean(true));
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_values_compare_equal_only_to_each_other() {
        let result = eval_eq(Value::empty(), Value::empty()).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = eval_eq(Value::empty(), Value::Int(5)).unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_empty_operand_in_an_ordering_is_a_type_mismatch() {
        let result = eval_lt(Value::empty(), Value::Int(5)).unwrap_err();
        assert!(matches!(
            result.kind(),
            RuntimeErrorKind::TypeMismatch { .. }
        ));
    }
}
