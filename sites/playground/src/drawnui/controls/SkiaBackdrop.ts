import type { Image } from "canvaskit-wasm";
import type { DrawingContext } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { SkiaImageEffects } from "../core/ImageEffects";
import { SkiaLayout } from "./SkiaLayout";

/**
 * Mirrors DrawnUi SkiaBackdrop: draws what is already painted beneath its box through `Blur` (points, default 5)
 * and `Brightness` (gamma, 1 = unchanged), tinted by `BackgroundColor`; children draw first and are blurred too.
 * The snapshot comes from the surface being drawn (`UseContext`) or the on-screen surface. Not cached (re-blurs
 * on every frame it is drawn in; inside a cached parent it is re-recorded with that parent).
 */
export class SkiaBackdrop extends SkiaLayout {
  Blur = 5;
  Brightness = 1;
  /** Snapshot the surface currently drawn into (true) or the on-screen surface (false). */
  UseContext = true;

  private snapshot?: Image;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.VerticalOptions = "Fill";
  }

  get HasEffects(): boolean { return this.Blur !== 0 || this.Brightness !== 1; }

  /** The last snapshot taken (C# GetImage hands it over: the caller owns it afterwards). */
  GetImage(): Image | undefined { const i = this.snapshot; this.snapshot = undefined; return i; }

  /** Background is painted inside Paint(), after the children and before the snapshot (C# order). */
  protected override PaintBackground(_ctx: DrawingContext): void {}
  protected override PaintsBackgroundWithoutColor(): boolean { return false; }

  protected override Paint(ctx: DrawingContext): void {
    const d = ctx.Destination;
    if (d.Width <= 0 || d.Height <= 0) return;
    const CK = Super.CK;
    const canvas = ctx.Context.Canvas;
    super.Paint(ctx); // children
    if (this.BackgroundColor) { const paint = this.CreateBackgroundPaint(d); canvas.drawRect(CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), paint); paint.delete(); }
    if (!this.HasEffects) return;
    // C# UseContext: snapshot the surface the pixels end up on — the on-screen surface or the nearest Image cache
    // surface (Origin = its top-left in canvas pixels). Content beneath is already there because caches are recorded
    // inline while the parent paints, so this also works while an Operations picture is being recorded.
    const surface = this.UseContext ? ctx.Context.Surface : this.Superview?.Surface;
    if (!surface) return;
    surface.flush();
    const origin = this.UseContext ? ctx.Context.Origin : undefined;
    const ox = origin?.X ?? 0, oy = origin?.Y ?? 0;
    const l = Math.floor(d.Left - ox), t = Math.floor(d.Top - oy), r = Math.ceil(d.Right - ox), b = Math.ceil(d.Bottom - oy);
    // whole-surface snapshot, sub-rect on draw: a bounded snapshot of a GPU surface is not origin-safe in CanvasKit
    const image = surface.makeImageSnapshot();
    if (!image) return;
    const paint = new CK.Paint();
    const blur = this.Blur > 0 ? CK.ImageFilter.MakeBlur(this.Blur * ctx.Scale, this.Blur * ctx.Scale, CK.TileMode.Mirror, null) : null;
    if (blur) paint.setImageFilter(blur);
    const gamma = this.Brightness !== 1 ? SkiaImageEffects.Gamma(this.Brightness) : null;
    if (gamma) paint.setColorFilter(gamma);
    const saved = canvas.save();
    canvas.clipRect(CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), CK.ClipOp.Intersect, true);
    canvas.drawImageRectOptions(image, CK.LTRBRect(l, t, r, b), CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), CK.FilterMode.Linear, CK.MipmapMode.None, paint);
    canvas.restoreToCount(saved);
    blur?.delete(); gamma?.delete(); paint.delete();
    const kill = this.snapshot;
    this.snapshot = image;
    kill?.delete();
    // What is beneath may change without invalidating this branch (an image that loads later, a sibling animating):
    // stale the ancestors' caches for the NEXT frame, after the current record has finished, without asking for one.
    queueMicrotask(() => { let p = this.Parent; while (p) { p.InvalidateCache(); p = p.Parent; } });
  }

  protected override OnDisposing(): void { this.snapshot?.delete(); this.snapshot = undefined; }
}
