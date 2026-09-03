import type { Path } from "canvaskit-wasm";
import type { SkiaControl } from "../core/SkiaControl";
import { type Color, Colors, type CornerRadius, SKRect, ScaledSize, SkiaShadow, type SkiaTouchAnimation, Thickness } from "../core/Types";
import { SKPoint, type GestureEventProcessingInfo, type SkiaGesturesParameters } from "../core/Gestures";
import { SkiaLayout } from "./SkiaLayout";
import { SkiaLabel } from "./SkiaLabel";
import { SkiaShape } from "./SkiaShape";
import { type PrebuiltControlStyle, ResolveControlStyle, type ResolvedControlStyle } from "../core/ControlStyle";

/**
 * Mirrors DrawnUi SkiaButton (default style): a SkiaShape frame tagged "BtnShape" + a centered SkiaLabel
 * tagged "BtnText". Consumes Down/Up/Tapped inside its rect; releases the pressed state when a pan exceeds
 * PanThreshold. Press feedback = ApplyEffect (ripple), no darkening.
 */
export class SkiaButton extends SkiaLayout {
  static PanThreshold = 5;

  Text = "";
  /** Look of the default content (C# SkiaButton.ControlStyle): background, corners, font, minimum size when left unset. */
  ControlStyle: PrebuiltControlStyle = "Unset";
  TextColor: Color = Colors.White;
  FontSize = 15;
  FontFamily = "";
  /** Glyph fallback chain for the label (React extension, same semantics as SkiaLabel.FontFamilyFallback). */
  FontFamilyFallback = "";
  /** Frame corner radius, points (DrawnUi default look uses 8). */
  CornerRadius: CornerRadius | number = 8;
  StrokeColor: Color = Colors.Transparent;
  StrokeWidth = 0;
  IsPressed = false;
  IsDisabled = false;
  LockPanning = false;
  /** Touch feedback played on Down (DrawnUi SkiaButton.ApplyEffect). */
  ApplyEffect: SkiaTouchAnimation = "None";
  TotalDown = 0;
  TotalTapped = 0;
  Down?: (sender: SkiaButton, args: SkiaGesturesParameters) => void;
  Up?: (sender: SkiaButton, args: SkiaGesturesParameters) => void;

  private readonly frame = new SkiaShape();
  private readonly label = new SkiaLabel();
  private lastDownPts = SKPoint.Empty;
  private hadDown = false;

  constructor() {
    super();
    this.Type = "Absolute";
    this.Padding = new Thickness(16, 10);
    this.frame.Tag = "BtnShape";
    this.frame.HorizontalOptions = "Fill";
    this.frame.VerticalOptions = "Fill";
    this.label.Tag = "BtnText";
    this.label.AccessibilityRole = "presentation"; // the button is the accessible node, not its inner label (C# MainLabel.AccessibilityRole = null)
    this.label.HorizontalOptions = "Center";
    this.label.VerticalOptions = "Center";
    this.AddSubView(this.frame);
    this.AddSubView(this.label);
  }

  /** Set to `Aria.RoleButton` to expose every button (React extension). */
  static override DefaultAccessibilityRole?: string;
  protected override DefaultAccessibilityLabel(): string | undefined { return this.Text || undefined; }
  protected override DefaultAccessibilityCanInteract(): boolean { return !this.IsDisabled; }

  /** The button's own BackgroundColor/CornerRadius/Stroke are the frame's; the button itself paints nothing. */
  protected override PaintBackground(): void {}

  get UsingControlStyle(): ResolvedControlStyle { return ResolveControlStyle(this.ControlStyle); }

