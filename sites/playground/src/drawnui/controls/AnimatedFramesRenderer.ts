import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { SkiaValueAnimator } from "../core/Animators";

/**
 * Mirrors DrawnUi AnimatedFramesRenderer: base for controls that play frames (Lottie, GIF, sprites) with a
 * range animator driven by the canvas frame loop: `AutoPlay`, `Repeat` (-1 = forever), `SpeedRatio`,
 * `DefaultFrame` (-1 = last), `Start`/`Stop`/`Seek`, `Started`/`Finished`.
 */
export class AnimatedFramesRenderer extends SkiaControl {
  Finished?: (sender: AnimatedFramesRenderer) => void;
  Started?: (sender: AnimatedFramesRenderer) => void;

  /** Range animator over the frames, created on the first layout (C# RangeAnimator). */
  Animator?: SkiaValueAnimator;
  /** Start() was asked before the control could play; plays as soon as it can. */
  PlayWhenAvailable = false;
  protected WasStarted = false;
  private delayedPlay = false;
  private wasLayout = false;
  private defaultFrame = 0;
  private speedRatio = 1;
  private repeat = 0;
  private autoPlay = true;

  /** Frame shown when not playing; -1 = the last frame. */
  get DefaultFrame(): number { return this.defaultFrame; }
  set DefaultFrame(v: number) { if (this.defaultFrame !== v) { this.defaultFrame = v; this.ApplyDefaultFrame(); } }
  /** Playback speed multiplier. */
  get SpeedRatio(): number { return this.speedRatio; }
  set SpeedRatio(v: number) { if (this.speedRatio !== v) { this.speedRatio = v; this.ApplySpeed(); } }
  /** > 0 how many more times to repeat, < 0 loops forever. */
  get Repeat(): number { return this.repeat; }
  set Repeat(v: number) { if (this.repeat !== v) { this.repeat = v; if (this.Animator) this.Animator.Repeat = v; } }
  get AutoPlay(): boolean { return this.autoPlay; }
  set AutoPlay(v: boolean) { if (this.autoPlay !== v) { this.autoPlay = v; if (!v) this.Stop(); } }

  get IsPlaying(): boolean { return !!this.Animator?.IsRunning; }
  protected get CanPlay(): boolean { return this.wasLayout && this.CheckCanStartAnimator(); }

  protected override Paint(ctx: DrawingContext): void { this.RenderFrame(ctx); }
  /** Draws the current frame into ctx.Destination. */
  protected RenderFrame(_ctx: DrawingContext): void {}

  protected override OnLayoutChanged(): void {
    super.OnLayoutChanged();
    // Arrange runs every frame here (C# only on a real layout change): initialize once, or a playing
    // animator would be restarted at its first frame on every draw
    if (!this.wasLayout) { this.wasLayout = true; this.InitializeAnimator(); }
    this.PlayIfNeeded();
  }

  PlayIfNeeded(): void {
    if (this.PlayWhenAvailable && this.Animator) {
      this.PlayWhenAvailable = false;
      if (!this.Animator.IsRunning) this.Start();
    }
  }

  InitializeAnimator(): void {
    if (!this.Animator) {
      const a = new SkiaValueAnimator(this);
      a.OnUpdated = (v) => this.OnAnimatorUpdated(v);
      a.OnStart = () => { this.WasStarted = true; this.OnStarted(); };
      a.OnStop = () => { if (this.WasStarted) this.OnFinished(); this.WasStarted = false; };
      this.Animator = a;
    }
    this.Animator.Repeat = this.repeat;
    this.OnAnimatorInitializing();
    if (this.delayedPlay || (this.autoPlay && this.CheckCanStartAnimator())) { this.delayedPlay = false; this.Start(); }
  }

  /** Configure the animator range/speed once the frames source is known. */
  protected OnAnimatorInitializing(): void {}
  /** The animator produced a new frame value. */
  protected OnAnimatorUpdated(_value: number): void {}
  protected CheckCanStartAnimator(): boolean { return true; }
  protected OnAnimatorStarting(): void {}
  protected OnAnimatorSeeking(_frame: number): void {}

  Seek(frame: number): void { this.OnAnimatorSeeking(frame); }

  Start(delayMs = 0): void {
    if (!this.Animator) { this.delayedPlay = true; return; }
    if (this.Animator.IsRunning) this.Animator.Stop();
    if (this.CanPlay) {
      this.Animator.Repeat = this.repeat; // the animator consumes its counter while looping
      this.OnAnimatorStarting();
      this.Animator.Start(delayMs);
    }
    if (!this.Animator.IsRunning) this.PlayWhenAvailable = true;
    this.InvalidateCache();
    this.RepaintComposition();
  }

  Stop(): void {
    this.PlayWhenAvailable = false;
    this.delayedPlay = false;
    if (!this.Animator) return;
    this.Animator.Stop();
    this.InvalidateCache();
    this.RepaintComposition();
  }

  /** Frame at a 0..1 ratio of the animator range. */
  GetFrameAt(ratio: number): number { return this.Animator ? (this.Animator.mMaxValue - this.Animator.mMinValue) * ratio : 0; }

  protected OnFinished(): void { this.Finished?.(this); }
  protected OnStarted(): void { this.Started?.(this); }
  protected ApplySpeed(): void {}
  protected ApplyDefaultFrame(): void { if (!this.IsPlaying && this.Animator) this.Seek(this.defaultFrame); }

  protected override OnDisposing(): void {
    if (this.Animator) { this.Stop(); this.Animator.Dispose(); this.Animator = undefined; }
  }
}
