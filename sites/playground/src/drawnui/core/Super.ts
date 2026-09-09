import { Thickness } from "./Types";
import { StylesCollection } from "./Styles";
// the "full" build: same API plus Skottie (SkiaLottie) and the paragraph module; +0.9 MB raw over the default build
import CanvasKitInit from "canvaskit-wasm/bin/full/canvaskit.js";
import type { CanvasKit, Font, Typeface } from "canvaskit-wasm";
import wasmUrl from "canvaskit-wasm/bin/full/canvaskit.wasm?url";

/** Mirrors DrawnUi.Net IFontCollection: fonts.AddFont(source, alias[, weight]), plus the Blazor-head AddSymbols/AddEmojis. */
export class FontCollection {
  /** Where the shipped subset fonts are served from (the demo copies them to public/fonts). */
  static ContentRoot = "fonts/";
  readonly Fonts: { Source: string; Alias: string; Weight: number }[] = [];

  /** Noto Sans Math + Symbols 2 subsets as "FontSymbols" / "FontSymbols2" (arrows, math, misc symbols) — DrawnUi.Blazor AddSymbols. */
  AddSymbols(): FontCollection {
    return this.AddFont(FontCollection.ContentRoot + "NotoSansMathSymbols-Subset.ttf", "FontSymbols")
      .AddFont(FontCollection.ContentRoot + "NotoSansSymbols2-Subset.ttf", "FontSymbols2");
  }

  /** Noto Color Emoji subset (faces + hands) as "FontEmoji" — DrawnUi.Blazor AddEmojis. */
  AddEmojis(): FontCollection {
    return this.AddFont(FontCollection.ContentRoot + "NotoColorEmoji-Subset.ttf", "FontEmoji");
  }
  /** Registers a face under an alias; weight 100..900 lets FontWeight / FontAttributes=Bold pick the right file. */
  AddFont(source: string, alias: string, weight = 400): FontCollection {
    this.Fonts.push({ Source: source, Alias: alias, Weight: weight });
    return this;
  }
}

/** Mirrors DrawnUi.Net DrawnUiBuilder: Super.UseDrawnUi().ConfigureFonts(...).ConfigureStyles(...).BuildAsync(). */
export class DrawnUiBuilder {
  private readonly fonts = new FontCollection();

  ConfigureFonts(configure: (fonts: FontCollection) => void): DrawnUiBuilder {
    configure(this.fonts);
    return this;
  }

  /**
   * Mirrors DrawnUi.Net ConfigureStyles: property defaults per control type, applied to every control unless the app
   * sets that property itself. The usual one, same as the .NET Blazor / Fiddle hosts:
   * `styles.AddStyle({ TargetType: SkiaLabel, ApplyToDerivedTypes: true, Setters: { FontFamily: "FontText" } })`.
   */
  ConfigureStyles(configure: (styles: StylesCollection) => void): DrawnUiBuilder {
    configure(new StylesCollection());
    return this;
  }

  /** Loads CanvasKit + registered fonts. Must complete before the first Canvas is created. */
  async BuildAsync(): Promise<void> {
    Super.CK = await CanvasKitInit({ locateFile: () => wasmUrl });
    const loaded = new Map<string, Typeface>(); // same file registered twice = one face
    for (const f of this.fonts.Fonts) {
      let face = loaded.get(f.Source);
      if (!face) {
        const data = await (await fetch(f.Source)).arrayBuffer();
        face = Super.CK.Typeface.MakeFreeTypeFaceFromData(data) ?? undefined;
        if (!face) throw new Error(`DrawnUi: cannot load font '${f.Source}'`);
        loaded.set(f.Source, face);
        // the same face as a CSS font under the alias: the accessibility overlay's selectable text lines up with the drawn glyphs
        if (typeof FontFace !== "undefined" && typeof document !== "undefined") {
          try { const css = new FontFace(f.Alias, data, { weight: String(f.Weight) }); await css.load(); document.fonts.add(css); } catch { /* selection text falls back to a system font */ }
        }
      }
      let weights = Super.Fonts.get(f.Alias);
      if (!weights) { weights = new Map(); Super.Fonts.set(f.Alias, weights); }
      weights.set(f.Weight, face);
      Super.DefaultTypeface ??= face;
      if (!Super.DefaultFontAlias) Super.DefaultFontAlias = f.Alias;
    }
    Super.SkiaDefaultTypeface ??= Super.CK.Typeface.GetDefault() ?? undefined;
    Super.DefaultTypeface ??= Super.SkiaDefaultTypeface;
  }
}

/** Mirrors DrawnUi static Super: global engine state. */

