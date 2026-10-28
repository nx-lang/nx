//! Closed numerical operations used by compiler-owned standard-library bodies.
//! This is runtime implementation support, not a host registration API.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MathOp {
    Sin = 0,
    Cos = 1,
    Tan = 2,
    Asin = 3,
    Acos = 4,
    Atan = 5,
    Atan2 = 6,
    Sqrt = 7,
    Hypot = 8,
    Abs = 9,
    Sign = 10,
    Min = 11,
    Max = 12,
    Clamp = 13,
    Floor = 14,
    Ceil = 15,
    Round = 16,
    RoundTiesToEven = 17,
    Trunc = 18,
    DegreesToRadians = 19,
    RadiansToDegrees = 20,
    Lerp = 21,
}

impl MathOp {
    pub const ALL: [Self; 22] = [
        Self::Sin,
        Self::Cos,
        Self::Tan,
        Self::Asin,
        Self::Acos,
        Self::Atan,
        Self::Atan2,
        Self::Sqrt,
        Self::Hypot,
        Self::Abs,
        Self::Sign,
        Self::Min,
        Self::Max,
        Self::Clamp,
        Self::Floor,
        Self::Ceil,
        Self::Round,
        Self::RoundTiesToEven,
        Self::Trunc,
        Self::DegreesToRadians,
        Self::RadiansToDegrees,
        Self::Lerp,
    ];
    pub fn from_code(code: u32) -> Option<Self> {
        Self::ALL.get(code as usize).copied()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Sqrt => "sqrt",
            Self::Hypot => "hypot",
            Self::Abs => "abs",
            Self::Sign => "sign",
            Self::Min => "min",
            Self::Max => "max",
            Self::Clamp => "clamp",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
            Self::RoundTiesToEven => "roundTiesToEven",
            Self::Trunc => "trunc",
            Self::DegreesToRadians => "degreesToRadians",
            Self::RadiansToDegrees => "radiansToDegrees",
            Self::Lerp => "lerp",
        }
    }
    pub fn params(self) -> &'static [&'static str] {
        match self {
            Self::Sin => &["x"],
            Self::Cos => &["x"],
            Self::Tan => &["x"],
            Self::Asin => &["x"],
            Self::Acos => &["x"],
            Self::Atan => &["x"],
            Self::Atan2 => &["y", "x"],
            Self::Sqrt => &["x"],
            Self::Hypot => &["x", "y"],
            Self::Abs => &["x"],
            Self::Sign => &["x"],
            Self::Min => &["a", "b"],
            Self::Max => &["a", "b"],
            Self::Clamp => &["value", "min", "max"],
            Self::Floor => &["x"],
            Self::Ceil => &["x"],
            Self::Round => &["x"],
            Self::RoundTiesToEven => &["x"],
            Self::Trunc => &["x"],
            Self::DegreesToRadians => &["x"],
            Self::RadiansToDegrees => &["x"],
            Self::Lerp => &["a", "b", "t"],
        }
    }
    pub fn arity(self) -> usize {
        self.params().len()
    }
    /// Canonical descriptor includes all signature fields and a semantic revision.
    pub fn descriptor(self, owner_module: &str) -> String {
        format!(
            "{owner_module}::{}({}):float64={}:v1",
            self.name(),
            self.params()
                .iter()
                .map(|p| format!("{p}:float64"))
                .collect::<Vec<_>>()
                .join(","),
            self as u8
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathError {
    pub code: &'static str,
    pub message: String,
}

fn minimum(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else {
        a.min(b)
    }
}
fn maximum(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        if a.is_sign_positive() || b.is_sign_positive() {
            0.0
        } else {
            -0.0
        }
    } else {
        a.max(b)
    }
}

/// Evaluate one bounded operation. Existing general arithmetic is unaffected.
pub fn evaluate(op: MathOp, args: &[f64]) -> Result<f64, MathError> {
    let error = |code, detail| MathError {
        code,
        message: format!("{}: {detail}", op.name()),
    };
    if args.len() != op.arity() {
        return Err(error("nx-math-domain", "wrong argument count"));
    }
    if args.iter().any(|x| !x.is_finite()) {
        return Err(error(
            "nx-math-non-finite-input",
            "arguments must be finite",
        ));
    }
    let x = args[0];
    let y = args.get(1).copied().unwrap_or(0.0);
    use MathOp::*;
    if (op == Sqrt && x < 0.0)
        || (matches!(op, Asin | Acos) && x.abs() > 1.0)
        || (op == Clamp && y > args[2])
    {
        return Err(error("nx-math-domain", "invalid domain or bounds"));
    }
    let result = match op {
        Sin => x.sin(),
        Cos => x.cos(),
        Tan => x.tan(),
        Asin => x.asin(),
        Acos => x.acos(),
        Atan => x.atan(),
        Atan2 => x.atan2(y),
        Sqrt => x.sqrt(),
        Hypot => x.hypot(y),
        Abs => x.abs(),
        Sign => {
            if x == 0.0 {
                x
            } else {
                x.signum()
            }
        }
        Min => minimum(x, y),
        Max => maximum(x, y),
        Clamp => minimum(maximum(x, y), args[2]),
        Floor => x.floor(),
        Ceil => x.ceil(),
        Trunc => x.trunc(),
        RoundTiesToEven => x.round_ties_even(),
        Round => {
            // Compare the fractional distance without rounding x + 0.5 first.
            let f = x.floor();
            let r = if x - f < 0.5 { f } else { f + 1.0 };
            if r == 0.0 {
                0.0_f64.copysign(x)
            } else {
                r
            }
        }
        DegreesToRadians => x * (std::f64::consts::PI / 180.0),
        RadiansToDegrees => x * (180.0 / std::f64::consts::PI),
        Lerp => {
            let t = args[2];
            if t == 0.0 {
                x
            } else if t == 1.0 {
                y
            } else if x == y {
                x
            } else if (x <= 0.0 && y >= 0.0) || (x >= 0.0 && y <= 0.0) {
                (1.0 - t) * x + t * y
            } else {
                let result = x + t * (y - x);
                if result.is_finite() {
                    result
                } else {
                    // Extrapolation can overflow the difference product before cancellation.
                    let alternative = (1.0 - t) * x + t * y;
                    if alternative.is_finite() {
                        alternative
                    } else {
                        // Both forms can overflow before cancellation; scale first.
                        (x * 0.5 + t * (y * 0.5 - x * 0.5)) * 2.0
                    }
                }
            }
        }
    };
    if !result.is_finite() {
        return Err(error("nx-math-overflow", "result is not finite"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_reference_corpus() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("math-cases.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let op = MathOp::ALL
                .into_iter()
                .find(|op| op.name() == case["op"].as_str().unwrap())
                .unwrap();
            let args = case["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            let actual = evaluate(op, &args);
            if let Some(code) = case["error"].as_str() {
                assert_eq!(actual.unwrap_err().code, code, "{}", case["name"]);
            } else {
                let actual = actual.unwrap();
                let expected = case["expected"].as_str().unwrap().parse::<f64>().unwrap();
                if case["exact"] == true {
                    assert_eq!(actual.to_bits(), expected.to_bits(), "{}", case["name"]);
                } else {
                    assert!(
                        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                        "{}: {actual} vs {expected}",
                        case["name"]
                    );
                }
            }
        }
    }
}
