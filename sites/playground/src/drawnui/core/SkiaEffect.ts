import type { Image, RuntimeEffect, Shader } from "canvaskit-wasm";
import type { DrawingContext, SkiaControl } from "./SkiaControl";
import { SkiaValueAnimator } from "./Animators";
import { Easing } from "./Easing";
import { SkiaImageManager } from "./SkiaImageManager";
import { Super } from "./Super";
import { type ShaderTileMode, SKRect, Thickness } from "./Types";

/** C# PostRendererEffectUseBackgroud: how a post-renderer sources its input texture. */
export type PostRendererEffectUseBackground = "Always" | "Once" | "Never";
export type SkiaFilterMode = "Linear" | "Nearest";
export type SkiaMipmapMode = "None" | "Nearest" | "Linear";

/**
 * C# CachedTexture: a control's rasterized output and the canvas rect it was rasterized over. Texel (0,0) sits at the
 * top-left of Bounds (a cache image); a whole-surface snapshot sets `Origin` = the canvas point its texel (0,0) is at,
 * so the texture shader is translated accordingly. Shaders sample textures in texel space (fragCoord - iOffset).
 */
export interface CachedTexture { Image: Image; Bounds: SKRect; Origin?: { X: number; Y: number } }

/**
 * Mirrors DrawnUi SkiaEffect: something attached to a control through `VisualEffects`. `Update()` re-renders the
 * parent (C# invalidates the parent's cache and redraws), `GetEffectMargin` reports what it paints outside the box.
 */
export class SkiaEffect {
  /** Set by Attach/Dettach. */
  Parent?: SkiaControl;
  Tag?: string;

  Attach(parent: SkiaControl): void { this.Parent = parent; }
  Dettach(): void { this.Parent = undefined; }

  /** Cached resources must be dropped here, not on the next draw (C# contract). */
  protected OnDisposing(): void {}
  Dispose(): void { this.OnDisposing(); this.Parent = undefined; }

  /** Effect parameters changed: the parent's cache is re-recorded and the composition redrawn (C# Parent.Update). */
  Update(): void {
    const p = this.Parent;
    if (!p) return;
    p.InvalidateEffectsMargin();
    p.InvalidateCache();
    p.RepaintComposition();
  }

  /** Extra space in PIXELS painted beyond the parent's DrawingRect. */
  GetEffectMargin(_scale: number): Thickness { return Thickness.Zero; }

  get NeedApply(): boolean { return this.Parent !== undefined; }
}

/** C# ISkiaGestureProcessor: an effect that receives the parent's gestures before its children; return non-null to consume. */
export interface ISkiaGestureProcessor {
  ProcessGestures(args: import("./Gestures").SkiaGesturesParameters, apply: import("./Gestures").GestureEventProcessingInfo): SkiaControl | null;
}

/** C# IPostRendererEffect: draws over the parent's output after it was painted / from its cache. */
export interface IPostRendererEffect {
  Render(ctx: DrawingContext): void;
  UseBackground: PostRendererEffectUseBackground;
  AquiredBackground: boolean;
  NeedApply: boolean;
}

export function IsPostRendererEffect(e: SkiaEffect): e is SkiaEffect & IPostRendererEffect {
  return typeof (e as unknown as IPostRendererEffect).Render === "function" && "UseBackground" in e;
}

/** Runtime effects compiled once per source text (C# SkSl cache per resource name). */
const compiledCache = new Map<string, RuntimeEffect>();
/** Shader files fetched once per url. */
const sourceCache = new Map<string, Promise<string>>();

interface UniformSlot { slot: number; count: number }

/**
 * Mirrors DrawnUi SkiaShaderEffect: an SkSL fragment shader (`ShaderSource` url or inline `ShaderCode`, optionally
 * wrapped by a `ShaderTemplate` at `//script-goes-here`) drawn over the parent control with the Shadertoy-style
 * uniforms `iResolution`, `iImageResolution`, `iTime`, `iOffset`, `iMouse` and the parent's own output as the
 * `iImage1` texture (`UseBackground` Always / Once / Never, `AutoCreateInputTexture`). Extra uniforms through
 * `SetUniform`, extra textures by overriding `CreateTexturesUniforms`. Uniforms not declared by the shader are
 * skipped, declared texture children that are not supplied sample transparent.
 */
