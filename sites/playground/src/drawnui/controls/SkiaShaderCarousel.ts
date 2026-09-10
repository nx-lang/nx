import type { DrawingContext } from "../core/SkiaControl";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import { ShaderTransitionEffect } from "../core/SkiaEffect";
import type { ScaledSize } from "../core/Types";
import type { SkiaControl } from "../core/SkiaControl";
import { SkiaCarousel } from "./SkiaCarousel";

/**
 * Mirrors DrawnUi SkiaShaderCarousel: slides never move, an attached `ShaderTransitionEffect` blends the cached
 * images of the outgoing and incoming slides (`TransitionShader` url of a gl-transitions style `transition(vec2 uv)`
 * SkSL, or `TransitionShaderCode`, optionally a custom `TransitionTemplate`). Slides MUST be cached with
 * `UseCache="Image"` (the effect samples their caches). One gesture moves at most one slide; a swipe during a running
 * transition wraps it up within `InterruptedTransitionMs` first. The carousel itself is cached as Image so the
 * overlapping slides are never shown directly.
 */
export class SkiaShaderCarousel extends SkiaCarousel {
  /** Url of the transition .sksl (fetched, wrapped by the adapter template). Can change at any time. */
  get TransitionShader(): string { return this.TransitionEffect.ShaderSource; }
  set TransitionShader(v: string) { this.TransitionEffect.ShaderSource = v ?? ""; }
  /** Raw SkSL alternative to TransitionShader. */
  get TransitionShaderCode(): string { return this.TransitionEffect.ShaderCode; }
  set TransitionShaderCode(v: string) { this.TransitionEffect.ShaderCode = v ?? ""; }
  /** Url of a custom template replacing the embedded gl-transitions adapter. */
  get TransitionTemplate(): string { return this.TransitionEffect.ShaderTemplate; }
  set TransitionTemplate(v: string) { this.TransitionEffect.ShaderTemplate = v ?? ""; }
  /** Wrap-up time of a transition interrupted by a new swipe, ms (0 = instant). */
  InterruptedTransitionMs = 50;
  /** The effect rendering the transition (extra uniforms, OnCompilationError). Created by CreateTransitionEffect. */
  readonly TransitionEffect: ShaderTransitionEffect;
  /** Index the current transition starts from, -1 before the first layout. */
  get TransitionFromIndex(): number { return this.indexFrom; }
  /** Index the current transition goes to (0 after the last slide when looped). */
  get TransitionToIndex(): number { return this.indexTo; }
  FromToChanged?: (sender: SkiaShaderCarousel) => void;

  private retrySetup = false;
  private lastProgress = 0;
  private initialized = false;
  private indexFrom = -1;
  private indexTo = -1;
  private indexFromLast = -1;
  private indexToLast = -1;
  private wasWrapped = false;
  // gesture targeting
  private wasInTransitionAtDown = false;
  private inGestureRelease = false;
  private gestureOrigin = SKPoint.Empty;
  private gestureFrom = SKPoint.Empty;
  private pendingTarget?: SKPoint;

  constructor() {
    super();
    this.RecyclingTemplate = "Disabled"; // the effect samples specific cells: every index keeps its own view
    this.UseCache = "Image";
    this.TransitionEffect = this.CreateTransitionEffect();
    this.VisualEffects = [this.TransitionEffect];
  }

  /** Factory for the transition effect; override for a subclass with extra uniforms. */
  protected CreateTransitionEffect(): ShaderTransitionEffect { return new ShaderTransitionEffect(); }

  protected OnFromToChanged(): void { this.FromToChanged?.(this); }

  override Render(ctx: DrawingContext): void {
    super.Render(ctx);
    if (this.retrySetup && !this.initialized && this.indexFrom >= 0 && this.indexTo >= 0 && this.indexTo <= this.MaxIndex) {
      this.initialized = this.SetupFromTo();
      if (this.initialized) { this.TransitionEffect.Progress = this.lastProgress; this.TransitionEffect.Update(); }
    }
  }

