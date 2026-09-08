import type { ColorFilter } from "canvaskit-wasm";
import { Super } from "./Super";
import type { Color } from "./Types";

/** DrawnUi SkiaImageEffect. */
export type SkiaImageEffect = "None" | "BlackAndWhite" | "Pastel" | "Tint" | "Darken" | "Lighten" | "Grayscale" | "Sepia" | "InvertColors" | "Contrast" | "Saturation" | "Brightness" | "Gamma" | "TSL" | "HSL" | "Custom";

/** SKBlendMode names understood by CanvasKit (`CK.BlendMode[name]`). */
export type BlendMode = "Clear" | "Src" | "Dst" | "SrcOver" | "DstOver" | "SrcIn" | "DstIn" | "SrcOut" | "DstOut" | "SrcATop" | "DstATop" | "Xor" | "Plus" | "Modulate" | "Screen" | "Overlay" | "Darken" | "Lighten" | "ColorDodge" | "ColorBurn" | "HardLight" | "SoftLight" | "Difference" | "Exclusion" | "Multiply" | "Hue" | "Saturation" | "Color" | "Luminosity";

/**
 * Port of DrawnUi SkiaImageEffects (color matrices). C# matrices use 0..255 translation values (SkiaSharp);
 * CanvasKit's MakeMatrix takes 0..1, so every translation is divided by 255 here to keep the same look.
 */
export const SkiaImageEffects = {
  Matrix(m: number[]): ColorFilter { return Super.CK.ColorFilter.MakeMatrix(m); },
  Tint(color: Color, mode: BlendMode): ColorFilter {
    const CK = Super.CK;
    return CK.ColorFilter.MakeBlend(Super.ParseColor(color), CK.BlendMode[mode] ?? CK.BlendMode.SrcIn);
  },
  Darken(amount: number): ColorFilter { const a = -amount / 255; return SkiaImageEffects.Matrix([1, 0, 0, 0, a, 0, 1, 0, 0, a, 0, 0, 1, 0, a, 0, 0, 0, 1, 0]); },
  Lighten(amount: number): ColorFilter { const a = amount / 255; return SkiaImageEffects.Matrix([1, 0, 0, 0, a, 0, 1, 0, 0, a, 0, 0, 1, 0, a, 0, 0, 0, 1, 0]); },
  Grayscale(): ColorFilter { return SkiaImageEffects.Matrix([0.2989, 0.587, 0.114, 0, 0, 0.2989, 0.587, 0.114, 0, 0, 0.2989, 0.587, 0.114, 0, 0, 0, 0, 0, 1, 0]); },
  Pastel(): ColorFilter { return SkiaImageEffects.Matrix([0.75, 0.25, 0.25, 0, 0, 0.25, 0.75, 0.25, 0, 0, 0.25, 0.25, 0.75, 0, 0, 0, 0, 0, 1, 0]); },
  Sepia(): ColorFilter { return SkiaImageEffects.Matrix([0.393, 0.769, 0.189, 0, 0, 0.349, 0.686, 0.168, 0, 0, 0.272, 0.534, 0.131, 0, 0, 0, 0, 0, 1, 0]); },
  InvertColors(): ColorFilter { return SkiaImageEffects.Matrix([-1, 0, 0, 0, 1, 0, -1, 0, 0, 1, 0, 0, -1, 0, 1, 0, 0, 0, 1, 0]); },
  Contrast(amount: number): ColorFilter {
    const c = amount + 1, l = (0.5 * (1 - amount)) / 255;
    return SkiaImageEffects.Matrix([c, 0, 0, 0, l, 0, c, 0, 0, l, 0, 0, c, 0, l, 0, 0, 0, 1, 0]);
  },
  Saturation(amount: number): ColorFilter {
    const rr = 0.213 * (1 - amount), rg = 0.715 * (1 - amount), rb = 0.072 * (1 - amount);
    return SkiaImageEffects.Matrix([rr + amount, rg, rb, 0, 0, rr, rg + amount, rb, 0, 0, rr, rg, rb + amount, 0, 0, 0, 0, 0, 1, 0]);
  },
  Brightness(amount: number): ColorFilter { return SkiaImageEffects.Matrix([1, 0, 0, 0, amount, 0, 1, 0, 0, amount, 0, 0, 1, 0, amount, 0, 0, 0, 1, 0]); },
  Lightness(amount: number): ColorFilter { const a = amount / 255; return SkiaImageEffects.Matrix([1, 0, 0, 0, a, 0, 1, 0, 0, a, 0, 0, 1, 0, a, 0, 0, 0, 1, 0]); },
  /** C# uses a lookup table (SKColorFilter.CreateTable); CanvasKit has no table filter, so gamma is approximated by a linear contrast/brightness fit. */
  Gamma(gamma: number): ColorFilter {
    if (gamma < 1) gamma += 1; else if (gamma > 1) gamma -= 1;
    const g = Math.max(0.01, gamma);
    // best linear fit of x^g on [0,1]: slope/offset from the endpoints and midpoint
    const mid = Math.pow(0.5, g), slope = 2 * (mid - 0) , offset = 0.5 - slope * 0.5 + (mid - 0.5) * 0;
    return SkiaImageEffects.Matrix([slope, 0, 0, 0, offset, 0, slope, 0, 0, offset, 0, 0, slope, 0, offset, 0, 0, 0, 1, 0]);
  },
  TintSL(tint: Color, saturation: number, lightness: number, mode: BlendMode): ColorFilter {
    const CK = Super.CK;
    const a = CK.ColorFilter.MakeCompose(SkiaImageEffects.Lightness(lightness), SkiaImageEffects.Saturation(saturation));
    return CK.ColorFilter.MakeCompose(SkiaImageEffects.Tint(tint, mode), a);
  },
};