export class SkiaShaderEffect extends SkiaEffect implements IPostRendererEffect {
  /** Url of the .sksl file (fetched once, compiled once per text). */
  get ShaderSource(): string { return this.shaderSource; }
  set ShaderSource(v: string) { if (this.shaderSource !== v) { this.shaderSource = v; this.ApplyShaderSource(); } }
  /** Inline SkSL, used when ShaderSource is not set. */
  get ShaderCode(): string { return this.shaderCode; }
  set ShaderCode(v: string) { if (this.shaderCode !== v) { this.shaderCode = v; this.ApplyShaderSource(); } }
  /** Url of a template .sksl whose `//script-goes-here` is replaced by the shader. */
  get ShaderTemplate(): string { return this.shaderTemplate; }
  set ShaderTemplate(v: string) { if (this.shaderTemplate !== v) { this.shaderTemplate = v; this.ApplyShaderSource(); } }
  /** Snapshot the surface being drawn (true) or the on-screen one. */
  UseContext = true;
  /** Create the `iImage1` texture from the parent's output; false for output-only shaders. */
  AutoCreateInputTexture = true;
  UseBackground: PostRendererEffectUseBackground = "Always";
  AquiredBackground = false;
  BlendMode = "SrcOver";
  FilterMode: SkiaFilterMode = "Linear";
  MipmapMode: SkiaMipmapMode = "None";
  /** Tile mode of the input textures. */
  TileMode: ShaderTileMode = "Clamp";
  /** Set every Render from the frame time (seconds); can be driven manually. */
  TimeSeconds = 0;
  /** Shadertoy iMouse.xy, pixels relative to the control. */
  MouseCurrent = { X: 0, Y: 0 };
  /** Shadertoy iMouse.zw: where a drag started, zero = not dragging. */
  MouseInitial = { X: 0, Y: 0 };
  /** Custom uniforms by name (float arrays sized like the SkSL type), merged on top of the standard ones. */
  readonly Uniforms = new Map<string, number[]>();
  /** Compilation errors land here; without a handler they are logged. */
  OnCompilationError?: (effect: SkiaShaderEffect, error: string) => void;
  /** Code as loaded/assembled for the last compile. */
  LoadedCode = "";

  private shaderSource = "";
  private shaderCode = "";
  private shaderTemplate = "";
  private compiled?: RuntimeEffect;
  private compiledKey = "";
  private uniformSlots = new Map<string, UniformSlot>();
  private uniformFloats = 0;
  private childNames: string[] = [];
  private hasNewShader = true;
  private compileFailed = false;
  private loading?: Promise<void>;
  private loadedSource?: { url: string; code: string };
  private loadedTemplate?: { url: string; code: string };
  private textureShader?: { image: Image; key: string; shader: Shader };
  private frozen?: CachedTexture;
  private frozenOwned = false;
  private paint?: import("canvaskit-wasm").Paint;

  get IsCompiled(): boolean { return this.compiled !== undefined; }
  /** C# NeedApply && IsCompiled; the compile itself is attempted here (sync once the source is loaded) so a fresh effect gets its first chance before the render decides between the plain blit and the shader. */
  override get NeedApply(): boolean { return super.NeedApply && this.EnsureCompiled(); }

  /** Loads (async, once) and compiles (sync) when needed; true when a compiled effect exists. */
  EnsureCompiled(): boolean {
    if (!this.EnsureLoaded()) return false;
    if ((this.hasNewShader && !this.compileFailed) || (!this.compiled && !this.compileFailed)) {
      this.hasNewShader = false;
      this.CompileShader();
    }
    return this.compiled !== undefined;
  }

  // ---- uniforms API (C# SetUniform overloads) ----
  SetUniform(name: string, ...values: number[]): this {
    if (name && values.length > 0) { this.Uniforms.set(name, values); this.Update(); }
    return this;
  }

