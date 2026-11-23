/**
 * Closed renderer-independent standard-color-v2 operations.
 *
 * A `Color` is a union of color spaces: `srgb` and `displayP3` hold encoded RGB, `oklab` and
 * `oklch` hold perceptual coordinates that may lie outside every screen's gamut. Conversions go
 * through unclamped linear-light sRGB, and only `toGamut` and the functions documented as mapping
 * into sRGB ever bring a color inside a gamut. This mirrors `crates/nx-value/src/color.rs`.
 */
export const nxStandardColorNames = [
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
export const nxStandardColorArities = [
  3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2,
  2, 2, 2, 2, 2, 2, 1, 1, 2, 4, 2, 1, 2, 3,
];
const colorKeys: any = {
  srgb: ["r", "g", "b", "alpha"],
  displayP3: ["r", "g", "b", "alpha"],
  oklab: ["l", "a", "b", "alpha"],
  oklch: ["l", "c", "h", "alpha"],
};
const colorRank: any = { srgb: 0, displayP3: 1, oklab: 2, oklch: 2 };
const colorIsRgb = (space: any) => space === "srgb" || space === "displayP3";
/** How far outside [0, 1] a converted channel may be and still count as inside a gamut. */
const colorGamutTolerance = 2e-6;
const colorJnd = 0.02;
const colorMapEpsilon = 0.0001;
function colorError(field: any, code: any = "color-invalid-channel"): never {
  throw Object.assign(new Error(`Invalid color: ${field}`), { code });
}
function colorUnit(x: any, field: any = "channel"): any {
  if (!Number.isFinite(x) || x < 0 || x > 1) colorError(field);
  return x;
}
function colorFinite(x: any): any {
  if (!Number.isFinite(x)) colorError("non-finite channel");
  return x;
}
function colorRecord(name: any, keys: any, values: any): any {
  return Object.assign(
    { $type: name },
    Object.fromEntries(keys.map((key: any, i: any) => [key, values[i]])),
  );
}
const colorNarrow = (x: any) => Math.fround(x) || 0;
/** Wraps finite degrees into [0, 360), also after float32 rounding. */
function colorWrapHue(h: any): any {
  h = ((h % 360) + 360) % 360;
  return colorNarrow(h) >= 360 ? 0 : h;
}
/** Checks channels for their space and writes them as a `Color` union-case record. */
function colorMakeIn(space: any, c: any): any {
  const keys = colorKeys[space];
  c.forEach((x: any, i: any) => {
    if (space === "oklab" && (i === 1 || i === 2)) colorFinite(x);
    else if (space === "oklch" && i === 1) {
      if (colorFinite(x) < 0) colorError(keys[i]);
    } else if (space === "oklch" && i === 2) {
      if (!(colorFinite(x) >= 0 && x < 360)) colorError(keys[i]);
    } else colorUnit(x, keys[i]);
  });
  return colorRecord(
    `Color.${space}`,
    keys,
    c.map((x: any, i: any) => colorNarrow(space === "oklch" && i === 2 ? colorWrapHue(x) : x)),
  );
}
function colorMake(c: any): any {
  return colorMakeIn("srgb", c);
}
/** Writes a computed color: a perceptual color has its lightness clamped and its hue wrapped. */
function colorFinish(space: any, c: any): any {
  c = c.map((x: any) => colorFinite(x));
  if (space === "oklab") c[0] = Math.max(0, Math.min(1, c[0]));
  if (space === "oklch") {
    c[0] = Math.max(0, Math.min(1, c[0]));
    c[1] = Math.max(0, c[1]);
    c[2] = colorWrapHue(c[2]);
  }
  c[3] = Math.max(0, Math.min(1, c[3]));
  return colorMakeIn(space, c);
}
/** Reads a `Color` union-case record, checking every channel for its space. */
function colorRead(v: any): any {
  if (v === null || typeof v !== "object") colorError("Color");
  const type = String(v.$type ?? "");
  const scope = type.lastIndexOf("::");
  const name = scope < 0 ? type : type.slice(scope + 2);
  const dot = name.lastIndexOf(".");
  const union = dot < 0 ? "" : name.slice(0, dot);
  const space = name.slice(dot + 1);
  if (!(union === "Color" || union.endsWith(".Color")) || !(space in colorKeys)) colorError("Color");
  const keys = colorKeys[space];
  const c = keys.map((k: any) => {
    const x = v[k] === undefined && k === "alpha" ? 1 : v[k];
    if (typeof x !== "number") colorError(k);
    colorFinite(x);
    if (Math.fround(x) !== x) colorError(k);
    return x;
  });
  colorMakeIn(space, c);
  if (Object.keys(v).some((k) => k !== "$type" && !keys.includes(k))) colorError("Color");
  return { space, c };
}
function colorClamp(x: any): any {
  return Math.max(0, Math.min(1, colorFinite(x)));
}
/** The sRGB transfer curve, extended to negative values by symmetry as CSS Color 4 does. */
function colorDecode(x: any): any {
  const a = Math.abs(x),
    y = a <= 0.04045 ? a / 12.92 : ((a + 0.055) / 1.055) ** 2.4;
  return x < 0 ? -y : y;
}
function colorEncode(x: any): any {
  const a = Math.abs(x),
    y = a <= 0.0031308 ? a * 12.92 : 1.055 * a ** (1 / 2.4) - 0.055;
  return x < 0 ? -y : y;
}

function colorHue(x: any): any {
  colorFinite(x);
  return ((x % 360) + 360) % 360;
}
function colorHsv(h: any, s: any, v: any): any {
  h = colorHue(h) / 60;
  const f = h - Math.floor(h),
    p = v * (1 - s),
    q = v * (1 - s * f),
    t = v * (1 - s * (1 - f));
  return [
    [v, t, p],
    [q, v, p],
    [p, v, t],
    [p, q, v],
    [t, p, v],
    [v, p, q],
  ][Math.floor(h)];
}
function colorHsl(c: any): any {
  const max = Math.max(...c.slice(0, 3)),
    min = Math.min(...c.slice(0, 3)),
    d = max - min,
    l = (max + min) / 2;
  let h = 0,
    s = 0;
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    h =
      60 *
      (max === c[0]
        ? ((c[1] - c[2]) / d) % 6
        : max === c[1]
          ? (c[2] - c[0]) / d + 2
          : (c[0] - c[1]) / d + 4);
  }
  return [colorHue(h), s, l, c[3]];
}
function colorFromHsl(c: any): any {
  const v = c[2] + c[1] * Math.min(c[2], 1 - c[2]),
    s = v === 0 ? 0 : 2 * (1 - c[2] / v);
  return [...colorHsv(c[0], s, v), c[3]];
}
const colorP3ToSrgb = [
  [1.2249401762805596, -0.22494017628055998, 0],
  [-0.04205695470968817, 1.0420569547096883, 0],
  [-0.019637554590334432, -0.07863604555063192, 1.0982736001409663],
];
const colorSrgbToP3 = [
  [0.8224619687143624, 0.17753803128563778, 0],
  [0.03319419885096163, 0.9668058011490381, 0],
  [0.01708263072112004, 0.07239744066396347, 0.9105199286149165],
];
function colorMul(m: any, v: any): any {
  return m.map((row: any) => row[0] * v[0] + row[1] * v[1] + row[2] * v[2]);
}
function colorLinToLab(v: any): any {
  const [r, g, b] = v,
    l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b),
    m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b),
    s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}
