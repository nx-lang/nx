//! Renderer-independent color validation and transformations.
use crate::NxValue;

pub const MODULE: &str = "@nx/color/color.nx";
pub const COLOR: &str = "@nx/color/color.nx::Color";
pub const NAMES: &[&str] = &[
    "rgb",
    "rgba",
    "srgb",
    "linearRgb",
    "hsl",
    "hsv",
    "hwb",
    "oklab",
    "oklch",
    "parse",
    "tryParse",
    "validate",
    "toHex",
    "toCss",
    "toRgb",
    "toHsl",
    "toHsv",
    "toHwb",
    "toLinearRgb",
    "toOklab",
    "toOklch",
    "withAlpha",
    "fadeIn",
    "fadeOut",
    "lighten",
    "darken",
    "saturate",
    "desaturate",
    "rotateHue",
    "complement",
    "grayscale",
    "invert",
    "mix",
    "composite",
    "relativeLuminance",
    "contrastRatio",
    "bestTextColor",
];
pub const ARITIES: &[usize] = &[
    3, 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 2,
    4, 2, 1, 2, 3,
];
#[derive(Debug, Clone)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
fn invalid(field: &str) -> Error {
    Error {
        code: "color-invalid-channel",
        message: format!("invalid color channel or parameter: {field}"),
    }
}
fn finite(x: f64, field: &str) -> Result<f64, Error> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(invalid(field))
    }
}
fn unit(x: f64, field: &str) -> Result<f64, Error> {
    finite(x, field)?;
    if (0.0..=1.0).contains(&x) {
        Ok(x)
    } else {
        Err(invalid(field))
    }
}
fn decode(x: f64) -> f64 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(x: f64) -> f64 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}
fn record(name: &str, fields: impl IntoIterator<Item = (String, NxValue)>) -> NxValue {
    NxValue::Record {
        type_name: Some(name.to_string()),
        properties: fields.into_iter().collect(),
    }
}
fn number(v: &NxValue) -> Result<f64, Error> {
    match v {
        NxValue::Float(x) => Ok(*x),
        NxValue::Float32(x) => Ok(*x as f64),
        NxValue::Int(x) => Ok(*x as f64),
        NxValue::Int32(x) => Ok(*x as f64),
        _ => Err(invalid("number")),
    }
}
pub fn make(c: [f64; 4]) -> Result<NxValue, Error> {
    for (i, x) in c.iter().enumerate() {
        unit(*x, ["r", "g", "b", "a"][i])?;
    }
    Ok(record(
        "Color",
        ["r", "g", "b", "a"].into_iter().zip(c).map(|(k, x)| {
            (
                k.into(),
                NxValue::Float32(if x == 0.0 { 0.0 } else { x as f32 }),
            )
        }),
    ))
}
pub fn channels(v: &NxValue) -> Result<[f64; 4], Error> {
    let NxValue::Record { properties, .. } = v else {
        return Err(invalid("Color"));
    };
    let mut out = [0.0; 4];
    for (i, k) in ["r", "g", "b", "a"].iter().enumerate() {
        let x = match properties.get(*k) {
            Some(value) => number(value)?,
            None if *k == "a" => 1.0,
            None => return Err(invalid(k)),
        };
        unit(x, k)?;
        if (x as f32) as f64 != x {
            return Err(invalid(k));
        }
        out[i] = x;
    }
    Ok(out)
}
pub fn parse(text: &str) -> Result<NxValue, Error> {
    let text = text.trim_matches(|c: char| c.is_ascii_whitespace());
    let lower = text.to_ascii_lowercase();
    let bad = || Error {
        code: "color-invalid-text",
        message: format!("invalid standalone color: {text}"),
    };
    let Some((name, tail)) = lower.split_once('(') else {
        if lower == "transparent" {
            return make([0.0; 4]);
        }
        if let Some(bytes) = csscolorparser::NAMED_COLORS.get(lower.as_str()) {
            return make([
                bytes[0] as f64 / 255.0,
                bytes[1] as f64 / 255.0,
                bytes[2] as f64 / 255.0,
                1.0,
            ]);
        }
        let digits = lower.strip_prefix('#').ok_or_else(bad)?;
        if !matches!(digits.len(), 3 | 4 | 6 | 8) || !digits.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(bad());
        }
        let mut hex = if digits.len() <= 4 {
            digits.chars().flat_map(|c| [c, c]).collect::<String>()
        } else {
            digits.to_string()
        };
        if hex.len() == 6 {
            hex.push_str("ff");
        }
        let mut channels = [0.0; 4];
        for (i, channel) in channels.iter_mut().enumerate() {
            *channel =
                u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| bad())? as f64 / 255.0;
        }
        return make(channels);
    };
    let mut body = tail.strip_suffix(')').ok_or_else(bad)?;
    let mut name = name;
    if name == "color" {
        body = body.trim_ascii();
        let split = body
            .find(|c: char| c.is_ascii_whitespace())
            .ok_or_else(bad)?;
        name = &body[..split];
        body = body[split..].trim_ascii();
        if !["srgb", "srgb-linear"].contains(&name) {
            return Err(bad());
        }
    }
    if ![
        "rgb",
        "rgba",
        "hsl",
        "hsla",
        "hwb",
        "oklab",
        "oklch",
        "srgb",
        "srgb-linear",
    ]
    .contains(&name)
    {
        return Err(bad());
    }
    let comma = body.contains(',');
    let parts: Vec<&str>;
    let alpha: &str;
    if comma {
        if !["rgb", "rgba", "hsl", "hsla"].contains(&name) || body.contains('/') {
            return Err(bad());
        }
        let all: Vec<_> = body.split(',').map(str::trim_ascii).collect();
        if all.len() != 3 && all.len() != 4 {
            return Err(bad());
        }
        alpha = all.get(3).copied().unwrap_or("1");
        parts = all[..3].to_vec();
    } else {
        let mut split = body.split('/');
        let channels = split.next().unwrap();
        alpha = split.next().unwrap_or("1").trim_ascii();
        if split.next().is_some() {
            return Err(bad());
        }
        parts = channels.split_ascii_whitespace().collect();
        if parts.len() != 3 {
            return Err(bad());
        }
    }
    let num = |token: &str, percent: f64, hue: bool| -> Result<f64, Error> {
        let (number, scale) = if let Some(n) = token.strip_suffix('%') {
            (n, percent / 100.0)
        } else if hue {
            if let Some(n) = token.strip_suffix("turn") {
                (n, 360.0)
            } else if let Some(n) = token.strip_suffix("grad") {
                (n, 0.9)
            } else if let Some(n) = token.strip_suffix("rad") {
                (n, 180.0 / std::f64::consts::PI)
            } else if let Some(n) = token.strip_suffix("deg") {
                (n, 1.0)
            } else {
                (token, 1.0)
            }
        } else {
            (token, 1.0)
        };
        if number.is_empty()
            || number
                .split('e')
                .next()
                .is_some_and(|mantissa| mantissa.ends_with('.'))
            || number
                .chars()
                .any(|c| !c.is_ascii_digit() && !['+', '-', '.', 'e'].contains(&c))
        {
            return Err(bad());
        }
        let n = number.parse::<f64>().map_err(|_| bad())? * scale;
        if n.is_finite() {
            Ok(n)
        } else {
            Err(bad())
        }
    };
    let alpha = num(alpha, 1.0, false)?.clamp(0.0, 1.0);
    let mut c = match name {
        "rgb" | "rgba" => {
            if comma
                && !(parts.iter().all(|s| s.ends_with('%'))
                    || parts.iter().all(|s| !s.ends_with('%')))
            {
                return Err(bad());
            }
            [
                num(parts[0], 255.0, false)?.clamp(0.0, 255.0) / 255.0,
                num(parts[1], 255.0, false)?.clamp(0.0, 255.0) / 255.0,
                num(parts[2], 255.0, false)?.clamp(0.0, 255.0) / 255.0,
                alpha,
            ]
        }
        "srgb" | "srgb-linear" => {
            let mut c = [
                num(parts[0], 1.0, false)?.clamp(0.0, 1.0),
                num(parts[1], 1.0, false)?.clamp(0.0, 1.0),
                num(parts[2], 1.0, false)?.clamp(0.0, 1.0),
                alpha,
            ];
            if name == "srgb-linear" {
                for v in &mut c[..3] {
                    *v = encode(*v);
                }
            }
            c
        }
        "hsl" | "hsla" | "hwb" => {
            if (comma && (!parts[1].ends_with('%') || !parts[2].ends_with('%')))
                || parts[0].ends_with('%')
            {
                return Err(bad());
            }
            let h = num(parts[0], 1.0, true)?.rem_euclid(360.0);
            let a = (num(parts[1], 100.0, false)? / 100.0).clamp(0.0, 1.0);
            let b = (num(parts[2], 100.0, false)? / 100.0).clamp(0.0, 1.0);
            if name == "hwb" {
                let sum = a + b;
                let rgb = if sum >= 1.0 {
                    [a / sum; 3]
                } else {
                    hsv(h, 1.0, 1.0).map(|x| x * (1.0 - sum) + a)
                };
                [rgb[0], rgb[1], rgb[2], alpha]
            } else {
                from_hsl([h, a, b, alpha])
            }
        }
        _ => {
            let l = num(parts[0], 1.0, false)?.clamp(0.0, 1.0);
            let x = num(parts[1], 0.4, false)?;
            let y = num(
                parts[2],
                if name == "oklab" { 0.4 } else { 1.0 },
                name == "oklch",
            )?;
            if name == "oklch" {
                if parts[2].ends_with('%') {
                    return Err(bad());
                }
                let h = y.rem_euclid(360.0).to_radians();
                from_lab([l, x.max(0.0) * h.cos(), x.max(0.0) * h.sin(), alpha])
            } else {
                from_lab([l, x, y, alpha])
            }
        }
    };
    for v in &mut c {
        if !v.is_finite() {
            return Err(bad());
        }
        *v = v.clamp(0.0, 1.0);
    }
    make(c).map_err(|_| bad())
}

