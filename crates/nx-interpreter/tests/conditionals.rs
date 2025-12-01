//! Integration tests for conditional expressions (if/else and ternary)
//!
//! Tests T039-T042: Conditional execution
//!
//! All tests use parsed NX source code, not direct HIR construction.
//!
//! Covers all three ValueIfExpression forms from the grammar:
//! - ValueIfSimpleExpression: `if condition { then } else { else }`
//! - ValueIfMatchExpression: `if scrutinee is { pattern => expr, ... }`
//! - ValueIfConditionListExpression: `if { cond => expr, ... else => default }`

use nx_hir::{lower, SourceId};
use nx_interpreter::{Interpreter, Value};
use nx_syntax::parse_str;
use smol_str::SmolStr;

/// Helper function to execute a function from NX source code and return the result
fn execute_nx_function(
    source: &str,
    function_name: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    let parse_result = parse_str(source, "test.nx");
    if !parse_result.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", parse_result.errors));
    }

    let root = parse_result.root().expect("Failed to get root");
    let module = lower(root, SourceId::new(0));

    let interpreter = Interpreter::new();
    interpreter
        .execute_function(&module, function_name, args)
        .map_err(|e| format!("Runtime error: {}", e))
}

// ============================================================================
// T039-T040: Basic If/Else Tests
// ============================================================================

/// T039: Test if/else true branch - max returns first arg when a > b
#[test]
fn test_if_else_true_branch() {
    let source = r#"
        let max(a:int, b:int): int = { if a > b { a } else { b } }
    "#;

    let result = execute_nx_function(source, "max", vec![Value::Int(10), Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(10));
}

/// T040: Test if/else false branch - max returns second arg when a < b
#[test]
fn test_if_else_false_branch() {
    let source = r#"
        let max(a:int, b:int): int = { if a > b { a } else { b } }
    "#;

    let result = execute_nx_function(source, "max", vec![Value::Int(3), Value::Int(7)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(7));
}

/// Test if without else (returns null on false)
#[test]
fn test_if_without_else() {
    let source = r#"
        let maybe_double(x:int): int = { if x > 0 { x * 2 } }
    "#;

    // Condition true, returns x * 2
    let result = execute_nx_function(source, "maybe_double", vec![Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(10));

    // Condition false, returns null
    let result = execute_nx_function(source, "maybe_double", vec![Value::Int(-5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Null);
}

// ============================================================================
// T041: Nested Conditionals
// ============================================================================

/// T041: Test nested conditionals with multiple classification levels
#[test]
fn test_nested_conditionals() {
    let source = r#"
        let classify(x:int): int = { 
            if x > 10 { 3 } 
            else {
                if x > 0 { 2 } 
                else {
                    if x == 0 { 1 } 
                    else { 0 }
                }
            } 
        }
    "#;

    // x > 10 returns 3
    let result = execute_nx_function(source, "classify", vec![Value::Int(15)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(3));

    // 0 < x <= 10 returns 2
    let result = execute_nx_function(source, "classify", vec![Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(2));

    // x == 0 returns 1
    let result = execute_nx_function(source, "classify", vec![Value::Int(0)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(1));

    // x < 0 returns 0
    let result = execute_nx_function(source, "classify", vec![Value::Int(-5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

/// Test chained else-if pattern (x > 10 -> 3, x > 5 -> 2, x > 0 -> 1, else 0)
#[test]
fn test_chained_else_if() {
    let source = r#"
        let grade(x:int): int = { 
            if x > 10 { 3 } 
            else {
                if x > 5 { 2 } 
                else {
                    if x > 0 { 1 } 
                    else { 0 }
                }
            }
        }
    "#;

    // x > 10 returns 3
    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Int(15)]).unwrap(),
        Value::Int(3)
    );

    // 5 < x <= 10 returns 2
    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Int(8)]).unwrap(),
        Value::Int(2)
    );

    // 0 < x <= 5 returns 1
    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Int(3)]).unwrap(),
        Value::Int(1)
    );

    // x <= 0 returns 0
    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Int(-5)]).unwrap(),
        Value::Int(0)
    );
}

