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

/** Recognized gesture (TouchActionResult). LongPressing/Pointer/Touch declared for parity, not produced yet. */
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

/** AppoMobi.Gestures MouseButton: which button pressed / released (DOM button 0..4). */
export type MouseButton = "Left" | "Middle" | "Right" | "XButton1" | "XButton2" | "Extended";
export type PointerDeviceType = "Mouse" | "Touch" | "Pen";

/** AppoMobi.Gestures PointerData: the device and button behind a Down / Up / Tapped (every button is delivered). */
export class PointerData {
  Button: MouseButton = "Left";
  /** 1 = Left, 2 = Right, 3 = Middle, 4+ = extended, like AppoMobi.Gestures. */
  ButtonNumber = 1;
  DeviceType: PointerDeviceType = "Mouse";
  /** DOM `buttons` bitmask of the buttons held (1 left, 2 right, 4 middle, 8 back, 16 forward). */
  PressedButtons = 0;
}

/** AppoMobi.Gestures TouchActionEventArgs. */
export class TouchActionEventArgs {
  Id = 0;
  /** Device and button of this event (mouse, pen, touch); undefined for wheel. */
  Pointer?: PointerData;
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
/** Where a context-menu request came from: right click, long press (Android fires contextmenu), or the keyboard Menu key. */
export type ContextMenuSource = "mouse" | "touch" | "keyboard";

/**
 * Arguments of SkiaControl.ContextMenu / Canvas.ContextMenu: a browser `contextmenu` request on the canvas (right
 * click, long press on touch, the Menu key). Handlers return true to take it: the browser's own menu ("Save image")
 * is then suppressed; otherwise it shows as usual.
 */
export class ContextMenuEventArgs {
  /** Deepest control under the point that had a ContextMenu handler (set while routing). */
  Control?: SkiaControl;
  /** Point inside Control, in pixels relative to its DrawingRect origin (set while routing). */
  Local: SKPoint = SKPoint.Empty;
  constructor(
    /** Point on the canvas, in points (CSS px). */
    public Location: SKPoint,
    /** Same point in pixels (canvas space). */
    public Pixels: SKPoint,
    public Source: ContextMenuSource,
    /** The DOM event: modifiers, target, preventDefault if you need it yourself. */
    public Native: MouseEvent,
  ) {}
}

export class ControlTappedEventArgs {
  constructor(
    public Control: SkiaControl,
    public Parameters: SkiaGesturesParameters,
    public ProcessingInfo: GestureEventProcessingInfo,
  ) {}
}
