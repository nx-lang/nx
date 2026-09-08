// Mirrors DrawnUi Features/Animators: AnimatorBase -> SkiaValueAnimator -> RenderingAnimator (+ RippleAnimator).
// Animators register with the Canvas (Superview.RegisterAnimator) and tick once per drawn frame.

import type { DrawingContext, SkiaControl } from "./SkiaControl";
import { Easing } from "./Easing";
import { Super } from "./Super";
import { type Color, Colors } from "./Types";

/** DrawnUi IOverlayEffect: drawn by the owning control after its content (PostAnimators). */
export interface IOverlayEffect {
  /** Return true to request another frame (still animating). */
  Render(context: DrawingContext, control: SkiaControl): boolean;
}

let uidCounter = 0;

/** DrawnUi AnimatorBase: registration + running state. Time is in nanoseconds like the .NET version. */
export class AnimatorBase {
  readonly Uid = ++uidCounter;
  Parent: SkiaControl | null;
  /** Post animators are also added to Parent.PostAnimators and drawn above its content. */
  IsPostAnimator = false;
  IsRunning = false;
  WasStarted = false;
  LastFrameTimeNanos = 0;
  StartFrameTimeNanos = 0;
  OnStart?: () => void;
  OnStop?: () => void;

  private delayHandle = 0;

  constructor(parent: SkiaControl) { this.Parent = parent; }

  protected Register(): boolean {
    const parent = this.Parent;
    if (!parent) return false;
    if (this.IsPostAnimator) {
      const effect = this as unknown as IOverlayEffect;
      if (typeof effect.Render !== "function") throw new Error("Post animator must implement IOverlayEffect");
      if (!parent.PostAnimators.includes(effect)) parent.PostAnimators.push(effect);
    }
    const ok = parent.Superview?.RegisterAnimator(this) ?? false;
    parent.Repaint();
    return ok;
  }

  protected Unregister(): void {
    const parent = this.Parent;
    if (!parent) return;
    if (this.IsPostAnimator) {
      const i = parent.PostAnimators.indexOf(this as unknown as IOverlayEffect);
      if (i >= 0) parent.PostAnimators.splice(i, 1);
    }
    parent.Superview?.UnregisterAnimator(this.Uid);
  }

  Start(delayMs = 0): void {
    if (this.delayHandle) { clearTimeout(this.delayHandle); this.delayHandle = 0; }
    if (delayMs > 0) {
      this.delayHandle = window.setTimeout(() => { this.delayHandle = 0; this.Start(); }, delayMs);
      return;
    }
    if (this.Register()) {
      this.WasStarted = true;
      if (!this.IsRunning) {
        this.LastFrameTimeNanos = 0;
        this.StartFrameTimeNanos = 0;
        this.SetRunning(true);
      }
    }
  }

  Stop(): void {
    if (this.delayHandle) { clearTimeout(this.delayHandle); this.delayHandle = 0; }
    this.Unregister();
    this.LastFrameTimeNanos = 0;
    this.StartFrameTimeNanos = 0;
    this.SetRunning(false);
    this.WasStarted = false;
  }

  /** Returns true when there is nothing more to do (not running / finished). */
  TickFrame(frameTimeNanos: number): boolean {
    if (!this.IsRunning) return true;
    this.LastFrameTimeNanos = frameTimeNanos;
    return false;
  }

  Dispose(): void {
    this.Stop();
    this.Parent = null;
    this.OnStart = undefined;
    this.OnStop = undefined;
  }

  private SetRunning(value: boolean): void {
    if (this.IsRunning === value) return;
    this.IsRunning = value;
    if (value) this.OnStart?.(); else this.OnStop?.();
  }
}

/** DrawnUi SkiaValueAnimator: drives mValue from mMinValue to mMaxValue over Speed ms with Easing. */
export class SkiaValueAnimator extends AnimatorBase {
  mValue = 0;
  mMinValue = Number.MIN_SAFE_INTEGER;
  mMaxValue = Number.MAX_SAFE_INTEGER;
  /** Duration in ms. */
  Speed = 0;
  Easing: Easing = Easing.Linear;
  /** -1 = forever, n = repeat n more times. */
  Repeat = 0;
  /** Eased progress, may exceed 1 on the finishing frame. */
  protected Progress = 0;
  OnUpdated?: (value: number) => void;
  Finished?: () => void;

  override TickFrame(frameTimeNanos: number): boolean {
    if (!this.IsRunning) return true;
    if (this.LastFrameTimeNanos === 0) {
      this.LastFrameTimeNanos = frameTimeNanos;
      this.StartFrameTimeNanos = frameTimeNanos;
    }
    const deltaFromStart = frameTimeNanos - this.StartFrameTimeNanos;
    const deltaNanos = frameTimeNanos - this.LastFrameTimeNanos;
    this.LastFrameTimeNanos = frameTimeNanos;

    let finished = this.UpdateValue(deltaNanos, deltaFromStart);
    // Always evaluate: subclasses derive their state here (a?.(f()) would skip f() with no subscriber).
    const reported = this.TransformReportedValue(deltaNanos);
    this.OnUpdated?.(reported);
    if (finished) finished = this.FinishedRunning();
    return finished;
  }

