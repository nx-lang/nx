//! Renderer-independent color validation and transformations.
//!
//! <para>A `Color` is a union of color spaces: `srgb` and `displayP3` hold encoded RGB, `oklab`
//! and `oklch` hold perceptual coordinates that may lie outside every screen's gamut. Conversions
//! go through unclamped linear-light sRGB, and only [`to_gamut`] and the functions documented as
//! mapping into sRGB ever bring a color inside a gamut.</para>
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
    "displayP3",
    "oklab",
    "oklch",
    "parse",
    "tryParse",
    "validate",
    "inGamut",
    "toGamut",
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
    3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2,
    1, 1, 2, 4, 2, 1, 2, 3,
];

/// How far outside [0, 1] a converted channel may be and still count as inside a gamut.
pub const GAMUT_TOLERANCE: f64 = 2e-6;
/// CSS Color 4 gamut mapping: the just-noticeable difference in ΔE_OK.
const JND: f64 = 0.02;
/// CSS Color 4 gamut mapping: the chroma search's precision.
const MAP_EPSILON: f64 = 0.0001;

/// A color space a `Color` can be stored in: one case of the `Color` union.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Srgb,
    DisplayP3,
    Oklab,
    Oklch,
}

impl Space {
    /// The union case's name.
    pub fn case(self) -> &'static str {
        match self {
            Space::Srgb => "srgb",
            Space::DisplayP3 => "displayP3",
            Space::Oklab => "oklab",
            Space::Oklch => "oklch",
        }
    }
    /// The space a union case names.
    pub fn from_case(case: &str) -> Option<Self> {
        Some(match case {
            "srgb" => Space::Srgb,
            "displayP3" => Space::DisplayP3,
            "oklab" => Space::Oklab,
            "oklch" => Space::Oklch,
            _ => return None,
        })
    }
    /// The case's field names, in storage order.
    pub fn keys(self) -> [&'static str; 4] {
        match self {
            Space::Srgb | Space::DisplayP3 => ["r", "g", "b", "alpha"],
            Space::Oklab => ["l", "a", "b", "alpha"],
            Space::Oklch => ["l", "c", "h", "alpha"],
        }
    }
    /// Whether the space is a gamut: encoded RGB limited to [0, 1].
    pub fn is_rgb(self) -> bool {
        matches!(self, Space::Srgb | Space::DisplayP3)
    }
    fn rank(self) -> u8 {
        match self {
            Space::Srgb => 0,
            Space::DisplayP3 => 1,
            Space::Oklab | Space::Oklch => 2,
        }
    }
}