function colorLabToLin(v: any): any {
  const [L, a, b] = v,
    l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3,
    m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3,
    s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}
function colorLabToLch(v: any): any {
  return [v[0], Math.hypot(v[1], v[2]), colorWrapHue((Math.atan2(v[2], v[1]) * 180) / Math.PI)];
}
function colorLchToLab(v: any): any {
  const h = (v[2] * Math.PI) / 180;
  return [v[0], v[1] * Math.cos(h), v[1] * Math.sin(h)];
}
function colorToLin(color: any): any {
  const v = color.c.slice(0, 3);
  switch (color.space) {
    case "srgb":
      return v.map(colorDecode);
    case "displayP3":
      return colorMul(colorP3ToSrgb, v.map(colorDecode));
    case "oklab":
      return colorLabToLin(v);
    default:
      return colorLabToLin(colorLchToLab(v));
  }
}
function colorFromLin(space: any, v: any): any {
  switch (space) {
    case "srgb":
      return v.map(colorEncode);
    case "displayP3":
      return colorMul(colorSrgbToP3, v).map(colorEncode);
    case "oklab":
      return colorLinToLab(v);
    default:
      return colorLabToLch(colorLinToLab(v));
  }
}
/** The color's channels in `space`, unclamped, alpha kept. */
function colorConvert(color: any, space: any): any {
  const v = color.c.slice(0, 3);
  let out: any;
  if (color.space === space) out = v;
  else if (color.space === "oklab" && space === "oklch") out = colorLabToLch(v);
  else if (color.space === "oklch" && space === "oklab") out = colorLchToLab(v);
  else if (colorIsRgb(color.space) && space === "oklch" && v[0] === v[1] && v[1] === v[2])
    out = [colorLinToLab(colorToLin(color))[0], 0, 0];
  else out = colorFromLin(space, colorToLin(color));
  return [...out, color.c[3]];
}
function colorRgbInGamut(v: any): any {
  return v.every((x: any) => x >= -colorGamutTolerance && x <= 1 + colorGamutTolerance);
}
function colorInGamut(color: any, gamut: any): any {
  return color.space === gamut || colorRgbInGamut(colorConvert(color, gamut).slice(0, 3));
}
/** The color in `gamut`'s encoded RGB, mapped with the CSS Color 4 algorithm when outside it. */
function colorToGamut(color: any, gamut: any): any {
  const alpha = color.c[3];
  if (color.space === gamut) return color.c.slice();
  const origin = colorConvert(color, "oklch").slice(0, 3);
  if (origin[0] >= 1) return [1, 1, 1, alpha];
  if (origin[0] <= 0) return [0, 0, 0, alpha];
  const direct = colorConvert(color, gamut).slice(0, 3);
  if (colorRgbInGamut(direct)) return [...direct.map((x: any) => Math.max(0, Math.min(1, x))), alpha];
  const dest = (lch: any) => colorFromLin(gamut, colorLabToLin(colorLchToLab(lch)));
  const clip = (v: any) => v.map((x: any) => Math.max(0, Math.min(1, x)));
  const delta = (clipped: any, lch: any) => {
    const a = colorLinToLab(colorToLin({ space: gamut, c: [...clipped, alpha] })),
      b = colorLchToLab(lch);
    return Math.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2);
  };
  const current = origin.slice();
  let clipped = clip(dest(current));
  if (delta(clipped, current) < colorJnd) return [...clipped, alpha];
  let min = 0,
    max = origin[1],
    minInGamut = true;
  while (max - min > colorMapEpsilon) {
    const chroma = (min + max) / 2;
    current[1] = chroma;
    const rgb = dest(current);
    if (minInGamut && colorRgbInGamut(rgb)) {
      min = chroma;
      continue;
    }
    clipped = clip(rgb);
    const e = delta(clipped, current);
    if (e < colorJnd) {
      if (colorJnd - e < colorMapEpsilon) break;
      minInGamut = false;
      min = chroma;
    } else max = chroma;
  }
  return [...clipped, alpha];
}
/** Writes linear-light sRGB as a color in `space`, mapping into the gamut of an RGB space. */
function colorFromLinIn(space: any, lin: any, alpha: any): any {
  lin.forEach((x: any) => colorFinite(x));
  if (colorIsRgb(space)) {
    const direct = colorFromLin(space, lin);
    return colorMakeIn(
      space,
      colorRgbInGamut(direct)
        ? [...direct.map((x: any) => Math.max(0, Math.min(1, x))), alpha]
        : colorToGamut({ space: "oklab", c: [...colorLinToLab(lin), alpha] }, space),
    );
  }
  return colorFinish(space, [...colorFromLin(space, lin), alpha]);
}
/** Writes an Oklch color in `space`, mapping into the gamut of an RGB space. */
function colorFromLchIn(space: any, lch: any): any {
  lch.forEach((x: any) => colorFinite(x));
  const color: any = {
    space: "oklch",
    c: [Math.max(0, Math.min(1, lch[0])), Math.max(0, lch[1]), colorWrapHue(lch[2]), lch[3]],
  };
  return colorIsRgb(space)
    ? colorMakeIn(space, colorToGamut(color, space))
    : colorFinish(space, colorConvert(color, space));
}
function colorWider(a: any, b: any): any {
  return colorRank[b] > colorRank[a] ? b : a;
}
/** Standalone CSS text for the color, in its own space. */
function colorCss(color: any): any {
  const [a, b, c, alpha] = color.c.map((x: any) => String(x === 0 ? 0 : x));
  switch (color.space) {
    case "srgb":
      return `rgb(${color.c
        .slice(0, 3)
        .map((x: any) => String(x * 255 || 0))
        .join(" ")} / ${alpha})`;
    case "displayP3":
      return `color(display-p3 ${a} ${b} ${c} / ${alpha})`;
    default:
      return `${color.space}(${a} ${b} ${c} / ${alpha})`;
  }
}
function colorGamutArg(v: any): any {
  if (v === "srgb" || v === "displayP3") return v;
  colorError("gamut");
}
/** The color as an sRGB screen shows it, for the functions defined only in sRGB. */
function colorShown(color: any): any {
  return colorToGamut(color, "srgb").map(colorNarrow);
}
function colorLuminance(c: any): any {
  if (c[3] !== 1) colorError("alpha must be opaque");
  return 0.2126 * colorDecode(c[0]) + 0.7152 * colorDecode(c[1]) + 0.0722 * colorDecode(c[2]);
}
function colorContrast(a: any, b: any): any {
  const x = colorLuminance(colorShown(a)),
    y = colorLuminance(colorShown(b));
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
function colorWrite(color: any): any {
  return colorMakeIn(color.space, color.c);
}

function colorParse(text: any): any {
  if (typeof text !== "string") colorError("text");
  const source = text.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, "");
  try {
    return colorParseValue(source);
  } catch {
    throw Object.assign(new Error(`invalid standalone color: ${source}`), { code: "color-invalid-text" });
  }
}
function trimColorWhitespace(text: any): any { return text.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, ""); }
function colorParseValue(text: any): any {
  if (typeof text !== "string") colorError("text");
  const t = text.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, "").toLowerCase();
  if (t === "transparent") return colorMake([0, 0, 0, 0]);
  if (colorNamed[t])
    return colorMake(colorNamed[t].map((x: any) => x / 255).concat(1));
  if (/^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/.test(t)) {
    let h = t.slice(1);
    if (h.length < 5) h = [...h].map((x: any) => x + x).join("");
    if (h.length === 6) h += "ff";
    return colorMake(
      [0, 2, 4, 6].map((i: any) => parseInt(h.slice(i, i + 2), 16) / 255),
    );
  }
  const m = /^(rgb|rgba|hsl|hsla|hwb|oklab|oklch|color)\(([\s\S]*)\)$/.exec(t);
  if (!m) colorError(text, "color-invalid-text");
  let name: any = m[1],
    body: any = m[2];
  if (body.includes(",") && !["rgb", "rgba", "hsl", "hsla"].includes(name))
    colorError(text, "color-invalid-text");
  if (body.includes(",") && body.includes("/"))
    colorError(text, "color-invalid-text");
  if (name === "color") {
    body = trimColorWhitespace(body);
    const p = /^(srgb|srgb-linear|display-p3)[\t\n\f\r ]+/.exec(body);
    if (!p) colorError(text, "color-invalid-text");
    name = p[1];
    body = body.slice(p[0].length);
  }
  const bad = () => colorError(text, "color-invalid-text");
  let tokens: any;
  if (body.includes(",")) {
    tokens = body.split(",").map((s: any) => trimColorWhitespace(s));
    if (tokens.length !== 3 && tokens.length !== 4) bad();
    if (
      (name === "rgb" || name === "rgba") &&
      !(
        tokens.slice(0, 3).every((s: any) => s.endsWith("%")) ||
        tokens.slice(0, 3).every((s: any) => !s.endsWith("%"))
      )
    )
      bad();
  } else {
    const slash = body.split("/");
    if (slash.length > 2) bad();
    tokens = trimColorWhitespace(slash[0]).split(/[\t\n\f\r ]+/);
    if (tokens.length !== 3) bad();
    if (slash.length === 2) tokens.push(trimColorWhitespace(slash[1]));
  }
  const number = (s: any, scale: any = 1, hue: any = false) => {
    const match =
      /^([+-]?(?:\d*\.\d+|\d+)(?:e[+-]?\d+)?)(%|deg|grad|rad|turn)?$/.exec(
        s,
      );
    if (!match) return bad();
    if (match[2] && match[2] !== "%" && !hue) return bad();
    let x = Number(match[1]);
    if (match[2] === "%") x = (x * scale) / 100;
    else if (match[2] === "turn") x *= 360;
    else if (match[2] === "rad") x *= 180 / Math.PI;
    else if (match[2] === "grad") x *= 0.9;
    if (!Number.isFinite(x)) return bad();
    return x;
  };
  if (
    ["hsl", "hsla", "hwb"].includes(name) &&
    ((body.includes(",") && (!tokens[1].endsWith("%") || !tokens[2].endsWith("%"))) || tokens[0].endsWith("%"))
  )
    bad();
  if (name === "oklch" && tokens[2].endsWith("%")) bad();
  const a = tokens[3] === undefined ? 1 : colorClamp(number(tokens[3]));
  let c: any,
    space: any = "srgb";
  if (name === "rgb" || name === "rgba")
    c = tokens
      .slice(0, 3)
      .map((s: any) => colorClamp(number(s, 255) / 255))
      .concat(a);
  else if (name === "srgb" || name === "srgb-linear" || name === "display-p3") {
    if (name === "display-p3") space = "displayP3";
    c = tokens
      .slice(0, 3)
      .map((s: any) => colorClamp(number(s)))
      .map((x: any) => (name === "srgb-linear" ? colorEncode(x) : x))
      .concat(a);
  }
  else if (name === "hsl" || name === "hsla")
    c = colorFromHsl([
      number(tokens[0], 1, true),
      colorClamp(number(tokens[1], 100) / 100),
      colorClamp(number(tokens[2], 100) / 100),
      a,
    ]);
  else if (name === "hwb") {
    let w = colorClamp(number(tokens[1], 100) / 100),
      b = colorClamp(number(tokens[2], 100) / 100);
    c = (
      w + b >= 1
        ? [w / (w + b), w / (w + b), w / (w + b)]
        : colorHsv(number(tokens[0], 1, true), 1, 1).map(
            (x: any) => x * (1 - w - b) + w,
          )
    ).concat(a);
  } else {
    const l = colorClamp(number(tokens[0])),
      x = number(tokens[1], 0.4);
    if (name === "oklch") {
      space = "oklch";
      c = [l, Math.max(0, x), number(tokens[2], 1, true), a];
    } else {
      space = "oklab";
      c = [l, x, number(tokens[2], 0.4), a];
    }
  }
  c.forEach(colorFinite);
  if (space === "srgb" || space === "displayP3") c = c.map(colorClamp);
  return colorFinish(space, c);
}
export function nxStandardColor(op: any, args: any): any {
  const name: any = nxStandardColorNames[op];
  if (args.length !== nxStandardColorArities[op]) colorError("arity");
  if (name === "parse") return colorParse(args[0]);
  if (name === "tryParse") {
    try {
      return colorRecord("ParseResult.parsed", ["value"], [colorParse(args[0])]);
    } catch (e: any) {
      return colorRecord("ParseResult.invalid", ["message"], [e.message]);
    }
  }
  if (
    ["rgb", "rgba", "srgb", "linearRgb", "hsl", "hsv", "hwb", "displayP3", "oklab", "oklch"].includes(
      name,
    )
  ) {
    let c = [args[0], args[1], args[2], args[3] ?? 1];
    c.forEach(colorFinite);
    colorUnit(c[3]);
    if (name === "rgb" || name === "rgba")
      c = c
        .slice(0, 3)
        .map((x: any) => colorUnit(x / 255))
        .concat(c[3]);
    else if (name === "linearRgb")
      c = c
        .slice(0, 3)
        .map((x: any) => colorEncode(colorUnit(x)))
        .concat(c[3]);
    else if (name === "hsl") {
      colorUnit(c[1]);
      colorUnit(c[2]);
      c = colorFromHsl(c);
    } else if (name === "hsv") {
      colorUnit(c[1]);
      colorUnit(c[2]);
      c = colorHsv(c[0], c[1], c[2]).concat(c[3]);
    } else if (name === "hwb") {
      colorUnit(c[1]);
      colorUnit(c[2]);
      const sum = c[1] + c[2];
      c = (
        sum >= 1
          ? [c[1] / sum, c[1] / sum, c[1] / sum]
          : colorHsv(c[0], 1, 1).map((x: any) => x * (1 - sum) + c[1])
      ).concat(c[3]);
    } else if (name === "displayP3") return colorMakeIn("displayP3", c);
    else if (name === "oklab") {
      colorUnit(c[0]);
      return colorMakeIn("oklab", c);
    } else if (name === "oklch") {
      colorUnit(c[0]);
      if (c[1] < 0) colorError("c");
      c[2] = colorWrapHue(c[2]);
      return colorMakeIn("oklch", c);
    }
    return colorMake(c);
  }
  const color = colorRead(args[0]);
  if (name === "validate") return colorWrite(color);
  if (name === "inGamut") return colorInGamut(color, colorGamutArg(args[1]));
  if (name === "toGamut") {
    const gamut = colorGamutArg(args[1]);
    return colorMakeIn(gamut, colorToGamut(color, gamut));
  }
  if (name === "toHex")
    return (
      "#" +
      colorShown(color)
        .map((x: any) =>
          Math.floor(x * 255 + 0.5)
            .toString(16)
            .padStart(2, "0"),
        )
        .join("")
    );
  if (name === "toCss") return colorCss(color);
  if (name === "toOklab" || name === "toOklch") {
    const space = name === "toOklab" ? "oklab" : "oklch";
    return colorRecord(name.slice(2) + "Channels", colorKeys[space], colorConvert(color, space));
  }
  if (name.startsWith("to")) {
    const c: any = colorShown(color);
    let v: any, keys: any;
    switch (name) {
      case "toRgb":
        v = c
          .slice(0, 3)
          .map((x: any) => x * 255)
          .concat(c[3]);
        keys = ["r", "g", "b", "alpha"];
        break;
      case "toHsl":
        v = colorHsl(c);
        keys = ["h", "s", "l", "alpha"];
        break;
      case "toHsv": {
        v = colorHsl(c);
        const max = Math.max(...c.slice(0, 3)),
          min = Math.min(...c.slice(0, 3));
        v = [v[0], max === 0 ? 0 : (max - min) / max, max, c[3]];
        keys = ["h", "s", "v", "alpha"];
        break;
      }
      case "toHwb":
        v = [colorHsl(c)[0], Math.min(...c.slice(0, 3)), 1 - Math.max(...c.slice(0, 3)), c[3]];
        keys = ["h", "w", "b", "alpha"];
        break;
      default:
        v = c.slice(0, 3).map(colorDecode).concat(c[3]);
        keys = ["r", "g", "b", "alpha"];
    }
    return colorRecord(name.slice(2) + "Channels", keys, v);
  }
  if (["withAlpha", "fadeIn", "fadeOut"].includes(name)) {
    const a = colorUnit(args[1]),
      c = color.c.slice();
    if (name === "withAlpha") c[3] = a;
    else if (name === "fadeIn") c[3] = Math.min(1, c[3] + a);
    else c[3] = Math.max(0, c[3] - a);
    return colorMakeIn(color.space, c);
  }
  if (name === "invert") {
    const a = colorUnit(args[1]);
    const [space, c]: any = colorIsRgb(color.space)
      ? [color.space, color.c]
      : ["displayP3", colorToGamut(color, "displayP3")];
    return colorMakeIn(
      space,
      c
        .slice(0, 3)
        .map((x: any) => Math.max(0, Math.min(1, x * (1 - a) + (1 - x) * a)))
        .concat(c[3]),
    );
  }
  if (["lighten", "darken", "saturate", "desaturate", "rotateHue", "complement", "grayscale"].includes(name)) {
    const v = colorConvert(color, "oklch");
    if (name === "complement") v[2] += 180;
    else if (name === "grayscale") v[1] = 0;
    else if (name === "rotateHue") v[2] += colorFinite(args[1]);
    else {
      const a = colorUnit(args[1]);
      if (name === "lighten") v[0] = Math.max(0, Math.min(1, v[0] + a));
      else if (name === "darken") v[0] = Math.max(0, Math.min(1, v[0] - a));
      else if (name === "saturate") v[1] *= 1 + a;
      else v[1] *= 1 - a;
    }
    return colorFromLchIn(color.space, v);
  }
  if (name === "relativeLuminance") return colorLuminance(colorShown(color));
  if (name === "contrastRatio") return colorContrast(color, colorRead(args[1]));
  if (name === "bestTextColor") {
    const a = colorRead(args[1]),
      b = colorRead(args[2]);
    return colorWrite(colorContrast(color, a) >= colorContrast(color, b) ? a : b);
  }
  if (name === "mix" || name === "composite") {
    const a = color,
      b = colorRead(args[1]),
      w = name === "mix" ? colorUnit(args[2]) : 0,
      space = name === "mix" ? args[3] : "linearSrgb";
    if (!["srgb", "linearSrgb", "oklab"].includes(space)) colorError("space");
    if (name === "mix" && w === 0) return colorWrite(a);
    if (name === "mix" && w === 1) return colorWrite(b);
    const result = colorWider(a.space, b.space);
    const coords = (x: any) =>
      space === "oklab"
        ? colorConvert(x, "oklab").slice(0, 3)
        : space === "srgb"
          ? colorToLin(x).map(colorEncode)
          : colorToLin(x);
    const pa = coords(a),
      pb = coords(b),
      wa = name === "mix" ? 1 - w : 1,
      wb = name === "mix" ? w : 1 - a.c[3],
      alpha = a.c[3] * wa + b.c[3] * wb;
    if (alpha <= 0) return colorMakeIn(result, [0, 0, 0, 0]);
    const out = pa.map((x: any, i: any) => (x * a.c[3] * wa + pb[i] * b.c[3] * wb) / alpha);
    if (space === "oklab" && !colorIsRgb(result))
      return colorFinish(
        result,
        result === "oklch" ? colorConvert({ space: "oklab", c: [...out, alpha] }, "oklch") : [...out, alpha],
      );
    const lin = space === "oklab" ? colorLabToLin(out) : space === "srgb" ? out.map(colorDecode) : out;
    return colorFromLinIn(result, lin, alpha);
  }
  colorError("operation");
}