  ReleaseFrozenSnapshot(): void {
    if (this.frozenOwned && this.frozen) this.DisposeImage(this.frozen.Image);
    this.frozen = undefined;
    this.frozenOwned = false;
  }

  private DisposeImage(image: Image): void {
    const sv = this.Parent?.Superview;
    if (sv) sv.DisposeObject({ Dispose: () => image.delete() }); else image.delete();
  }

  protected ApplyShaderSource(): void {
    this.hasNewShader = true;
    this.compileFailed = false;
    this.Update();
  }

  // ---- textures ----
  /** Snapshot of what the parent painted (C# CreateSnapshot): the surface the pixels are on, mapped through its origin. */
  protected CreateSnapshot(ctx: DrawingContext, destination: SKRect): CachedTexture | undefined {
    const surface = this.UseContext ? ctx.Context.Surface : this.Parent?.Superview?.Surface;
    if (!surface) return undefined;
    surface.flush();
    const origin = this.UseContext ? ctx.Context.Origin : undefined;
    const ox = origin?.X ?? 0, oy = origin?.Y ?? 0;
    // whole-surface snapshot: a bounded snapshot of a GPU surface is not origin-safe in CanvasKit; the texture shader
    // gets a local translation so its texel (0,0) is the destination's top-left
    const image = surface.makeImageSnapshot();
    if (!image) return undefined;
    return { Image: image, Bounds: destination, Origin: { X: ox, Y: oy } };
  }

  /** C# GetPrimaryTexture: the parent's cache, a frozen snapshot (Once) or nothing (Never). */
  protected GetPrimaryTexture(_ctx: DrawingContext, _destination: SKRect): CachedTexture | undefined {
    switch (this.UseBackground) {
      case "Never": return undefined;
      case "Once":
        if (!this.AquiredBackground || !this.frozen) {
          this.ReleaseFrozenSnapshot();
          let snapshot = this.Parent?.CachedImage;
          let owned = false;
          if (!snapshot && this.AutoCreateInputTexture) { snapshot = this.CreateSnapshot(_ctx, _destination); owned = true; }
          if (snapshot) { this.frozen = snapshot; this.frozenOwned = owned; this.AquiredBackground = true; }
        }
        return this.frozen;
      default:
        return this.Parent?.CachedImage;
    }
  }

  /** Image → shader with the effect's sampling, translated so texel (0,0) = bounds top-left; cached per image. */
  protected CreateTextureShader(texture: CachedTexture): Shader {
    const CK = Super.CK;
    // texel (0,0) must land on the texture's top-left: a snapshot of a whole surface is shifted by where that top-left is
    const dx = texture.Origin ? texture.Bounds.Left - texture.Origin.X : 0, dy = texture.Origin ? texture.Bounds.Top - texture.Origin.Y : 0;
    const key = `${this.FilterMode}|${this.MipmapMode}|${this.TileMode}|${dx},${dy}`;
    const c = this.textureShader;
    if (c && c.image === texture.Image && c.key === key) return c.shader;
    c?.shader.delete();
    const tile = (CK.TileMode as unknown as Record<string, import("canvaskit-wasm").TileMode>)[this.TileMode] ?? CK.TileMode.Clamp;
    const fm = this.FilterMode === "Nearest" ? CK.FilterMode.Nearest : CK.FilterMode.Linear;
    const mm = this.MipmapMode === "Linear" ? CK.MipmapMode.Linear : this.MipmapMode === "Nearest" ? CK.MipmapMode.Nearest : CK.MipmapMode.None;
    const local = dx !== 0 || dy !== 0 ? CK.Matrix.translated(-dx, -dy) : undefined;
    const shader = texture.Image.makeShaderOptions(tile, tile, fm, mm, local);
    this.textureShader = { image: texture.Image, key, shader };
    return shader;
  }

  /** Texture children by name (C# CreateTexturesUniforms). Override to add `iImage2` etc. */
  protected CreateTexturesUniforms(_ctx: DrawingContext, _destination: SKRect, primaryTexture?: Shader): Record<string, Shader | undefined> {
    return { iImage1: primaryTexture };
  }

