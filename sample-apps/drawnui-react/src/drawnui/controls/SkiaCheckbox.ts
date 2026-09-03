import { type Color, Colors, Thickness } from "../core/Types";
import type { SkiaControl } from "../core/SkiaControl";
import { SkiaShape } from "./SkiaShape";
import { SkiaSvg } from "./SkiaSvg";
import { SkiaToggle } from "./SkiaToggle";

const SvgCupertinoCheck = '<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none"><path d="M4 12.6111L8.92308 17.5L20 6.5" stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
const SvgMaterialCheck = '<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none"><path d="M5 13L9 17L19 7" stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
const SvgWindowsCheck = '<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none"><path d="M4 11.6L10 17.6L20 7.6" stroke="#000000" stroke-width="2.5" stroke-linecap="square" stroke-linejoin="round"/></svg>';

/** Mirrors DrawnUi SkiaCheckbox: `FrameOff` outline, `FrameOn` filled frame with a `ViewCheckOn` mark, per-style looks. */
export class SkiaCheckbox extends SkiaToggle {
  static override DefaultAccessibilityRole?: string = "checkbox";

  FrameOff!: SkiaShape;
  FrameOn!: SkiaShape;
  ViewCheckOn!: SkiaControl;
  private colorCheckOn?: Color;

  private static readonly DefaultAccent = "#DC143C";
  private static readonly DefaultOutline = "#8E959D";

  /** Color of the check mark (inner square for the default look). */
  get ColorCheckOn(): Color { return this.colorCheckOn ?? (this.UsingControlStyle === "Unset" ? SkiaCheckbox.DefaultAccent : Colors.White); }
  set ColorCheckOn(v: Color) { this.colorCheckOn = v; this.ApplyProperties(); }

  protected override StyleDefault(name: "ColorThumbOn" | "ColorFrameOn" | "ColorThumbOff" | "ColorFrameOff"): Color | undefined {
    const table: Record<string, Partial<Record<typeof name, Color>>> = {
      Unset: { ColorFrameOff: SkiaCheckbox.DefaultOutline, ColorFrameOn: SkiaCheckbox.DefaultAccent },
      Cupertino: { ColorFrameOff: "#BFBFBF", ColorFrameOn: "#007AFF" },
      Material: { ColorFrameOff: "#757575", ColorFrameOn: "#2196F3" },
      Material3: { ColorFrameOff: "#49454F", ColorFrameOn: "#6750A4" },
      Windows: { ColorFrameOff: "#999999", ColorFrameOn: "#0078D7" },
    };
    return table[this.UsingControlStyle]?.[name];
  }

  private SetContentSize(w: number, h: number): void {
    if (this.WidthRequest < 0) this.WidthRequest = w;
    if (this.HeightRequest < 0) this.HeightRequest = h;
  }

  protected override CreateDefaultContent(): void {
    const off = new SkiaShape(); off.Tag = "FrameOff"; off.Type = "Rectangle"; off.HorizontalOptions = "Fill"; off.VerticalOptions = "Fill";
    const on = new SkiaShape(); on.Tag = "FrameOn"; on.Type = "Rectangle"; on.HorizontalOptions = "Fill"; on.VerticalOptions = "Fill";
    let check: SkiaControl;
    const svg = (markup: string, margin: number) => { const v = new SkiaSvg(); v.Tag = "ViewCheckOn"; v.SvgString = markup; v.TintColor = Colors.White; v.Margin = new Thickness(margin); v.HorizontalOptions = "Fill"; v.VerticalOptions = "Fill"; return v; };
    switch (this.usingStyle) {
      case "Cupertino": this.SetContentSize(22, 22); off.StrokeWidth = 1.5; off.CornerRadius = 4; on.CornerRadius = 4; check = svg(SvgCupertinoCheck, 2); break;
      case "Material": this.SetContentSize(24, 24); off.StrokeWidth = 2; off.CornerRadius = 2; on.CornerRadius = 2; check = svg(SvgMaterialCheck, 2); break;
      case "Material3": this.SetContentSize(18, 18); off.StrokeWidth = 2; off.CornerRadius = 2; on.CornerRadius = 2; check = svg(SvgMaterialCheck, 2); break;
      case "Windows": this.SetContentSize(20, 20); off.StrokeWidth = 1; off.CornerRadius = 0; on.CornerRadius = 0; check = svg(SvgWindowsCheck, 1); break;
      default: {
        this.SetContentSize(22, 22); off.StrokeWidth = 1; on.StrokeWidth = 1;
        const sq = new SkiaShape(); sq.Tag = "ViewCheckOn"; sq.Type = "Rectangle"; sq.Margin = new Thickness(3); sq.LockRatio = 1; sq.HorizontalOptions = "Fill"; sq.VerticalOptions = "Fill"; sq.UseCache = "Operations";
        check = sq;
      }
    }
    on.AddSubView(check);
    this.FrameOff = off; this.FrameOn = on; this.ViewCheckOn = check;
    this.AddSubView(off);
    this.AddSubView(on);
  }

  override ApplyProperties(): void {
    if (!this.contentCreated) return;
    const on = this.IsToggled;
    this.FrameOff.StrokeColor = this.ColorFrameOff;
    if (this.usingStyle === "Unset") { this.FrameOn.StrokeColor = this.ColorFrameOn; (this.ViewCheckOn as SkiaShape).BackgroundColor = this.ColorCheckOn; }
    else { this.FrameOn.BackgroundColor = this.ColorFrameOn; (this.ViewCheckOn as SkiaSvg).TintColor = this.ColorCheckOn; }
    this.FrameOn.IsVisible = on;
    this.ViewCheckOn.IsVisible = on;
    this.FrameOff.IsVisible = !on;
    this.Update();
  }
}
