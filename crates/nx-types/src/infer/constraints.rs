//! Value constraints: `where` clauses on numeric types and numeric patterns in match arms.
//!
//! <para>A constrained type is its base type plus a check. This module turns a pattern's terms into
//! the interval they admit, reports what a constraint may not say, and decides at a binding site
//! whether a value needs a runtime check, can be proven at compile time, or already satisfies the
//! constraint through its own type.</para>

use super::InferenceContext;
use crate::ty::{Interval, IntervalEnd, Primitive, Type};
use nx_diagnostics::TextSpan;
use nx_hir::ast::{self, CompareOp, Literal, NumericPattern, PatternBound, PatternTerm};

/// The largest magnitude an integer bound may have: `int`'s range, and the part of `int64`'s a
/// JSON number and the TypeScript runtime's `number` hold exactly.
pub(crate) const MAX_EXACT_INTEGER: i64 = (1 << 53) - 1;

/// What a constraint may use, for the `invalid-constraint` message.
const SUPPORTED_TERMS: &str =
    "a constraint is a pattern of `finite`, `in a..b`, `in a..=b`, `<`, `<=`, `>` and `>=` terms joined by `&&`";

impl InferenceContext<'_> {
    /// The interval `pattern` admits on `base`, or `None` when a term was reported.
    ///
    /// <para>Each bound is checked as a literal written at the base type: a real bound on an
    /// integer base, a bound outside the base's range and an `int64` bound beyond ±(2^53−1) are
    /// reported, and a `float32` bound is rounded to `float32`. `context` names the base as source
    /// wrote it, for the leading-operand fix (`int where >= 1`).</para>
    pub(super) fn pattern_interval(
        &mut self,
        base: Primitive,
        pattern: &NumericPattern,
        context: &str,
        report: bool,
    ) -> Option<Interval> {
        let mut interval = Interval::ALL;
        let mut ok = true;
        for term in &pattern.terms {
            let term_interval = match term {
                PatternTerm::Finite => {
                    let Some(finite) = finite_interval(base) else {
                        if report {
                            self.error(
                                "invalid-constraint",
                                "`finite` applies only to floating types".to_owned(),
                                pattern.span,
                            );
                        }
                        ok = false;
                        continue;
                    };
                    finite
                }
                PatternTerm::Range {
                    start,
                    end,
                    inclusive,
                } => {
                    let low = self.pattern_bound(base, start, report);
                    let high = self.pattern_bound(base, end, report);
                    match (low, high) {
                        (Some(low), Some(high)) => Interval {
                            low: Some(IntervalEnd::inclusive(low)),
                            high: Some(if *inclusive {
                                IntervalEnd::inclusive(high)
                            } else {
                                IntervalEnd::exclusive(high)
                            }),
                        },
                        _ => {
                            ok = false;
                            continue;
                        }
                    }
                }
                PatternTerm::Compare { op, bound } => {
                    let Some(value) = self.pattern_bound(base, bound, report) else {
                        ok = false;
                        continue;
                    };
                    match op {
                        CompareOp::Lt => Interval {
                            low: None,
                            high: Some(IntervalEnd::exclusive(value)),
                        },
                        CompareOp::Le => Interval {
                            low: None,
                            high: Some(IntervalEnd::inclusive(value)),
                        },
                        CompareOp::Gt => Interval {
                            low: Some(IntervalEnd::exclusive(value)),
                            high: None,
                        },
                        CompareOp::Ge => Interval {
                            low: Some(IntervalEnd::inclusive(value)),
                            high: None,
                        },
                    }
                }
                PatternTerm::Unsupported { kind, span, text } => {
                    ok = false;
                    if report {
                        let message = match kind {
                            ast::UnsupportedTerm::Or => format!(
                                "`||` is not supported in a value pattern; {SUPPORTED_TERMS}, and a match arm lists alternatives with commas"
                            ),
                            ast::UnsupportedTerm::NotEqual => format!(
                                "`{text}` is not supported in a value pattern; {SUPPORTED_TERMS}"
                            ),
                            ast::UnsupportedTerm::Expression {
                                suggestion: Some(suggestion),
                            } => format!(
                                "A constraint has no operand: it is matched against the value itself, so write `{context} where {suggestion}`"
                            ),
                            ast::UnsupportedTerm::Expression { suggestion: None } => format!(
                                "`{text}` is not a value pattern; {SUPPORTED_TERMS}"
                            ),
                        };
                        self.error("invalid-constraint", message, *span);
                    }
                    continue;
                }
            };
            interval = interval.intersect(&term_interval);
        }
        if !ok {
            return None;
        }
        let interval = if base.is_integer() {
            interval.normalized_for_integers()
        } else {
            interval
        };
        if admits_none(base, &interval) {
            if report {
                self.error(
                    "empty-constraint",
                    format!("`{}` matches no value of `{}`", pattern.spelling(), base),
                    pattern.span,
                );
            }
            return None;
        }
        Some(interval)
    }

    /// One bound, checked as a literal written at `base`, or `None` once reported.
    fn pattern_bound(
        &mut self,
        base: Primitive,
        bound: &PatternBound,
        report: bool,
    ) -> Option<f64> {
        match bound_value(base, bound) {
            Ok(value) => Some(value),
            Err(problem) => {
                if report {
                    self.error("invalid-constraint", problem, bound.span);
                }
                None
            }
        }
    }

    /// Resolves a `where` clause over the already-resolved `base`.
    ///
    /// <para>The base must be one numeric value: an occurrence, a non-numeric type and an error
    /// are reported (an error silently) and give the base back unconstrained, so nothing cascades
    /// from the rejected clause. A base that is itself constrained, through an alias, intersects
    /// with the clause.</para>
    pub(super) fn constrained_type(
        &mut self,
        base_ref: &ast::TypeRef,
        base: Type,
        pattern: &NumericPattern,
    ) -> Type {
        let report = self.foreign_alias_depth == 0;
        if base.is_error() {
            return base;
        }
        if matches!(base, Type::Seq { .. }) {
            if report {
                let item = base.item().to_string();
                let suffix = base.occurrence().suffix();
                self.error(
                    "invalid-constraint",
                    format!(
                        "A constraint applies to one value, not to `{base}`; constrain the item type and put the suffix on its name: `type Score = {item} where {}` and `Score{suffix}`",
                        pattern.spelling()
                    ),
                    pattern.span,
                );
            }
            return base;
        }
        let (primitive, existing) = match &base {
            Type::Primitive(primitive) if primitive.is_numeric() => (*primitive, Interval::ALL),
            Type::Constrained { base, interval } => (*base, *interval),
            _ => {
                if report {
                    self.error(
                        "invalid-constraint",
                        format!(
                            "Constraints apply to numeric types (`int`, `int32`, `int64`, `float32`, `float64`), not to `{base}`"
                        ),
                        pattern.span,
                    );
                }
                return base;
            }
        };
        let context = ast::spell_type_ref(base_ref);
        let Some(interval) = self.pattern_interval(primitive, pattern, &context, report) else {
            return base;
        };
        let combined = existing.intersect(&interval);
        let combined = if primitive.is_integer() {
            combined.normalized_for_integers()
        } else {
            combined
        };
        if admits_none(primitive, &combined) {
            if report {
                self.error(
                    "empty-constraint",
                    format!("`{}` matches no value `{base}` admits", pattern.spelling()),
                    pattern.span,
                );
            }
            return base;
        }
        Type::constrained(primitive, combined)
    }

    /// Reports a constrained type where none may stand: a type argument or a function type.
    pub(super) fn reject_nested_constraint(
        &mut self,
        ty: &Type,
        what: &str,
        span: TextSpan,
    ) -> bool {
        if ty.constraint().is_none() || self.foreign_alias_depth > 0 {
            return false;
        }
        self.error(
            "invalid-constraint",
            format!(
                "A constrained type cannot be {what}; `{}` carries a constraint. Constraints apply to fields, props, state, parameters, results, annotations and aliases",
                ty.item()
            ),
            span,
        );
        true
    }
}