export class Super {
  // ---- insets (C# Super.Insets / InsetsChanged): the browser's safe area from env(safe-area-inset-*) ----
  private static insets?: Thickness;
  private static insetsProbe?: HTMLDivElement;
  private static readonly insetsChanged = new Set<() => void>();
  /** Safe-area insets in CSS px (points), measured once and on resize; zero outside a browser. */
  static get Insets(): Thickness {
    if (!Super.insets) Super.MeasureInsets();
    return Super.insets ?? Thickness.Zero;
  }
  /** Subscribe to inset changes (orientation / resize); returns the unsubscribe. */
  static OnInsetsChanged(handler: () => void): () => void { Super.insetsChanged.add(handler); return () => Super.insetsChanged.delete(handler); }
  static MeasureInsets(): void {
    if (typeof document === "undefined") { Super.insets = Thickness.Zero; return; }
    let probe = Super.insetsProbe;
    if (!probe) {
      probe = document.createElement("div");
      probe.style.cssText = "position:fixed;left:0;top:0;width:0;height:0;visibility:hidden;pointer-events:none;padding:env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);";
      document.body.appendChild(probe);
      Super.insetsProbe = probe;
      window.addEventListener("resize", () => Super.MeasureInsets());
    }
    const cs = getComputedStyle(probe);
    const next = new Thickness(parseFloat(cs.paddingLeft) || 0, parseFloat(cs.paddingTop) || 0, parseFloat(cs.paddingRight) || 0, parseFloat(cs.paddingBottom) || 0);
    const prev = Super.insets;
    Super.insets = next;
    if (prev && (prev.Left !== next.Left || prev.Top !== next.Top || prev.Right !== next.Right || prev.Bottom !== next.Bottom)) for (const h of [...Super.insetsChanged]) h();
  }

  /** CanvasKit instance, valid after BuildAsync(). */
  static CK: CanvasKit;
  /** Registered typefaces by alias (FontFamily) and weight. */
  static readonly Fonts = new Map<string, Map<number, Typeface>>();
  /** First registered font, or CanvasKit's built-in one. */
  static DefaultTypeface?: Typeface;
  /** CanvasKit's built-in face: what an empty FontFamily resolves to, like C# SKTypeface.CreateDefault(). */
  static SkiaDefaultTypeface?: Typeface;
  private static readonly fontCache = new Map<string, Font>();

  static UseDrawnUi(): DrawnUiBuilder { return new DrawnUiBuilder(); }

  /** Master switch: false makes every control render uncached (DrawnUi Super.CacheEnabled). */
  static CacheEnabled = true;

  private static readonly colorCache = new Map<string, Float32Array>();

  /**
   * Color string -> CanvasKit color. Hex follows the MAUI/DrawnUi convention: #RGB, #ARGB, #RRGGBB, #AARRGGBB
   * (alpha FIRST — CSS puts it last, CanvasKit's own parser would read "#22FFFFFF" as opaque cyan).
   * rgb()/rgba() strings are passed to CanvasKit as is.
   */
  static ParseColor(color: string): Float32Array {
    let c = Super.colorCache.get(color);
    if (c) return c;
    if (color.startsWith("#")) {
      let h = color.slice(1);
      if (h.length === 3 || h.length === 4) h = [...h].map((ch) => ch + ch).join("");
      let a = 1, rgb = h;
      if (h.length === 8) { a = parseInt(h.slice(0, 2), 16) / 255; rgb = h.slice(2); }
      const r = parseInt(rgb.slice(0, 2), 16) / 255, g = parseInt(rgb.slice(2, 4), 16) / 255, b = parseInt(rgb.slice(4, 6), 16) / 255;
      c = Super.CK.Color4f(r, g, b, a);
    } else {
      c = Super.CK.parseColorString(color);
    }
    Super.colorCache.set(color, c);
    return c;
  }

  /** Typeface for an alias at a weight: exact, else the nearest registered weight, else the default face. */
  /** Alias of the first registered font: what an empty FontFamily resolves to (weights included). */
  static DefaultFontAlias = "";

  static GetTypeface(alias?: string, weight = 0): Typeface | null {
    return Super.ResolveTypeface(alias, weight).Typeface;
  }

  /** Nearest registered weight of the alias (empty alias = the first registered family); reports the weight actually used. */
  private static ResolveTypeface(alias: string | undefined, weight: number): { Typeface: Typeface | null; Weight: number } {
    // No family: the Skia built-in face, like C# SkiaFontManager.GetFont("") -> SKTypeface.CreateDefault(). An app that
    // wants its own font everywhere registers it as a style default (ConfigureStyles), exactly as the .NET hosts do.
    if (!alias) return { Typeface: Super.SkiaDefaultTypeface ?? Super.DefaultTypeface ?? null, Weight: 400 };
    const weights = Super.Fonts.get(alias);
    const target = weight > 0 ? weight : 400;
    if (weights && weights.size > 0) {
      if (weights.has(target)) return { Typeface: weights.get(target)!, Weight: target };
      let best: number | undefined;
      for (const w of weights.keys()) if (best === undefined || Math.abs(w - target) < Math.abs(best - target)) best = w;
      return { Typeface: weights.get(best!)!, Weight: best! };
    }
    return { Typeface: Super.DefaultTypeface ?? null, Weight: 400 };
  }

  /** Shared Font (typeface + pixel size + synthetic italic), cached across all labels. */
  static GetFont(alias: string, weight: number, italic: boolean, sizePx: number): Font {
    const key = `${alias}|${weight}|${italic ? 1 : 0}|${sizePx}`;
    let font = Super.fontCache.get(key);
    if (!font) {
      const resolved = Super.ResolveTypeface(alias, weight);
      font = new Super.CK.Font(resolved.Typeface, sizePx);
      font.setSubpixel(true);
      if (italic) font.setSkewX(-0.25);
      // no bold face registered: synthetic bold, like C# Font.Embolden = IsBold
      if (weight >= 600 && resolved.Weight < 600) font.setEmbolden(true);
      Super.fontCache.set(key, font);
    }
    return font;
  }
}