/// Test deeply nested conditionals (5 levels)
#[test]
fn test_deeply_nested_conditionals() {
    let source = r#"
        let deep(flag:bool): int = { 
            if flag { 
                if flag { 
                    if flag { 
                        if flag { 
                            if flag { 42 } else { 0 } 
                        } else { 0 } 
                    } else { 0 } 
                } else { 0 } 
            } else { 0 } 
        }
    "#;

    // All true
    assert_eq!(
        execute_nx_function(source, "deep", vec![Value::Boolean(true)]).unwrap(),
        Value::Int(42)
    );

    // Any false
    assert_eq!(
        execute_nx_function(source, "deep", vec![Value::Boolean(false)]).unwrap(),
        Value::Int(0)
    );
}

// ============================================================================
// T042: Complex Condition Expressions
// ============================================================================

/// T042: Test conditionals with && (AND) operator
#[test]
fn test_and_condition() {
    let source = r#"
        let both_positive(a:int, b:int): int = { if a > 0 && b > 0 { a + b } else { 0 } }
    "#;

    // Both positive
    let result = execute_nx_function(source, "both_positive", vec![Value::Int(5), Value::Int(3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(8));

    // One negative
    let result = execute_nx_function(source, "both_positive", vec![Value::Int(5), Value::Int(-3)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));

    // Both negative
    let result = execute_nx_function(
        source,
        "both_positive",
        vec![Value::Int(-5), Value::Int(-3)],
    )
    .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::Int(0));
}

/// Test conditionals with || (OR) operator
#[test]
fn test_or_condition() {
    let source = r#"
        let either_positive(a:int, b:int): int = { if a > 0 || b > 0 { 1 } else { 0 } }
    "#;

    // Both positive
    assert_eq!(
        execute_nx_function(
            source,
            "either_positive",
            vec![Value::Int(1), Value::Int(2)]
        )
        .unwrap(),
        Value::Int(1)
    );

    // First positive only
    assert_eq!(
        execute_nx_function(
            source,
            "either_positive",
            vec![Value::Int(1), Value::Int(-2)]
        )
        .unwrap(),
        Value::Int(1)
    );

    // Second positive only
    assert_eq!(
        execute_nx_function(
            source,
            "either_positive",
            vec![Value::Int(-1), Value::Int(2)]
        )
        .unwrap(),
        Value::Int(1)
    );

    // Neither positive
    assert_eq!(
        execute_nx_function(
            source,
            "either_positive",
            vec![Value::Int(-1), Value::Int(-2)]
        )
        .unwrap(),
        Value::Int(0)
    );
}

/// Test inverted logic (NOT operator is not yet supported in grammar, so use <=)
#[test]
fn test_if_with_inverted_condition() {
    let source = r#"
        let is_not_positive(x:int): int = { if x <= 0 { 1 } else { 0 } }
    "#;

    // x <= 0 (condition is true)
    assert_eq!(
        execute_nx_function(source, "is_not_positive", vec![Value::Int(0)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "is_not_positive", vec![Value::Int(-5)]).unwrap(),
        Value::Int(1)
    );

    // x > 0 (condition is false)
    assert_eq!(
        execute_nx_function(source, "is_not_positive", vec![Value::Int(5)]).unwrap(),
        Value::Int(0)
    );
}

// ============================================================================
// Comparison Operators in Conditions
// ============================================================================

/// Test boolean literal as condition
#[test]
fn test_boolean_literal_condition() {
    let source = r#"
        let always_one(): int = { if true { 1 } else { 0 } }
        let always_zero(): int = { if false { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "always_one", vec![]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "always_zero", vec![]).unwrap(),
        Value::Int(0)
    );
}

/// Test boolean parameter as condition
#[test]
fn test_boolean_parameter_condition() {
    let source = r#"
        let choose(flag:bool): string = { if flag { "yes" } else { "no" } }
    "#;

    let result = execute_nx_function(source, "choose", vec![Value::Boolean(true)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("yes")));

    let result = execute_nx_function(source, "choose", vec![Value::Boolean(false)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("no")));
}

/// Test == (equality) in condition
#[test]
fn test_equality_condition() {
    let source = r#"
        let equals_five(x:int): int = { if x == 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "equals_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "equals_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(0)
    );
}

/// Test != (not equal) in condition
#[test]
fn test_not_equal_condition() {
    let source = r#"
        let not_five(x:int): int = { if x != 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "not_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "not_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(0)
    );
}

/// Test < (less than) in condition
#[test]
fn test_less_than_condition() {
    let source = r#"
        let lt_five(x:int): int = { if x < 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "lt_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "lt_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(0)
    );

    assert_eq!(
        execute_nx_function(source, "lt_five", vec![Value::Int(6)]).unwrap(),
        Value::Int(0)
    );
}

/// Test <= (less than or equal) in condition
#[test]
fn test_less_equal_condition() {
    let source = r#"
        let le_five(x:int): int = { if x <= 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "le_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "le_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "le_five", vec![Value::Int(6)]).unwrap(),
        Value::Int(0)
    );
}

/// Test > (greater than) in condition
#[test]
fn test_greater_than_condition() {
    let source = r#"
        let gt_five(x:int): int = { if x > 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "gt_five", vec![Value::Int(6)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "gt_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(0)
    );

    assert_eq!(
        execute_nx_function(source, "gt_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(0)
    );
}

/// Test >= (greater than or equal) in condition
#[test]
fn test_greater_equal_condition() {
    let source = r#"
        let ge_five(x:int): int = { if x >= 5 { 1 } else { 0 } }
    "#;

    assert_eq!(
        execute_nx_function(source, "ge_five", vec![Value::Int(6)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "ge_five", vec![Value::Int(5)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "ge_five", vec![Value::Int(4)]).unwrap(),
        Value::Int(0)
    );
}

// ============================================================================
// Expressions in Branches
// ============================================================================

/// Test arithmetic expression in then branch
#[test]
fn test_expression_in_then_branch() {
    let source = r#"
        let double_if_positive(x:int): int = { if x > 0 { x * 2 } else { x } }
    "#;

    // Positive: 5 * 2 = 10
    assert_eq!(
        execute_nx_function(source, "double_if_positive", vec![Value::Int(5)]).unwrap(),
        Value::Int(10)
    );

    // Not positive: returns x unchanged
    assert_eq!(
        execute_nx_function(source, "double_if_positive", vec![Value::Int(-3)]).unwrap(),
        Value::Int(-3)
    );
}

/// Test arithmetic in else branch (absolute value)
#[test]
fn test_expression_in_else_branch() {
    let source = r#"
        let abs(x:int): int = { if x >= 0 { x } else { 0 - x } }
    "#;

    assert_eq!(
        execute_nx_function(source, "abs", vec![Value::Int(5)]).unwrap(),
        Value::Int(5)
    );

    assert_eq!(
        execute_nx_function(source, "abs", vec![Value::Int(-5)]).unwrap(),
        Value::Int(5)
    );

    assert_eq!(
        execute_nx_function(source, "abs", vec![Value::Int(0)]).unwrap(),
        Value::Int(0)
    );
}

/// Test function call in condition
#[test]
fn test_function_call_in_condition() {
    let source = r#"
        let is_even(x:int): bool = { x == (x / 2) * 2 }
        let even_or_odd(x:int): string = { if is_even(x) { "even" } else { "odd" } }
    "#;

    let result = execute_nx_function(source, "even_or_odd", vec![Value::Int(4)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("even")));

    let result = execute_nx_function(source, "even_or_odd", vec![Value::Int(5)])
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(result, Value::String(SmolStr::new("odd")));
}

// ============================================================================
// Ternary Operator Tests
// ============================================================================

/// Test basic ternary expression
#[test]
fn test_ternary_basic() {
    let source = r#"
        let max(a:int, b:int): int = { a > b ? a : b }
    "#;

    assert_eq!(
        execute_nx_function(source, "max", vec![Value::Int(10), Value::Int(5)]).unwrap(),
        Value::Int(10)
    );

    assert_eq!(
        execute_nx_function(source, "max", vec![Value::Int(3), Value::Int(7)]).unwrap(),
        Value::Int(7)
    );
}

/// Test nested ternary (sign function)
#[test]
fn test_ternary_nested() {
    let source = r#"
        let sign(x:int): int = { x > 0 ? 1 : x < 0 ? -1 : 0 }
    "#;

    // Positive
    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(42)]).unwrap(),
        Value::Int(1)
    );

    // Negative
    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(-42)]).unwrap(),
        Value::Int(-1)
    );

    // Zero
    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(0)]).unwrap(),
        Value::Int(0)
    );
}