/// One bound's value, checked as a literal written at `base`: rounded to `float32` on a `float32`
/// base, or the reason it cannot be one.
pub fn bound_value(base: Primitive, bound: &PatternBound) -> Result<f64, String> {
    let problem = match (bound.value.as_ref(), base) {
        (None, _) => Some(format!("`{}` is not a number", bound.text)),
        (Some(Literal::Int(value)), Primitive::Int32) => {
            (i32::try_from(*value).is_err()).then(|| out_of_range(&bound.text, base))
        }
        (Some(Literal::Int(value)), Primitive::Int | Primitive::Int64) => {
            (value.unsigned_abs() > MAX_EXACT_INTEGER as u64).then(|| {
                if base == Primitive::Int64 {
                    format!(
                        "The bound `{}` is out of range: an `int64` bound must lie within ±(2^53−1), which a JSON number and the TypeScript runtime hold exactly",
                        bound.text
                    )
                } else {
                    out_of_range(&bound.text, base)
                }
            })
        }
        (Some(Literal::Float(_)), base) if base.is_integer() => Some(format!(
            "The bound `{}` is not an integer; a bound on `{}` must be an integer literal",
            bound.text, base
        )),
        (Some(Literal::Int(value)), Primitive::Float32 | Primitive::Float64) => {
            if value.unsigned_abs() > MAX_EXACT_INTEGER as u64 {
                Some(out_of_range(&bound.text, base))
            } else if base == Primitive::Float32 && (*value as f32) as i64 != *value {
                Some(format!(
                    "The bound `{}` is not exactly representable as `float32`; write a bound `float32` holds, such as `{}`",
                    bound.text,
                    (*value as f32) as i64
                ))
            } else {
                None
            }
        }
        (Some(Literal::Float(value)), Primitive::Float32) => {
            (!value.0.is_finite() || !(value.0 as f32).is_finite())
                .then(|| out_of_range(&bound.text, base))
        }
        (Some(Literal::Float(value)), Primitive::Float64) => {
            (!value.0.is_finite()).then(|| out_of_range(&bound.text, base))
        }
        _ => Some(format!("`{}` is not a numeric bound", bound.text)),
    };
    if let Some(problem) = problem {
        return Err(problem);
    }
    let value = match bound.value.as_ref() {
        Some(Literal::Int(value)) => *value as f64,
        Some(Literal::Float(value)) => value.0,
        _ => return Err(format!("`{}` is not a numeric bound", bound.text)),
    };
    Ok(if base == Primitive::Float32 {
        value as f32 as f64
    } else {
        value
    })
}

