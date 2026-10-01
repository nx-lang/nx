//! The canonical text forms of numbers, which are the ECMAScript ones on every NX backend.

/// The ECMAScript `Number::toString` text of a float, from its shortest round-trip scientific
/// form at the float's own width.
///
/// <para>Rust's `{:e}` prints the shortest digits that round-trip, which is the hard part. What is
/// left is ECMAScript's layout of those digits: plain decimal when the decimal point falls within
/// 21 digits to the right or 6 to the left, the exponent form `1e+21` or `1e-7` beyond that, `0`
/// for either zero, and `NaN`, `Infinity` and `-Infinity` spelled out.</para>
fn layout(
    is_nan: bool,
    is_infinite: bool,
    is_negative: bool,
    is_zero: bool,
    scientific: &str,
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
    let exponent: i64 = exponent.parse().unwrap_or(0);

    // `k` digits, with the decimal point after the `n`th.
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let n = exponent.saturating_add(1);
    let zeros = |count: i64| "0".repeat(usize::try_from(count).unwrap_or(0));

    let body = if k <= n && n <= 21 {
        format!("{digits}{}", zeros(n.saturating_sub(k)))
    } else if 0 < n && n <= 21 {
        let (whole, fraction) = digits.split_at(usize::try_from(n).unwrap_or(0).min(digits.len()));
        format!("{whole}.{fraction}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", zeros(n.saturating_neg()))
    } else {
        let exponent = n.saturating_sub(1);
        let sign = if exponent < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1.min(digits.len()));
        if rest.is_empty() {
            format!("{first}e{sign}{}", exponent.unsigned_abs())
        } else {
            format!("{first}.{rest}e{sign}{}", exponent.unsigned_abs())
        }
    };

    if is_negative {
        format!("-{body}")
    } else {
        body
    }
}

/// The canonical text of a `float64`.
pub(crate) fn float64_text(value: f64) -> String {
    layout(
        value.is_nan(),
        value.is_infinite(),
        value.is_sign_negative(),
        value == 0.0,
        &format!("{value:e}"),
    )
}

/// The canonical text of a `float32`, which the runtime carries as the `f64` it widens to.
///
/// <para>Printing the carried value would print the widening's digits, `0.10000000149011612` for
/// the `float32` nearest `0.1`. This prints the shortest digits that round-trip to the same
/// `float32`, `0.1`, in the layout every other number prints in.</para>
pub fn float32_text(value: f64) -> String {
    let value = value as f32;
    layout(
        value.is_nan(),
        value.is_infinite(),
        value.is_sign_negative(),
        value == 0.0,
        &format!("{value:e}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_in_the_ecmascript_form() {
        assert_eq!(float64_text(1.0), "1");
        assert_eq!(float64_text(0.1), "0.1");
        assert_eq!(float64_text(-2.25), "-2.25");
        assert_eq!(float64_text(123.456), "123.456");
        assert_eq!(float64_text(1e21), "1e+21");
        assert_eq!(float64_text(1.5e21), "1.5e+21");
        assert_eq!(float64_text(1e-7), "1e-7");
        assert_eq!(float64_text(1.5e-7), "1.5e-7");
        assert_eq!(float64_text(0.000001), "0.000001");
        assert_eq!(float64_text(-0.0), "0");
        assert_eq!(
            float64_text(123456789012345680000.0),
            "123456789012345680000"
        );
        assert_eq!(float64_text(f64::NAN), "NaN");
        assert_eq!(float64_text(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn a_float32_prints_the_shortest_digits_that_round_trip_as_a_float32() {
        assert_eq!(float32_text(f64::from(0.1f32)), "0.1");
        assert_eq!(float32_text(f64::from(2.3f32 * 3.0f32)), "6.8999996");
        assert_eq!(float32_text(1.0), "1");
        assert_eq!(float32_text(0.0), "0");
    }
}