  /** Standard uniforms + `Uniforms` (C# CreateUniforms). Override, call super and add your own with `Set`. */
  protected CreateUniforms(destination: SKRect, textureBounds: SKRect | undefined, values: Float32Array): void {
    this.Set(values, "iResolution", destination.Width, destination.Height);
    this.Set(values, "iImageResolution", destination.Width, destination.Height);
    this.Set(values, "iTime", this.TimeSeconds);
    // iOffset is where the texture starts on screen: shaders sample (fragCoord - iOffset)
    this.Set(values, "iOffset", textureBounds?.Left ?? destination.Left, textureBounds?.Top ?? destination.Top);
    this.Set(values, "iMouse", this.MouseCurrent.X, this.MouseCurrent.Y, this.MouseInitial.X, this.MouseInitial.Y);
    for (const [k, v] of this.Uniforms) this.Set(values, k, ...v);
  }

  /** Writes a uniform by name into the flat float array; undeclared names are skipped like C#. */
  protected Set(values: Float32Array, name: string, ...v: number[]): void {
    const u = this.uniformSlots.get(name);
    if (!u) return;
    const n = Math.min(u.count, v.length);
    for (let i = 0; i < n; i++) values[u.slot + i] = v[i];
  }

  // ---- compilation ----
  private static async FetchText(url: string): Promise<string> {
    let p = sourceCache.get(url);
    if (!p) { p = fetch(url).then((r) => { if (!r.ok) throw new Error(`${r.status} ${url}`); return r.text(); }); sourceCache.set(url, p); }
    return p;
  }

  /** Fetches ShaderSource / ShaderTemplate when needed; true when the code is ready to compile. */
  private EnsureLoaded(): boolean {
    const needSource = this.shaderSource && this.loadedSource?.url !== this.shaderSource;
    const needTemplate = this.shaderTemplate && this.loadedTemplate?.url !== this.shaderTemplate;
    if (!needSource && !needTemplate) return true;
    if (this.loading) return false;
    const src = this.shaderSource, tpl = this.shaderTemplate;
    this.loading = (async () => {
      try {
        if (needSource) this.loadedSource = { url: src, code: await SkiaShaderEffect.FetchText(src) };
        if (needTemplate) this.loadedTemplate = { url: tpl, code: await SkiaShaderEffect.FetchText(tpl) };
        this.hasNewShader = true;
        this.compileFailed = false;
      } catch (e) {
        this.compileFailed = true;
        this.SendError(String(e));
      } finally {
        this.loading = undefined;
        this.Update();
      }
    })();
    return false;
  }

  protected SendError(error: string): void {
    if (this.OnCompilationError) this.OnCompilationError(this, error);
    else console.error(`[SkiaShaderEffect] ${error}`);
  }

  /** The template wrapping the code; subclasses supply a default (ShaderTransitionEffect). */
  protected GetTemplate(): string { return this.loadedTemplate?.code ?? ""; }

  protected CompileShader(): void {
    let code = this.shaderSource ? this.loadedSource?.code ?? "" : this.shaderCode;
    code = code.replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n");
    this.LoadedCode = code;
    const template = this.GetTemplate();
    if (template) code = template.replace("//script-goes-here", code);
    if (this.compiledKey === code && this.compiled) return;
    this.compiled = undefined;
    this.compiledKey = code;
    this.textureShader?.shader.delete(); this.textureShader = undefined;
    if (!code.trim()) return;
    let effect = compiledCache.get(code);
    if (!effect) {
      let error = "";
      effect = Super.CK.RuntimeEffect.Make(code, (e) => { error = e; }) ?? undefined;
      if (!effect) { this.compileFailed = true; this.SendError(error || "shader compilation failed"); return; }
      compiledCache.set(code, effect);
    }
    this.compiled = effect;
    // uniform layout: flat floats in declaration order; a uniform spans up to the next slot (arrays included)
    const count = effect.getUniformCount();
    const slots: { name: string; slot: number }[] = [];
    for (let i = 0; i < count; i++) slots.push({ name: effect.getUniformName(i), slot: effect.getUniform(i).slot });
    this.uniformFloats = effect.getUniformFloatCount();
    this.uniformSlots = new Map();
    slots.forEach((s, i) => this.uniformSlots.set(s.name, { slot: s.slot, count: (i + 1 < slots.length ? slots[i + 1].slot : this.uniformFloats) - s.slot }));
    this.childNames = [...code.matchAll(/uniform\s+shader\s+(\w+)\s*;/g)].map((m) => m[1]);
  }

