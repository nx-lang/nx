import type { ControlTappedEventArgs } from "../core/Gestures";
import { type Color, Colors, type SKRect } from "../core/Types";
import type { SkiaLabel } from "./SkiaLabel";

/**
 * Mirrors DrawnUi TextSpan: a styled fragment inside a SkiaLabel (`<SkiaLabel><TextSpan .../></SkiaLabel>`).
 * Unset TextColor / FontSize / FontFamily inherit from the label (C# HasSetColor / HasSetSize / HasSetFont).
 * Not a SkiaControl: it has no box of its own, the label lays it out and keeps its hit `Rects`.
 */
export class TextSpan {
  /** The label hosting this span (C# ParentControl). */
  Parent?: SkiaLabel;
  /** Set by the reconciler when React hides the element. */
  IsVisible = true;
  /** Rectangles of the drawn fragments, pixels relative to the label's top-left, refreshed by every paint. */
  readonly Rects: SKRect[] = [];

  Tapped?: (span: TextSpan, e: ControlTappedEventArgs) => void;
  /** Free payload (SkiaRichLabel stores the link url here). */
  Tag?: string;
  /** Listen to taps even without a Tapped handler. */
  ForceCaptureInput = false;

  private text = "";
  private textColor?: Color;
  private fontSize?: number;
  private fontFamily?: string;
  private fontWeight = 0;
  private isBold = false;
  private isItalic = false;
  private underline = false;
  private underlineWidth = -1;
  private strikeout = false;
  private strikeoutWidth = 1;
  private strikeoutColor: Color = Colors.Red;
  private backgroundColor?: Color;

  private Set<K extends keyof this>(key: K, v: this[K]): void { if (this[key] !== v) { this[key] = v; this.Update(); } }

  get Text(): string { return this.text; }
  set Text(v: string) { this.Set("text" as keyof this, v as this[keyof this]); }
  get TextColor(): Color | undefined { return this.textColor; }
  set TextColor(v: Color | undefined) { this.Set("textColor" as keyof this, v as this[keyof this]); }
  get FontSize(): number | undefined { return this.fontSize; }
  set FontSize(v: number | undefined) { this.Set("fontSize" as keyof this, v as this[keyof this]); }
  get FontFamily(): string | undefined { return this.fontFamily; }
  set FontFamily(v: string | undefined) { this.Set("fontFamily" as keyof this, v as this[keyof this]); }
  get FontWeight(): number { return this.fontWeight; }
  set FontWeight(v: number) { this.Set("fontWeight" as keyof this, v as this[keyof this]); }
  get IsBold(): boolean { return this.isBold; }
  set IsBold(v: boolean) { this.Set("isBold" as keyof this, v as this[keyof this]); }
  get IsItalic(): boolean { return this.isItalic; }
  set IsItalic(v: boolean) { this.Set("isItalic" as keyof this, v as this[keyof this]); }
  get Underline(): boolean { return this.underline; }
  set Underline(v: boolean) { this.Set("underline" as keyof this, v as this[keyof this]); }
  /** Points; negative = that many pixels (C# default -1 = 1px). */
  get UnderlineWidth(): number { return this.underlineWidth; }
  set UnderlineWidth(v: number) { this.Set("underlineWidth" as keyof this, v as this[keyof this]); }
  get Strikeout(): boolean { return this.strikeout; }
  set Strikeout(v: boolean) { this.Set("strikeout" as keyof this, v as this[keyof this]); }
  /** Points. */
  get StrikeoutWidth(): number { return this.strikeoutWidth; }
  set StrikeoutWidth(v: number) { this.Set("strikeoutWidth" as keyof this, v as this[keyof this]); }
  get StrikeoutColor(): Color { return this.strikeoutColor; }
  set StrikeoutColor(v: Color) { this.Set("strikeoutColor" as keyof this, v as this[keyof this]); }
  get BackgroundColor(): Color | undefined { return this.backgroundColor; }
  set BackgroundColor(v: Color | undefined) { this.Set("backgroundColor" as keyof this, v as this[keyof this]); }

  get HasTapHandler(): boolean { return !!this.Tapped || this.ForceCaptureInput; }
  get HasDecorations(): boolean { return (this.underline && this.underlineWidth !== 0) || this.strikeout; }

  /** Invalidates the hosting label (a span has no rendering of its own). */
  Update(): void { this.Parent?.Update(); }

  /** x/y in pixels relative to the label's top-left. */
  HitIsInside(x: number, y: number): boolean {
    for (const r of this.Rects) if (x >= r.Left && x < r.Right && y >= r.Top && y < r.Bottom) return true;
    return false;
  }

  FireTap(e: ControlTappedEventArgs): void { this.Tapped?.(this, e); }
}
