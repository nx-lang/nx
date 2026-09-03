import { type Color, Colors, SkiaShadow, Thickness } from "../core/Types";
import { Easing } from "../core/Easing";
import { SkiaShape } from "./SkiaShape";
import { SkiaToggle } from "./SkiaToggle";

/**
 * Mirrors DrawnUi SkiaSwitch: a `Frame` SkiaShape + a `Thumb` circle translated to the on/off end, with the
 * Default / Cupertino / Material / Material3 / Windows looks and geometry of the C# style builders. Shadows omitted.
 */
export class SkiaSwitch extends SkiaToggle {
  static AnimationSpeed = 200;
  static override DefaultAccessibilityRole?: string = "switch";

  Track!: SkiaShape;
  Thumb!: SkiaShape;

  private static readonly DefaultAccent = "#DC143C";   // Crimson
  private static readonly DefaultTrack = "#D7DBE0";
  private static readonly MaterialOutline = "#79747E";

  protected override StyleDefault(name: "ColorThumbOn" | "ColorFrameOn" | "ColorThumbOff" | "ColorFrameOff"): Color | undefined {
    const s = this.UsingControlStyle;
    const table: Record<string, Partial<Record<typeof name, Color>>> = {
      Unset: { ColorFrameOn: SkiaSwitch.DefaultAccent, ColorFrameOff: SkiaSwitch.DefaultTrack },
      Cupertino: { ColorFrameOff: "#E5E5E5", ColorFrameOn: "#30D158", ColorThumbOff: Colors.White, ColorThumbOn: Colors.White },
      Material: { ColorFrameOff: "#9E9E9E", ColorFrameOn: "#2196F3", ColorThumbOff: Colors.White, ColorThumbOn: Colors.White },
      Material3: { ColorFrameOff: "#E6E0E9", ColorFrameOn: "#6750A4", ColorThumbOff: SkiaSwitch.MaterialOutline, ColorThumbOn: Colors.White },
      Windows: { ColorFrameOff: "#767676", ColorFrameOn: "#0078D7", ColorThumbOff: "#767676", ColorThumbOn: Colors.White },
    };
    return table[s]?.[name];
  }

  private SetContentSize(w: number, h: number): void {
    if (this.WidthRequest < 0) this.WidthRequest = w;
    if (this.HeightRequest < 0) this.HeightRequest = h;
  }

  protected override CreateDefaultContent(): void {
    const frame = new SkiaShape(); frame.Tag = "Frame"; frame.Type = "Rectangle"; frame.HorizontalOptions = "Fill"; frame.VerticalOptions = "Fill";
    const thumb = new SkiaShape(); thumb.Tag = "Thumb"; thumb.Type = "Circle"; thumb.HorizontalOptions = "Start"; thumb.VerticalOptions = "Fill"; thumb.LockRatio = -1;
    thumb.UseCache = "Operations";
    switch (this.usingStyle) {
      case "Cupertino": this.SetContentSize(51, 31); frame.CornerRadius = 100; thumb.Margin = new Thickness(2); thumb.Shadows = [new SkiaShadow({ X: 0, Y: 3, Blur: 3, Opacity: 0.1, Color: Colors.Black })]; break;
      case "Material": this.SetContentSize(46, 28); frame.CornerRadius = 7; frame.HeightRequest = 15; frame.VerticalOptions = "Center"; thumb.Margin = Thickness.Zero; thumb.Shadows = [new SkiaShadow({ X: 1, Y: 1, Blur: 3, Opacity: 0.1, Color: Colors.Black })]; break;
      case "Material3": this.SetContentSize(52, 32); frame.CornerRadius = 16; frame.StrokeWidth = 2; frame.StrokeColor = SkiaSwitch.MaterialOutline; thumb.Margin = new Thickness(4); thumb.WidthRequest = 24; thumb.LockRatio = 1; thumb.VerticalOptions = "Center"; thumb.Shadows = [new SkiaShadow({ X: 1, Y: 1, Blur: 3, Opacity: 0.1, Color: Colors.Black })]; break;
      case "Windows": this.SetContentSize(48, 22); frame.CornerRadius = 12; frame.StrokeWidth = 2.5; frame.StrokeColor = "#767676"; thumb.Margin = new Thickness(5.5); break;
      default: this.SetContentSize(46, 28); frame.CornerRadius = 20; thumb.Margin = new Thickness(2); break;
    }
    this.Track = frame; this.Thumb = thumb;
    this.AddSubView(frame);
    this.AddSubView(thumb);
  }

  /** C# GetThumbPosForOn: track width minus thumb width and margins, in points. */
  private ThumbPosForOn(): number {
    const scale = this.RenderingScale || 1;
    const trackW = this.Track.DrawingRect.Width / scale + this.Track.Margin.HorizontalThickness;
    const thumbW = (this.Thumb.DrawingRect.Width > 0 ? this.Thumb.DrawingRect.Width / scale : this.Thumb.WidthRequest > 0 ? this.Thumb.WidthRequest : this.HeightRequest - this.Thumb.Margin.VerticalThickness);
    return trackW - thumbW - this.Thumb.Margin.HorizontalThickness;
  }

  override ApplyProperties(): void {
    if (!this.contentCreated) return;
    const on = this.IsToggled;
    const s = this.usingStyle;
    this.Thumb.BackgroundColor = on ? this.ColorThumbOn : this.ColorThumbOff;
    if (on) {
      this.Track.BackgroundColor = this.ColorFrameOn;
      if (s === "Windows" || s === "Material3") this.Track.StrokeColor = this.ColorFrameOn;
    } else if (s === "Windows") {
      this.Track.BackgroundColor = Colors.Transparent; this.Track.StrokeColor = this.ColorFrameOff;
    } else if (s === "Material3") {
      this.Track.BackgroundColor = this.ColorFrameOff; this.Track.StrokeColor = SkiaSwitch.MaterialOutline;
    } else {
      this.Track.BackgroundColor = this.ColorFrameOff;
    }
    this.Track.Update();
    this.Thumb.Update();
    this.PositionThumb();
  }

  private animating = false;

  /** Moves the thumb to its resting position; animated once laid out, snapped otherwise (C# CanAnimate = LayoutReady && IsAnimated). */
  private PositionThumb(): void {
    if (this.Thumb.DrawingRect.Width <= 0) return; // not laid out yet: OnLayoutChanged snaps it
    const target = this.IsToggled ? this.ThumbPosForOn() : 0;
    if (this.IsAnimated && Math.abs(this.Thumb.TranslationX - target) > 0.5) {
      this.animating = true;
      void this.Thumb.TranslateToAsync(target, 0, SkiaSwitch.AnimationSpeed, Easing.CubicOut).catch(() => {}).finally(() => { this.animating = false; });
    } else { this.Thumb.TranslationX = target; this.Thumb.RepaintComposition(); }
  }

  protected override OnLayoutChanged(): void {
    super.OnLayoutChanged();
    if (!this.contentCreated || this.animating) return;
    const target = this.IsToggled ? this.ThumbPosForOn() : 0;
    if (Math.abs(this.Thumb.TranslationX - target) > 0.5) this.Thumb.TranslationX = target;
  }
}
