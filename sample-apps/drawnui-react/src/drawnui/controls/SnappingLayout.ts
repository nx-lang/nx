import { SKPoint } from "../core/Gestures";
import { Easing } from "../core/Easing";
import { SkiaValueAnimator } from "../core/Animators";
import { Spring, SpringWithVelocityAnimator, RubberBandUtils, VelocityAccumulator } from "../core/ScrollAnimators";
import { SKRect } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SnappingLayout: a layout whose content position snaps to `SnapPoints` (points). Base of
 * SkiaCarousel and SkiaDrawer: nearest/next anchor selection by velocity, spring (Bounces) or linear snap
 * animation, rubber-band clamping, `Scrolled` / `TransitionChanged` / `Stopped` callbacks.
 */
export abstract class SnappingLayout extends SkiaLayout {
  Animated = true;
  RespondsToGestures = true;
  IgnoreWrongDirection = false;
  Bounces = false;
  RubberDamping = 0.7;
  AutoVelocityMultiplyPts = 25;
  RubberEffect = 0.15;
  /** Fraction of a snap step a drag must cover before it counts as a direction (C# SnapDistanceRatio). */
  SnapDistanceRatio = 0.2;

  SnapPoints: SKPoint[] = [];
  /** Current content offset, points (C# CurrentPosition). */
  CurrentPosition = SKPoint.Empty;
  /** Anchor the content rests on or travels to (C# CurrentSnap). */
  CurrentSnap = new SKPoint(-1, -1);
  ContentOffsetBounds: SKRect = SKRect.Empty;
  IsUserPanning = false;
  IsUserFocused = false;

  Scrolled?: (sender: SnappingLayout, position: SKPoint) => void;
  TransitionChanged?: (sender: SnappingLayout, inTransition: boolean) => void;
  Stopped?: (sender: SnappingLayout, position: SKPoint) => void;

  private inTransition = false;
  get InTransition(): boolean { return this.inTransition; }
  protected set InTransition(v: boolean) { if (this.inTransition !== v) { this.inTransition = v; this.OnTransitionChanged(); } }
  protected OnTransitionChanged(): void { this.TransitionChanged?.(this, this.inTransition); }

  protected readonly velocityAccumulator = new VelocityAccumulator();
  private readonly springX = new SpringWithVelocityAnimator(this);
  private readonly springY = new SpringWithVelocityAnimator(this);
  private readonly range = new SkiaValueAnimator(this);
  private rangeFrom = SKPoint.Empty;
  private rangeTo = SKPoint.Empty;

  constructor() {
    super();
    this.Type = "Absolute";
    this.springX.OnUpdated = (v) => this.ApplyPosition(new SKPoint(v, this.CurrentPosition.Y));
    this.springY.OnUpdated = (v) => this.ApplyPosition(new SKPoint(this.CurrentPosition.X, v));
    this.springX.OnStop = this.springY.OnStop = () => { if (!this.springX.IsRunning && !this.springY.IsRunning) this.OnAnimationStopped(); };
    this.range.mMinValue = 0; this.range.mMaxValue = 1; this.range.Easing = Easing.CubicInOut;
    this.range.OnUpdated = (t) => this.ApplyPosition(new SKPoint(this.rangeFrom.X + (this.rangeTo.X - this.rangeFrom.X) * t, this.rangeFrom.Y + (this.rangeTo.Y - this.rangeFrom.Y) * t));
    this.range.OnStop = () => this.OnAnimationStopped();
  }

  private OnAnimationStopped(): void {
    this.InTransition = !this.CheckTransitionEnded();
    this.UpdateReportedPosition(); // subclasses gate reporting on InTransition: report the settled state now
    this.Stopped?.(this, this.CurrentPosition);
  }

  protected StopSnapAnimators(): void { this.springX.Stop(); this.springY.Stop(); this.range.Stop(); }

  // ---- clamping ----
  ClampOffset(x: number, y: number, rubber: boolean): SKPoint {
    const b = this.ContentOffsetBounds;
    if (!rubber) return new SKPoint(Math.max(b.Left, Math.min(b.Right, x)), Math.max(b.Top, Math.min(b.Bottom, y)));
    const c = RubberBandUtils.ClampOnTrack(x, y, b, this.RubberEffect, this.DrawingRect.Width / this.RenderingScale, this.DrawingRect.Height / this.RenderingScale);
    return new SKPoint(c.X, c.Y);
  }

  /** C# GetContentOffsetBounds: the extent covered by the snap points. */
  protected BoundsFromSnapPoints(): SKRect {
    if (this.SnapPoints.length === 0) return SKRect.Empty;
    let l = Infinity, t = Infinity, r = -Infinity, b = -Infinity;
    for (const p of this.SnapPoints) { l = Math.min(l, p.X); t = Math.min(t, p.Y); r = Math.max(r, p.X); b = Math.max(b, p.Y); }
    return new SKRect(l, t, r, b);
  }