const colorNamed: Record<string, number[]> = {
  aliceblue: [240, 248, 255],
  antiquewhite: [250, 235, 215],
  aqua: [0, 255, 255],
  aquamarine: [127, 255, 212],
  azure: [240, 255, 255],
  beige: [245, 245, 220],
  bisque: [255, 228, 196],
  black: [0, 0, 0],
  blanchedalmond: [255, 235, 205],
  blue: [0, 0, 255],
  blueviolet: [138, 43, 226],
  brown: [165, 42, 42],
  burlywood: [222, 184, 135],
  cadetblue: [95, 158, 160],
  chartreuse: [127, 255, 0],
  chocolate: [210, 105, 30],
  coral: [255, 127, 80],
  cornflowerblue: [100, 149, 237],
  cornsilk: [255, 248, 220],
  crimson: [220, 20, 60],
  cyan: [0, 255, 255],
  darkblue: [0, 0, 139],
  darkcyan: [0, 139, 139],
  darkgoldenrod: [184, 134, 11],
  darkgray: [169, 169, 169],
  darkgreen: [0, 100, 0],
  darkgrey: [169, 169, 169],
  darkkhaki: [189, 183, 107],
  darkmagenta: [139, 0, 139],
  darkolivegreen: [85, 107, 47],
  darkorange: [255, 140, 0],
  darkorchid: [153, 50, 204],
  darkred: [139, 0, 0],
  darksalmon: [233, 150, 122],
  darkseagreen: [143, 188, 143],
  darkslateblue: [72, 61, 139],
  darkslategray: [47, 79, 79],
  darkslategrey: [47, 79, 79],
  darkturquoise: [0, 206, 209],
  darkviolet: [148, 0, 211],
  deeppink: [255, 20, 147],
  deepskyblue: [0, 191, 255],
  dimgray: [105, 105, 105],
  dimgrey: [105, 105, 105],
  dodgerblue: [30, 144, 255],
  firebrick: [178, 34, 34],
  floralwhite: [255, 250, 240],
  forestgreen: [34, 139, 34],
  fuchsia: [255, 0, 255],
  gainsboro: [220, 220, 220],
  ghostwhite: [248, 248, 255],
  gold: [255, 215, 0],
  goldenrod: [218, 165, 32],
  gray: [128, 128, 128],
  green: [0, 128, 0],
  greenyellow: [173, 255, 47],
  grey: [128, 128, 128],
  honeydew: [240, 255, 240],
  hotpink: [255, 105, 180],
  indianred: [205, 92, 92],
  indigo: [75, 0, 130],
  ivory: [255, 255, 240],
  khaki: [240, 230, 140],
  lavender: [230, 230, 250],
  lavenderblush: [255, 240, 245],
  lawngreen: [124, 252, 0],
  lemonchiffon: [255, 250, 205],
  lightblue: [173, 216, 230],
  lightcoral: [240, 128, 128],
  lightcyan: [224, 255, 255],
  lightgoldenrodyellow: [250, 250, 210],
  lightgray: [211, 211, 211],
  lightgreen: [144, 238, 144],
  lightgrey: [211, 211, 211],
  lightpink: [255, 182, 193],
  lightsalmon: [255, 160, 122],
  lightseagreen: [32, 178, 170],
  lightskyblue: [135, 206, 250],
  lightslategray: [119, 136, 153],
  lightslategrey: [119, 136, 153],
  lightsteelblue: [176, 196, 222],
  lightyellow: [255, 255, 224],
  lime: [0, 255, 0],
  limegreen: [50, 205, 50],
  linen: [250, 240, 230],
  magenta: [255, 0, 255],
  maroon: [128, 0, 0],
  mediumaquamarine: [102, 205, 170],
  mediumblue: [0, 0, 205],
  mediumorchid: [186, 85, 211],
  mediumpurple: [147, 112, 219],
  mediumseagreen: [60, 179, 113],
  mediumslateblue: [123, 104, 238],
  mediumspringgreen: [0, 250, 154],
  mediumturquoise: [72, 209, 204],
  mediumvioletred: [199, 21, 133],
  midnightblue: [25, 25, 112],
  mintcream: [245, 255, 250],
  mistyrose: [255, 228, 225],
  moccasin: [255, 228, 181],
  navajowhite: [255, 222, 173],
  navy: [0, 0, 128],
  oldlace: [253, 245, 230],
  olive: [128, 128, 0],
  olivedrab: [107, 142, 35],
  orange: [255, 165, 0],
  orangered: [255, 69, 0],
  orchid: [218, 112, 214],
  palegoldenrod: [238, 232, 170],
  palegreen: [152, 251, 152],
  paleturquoise: [175, 238, 238],
  palevioletred: [219, 112, 147],
  papayawhip: [255, 239, 213],
  peachpuff: [255, 218, 185],
  peru: [205, 133, 63],
  pink: [255, 192, 203],
  plum: [221, 160, 221],
  powderblue: [176, 224, 230],
  purple: [128, 0, 128],
  rebeccapurple: [102, 51, 153],
  red: [255, 0, 0],
  rosybrown: [188, 143, 143],
  royalblue: [65, 105, 225],
  saddlebrown: [139, 69, 19],
  salmon: [250, 128, 114],
  sandybrown: [244, 164, 96],
  seagreen: [46, 139, 87],
  seashell: [255, 245, 238],
  sienna: [160, 82, 45],
  silver: [192, 192, 192],
  skyblue: [135, 206, 235],
  slateblue: [106, 90, 205],
  slategray: [112, 128, 144],
  slategrey: [112, 128, 144],
  snow: [255, 250, 250],
  springgreen: [0, 255, 127],
  steelblue: [70, 130, 180],
  tan: [210, 180, 140],
  teal: [0, 128, 128],
  thistle: [216, 191, 216],
  tomato: [255, 99, 71],
  turquoise: [64, 224, 208],
  violet: [238, 130, 238],
  wheat: [245, 222, 179],
  white: [255, 255, 255],
  whitesmoke: [245, 245, 245],
  yellow: [255, 255, 0],
  yellowgreen: [154, 205, 50],
};