/// The interval `pattern` admits on `base`, as the checker resolves it, without reporting: bounds
/// checked and rounded at the base, an integer base's ends made inclusive. `None` when a term is
/// unsupported or a bound unreadable, or the interval is empty.
pub fn pattern_interval_at(base: Primitive, pattern: &NumericPattern) -> Option<Interval> {
    let mut interval = Interval::ALL;
    for term in &pattern.terms {
        let term = match term {
            PatternTerm::Finite => finite_interval(base)?,
            PatternTerm::Range {
                start,
                end,
                inclusive,
            } => Interval {
                low: Some(IntervalEnd::inclusive(bound_value(base, start).ok()?)),
                high: Some(if *inclusive {
                    IntervalEnd::inclusive(bound_value(base, end).ok()?)
                } else {
                    IntervalEnd::exclusive(bound_value(base, end).ok()?)
                }),
            },
            PatternTerm::Compare { op, bound } => {
                let value = bound_value(base, bound).ok()?;
                match op {
                    CompareOp::Lt => Interval {
                        low: None,
                        high: Some(IntervalEnd::exclusive(value)),
                    },
                    CompareOp::Le => Interval {
                        low: None,
                        high: Some(IntervalEnd::inclusive(value)),
                    },
                    CompareOp::Gt => Interval {
                        low: Some(IntervalEnd::exclusive(value)),
                        high: None,
                    },
                    CompareOp::Ge => Interval {
                        low: Some(IntervalEnd::inclusive(value)),
                        high: None,
                    },
                }
            }
            PatternTerm::Unsupported { .. } => return None,
        };
        interval = interval.intersect(&term);
    }
    let interval = if base.is_integer() {
        interval.normalized_for_integers()
    } else {
        interval
    };
    (!admits_none(base, &interval)).then_some(interval)
}

/// Whether no value of `base` lies in `interval`: its ends are read within the base's own range, and
/// a `float32` base's exclusive ends as the nearest `float32` inside them. The interval itself is
/// kept as written, so `int32 where >= 0` gains no upper bound.
pub(crate) fn admits_none(base: Primitive, interval: &Interval) -> bool {
    let (low, high) = match base {
        Primitive::Int32 => (f64::from(i32::MIN), f64::from(i32::MAX)),
        Primitive::Int | Primitive::Int64 => {
            (-(MAX_EXACT_INTEGER as f64), MAX_EXACT_INTEGER as f64)
        }
        Primitive::Float32 => (-f64::from(f32::MAX), f64::from(f32::MAX)),
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    };
    let range = Interval {
        low: Some(IntervalEnd::inclusive(low)),
        high: Some(IntervalEnd::inclusive(high)),
    };
    let interval = if base == Primitive::Float32 {
        // The next `float32` past an exclusive end. A bound is already a `float32`.
        let inside = |end: IntervalEnd, up: bool| {
            if !end.exclusive {
                return end;
            }
            let value = end.value.0 as f32;
            IntervalEnd::inclusive(f64::from(if up {
                value.next_up()
            } else {
                value.next_down()
            }))
        };
        Interval {
            low: interval.low.map(|end| inside(end, true)),
            high: interval.high.map(|end| inside(end, false)),
        }
    } else {
        *interval
    };
    interval.intersect(&range).is_empty()
}

