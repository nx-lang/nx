import type { Path } from "canvaskit-wasm";
import { type DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type Color, Colors, CornerRadius, SKRect, type ShapeType, SkiaShadow, type SkiaPoint, type StrokeCap, Thickness } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SkiaShape: a SkiaLayout (Absolute) that paints a geometric background, strokes its outline
 * INSIDE its bounds (the rect is deflated by half the stroke, so the outer stroke edge sits on DrawingRect),
 * and clips its children to the stroke-inset shape. Cached as Operations by default, like DrawnUi.
 */
export class SkiaShape extends SkiaLayout {
  /** Rectangle | Circle | Ellipse | Arc | Path | Polygon | Line (Squricle/Custom draw as Rectangle). Layout is always Absolute. */
  declare Type: ShapeType;
  private cornerRadius: CornerRadius = CornerRadius.Zero;
  /** Uniform number or per-corner CornerRadius, points. */
  get CornerRadius(): CornerRadius | number { return this.cornerRadius; }
  set CornerRadius(v: CornerRadius | number) { this.cornerRadius = typeof v === "number" ? new CornerRadius(v) : v; this.Update(); }
  /** Points; a negative value means device pixels. 0 = no stroke. */
  StrokeWidth = 0;
  StrokeColor: Color = Colors.Gray;
  StrokeCap: StrokeCap = "Round";
  /** Hollow shape: the background is not filled, only the stroke (and children) are drawn. */
  ClipBackgroundColor = false;
  /** Polygon / Line vertices as ratios (0..1) of the shape rect. */
  Points: SkiaPoint[] = [];
  /** SVG path data for Type=Path, fitted and centered inside the rect. */
  PathData?: string;

  private shadows: SkiaShadow[] = [];
  /** Drop shadows painted with the background (DrawnUi SkiaShape.Shadows); plain `{X, Y, Blur, Opacity, Color}` literals accepted. */
  get Shadows(): (SkiaShadow | Partial<SkiaShadow>)[] { return this.shadows; }
  set Shadows(v: (SkiaShadow | Partial<SkiaShadow>)[] | undefined) { this.shadows = (v ?? []).map(SkiaShadow.From); this.Update(); }

  protected override PaintsBackgroundWithoutColor(): boolean { return this.shadows.length > 0; }

  /** C# MergeShadowMargin: spread = 3 * Blur * scale around the offset shadow. */
  override ComputeEffectsMargin(scale: number): Thickness {
    const inner = super.ComputeEffectsMargin(scale);
    let l = inner.Left, t = inner.Top, r = inner.Right, b = inner.Bottom;
    for (const s of this.shadows) {
      const spread = 3 * s.Blur * scale, dx = s.X * scale, dy = s.Y * scale;
      l = Math.max(l, spread - dx); t = Math.max(t, spread - dy); r = Math.max(r, spread + dx); b = Math.max(b, spread + dy);
    }
    return new Thickness(l, t, r, b);
  }

  /** C# CreateShadow: DropShadow (shape + shadow) or DropShadowOnly; a fully opaque Color takes the shadow's Opacity. */
  static ShadowFilter(shadow: SkiaShadow, scale: number) {
    const CK = Super.CK;
    const c = Super.ParseColor(shadow.Color);
    const color = c[3] >= 0.999 ? CK.Color4f(c[0], c[1], c[2], shadow.Opacity) : c;
    const make = shadow.ShadowOnly ? CK.ImageFilter.MakeDropShadowOnly : CK.ImageFilter.MakeDropShadow;
    return make(shadow.X * scale, shadow.Y * scale, shadow.Blur * scale, shadow.Blur * scale, color, null);
  }

  constructor() {
    super();
    this.Type = "Rectangle";
    this.UseCache = "Operations";
  }

  // ---- geometry ----

  private StrokePixels(scale: number): number {
    if (this.StrokeWidth === 0 || this.StrokeColor === Colors.Transparent) return 0;
    return this.StrokeWidth > 0 ? this.StrokeWidth * scale : -this.StrokeWidth;
  }

  /** Rect the outline is drawn on: bounds deflated by half the stroke so the stroke stays inside. */
  private StrokeAwareRect(r: SKRect, scale: number): SKRect {
    const half = this.StrokePixels(scale) / 2;
    return new SKRect(r.Left + half, r.Top + half, r.Right - half, r.Bottom - half);
  }

  private UsesStroke(): boolean { return this.StrokeWidth !== 0 && this.StrokeColor !== Colors.Transparent; }

