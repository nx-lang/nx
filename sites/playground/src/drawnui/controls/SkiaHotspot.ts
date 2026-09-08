import { SkiaControl } from "../core/SkiaControl";
import type { GestureEventProcessingInfo, SkiaGesturesParameters } from "../core/Gestures";

/** Mirrors DrawnUi SkiaHotspot: invisible Fill/Fill tap area. Consumes only Tapped (and Panning when LockPanning). */
export class SkiaHotspot extends SkiaControl {
  LockPanning = false;
  TouchDown = false;
  TotalDown = 0;
  TotalTapped = 0;
  Down?: (sender: SkiaHotspot, args: SkiaGesturesParameters) => void;
  Up?: (sender: SkiaHotspot, args: SkiaGesturesParameters) => void;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.VerticalOptions = "Fill";
  }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Down") {
      this.TotalDown++;
      this.TouchDown = true;
      this.Down?.(this, args);
    } else if (args.Type === "Up") {
      this.TouchDown = false;
      this.Up?.(this, args);
    } else if (args.Type === "Tapped") {
      const consumed = this.SendTapped(args, apply);
      this.TotalTapped++;
      return consumed ? this : null;
    } else if (args.Type === "Panning") {
      if (this.LockPanning) return this;
    }
    return null;
  }
}