  /** Builds this frame's shader (C# CreateShader). */
  CreateShader(ctx: DrawingContext, source: CachedTexture | undefined): { shader: Shader; children: Shader[] } | undefined {
    if (!this.EnsureCompiled()) return undefined;
    const effect = this.compiled;
    if (!effect) return undefined;
    const destination = ctx.Destination;
    let owned: Image | undefined;
    if (this.UseBackground !== "Never") {
      if (!source && this.AutoCreateInputTexture) { source = this.CreateSnapshot(ctx, destination); owned = source?.Image; }
      if (!source) return undefined;
    }
    const primary = source ? this.CreateTextureShader(source) : undefined;
    const textures = this.CreateTexturesUniforms(ctx, destination, primary);
    const values = new Float32Array(this.uniformFloats);
    this.CreateUniforms(destination, source?.Bounds, values);
    const CK = Super.CK;
    const temp: Shader[] = [];
    const children = this.childNames.map((n) => {
      const s = textures[n];
      if (s) return s;
      const empty = CK.Shader.MakeColor(CK.TRANSPARENT, CK.ColorSpace.SRGB); temp.push(empty); return empty;
    });
    const shader = effect.makeShaderWithChildren(values, children);
    if (owned) this.DisposeImage(owned);
    return { shader, children: temp };
  }

  /** IPostRendererEffect: draws the shader over ctx.Destination (C# Render). */
  Render(ctx: DrawingContext): void {
    this.TimeSeconds = performance.now() / 1000;
    const texture = this.GetPrimaryTexture(ctx, ctx.Destination);
    const built = this.CreateShader(ctx, texture);
    if (!built) return;
    const CK = Super.CK;
    const paint = this.paint ?? (this.paint = new CK.Paint());
    paint.setBlendMode((CK.BlendMode as unknown as Record<string, import("canvaskit-wasm").BlendMode>)[this.BlendMode] ?? CK.BlendMode.SrcOver);
    paint.setShader(built.shader);
    const d = ctx.Destination;
    ctx.Context.Canvas.drawRect(CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), paint);
    paint.setShader(null);
    built.shader.delete();
    for (const t of built.children) t.delete();
  }

  protected override OnDisposing(): void {
    this.textureShader?.shader.delete(); this.textureShader = undefined;
    this.ReleaseFrozenSnapshot();
    this.paint?.delete(); this.paint = undefined;
    this.compiled = undefined; // compiled effects stay in the shared cache
    super.OnDisposing();
  }
}

/**
 * Mirrors DrawnUi ShaderDoubleTexturesEffect: two input textures. `iImage1` comes from `ControlFrom`'s cache or a
 * `PrimarySource` image, `iImage2` from `ControlTo`'s cache or a `SecondarySource` image (both resized to the parent
 * box like C#), else from the parent as usual.
 */
export class ShaderDoubleTexturesEffect extends SkiaShaderEffect {
  get ControlFrom(): SkiaControl | undefined { return this.controlFrom; }
  set ControlFrom(v: SkiaControl | undefined) { this.controlFrom = v; }
  get ControlTo(): SkiaControl | undefined { return this.controlTo; }
  set ControlTo(v: SkiaControl | undefined) { if (this.controlTo !== v) { this.controlTo = v; this.secondary?.delete(); this.secondary = undefined; } }
  get PrimarySource(): string { return this.primarySource; }
  set PrimarySource(v: string) { if (this.primarySource !== v) { this.primarySource = v; this.primaryImage = undefined; this.LoadTexture(v, (i) => { this.primaryImage = i; this.Update(); }); } }
  get SecondarySource(): string { return this.secondarySource; }
  set SecondarySource(v: string) { if (this.secondarySource !== v) { this.secondarySource = v; this.secondaryImage = undefined; this.LoadTexture(v, (i) => { this.secondaryImage = i; this.secondary?.delete(); this.secondary = undefined; this.Update(); }); } }