  /** Points the effect at the from/to views; false when a view does not exist yet (retried after the next render). */
  protected SetupFromTo(): boolean {
    this.indexToLast = this.indexTo;
    this.indexFromLast = this.indexFrom;
    const viewFrom = this.GetExistingView(this.indexFrom);
    const viewTo = this.GetExistingView(this.indexTo);
    if (!viewFrom || !viewTo) { this.retrySetup = true; this.Repaint(); return false; }
    this.retrySetup = false;
    this.TransitionEffect.ControlFrom = viewFrom;
    this.TransitionEffect.ControlTo = viewTo;
    return true;
  }

  protected override MeasureAbsolute(w: number, h: number, scale: number): ScaledSize {
    this.initialized = false;
    return super.MeasureAbsolute(w, h, scale);
  }

  protected override OnChildrenInitialized(): void {
    this.indexFrom = -1; this.indexTo = -1; this.indexFromLast = -1; this.indexToLast = -1;
    this.initialized = false;
    super.OnChildrenInitialized();
  }

  protected override OnScrollProgressChanged(): void {
    if (this.IsLooped && this.MaxIndex > 0) {
      const slides = this.MaxIndex + 1;
      let scaled = this.ScrollProgress * this.MaxIndex;
      if (scaled < 0 || scaled > this.MaxIndex) {
        // beyond the real strip: a true wrap only when panning, snapping to a virtual anchor, or continuing a wrap
        // already shown; otherwise it is spring overshoot at an edge — clamp instead of flashing the far slide
        let wrap = this.IsUserPanning || this.wasWrapped;
        if (!wrap) { const anchor = this.GetVirtualAnchor(this.CurrentSnap); wrap = scaled < 0 ? anchor.Id === -1 : anchor.Id === -2; }
        this.wasWrapped = wrap;
        scaled = wrap ? ((scaled % slides) + slides) % slides : Math.max(0, Math.min(this.MaxIndex, scaled));
      } else this.wasWrapped = false;
      const currentIndex = Math.floor(scaled);
      const progress = scaled - currentIndex;
      if (this.indexFrom !== currentIndex || !this.initialized) {
        this.indexFrom = currentIndex;
        this.indexTo = (currentIndex + 1) % slides;
        if (!this.initialized || this.indexToLast !== this.indexTo || this.indexFromLast !== this.indexFrom) this.initialized = this.SetupFromTo();
        this.OnFromToChanged();
      }
      this.lastProgress = progress;
      this.TransitionEffect.Progress = progress;
      this.TransitionEffect.Update();
      return;
    }
    const sp = this.ScrollProgress;
    if (!this.initialized || (sp >= 0 && sp <= 1)) { // ignore bouncing
      let currentIndex = 0;
      if (sp > 0) currentIndex = Math.floor(this.MaxIndex * sp);
      let progress = this.TransitionProgress;
      if (this.indexFrom !== currentIndex || !this.initialized) {
        if (currentIndex < this.MaxIndex) {
          this.indexTo = currentIndex + 1;
          this.indexFrom = currentIndex;
          if (!this.initialized || this.indexToLast !== this.indexTo || this.indexFromLast !== this.indexFrom) this.initialized = this.SetupFromTo();
        } else progress = 1;
        this.OnFromToChanged();
      }
      this.lastProgress = progress;
      this.TransitionEffect.Progress = progress;
      this.TransitionEffect.Update();
    }
  }