  /** Passed over mValue; subclasses derive their own reported value here. */
  protected TransformReportedValue(_deltaT: number): number { return this.mValue; }

  /** Updates mValue from elapsed time; true when the target was reached. */
  protected UpdateValue(_deltaT: number, deltaFromStart: number): boolean {
    const elapsedMs = deltaFromStart / 1_000_000;
    const progress = this.Speed > 0 ? elapsedMs / this.Speed : 1;
    const deltaValue = this.mMaxValue - this.mMinValue;
    const eased = this.Easing.Ease(Math.min(progress, 1));
    this.Progress = eased;
    const value = deltaValue * eased + this.mMinValue;
    if (value < this.mMinValue) { this.mValue = this.mMinValue; return false; }
    if (value >= this.mMaxValue || progress >= 1) { this.mValue = this.mMaxValue; return true; }
    this.mValue = value;
    return false;
  }

  protected FinishedRunning(): boolean {
    if (this.Repeat < 0 || this.Repeat > 0) {
      if (this.Repeat > 0) this.Repeat--;
      this.mValue = this.mMinValue;
      this.LastFrameTimeNanos = 0;
      this.StartFrameTimeNanos = 0;
      return false;
    }
    this.Stop();
    this.Finished?.();
    return true;
  }
}

/** DrawnUi RenderingAnimator: a value animator that paints itself above its Parent (IOverlayEffect). */
export class RenderingAnimator extends SkiaValueAnimator implements IOverlayEffect {
  constructor(parent: SkiaControl) { super(parent); }

  override Stop(): void {
    const parent = this.Parent;
    super.Stop();
    parent?.Repaint();
  }

  Render(context: DrawingContext, control: SkiaControl): boolean {
    return this.OnRendering(context, control);
  }

  protected OnRendering(_context: DrawingContext, _control: SkiaControl): boolean { return false; }

  /** Top-left of the control in canvas pixels (its last drawn position). */
  protected static GetSelfDrawingLocation(control: SkiaControl): { X: number; Y: number } {
    return { X: control.DrawingRect.Left, Y: control.DrawingRect.Top };
  }

  /** Runs draw() clipped to the control's shape when control.ClipEffects is set. */
  protected static DrawWithClipping(context: DrawingContext, control: SkiaControl, draw: () => void): void {
    const canvas = context.Context.Canvas;
    if (!control.ClipEffects) { draw(); return; }
    const clip = control.CreateClip();
    const count = canvas.save();
    canvas.clipPath(clip, Super.CK.ClipOp.Intersect, true);
    draw();
    canvas.restoreToCount(count);
    clip.delete();
  }
}

/** DrawnUi RippleAnimator: expanding fading circle from the touch point, 500ms CubicIn. */
export class RippleAnimator extends RenderingAnimator {
  static DiameterDefault = 300.0;
  static OpacityDefault = 0.2;

  Color: Color = Colors.White;
  /** Touch point inside the control, points. */
  X = 0;
  Y = 0;
  Diameter = 0;
  Opacity = 0;

  constructor(control: SkiaControl) {
    super(control);
    this.IsPostAnimator = true;
    this.Speed = 500;
    this.mMinValue = 0;
    this.mMaxValue = 1;
    this.Easing = Easing.CubicIn;
  }

  protected override OnRendering(context: DrawingContext, control: SkiaControl): boolean {
    if (!this.IsRunning) return false;
    const loc = RenderingAnimator.GetSelfDrawingLocation(control);
    const x = this.X * context.Scale + loc.X;
    const y = this.Y * context.Scale + loc.Y;
    RenderingAnimator.DrawWithClipping(context, control, () => {
      const paint = new Super.CK.Paint();
      paint.setAntiAlias(true);
      const c = Super.ParseColor(this.Color);
      paint.setColor(Super.CK.Color4f(c[0], c[1], c[2], this.Opacity));
      // Same as the .NET RippleAnimator: Diameter is passed as the circle radius.
      context.Context.Canvas.drawCircle(x, y, this.Diameter * context.Scale, paint);
      paint.delete();
    });
    return true;
  }

  protected override TransformReportedValue(deltaT: number): number {
    const progress = super.TransformReportedValue(deltaT);
    const opacityProgress = progress * 1.15;
    if (opacityProgress <= 1) this.Opacity = RippleAnimator.OpacityDefault - RippleAnimator.OpacityDefault * opacityProgress;
    this.Diameter = RippleAnimator.DiameterDefault * progress;
    return progress;
  }
}
