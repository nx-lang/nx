import type { SkiaControl } from "../core/SkiaControl";
import type { GestureEventProcessingInfo, SkiaGesturesParameters } from "../core/Gestures";
import { type PrebuiltControlStyle, ResolveControlStyle, type ResolvedControlStyle } from "../core/ControlStyle";
import { type Color, Colors, type ScaledSize } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SkiaToggle: base of SkiaSwitch / SkiaCheckbox / SkiaRadioButton. Tapped flips IsToggled,
 * `Toggled` reports user changes, `ControlStyle` selects the look the default content is built with (once, at first
 * measure, like C# CreateDefaultContent). Colors left unset take the style defaults (C# SetStyleDefault).
 */
export abstract class SkiaToggle extends SkiaLayout {
  ControlStyle: PrebuiltControlStyle = "Unset";
  /** Fires on every IsToggled change; C# fires only on user/external changes, not on DefaultValue application. */
  Toggled?: (sender: SkiaToggle, value: boolean) => void;
  DefaultValue = false;
  IsAnimated = true;
  RespondsToGestures = true;

  private isToggled = false;
  private colorThumbOn?: Color;
  private colorFrameOn?: Color;
  private colorThumbOff?: Color;
  private colorFrameOff?: Color;
  protected contentCreated = false;
  protected usingStyle: ResolvedControlStyle = "Unset";

  constructor() {
    super();
    this.Type = "Absolute";
  }

  get UsingControlStyle(): ResolvedControlStyle { return this.contentCreated ? this.usingStyle : ResolveControlStyle(this.ControlStyle); }

  get IsToggled(): boolean { return this.isToggled; }
  set IsToggled(v: boolean) {
    if (this.isToggled === v) return;
    this.isToggled = v;
    this.OnToggledChanged();
  }

  // C# defaults: thumb White / frame Red (on), thumb White / frame DarkGray (off); styles override the unset ones
  get ColorThumbOn(): Color { return this.colorThumbOn ?? this.StyleDefault("ColorThumbOn") ?? Colors.White; }
  set ColorThumbOn(v: Color) { this.colorThumbOn = v; this.ApplyProperties(); }
  get ColorFrameOn(): Color { return this.colorFrameOn ?? this.StyleDefault("ColorFrameOn") ?? Colors.Red; }
  set ColorFrameOn(v: Color) { this.colorFrameOn = v; this.ApplyProperties(); }
  get ColorThumbOff(): Color { return this.colorThumbOff ?? this.StyleDefault("ColorThumbOff") ?? Colors.White; }
  set ColorThumbOff(v: Color) { this.colorThumbOff = v; this.ApplyProperties(); }
  get ColorFrameOff(): Color { return this.colorFrameOff ?? this.StyleDefault("ColorFrameOff") ?? Colors.DarkGray; }
  set ColorFrameOff(v: Color) { this.colorFrameOff = v; this.ApplyProperties(); }

  /** Per-style color defaults (C# SetStyleDefault); undefined = the SkiaToggle default. */
  protected StyleDefault(_name: "ColorThumbOn" | "ColorFrameOn" | "ColorThumbOff" | "ColorFrameOff"): Color | undefined { return undefined; }

  protected OnToggledChanged(): void {
    this.ApplyProperties();
    this.Toggled?.(this, this.isToggled);
    this.NotifyAccessibility();
  }

  /** Builds the look once (C# CreateDefaultContent) — subclasses add their views here. */
  protected abstract CreateDefaultContent(): void;
  /** Pushes IsToggled + colors into the views (C# ApplyProperties). */
  abstract ApplyProperties(): void;

  /** Content (and the size requests the style sets) must exist before Measure reads WidthRequest/HeightRequest. */
  override Measure(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    if (!this.contentCreated) {
      this.contentCreated = true;
      this.usingStyle = ResolveControlStyle(this.ControlStyle);
      this.CreateDefaultContent();
      this.ApplyProperties();
    }
    return super.Measure(widthConstraint, heightConstraint, scale);
  }

  protected override DefaultAccessibilityCanInteract(): boolean { return this.RespondsToGestures; }
  override get AccessibilityIsPressed(): boolean | undefined { return this.isToggled; }
  override set AccessibilityIsPressed(_v: boolean | undefined) { /* derived from IsToggled */ }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Tapped" && this.RespondsToGestures) { this.IsToggled = !this.IsToggled; return this; }
    return super.ProcessGestures(args, apply);
  }
}
