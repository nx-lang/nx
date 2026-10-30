use crate::{absent, error, numeric, Limits, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f64,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Paint {
    Color {
        color: [f32; 4],
    },
    LinearGradient {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stops: Vec<GradientStop>,
    },
    RadialGradient {
        cx: f64,
        cy: f64,
        radius: f64,
        stops: Vec<GradientStop>,
    },
}
// Skia collapses shorter gradients to their final color, regardless of the canvas scale.
const MIN_GRADIENT_SPAN: f64 = 1.0 / 32768.0;
impl Paint {
    pub(crate) fn validate_span(&self, scale: f64, path: &str, property: &str) -> Result<()> {
        self.validate_text_span(0.0, 0.0, scale, path, property)
    }
    pub(crate) fn validate_text_span(
        &self,
        origin_x: f64,
        origin_y: f64,
        scale: f64,
        path: &str,
        property: &str,
    ) -> Result<()> {
        let coordinate = |value: f64, origin: f64| -> Result<f32> {
            let value = ((value - origin) / scale) as f32;
            if !value.is_finite() {
                return Err(error(
                    path,
                    format!(
                        "{property}: text-normalized gradient coordinate outside backend range"
                    ),
                ));
            }
            Ok(value)
        };
        let span = match self {
            Self::Color { .. } => return Ok(()),
            Self::LinearGradient { x1, y1, x2, y2, .. } => {
                ((coordinate(*x2, origin_x)? - coordinate(*x1, origin_x)?) as f64)
                    .hypot((coordinate(*y2, origin_y)? - coordinate(*y1, origin_y)?) as f64)
            }
            Self::RadialGradient { cx, cy, radius, .. } => {
                coordinate(*cx, origin_x)?;
                coordinate(*cy, origin_y)?;
                coordinate(*radius, 0.0)? as f64
            }
        };
        // Leave a small margin for float32 length/normalization rounding at the backend.
        if span <= MIN_GRADIENT_SPAN * (1.0 + 4.0 * f32::EPSILON as f64) {
            return Err(error(path, format!("{property}: gradient span must exceed 1/32768 backend units with a float32 rounding margin (after text normalization)")));
        }
        Ok(())
    }

    pub fn visible(&self) -> bool {
        match self {
            Self::Color { color } => color[3] > 0.0,
            Self::LinearGradient { stops, .. } | Self::RadialGradient { stops, .. } => {
                stops.iter().any(|s| s.color[3] > 0.0)
            }
        }
    }
    pub fn has_transparency(&self) -> bool {
        match self {
            Self::Color { .. } => false,
            Self::LinearGradient { stops, .. } | Self::RadialGradient { stops, .. } => {
                stops.iter().any(|s| s.color[3] < 1.0)
            }
        }
    }
}
fn color(value: &Value, path: &str, property: &str) -> Result<[f32; 4]> {
    let value: nx_value::NxValue = serde_json::from_value(value.clone())
        .map_err(|_| error(path, format!("{property}: invalid Color record")))?;
    nx_value::color::channels(&value)
        .map(|c| c.map(|x| x as f32))
        .map_err(|failure| {
            let mut d = error(path, format!("{property}: {}", failure.message));
            d.code = failure.code.into();
            d
        })
}
pub(crate) fn prepare(
    v: Option<&Value>,
    default: Option<Paint>,
    path: &str,
    property: &str,
    limits: &Limits,
    used: &mut usize,
) -> Result<Option<Paint>> {
    let Some(v) = v.filter(|v| !absent(v)) else {
        return Ok(default);
    };
    if v.as_str() == Some("none") {
        return Ok(None);
    }
    let kind = v
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .rsplit('.')
        .next()
        .unwrap_or("");
    if kind == "color" {
        let c = color(
            v.get("color")
                .ok_or_else(|| error(path, format!("{property}: solid requires Color")))?,
            path,
            property,
        )?;
        return Ok((c[3] > 0.0).then_some(Paint::Color { color: c }));
    }
    if !["linearGradient", "radialGradient"].contains(&kind) {
        return Err(error(path, format!("{property}: unsupported paint")));
    }
    let values = v
        .get("stops")
        .and_then(Value::as_array)
        .ok_or_else(|| error(path, format!("{property}.stops: expected sequence")))?;
    if values.len() < 2 {
        return Err(error(
            path,
            format!("{property}.stops: at least two stops required"),
        ));
    }
    *used = used
        .checked_add(values.len())
        .filter(|n| *n <= limits.gradient_stops)
        .ok_or_else(|| error(path, format!("{property}: gradient_stops limit exceeded")))?;
    let mut stops = Vec::with_capacity(values.len());
    let mut previous = 0.0;
    for (i, value) in values.iter().enumerate() {
        let name = format!("{property}.stops[{i}]");
        let offset = value
            .get("offset")
            .and_then(Value::as_f64)
            .filter(|x| x.is_finite() && *x >= previous && *x <= 1.0)
            .ok_or_else(|| {
                error(
                    path,
                    format!("{name}.offset: expected nondecreasing offsets in [0,1]"),
                )
            })?;
        let c = color(
            value
                .get("color")
                .ok_or_else(|| error(path, format!("{name}.color: missing Color")))?,
            path,
            &name,
        )?;
        stops.push(GradientStop { offset, color: c });
        previous = offset;
    }
    let n = |key: &str| -> Result<f64> {
        let value = v
            .get(key)
            .ok_or_else(|| error(path, format!("{property}.{key}: missing coordinate")))?;
        numeric(value, path, limits.coordinate)
            .map_err(|_| error(path, format!("{property}.{key}: coordinate outside range")))
    };
    let paint = if kind == "linearGradient" {
        let (x1, y1, x2, y2) = (n("x1")?, n("y1")?, n("x2")?, n("y2")?);
        if (x1 as f32, y1 as f32) == (x2 as f32, y2 as f32) {
            return Err(error(
                path,
                format!("{property}: linear endpoints must differ at backend precision"),
            ));
        }
        Paint::LinearGradient {
            x1,
            y1,
            x2,
            y2,
            stops,
        }
    } else {
        let (cx, cy, radius) = (n("cx")?, n("cy")?, n("radius")?);
        if radius <= 0.0 || radius as f32 <= 0.0 {
            return Err(error(
                path,
                format!("{property}.radius: must be positive at backend precision"),
            ));
        }
        Paint::RadialGradient {
            cx,
            cy,
            radius,
            stops,
        }
    };
    paint.validate_span(1.0, path, property)?;
    Ok(Some(paint))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_text_endpoints_are_checked_at_backend_precision() {
        let paint = Paint::LinearGradient {
            x1: 0.0,
            y1: 0.0,
            x2: 0.000032,
            y2: 0.0,
            stops: vec![],
        };
        assert!(paint.validate_span(1.0, "", "fill").is_ok());
        assert!(paint
            .validate_text_span(-1500.0, 0.0, 0.1, "", "fill")
            .is_err());
        assert!(paint.validate_text_span(0.0, 0.0, 0.1, "", "fill").is_ok());
    }
    #[test]
    fn tiny_gradients_and_large_text_are_rejected() {
        for span in [0.00002, 0.00005, 1.0 / 32768.0, 2.0 / 32768.0, 0.001] {
            for paint in [
                Paint::LinearGradient {
                    x1: 0.0,
                    y1: 0.0,
                    x2: span,
                    y2: 0.0,
                    stops: vec![],
                },
                Paint::RadialGradient {
                    cx: 0.0,
                    cy: 0.0,
                    radius: span,
                    stops: vec![],
                },
            ] {
                assert_eq!(
                    paint.validate_span(1.0, "", "fill").is_ok(),
                    span > MIN_GRADIENT_SPAN
                );
                assert_eq!(
                    paint.validate_span(2.0, "", "fill").is_ok(),
                    span > 2.0 * MIN_GRADIENT_SPAN
                );
            }
        }
    }
}
