import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type PrebuiltControlStyle, ResolveControlStyle, type ResolvedControlStyle } from "../core/ControlStyle";
import { type Color, ScaledSize } from "../core/Types";

/**
 * Mirrors DrawnUi SkiaProgress (SkiaRangeBase): a horizontal track with a progress trail, per-style track height and
 * palette (Material3 adds the gap + stop dot). Painted directly instead of child shapes; same geometry as the C# builders.
 */
export class SkiaProgress extends SkiaControl {
  static override DefaultAccessibilityRole?: string = "progressbar";
  ControlStyle: PrebuiltControlStyle = "Unset";
  private value = 0;
  private min = 0;
  private max = 100;
  private trackColor?: Color;
  private progressColor?: Color;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.MinimumWidthRequest = 100;
    this.UseCache = "ImageDoubleBuffered";
  }

  get UsingControlStyle(): ResolvedControlStyle { return ResolveControlStyle(this.ControlStyle); }
  get Value(): number { return this.value; }
  set Value(v: number) { if (this.value !== v) { this.value = v; this.Update(); this.NotifyAccessibility(); } }
  get Min(): number { return this.min; }
  set Min(v: number) { this.min = v; this.Update(); }
  get Max(): number { return this.max; }
  set Max(v: number) { this.max = v; this.Update(); }
  /** Unset = per-style palette. */
  get TrackColor(): Color { return this.trackColor ?? SkiaProgress.Palette(this.UsingControlStyle).track; }
  set TrackColor(v: Color) { this.trackColor = v; this.Update(); }
  get ProgressColor(): Color { return this.progressColor ?? SkiaProgress.Palette(this.UsingControlStyle).progress; }
  set ProgressColor(v: Color) { this.progressColor = v; this.Update(); }

  /** C# ResolvedTrackColor / ResolvedProgressColor / track heights per style. */
  static Palette(s: ResolvedControlStyle): { track: Color; progress: Color; height: number; radius: number } {
    switch (s) {
      case "Cupertino": return { track: "#E5E5EA", progress: "#007AFF", height: 4, radius: 2 };
      case "Material": return { track: "#E8EAED", progress: "#2196F3", height: 4, radius: 2 };
      case "Material3": return { track: "#E6E0E9", progress: "#6750A4", height: 4, radius: 2 };
      case "Windows": return { track: "#F3F2F1", progress: "#0078D4", height: 6, radius: 3 };
      default: return { track: "#D7DBE0", progress: "#DC143C", height: 8, radius: 4 };
    }
  }

  get Ratio(): number { return this.max > this.min ? Math.max(0, Math.min(1, (this.value - this.min) / (this.max - this.min))) : 0; }

  protected override MeasureAbsolute(w: number, _h: number, scale: number): ScaledSize {
    const p = SkiaProgress.Palette(this.UsingControlStyle);
    return ScaledSize.FromPixels(isFinite(w) ? w : 200 * scale, p.height * scale, scale);
  }

  protected override Paint(ctx: DrawingContext): void {
    const CK = Super.CK;
    const p = SkiaProgress.Palette(this.UsingControlStyle);
    const scale = ctx.Scale;
    const d = ctx.Destination;
    const h = p.height * scale, r = p.radius * scale;
    const top = d.Top + (d.Height - h) / 2;
    const progressW = d.Width * this.Ratio;
    const paint = new CK.Paint();
    paint.setAntiAlias(true);
    const m3 = this.UsingControlStyle === "Material3";
    // background track (Material3: starts after the trail + 4pt gap)
    paint.setColor(Super.ParseColor(this.TrackColor));
    const bgLeft = m3 && progressW > 0 ? Math.min(d.Right, d.Left + progressW + 4 * scale) : d.Left;
    if (d.Right - bgLeft > 0) ctx.Context.Canvas.drawRRect(CK.RRectXY(CK.LTRBRect(bgLeft, top, d.Right, top + h), r, r), paint);
    paint.setColor(Super.ParseColor(this.ProgressColor));
    if (progressW > 0) ctx.Context.Canvas.drawRRect(CK.RRectXY(CK.LTRBRect(d.Left, top, d.Left + progressW, top + h), r, r), paint);
    if (m3) { const dot = 4 * scale; ctx.Context.Canvas.drawCircle(d.Right - dot / 2, top + h / 2, dot / 2, paint); }
    paint.delete();
  }

  protected override DefaultAccessibilityLabel(): string | undefined { return `${Math.round(this.Ratio * 100)}%`; }
}
