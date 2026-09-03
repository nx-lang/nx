import type { SkiaControl } from "../core/SkiaControl";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import type { DrawerDirection } from "../core/Types";
import { SnappingLayout } from "./SnappingLayout";

/**
 * Mirrors DrawnUi SkiaDrawer: a panel that slides from an edge (`Direction`), leaving `HeaderSize` points
 * visible when closed (or travelling `AmplitudeSize`), dragged by gestures and snapped open/closed with
 * velocity; `IsOpen` drives and reports the state. The drawer moves itself with TranslationX/Y (like C#),
 * so it must be aligned to its edge by the parent (VerticalOptions="End" for FromBottom, etc.).
 */
export class SkiaDrawer extends SnappingLayout {
  Direction: DrawerDirection = "FromBottom";
  /** Points that stay visible when closed. */
  HeaderSize = 0;
  /** Travel distance override, points; -1 = size minus header. */
  AmplitudeSize = -1;
  /** Close when a touch lands outside the drawer while open. */
  AutoClose = false;
  IsOpenChanged?: (sender: SkiaDrawer, isOpen: boolean) => void;
  StateTransitionComplete?: (sender: SkiaDrawer, isOpen: boolean) => void;

  private isOpen = false;
  private lastSize = { W: 0, H: 0 };
  private wasDrawn = false;
  private panningOffset = SKPoint.Empty;
  private childWasTapped = false;
  private hadDown = false;

  constructor() { super(); this.HorizontalOptions = "Fill"; }

  get IsOpen(): boolean { return this.isOpen; }
  set IsOpen(v: boolean) {
    if (this.isOpen === v) return;
    this.isOpen = v;
    this.IsOpenChanged?.(this, v);
    this.NotifyAccessibility();
    if (this.SnapPoints.length === 2) this.ScrollToOffset(v ? this.SnapPoints[0] : this.SnapPoints[1], SKPoint.Empty, this.Animated && this.wasDrawn);
  }
  Open(): void { this.IsOpen = true; }
  Close(): void { this.IsOpen = false; }

  get Horizontal(): boolean { return this.Direction === "FromLeft" || this.Direction === "FromRight"; }

  /** C# GetOffsetToHide: translation that hides everything but the header. */
  private OffsetToHide(): SKPoint {
    const w = this.DrawingRect.Width / this.RenderingScale, h = this.DrawingRect.Height / this.RenderingScale;
    const travelH = this.AmplitudeSize >= 0 ? this.AmplitudeSize : h - this.HeaderSize;
    const travelW = this.AmplitudeSize >= 0 ? this.AmplitudeSize : w - this.HeaderSize;
    switch (this.Direction) {
      case "FromLeft": return new SKPoint(-travelW, 0);
      case "FromRight": return new SKPoint(travelW, 0);
      case "FromTop": return new SKPoint(0, -travelH);
      default: return new SKPoint(0, travelH);
    }
  }

  protected override GetAutoVelocity(displacement: SKPoint): SKPoint {
    const v = 1500;
    return this.Horizontal ? new SKPoint(-v * Math.sign(displacement.X), 0) : new SKPoint(0, -v * Math.sign(displacement.Y));
  }

  /** C# ApplyOptions: SnapPoints = [open, hidden]; snap to the current IsOpen. */
  private ApplyOptions(): void {
    const hide = this.OffsetToHide();
    this.SnapPoints = [SKPoint.Empty, hide];
    this.ContentOffsetBounds = this.BoundsFromSnapPoints();
    this.CurrentSnap = new SKPoint(-1, -1);
    this.ScrollToOffset(this.isOpen ? this.SnapPoints[0] : this.SnapPoints[1], SKPoint.Empty, this.Animated && this.wasDrawn);
  }

  protected override OnLayoutChanged(): void {
    super.OnLayoutChanged();
    const w = this.DrawingRect.Width, h = this.DrawingRect.Height;
    if (w !== this.lastSize.W || h !== this.lastSize.H) { this.lastSize = { W: w, H: h }; this.ApplyOptions(); }
    this.wasDrawn = true;
  }

  override ApplyPosition(position: SKPoint): void {
    this.TranslationX = position.X;
    this.TranslationY = position.Y;
    super.ApplyPosition(position);
    this.RepaintComposition();
  }

  override UpdateReportedPosition(): void {
    if (this.InTransition || this.SnapPoints.length < 2) return;
    const hidden = this.SnapPoints[1];
    const open = !(Math.abs(hidden.X - this.CurrentSnap.X) <= 1 && Math.abs(hidden.Y - this.CurrentSnap.Y) <= 1);
    if (open !== this.isOpen) { this.isOpen = open; this.IsOpenChanged?.(this, open); this.NotifyAccessibility(); }
  }

  override CheckTransitionEnded(): boolean {
    const ended = super.CheckTransitionEnded();
    if (ended && this.InTransition) this.StateTransitionComplete?.(this, this.isOpen);
    return ended;
  }

