/** Closed standard-math implementation shared with executable generation. */
export declare const nxStandardMathNames: readonly ["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sqrt", "hypot", "abs", "sign", "min", "max", "clamp", "floor", "ceil", "round", "roundTiesToEven", "trunc", "degreesToRadians", "radiansToDegrees", "lerp"];
export declare const nxStandardMathArities: readonly [1, 1, 1, 1, 1, 1, 2, 1, 2, 1, 1, 2, 2, 3, 1, 1, 1, 1, 1, 1, 1, 3];
export declare function nxStandardMath(op: number, args: readonly number[]): number;
