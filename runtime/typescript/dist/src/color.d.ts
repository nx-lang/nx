/**
 * Closed renderer-independent standard-color-v2 operations.
 *
 * A `Color` is a union of color spaces: `srgb` and `displayP3` hold encoded RGB, `oklab` and
 * `oklch` hold perceptual coordinates that may lie outside every screen's gamut. Conversions go
 * through unclamped linear-light sRGB, and only `toGamut` and the functions documented as mapping
 * into sRGB ever bring a color inside a gamut. This mirrors `crates/nx-value/src/color.rs`.
 */
export declare const nxStandardColorNames: string[];
export declare const nxStandardColorArities: number[];
export declare function nxStandardColor(op: any, args: any): any;
