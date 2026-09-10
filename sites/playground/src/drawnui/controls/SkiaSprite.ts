import type { Image } from "canvaskit-wasm";
import type { DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { SkiaImageManager } from "../core/SkiaImageManager";
import { SKRect, ScaledSize } from "../core/Types";
import { AnimatedFramesRenderer } from "./AnimatedFramesRenderer";

/** DrawnUi SpritePlacementConfig: how a frame maps to the sprite's box (units = points). */
export interface SpritePlacementConfig {
  UnitsPerPixel?: number;
  WidthUnits?: number;
  HeightUnits?: number;
  AnchorX?: number;
  AnchorY?: number;
  OffsetXUnits?: number;
  OffsetYUnits?: number;
}

interface FrameContentBounds { Left: number; Top: number; Width: number; Height: number }

/**
 * Mirrors DrawnUi SkiaSprite: plays a spritesheet (`Source`, `Columns` × `Rows` frames, `MaxFrames`, optional
 * `FrameSequence` / `AnimationName`) at `FramesPerSecond` on the frames animator (`AutoPlay`, `Repeat`,
 * `SpeedRatio`, `Start` / `Stop` / `Seek`). Every frame is drawn from the sheet with nearest sampling, AspectFit
 * into the box; transparent frame borders are trimmed like the C# SpriteFrameImage (content bounds scanned
 * once per frame). `ApplyPlacementConfig` places the frame by units-per-pixel / size / anchor / offset.
 */
export class SkiaSprite extends AnimatedFramesRenderer {
  /** Decoded sheets per source (C# CachedSpriteSheets; the images belong to SkiaImageManager too). */
  static readonly CachedSpriteSheets = new Map<string, Image>();
  static readonly RegisteredAnimations = new Map<string, number[]>();
  /** Registers a named frame sequence usable through `AnimationName`. */
  static CreateAnimationSequence(name: string, frameSequence: number[]): number[] { SkiaSprite.RegisteredAnimations.set(name, frameSequence); return frameSequence; }

  private source = "";
  private columns = 1;
  private rows = 1;
  private maxFrames = 0;
  private framesPerSecond = 24;
  private frameSequence?: number[];
  private animationName?: string;
  private currentFrame = 0;
  private loadGeneration = 0;
  private lastColumns = -1;
  private lastRows = -1;
  private contentBounds = new Map<number, FrameContentBounds>();
  private pixels?: Uint8Array;
  private frameRect = { X: 0, Y: 0, Content: { Left: 0, Top: 0, Width: 0, Height: 0 } as FrameContentBounds };

  /** The decoded sheet (C# SpriteSheet). */
  SpriteSheet?: Image;
  FrameWidth = 0;
  FrameHeight = 0;
  TotalFrames = 0;
  DurationMs = 0;
  FrameDurationMs = 0;
  /** Placement (C# Render* fields), see ApplyPlacementConfig. */
  RenderUnitsPerPixel = -1;
  RenderWidthUnits = -1;
  RenderHeightUnits = -1;
  RenderAnchorX = 0.5;
  RenderAnchorY = 0.5;
  RenderOffsetXUnits = 0;
  RenderOffsetYUnits = 0;
  /** Where the current frame was drawn last time, canvas pixels (C# Display.DisplayRect). */
  DisplayRect: SKRect = SKRect.Empty;
  Success?: (sender: SkiaSprite, source: string) => void;
  Error?: (sender: SkiaSprite, error: Error) => void;

  get Source(): string { return this.source; }
  set Source(v: string) { if (this.source !== v) { this.source = v ?? ""; this.ReloadSource(); } }
  get Columns(): number { return this.columns; }
  set Columns(v: number) { if (this.columns !== v) { this.columns = v; this.Update(); } }
  get Rows(): number { return this.rows; }
  set Rows(v: number) { if (this.rows !== v) { this.rows = v; this.Update(); } }
  /** Frames to use from the sheet, 0 = all. */
  get MaxFrames(): number { return this.maxFrames; }
  set MaxFrames(v: number) { if (this.maxFrames !== v) { this.maxFrames = v; this.RecalculateFrames(); this.Update(); } }
  get FramesPerSecond(): number { return this.framesPerSecond; }
  set FramesPerSecond(v: number) { if (this.framesPerSecond !== v) { this.framesPerSecond = v; this.RecalculateFrames(); if (this.Animator) { this.OnAnimatorInitializing(); } } }
  /** Custom order of frames to play instead of 0..TotalFrames-1. */
  get FrameSequence(): number[] | undefined { return this.frameSequence; }
  set FrameSequence(v: number[] | undefined) { if (this.frameSequence !== v) { this.frameSequence = v; this.RecalculateFrames(); this.Update(); } }
  get AnimationName(): string | undefined { return this.animationName; }
  set AnimationName(v: string | undefined) {
    if (this.animationName === v) return;
    this.animationName = v;
    if (v) { const seq = SkiaSprite.RegisteredAnimations.get(v); if (seq) this.FrameSequence = seq; else console.warn(`[SkiaSprite] Animation sequence '${v}' not found`); }
  }
  get CurrentFrame(): number { return this.currentFrame; }
  set CurrentFrame(v: number) { this.SetCurrentFrame(v); }

  ApplyPlacementConfig(placement?: SpritePlacementConfig): void {
    this.RenderUnitsPerPixel = placement?.UnitsPerPixel ?? -1;
    this.RenderWidthUnits = placement?.WidthUnits ?? -1;
    this.RenderHeightUnits = placement?.HeightUnits ?? -1;
    this.RenderAnchorX = placement?.AnchorX ?? 0.5;
    this.RenderAnchorY = placement?.AnchorY ?? 0.5;
    this.RenderOffsetXUnits = placement?.OffsetXUnits ?? 0;
    this.RenderOffsetYUnits = placement?.OffsetYUnits ?? 0;
    this.Update();
  }

  // ---- loading (C# ReloadSource / LoadSourceAsync / SetSpriteSheet) ----
  ReloadSource(): void {
    if (!this.source) return;
    const generation = ++this.loadGeneration;
    const source = this.source;
    SkiaSprite.LoadSourceAsync(source).then(
      (image) => { if (generation !== this.loadGeneration) return; this.SetSpriteSheet(image); this.Success?.(this, source); },
      (e: Error) => { if (generation === this.loadGeneration) this.Error?.(this, e); },
    );
  }

  static async LoadSourceAsync(source: string): Promise<Image> {
    const cached = SkiaSprite.CachedSpriteSheets.get(source);
    if (cached) return cached;
    const image = await SkiaImageManager.Instance.LoadImageAsync(source);
    SkiaSprite.CachedSpriteSheets.set(source, image);
    return image;
  }

  SetSpriteSheet(image: Image): void {
    if (!image || image === this.SpriteSheet || this.IsDisposed) return;
    const wasPlaying = this.IsPlaying;
    if (wasPlaying) this.Stop();
    this.SpriteSheet = image;
    this.pixels = undefined;
    this.contentBounds.clear();
    this.RecalculateFrames();
    if (this.TotalFrames > 0) {
      this.InitializeAnimator(); // AutoPlay applied inside
      this.SetCurrentFrame(this.DefaultFrame);
      if (wasPlaying && !this.IsPlaying) this.Start();
    }
    this.Update();
  }

  protected RecalculateFrames(): void {
    const sheet = this.SpriteSheet;
    if (!sheet) return;
    this.contentBounds.clear();
    const framesX = Math.max(1, this.columns), framesY = Math.max(1, this.rows);
    const inSheet = Math.min(framesX * framesY, this.maxFrames > 0 ? this.maxFrames : Number.MAX_SAFE_INTEGER);
    this.FrameWidth = Math.floor(sheet.width() / framesX);
    this.FrameHeight = Math.floor(sheet.height() / framesY);
    this.TotalFrames = this.frameSequence && this.frameSequence.length > 0 ? this.frameSequence.length : inSheet;
    this.FrameDurationMs = 1000 / (this.framesPerSecond || 24);
    this.DurationMs = this.TotalFrames * this.FrameDurationMs;
    this.lastColumns = this.columns; this.lastRows = this.rows;
  }

  protected GetFrameNumberFromTime(msTime: number): number {
    if (this.DurationMs <= 0) return 0;
    if (msTime < 0) msTime = this.DurationMs + msTime;
    msTime %= this.DurationMs;
    const frame = Math.floor(msTime / this.FrameDurationMs);
    if (this.frameSequence && this.frameSequence.length > 0) return frame % this.frameSequence.length;
    return Math.min(frame, this.TotalFrames - 1);
  }

  protected SetCurrentFrame(frameNumber: number): void {
    if (!this.SpriteSheet || this.TotalFrames === 0) return;
    frameNumber = Math.max(0, Math.min(frameNumber, this.TotalFrames - 1));
    const actual = this.frameSequence && this.frameSequence.length > 0 ? this.frameSequence[Math.min(frameNumber, this.frameSequence.length - 1)] : frameNumber;
    this.currentFrame = frameNumber;
    const framesX = Math.max(1, this.columns);
    const x = (actual % framesX) * this.FrameWidth, y = Math.floor(actual / framesX) * this.FrameHeight;
    this.frameRect = { X: x, Y: y, Content: this.GetFrameContentBounds(actual, x, y) };
    this.InvalidateCache();
    this.RepaintComposition();
  }

  /** Transparent borders of a frame, scanned once (C# ScanFrameContentBounds over the bitmap alpha). */
  private GetFrameContentBounds(frame: number, frameX: number, frameY: number): FrameContentBounds {
    const cached = this.contentBounds.get(frame);
    if (cached) return cached;
    const full = { Left: 0, Top: 0, Width: this.FrameWidth, Height: this.FrameHeight };
    const sheet = this.SpriteSheet;
    if (!sheet || this.FrameWidth <= 0 || this.FrameHeight <= 0) return full;
    if (!this.pixels) {
      const CK = Super.CK;
      const w = sheet.width(), h = sheet.height();
      const px = sheet.readPixels(0, 0, { width: w, height: h, colorType: CK.ColorType.RGBA_8888, alphaType: CK.AlphaType.Unpremul, colorSpace: CK.ColorSpace.SRGB });
      if (!px) { this.contentBounds.set(frame, full); return full; }
      this.pixels = px as Uint8Array;
    }
    const w = sheet.width();
    const maxX = Math.min(frameX + this.FrameWidth, w), maxY = Math.min(frameY + this.FrameHeight, sheet.height());
    let minX = this.FrameWidth, minY = this.FrameHeight, mx = -1, my = -1;
    for (let yy = frameY; yy < maxY; yy++) {
      for (let xx = frameX; xx < maxX; xx++) {
        if (this.pixels[(yy * w + xx) * 4 + 3] === 0) continue;
        const lx = xx - frameX, ly = yy - frameY;
        if (lx < minX) minX = lx; if (ly < minY) minY = ly; if (lx > mx) mx = lx; if (ly > my) my = ly;
      }
    }
    const bounds = mx < minX || my < minY ? full : { Left: minX, Top: minY, Width: mx - minX + 1, Height: my - minY + 1 };
    this.contentBounds.set(frame, bounds);
    return bounds;
  }

  // ---- animator ----
  protected override OnAnimatorUpdated(value: number): void { this.Seek(value); }
  protected override OnAnimatorSeeking(time: number): void {
    if (!this.SpriteSheet) return;
    const frame = this.GetFrameNumberFromTime(time);
    if (frame !== this.currentFrame) this.SetCurrentFrame(frame);
  }
  protected override OnAnimatorInitializing(): void {
    if (!this.SpriteSheet || !this.Animator) return;
    this.ApplySpeed();
    this.Animator.mValue = 0; this.Animator.mMinValue = 0; this.Animator.mMaxValue = this.DurationMs;
  }
  protected override ApplySpeed(): void {
    if (!this.SpriteSheet || !this.Animator) return;
    this.Animator.Speed = this.SpeedRatio < 1 ? this.DurationMs * (1 + this.SpeedRatio) : this.DurationMs / this.SpeedRatio;
  }
  protected override CheckCanStartAnimator(): boolean { return !!this.SpriteSheet && this.TotalFrames > 0; }
  override Start(delayMs = 0): void { if (this.SpriteSheet && this.TotalFrames > 0) super.Start(delayMs); }

  // ---- measure / draw (C# SpriteFrameImage: AspectFit of the FRAME into the box, then the placement config) ----
  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    let w = widthConstraint, h = heightConstraint;
    const fw = this.FrameWidth, fh = this.FrameHeight;
    if (fw > 0 && fh > 0) {
      if (this.RenderUnitsPerPixel > 0) { w = isFinite(w) ? w : fw * this.RenderUnitsPerPixel * scale; h = isFinite(h) ? h : fh * this.RenderUnitsPerPixel * scale; }
      if (!isFinite(w) && isFinite(h)) w = h * fw / fh;
      else if (!isFinite(h) && isFinite(w)) h = w * fh / fw;
    }
    if (!isFinite(w) || !isFinite(h)) return ScaledSize.FromPixels(0, 0, scale);
    return ScaledSize.FromPixels(w, h, scale);
  }

  /** C# ResolveFrameMetrics: source rect of the trimmed content and its display rect inside dest. */
  private ResolveFrameMetrics(dest: SKRect, scale: number): { src: SKRect; display: SKRect } {
    const fw = Math.max(1, this.FrameWidth), fh = Math.max(1, this.FrameHeight);
    const c = this.frameRect.Content;
    const cx = Math.max(0, Math.min(c.Left, fw - 1)), cy = Math.max(0, Math.min(c.Top, fh - 1));
    const cw = c.Width > 0 ? Math.min(c.Width, fw - cx) : fw, ch = c.Height > 0 ? Math.min(c.Height, fh - cy) : fh;
    const fit = Math.min(dest.Width / fw, dest.Height / fh); // AspectFit of the logical frame
    let logicalW = fw * fit, logicalH = fh * fit;
    if (this.RenderUnitsPerPixel > 0) { logicalW = fw * this.RenderUnitsPerPixel * scale; logicalH = fh * this.RenderUnitsPerPixel * scale; }
    if (this.RenderWidthUnits > 0 || this.RenderHeightUnits > 0) {
      if (this.RenderWidthUnits > 0) logicalW = this.RenderWidthUnits * scale;
      if (this.RenderHeightUnits > 0) logicalH = this.RenderHeightUnits * scale;
      if (this.RenderWidthUnits > 0 && this.RenderHeightUnits <= 0) logicalH = logicalW * fh / fw;
      else if (this.RenderHeightUnits > 0 && this.RenderWidthUnits <= 0) logicalW = logicalH * fw / fh;
    }
    const pxX = logicalW / fw, pxY = logicalH / fh;
    const anchorX = dest.Left + dest.Width * this.RenderAnchorX, anchorY = dest.Top + dest.Height * this.RenderAnchorY;
    const left = anchorX - logicalW * this.RenderAnchorX + cx * pxX + Math.round(this.RenderOffsetXUnits * scale);
    const top = anchorY - logicalH * this.RenderAnchorY + cy * pxY + Math.round(this.RenderOffsetYUnits * scale);
    return {
      src: new SKRect(this.frameRect.X + cx, this.frameRect.Y + cy, this.frameRect.X + cx + cw, this.frameRect.Y + cy + ch),
      display: new SKRect(left, top, left + cw * pxX, top + ch * pxY),
    };
  }

  protected override RenderFrame(ctx: DrawingContext): void {
    const sheet = this.SpriteSheet;
    if (!sheet || this.TotalFrames === 0) return;
    if (this.columns !== this.lastColumns || this.rows !== this.lastRows) { this.RecalculateFrames(); this.SetCurrentFrame(this.currentFrame); }
    const CK = Super.CK;
    const { src, display } = this.ResolveFrameMetrics(ctx.Destination, ctx.Scale);
    this.DisplayRect = display;
    const paint = new CK.Paint();
    paint.setAntiAlias(false);
    ctx.Context.Canvas.drawImageRectOptions(sheet, CK.LTRBRect(src.Left, src.Top, src.Right, src.Bottom), CK.LTRBRect(display.Left, display.Top, display.Right, display.Bottom), CK.FilterMode.Nearest, CK.MipmapMode.None, paint);
    paint.delete();
  }

  protected override OnDisposing(): void {
    this.loadGeneration++;
    super.OnDisposing();
    this.SpriteSheet = undefined; // shared with the cache, never deleted here
    this.pixels = undefined;
  }
}
