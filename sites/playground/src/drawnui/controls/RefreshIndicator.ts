import type { ScrollOrientation } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";

/** C# IRefreshIndicator: the pull-to-refresh view a SkiaScroll drives while overscrolling. */
export interface IRefreshIndicator {
  /** ratio = overscroll / RefreshShowDistance (not clamped by the caller), ptsScrollOffset = current offset in points. */
  SetDragRatio(ratio: number, ptsScrollOffset: number, ptsLimit: number, ptsTrigger: number): void;
  IsVisible: boolean;
  IsRunning: boolean;
}

/**
 * Mirrors DrawnUi RefreshIndicator: a layout parked above (or left of) the viewport that `SetDragRatio` pulls into
 * view with the C# position curve and fades in; `IsRunning` becomes true once fully shown (the refresh is in
 * progress) — override `OnIsRunningChanged` to start / stop your animation. Put the visuals inside as children.
 */
export class RefreshIndicator extends SkiaLayout implements IRefreshIndicator {
  private orientation: ScrollOrientation = "Vertical";
  get Orientation(): ScrollOrientation { return this.orientation; }
  set Orientation(v: ScrollOrientation) {
    if (this.orientation === v) return;
    if (v === "Both") throw new Error("DrawnUi: RefreshIndicator supports Vertical or Horizontal");
    const prev = this.orientation; this.orientation = v; this.UpdateOrientation(prev);
  }
  private isRunning = false;
  /** Read-only: the indicator is fully shown (refresh in progress). */
  get IsRunning(): boolean { return this.isRunning; }
  set IsRunning(v: boolean) { if (this.isRunning !== v) { this.isRunning = v; this.OnIsRunningChanged(v); } }
  VisibleRatio = 0;

  constructor() {
    super();
    this.Type = "Absolute";
    this.InputTransparent = true;
    this.HorizontalOptions = "Fill";
    this.VerticalOptions = "Start";
    this.IsVisible = false;
  }

  /** Only swaps alignments still at the previous orientation's defaults (C# UpdateOrientation). */
  protected UpdateOrientation(previous: ScrollOrientation): void {
    const [oldH, oldV] = previous === "Horizontal" ? ["Start", "Fill"] : ["Fill", "Start"];
    const [newH, newV] = this.orientation === "Horizontal" ? ["Start", "Fill"] : ["Fill", "Start"];
    if (this.HorizontalOptions === oldH) this.HorizontalOptions = newH as typeof this.HorizontalOptions;
    if (this.VerticalOptions === oldV) this.VerticalOptions = newV as typeof this.VerticalOptions;
    this.InvalidateMeasure();
  }

  /**
   * C# SetDragRatio: 0..1 (clamped here) -> position + opacity; running once fully shown. The view slides in with the
   * pull: its far edge follows the overscroll until ptsLimit (RefreshShowDistance), where it stays while refreshing.
   */
  SetDragRatio(ratio: number, ptsScrollOffset: number, ptsLimit: number, _ptsTrigger: number): void {
    ratio = Math.max(0, Math.min(1, ratio));
    this.VisibleRatio = ratio;
    let opacity = ratio;
    const scale = this.RenderingScale || 1;
    const size = (this.orientation === "Horizontal" ? this.MeasuredSize.Pixels.Width : this.MeasuredSize.Pixels.Height) / scale;
    if (ratio <= 0) {
      // parked just outside the viewport (C#: dividing by 0 poisoned the transform, so it is placed explicitly)
      if (this.orientation === "Horizontal") this.TranslationX = -size; else this.TranslationY = -size;
      opacity = 0;
    } else if (size > 0) {
      const shown = Math.min(Math.max(0, ptsScrollOffset), ptsLimit);
      const pos = shown - size + Math.max(0, (ptsLimit - size) / 2); // centered in the gap when the gap is taller than the view
      if (this.orientation === "Horizontal") this.TranslationX = pos; else this.TranslationY = pos;
      opacity = ratio;
    }
    this.IsRunning = opacity >= 1;
    if (this.IsRunning) opacity = 1;
    this.Opacity = opacity;
    this.SetAnimationState(opacity !== 0);
    this.RepaintComposition();
  }

  /** C# SetAnimationState: shown while it has any opacity. */
  SetAnimationState(running: boolean): void { this.IsVisible = running; }

  protected OnIsRunningChanged(_value: boolean): void {}
}