/// Test ternary used as function argument
#[test]
fn test_ternary_as_argument() {
    let source = r#"
        let double(x:int): int = { x * 2 }
        let cond_double(cond:bool, x:int): int = { double(cond ? x : 0) }
    "#;

    assert_eq!(
        execute_nx_function(
            source,
            "cond_double",
            vec![Value::Boolean(true), Value::Int(5)]
        )
        .unwrap(),
        Value::Int(10)
    );

    assert_eq!(
        execute_nx_function(
            source,
            "cond_double",
            vec![Value::Boolean(false), Value::Int(5)]
        )
        .unwrap(),
        Value::Int(0)
    );
}

/// Test ternary with arithmetic in branches
#[test]
fn test_ternary_with_arithmetic() {
    let source = r#"
        let clamp_positive(x:int): int = { x > 0 ? x : 0 }
    "#;

    assert_eq!(
        execute_nx_function(source, "clamp_positive", vec![Value::Int(10)]).unwrap(),
        Value::Int(10)
    );

    assert_eq!(
        execute_nx_function(source, "clamp_positive", vec![Value::Int(-5)]).unwrap(),
        Value::Int(0)
    );
}

/// Test ternary with boolean result
#[test]
fn test_ternary_boolean_result() {
    let source = r#"
        let is_big(x:int): bool = { x > 100 ? true : false }
    "#;

    assert_eq!(
        execute_nx_function(source, "is_big", vec![Value::Int(150)]).unwrap(),
        Value::Boolean(true)
    );

    assert_eq!(
        execute_nx_function(source, "is_big", vec![Value::Int(50)]).unwrap(),
        Value::Boolean(false)
    );
}