  /** Slides never move: the transition effect renders the change (C# AnimateVisibleChild no-op). */
  protected override SlideOffset(_offset: SKPoint): SKPoint { return SKPoint.Empty; }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Down") {
      const interrupted = this.InTransition;
      const result = super.ProcessGestures(args, apply);
      // read after base: Down may have normalized a looped virtual position; while phase 1 is still wrapping up,
      // the real origin is the pending phase-2 slide
      this.wasInTransitionAtDown = interrupted;
      this.gestureOrigin = this.pendingTarget ?? this.CurrentSnap;
      this.gestureFrom = this.CurrentPosition;
      this.pendingTarget = undefined;
      return result;
    }
    if (args.Type === "Up") {
      this.inGestureRelease = true;
      try { return super.ProcessGestures(args, apply); } finally { this.inGestureRelease = false; }
    }
    return super.ProcessGestures(args, apply);
  }

  /**
   * Gesture targeting (C#): one gesture moves at most one slide from the slide it started on; a flick moves one slide
   * in its direction, a slow drag snaps within one step of the origin; a swipe that interrupted a running transition
   * first wraps that transition up within InterruptedTransitionMs, then the new one plays.
   */
  protected override ScrollToOffset(targetOffset: SKPoint, velocity: SKPoint, animate: boolean): boolean {
    if (!this.inGestureRelease || !animate || this.SnapPoints.length < 2) return super.ScrollToOffset(targetOffset, velocity, animate);
    const axis = (p: SKPoint) => (this.IsVertical ? p.Y : p.X);
    const step = new SKPoint(this.SnapPoints[1].X - this.SnapPoints[0].X, this.SnapPoints[1].Y - this.SnapPoints[0].Y);
    const stepAxis = axis(step);
    if (stepAxis !== 0) {
      const vel = axis(velocity);
      const disp = axis(this.CurrentPosition) - axis(this.gestureFrom);
      let k: number;
      if (Math.abs(vel) >= 100) k = Math.sign(vel) * Math.sign(stepAxis);
      else k = Math.max(-1, Math.min(1, Math.round(disp / stepAxis)));
      let capped = new SKPoint(this.gestureOrigin.X + step.X * k, this.gestureOrigin.Y + step.Y * k);
      if (!this.IsLooped) {
        const first = this.SnapPoints[0], last = this.SnapPoints[this.SnapPoints.length - 1];
        const lo = new SKPoint(Math.min(first.X, last.X), Math.min(first.Y, last.Y)), hi = new SKPoint(Math.max(first.X, last.X), Math.max(first.Y, last.Y));
        capped = new SKPoint(Math.max(lo.X, Math.min(hi.X, capped.X)), Math.max(lo.Y, Math.min(hi.Y, capped.Y)));
      }
      targetOffset = capped; // looped: may be the virtual slot past the edge, the base wraps it
    }
    if (!this.wasInTransitionAtDown) return super.ScrollToOffset(targetOffset, velocity, animate);
    this.wasInTransitionAtDown = false;

    // the pan may already have carried the position PAST the interrupted transition's destination toward the new
    // target: wrapping it up would animate backward first. Only run phase 1 while that destination is still ahead.
    const toOrigin = axis(this.gestureOrigin) - axis(this.CurrentPosition);
    const toTarget = axis(targetOffset) - axis(this.CurrentPosition);
    const same = targetOffset.X === this.gestureOrigin.X && targetOffset.Y === this.gestureOrigin.Y;
    if (!same && (toOrigin === 0 || Math.sign(toOrigin) !== Math.sign(toTarget))) return super.ScrollToOffset(targetOffset, velocity, animate);
    this.pendingTarget = same ? undefined : targetOffset;

    if (this.InterruptedTransitionMs <= 0) {
      super.ScrollToOffset(this.gestureOrigin, SKPoint.Empty, false);
      const instant = this.pendingTarget;
      if (instant) { this.pendingTarget = undefined; return super.ScrollToOffset(instant, velocity, true); }
      return true;
    }
    // phase 1: finish the interrupted transition fast; LinearSpeedMs (ms per slide) scaled to the remaining distance
    const cell = this.IsVertical ? this.CellSize.H : this.CellSize.W;
    const remaining = Math.abs(toOrigin);
    const keep = this.LinearSpeedMs;
    this.LinearSpeedMs = remaining > 0 && cell > 0 ? (this.InterruptedTransitionMs * cell) / remaining : keep;
    let started: boolean;
    try { started = super.ScrollToOffset(this.gestureOrigin, SKPoint.Empty, true); } finally { this.LinearSpeedMs = keep; }
    const next = this.pendingTarget;
    if (!started && next) { this.pendingTarget = undefined; return super.ScrollToOffset(next, velocity, true); }
    return started;
  }

  protected override OnTransitionChanged(): void {
    super.OnTransitionChanged();
    // phase 2: the interrupted transition is wrapped up, now go where the swipe pointed
    const next = this.pendingTarget;
    if (!this.InTransition && next) { this.pendingTarget = undefined; this.ScrollToOffset(next, SKPoint.Empty, true); }
  }
}