  // ---- anchors ----
  private static Dist(a: SKPoint, b: SKPoint): number { return Math.hypot(a.X - b.X, a.Y - b.Y); }

  FindNearestAnchor(current: SKPoint): SKPoint {
    let best = current, d = Infinity;
    for (const p of this.SnapPoints) { const dd = SnappingLayout.Dist(p, current); if (dd < d) { d = dd; best = p; } }
    return best;
  }

  /** The closest anchor lying in the direction of the velocity (dot product), else the origin. */
  SelectNextAnchor(origin: SKPoint, velocity: SKPoint): SKPoint {
    const len = Math.hypot(velocity.X, velocity.Y);
    if (len === 0) return origin;
    const nx = velocity.X / len, ny = velocity.Y / len;
    const ordered = [...this.SnapPoints].sort((a, b) => SnappingLayout.Dist(a, origin) - SnappingLayout.Dist(b, origin));
    for (const anchor of ordered) {
      const dx = anchor.X - origin.X, dy = anchor.Y - origin.Y, l = Math.hypot(dx, dy);
      if (l === 0) continue;
      if (nx * (dx / l) + ny * (dy / l) > 0) return anchor;
    }
    return origin;
  }

  ScrollToNearestAnchor(location: SKPoint, velocity: SKPoint): void {
    if (this.SnapPoints.length === 0) return;
    const origin = this.FindNearestAnchor(location);
    const target = this.SelectNextAnchor(origin, velocity);
    if (SnappingLayout.Dist(location, target) >= 0.5) this.ScrollToOffset(target, velocity, this.CanAnimate);
    else this.UpdateReportedPosition();
  }

  get CanAnimate(): boolean { return this.IsVisible && this.DrawingRect.Height > 0; }

  protected GetAutoVelocity(displacement: SKPoint): SKPoint {
    const v = this.AutoVelocityMultiplyPts * this.RenderingScale;
    return new SKPoint(-v * Math.sign(displacement.X), -v * Math.sign(displacement.Y));
  }

  /** C# ScrollToOffset: spring when Bounces, else a duration derived from the velocity (0.1–0.8 s). */
  protected ScrollToOffset(target: SKPoint, velocity: SKPoint, animate: boolean): boolean {
    if (target.X === this.CurrentSnap.X && target.Y === this.CurrentSnap.Y && !animate) return false;
    this.StopSnapAnimators();
    if (animate && this.DrawingRect.Height > 0) {
      const start = this.CurrentPosition;
      const displacement = new SKPoint(start.X - target.X, start.Y - target.Y);
      if (velocity.X === 0 && velocity.Y === 0) velocity = this.GetAutoVelocity(displacement);
      if (displacement.X !== 0 || displacement.Y !== 0) {
        this.InTransition = true;
        if (this.Bounces) {
          const spring = new Spring(1 * (1 + this.RubberDamping), 200, 0.5 * (1 + this.RubberDamping));
          if (displacement.X !== 0) { this.springX.Initialize(target.X, displacement.X, velocity.X, spring); this.springX.Start(); }
          if (displacement.Y !== 0) { this.springY.Initialize(target.Y, displacement.Y, velocity.Y, spring); this.springY.Start(); }
        } else {
          const magnitude = Math.hypot(velocity.X, velocity.Y);
          let speed = 0.3;
          if (magnitude > 10) speed = 0.7 * Math.max(0.1, Math.min(0.8, 300 / magnitude));
          else { const h = this.DrawingRect.Height / this.RenderingScale; speed *= Math.max(Math.abs(displacement.X), Math.abs(displacement.Y)) / (h || 1); }
          this.rangeFrom = start; this.rangeTo = target; this.range.Speed = Math.max(60, speed * 1000);
          this.range.Start();
        }
      }
    } else {
      this.CurrentSnap = target;
      this.ApplyPosition(target);
    }
    this.CurrentSnap = target;
    this.UpdateReportedPosition();
    return true;
  }

  /** Subclasses move their content here; base records the position and reports (C# ApplyPosition). */
  ApplyPosition(position: SKPoint): void {
    this.CurrentPosition = position;
    this.UpdateReportedPosition();
    this.Scrolled?.(this, position);
  }

  CheckTransitionEnded(): boolean { return Math.abs(this.CurrentPosition.X - this.CurrentSnap.X) <= 1 && Math.abs(this.CurrentPosition.Y - this.CurrentSnap.Y) <= 1; }

  /** Subclasses translate CurrentSnap into their own state (SelectedIndex, IsOpen). */
  UpdateReportedPosition(): void {}
}
