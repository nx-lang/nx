// Mirrors AppoMobi.Gestures + DrawnUi.Features.Gestures types. Locations are PIXELS (like DrawnUi Event.Location).

import type { SkiaControl } from "./SkiaControl";

export class SKPoint {
  constructor(public X = 0, public Y = 0) {}
  static readonly Empty = new SKPoint();
  Add(p: SKPoint) { return new SKPoint(this.X + p.X, this.Y + p.Y); }
  Subtract(p: SKPoint) { return new SKPoint(this.X - p.X, this.Y - p.Y); }
}

/** Raw platform action (subset of TouchActionType). */
export type TouchActionType = "Pressed" | "Moved" | "Released" | "Cancelled" | "Wheel" | "Pointer";

/** Recognized gesture (TouchActionResult). LongPressing/Wheel/Pointer/Touch declared for parity, not produced yet. */
export type TouchActionResult = "Touch" | "Down" | "Up" | "Tapped" | "LongPressing" | "Panning" | "Wheel" | "Pointer";

export type GesturesMode = "Disabled" | "Enabled" | "Lock";

export type LockTouch = "Disabled" | "Enabled" | "PassNone" | "PassTap" | "PassTapAndLongPress";

/** TouchActionEventArgs.DistanceInfo. Velocity is pixels per second. */
export class DistanceInfo {
  Start = SKPoint.Empty;
  End = SKPoint.Empty;
  Delta = SKPoint.Empty;
  Total = SKPoint.Empty;
  Velocity = SKPoint.Empty;
}

/** AppoMobi.Gestures TouchActionEventArgs. */
export class TouchActionEventArgs {
  Id = 0;
  Type: TouchActionType = "Pressed";
  /** Pixels. */
  Location = SKPoint.Empty;
  StartingLocation = SKPoint.Empty;
  Distance = new DistanceInfo();
  NumberOfTouches = 1;
  IsInContact = false;
  Scale = 1;
  Timestamp = performance.now();
  /** ms since the previous event of the same pointer. */
  DeltaTimeMs = 0;
  /** Mouse wheel: Delta > 0 = wheel down (browser deltaY sign). */
  Wheel = { Delta: 0 };

  /** Same as the .NET helper: derives Start/End/Delta/Total from the previous event of the same pointer. */
  static FillDistanceInfo(current: TouchActionEventArgs, previous: TouchActionEventArgs | undefined): void {
    if (!previous) { current.Distance = new DistanceInfo(); return; }
    current.StartingLocation = previous.StartingLocation;
    current.IsInContact = previous.IsInContact;
    current.DeltaTimeMs = current.Timestamp - previous.Timestamp;
    const d = new DistanceInfo();
    d.Start = previous.Location;
    const released = current.Type === "Released" || current.Type === "Cancelled";
    d.End = released ? previous.Location : current.Location;
    d.Delta = released ? SKPoint.Empty : current.Location.Subtract(previous.Location);
    d.Total = previous.Distance.Total.Add(d.Delta);
    const secs = current.DeltaTimeMs / 1000;
    d.Velocity = secs > 0 && !released ? new SKPoint(d.Delta.X / secs, d.Delta.Y / secs) : previous.Distance.Velocity;
    current.Distance = d;
  }
}

/** DrawnUi SkiaGesturesParameters: recognized gesture + its raw event. */
export class SkiaGesturesParameters {
  Type: TouchActionResult = "Touch";
  Event = new TouchActionEventArgs();
  ArrivedTimeNanos = 0;

  static Create(action: TouchActionResult, args: TouchActionEventArgs): SkiaGesturesParameters {
    const p = new SkiaGesturesParameters();
    p.Type = action;
    p.Event = args;
    p.ArrivedTimeNanos = Math.round(performance.now() * 1e6);
    return p;
  }
}

/** DrawnUi GestureEventProcessingInfo. */
export class GestureEventProcessingInfo {
  constructor(
    public MappedLocation = SKPoint.Empty,
    public ChildOffset = SKPoint.Empty,
    public ChildOffsetDirect = SKPoint.Empty,
    public AlreadyConsumed: SkiaControl | null = null,
  ) {}
  static readonly Empty = new GestureEventProcessingInfo();
}

/** DrawnUi SkiaGesturesInfo: payload of ConsumeGestures, set Consumed=true to stop propagation. */
export class SkiaGesturesInfo {
  Consumed = false;
  constructor(public Args: SkiaGesturesParameters, public Info: GestureEventProcessingInfo) {}
}

/** DrawnUi ControlTappedEventArgs. */
export class ControlTappedEventArgs {
  constructor(
    public Control: SkiaControl,
    public Parameters: SkiaGesturesParameters,
    public ProcessingInfo: GestureEventProcessingInfo,
  ) {}
}