/// A validated color: its space and its four stored channels, alpha last.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub space: Space,
    pub c: [f64; 4],
}

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
/// The sRGB transfer curve, extended to negative values by symmetry as CSS Color 4 does.
fn decode(x: f64) -> f64 {
    let a = x.abs();
    let y = if a <= 0.04045 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    };
    y.copysign(x)
}
fn encode(x: f64) -> f64 {
    let a = x.abs();
    let y = if a <= 0.0031308 {
        a * 12.92
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    };
    y.copysign(x)
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
fn narrow(x: f64) -> f64 {
    let x = x as f32 as f64;
    if x == 0.0 {
        0.0
    } else {
        x
    }
}
/// Wraps finite degrees into [0, 360), also after float32 rounding.
fn hue(h: f64) -> f64 {
    // Adding zero turns the -0.0 that `rem_euclid` leaves for -360 into 0.0, as TypeScript gives.
    let h = h.rem_euclid(360.0) + 0.0;
    if narrow(h) >= 360.0 {
        0.0
    } else {
        h
    }
}

/// Checks channels for their space and writes them as a `Color` union-case record, rounding each
/// to float32.
pub fn make_in(space: Space, c: [f64; 4]) -> Result<NxValue, Error> {
    let keys = space.keys();
    for (i, x) in c.iter().enumerate() {
        match (space, i) {
            (Space::Oklab, 1 | 2) => {
                finite(*x, keys[i])?;
            }
            (Space::Oklch, 1) => {
                if finite(*x, keys[i])? < 0.0 {
                    return Err(invalid(keys[i]));
                }
            }
            (Space::Oklch, 2) => {
                if !(0.0..360.0).contains(&finite(*x, keys[i])?) {
                    return Err(invalid(keys[i]));
                }
            }
            _ => {
                unit(*x, keys[i])?;
            }
        }
    }
    Ok(record(
        &format!("Color.{}", space.case()),
        keys.into_iter().zip(c).map(|(k, x)| {
            let x = if space == Space::Oklch && k == "h" {
                hue(x)
            } else {
                narrow(x)
            };
            (k.into(), NxValue::Float32(x as f32))
        }),
    ))
}
/// Writes encoded sRGB channels and alpha as `Color.srgb`.
pub fn make(c: [f64; 4]) -> Result<NxValue, Error> {
    make_in(Space::Srgb, c)
}
/// Writes a computed color in `space`: a perceptual color has its lightness clamped and its hue
/// wrapped; an RGB color must already be inside its gamut.
fn finish(space: Space, mut c: [f64; 4]) -> Result<NxValue, Error> {
    for x in &c {
        finite(*x, "result")?;
    }
    match space {
        Space::Oklab => c[0] = c[0].clamp(0.0, 1.0),
        Space::Oklch => {
            c[0] = c[0].clamp(0.0, 1.0);
            c[1] = c[1].max(0.0);
            c[2] = hue(c[2]);
        }
        _ => {}
    }
    c[3] = c[3].clamp(0.0, 1.0);
    make_in(space, c)
}

/// Reads a `Color` union-case record, checking every channel for its space.
pub fn read(v: &NxValue) -> Result<Color, Error> {
    let NxValue::Record {
        type_name,
        properties,
    } = v
    else {
        return Err(invalid("Color"));
    };
    let name = type_name.as_deref().unwrap_or("");
    let name = name.rsplit("::").next().unwrap_or(name);
    let space = name
        .rsplit_once('.')
        .filter(|(union, _)| *union == "Color" || union.ends_with(".Color"))
        .and_then(|(_, case)| Space::from_case(case))
        .ok_or_else(|| invalid("Color"))?;
    let keys = space.keys();
    let mut c = [0.0; 4];
    for (i, k) in keys.iter().enumerate() {
        let x = match properties.get(*k) {
            Some(value) => number(value)?,
            None if *k == "alpha" => 1.0,
            None => return Err(invalid(k)),
        };
        finite(x, k)?;
        if (x as f32) as f64 != x {
            return Err(invalid(k));
        }
        c[i] = x;
    }
    // Checks the ranges and the keys present together.
    make_in(space, c)?;
    if properties.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err(invalid("Color"));
    }
    Ok(Color { space, c })
}
/// The channels of a `Color.srgb`; any other space is refused, so no caller reads P3 or Oklch
/// numbers as sRGB.
pub fn channels(v: &NxValue) -> Result<[f64; 4], Error> {
    let color = read(v)?;
    if color.space != Space::Srgb {
        return Err(invalid("Color.srgb"));
    }
    Ok(color.c)
}
/// The color as an sRGB screen shows it: mapped into sRGB and rounded to float32.
pub fn srgb_channels(v: &NxValue) -> Result<[f64; 4], Error> {
    Ok(to_gamut(read(v)?, Space::Srgb).map(narrow))
}

// ------------------------------------------------------------------------------------------------
// Conversions. The hub is linear-light sRGB with unbounded channels.
// ------------------------------------------------------------------------------------------------

