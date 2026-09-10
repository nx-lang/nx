import type { ManagedSkottieAnimation } from "canvaskit-wasm";
import type { DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type Color, Colors } from "../core/Types";
import { AnimatedFramesRenderer } from "./AnimatedFramesRenderer";

/**
 * Mirrors DrawnUi SkiaLottie on CanvasKit Skottie: `Source` (URL / app path of the Lottie JSON), `AutoPlay`, `Repeat`,
 * `SpeedRatio`, `DefaultFrame` / `DefaultFrameWhenOn` + `IsOn` (animated toggles), `StopAtCurrentFrame`,
 * `ColorTint` / `Colors` (replace the animation colors in the JSON like C# ApplyTint), `GoToStart` / `GoToEnd`.
 * Requires the CanvasKit "full" build (Skottie module).
 */
export class SkiaLottie extends AnimatedFramesRenderer {
  /** JSON text per source, so the same file is fetched once (C# CachedAnimations). */
  static readonly CachedAnimations = new Map<string, string>();
  private static readonly inFlight = new Map<string, Promise<string>>();

  /** Stop() / Finished leave the animation where it is instead of seeking to DefaultFrame; Start() resumes from there. */
  StopAtCurrentFrame = false;
  /** Changing IsOn while stopped seeks to the matching default frame. */
  ApplyIsOnWhenNotPlaying = true;
  /** Hook to modify the JSON before it is parsed (C# ProcessJson). */
  ProcessJson?: (json: string) => string;
  Success?: (sender: SkiaLottie, source: string) => void;
  Error?: (sender: SkiaLottie, error: Error) => void;

  private source = "";
  private isOn = false;
  private defaultFrameWhenOn = 0;
  private colorTint: Color = Colors.Transparent;
  private colors: Color[] = [];
  private loadGeneration = 0;
  private needSeek = false;
  private inPoint = 0;
  private outPoint = 0;

  /** Current Skottie animation. */
  Animation?: ManagedSkottieAnimation;

  constructor() {
    super();
    this.UseCache = "ImageDoubleBuffered";
  }

  get Source(): string { return this.source; }
  set Source(v: string) { if (this.source !== v) { this.source = v; this.ReloadSource(); } }

  /** Toggle state: when stopped shows DefaultFrameWhenOn instead of DefaultFrame. */
  get IsOn(): boolean { return this.isOn; }
  set IsOn(v: boolean) {
    if (this.isOn === v) return;
    this.isOn = v;
    if (!this.Animator) return;
    if (this.IsPlaying || !v) this.Stop();
    else if (this.ApplyIsOnWhenNotPlaying) this.SeekToDefaultFrame();
  }
  get DefaultFrameWhenOn(): number { return this.defaultFrameWhenOn; }
  set DefaultFrameWhenOn(v: number) { if (this.defaultFrameWhenOn !== v) { this.defaultFrameWhenOn = v; if (!this.IsPlaying && this.Animator && this.isOn) this.Seek(v); } }
  /** Single tint replacing every color of the animation (alpha preserved). */
  get ColorTint(): Color { return this.colorTint; }
  set ColorTint(v: Color) { if (this.colorTint !== v) { this.colorTint = v; this.ReloadSource(); } }
  /** Per-distinct-color replacements in order of appearance; the last one covers the rest. */
  get Colors(): Color[] { return this.colors; }
  set Colors(v: Color[]) { if (this.colors !== v) { this.colors = v ?? []; this.ReloadSource(); } }

  /** Total frames of the loaded animation. */
  get TotalFrames(): number { return this.outPoint - this.inPoint; }

  // ---- playback ----
  override Stop(): void {
    super.Stop();
    if (!this.StopAtCurrentFrame) this.SeekToDefaultFrame();
  }

  protected override OnFinished(): void {
    super.OnFinished();
    if (!this.StopAtCurrentFrame) this.SeekToDefaultFrame();
  }

