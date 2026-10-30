/** Closed renderer-independent standard-color-v1 operations. */
export const nxStandardColorNames = [
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
export const nxStandardColorArities = [
  3, 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
  2, 2, 2, 1, 1, 2, 4, 2, 1, 2, 3,
];
const colorModule = "@nx/color/color.nx";
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
function colorMake(c: any): any {
  return colorRecord(
    "Color",
    ["r", "g", "b", "a"],
    c.map((x: any) => Math.fround(colorUnit(x)) || 0),
  );
}
function colorChannels(c: any): any {
  if (c === null || typeof c !== "object") colorError("Color");
  return ["r", "g", "b", "a"].map((k: any) => {
    const x = colorUnit(c[k] === undefined && k === "a" ? 1 : c[k], k);
    if (Math.fround(x) !== x) colorError(k);
    return x;
  });
}
function colorClamp(x: any): any {
  return Math.max(0, Math.min(1, colorFinite(x)));
}
function colorDecode(x: any): any {
  return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
}
function colorEncode(x: any): any {
  return x <= 0.0031308 ? x * 12.92 : 1.055 * x ** (1 / 2.4) - 0.055;
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
function colorLab(c: any): any {
  const [r, g, b] = c.slice(0, 3).map(colorDecode),
    l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b),
    m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b),
    s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    c[3],
  ];
}
function colorFromLab(c: any): any {
  const [L, a, b, alpha] = c,
    l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3,
    m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3,
    s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ]
    .map(colorEncode)
    .concat(alpha);
}
function colorLuminance(c: any): any {
  if (c[3] !== 1) colorError("alpha must be opaque");
  return c
    .slice(0, 3)
    .map(colorDecode)
    .reduce(
      (s: any, x: any, i: any) => s + x * ([0.2126, 0.7152, 0.0722][i] ?? 0),
      0,
    );
}
function colorContrast(a: any, b: any): any {
  const x = colorLuminance(a),
    y = colorLuminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
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
    const p = /^(srgb|srgb-linear)[\t\n\f\r ]+/.exec(body);
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
  let c: any;
  if (name === "rgb" || name === "rgba")
    c = tokens
      .slice(0, 3)
      .map((s: any) => colorClamp(number(s, 255) / 255))
      .concat(a);
  else if (name === "srgb" || name === "srgb-linear")
    c = tokens
      .slice(0, 3)
      .map((s: any) => colorClamp(number(s)))
      .map((x: any) => (name === "srgb" ? x : colorEncode(x)))
      .concat(a);
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
      x = number(tokens[1], 0.4),
      y = number(tokens[2], 0.4, name === "oklch");
    c = colorFromLab(
      name === "oklab"
        ? [l, x, y, a]
        : [
            l,
            Math.max(0, x) *
              Math.cos((number(tokens[2], 1, true) * Math.PI) / 180),
            Math.max(0, x) *
              Math.sin((number(tokens[2], 1, true) * Math.PI) / 180),
            a,
          ],
    );
  }
  return colorMake(c.map(colorClamp));
}
export function nxStandardColor(op: any, args: any): any {
  const name: any = nxStandardColorNames[op];
  if (args.length !== nxStandardColorArities[op]) colorError("arity");
  if (name === "parse") return colorParse(args[0]);
  if (name === "tryParse") {
    try {
      return colorRecord(
        "ParseResult.parsed",
        ["value"],
        [colorParse(args[0])],
      );
    } catch (e: any) {
      return colorRecord("ParseResult.invalid", ["message"], [e.message]);
    }
  }
  if (
    [
      "rgb",
      "rgba",
      "srgb",
      "linearRgb",
      "hsl",
      "hsv",
      "hwb",
      "oklab",
      "oklch",
    ].includes(name)
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
    } else if (name === "oklab" || name === "oklch") {
      colorUnit(c[0]);
      if (name === "oklch") {
        if (c[1] < 0) colorError("chroma");
        const h = (colorHue(c[2]) * Math.PI) / 180;
        c = [c[0], c[1] * Math.cos(h), c[1] * Math.sin(h), c[3]];
      }
      c = colorFromLab(c).map(colorClamp);
    }
    return colorMake(c);
  }
  let c = colorChannels(args[0]);
  if (name === "validate") return colorMake(c);
  if (name === "toHex")
    return (
      "#" +
      c
        .map((x: any) =>
          Math.floor(x * 255 + 0.5)
            .toString(16)
            .padStart(2, "0"),
        )
        .join("")
    );
  if (name === "toCss")
    return `rgba(${c[0] * 255}, ${c[1] * 255}, ${c[2] * 255}, ${c[3]})`;
  if (name.startsWith("to")) {
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
        v = [
          colorHsl(c)[0],
          Math.min(...c.slice(0, 3)),
          1 - Math.max(...c.slice(0, 3)),
          c[3],
        ];
        keys = ["h", "w", "b", "alpha"];
        break;
      case "toLinearRgb":
        v = c.slice(0, 3).map(colorDecode).concat(c[3]);
        keys = ["r", "g", "b", "alpha"];
        break;
      default:
        v = colorLab(c);
        keys = ["l", "a", "b", "alpha"];
        if (name === "toOklch") {
          v = [
            v[0],
            Math.hypot(v[1], v[2]),
            colorHue((Math.atan2(v[2], v[1]) * 180) / Math.PI),
            v[3],
          ];
          if (c[0] === c[1] && c[1] === c[2]) {
            v[1] = 0;
            v[2] = 0;
          }
          keys = ["l", "c", "h", "alpha"];
        }
    }
    return colorRecord(name.slice(2) + "Channels", keys, v);
  }
  if (["withAlpha", "fadeIn", "fadeOut", "invert"].includes(name)) {
    const a = colorUnit(args[1]);
    if (name === "withAlpha") c[3] = a;
    else if (name === "fadeIn") c[3] = Math.min(1, c[3] + a);
    else if (name === "fadeOut") c[3] = Math.max(0, c[3] - a);
    else
      c = c
        .slice(0, 3)
        .map((x: any) => x * (1 - a) + (1 - x) * a)
        .concat(c[3]);
    return colorMake(c);
  }
  if (
    [
      "lighten",
      "darken",
      "saturate",
      "desaturate",
      "rotateHue",
      "complement",
      "grayscale",
    ].includes(name)
  ) {
    c = colorHsl(c);
    if (name === "complement") c[0] += 180;
    else if (name === "grayscale") c[1] = 0;
    else if (name === "rotateHue") c[0] += colorFinite(args[1]);
    else {
      const a = colorUnit(args[1]),
        i = name === "lighten" || name === "darken" ? 2 : 1;
      c[i] = colorClamp(
        c[i] + (["darken", "desaturate"].includes(name) ? -a : a),
      );
    }
    return colorMake(colorFromHsl(c));
  }
  if (name === "relativeLuminance") return colorLuminance(c);
  if (name === "contrastRatio") return colorContrast(c, colorChannels(args[1]));
  if (name === "bestTextColor") {
    const a = colorChannels(args[1]),
      b = colorChannels(args[2]);
    return colorMake(colorContrast(c, a) >= colorContrast(c, b) ? a : b);
  }
  if (name === "mix" || name === "composite") {
    let b = colorChannels(args[1]),
      w = name === "mix" ? colorUnit(args[2]) : 0,
      space = name === "mix" ? args[3] : "linearSrgb";
    if (name === "mix" && w === 0) return colorMake(c);
    if (name === "mix" && w === 1) return colorMake(b);
    if (!["srgb", "linearSrgb", "oklab"].includes(space)) colorError("space");
    if (space === "oklab") {
      c = colorLab(c);
      b = colorLab(b);
    } else if (space === "linearSrgb") {
      c = c.slice(0, 3).map(colorDecode).concat(c[3]);
      b = b.slice(0, 3).map(colorDecode).concat(b[3]);
    }
    const wa = name === "mix" ? 1 - w : 1,
      wb = name === "mix" ? w : 1 - c[3],
      a = c[3] * wa + b[3] * wb;
    if (a === 0) return colorMake([0, 0, 0, 0]);
    let out = c
      .slice(0, 3)
      .map((x: any, i: any) => (x * c[3] * wa + b[i] * b[3] * wb) / a)
      .concat(a);
    if (space === "oklab") out = colorFromLab(out);
    else if (space === "linearSrgb")
      out = out.slice(0, 3).map(colorEncode).concat(a);
    return colorMake(out.map(colorClamp));
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
