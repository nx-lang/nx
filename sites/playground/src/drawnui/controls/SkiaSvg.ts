import type { Image } from "canvaskit-wasm";
import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type Color, Colors, type DrawImageAlignment, SKRect, ScaledSize, type TransformAspect } from "../core/Types";
import { SkiaImage } from "./SkiaImage";

/**
 * Mirrors DrawnUi SkiaSvg. CanvasKit's npm build has no SVG renderer, so the browser decodes the
 * SVG (HTMLImageElement) and it is rasterized at the exact pixel size it is displayed at, re-rasterized
 * only when that size changes — crisp at any scale, one bitmap per size. TintColor recolors via SrcIn.
 */
export class SkiaSvg extends SkiaControl {
  Aspect: TransformAspect = "AspectFitFill";
  HorizontalAlignment: DrawImageAlignment = "Center";
  VerticalAlignment: DrawImageAlignment = "Center";
  /** Transparent = keep the SVG's own colors. */
  TintColor: Color = Colors.Transparent;

  Success?: (sender: SkiaSvg, source: string) => void;
  Error?: (sender: SkiaSvg, error: Error) => void;
  IsLoading = false;
  /** Where the scaled svg was drawn last frame, canvas pixels. */
  DisplayRect: SKRect = SKRect.Empty;

  private source = "";
  private svgString = "";
  private loadGeneration = 0;
  private element?: HTMLImageElement;

  constructor() {
    super();
    this.UseCache = "Operations"; // DrawnUi default for SkiaSvg
  }
  private intrinsic = { Width: 0, Height: 0 };
  private raster?: Image;
  private rasterKey = "";

  /** URL of an .svg file (relative to the page or absolute). */
  get Source(): string { return this.source; }
  set Source(value: string) {
    if (this.source === value) return;
    this.source = value;
    this.svgString = "";
    if (!value) { this.Clear(); return; }
    const generation = ++this.loadGeneration;
    this.IsLoading = true;
    fetch(value)
      .then((r) => { if (!r.ok) throw new Error(`DrawnUi: ${r.status} loading svg '${value}'`); return r.text(); })
      .then((text) => { if (generation === this.loadGeneration) return this.Decode(text, value, generation); })
      .catch((e: Error) => { if (generation !== this.loadGeneration) return; this.IsLoading = false; this.Error?.(this, e); });
  }

  /** Inline SVG markup; takes precedence over Source. */
  get SvgString(): string { return this.svgString; }
  set SvgString(value: string) {
    if (this.svgString === value) return;
    this.svgString = value;
    this.source = "";
    if (!value) { this.Clear(); return; }
    const generation = ++this.loadGeneration;
    void this.Decode(value, "", generation);
  }

  private Clear(): void {
    this.loadGeneration++;
    this.element = undefined;
    this.intrinsic = { Width: 0, Height: 0 };
    this.DropRaster();
    this.IsLoading = false;
    this.Update();
  }

  private DropRaster(): void {
    this.raster?.delete();
    this.raster = undefined;
    this.rasterKey = "";
  }

  private async Decode(svg: string, sourceName: string, generation: number): Promise<void> {
    this.IsLoading = true;
    // browsers refuse to rasterize an <img> SVG whose root has no xmlns (SkiaSharp's parser does not care)
    if (!/<svg[^>]*\sxmlns\s*=/.test(svg)) svg = svg.replace(/<svg(?=[\s>])/, '<svg xmlns="http://www.w3.org/2000/svg"');
    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    try {
      const img = new window.Image();
      img.src = url;
      await img.decode();
      if (generation !== this.loadGeneration) return;
      this.element = img;
      this.intrinsic = SkiaSvg.IntrinsicSize(img, svg);
      this.DropRaster();
      this.IsLoading = false;
      this.Update();
      this.Success?.(this, sourceName);
    } catch (e) {
      if (generation !== this.loadGeneration) return;
      this.IsLoading = false;
      this.Error?.(this, e instanceof Error ? e : new Error(String(e)));
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  /** Natural size: the decoded image's, else the viewBox (SVGs without width/height decode as 0x0 or 300x150). */
  private static IntrinsicSize(img: HTMLImageElement, svg: string): { Width: number; Height: number } {
    const vb = /viewBox\s*=\s*"([^"]+)"/i.exec(svg);
    if (vb) {
      const p = vb[1].trim().split(/[\s,]+/).map(Number);
      if (p.length === 4 && p[2] > 0 && p[3] > 0) return { Width: p[2], Height: p[3] };
    }
    return { Width: img.naturalWidth || 1, Height: img.naturalHeight || 1 };
  }

  /** Bitmap of the svg at exactly w x h pixels (cached until the size changes). */
  private GetRaster(w: number, h: number): Image | undefined {
    const el = this.element;
    if (!el || w < 1 || h < 1) return undefined;
    const key = `${w}x${h}`;
    if (this.raster && this.rasterKey === key) return this.raster;
    this.DropRaster();
    const surface = typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(w, h) : Object.assign(document.createElement("canvas"), { width: w, height: h });
    const ctx = surface.getContext("2d") as OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D | null;
    if (!ctx) return undefined;
    ctx.drawImage(el, 0, 0, w, h);
    this.raster = Super.CK.MakeImageFromCanvasImageSource(surface);
    this.rasterKey = key;
    return this.raster;
  }

  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    let w = widthConstraint, h = heightConstraint;
    const { Width, Height } = this.intrinsic;
    if (Width > 0 && Height > 0) {
      const aspect = Width / Height;
      if (!isFinite(w) && isFinite(h)) w = h * aspect;
      else if (!isFinite(h) && isFinite(w)) h = w / aspect;
    }
    if (!isFinite(w) || !isFinite(h)) return ScaledSize.FromPixels(0, 0, scale);
    return ScaledSize.FromPixels(w, h, scale);
  }

  protected override Paint(ctx: DrawingContext): void {
    const { Width, Height } = this.intrinsic;
    if (!this.element || Width <= 0 || Height <= 0) return;
    const dest = ctx.Destination;
    const scaled = SkiaImage.RescaleAspect(Width, Height, dest, this.Aspect);
    const display = SkiaImage.CalculateDisplayRect(dest, Width * scaled.X, Height * scaled.Y, this.HorizontalAlignment, this.VerticalAlignment);
    this.DisplayRect = display;
    const raster = this.GetRaster(Math.round(display.Width), Math.round(display.Height));
    if (!raster) return;

    const CK = Super.CK;
    const canvas = ctx.Context.Canvas;
    const overflows = display.Left < dest.Left || display.Top < dest.Top || display.Right > dest.Right || display.Bottom > dest.Bottom;
    const saved = canvas.save();
    if (overflows) canvas.clipRect(CK.LTRBRect(dest.Left, dest.Top, dest.Right, dest.Bottom), CK.ClipOp.Intersect, true);
    const paint = new CK.Paint();
    paint.setAntiAlias(true);
    const tint = Super.ParseColor(this.TintColor);
    if (tint[3] > 0) {
      const filter = CK.ColorFilter.MakeBlend(tint, CK.BlendMode.SrcIn);
      paint.setColorFilter(filter);
      filter.delete();
    }
    canvas.drawImageRectOptions(raster, CK.LTRBRect(0, 0, raster.width(), raster.height()),
      CK.LTRBRect(display.Left, display.Top, display.Right, display.Bottom), CK.FilterMode.Linear, CK.MipmapMode.None, paint);
    paint.delete();
    canvas.restoreToCount(saved);
  }
}
