import type { Image } from "canvaskit-wasm";
import type { DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type DrawImageAlignment, SKRect, ScaledSize, type TransformAspect } from "../core/Types";
import { AnimatedFramesRenderer } from "./AnimatedFramesRenderer";
import { SkiaImage } from "./SkiaImage";

/** Mirrors DrawnUi GifAnimation: every frame decoded once, seek by frame or by time. */
export class GifAnimation {
  Frames: Image[] = [];
  /** Current frame image, changed by SeekFrame. */
  Frame?: Image;
  Width = 0;
  Height = 0;
  DurationMs = 0;
  TotalFrames = 0;
  private positionsMs: number[] = [];

  /** Negative = the last frame. */
  SeekFrame(frame: number): void {
    if (frame > this.TotalFrames - 1) frame = this.TotalFrames - 1;
    if (frame < 0) frame = this.TotalFrames - 1;
    if (frame >= 0 && frame <= this.TotalFrames - 1) this.Frame = this.Frames[frame];
  }

  GetFrameNumber(msTime: number): number {
    if (this.positionsMs.length === 0 || this.DurationMs <= 0) return 0;
    if (msTime < 0) msTime = this.DurationMs + msTime;
    msTime %= this.DurationMs;
    for (let i = 0; i < this.positionsMs.length; i++) if (msTime < this.positionsMs[i]) return i;
    return 0;
  }

  /** Decodes every frame of an encoded GIF (CanvasKit AnimatedImage). */
  LoadFromBytes(bytes: ArrayBuffer): void {
    const CK = Super.CK;
    const animated = CK.MakeAnimatedImageFromEncoded(bytes);
    if (!animated) return;
    const count = animated.getFrameCount();
    const frames: Image[] = [];
    const positions: number[] = [];
    let total = 0;
    for (let i = 0; i < count; i++) {
      if (i > 0) animated.decodeNextFrame();
      const duration = Math.max(1, animated.currentFrameDuration()); // ms
      const image = animated.makeImageAtCurrentFrame();
      if (!image) break;
      frames.push(image);
      total += duration;
      positions.push(total);
    }
    this.Width = animated.width();
    this.Height = animated.height();
    animated.delete();
    for (const f of this.Frames) f.delete();
    this.Frames = frames;
    this.TotalFrames = frames.length;
    this.DurationMs = total;
    this.positionsMs = positions;
    this.SeekFrame(0);
  }

  Dispose(): void {
    for (const f of this.Frames) f.delete();
    this.Frames = [];
    this.Frame = undefined;
    this.TotalFrames = 0;
  }
}

/**
 * Mirrors DrawnUi SkiaGif: `Source` (URL / app path), `Aspect` (default AspectFitFill), `AutoPlay`, `Repeat`,
 * `SpeedRatio`, `DefaultFrame`, `Start`/`Stop`/`Seek(ms)`; frames are drawn like SkiaImage draws its bitmap.
 */
export class SkiaGif extends AnimatedFramesRenderer {
  Aspect: TransformAspect = "AspectFitFill";
  HorizontalAlignment: DrawImageAlignment = "Center";
  VerticalAlignment: DrawImageAlignment = "Center";
  Success?: (sender: SkiaGif, source: string) => void;
  Error?: (sender: SkiaGif, error: Error) => void;

  Animation?: GifAnimation;
  private source = "";
  private loadGeneration = 0;

  constructor() {
    super();
    this.UseCache = "ImageDoubleBuffered";
  }

  get Source(): string { return this.source; }
  set Source(v: string) { if (this.source !== v) { this.source = v; this.ReloadSource(); } }

  override Start(delayMs = 0): void { if ((this.Animation?.TotalFrames ?? 0) > 0) super.Start(delayMs); }

  protected override OnAnimatorUpdated(value: number): void { this.Seek(value); }

  /** Seeks by time in ms (the animator range is 0..DurationMs). */
  protected override OnAnimatorSeeking(time: number): void {
    const a = this.Animation;
    if (!a) return;
    const before = a.Frame;
    a.SeekFrame(a.GetFrameNumber(time));
    if (a.Frame !== before) { this.InvalidateCache(); this.RepaintComposition(); }
  }