  private controlFrom?: SkiaControl;
  private controlTo?: SkiaControl;
  private primarySource = "";
  private secondarySource = "";
  private primaryImage?: Image;
  private secondaryImage?: Image;
  private resizedPrimary?: { source: Image; w: number; h: number; image: Image };
  private resizedSecondary?: { source: Image; w: number; h: number; image: Image };
  private secondary?: Shader;

  private LoadTexture(url: string, done: (image: Image) => void): void {
    if (!url) return;
    SkiaImageManager.Instance.LoadImageAsync(url).then((i) => { if (url === this.primarySource || url === this.secondarySource) done(i); }).catch((e) => this.SendError(String(e)));
  }

  /** C# Resize*LoadedBitmap: the file texture spans the parent box. */
  private Resized(ctx: DrawingContext, source: Image, slot: "p" | "s"): Image | undefined {
    const d = this.Parent?.DrawingRect;
    if (!d || d.Width <= 0 || d.Height <= 0) return undefined;
    const w = Math.round(d.Width), h = Math.round(d.Height);
    const cur = slot === "p" ? this.resizedPrimary : this.resizedSecondary;
    if (cur && cur.source === source && cur.w === w && cur.h === h) return cur.image;
    const main = ctx.Context.Surface;
    if (!main) return undefined;
    const surface = main.makeSurface({ ...main.imageInfo(), width: w, height: h });
    if (!surface) return undefined;
    const CK = Super.CK;
    const canvas = surface.getCanvas();
    canvas.clear(CK.TRANSPARENT);
    canvas.drawImageRectOptions(source, CK.XYWHRect(0, 0, source.width(), source.height()), CK.XYWHRect(0, 0, w, h), CK.FilterMode.Linear, CK.MipmapMode.Linear, null);
    const image = surface.makeImageSnapshot();
    surface.delete();
    cur?.image.delete();
    const entry = { source, w, h, image };
    if (slot === "p") this.resizedPrimary = entry; else this.resizedSecondary = entry;
    if (slot === "s") { this.secondary?.delete(); this.secondary = undefined; }
    return image;
  }

  protected override GetPrimaryTexture(ctx: DrawingContext, destination: SKRect): CachedTexture | undefined {
    if (this.controlFrom) return this.controlFrom.CachedImage;
    if (this.primarySource) {
      const img = this.primaryImage ? this.Resized(ctx, this.primaryImage, "p") : undefined;
      return img ? { Image: img, Bounds: destination } : undefined;
    }
    return super.GetPrimaryTexture(ctx, destination);
  }

  /** C# GetSecondaryTexture: ControlTo's cache image or the resized SecondarySource, as a clamped shader. */
  protected GetSecondaryTexture(ctx: DrawingContext): Shader | undefined {
    const CK = Super.CK;
    let image: Image | undefined;
    if (this.controlTo) image = this.controlTo.CachedImage?.Image;
    else if (this.secondaryImage) image = this.Resized(ctx, this.secondaryImage, "s");
    if (!image) return undefined;
    if (this.secondary && this.secondaryImageRef === image) return this.secondary;
    this.secondary?.delete();
    this.secondary = image.makeShaderOptions(CK.TileMode.Clamp, CK.TileMode.Clamp, CK.FilterMode.Linear, CK.MipmapMode.None);
    this.secondaryImageRef = image;
    return this.secondary;
  }
  private secondaryImageRef?: Image;

  protected override CreateTexturesUniforms(ctx: DrawingContext, destination: SKRect, primaryTexture?: Shader): Record<string, Shader | undefined> {
    return { ...super.CreateTexturesUniforms(ctx, destination, primaryTexture), iImage2: this.GetSecondaryTexture(ctx) };
  }