  // ---- gestures (port of C# SkiaDrawer.ProcessGestures, single pointer) ----
  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    const consumedDefault = this.BlockGesturesBelow ? this : null;
    const scale = this.RenderingScale;
    const e = args.Event;
    // the drawer sits in a full-size layer: touches outside its (translated) box are not ours
    const local = apply.MappedLocation;
    const hit = this.HitIsInside(local.X + apply.ChildOffset.X, local.Y + apply.ChildOffset.Y);
    if (!this.hadDown && args.Type !== "Up") {
      if (!hit) { if (this.AutoClose && this.isOpen && !this.InTransition) this.IsOpen = false; return null; }
    }
    let passed = false;
    const passToChildren = () => { passed = true; return super.ProcessGestures(args, apply); };
    let consumed: SkiaControl | null = null;
    if (args.Type === "Up" || args.Type === "Tapped" || !this.IsUserPanning || !this.RespondsToGestures) {
      consumed = passToChildren();
      if (consumed === this) consumed = null;
      if (consumed && args.Type !== "Up") { if (args.Type === "Tapped") this.childWasTapped = true; return consumed; }
    }
    if (!this.RespondsToGestures) return consumed;
    const resetPan = () => {
      this.IsUserFocused = true; this.IsUserPanning = false; this.childWasTapped = false;
      this.StopSnapAnimators(); this.velocityAccumulator.Clear();
      this.panningOffset = new SKPoint(this.TranslationX, this.TranslationY);
    };
    switch (args.Type) {
      case "Tapped": case "LongPressing": consumed = this; break;
      case "Down": this.hadDown = true; resetPan(); break;
      case "Panning": {
        if (!this.hadDown) return consumedDefault;
        const horizontal = this.Horizontal;
        // dragging further open at the open edge: no rubber (C# lockBounce)
        const openEdge = this.SnapPoints[0], atOpen = openEdge && Math.abs(this.CurrentPosition.X - openEdge.X) <= 1 && Math.abs(this.CurrentPosition.Y - openEdge.Y) <= 1;
        const dx = e.Distance.Delta.X, dy = e.Distance.Delta.Y;
        const lockBounce = !!atOpen && ((this.Direction === "FromLeft" && dx > 0) || (this.Direction === "FromRight" && dx < 0) || (this.Direction === "FromBottom" && dy < 0) || (this.Direction === "FromTop" && dy > 0));
        if (!this.IsUserFocused) { resetPan(); this.panningOffset = new SKPoint(this.panningOffset.X - dx / scale, this.panningOffset.Y - dy / scale); }
        let x = this.panningOffset.X + dx / scale, y = this.panningOffset.Y + dy / scale;
        if (!this.IsUserPanning) {
          const tx = Math.abs(e.Distance.Total.X), ty = Math.abs(e.Distance.Total.Y);
          const mainHorizontal = tx > ty * 0.9, mainVertical = ty > tx * 0.9;
          if (this.IgnoreWrongDirection && ((horizontal && !mainHorizontal) || (!horizontal && !mainVertical))) break;
          if (horizontal ? tx < scale : ty < scale) break;
          this.IsUserPanning = true;
        }
        if (horizontal) { this.velocityAccumulator.CaptureVelocity(e.Distance.Velocity.X / scale, 0, args.ArrivedTimeNanos); y = 0; }
        else { this.velocityAccumulator.CaptureVelocity(0, e.Distance.Velocity.Y / scale, args.ArrivedTimeNanos); x = 0; }
        this.panningOffset = new SKPoint(x, y);
        const clamped = this.ClampOffset(x, y, this.Bounces && !lockBounce);
        if (!this.Bounces && lockBounce && Math.abs(clamped.X) <= 1 && Math.abs(clamped.Y) <= 1) { this.IsUserPanning = false; return null; } // let a parent scroll take it
        this.ApplyPosition(clamped);
        consumed = this;
        break;
      }
      case "Up": {
        this.hadDown = false;
        if (this.childWasTapped || !this.IsUserPanning) break;
        const final = this.velocityAccumulator.CalculateFinalVelocity(3000);
        const v = this.Horizontal ? new SKPoint(final.X, 0) : new SKPoint(0, final.Y);
        this.CurrentSnap = this.CurrentPosition;
        this.ScrollToNearestAnchor(this.CurrentPosition, v);
        this.IsUserPanning = false; this.IsUserFocused = false;
        consumed = this;
        break;
      }
    }
    if (consumed || this.IsUserPanning) return consumed ?? (args.Type !== "Up" ? this : consumedDefault);
    if (!passed) return passToChildren();
    return consumedDefault;
  }

  protected override DefaultAccessibilityLabel(): string | undefined { return this.isOpen ? "Drawer open" : "Drawer closed"; }
}