  SeekToDefaultFrame(): void {
    if (this.Animator) this.Seek(this.isOn ? this.defaultFrameWhenOn : this.DefaultFrame);
    else this.needSeek = true;
  }

  protected override OnLayoutChanged(): void {
    super.OnLayoutChanged();
    if (this.needSeek) { this.needSeek = false; if (!this.IsPlaying) this.SeekToDefaultFrame(); }
  }

  protected override OnAnimatorSeeking(frame: number): void {
    if (this.Animation) {
      if (frame < 0) frame = this.outPoint;
      this.Animation.seekFrame(frame);
      this.InvalidateCache();
      this.RepaintComposition();
    }
  }

  protected override ApplyDefaultFrame(): void {
    if (!this.IsPlaying && this.Animator && !this.isOn) this.Seek(this.DefaultFrame);
  }

  protected override OnAnimatorUpdated(value: number): void {
    if (!this.Animation) return;
    this.Animation.seekFrame(value);
    this.InvalidateCache(); // own cache re-records, ancestors composite again; no remeasure (C# Update())
    this.RepaintComposition();
  }

  protected override ApplySpeed(): void {
    if (!this.Animation || !this.Animator) return;
    const durationMs = this.Animation.duration() * 1000;
    this.Animator.Speed = this.SpeedRatio < 1 ? durationMs * (1 + this.SpeedRatio) : durationMs / this.SpeedRatio;
  }

  protected override OnAnimatorInitializing(): void {
    if (!this.Animation || !this.Animator) return;
    this.ApplySpeed();
    this.Animator.mValue = this.inPoint;
    this.Animator.mMinValue = this.inPoint;
    this.Animator.mMaxValue = this.outPoint;
  }

  protected override CheckCanStartAnimator(): boolean { return !!this.Animation; }

  protected override OnAnimatorStarting(): void {
    if (!this.StopAtCurrentFrame && this.Animator) this.Animator.mValue = this.inPoint;
  }

  GoToStart(): void { if (this.Animation) { this.Animation.seek(0); this.InvalidateCache(); this.RepaintComposition(); } }
  GoToEnd(): void { if (this.Animation) { this.Animation.seek(1); this.InvalidateCache(); this.RepaintComposition(); } }

  protected override RenderFrame(ctx: DrawingContext): void {
    const a = this.Animation;
    if (!a) return;
    const r = ctx.Destination;
    a.render(ctx.Context.Canvas, Super.CK.LTRBRect(r.Left, r.Top, r.Right, r.Bottom));
  }

  // ---- loading ----
  ReloadSource(): void {
    if (!this.source) return;
    const generation = ++this.loadGeneration;
    const source = this.source;
    this.LoadSource(source).then(
      (animation) => { if (generation !== this.loadGeneration) return; if (animation) { this.Success?.(this, source); this.SetAnimation(animation, true); } else this.Error?.(this, new Error(`Failed to load source ${source}`)); },
      (e: Error) => { if (generation === this.loadGeneration) this.Error?.(this, e); },
    );
  }

  private static FetchJson(source: string): Promise<string> {
    const cached = SkiaLottie.CachedAnimations.get(source);
    if (cached) return Promise.resolve(cached);
    let p = SkiaLottie.inFlight.get(source);
    if (!p) {
      p = fetch(source).then(async (r) => { if (!r.ok) throw new Error(`${r.status} ${source}`); const json = await r.text(); SkiaLottie.CachedAnimations.set(source, json); return json; })
        .finally(() => SkiaLottie.inFlight.delete(source));
      SkiaLottie.inFlight.set(source, p);
    }
    return p;
  }

  /** Loads (does not apply) the animation for a source; C# LoadSource. */
  async LoadSource(source: string): Promise<ManagedSkottieAnimation | undefined> {
    if (!source) return undefined;
    return this.CreateAnimation(await SkiaLottie.FetchJson(source));
  }