  protected override OnDisposing(): void {
    this.secondary?.delete(); this.secondary = undefined;
    this.resizedPrimary?.image.delete(); this.resizedPrimary = undefined;
    this.resizedSecondary?.image.delete(); this.resizedSecondary = undefined;
    this.controlFrom = undefined; this.controlTo = undefined;
    super.OnDisposing();
  }
}

/**
 * Mirrors DrawnUi ShaderTransitionEffect: `iImage1` = from, `iImage2` = to, plus `progress` (0..1) and `ratio`
 * (width / height) uniforms; without a ShaderTemplate the gl-transitions adapter `DefaultTemplate` wraps a
 * `transition(vec2 uv)` function using `getFromColor` / `getToColor`.
 */
export class ShaderTransitionEffect extends ShaderDoubleTexturesEffect {
  static readonly DefaultTemplate = `
uniform float ratio; // width / height
uniform float progress; // 0.0 - 1.0
uniform shader iImage1; // Texture
uniform shader iImage2; // Texture for backside
uniform float2 iOffset; // Top-left corner of DrawingRect
uniform float2 iResolution; // Viewport resolution (pixels)
uniform float2 iImageResolution; // iImage1 resolution (pixels)
uniform float  iTime; // Shader playback time (s)
uniform float4 iMouse; // Mouse drag pos=.xy Click pos=.zw (pixels)

//In GLSL, the texture coordinate origin is at the bottom-left corner,
//whereas in SKSL the origin is at the top-left corner.

vec4 getFromColor(vec2 uv) {
    vec2 adjustedUV = float2(uv.x, 1.0 - uv.y) * iImageResolution;
    return iImage1.eval(adjustedUV);
}

vec4 getToColor(vec2 uv) {
    vec2 adjustedUV = float2(uv.x, 1.0 - uv.y) * iImageResolution;
    return iImage2.eval(adjustedUV);
}

//script-goes-here
`;

  /** 0 = fully ControlFrom, 1 = fully ControlTo; call Update() after changing. */
  Progress = 0;

  protected override GetTemplate(): string { return super.GetTemplate() || ShaderTransitionEffect.DefaultTemplate; }

  protected override CreateUniforms(destination: SKRect, textureBounds: SKRect | undefined, values: Float32Array): void {
    super.CreateUniforms(destination, textureBounds, values);
    this.Set(values, "progress", this.Progress);
    this.Set(values, "ratio", destination.Height > 0 ? destination.Width / destination.Height : 1);
  }
}

/**
 * Mirrors DrawnUi AnimatedShaderEffect: a `progress` 0→1 uniform driven by an animator over `DurationMs`
 * (2500) plus `iCenter` (normalized). `Play()` restarts, `Completed` fires when it finishes, `Stop()` is silent.
 */
export class AnimatedShaderEffect extends SkiaShaderEffect {
  Center = { X: 0.5, Y: 0.5 };
  DurationMs = 2500;
  Progress = 0;
  Completed?: (effect: AnimatedShaderEffect) => void;
  protected Animator?: SkiaValueAnimator;

  Play(): void {
    this.Animator?.Stop();
    const parent = this.Parent;
    if (!parent) return;
    if (!this.Animator) {
      const a = new SkiaValueAnimator(parent);
      a.OnUpdated = (v) => { this.Progress = v; this.Update(); };
      a.Finished = () => this.Completed?.(this);
      this.Animator = a;
    }
    this.Progress = 0;
    this.AquiredBackground = false; // Once mode re-captures on the next render
    const a = this.Animator;
    a.mMinValue = 0; a.mMaxValue = 1; a.Speed = this.DurationMs; a.Easing = Easing.Linear; a.Repeat = 0;
    a.Start();
    this.Update();
  }

  Stop(): void { this.Animator?.Stop(); }

  protected override CreateUniforms(destination: SKRect, textureBounds: SKRect | undefined, values: Float32Array): void {
    super.CreateUniforms(destination, textureBounds, values);
    this.Set(values, "progress", this.Progress);
    this.Set(values, "iCenter", this.Center.X, this.Center.Y);
  }

  protected override OnDisposing(): void { this.Animator?.Stop(); this.Animator = undefined; super.OnDisposing(); }
}
