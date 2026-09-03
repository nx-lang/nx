import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { Super } from "../core/Super";
import type { GestureEventProcessingInfo, SkiaGesturesParameters } from "../core/Gestures";
import { type PrebuiltControlStyle, ResolveControlStyle, type ResolvedControlStyle } from "../core/ControlStyle";
import { type Color, Colors, ScaledSize, SkiaShadow, Thickness } from "../core/Types";
import { SkiaShape } from "./SkiaShape";

type RangeZone = "Unknown" | "Start" | "End";

/**
 * Mirrors DrawnUi SkiaSlider: single or range (`EnableRange`) slider with Min/Max/Step, `Start`/`End` values,
 * thumb drag, click-on-trail, per-style looks (Default / Cupertino / Material / Material3 / Windows).
 * Track, selected trail and thumbs are painted directly; the C# thumb-position math (SliderHeight = thumb box) is kept.
 */
export class SkiaSlider extends SkiaControl {
  static override DefaultAccessibilityRole?: string = "slider";
  ControlStyle: PrebuiltControlStyle = "Unset";
  Min = 0;
  Max = 100;
  Step = 1;
  EnableRange = false;
  /** Minimum distance between the two thumbs, in value units. */
  RangeMin = 0;
  ClickOnTrailEnabled = true;
  RespondsToGestures = true;
  IsPressed = false;
  IsUserPanning = false;
  /** Extra points around a thumb that still count as grabbing it (C# moreHotspotSize). */
  MoreHotspotSize = 10;
  StartChanged?: (sender: SkiaSlider, value: number) => void;
  EndChanged?: (sender: SkiaSlider, value: number) => void;