  /** Builds a Skottie animation from JSON, with the tint / colors replacement applied (C# CreateAnimation). */
  CreateAnimation(json: string): ManagedSkottieAnimation | undefined {
    if (!json) return undefined;
    if (this.colors.length > 0) json = SkiaLottie.ApplyTint(json, this.colors);
    else if (Super.ParseColor(this.colorTint)[3] > 0) json = SkiaLottie.ApplyTint(json, [this.colorTint]);
    if (this.ProcessJson) json = this.ProcessJson(json);
    const make = (Super.CK as unknown as { MakeManagedAnimation?: (j: string) => ManagedSkottieAnimation }).MakeManagedAnimation;
    if (!make) { console.error("[SkiaLottie] CanvasKit build has no Skottie (use canvaskit-wasm/bin/full)"); return undefined; }
    try { return make.call(Super.CK, json); } catch (e) { console.error("[SkiaLottie] failed to parse animation", e); return undefined; }
  }

  SetAnimation(animation: ManagedSkottieAnimation, disposePrevious: boolean): void {
    if (!animation || animation === this.Animation) return;
    const wasPlaying = this.IsPlaying;
    const kill = this.Animation;
    if (wasPlaying) this.Stop();
    this.Animation = animation;
    this.inPoint = 0;
    this.outPoint = Math.round(animation.duration() * animation.fps());
    this.InitializeAnimator(); // AutoPlay applied inside
    this.OnAnimatorSeeking(this.isOn ? this.defaultFrameWhenOn : this.DefaultFrame);
    if (wasPlaying && !this.IsPlaying) this.Start();
    if (kill && disposePrevious) kill.delete();
    this.Update();
  }

  protected override OnDisposing(): void {
    this.loadGeneration++;
    super.OnDisposing();
    this.Animation?.delete();
    this.Animation = undefined;
  }

  // ---- C# ApplyTint: every distinct color of the JSON maps to the next tint, alpha preserved ----
  static ApplyTint(json: string, tints: Color[]): string {
    const doc = JSON.parse(json);
    const found: { set: (c: number[]) => void; color: number[] }[] = [];
    const key = (c: number[]) => c.slice(0, 3).map((v) => v.toFixed(4)).join(",");
    const walk = (node: unknown): void => {
      if (Array.isArray(node)) { for (const item of node) walk(item); return; }
      if (!node || typeof node !== "object") return;
      const obj = node as Record<string, unknown>;
      for (const name of Object.keys(obj)) {
        const value = obj[name];
        if (name === "k" && SkiaLottie.IsColorArray(value)) {
          const arr = value as number[];
          found.push({ color: [arr[0], arr[1], arr[2], arr.length > 3 ? arr[3] : 1], set: (c) => { obj[name] = arr.length > 3 ? c : c.slice(0, 3); } });
        } else if ((name === "sc" || name === "fc") && typeof value === "string") {
          const c = Super.ParseColor(value);
          found.push({ color: [c[0], c[1], c[2], c[3]], set: (nc) => { obj[name] = SkiaLottie.ToHex(nc); } });
        }
        walk(value);
      }
    };
    walk(doc);
    const mapping = new Map<string, number[]>();
    let index = -1;
    let tint = Super.ParseColor(Colors.Black);
    for (const f of found) {
      const k = key(f.color);
      if (mapping.has(k)) continue;
      index++;
      if (index < tints.length) tint = Super.ParseColor(tints[index]);
      mapping.set(k, [tint[0], tint[1], tint[2]]);
    }
    for (const f of found) { const m = mapping.get(key(f.color))!; f.set([m[0], m[1], m[2], f.color[3]]); }
    return JSON.stringify(doc);
  }

  private static IsColorArray(v: unknown): boolean {
    return Array.isArray(v) && (v.length === 3 || v.length === 4) && v.every((x) => typeof x === "number" && x >= 0 && x <= 1);
  }

  private static ToHex(c: number[]): string {
    const h = (v: number) => Math.round(v * 255).toString(16).padStart(2, "0").toUpperCase();
    return `#${h(c[3])}${h(c[0])}${h(c[1])}${h(c[2])}`;
  }
}