  protected override OnAnimatorInitializing(): void {
    if (!this.Animation || !this.Animator) return;
    this.ApplySpeed();
    this.Animator.mValue = 0;
    this.Animator.mMinValue = 0;
    this.Animator.mMaxValue = this.Animation.DurationMs;
  }

  protected override ApplySpeed(): void {
    if (!this.Animation || !this.Animator) return;
    const d = this.Animation.DurationMs;
    this.Animator.Speed = this.SpeedRatio < 1 ? d * (1 + this.SpeedRatio) : d / this.SpeedRatio;
  }

  protected override CheckCanStartAnimator(): boolean { return (this.Animation?.TotalFrames ?? 0) > 0; }

  SetAnimation(animation: GifAnimation, disposePrevious: boolean): void {
    if (!animation || animation === this.Animation) return;
    const wasPlaying = this.IsPlaying;
    const kill = this.Animation;
    if (wasPlaying) this.Stop();
    this.Animation = animation;
    this.InitializeAnimator();
    this.OnAnimatorSeeking(this.DefaultFrame);
    if (wasPlaying && !this.IsPlaying) this.Start();
    if (kill && disposePrevious) kill.Dispose();
    this.Update();
  }

  ReloadSource(): void {
    if (!this.source) return;
    const generation = ++this.loadGeneration;
    const source = this.source;
    this.LoadSource(source).then(
      (animation) => { if (generation !== this.loadGeneration) return; if (animation) { this.Success?.(this, source); this.SetAnimation(animation, true); } else this.Error?.(this, new Error(`Failed to load source ${source}`)); },
      (e: Error) => { if (generation === this.loadGeneration) this.Error?.(this, e); },
    );
  }

  /** Loads (does not apply) a GIF; C# LoadSource. */
  async LoadSource(source: string): Promise<GifAnimation | undefined> {
    if (!source) return undefined;
    const r = await fetch(source);
    if (!r.ok) throw new Error(`${r.status} ${source}`);
    const animation = new GifAnimation();
    animation.LoadFromBytes(await r.arrayBuffer());
    return animation.TotalFrames > 0 ? animation : undefined;
  }

  /** Same sizing as SkiaImage: an unbounded axis follows the GIF aspect. */
  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    const a = this.Animation;
    let w = widthConstraint, h = heightConstraint;
    if (a && a.Width > 0 && a.Height > 0) {
      const aspect = a.Width / a.Height;
      if (!isFinite(w) && isFinite(h)) w = h * aspect;
      else if (!isFinite(h) && isFinite(w)) h = w / aspect;
    }
    if (!isFinite(w) || !isFinite(h)) return ScaledSize.FromPixels(0, 0, scale);
    return ScaledSize.FromPixels(w, h, scale);
  }

  protected override RenderFrame(ctx: DrawingContext): void {
    const img = this.Animation?.Frame;
    if (!img) return;
    const CK = Super.CK;
    const dest = ctx.Destination;
    const scaled = SkiaImage.RescaleAspect(img.width(), img.height(), dest, this.Aspect);
    const display = SkiaImage.CalculateDisplayRect(dest, img.width() * scaled.X, img.height() * scaled.Y, this.HorizontalAlignment, this.VerticalAlignment);
    const canvas = ctx.Context.Canvas;
    const saved = canvas.save();
    canvas.clipRect(CK.LTRBRect(dest.Left, dest.Top, dest.Right, dest.Bottom), CK.ClipOp.Intersect, true);
    const paint = new CK.Paint();
    paint.setAntiAlias(true);
    canvas.drawImageRectOptions(img, CK.LTRBRect(0, 0, img.width(), img.height()), CK.LTRBRect(display.Left, display.Top, display.Right, display.Bottom), CK.FilterMode.Linear, CK.MipmapMode.Linear, paint);
    paint.delete();
    canvas.restoreToCount(saved);
  }

  protected override OnDisposing(): void {
    this.loadGeneration++;
    super.OnDisposing();
    this.Animation?.Dispose();
    this.Animation = undefined;
  }
}

export type { SKRect };