  private start = 0;
  private end = 0;
  private sliderHeight?: number;
  private thumbColor?: Color;
  private trackColor?: Color;
  private trackSelectedColor?: Color;
  private touchArea: RangeZone = "Unknown";
  private lastTouchX = 0;
  /** Thumb positions in points from the left edge (C# StartThumbX / EndThumbX). */
  StartThumbX = 0;
  EndThumbX = 0;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.MinimumWidthRequest = 64;
    this.UseCache = "ImageDoubleBuffered";
  }

  get UsingControlStyle(): ResolvedControlStyle { return ResolveControlStyle(this.ControlStyle); }

  get Start(): number { return this.start; }
  set Start(v: number) { v = this.Clamp(v); if (this.start !== v) { this.start = v; this.StartChanged?.(this, v); this.SyncThumbsFromValues(); } }
  get End(): number { return this.end; }
  set End(v: number) { v = this.Clamp(v); if (this.end !== v) { this.end = v; this.EndChanged?.(this, v); this.SyncThumbsFromValues(); this.NotifyAccessibility(); } }

  /** Thumb box size in points (C# SliderHeight); unset = per style (35 default, 28 Cupertino, 20 Material/Windows, +8 in range mode). */
  get SliderHeight(): number { return this.sliderHeight ?? this.Look().thumb + (this.EnableRange && this.UsingControlStyle !== "Unset" ? 8 : 0); }
  set SliderHeight(v: number) { this.sliderHeight = v; this.Update(); }
  get ThumbColor(): Color { return this.thumbColor ?? this.Look().thumbColor; }
  set ThumbColor(v: Color) { this.thumbColor = v; this.Update(); }
  get TrackColor(): Color { return this.trackColor ?? this.Look().track; }
  set TrackColor(v: Color) { this.trackColor = v; this.Update(); }
  get TrackSelectedColor(): Color { return this.trackSelectedColor ?? this.Look().selected; }
  set TrackSelectedColor(v: Color) { this.trackSelectedColor = v; this.Update(); }

  /** C# style builders: thumb diameter, track height, palette, thumb shadow. */
  private Look(): { thumb: number; trackH: number; track: Color; selected: Color; thumbColor: Color; shadow: SkiaShadow } {
    switch (this.UsingControlStyle) {
      case "Cupertino": return { thumb: 28, trackH: 2, track: "#CCCCCC", selected: "#007AFF", thumbColor: Colors.White, shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 3, Opacity: 0.2, Color: Colors.Gray }) };
      case "Material": return { thumb: 20, trackH: 4, track: "#E8EAED", selected: "#2196F3", thumbColor: "#2196F3", shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 2, Opacity: 0.3, Color: Colors.Black }) };
      case "Material3": return { thumb: 20, trackH: 4, track: "#E6E0E9", selected: "#6750A4", thumbColor: "#6750A4", shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 2, Opacity: 0.3, Color: Colors.Black }) };
      case "Windows": return { thumb: 20, trackH: 4, track: "#C6C6C6", selected: "#0078D4", thumbColor: "#0078D4", shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 2, Opacity: 0.25, Color: Colors.Black }) };
      default: return { thumb: 35, trackH: 6, track: "#D7DBE0", selected: "#DC143C", thumbColor: "#DC143C", shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 3, Opacity: 0.25, Color: Colors.Black }) };
    }
  }

  /** Thumb shadows paint outside the box: the cache must include them. */
  override ComputeEffectsMargin(scale: number): Thickness {
    const s = this.Look().shadow;
    const spread = 3 * s.Blur * scale;
    return new Thickness(Math.max(0, spread - s.X * scale), Math.max(0, spread - s.Y * scale), spread + s.X * scale, spread + s.Y * scale);
  }

  private Clamp(v: number): number { return Math.max(this.Min, Math.min(this.Max, v)); }
  private get WidthPts(): number { return this.DrawingRect.Width / (this.RenderingScale || 1); }
  private get TotalLength(): number { return Math.max(0, this.WidthPts - this.SliderHeight); }

  private PositionFromValue(v: number): number { return this.Max > this.Min ? ((v - this.Min) / (this.Max - this.Min)) * this.TotalLength : 0; }
  private ValueFromPosition(x: number): number { const t = this.TotalLength; return t <= 0 ? this.Min : this.Min + (x / t) * (this.Max - this.Min); }
  private AdjustToStep(v: number): number { return this.Step > 0 ? this.Min + Math.round((v - this.Min) / this.Step) * this.Step : v; }

  private SyncThumbsFromValues(): void {
    this.StartThumbX = this.PositionFromValue(this.start);
    this.EndThumbX = this.PositionFromValue(this.end);
    this.Update();
  }

  private SetStartOffsetClamped(x: number): void {
    const max = this.EnableRange ? this.EndThumbX - this.RangeMin / (this.Step || 1) : this.TotalLength;
    this.StartThumbX = Math.max(0, Math.min(max, x));
  }
  private SetEndOffsetClamped(x: number): void {
    const min = this.EnableRange ? this.StartThumbX + this.RangeMin / (this.Step || 1) : 0;
    this.EndThumbX = Math.max(min, Math.min(this.TotalLength, x));
  }

  /** C# RecalculateValues: positions -> stepped values, raising the *Changed callbacks. */
  private RecalculateValues(): void {
    if (this.EnableRange) {
      const s = this.AdjustToStep(this.ValueFromPosition(this.StartThumbX));
      if (s !== this.start) { this.start = this.Clamp(s); this.StartChanged?.(this, this.start); }
    }
    const e = this.AdjustToStep(this.ValueFromPosition(this.EndThumbX));
    if (e !== this.end) { this.end = this.Clamp(e); this.EndChanged?.(this, this.end); this.NotifyAccessibility(); }
    this.Update();
  }

  protected override MeasureAbsolute(w: number, _h: number, scale: number): ScaledSize {
    return ScaledSize.FromPixels(isFinite(w) ? w : 200 * scale, this.SliderHeight * scale, scale);
  }

  protected override OnLayoutChanged(): void { this.StartThumbX = this.PositionFromValue(this.start); this.EndThumbX = this.PositionFromValue(this.end); }

  protected override Paint(ctx: DrawingContext): void {
    const CK = Super.CK;
    const look = this.Look();
    const scale = ctx.Scale;
    const d = ctx.Destination;
    const H = this.SliderHeight * scale;
    const cy = d.Top + d.Height / 2;
    const trackH = look.trackH * scale, r = trackH / 2;
    const thumb = look.thumb * scale;
    const paint = new CK.Paint(); paint.setAntiAlias(true);
    const canvas = ctx.Context.Canvas;
    // track
    paint.setColor(Super.ParseColor(this.TrackColor));
    canvas.drawRRect(CK.RRectXY(CK.LTRBRect(d.Left, cy - r, d.Right, cy + r), r, r), paint);
    // selected trail between thumb centers (from the left edge in single mode)
    const endC = d.Left + this.EndThumbX * scale + H / 2;
    const startC = this.EnableRange ? d.Left + this.StartThumbX * scale + H / 2 : d.Left;
    paint.setColor(Super.ParseColor(this.TrackSelectedColor));
    if (endC > startC) canvas.drawRRect(CK.RRectXY(CK.LTRBRect(startC, cy - r, endC, cy + r), r, r), paint);
    const drawThumb = (c: number) => {
      const s = this.UsingControlStyle;
      // the thumb's body carries the style shadow (C# SliderThumb shape Shadows)
      const shadowed = (radius: number, color: Color) => {
        const filter = SkiaShape.ShadowFilter(look.shadow, scale);
        paint.setImageFilter(filter); paint.setColor(Super.ParseColor(color)); canvas.drawCircle(c, cy, radius, paint);
        paint.setImageFilter(null); filter.delete();
      };
      if (s === "Unset") {
        const inner = thumb - 10 * scale; // C# Margin 5 around the accent circle
        shadowed(inner / 2, this.ThumbColor);
        paint.setColor(Super.ParseColor(Colors.White)); canvas.drawCircle(c, cy, 3 * scale, paint);
      } else if (s === "Windows") {
        shadowed(thumb / 2, Colors.White);
        paint.setStyle(CK.PaintStyle.Stroke); paint.setStrokeWidth(scale); paint.setColor(Super.ParseColor("#E5E5E5")); canvas.drawCircle(c, cy, thumb / 2 - scale / 2, paint);
        paint.setStyle(CK.PaintStyle.Fill); paint.setColor(Super.ParseColor(this.ThumbColor)); canvas.drawCircle(c, cy, 5 * scale, paint);
      } else if (s === "Cupertino") {
        shadowed(thumb / 2, this.ThumbColor);
        paint.setStyle(CK.PaintStyle.Stroke); paint.setStrokeWidth(0.5 * scale); paint.setColor(Super.ParseColor("#CCCCCC")); canvas.drawCircle(c, cy, thumb / 2, paint);
        paint.setStyle(CK.PaintStyle.Fill);
      } else {
        shadowed(thumb / 2, this.ThumbColor);
      }
    };
    if (this.EnableRange) drawThumb(startC);
    drawThumb(endC);
    paint.delete();
  }

  // ---- gestures (port of C# SkiaSlider.ProcessGestures, single pointer) ----
  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (!this.RespondsToGestures) return super.ProcessGestures(args, apply);
    const scale = this.RenderingScale || 1;
    const localX = (apply.MappedLocation.X + apply.ChildOffset.X - this.DrawingRect.Left) / scale;
    const H = this.SliderHeight, more = this.MoreHotspotSize;
    if (args.Type === "Down") {
      this.IsUserPanning = false;
      if (this.EnableRange && localX >= this.StartThumbX - more && localX <= this.StartThumbX + H + more) this.touchArea = "Start";
      else if (localX >= this.EndThumbX - more && localX <= this.EndThumbX + H + more) this.touchArea = "End";
      else this.touchArea = "Unknown";
      this.IsPressed = true;
      if (this.touchArea === "Unknown" && this.ClickOnTrailEnabled) {
        const half = H / 2;
        if (this.EnableRange && localX <= this.WidthPts / 2) { this.touchArea = "Start"; this.SetStartOffsetClamped(localX - half); }
        else { this.touchArea = "End"; this.SetEndOffsetClamped(localX - half); }
        this.RecalculateValues();
      }
      this.lastTouchX = this.touchArea === "Start" ? this.StartThumbX : this.EndThumbX;
      return this;
    }
    if (args.Type === "Panning") {
      if (this.touchArea === "Unknown") return null;
      this.IsUserPanning = true;
      const maybe = this.lastTouchX + args.Event.Distance.Total.X / scale;
      if (this.touchArea === "Start") this.SetStartOffsetClamped(maybe); else this.SetEndOffsetClamped(maybe);
      this.RecalculateValues();
      return this;
    }
    if (args.Type === "Up") { this.IsUserPanning = false; this.IsPressed = false; return null; }
    if (args.Type === "Tapped") return this.touchArea !== "Unknown" ? this : null;
    return super.ProcessGestures(args, apply);
  }

  protected override DefaultAccessibilityCanInteract(): boolean { return this.RespondsToGestures; }
  protected override DefaultAccessibilityLabel(): string | undefined { return this.EnableRange ? `${this.start} – ${this.end}` : `${this.end}`; }
}