  /** C# Create*StyleContent defaults: accent, corner radius, font size/weight, minimum content size. */
  private static Look(s: ResolvedControlStyle): { bg: Color; radius: number; font: number; weight: number; minH: number; shadow?: SkiaShadow } {
    switch (s) {
      case "Cupertino": return { bg: "#007AFF", radius: 8, font: 17, weight: 600, minH: 36, shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 2, Opacity: 0.2, Color: Colors.Black }) };
      case "Material": return { bg: "#2196F3", radius: 4, font: 14, weight: 0, minH: 40, shadow: new SkiaShadow({ X: 0, Y: 2, Blur: 4, Opacity: 0.3, Color: Colors.Black }) };
      case "Material3": return { bg: "#6750A4", radius: 20, font: 14, weight: 0, minH: 40 };
      case "Windows": return { bg: "#0078D7", radius: 4, font: 15, weight: 500, minH: 32, shadow: new SkiaShadow({ X: 0, Y: 1, Blur: 1, Opacity: 0.2, Color: Colors.Black }) };
      default: return { bg: "#DC143C", radius: 8, font: 15, weight: 0, minH: 41 };
    }
  }

  protected override MeasureAbsolute(w: number, h: number, scale: number): ScaledSize {
    const look = SkiaButton.Look(this.UsingControlStyle);
    this.frame.BackgroundColor = this.BackgroundColor ?? look.bg;
    this.frame.CornerRadius = this.CornerRadius === 8 ? look.radius : this.CornerRadius; // C#: 8 = "not customized"
    this.frame.StrokeColor = this.StrokeColor;
    this.frame.StrokeWidth = this.StrokeWidth;
    if ((this.frame.Shadows[0] as SkiaShadow | undefined) !== look.shadow) this.frame.Shadows = look.shadow ? [look.shadow] : [];
    if (this.MinimumHeightRequest < 0 && this.UsingControlStyle !== "Unset") this.MinimumHeightRequest = look.minH;
    this.label.Text = this.Text;
    this.label.TextColor = this.TextColor;
    this.label.FontSize = this.FontSize === 15 ? look.font : this.FontSize;
    this.label.FontWeight = look.weight;
    this.label.FontFamily = this.FontFamily;
    this.label.FontFamilyFallback = this.FontFamilyFallback;
    // Size comes from the label + Padding; the Fill frame follows whatever the button is arranged to.
    const px = this.Padding.HorizontalThickness * scale, py = this.Padding.VerticalThickness * scale;
    const l = this.label.Measure(isFinite(w) ? w - px : w, isFinite(h) ? h - py : h, scale);
    return ScaledSize.FromPixels(l.Pixels.Width + px, l.Pixels.Height + py, scale);
  }

  /** The frame sits under the padding box: arrange children in the full rect, the label centered inside padding. */
  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const r = this.DrawingRect;
    this.frame.Arrange(r, -1, -1, scale);
    const p = this.Padding;
    this.label.Arrange(new SKRect(r.Left + p.Left * scale, r.Top + p.Top * scale, r.Right - p.Right * scale, r.Bottom - p.Bottom * scale), -1, -1, scale);
  }

  /** Ripple clipped to the rounded frame. */
  override CreateClip(): Path { return this.frame.CreateClip(); }

  /** DrawnUi SkiaButton.OnDown: press feedback; true = consume Down. */
  protected OnDown(_args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): boolean {
    if (this.ApplyEffect === "Ripple") {
      const pts = this.GetOffsetInsideControlInPoints(apply.MappedLocation, apply.ChildOffset);
      this.PlayRippleAnimation(this.TouchEffectColor, pts.X, pts.Y);
    }
    return true;
  }

  private SetUp(args: SkiaGesturesParameters): void {
    this.IsPressed = false;
    this.hadDown = false;
    this.Up?.(this, args);
    this.Repaint();
  }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (this.IsDisabled) return null;
    const point = args.Event.Location;

    if (args.Type === "Down") {
      this.IsPressed = true;
      this.lastDownPts = point;
      this.hadDown = true;
      this.TotalDown++;
      this.Down?.(this, args);
      this.Repaint();
      return this.OnDown(args, apply) ? this : null;
    }
    if (args.Type === "Panning") {
      if (this.LockPanning) return this;
      const t = SkiaButton.PanThreshold * this.RenderingScale;
      if (Math.abs(point.X - this.lastDownPts.X) > t || Math.abs(point.Y - this.lastDownPts.Y) > t) {
        if (this.hadDown) this.SetUp(args);
        this.hadDown = false;
        return null;
      }
    } else if (args.Type === "Up") {
      this.SetUp(args);
    } else if (args.Type === "Tapped") {
      this.TotalTapped++;
      return this.SendTapped(args, apply) ? this : null;
    }
    return this.hadDown ? this : null;
  }
}
