import type { SkiaControl } from "../core/SkiaControl";
import type { GestureEventProcessingInfo, SkiaGesturesParameters } from "../core/Gestures";
import { type Color, Colors, Thickness } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";
import { SkiaLabel } from "./SkiaLabel";
import { SkiaShape } from "./SkiaShape";
import { SkiaToggle } from "./SkiaToggle";

/** DrawnUi RadioButtons manager: one toggled button per group (GroupName, else the parent). */
const groups = new Map<string | SkiaControl, Set<SkiaRadioButton>>();

/** Mirrors DrawnUi SkiaRadioButton: circle indicator + text, exclusive inside its group; tapping an untoggled one toggles it. */
export class SkiaRadioButton extends SkiaToggle {
  static override DefaultAccessibilityRole?: string = "radio";

  private text = "";
  private groupName = "";
  private groupKey?: string | SkiaControl;
  ViewOn!: SkiaShape;
  ViewText!: SkiaLabel;

  private static readonly DefaultAccent = "#DC143C";
  private static readonly DefaultOutline = "#8E959D";

  constructor(text = "") {
    super();
    this.text = text;
    this.Type = "Absolute";
    this.UseCache = "Image";
  }

  get Text(): string { return this.text; }
  set Text(v: string) { this.text = v; this.ApplyProperties(); }
  /** Buttons sharing a GroupName are exclusive; without one, siblings under the same parent are. */
  get GroupName(): string { return this.groupName; }
  set GroupName(v: string) { this.groupName = v; this.UpdateGroup(); }

  protected override StyleDefault(name: "ColorThumbOn" | "ColorFrameOn" | "ColorThumbOff" | "ColorFrameOff"): Color | undefined {
    const table: Record<string, Partial<Record<typeof name, Color>>> = {
      Unset: { ColorThumbOff: SkiaRadioButton.DefaultOutline, ColorThumbOn: SkiaRadioButton.DefaultAccent },
      Cupertino: { ColorThumbOff: "#BFBFBF", ColorThumbOn: "#007AFF" },
      Material: { ColorThumbOff: "#757575", ColorThumbOn: "#2196F3" },
      Material3: { ColorThumbOff: "#49454F", ColorThumbOn: "#6750A4" },
      Windows: { ColorThumbOff: "#767676", ColorThumbOn: "#0078D7" },
    };
    return table[this.UsingControlStyle]?.[name];
  }

  protected override CreateDefaultContent(): void {
    if (this.MinimumHeightRequest < 0) this.MinimumHeightRequest = 24;
    const s = this.usingStyle;
    const size = s === "Unset" ? 18 : 20;
    const box = new SkiaLayout(); box.Type = "Absolute"; box.HeightRequest = size; box.LockRatio = 1; box.VerticalOptions = "Center";
    const off = new SkiaShape(); off.Type = "Circle"; off.HorizontalOptions = "Fill"; off.VerticalOptions = "Fill";
    const on = new SkiaShape(); on.Tag = "On"; on.Type = "Circle"; on.HorizontalOptions = "Fill"; on.VerticalOptions = "Fill";
    const dot = new SkiaShape(); dot.Type = "Circle"; dot.HorizontalOptions = "Fill"; dot.VerticalOptions = "Fill";
    switch (s) {
      case "Cupertino": off.StrokeWidth = 1.5; dot.Margin = new Thickness(6.5); dot.BackgroundColor = Colors.White; on.AddSubView(dot); break;
      case "Windows": off.StrokeWidth = 1; on.StrokeWidth = 5; break;
      case "Material": case "Material3": off.StrokeWidth = 2; on.StrokeWidth = 2; dot.Margin = new Thickness(5); on.AddSubView(dot); break;
      default: off.StrokeWidth = 2; on.StrokeWidth = 2; dot.Margin = new Thickness(4); on.AddSubView(dot); break;
    }
    box.AddSubView(off); box.AddSubView(on);
    const label = new SkiaLabel(); label.Tag = "Text"; label.Margin = new Thickness(s === "Unset" ? 26 : 28, 0, 0, 0); label.FontSize = 14; label.MaxLines = 2; label.TextColor = Colors.Black; label.VerticalOptions = "Center";
    label.AccessibilityRole = "presentation";
    this.ViewOn = on; this.ViewText = label;
    this.offRing = off; this.dot = dot;
    this.AddSubView(box);
    this.AddSubView(label);
    this.UpdateGroup();
  }
  private offRing?: SkiaShape;
  private dot?: SkiaShape;

  override ApplyProperties(): void {
    if (!this.contentCreated) return;
    const s = this.usingStyle;
    this.offRing!.StrokeColor = this.ColorThumbOff;
    if (s === "Cupertino") this.ViewOn.BackgroundColor = this.ColorThumbOn;
    else this.ViewOn.StrokeColor = this.ColorThumbOn;
    if (this.dot && s !== "Cupertino") this.dot.BackgroundColor = this.ColorThumbOn;
    this.ViewOn.IsVisible = this.IsToggled;
    this.ViewText.Text = this.text;
    this.Update();
  }

  protected override DefaultAccessibilityLabel(): string | undefined { return this.text || undefined; }

  // ---- group (C# RadioButtons.All) ----
  private UpdateGroup(): void {
    if (this.groupKey) groups.get(this.groupKey)?.delete(this);
    this.groupKey = this.groupName || this.Parent;
    if (!this.groupKey) return;
    let set = groups.get(this.groupKey);
    if (!set) { set = new Set(); groups.set(this.groupKey, set); }
    set.add(this);
  }

  protected override OnToggledChanged(): void {
    if (this.IsToggled) {
      if (!this.groupKey || this.groupKey !== (this.groupName || this.Parent)) this.UpdateGroup();
      for (const other of groups.get(this.groupKey!) ?? []) if (other !== this && other.IsToggled) other.IsToggled = false;
    }
    super.OnToggledChanged();
  }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Tapped") {
      if (!this.IsToggled && this.RespondsToGestures) { this.IsToggled = true; return this; }
      return null;
    }
    return super.ProcessGestures(args, apply);
  }
}