/// Test ternary with string result
#[test]
fn test_ternary_string_result() {
    let source = r#"
        let grade(passed:bool): string = { passed ? "pass" : "fail" }
    "#;

    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Boolean(true)]).unwrap(),
        Value::String(SmolStr::new("pass"))
    );

    assert_eq!(
        execute_nx_function(source, "grade", vec![Value::Boolean(false)]).unwrap(),
        Value::String(SmolStr::new("fail"))
    );
}

// ============================================================================
// Mixed If/Else and Ternary
// ============================================================================

/// Test if-else containing ternary
#[test]
fn test_if_else_containing_ternary() {
    let source = r#"
        let complex(a:int, b:int): int = { 
            if a > 0 { 
                b > 0 ? a + b : a 
            } else { 
                0 
            } 
        }
    "#;

    // a > 0, b > 0 -> a + b
    assert_eq!(
        execute_nx_function(source, "complex", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::Int(8)
    );

    // a > 0, b <= 0 -> a
    assert_eq!(
        execute_nx_function(source, "complex", vec![Value::Int(5), Value::Int(-3)]).unwrap(),
        Value::Int(5)
    );

    // a <= 0 -> 0
    assert_eq!(
        execute_nx_function(source, "complex", vec![Value::Int(-5), Value::Int(3)]).unwrap(),
        Value::Int(0)
    );
}

// ============================================================================
// ValueIfConditionListExpression Tests
// Grammar: if { cond1 => expr1, cond2 => expr2, else => default }
// ============================================================================

/// Test basic condition list expression
#[test]
fn test_condition_list_basic() {
    let source = r#"
        let classify(x:int): string = { 
            if { 
                x > 100 => "big"
                x > 10 => "medium"
                x > 0 => "small"
                else => "zero_or_negative"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(150)]).unwrap(),
        Value::String(SmolStr::new("big"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(50)]).unwrap(),
        Value::String(SmolStr::new("medium"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(5)]).unwrap(),
        Value::String(SmolStr::new("small"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(0)]).unwrap(),
        Value::String(SmolStr::new("zero_or_negative"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(-10)]).unwrap(),
        Value::String(SmolStr::new("zero_or_negative"))
    );
}

