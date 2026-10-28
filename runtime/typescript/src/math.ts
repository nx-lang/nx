/** Closed standard-math implementation shared with executable generation. */
export const nxStandardMathNames = [
  "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sqrt", "hypot", "abs", "sign",
  "min", "max", "clamp", "floor", "ceil", "round", "roundTiesToEven", "trunc",
  "degreesToRadians", "radiansToDegrees", "lerp",
] as const;
export const nxStandardMathArities = [1, 1, 1, 1, 1, 1, 2, 1, 2, 1, 1, 2, 2, 3, 1, 1, 1, 1, 1, 1, 1, 3] as const;

export function nxStandardMath(op: number, args: readonly number[]): number {
  const name = nxStandardMathNames[op];
  const fail = (code: string, detail: string): never => {
    throw Object.assign(new Error(`${name ?? op}: ${detail}`), { code });
  };
  if (name === undefined || args.length !== nxStandardMathArities[op]) {
    return fail("nx-math-domain", "unknown operation or wrong argument count");
  }
  if (args.some(x => !Number.isFinite(x))) {
    return fail("nx-math-non-finite-input", "arguments must be finite");
  }
  const x = args[0]!;
  const y = args[1]!;
  const t = args[2]!;
  if ((name === "sqrt" && x < 0) || ((name === "asin" || name === "acos") && Math.abs(x) > 1)
    || (name === "clamp" && y > t)) {
    return fail("nx-math-domain", "invalid domain or bounds");
  }
  let result: number;
  switch (name) {
    case "sin": result = Math.sin(x); break;
    case "cos": result = Math.cos(x); break;
    case "tan": result = Math.tan(x); break;
    case "asin": result = Math.asin(x); break;
    case "acos": result = Math.acos(x); break;
    case "atan": result = Math.atan(x); break;
    case "atan2": result = Math.atan2(x, y); break;
    case "sqrt": result = Math.sqrt(x); break;
    case "hypot": result = Math.hypot(x, y); break;
    case "abs": result = Math.abs(x); break;
    case "sign": result = Math.sign(x); break;
    case "min": result = Math.min(x, y); break;
    case "max": result = Math.max(x, y); break;
    case "clamp": result = Math.min(Math.max(x, y), t); break;
    case "floor": result = Math.floor(x); break;
    case "ceil": result = Math.ceil(x); break;
    case "round": result = Math.round(x); break;
    case "roundTiesToEven": {
      const floor = Math.floor(x);
      const distance = x - floor;
      result = distance < 0.5 ? floor : distance > 0.5 ? floor + 1 : floor % 2 === 0 ? floor : floor + 1;
      if (result === 0 && (x < 0 || Object.is(x, -0))) result = -0;
      break;
    }
    case "trunc": result = Math.trunc(x); break;
    case "degreesToRadians": result = x * (Math.PI / 180); break;
    case "radiansToDegrees": result = x * (180 / Math.PI); break;
    case "lerp":
      result = t === 0 ? x : t === 1 ? y : x === y ? x
        : ((x <= 0 && y >= 0) || (x >= 0 && y <= 0)) ? (1 - t) * x + t * y : x + t * (y - x);
      // Retry extrapolation without an overflowing difference product before cancellation.
      if (!Number.isFinite(result)) result = (1 - t) * x + t * y;
      if (!Number.isFinite(result)) result = (x * 0.5 + t * (y * 0.5 - x * 0.5)) * 2;
      break;
  }
  if (!Number.isFinite(result)) return fail("nx-math-overflow", "result is not finite");
  return result;
}