fn out_of_range(text: &str, base: Primitive) -> String {
    format!("The bound `{text}` is out of range for `{base}`")
}

/// Whether a constant of numeric value `value` lies in `interval` once it takes `base`.
pub(crate) fn constant_satisfies(value: f64, base: Primitive, interval: &Interval) -> bool {
    let value = if base == Primitive::Float32 {
        value as f32 as f64
    } else {
        value
    };
    interval.contains(value)
}

/// A runtime check a binding site needs: the base the comparison is made at, the interval, and
/// the name of what is being bound.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintCheckSite {
    /// The constrained site's base type.
    pub base: Primitive,
    /// The values the site admits.
    pub interval: Interval,
    /// What is being bound: a parameter, field or prop name, or `result`.
    pub site: nx_hir::Name,
}

impl InferenceContext<'_> {
    /// Checks a constant bound to a constrained site at compile time: `Some(true)` when every
    /// number it holds lies in the interval, `Some(false)` once a violation is reported, and
    /// `None` when `expr` is not a constant, so the caller decides on a runtime check.
    ///
    /// <para>A constant is a numeric literal, a signed one, an arithmetic expression over them,
    /// or a braced list of those, which binds item by item at a sequence site.</para>
    pub(super) fn check_constant_constraint(
        &mut self,
        expr: nx_hir::ExprId,
        base: Primitive,
        interval: &Interval,
        expected: &Type,
        span: TextSpan,
        site: &str,
    ) -> Option<bool> {
        let values = self.constant_numbers(expr)?;
        for (value, value_span) in values {
            if !constant_satisfies(value, base, interval) {
                let written = if base == Primitive::Float32 {
                    ast::spell_float32_bound(value)
                } else if base.is_float() {
                    ast::spell_bound(value, true)
                } else if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    format!("{value}")
                };
                let what = if site.is_empty() {
                    String::new()
                } else {
                    format!(" for `{site}`")
                };
                self.error(
                    "constraint-violation",
                    format!(
                        "`{written}` is outside the constraint{what}: `{}`",
                        expected.item()
                    ),
                    value_span.unwrap_or(span),
                );
                return Some(false);
            }
        }
        Some(true)
    }

    /// The numbers a constant expression holds, each with its span, or `None` when `expr` is not
    /// a constant.
    fn constant_numbers(&self, expr: nx_hir::ExprId) -> Option<Vec<(f64, Option<TextSpan>)>> {
        let module = self.module.raw_module();
        let span = Some(module.expr_span(expr)).filter(|span| !span.is_empty());
        match module.expr(expr) {
            ast::Expr::Literal(Literal::Int(value)) => Some(vec![(*value as f64, span)]),
            ast::Expr::Literal(Literal::Int32(value)) => Some(vec![(*value as f64, span)]),
            ast::Expr::Literal(Literal::Float(value) | Literal::Float32(value)) => {
                Some(vec![(value.0, span)])
            }
            ast::Expr::BinaryOp { .. } | ast::Expr::UnaryOp { .. } => {
                match self.fold_constant(expr)? {
                    Ok(Literal::Int(value)) => Some(vec![(value as f64, span)]),
                    Ok(Literal::Float(value)) => Some(vec![(value.0, span)]),
                    _ => None,
                }
            }
            ast::Expr::Array { elements, .. } => {
                let mut values = Vec::new();
                for element in elements {
                    values.extend(self.constant_numbers(*element)?);
                }
                Some(values)
            }
            _ => None,
        }
    }
}

/// Inclusive finite magnitudes at the selected floating precision.
fn finite_interval(base: Primitive) -> Option<Interval> {
    let max = match base {
        Primitive::Float32 => f32::MAX as f64,
        Primitive::Float64 => f64::MAX,
        _ => return None,
    };
    Some(Interval {
        low: Some(IntervalEnd::inclusive(-max)),
        high: Some(IntervalEnd::inclusive(max)),
    })
}