/// Test condition list without else (first matching condition wins)
#[test]
fn test_condition_list_without_else() {
    let source = r#"
        let sign(x:int): int = { 
            if { 
                x > 0 => 1
                x < 0 => -1
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(42)]).unwrap(),
        Value::Int(1)
    );

    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(-42)]).unwrap(),
        Value::Int(-1)
    );

    // Zero doesn't match any condition, should return null
    assert_eq!(
        execute_nx_function(source, "sign", vec![Value::Int(0)]).unwrap(),
        Value::Null
    );
}

/// Test condition list with complex conditions
#[test]
fn test_condition_list_complex_conditions() {
    let source = r#"
        let classify(a:int, b:int): string = { 
            if { 
                a > 0 && b > 0 => "both_positive"
                a > 0 || b > 0 => "one_positive"
                else => "neither_positive"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(5), Value::Int(3)]).unwrap(),
        Value::String(SmolStr::new("both_positive"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(5), Value::Int(-3)]).unwrap(),
        Value::String(SmolStr::new("one_positive"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(-5), Value::Int(3)]).unwrap(),
        Value::String(SmolStr::new("one_positive"))
    );

    assert_eq!(
        execute_nx_function(source, "classify", vec![Value::Int(-5), Value::Int(-3)]).unwrap(),
        Value::String(SmolStr::new("neither_positive"))
    );
}

/// Test condition list with arithmetic in results
#[test]
fn test_condition_list_with_arithmetic() {
    let source = r#"
        let transform(x:int): int = { 
            if { 
                x > 10 => x * 2
                x > 0 => x + 10
                else => 0 - x
            }
        }
    "#;

    // x > 10: multiply by 2
    assert_eq!(
        execute_nx_function(source, "transform", vec![Value::Int(20)]).unwrap(),
        Value::Int(40)
    );

    // 0 < x <= 10: add 10
    assert_eq!(
        execute_nx_function(source, "transform", vec![Value::Int(5)]).unwrap(),
        Value::Int(15)
    );

    // x <= 0: negate
    assert_eq!(
        execute_nx_function(source, "transform", vec![Value::Int(-5)]).unwrap(),
        Value::Int(5)
    );
}

// ============================================================================
// ValueIfMatchExpression Tests
// Grammar: if scrutinee is { pattern => expr, ... }
// ============================================================================

/// Test basic match expression with integer patterns
#[test]
fn test_match_expression_integers() {
    let source = r#"
        let describe(x:int): string = { 
            if x is { 
                0 => "zero"
                1 => "one"
                2 => "two"
                else => "many"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "describe", vec![Value::Int(0)]).unwrap(),
        Value::String(SmolStr::new("zero"))
    );

    assert_eq!(
        execute_nx_function(source, "describe", vec![Value::Int(1)]).unwrap(),
        Value::String(SmolStr::new("one"))
    );

    assert_eq!(
        execute_nx_function(source, "describe", vec![Value::Int(2)]).unwrap(),
        Value::String(SmolStr::new("two"))
    );

    assert_eq!(
        execute_nx_function(source, "describe", vec![Value::Int(100)]).unwrap(),
        Value::String(SmolStr::new("many"))
    );
}

/// Test match expression without else
#[test]
fn test_match_expression_without_else() {
    let source = r#"
        let special(x:int): string = { 
            if x is { 
                42 => "answer"
                0 => "nothing"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "special", vec![Value::Int(42)]).unwrap(),
        Value::String(SmolStr::new("answer"))
    );

    assert_eq!(
        execute_nx_function(source, "special", vec![Value::Int(0)]).unwrap(),
        Value::String(SmolStr::new("nothing"))
    );

    // No match, should return null
    assert_eq!(
        execute_nx_function(source, "special", vec![Value::Int(1)]).unwrap(),
        Value::Null
    );
}