const P3_TO_SRGB: [[f64; 3]; 3] = [
    [1.2249401762805596, -0.22494017628055998, 0.0],
    [-0.04205695470968817, 1.0420569547096883, 0.0],
    [
        -0.019637554590334432,
        -0.07863604555063192,
        1.0982736001409663,
    ],
];
const SRGB_TO_P3: [[f64; 3]; 3] = [
    [0.8224619687143624, 0.17753803128563778, 0.0],
    [0.03319419885096163, 0.9668058011490381, 0.0],
    [0.01708263072112004, 0.07239744066396347, 0.9105199286149165],
];
fn mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
}
fn lin_to_lab(v: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = v;
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ]
}
fn lab_to_lin(v: [f64; 3]) -> [f64; 3] {
    let [ll, a, b] = v;
    let l = (ll + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (ll - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (ll - 0.0894841775 * a - 1.291485548 * b).powi(3);
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
    ]
}
fn lab_to_lch(v: [f64; 3]) -> [f64; 3] {
    [v[0], v[1].hypot(v[2]), hue(v[2].atan2(v[1]).to_degrees())]
}
fn lch_to_lab(v: [f64; 3]) -> [f64; 3] {
    let h = v[2].to_radians();
    [v[0], v[1] * h.cos(), v[1] * h.sin()]
}
fn rgb3(c: [f64; 4]) -> [f64; 3] {
    [c[0], c[1], c[2]]
}
fn with_alpha(v: [f64; 3], alpha: f64) -> [f64; 4] {
    [v[0], v[1], v[2], alpha]
}
/// Linear-light sRGB, unbounded.
fn to_lin(color: Color) -> [f64; 3] {
    let v = rgb3(color.c);
    match color.space {
        Space::Srgb => v.map(decode),
        Space::DisplayP3 => mul(&P3_TO_SRGB, v.map(decode)),
        Space::Oklab => lab_to_lin(v),
        Space::Oklch => lab_to_lin(lch_to_lab(v)),
    }
}
fn from_lin(space: Space, v: [f64; 3]) -> [f64; 3] {
    match space {
        Space::Srgb => v.map(encode),
        Space::DisplayP3 => mul(&SRGB_TO_P3, v).map(encode),
        Space::Oklab => lin_to_lab(v),
        Space::Oklch => lab_to_lch(lin_to_lab(v)),
    }
}
/// The color's channels in `space`, unclamped, alpha kept.
fn convert(color: Color, space: Space) -> [f64; 4] {
    let alpha = color.c[3];
    let v = rgb3(color.c);
    let out = match (color.space, space) {
        (a, b) if a == b => v,
        (Space::Oklab, Space::Oklch) => lab_to_lch(v),
        (Space::Oklch, Space::Oklab) => lch_to_lab(v),
        (Space::Srgb | Space::DisplayP3, Space::Oklch) if v[0] == v[1] && v[1] == v[2] => {
            // An RGB gray has no hue.
            [lin_to_lab(to_lin(color))[0], 0.0, 0.0]
        }
        _ => from_lin(space, to_lin(color)),
    };
    with_alpha(out, alpha)
}
fn oklab_of(color: Color) -> [f64; 3] {
    rgb3(convert(color, Space::Oklab))
}
fn rgb_in_gamut(v: [f64; 3]) -> bool {
    v.iter()
        .all(|x| (-GAMUT_TOLERANCE..=1.0 + GAMUT_TOLERANCE).contains(x))
}
/// Whether a screen with `gamut` can show the color.
pub fn in_gamut(color: Color, gamut: Space) -> bool {
    color.space == gamut || rgb_in_gamut(rgb3(convert(color, gamut)))
}
/// The color in `gamut`'s encoded RGB, mapped with the CSS Color 4 algorithm when outside it.
pub fn to_gamut(color: Color, gamut: Space) -> [f64; 4] {
    debug_assert!(gamut.is_rgb());
    let alpha = color.c[3];
    if color.space == gamut {
        return color.c;
    }
    let origin = rgb3(convert(color, Space::Oklch));
    if origin[0] >= 1.0 {
        return [1.0, 1.0, 1.0, alpha];
    }
    if origin[0] <= 0.0 {
        return [0.0, 0.0, 0.0, alpha];
    }
    let direct = rgb3(convert(color, gamut));
    if rgb_in_gamut(direct) {
        return with_alpha(direct.map(|x| x.clamp(0.0, 1.0)), alpha);
    }
    let dest = |lch: [f64; 3]| from_lin(gamut, lab_to_lin(lch_to_lab(lch)));
    let clip = |v: [f64; 3]| v.map(|x| x.clamp(0.0, 1.0));
    let delta = |clipped: [f64; 3], lch: [f64; 3]| {
        let a = lin_to_lab(to_lin(Color {
            space: gamut,
            c: with_alpha(clipped, alpha),
        }));
        let b = lch_to_lab(lch);
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    let mut current = origin;
    let mut clipped = clip(dest(current));
    if delta(clipped, current) < JND {
        return with_alpha(clipped, alpha);
    }
    let (mut min, mut max) = (0.0, origin[1]);
    let mut min_in_gamut = true;
    while max - min > MAP_EPSILON {
        let chroma = (min + max) / 2.0;
        current[1] = chroma;
        let rgb = dest(current);
        if min_in_gamut && rgb_in_gamut(rgb) {
            min = chroma;
            continue;
        }
        clipped = clip(rgb);
        let e = delta(clipped, current);
        if e < JND {
            if JND - e < MAP_EPSILON {
                break;
            }
            min_in_gamut = false;
            min = chroma;
        } else {
            max = chroma;
        }
    }
    with_alpha(clipped, alpha)
}
/// Writes linear-light sRGB as a color in `space`, mapping into the gamut of an RGB space.
fn from_lin_in(space: Space, lin: [f64; 3], alpha: f64) -> Result<NxValue, Error> {
    for x in lin {
        finite(x, "result")?;
    }
    if space.is_rgb() {
        let direct = from_lin(space, lin);
        let c = if rgb_in_gamut(direct) {
            with_alpha(direct.map(|x| x.clamp(0.0, 1.0)), alpha)
        } else {
            let lab = lin_to_lab(lin);
            let color = Color {
                space: Space::Oklab,
                c: with_alpha(lab, alpha),
            };
            to_gamut(color, space)
        };
        return make_in(space, c);
    }
    finish(space, with_alpha(from_lin(space, lin), alpha))
}
/// Writes an Oklch color in `space`, mapping into the gamut of an RGB space.
fn from_lch_in(space: Space, lch: [f64; 4]) -> Result<NxValue, Error> {
    for x in lch {
        finite(x, "result")?;
    }
    let lch = [lch[0].clamp(0.0, 1.0), lch[1].max(0.0), hue(lch[2]), lch[3]];
    let color = Color {
        space: Space::Oklch,
        c: lch,
    };
    if space.is_rgb() {
        make_in(space, to_gamut(color, space))
    } else {
        finish(space, convert(color, space))
    }
}
fn wider(a: Space, b: Space) -> Space {
    if b.rank() > a.rank() {
        b
    } else {
        a
    }
}
/// Formats a number as JavaScript's `String(number)` does, so both runtimes write the same text.
fn js_number(x: f64) -> String {
    if x == 0.0 {
        "0".into()
    } else if x.abs() < 1e-6 {
        format!("{x:e}")
    } else {
        format!("{x}")
    }
}
/// Standalone CSS text for the color, in its own space.
pub fn css(color: Color) -> String {
    let [a, b, c, alpha] = color.c.map(js_number);
    match color.space {
        Space::Srgb => format!(
            "rgb({} {} {} / {alpha})",
            js_number(color.c[0] * 255.0),
            js_number(color.c[1] * 255.0),
            js_number(color.c[2] * 255.0)
        ),
        Space::DisplayP3 => format!("color(display-p3 {a} {b} {c} / {alpha})"),
        Space::Oklab => format!("oklab({a} {b} {c} / {alpha})"),
        Space::Oklch => format!("oklch({a} {b} {c} / {alpha})"),
    }
}
/// Text a host paints a swatch from: `#rrggbbaa` for sRGB, as swatches have always been, and the
/// color's own CSS for every other space, which browsers map for the screen they draw on.
pub fn swatch(color: Color) -> String {
    if color.space == Space::Srgb {
        let [r, g, b, a] = color.c.map(|x| (x * 255.0).round() as u8);
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    } else {
        css(color)
    }
}
/// Whether an sRGB screen shows the color differently from how it was written.
pub fn outside_srgb(color: Color) -> bool {
    !in_gamut(color, Space::Srgb)
}
fn space_arg(v: &NxValue, field: &str) -> Result<Space, Error> {
    match v {
        NxValue::String(s) if s == "srgb" => Ok(Space::Srgb),
        NxValue::String(s) if s == "displayP3" => Ok(Space::DisplayP3),
        _ => Err(invalid(field)),
    }
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
    // The predefined spaces are written only inside `color()`: `display-p3(…)` is not CSS.
    if matches!(name, "srgb" | "srgb-linear" | "display-p3") {
        return Err(bad());
    }
    if name == "color" {
        body = body.trim_ascii();
        let split = body
            .find(|c: char| c.is_ascii_whitespace())
            .ok_or_else(bad)?;
        name = &body[..split];
        body = body[split..].trim_ascii();
        if !["srgb", "srgb-linear", "display-p3"].contains(&name) {
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
        "display-p3",
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
    let mut space = Space::Srgb;
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
        "srgb" | "srgb-linear" | "display-p3" => {
            if name == "display-p3" {
                space = Space::DisplayP3;
            }
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
            if name == "oklch" {
                if parts[2].ends_with('%') {
                    return Err(bad());
                }
                space = Space::Oklch;
                [l, x.max(0.0), num(parts[2], 1.0, true)?, alpha]
            } else {
                space = Space::Oklab;
                [l, x, num(parts[2], 0.4, false)?, alpha]
            }
        }
    };
    for v in &mut c {
        if !v.is_finite() {
            return Err(bad());
        }
    }
    if matches!(space, Space::Srgb | Space::DisplayP3) {
        for v in &mut c {
            *v = v.clamp(0.0, 1.0);
        }
    }
    finish(space, c).map_err(|_| bad())
}
fn wrapped_hue(h: f64) -> f64 {
    let h = h.rem_euclid(360.0) + 0.0;
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

fn luminance(c: [f64; 4]) -> Result<f64, Error> {
    if c[3] != 1.0 {
        return Err(invalid("alpha must be opaque"));
    }
    Ok(0.2126 * decode(c[0]) + 0.7152 * decode(c[1]) + 0.0722 * decode(c[2]))
}
/// The color as an sRGB screen shows it, for the functions defined only in sRGB.
fn shown(color: Color) -> [f64; 4] {
    to_gamut(color, Space::Srgb).map(narrow)
}
fn contrast(a: Color, b: Color) -> Result<f64, Error> {
    let a = luminance(shown(a))?;
    let b = luminance(shown(b))?;
    Ok((a.max(b) + 0.05) / (a.min(b) + 0.05))
}
fn write(color: Color) -> Result<NxValue, Error> {
    make_in(color.space, color.c)
}
/// Evaluates a closed standard-color operation on ordinary host values.
pub fn evaluate(op: u32, args: &[NxValue]) -> Result<NxValue, Error> {
    let name = *NAMES.get(op as usize).ok_or_else(|| invalid("operation"))?;
    if args.len() != ARITIES[op as usize] {
        return Err(invalid("arity"));
    }
    let n = |i: usize| number(&args[i]);
    let c = |i: usize| read(&args[i]);
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
        "rgb" | "rgba" | "srgb" | "linearRgb" | "hsl" | "hsv" | "hwb" | "displayP3" | "oklab"
        | "oklch" => {
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
                "displayP3" => return make_in(Space::DisplayP3, v),
                "oklab" => {
                    unit(v[0], "l")?;
                    return make_in(Space::Oklab, v);
                }
                "oklch" => {
                    unit(v[0], "l")?;
                    if v[1] < 0.0 {
                        return Err(invalid("c"));
                    }
                    v[2] = hue(v[2]);
                    return make_in(Space::Oklch, v);
                }
                _ => {}
            }
            make(v)
        }
        "validate" => write(c(0)?),
        "inGamut" => Ok(NxValue::Bool(in_gamut(
            c(0)?,
            space_arg(&args[1], "gamut")?,
        ))),
        "toGamut" => {
            let gamut = space_arg(&args[1], "gamut")?;
            make_in(gamut, to_gamut(c(0)?, gamut))
        }
        "toHex" => {
            let c = shown(c(0)?);
            Ok(NxValue::String(format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (c[0] * 255.0 + 0.5).floor() as u8,
                (c[1] * 255.0 + 0.5).floor() as u8,
                (c[2] * 255.0 + 0.5).floor() as u8,
                (c[3] * 255.0 + 0.5).floor() as u8
            )))
        }
        "toCss" => Ok(NxValue::String(css(c(0)?))),
        "toOklab" | "toOklch" => {
            let color = c(0)?;
            let (ty, space) = if name == "toOklab" {
                ("OklabChannels", Space::Oklab)
            } else {
                ("OklchChannels", Space::Oklch)
            };
            Ok(record(
                ty,
                space
                    .keys()
                    .into_iter()
                    .zip(convert(color, space))
                    .map(|(k, v)| (k.into(), NxValue::Float(v))),
            ))
        }
        "toRgb" | "toHsl" | "toHsv" | "toHwb" | "toLinearRgb" => {
            let c = shown(c(0)?);
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
                _ => (
                    "LinearRgbChannels",
                    ["r", "g", "b", "alpha"],
                    [decode(c[0]), decode(c[1]), decode(c[2]), c[3]],
                ),
            };
            Ok(record(
                ty,
                keys.into_iter()
                    .zip(v)
                    .map(|(k, v)| (k.into(), NxValue::Float(v))),
            ))
        }
        "withAlpha" | "fadeIn" | "fadeOut" => {
            let mut color = c(0)?;
            let amount = unit(n(1)?, "amount")?;
            let a = &mut color.c[3];
            match name {
                "withAlpha" => *a = amount,
                "fadeIn" => *a = (*a + amount).min(1.0),
                _ => *a = (*a - amount).max(0.0),
            }
            write(color)
        }
        "invert" => {
            let color = c(0)?;
            let amount = unit(n(1)?, "amount")?;
            let (space, mut v) = if color.space.is_rgb() {
                (color.space, color.c)
            } else {
                (Space::DisplayP3, to_gamut(color, Space::DisplayP3))
            };
            for x in &mut v[..3] {
                *x = (*x * (1.0 - amount) + (1.0 - *x) * amount).clamp(0.0, 1.0);
            }
            make_in(space, v)
        }
        "lighten" | "darken" | "saturate" | "desaturate" | "rotateHue" | "complement"
        | "grayscale" => {
            let color = c(0)?;
            let mut v = convert(color, Space::Oklch);
            match name {
                "complement" => v[2] += 180.0,
                "grayscale" => v[1] = 0.0,
                "rotateHue" => v[2] += finite(n(1)?, "degrees")?,
                _ => {
                    let amount = unit(n(1)?, "amount")?;
                    match name {
                        "lighten" => v[0] = (v[0] + amount).clamp(0.0, 1.0),
                        "darken" => v[0] = (v[0] - amount).clamp(0.0, 1.0),
                        "saturate" => v[1] *= 1.0 + amount,
                        _ => v[1] *= 1.0 - amount,
                    }
                }
            }
            from_lch_in(color.space, v)
        }
        "relativeLuminance" => Ok(NxValue::Float(luminance(shown(c(0)?))?)),
        "contrastRatio" => Ok(NxValue::Float(contrast(c(0)?, c(1)?)?)),
        "bestTextColor" => {
            let bg = c(0)?;
            let a = c(1)?;
            let b = c(2)?;
            write(if contrast(bg, a)? >= contrast(bg, b)? {
                a
            } else {
                b
            })
        }
        "mix" | "composite" => {
            let a = c(0)?;
            let b = c(1)?;
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
            if !["srgb", "linearSrgb", "oklab"].contains(&space) {
                return Err(invalid("space"));
            }
            if name == "mix" && weight == 0.0 {
                return write(a);
            }
            if name == "mix" && weight == 1.0 {
                return write(b);
            }
            let result = wider(a.space, b.space);
            let coords = |color: Color| -> [f64; 3] {
                match space {
                    "oklab" => oklab_of(color),
                    "srgb" => to_lin(color).map(encode),
                    _ => to_lin(color),
                }
            };
            let (pa, pb) = (coords(a), coords(b));
            let (wa, wb) = if name == "mix" {
                (1.0 - weight, weight)
            } else {
                (1.0, 1.0 - a.c[3])
            };
            let alpha = a.c[3] * wa + b.c[3] * wb;
            if alpha <= 0.0 {
                return make_in(result, [0.0; 4]);
            }
            let mut out = [0.0; 3];
            for i in 0..3 {
                out[i] = (pa[i] * a.c[3] * wa + pb[i] * b.c[3] * wb) / alpha;
            }
            if space == "oklab" && !result.is_rgb() {
                let c = with_alpha(out, alpha);
                return finish(
                    result,
                    if result == Space::Oklch {
                        convert(
                            Color {
                                space: Space::Oklab,
                                c,
                            },
                            Space::Oklch,
                        )
                    } else {
                        c
                    },
                );
            }
            let lin = match space {
                "oklab" => lab_to_lin(out),
                "srgb" => out.map(decode),
                _ => out,
            };
            from_lin_in(result, lin, alpha)
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
    fn s(text: &str) -> NxValue {
        NxValue::String(text.into())
    }
    #[test]
    fn a_wrapped_hue_is_never_negative_zero() {
        let args = [0.5, 0.1, -360.0, 1.0].map(NxValue::Float);
        let color = read(&call("oklch", &args)).unwrap();
        assert_eq!(color.c[2].to_bits(), 0.0_f64.to_bits());
    }
    #[test]
    fn names_and_arities_agree() {
        assert_eq!(NAMES.len(), ARITIES.len());
    }
    #[test]
    fn parsing_and_float32_roundtrip() {
        for text in [
            "red",
            "#f008",
            "#12345678",
            "rgb(255, 0, 0)",
            "hsl(180 100% 50%)",
            "color(display-p3 1 0 0.25 / 0.5)",
            "oklab(0.7 -0.1 0.2)",
            "oklch(70% 0.3 145)",
            "oklch(0.5 0.0000001 359.99999999)",
        ] {
            let v = parse(text).unwrap();
            let NxValue::String(css) = call("toCss", std::slice::from_ref(&v)) else {
                panic!()
            };
            assert_eq!(v, parse(&css).unwrap(), "{text} -> {css}");
        }
        for text in ["#12", "#gg0000", "currentColor", "var(--accent)", "none"] {
            assert_eq!(parse(text).unwrap_err().code, "color-invalid-text");
        }
        assert!(make([f64::NAN, 0.0, 0.0, 1.0]).is_err());
    }
    #[test]
    fn wide_colors_keep_their_space() {
        let p3 = read(&parse("color(display-p3 1 0 0)").unwrap()).unwrap();
        assert_eq!(p3.space, Space::DisplayP3);
        let lch = read(&call(
            "oklch",
            &[
                NxValue::Float(0.7),
                NxValue::Float(0.3),
                NxValue::Float(145.0),
                NxValue::Float(1.0),
            ],
        ))
        .unwrap();
        assert_eq!(lch.space, Space::Oklch);
        assert!(!in_gamut(lch, Space::Srgb));
        assert!(!in_gamut(lch, Space::DisplayP3));
        assert!(!in_gamut(p3, Space::Srgb));
        assert!(in_gamut(p3, Space::DisplayP3));
    }
    #[test]
    fn gamut_mapping_keeps_hue_and_lightness_close() {
        let p3 = read(&parse("color(display-p3 1 0 0)").unwrap()).unwrap();
        let mapped = to_gamut(p3, Space::Srgb);
        assert!(mapped[..3].iter().all(|x| (0.0..=1.0).contains(x)));
        let before = convert(p3, Space::Oklch);
        let after = convert(
            Color {
                space: Space::Srgb,
                c: mapped,
            },
            Space::Oklch,
        );
        assert!((before[2] - after[2]).abs() < 2.0, "{before:?} {after:?}");
        assert!((before[0] - after[0]).abs() < 0.03, "{before:?} {after:?}");
        let blue = read(&parse("#2563eb").unwrap()).unwrap();
        assert_eq!(to_gamut(blue, Space::Srgb), blue.c);
        let p3_blue = to_gamut(blue, Space::DisplayP3);
        let back = to_gamut(
            Color {
                space: Space::DisplayP3,
                c: p3_blue,
            },
            Space::Srgb,
        );
        for (back, original) in back.iter().zip(blue.c) {
            assert!((back - original).abs() < 1e-9);
        }
    }
    #[test]
    fn mixing_and_contrast_reference() {
        let black = parse("black").unwrap();
        let white = parse("white").unwrap();
        assert_eq!(
            call("contrastRatio", &[black.clone(), white.clone()]),
            NxValue::Float(21.0)
        );
        let out = call("mix", &[black, white, NxValue::Float(0.5), s("linearSrgb")]);
        assert!((channels(&out).unwrap()[0] - 0.735356983).abs() < 1e-6);
        let out = call(
            "mix",
            &[
                parse("red").unwrap(),
                parse("#0000ff00").unwrap(),
                NxValue::Float(0.5),
                s("srgb"),
            ],
        );
        assert_eq!(channels(&out).unwrap(), [1.0, 0.0, 0.0, 0.5]);
        let wide = call(
            "mix",
            &[
                parse("red").unwrap(),
                parse("color(display-p3 0 1 0)").unwrap(),
                NxValue::Float(0.5),
                s("oklab"),
            ],
        );
        assert_eq!(read(&wide).unwrap().space, Space::DisplayP3);
    }
    #[test]
    fn transformations_work_in_oklch_and_keep_the_space() {
        let p3 = parse("color(display-p3 1 0 0)").unwrap();
        let out = read(&call("rotateHue", &[p3, NxValue::Float(120.0)])).unwrap();
        assert_eq!(out.space, Space::DisplayP3);
        let gray = call("grayscale", &[parse("#ff0000").unwrap()]);
        let [r, g, b, _] = channels(&gray).unwrap();
        assert!((r - g).abs() < 1e-6 && (g - b).abs() < 1e-6);
        let light = call("lighten", &[parse("#2563eb").unwrap(), NxValue::Float(0.1)]);
        let before = convert(read(&parse("#2563eb").unwrap()).unwrap(), Space::Oklch);
        let after = convert(read(&light).unwrap(), Space::Oklch);
        assert!((after[0] - before[0] - 0.1).abs() < 0.01);
        assert!((after[2] - before[2]).abs() < 2.0);
    }
    #[test]
    fn records_must_name_their_case() {
        let mut properties = std::collections::BTreeMap::new();
        properties.insert("r".to_string(), NxValue::Float(1.0));
        properties.insert("g".to_string(), NxValue::Float(0.0));
        properties.insert("b".to_string(), NxValue::Float(0.0));
        let untyped = NxValue::Record {
            type_name: None,
            properties: properties.clone(),
        };
        assert!(read(&untyped).is_err());
        let typed = NxValue::Record {
            type_name: Some("Color.srgb".into()),
            properties,
        };
        assert_eq!(read(&typed).unwrap().c, [1.0, 0.0, 0.0, 1.0]);
    }
}