fn wrapped_hue(h: f64) -> f64 {
    let h = h.rem_euclid(360.0);
    if h >= 360.0 || h == 0.0 {
        0.0
    } else {
        h
    }
}
fn hsv(h: f64, s: f64, v: f64) -> [f64; 3] {
    let h = wrapped_hue(h) / 60.0;
    let f = h - h.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match h as u8 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
fn hsl(c: [f64; 4]) -> [f64; 4] {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    let l = (max + min) / 2.0;
    if d == 0.0 {
        return [0.0, 0.0, l, c[3]];
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = 60.0
        * if max == c[0] {
            ((c[1] - c[2]) / d).rem_euclid(6.0)
        } else if max == c[1] {
            (c[2] - c[0]) / d + 2.0
        } else {
            (c[0] - c[1]) / d + 4.0
        };
    [wrapped_hue(h), s, l, c[3]]
}

fn from_hsl(c: [f64; 4]) -> [f64; 4] {
    let v = c[2] + c[1] * c[2].min(1.0 - c[2]);
    let s = if v == 0.0 {
        0.0
    } else {
        2.0 * (1.0 - c[2] / v)
    };
    let rgb = hsv(c[0], s, v);
    [rgb[0], rgb[1], rgb[2], c[3]]
}
fn lab(c: [f64; 4]) -> [f64; 4] {
    let [r, g, b] = [decode(c[0]), decode(c[1]), decode(c[2])];
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
        c[3],
    ]
}
fn from_lab(c: [f64; 4]) -> [f64; 4] {
    let [ll, a, b, alpha] = c;
    let l = (ll + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (ll - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (ll - 0.0894841775 * a - 1.291485548 * b).powi(3);
    [
        encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
        encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
        encode(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
        alpha,
    ]
}

fn clip(mut c: [f64; 4]) -> Result<NxValue, Error> {
    for x in &mut c {
        finite(*x, "result")?;
        *x = x.clamp(0.0, 1.0);
    }
    make(c)
}
fn luminance(c: [f64; 4]) -> Result<f64, Error> {
    if c[3] != 1.0 {
        return Err(invalid("alpha must be opaque"));
    }
    Ok(0.2126 * decode(c[0]) + 0.7152 * decode(c[1]) + 0.0722 * decode(c[2]))
}
fn contrast(a: [f64; 4], b: [f64; 4]) -> Result<f64, Error> {
    let a = luminance(a)?;
    let b = luminance(b)?;
    Ok((a.max(b) + 0.05) / (a.min(b) + 0.05))
}
/// Evaluates a closed standard-color operation on ordinary host values.
pub fn evaluate(op: u32, args: &[NxValue]) -> Result<NxValue, Error> {
    let name = *NAMES.get(op as usize).ok_or_else(|| invalid("operation"))?;
    if args.len() != ARITIES[op as usize] {
        return Err(invalid("arity"));
    }
    let n = |i: usize| number(&args[i]);
    let c = |i: usize| channels(&args[i]);
    match name {
        "parse" | "tryParse" => {
            let NxValue::String(text) = &args[0] else {
                return Err(invalid("text"));
            };
            let value = parse(text);
            if name == "parse" {
                value
            } else {
                Ok(match value {
                    Ok(v) => record("ParseResult.parsed", [("value".into(), v)]),
                    Err(e) => record(
                        "ParseResult.invalid",
                        [("message".into(), NxValue::String(e.message))],
                    ),
                })
            }
        }
        "rgb" | "rgba" | "srgb" | "linearRgb" | "hsl" | "hsv" | "hwb" | "oklab" | "oklch" => {
            let mut v = [
                n(0)?,
                n(1)?,
                n(2)?,
                if args.len() == 4 { n(3)? } else { 1.0 },
            ];
            unit(v[3], "alpha")?;
            for x in v {
                finite(x, "channel")?;
            }
            match name {
                "rgb" | "rgba" => {
                    for (index, x) in v[..3].iter_mut().enumerate() {
                        *x = unit(*x / 255.0, ["r", "g", "b"][index])?;
                    }
                }
                "srgb" => {}
                "linearRgb" => {
                    for x in &mut v[..3] {
                        *x = encode(unit(*x, "linearRgb")?);
                    }
                }
                "hsl" => {
                    unit(v[1], "s")?;
                    unit(v[2], "l")?;
                    v = from_hsl(v);
                }
                "hsv" => {
                    unit(v[1], "s")?;
                    unit(v[2], "v")?;
                    let rgb = hsv(v[0], v[1], v[2]);
                    v = [rgb[0], rgb[1], rgb[2], v[3]];
                }
                "hwb" => {
                    unit(v[1], "w")?;
                    unit(v[2], "b")?;
                    let sum = v[1] + v[2];
                    if sum >= 1.0 {
                        let x = v[1] / sum;
                        v = [x, x, x, v[3]];
                    } else {
                        let rgb = hsv(v[0], 1.0, 1.0).map(|x| x * (1.0 - sum) + v[1]);
                        v = [rgb[0], rgb[1], rgb[2], v[3]];
                    }
                }
                "oklab" => {
                    unit(v[0], "l")?;
                    v = from_lab(v);
                }
                "oklch" => {
                    unit(v[0], "l")?;
                    if v[1] < 0.0 {
                        return Err(invalid("chroma"));
                    }
                    let h = v[2].rem_euclid(360.0).to_radians();
                    v = from_lab([v[0], v[1] * h.cos(), v[1] * h.sin(), v[3]]);
                }
                _ => {}
            }
            if ["oklab", "oklch"].contains(&name) {
                clip(v)
            } else {
                make(v)
            }
        }
        "validate" => make(c(0)?),
        "toHex" => {
            let c = c(0)?;
            Ok(NxValue::String(format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (c[0] * 255.0 + 0.5).floor() as u8,
                (c[1] * 255.0 + 0.5).floor() as u8,
                (c[2] * 255.0 + 0.5).floor() as u8,
                (c[3] * 255.0 + 0.5).floor() as u8
            )))
        }
        "toCss" => {
            let c = c(0)?;
            Ok(NxValue::String(format!(
                "rgba({}, {}, {}, {})",
                c[0] * 255.0,
                c[1] * 255.0,
                c[2] * 255.0,
                c[3]
            )))
        }
        "toRgb" | "toHsl" | "toHsv" | "toHwb" | "toLinearRgb" | "toOklab" | "toOklch" => {
            let c = c(0)?;
            let (ty, keys, v) = match name {
                "toRgb" => (
                    "RgbChannels",
                    ["r", "g", "b", "alpha"],
                    [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0, c[3]],
                ),
                "toHsl" => ("HslChannels", ["h", "s", "l", "alpha"], hsl(c)),
                "toHsv" => ("HsvChannels", ["h", "s", "v", "alpha"], {
                    let max = c[0].max(c[1]).max(c[2]);
                    let min = c[0].min(c[1]).min(c[2]);
                    [
                        hsl(c)[0],
                        if max == 0.0 { 0.0 } else { (max - min) / max },
                        max,
                        c[3],
                    ]
                }),
                "toHwb" => (
                    "HwbChannels",
                    ["h", "w", "b", "alpha"],
                    [
                        hsl(c)[0],
                        c[0].min(c[1]).min(c[2]),
                        1.0 - c[0].max(c[1]).max(c[2]),
                        c[3],
                    ],
                ),
                "toLinearRgb" => (
                    "LinearRgbChannels",
                    ["r", "g", "b", "alpha"],
                    [decode(c[0]), decode(c[1]), decode(c[2]), c[3]],
                ),
                "toOklab" => ("OklabChannels", ["l", "a", "b", "alpha"], lab(c)),
                _ => ("OklchChannels", ["l", "c", "h", "alpha"], {
                    let lab = lab(c);
                    let chroma = lab[1].hypot(lab[2]);
                    if c[0] == c[1] && c[1] == c[2] {
                        [lab[0], 0.0, 0.0, c[3]]
                    } else {
                        [
                            lab[0],
                            chroma,
                            lab[2].atan2(lab[1]).to_degrees().rem_euclid(360.0),
                            c[3],
                        ]
                    }
                }),
            };
            Ok(record(
                ty,
                keys.into_iter()
                    .zip(v)
                    .map(|(k, v)| (k.into(), NxValue::Float(v))),
            ))
        }
        "withAlpha" | "fadeIn" | "fadeOut" | "invert" => {
            let mut v = c(0)?;
            let amount = unit(n(1)?, "amount")?;
            match name {
                "withAlpha" => v[3] = amount,
                "fadeIn" => v[3] = (v[3] + amount).min(1.0),
                "fadeOut" => v[3] = (v[3] - amount).max(0.0),
                _ => {
                    for x in &mut v[..3] {
                        *x = *x * (1.0 - amount) + (1.0 - *x) * amount;
                    }
                }
            }
            make(v)
        }
        "lighten" | "darken" | "saturate" | "desaturate" | "rotateHue" | "complement"
        | "grayscale" => {
            let mut v = hsl(c(0)?);
            match name {
                "complement" => v[0] = (v[0] + 180.0).rem_euclid(360.0),
                "grayscale" => v[1] = 0.0,
                "rotateHue" => v[0] = (v[0] + finite(n(1)?, "degrees")?).rem_euclid(360.0),
                _ => {
                    let amount = unit(n(1)?, "amount")?;
                    let i = if name == "lighten" || name == "darken" {
                        2
                    } else {
                        1
                    };
                    v[i] = (v[i]
                        + if name == "darken" || name == "desaturate" {
                            -amount
                        } else {
                            amount
                        })
                    .clamp(0.0, 1.0);
                }
            }
            make(from_hsl(v))
        }
        "relativeLuminance" => Ok(NxValue::Float(luminance(c(0)?)?)),
        "contrastRatio" => Ok(NxValue::Float(contrast(c(0)?, c(1)?)?)),
        "bestTextColor" => {
            let bg = c(0)?;
            let a = c(1)?;
            let b = c(2)?;
            make(if contrast(bg, a)? >= contrast(bg, b)? {
                a
            } else {
                b
            })
        }
        "mix" | "composite" => {
            let mut a = c(0)?;
            let mut b = c(1)?;
            let (weight, space) = if name == "mix" {
                (
                    unit(n(2)?, "weight")?,
                    match &args[3] {
                        NxValue::String(s) => s.as_str(),
                        _ => return Err(invalid("space")),
                    },
                )
            } else {
                (0.0, "linearSrgb")
            };
            if name == "mix" && weight == 0.0 {
                return make(a);
            }
            if name == "mix" && weight == 1.0 {
                return make(b);
            }
            if !["srgb", "linearSrgb", "oklab"].contains(&space) {
                return Err(invalid("space"));
            }
            if space == "oklab" {
                a = lab(a);
                b = lab(b);
            } else if space == "linearSrgb" {
                for x in &mut a[..3] {
                    *x = decode(*x);
                }
                for x in &mut b[..3] {
                    *x = decode(*x);
                }
            }
            let (wa, wb) = if name == "mix" {
                (1.0 - weight, weight)
            } else {
                (1.0, 1.0 - a[3])
            };
            let alpha = a[3] * wa + b[3] * wb;
            let mut out = [0.0, 0.0, 0.0, alpha];
            if alpha > 0.0 {
                for i in 0..3 {
                    out[i] = (a[i] * a[3] * wa + b[i] * b[3] * wb) / alpha;
                }
                if space == "oklab" {
                    out = from_lab(out);
                } else if space == "linearSrgb" {
                    for x in &mut out[..3] {
                        *x = encode(*x);
                    }
                }
            }
            clip(out)
        }
        _ => Err(invalid("operation")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn call(name: &str, args: &[NxValue]) -> NxValue {
        evaluate(NAMES.iter().position(|n| *n == name).unwrap() as u32, args).unwrap()
    }
    #[test]
    fn parsing_and_float32_roundtrip() {
        for text in [
            "red",
            "#f008",
            "#12345678",
            "rgb(255, 0, 0)",
            "hsl(180 100% 50%)",
        ] {
            let v = parse(text).unwrap();
            let NxValue::String(css) = call("toCss", std::slice::from_ref(&v)) else {
                panic!()
            };
            assert_eq!(v, parse(&css).unwrap(), "{text}");
        }
        for text in ["#12", "#gg0000", "currentColor", "var(--accent)", "none"] {
            assert_eq!(parse(text).unwrap_err().code, "color-invalid-text");
        }
        assert!(make([f64::NAN, 0.0, 0.0, 1.0]).is_err());
    }
    #[test]
    fn mixing_and_contrast_reference() {
        let black = parse("black").unwrap();
        let white = parse("white").unwrap();
        assert_eq!(
            call("contrastRatio", &[black.clone(), white.clone()]),
            NxValue::Float(21.0)
        );
        let out = call(
            "mix",
            &[
                black,
                white,
                NxValue::Float(0.5),
                NxValue::String("linearSrgb".into()),
            ],
        );
        assert!((channels(&out).unwrap()[0] - 0.735356983).abs() < 1e-6);
        let out = call(
            "mix",
            &[
                parse("red").unwrap(),
                parse("#0000ff00").unwrap(),
                NxValue::Float(0.5),
                NxValue::String("srgb".into()),
            ],
        );
        assert_eq!(channels(&out).unwrap(), [1.0, 0.0, 0.0, 0.5]);
    }
}