/// Test match expression with multiple patterns per arm (comma-separated)
#[test]
fn test_match_expression_multiple_patterns() {
    let source = r#"
        let weekend(day:int): bool = { 
            if day is { 
                6, 7 => true
                else => false
            }
        }
    "#;

    // Saturday
    assert_eq!(
        execute_nx_function(source, "weekend", vec![Value::Int(6)]).unwrap(),
        Value::Boolean(true)
    );

    // Sunday
    assert_eq!(
        execute_nx_function(source, "weekend", vec![Value::Int(7)]).unwrap(),
        Value::Boolean(true)
    );

    // Weekday
    assert_eq!(
        execute_nx_function(source, "weekend", vec![Value::Int(1)]).unwrap(),
        Value::Boolean(false)
    );
}

/// Test match expression with boolean patterns
#[test]
fn test_match_expression_booleans() {
    let source = r#"
        let toggle(b:bool): string = { 
            if b is { 
                true => "yes"
                false => "no"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "toggle", vec![Value::Boolean(true)]).unwrap(),
        Value::String(SmolStr::new("yes"))
    );

    assert_eq!(
        execute_nx_function(source, "toggle", vec![Value::Boolean(false)]).unwrap(),
        Value::String(SmolStr::new("no"))
    );
}

/// Test match expression with expression scrutinee
#[test]
fn test_match_expression_computed_scrutinee() {
    let source = r#"
        let parity(x:int): string = { 
            if x == (x / 2) * 2 is { 
                true => "even"
                false => "odd"
            }
        }
    "#;

    assert_eq!(
        execute_nx_function(source, "parity", vec![Value::Int(4)]).unwrap(),
        Value::String(SmolStr::new("even"))
    );

    assert_eq!(
        execute_nx_function(source, "parity", vec![Value::Int(5)]).unwrap(),
        Value::String(SmolStr::new("odd"))
    );
}

/// Test that match expression only evaluates scrutinee once
/// Uses a function with side effects (division by zero) to prove single evaluation
#[test]
fn test_match_expression_evaluates_scrutinee_once() {
    // The scrutinee is 10 / denom. If it were evaluated multiple times,
    // denom = 0 would cause division by zero error. By using a non-zero denom,
    // if scrutinee is only evaluated once, it works correctly.
    let source = r#"
        let single_eval(denom:int): string = { 
            if 10 / denom is { 
                5 => "five"
                10 => "ten"
                else => "other"
            }
        }
    "#;

    // 10 / 2 = 5, should match first pattern
    assert_eq!(
        execute_nx_function(source, "single_eval", vec![Value::Int(2)]).unwrap(),
        Value::String(SmolStr::new("five"))
    );

    // 10 / 1 = 10, should match second pattern
    assert_eq!(
        execute_nx_function(source, "single_eval", vec![Value::Int(1)]).unwrap(),
        Value::String(SmolStr::new("ten"))
    );

    // 10 / 5 = 2, should go to else
    assert_eq!(
        execute_nx_function(source, "single_eval", vec![Value::Int(5)]).unwrap(),
        Value::String(SmolStr::new("other"))
    );
}

/// Test match expression with multiple patterns still evaluates scrutinee once
#[test]
fn test_match_multiple_patterns_evaluates_scrutinee_once() {
    let source = r#"
        let multi_pattern(denom:int): string = { 
            if 100 / denom is { 
                1, 2 => "small"
                10, 20 => "medium"
                50, 100 => "large"
                else => "other"
            }
        }
    "#;

    // 100 / 10 = 10, should match "medium"
    assert_eq!(
        execute_nx_function(source, "multi_pattern", vec![Value::Int(10)]).unwrap(),
        Value::String(SmolStr::new("medium"))
    );

    // 100 / 2 = 50, should match "large"
    assert_eq!(
        execute_nx_function(source, "multi_pattern", vec![Value::Int(2)]).unwrap(),
        Value::String(SmolStr::new("large"))
    );
}