  /** Builds the outline path for rect (canvas pixels). Caller deletes it. */
  protected CreateShapePath(rect: SKRect, scale: number): Path {
    const CK = Super.CK;
    const b = new CK.PathBuilder();
    const type = this.Type;
    if (type === "Circle") {
      const d = Math.min(rect.Width, rect.Height);
      const cx = rect.Left + rect.Width / 2, cy = rect.Top + rect.Height / 2;
      b.addOval(CK.LTRBRect(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2));
    } else if (type === "Ellipse") {
      b.addOval(CK.LTRBRect(rect.Left, rect.Top, rect.Right, rect.Bottom));
    } else if (type === "Arc") {
      b.addArc(CK.LTRBRect(rect.Left, rect.Top, rect.Right, rect.Bottom), this.Value1, this.Value2);
    } else if (type === "Polygon" || type === "Line") {
      const pts = this.Points;
      if (pts.length >= 2) {
        const p = (i: number) => [Math.round(rect.Left + pts[i].X * rect.Width), Math.round(rect.Top + pts[i].Y * rect.Height)] as const;
        b.moveTo(...p(0));
        for (let i = 1; i < pts.length; i++) b.lineTo(...p(i));
        if (type === "Polygon") b.close();
      }
    } else if (type === "Path" && this.PathData) {
      const src = CK.Path.MakeFromSVGString(this.PathData);
      if (src) {
        const bounds = src.getBounds(); // [l, t, r, b]
        const w = bounds[2] - bounds[0], h = bounds[3] - bounds[1];
        const s = w > 0 && h > 0 ? Math.min(rect.Width / w, rect.Height / h) : 1;
        const tx = rect.Left + (rect.Width - w * s) / 2 - bounds[0] * s;
        const ty = rect.Top + (rect.Height - h * s) / 2 - bounds[1] * s;
        b.addPath(src, s, 0, tx, 0, s, ty, 0, 0, 1); // addPath(path, 3x3 matrix)
        src.delete();
      }
    } else {
      const c = this.cornerRadius;
      if (c.TopLeft || c.TopRight || c.BottomLeft || c.BottomRight) {
        const tl = c.TopLeft * scale, tr = c.TopRight * scale, br = c.BottomRight * scale, bl = c.BottomLeft * scale;
        b.addRRect(Float32Array.of(rect.Left, rect.Top, rect.Right, rect.Bottom, tl, tl, tr, tr, br, br, bl, bl));
      } else {
        b.addRect(CK.LTRBRect(rect.Left, rect.Top, rect.Right, rect.Bottom));
      }
    }
    const path = b.detach();
    b.delete();
    return path;
  }

  /** Children live inside the stroke (full width) + Padding. */
  private ChildrenRect(): SKRect {
    const scale = this.RenderingScale;
    const s = this.StrokePixels(scale);
    const r = this.DrawingRect;
    return new SKRect(r.Left + s, r.Top + s, r.Right - s, r.Bottom - s);
  }

  /** Clip for overlay effects (ripple) and children: the shape itself, inset by the stroke. */
  override CreateClip(): Path {
    return this.CreateShapePath(this.ChildrenRect(), this.RenderingScale);
  }

  // ---- layout ----

  protected override OnLayoutChanged(): void {
    // Absolute layout of children inside the stroke-inset rect, then Padding (handled by base via DrawingRect).
    const saved = this.DrawingRect;
    (this as { DrawingRect: SKRect }).DrawingRect = this.ChildrenRect();
    super.OnLayoutChanged();
    (this as { DrawingRect: SKRect }).DrawingRect = saved;
  }

  // ---- paint ----

  protected override PaintBackground(ctx: DrawingContext): void {
    const type = this.Type;
    if (type === "Arc" || type === "Line") return; // open shapes: stroke only
    const CK = Super.CK;
    const canvas = ctx.Context.Canvas;
    const rect = this.StrokeAwareRect(ctx.Destination, ctx.Scale);
    const path = this.CreateShapePath(rect, ctx.Scale);
    // C# PaintWithShadowsInternal: the fill is drawn once per shadow with a drop-shadow filter; a hollow shape
    // (ClipBackgroundColor) clips its own outline away so only the shadow outside remains
    for (const shadow of this.shadows) {
      const paint = this.CreateBackgroundPaint(rect);
      if (this.ClipBackgroundColor) paint.setColor(Super.ParseColor(this.StrokeColor === Colors.Transparent ? Colors.Black : this.StrokeColor));
      const filter = SkiaShape.ShadowFilter(shadow, ctx.Scale);
      paint.setImageFilter(filter);
      const saved = canvas.save();
      if (this.ClipBackgroundColor) canvas.clipPath(path, CK.ClipOp.Difference, true);
      canvas.drawPath(path, paint);
      canvas.restoreToCount(saved);
      filter.delete();
      paint.delete();
    }
    if (!this.ClipBackgroundColor && (this.shadows.length === 0 || this.shadows.some((s) => s.ShadowOnly))) {
      const paint = this.CreateBackgroundPaint(rect);
      canvas.drawPath(path, paint);
      paint.delete();
    }
    path.delete();
  }

  protected override Paint(ctx: DrawingContext): void {
    const CK = Super.CK;
    const canvas = ctx.Context.Canvas;
    if (this.Views.length > 0) {
      const clip = this.CreateClip();
      const saved = canvas.save();
      canvas.clipPath(clip, CK.ClipOp.Intersect, true);
      super.Paint(ctx); // children
      canvas.restoreToCount(saved);
      clip.delete();
    }
    if (this.UsesStroke()) {
      const rect = this.StrokeAwareRect(ctx.Destination, ctx.Scale);
      const path = this.CreateShapePath(rect, ctx.Scale);
      const paint = new CK.Paint();
      paint.setAntiAlias(true);
      paint.setStyle(CK.PaintStyle.Stroke);
      paint.setStrokeWidth(Math.max(1, this.StrokePixels(ctx.Scale)));
      paint.setColor(Super.ParseColor(this.StrokeColor));
      paint.setStrokeCap(this.StrokeCap === "Butt" ? CK.StrokeCap.Butt : this.StrokeCap === "Square" ? CK.StrokeCap.Square : CK.StrokeCap.Round);
      paint.setStrokeJoin(this.StrokeCap === "Round" ? CK.StrokeJoin.Round : CK.StrokeJoin.Miter);
      canvas.drawPath(path, paint);
      paint.delete();
      path.delete();
    }
  }
}

/** DrawnUi SkiaFrame = SkiaShape Type=Rectangle (alias, not a layout type). */
export class SkiaFrame extends SkiaShape {}
