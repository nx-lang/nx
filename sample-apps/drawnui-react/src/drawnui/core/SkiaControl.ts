import type { Canvas as SkCanvas, Image, Path, SkPicture, Surface } from "canvaskit-wasm";
import { Super } from "./Super";
import { type Color, Colors, type LayoutOptions, SKRect, ScaledSize, type SkiaCacheType, type SkiaGradient, type SkiaTouchAnimation, Thickness } from "./Types";
import { type IOverlayEffect, RippleAnimator, SkiaValueAnimator } from "./Animators";
import { Easing } from "./Easing";
import type { Canvas } from "./Canvas";
import { Aria } from "./Accessibility";
import { ControlTappedEventArgs, GestureEventProcessingInfo, type LockTouch, SKPoint, SkiaGesturesInfo, SkiaGesturesParameters, TouchActionEventArgs } from "./Gestures";

/** Mirrors DrawnUi DrawingContext: ctx.Context.Canvas / Surface, ctx.Destination (pixels), ctx.Scale. */
export interface DrawingContext {
  Context: { Canvas: SkCanvas; Surface?: Surface };
  Destination: SKRect;
  Scale: number;
}

/** Mirrors DrawnUi CachedObject: what a cached control replays instead of repainting. */
export class CachedObject {
  constructor(
    readonly Type: SkiaCacheType,
    /** Rect the content was recorded for, canvas pixels at recording time. */
    readonly Bounds: SKRect,
    readonly Scale: number,
    readonly Picture?: SkPicture,
    readonly Image?: Image,
  ) {}

  /** Replays the cache with its top-left moved to (left, top). */
  Draw(canvas: SkCanvas, left: number, top: number): void {
    const CK = Super.CK;
    if (this.Image) {
      // Nearest sampling: a cached bitmap is blitted 1:1, never resampled.
      canvas.drawImageOptions(this.Image, left, top, CK.FilterMode.Nearest, CK.MipmapMode.None, null);
    } else if (this.Picture) {
      const saved = canvas.save();
      canvas.translate(left - this.Bounds.Left, top - this.Bounds.Top);
      canvas.drawPicture(this.Picture);
      canvas.restoreToCount(saved);
    }
  }

  Dispose(): void {
    this.Picture?.delete();
    this.Image?.delete();
  }
}

/**
 * Base of every drawn control. Same contract as DrawnUi SkiaControl:
 * Measure(constraints px, scale) -> MeasuredSize (includes Margin),
 * Arrange(destination px) -> DrawingRect,
 * Render(ctx) -> background + Paint(ctx),
 * ProcessGestures(args, apply) -> consumer or null.
 */
export class SkiaControl {
  // ---- layout properties (points) ----
  HorizontalOptions: LayoutOptions = "Start";
  VerticalOptions: LayoutOptions = "Start";
  /** -1 = auto. */
  WidthRequest = -1;
  HeightRequest = -1;
  /** Caps the available size (points); a Fill control fills only up to it and stays start-aligned, like DrawnUi. -1 = none. */
  MaximumWidthRequest = -1;
  MaximumHeightRequest = -1;
  /** Floors the measured size (points). -1 = none. */
  MinimumWidthRequest = -1;
  MinimumHeightRequest = -1;
  /** Grid placement (C# attached properties SkiaLayout.Column/Row/ColumnSpan/RowSpan). */
  Column = 0;
  Row = 0;
  ColumnSpan = 1;
  RowSpan = 1;
  Margin: Thickness = Thickness.Zero;
  /**
   * DrawnUi LockRatio: 0 = off; positive = square of the LARGER of the two sizes (requests or constraints),
   * negative = square of the SMALLER one. Infinite sides are ignored, so WidthRequest=150 + LockRatio=1 = 150x150.
   */
  LockRatio = 0;
  BackgroundColor?: Color;
  /** Gradient painted as the background (over BackgroundColor). */
  FillGradient?: SkiaGradient;
  IsVisible = true;
  Tag?: string;

  // ---- render transforms (MAUI VisualElement names; points, applied at render around DrawingRect) ----
  TranslationX = 0;
  TranslationY = 0;
  /** Degrees, around (AnchorX, AnchorY). */
  Rotation = 0;
  ScaleX = 1;
  ScaleY = 1;
  /** Degrees. */
  SkewX = 0;
  SkewY = 0;
  /** Pivot for Rotation/Scale/Skew as a fraction of the box (MAUI default 0.5). */
  AnchorX = 0.5;
  AnchorY = 0.5;
  /** 0..1, applied as a layer alpha over the whole subtree (MAUI VisualElement.Opacity). */
  Opacity = 1;
  /** Clip everything this control draws (content, children, effects per ClipEffects) to its DrawingRect. */
  IsClippedToBounds = false;

  /** Sets ScaleX and ScaleY together (MAUI Scale). */
  get Scale(): number { return this.ScaleX; }
  set Scale(v: number) { this.ScaleX = v; this.ScaleY = v; }
  get HasTransform(): boolean {
    return this.TranslationX !== 0 || this.TranslationY !== 0 || this.Rotation !== 0 || this.ScaleX !== 1 || this.ScaleY !== 1 || this.SkewX !== 0 || this.SkewY !== 0;
  }
  /** Matrix applied at the last render (canvas space), undefined when none; gestures map through its inverse. */
  RenderTransformMatrix?: number[];
  /** Generic numeric parameters (DrawnUi Value1/Value2): SkiaShape Arc uses start angle / sweep angle in degrees. */
  Value1 = 0;
  Value2 = 0;
  /** Extra points beyond the viewport a virtualized layout keeps realized (DrawnUi VirtualisationInflated). */
  VirtualisationInflated = 0;

  // ---- data binding (MAUI BindableObject subset) ----
  private bindingContext: unknown = undefined;
  /** Bound item; recycled cells get a new one on every rebind. */
  get BindingContext(): unknown { return this.bindingContext; }
  set BindingContext(value: unknown) {
    if (this.bindingContext === value) return;
    this.bindingContext = value;
    this.OnBindingContextChanged();
  }
  /** Index of the bound item inside its ItemsSource, -1 when not templated. */
  ContextIndex = -1;
  /** Override to react to a new BindingContext (DrawnUi ContextPropertyChanged / cell SetContent). */
  protected OnBindingContextChanged(): void {}

  // ---- caching (same names as DrawnUi) ----
  /** What to cache for this subtree; see SkiaCacheType. Layouts default to None, SkiaLabel/SkiaSvg to Operations. */
  UseCache: SkiaCacheType = "None";
  /** The cache type actually applied (None while caching is disabled globally). */
  get UsingCacheType(): SkiaCacheType {
    if (!Super.CacheEnabled) return "None";
    switch (this.UseCache) {
      case "None": return "None";
      case "Operations": case "OperationsFull": return "Operations";
      case "ImageDoubleBuffered": return "ImageDoubleBuffered";
      default: return "Image"; // Image, GPU, ImageComposite(GPU) -> single offscreen image for now
    }
  }
  /** Current cache, if any. */
  RenderObject?: CachedObject;
  private cacheDirty = true;

  // ---- gesture properties ----
  /** Control itself ignores input (things below it still get it). */
  InputTransparent = false;
  /** Consume every gesture that lands on this control so nothing below (z-order) receives it. */
  BlockGesturesBelow = false;
  LockChildrenGestures: LockTouch = "Disabled";

  // ---- touch feedback (same names as DrawnUi) ----
  TouchEffectColor: Color = Colors.White;
  /** Plays a ripple (from the tap point) when Tapped fires. */
  AnimationTapped: SkiaTouchAnimation = "None";
  /** Ripple duration ms, 0 = animator default (500). */
  AnimationTappedSpeed = 0;
  /** Overlay effects drawn above this control's content every frame (ripple etc). */
  readonly PostAnimators: IOverlayEffect[] = [];
  /** Clip overlay effects to the control's shape (CreateClip). */
  ClipEffects = true;

  // ---- gesture events (single handler each; C# events map to one callback prop) ----
  Tapped?: (sender: SkiaControl, e: ControlTappedEventArgs) => void;
  ChildTapped?: (sender: SkiaControl, e: ControlTappedEventArgs) => void;
  /** Raw gesture hook: set e.Consumed = true to stop propagation (not for Up). */
  ConsumeGestures?: (sender: SkiaControl, e: SkiaGesturesInfo) => void;

  // ---- tree ----
  Parent?: SkiaControl;
  /** Containers override; a leaf control cannot host children. */
  AddSubView(_control: SkiaControl): void { throw new Error(`DrawnUi: ${this.constructor.name} cannot host children`); }
  InsertSubView(_index: number, control: SkiaControl): void { this.AddSubView(control); }
  RemoveSubView(_control: SkiaControl): void {}
  /** Set by Canvas on its Content. Children resolve through Parent. */
  _superview?: Canvas;
  get Superview(): Canvas | undefined {
    return this.Parent ? this.Parent.Superview : this._superview;
  }

  // ---- state (pixels) ----
  MeasuredSize: ScaledSize = ScaledSize.Default;
  DrawingRect: SKRect = SKRect.Empty;
  RenderingScale = 1;
  NeedMeasure = true;

  private lastWidthConstraint = NaN;
  private lastHeightConstraint = NaN;

  /**
   * Public non-virtual entry like DrawnUi: applies Margin/requests, then MeasureAbsolute for content.
   * A control that was not invalidated and gets the same constraints + scale returns its previous size —
   * this is what keeps a cached tree from re-measuring text every frame.
   */
  Measure(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    if (!this.NeedMeasure && this.RenderingScale === scale
      && Object.is(this.lastWidthConstraint, widthConstraint) && Object.is(this.lastHeightConstraint, heightConstraint)) {
      return this.MeasuredSize;
    }
    this.lastWidthConstraint = widthConstraint;
    this.lastHeightConstraint = heightConstraint;
    this.RenderingScale = scale;
    const mx = this.Margin.HorizontalThickness * scale;
    const my = this.Margin.VerticalThickness * scale;

    // DrawnUi CalculateSizeRequest: with LockRatio a single set request drives BOTH sides
    // (WidthRequest=42 + LockRatio=1 => 42x42 whatever the cell is), requests then win over locked constraints.
    let reqW = this.WidthRequest, reqH = this.HeightRequest;
    if (this.LockRatio !== 0 && (reqW >= 0 || reqH >= 0)) {
      const both = reqW >= 0 && reqH >= 0;
      reqW = reqH = both ? (this.LockRatio > 0 ? Math.max(reqW, reqH) : Math.min(reqW, reqH)) : Math.max(reqW, reqH);
    }

    let w = widthConstraint - mx;
    let h = heightConstraint - my;
    if (reqW >= 0) w = reqW * scale;
    else if (this.MaximumWidthRequest >= 0) w = Math.min(w, this.MaximumWidthRequest * scale);
    if (reqH >= 0) h = reqH * scale;
    else if (this.MaximumHeightRequest >= 0) h = Math.min(h, this.MaximumHeightRequest * scale);

    // DrawnUi CreateMeasureRequest: no request set -> the constraints themselves are locked (SmartMax/SmartMin * |ratio|)
    let locked = false;
    if (this.LockRatio !== 0 && reqW < 0 && reqH < 0) {
      const lock = (this.LockRatio > 0 ? SkiaControl.SmartMax(w, h) : SkiaControl.SmartMin(w, h)) * Math.abs(this.LockRatio);
      if (lock > 0 && isFinite(lock)) { w = h = lock; locked = true; }
    }

    const content = this.MeasureAbsolute(w, h, scale);

    let rw = locked || reqW >= 0 ? w : this.HorizontalOptions === "Fill" && isFinite(w) ? w : content.Pixels.Width;
    let rh = locked || reqH >= 0 ? h : this.VerticalOptions === "Fill" && isFinite(h) ? h : content.Pixels.Height;
    if (this.MinimumWidthRequest >= 0) rw = Math.max(rw, this.MinimumWidthRequest * scale);
    if (this.MinimumHeightRequest >= 0) rh = Math.max(rh, this.MinimumHeightRequest * scale);
    if (this.MaximumWidthRequest >= 0 && reqW < 0) rw = Math.min(rw, this.MaximumWidthRequest * scale);
    if (this.MaximumHeightRequest >= 0 && reqH < 0) rh = Math.min(rh, this.MaximumHeightRequest * scale);

    this.MeasuredSize = ScaledSize.FromPixels(rw + mx, rh + my, scale);
    this.NeedMeasure = false;
    return this.MeasuredSize;
  }

  /** Override to measure own content, constraints already exclude Margin. */
  protected MeasureAbsolute(_widthConstraint: number, _heightConstraint: number, scale: number): ScaledSize {
    return ScaledSize.FromPixels(0, 0, scale);
  }

  /** Places MeasuredSize inside destination per Margin + Horizontal/VerticalOptions -> DrawingRect. */
  Arrange(destination: SKRect, _widthRequest: number, _heightRequest: number, scale: number): void {
    const m = this.Margin;
    const availL = destination.Left + m.Left * scale;
    const availT = destination.Top + m.Top * scale;
    let availW = destination.Width - m.HorizontalThickness * scale;
    let availH = destination.Height - m.VerticalThickness * scale;
    // DrawnUi DefineAvailableSize: a Maximum*Request caps the available box; a Fill control then fills only that.
    if (this.WidthRequest < 0 && this.MaximumWidthRequest >= 0) availW = Math.min(availW, this.MaximumWidthRequest * scale);
    if (this.HeightRequest < 0 && this.MaximumHeightRequest >= 0) availH = Math.min(availH, this.MaximumHeightRequest * scale);

    const w = this.HorizontalOptions === "Fill" ? availW : Math.min(availW, this.MeasuredSize.Pixels.Width - m.HorizontalThickness * scale);
    const h = this.VerticalOptions === "Fill" ? availH : Math.min(availH, this.MeasuredSize.Pixels.Height - m.VerticalThickness * scale);

    // Alignment happens inside the ORIGINAL box (Center/End of a capped control still center/end in the parent).
    const fullW = destination.Width - m.HorizontalThickness * scale, fullH = destination.Height - m.VerticalThickness * scale;
    const x = availL + SkiaControl.Align(this.HorizontalOptions, fullW, w);
    const y = availT + SkiaControl.Align(this.VerticalOptions, fullH, h);
    this.DrawingRect = SKRect.Create(x, y, w, h);
    this.OnLayoutChanged();
    // DrawnUi OnLayoutReady: an accessible control registers itself on its first layout
    if (!this.registeredWithAccessibility && this.IsAccessibilityElement) this.NotifyAccessibility();
  }

  // ---- accessibility (DrawnUi ISkiaAccessibilityNode) ----
  private static nextAccessibilityId = 1;
  /** Stable identity for the DOM overlay. */
  readonly AccessibilityId = SkiaControl.nextAccessibilityId++;
  private registeredWithAccessibility = false;
  private accessibilityRole?: string;
  private accessibilityLabel?: string;
  private accessibilityHint?: string;
  private accessibilityCanInteract?: boolean;
  private accessibilityIsPressed?: boolean;
  private accessibilityLive?: string;

  /**
   * ARIA role; setting it exposes the control to assistive technology (use `Aria` constants). Unset = the class
   * default (`SkiaLabel.DefaultAccessibilityRole` etc.), which is itself unset unless the app opts in.
   */
  get AccessibilityRole(): string | undefined { return this.accessibilityRole ?? (this.constructor as typeof SkiaControl).DefaultAccessibilityRole; }
  set AccessibilityRole(v: string | undefined) { if (this.accessibilityRole !== v) { this.accessibilityRole = v; this.AccessibilityChanged(); } }
  /** Per-class default role (React extension): e.g. `SkiaLabel.DefaultAccessibilityRole = Aria.RoleText` reads every label. */
  static DefaultAccessibilityRole?: string;

  /** Spoken label; unset = the control's own text (SkiaLabel.Text, SkiaButton.Text). */
  get AccessibilityLabel(): string | undefined { return this.accessibilityLabel ?? this.DefaultAccessibilityLabel(); }
  set AccessibilityLabel(v: string | undefined) { if (this.accessibilityLabel !== v) { this.accessibilityLabel = v; this.AccessibilityChanged(); } }
  protected DefaultAccessibilityLabel(): string | undefined { return undefined; }

  get AccessibilityHint(): string | undefined { return this.accessibilityHint; }
  set AccessibilityHint(v: string | undefined) { if (this.accessibilityHint !== v) { this.accessibilityHint = v; this.AccessibilityChanged(); } }

  /** Tab stop + activation; unset = true when the control handles Tapped. */
  get AccessibilityCanInteract(): boolean { return this.accessibilityCanInteract ?? this.DefaultAccessibilityCanInteract(); }
  set AccessibilityCanInteract(v: boolean) { if (this.accessibilityCanInteract !== v) { this.accessibilityCanInteract = v; this.AccessibilityChanged(); } }
  protected DefaultAccessibilityCanInteract(): boolean { return !!this.Tapped; }

  /** aria-pressed for toggles; undefined = not a toggle. */
  get AccessibilityIsPressed(): boolean | undefined { return this.accessibilityIsPressed; }
  set AccessibilityIsPressed(v: boolean | undefined) { if (this.accessibilityIsPressed !== v) { this.accessibilityIsPressed = v; this.AccessibilityChanged(); } }

  /** aria-live: `Aria.LivePolite` / `Aria.LiveAssertive`; changes are announced immediately. */
  get AccessibilityLive(): string | undefined { return this.accessibilityLive; }
  set AccessibilityLive(v: string | undefined) { if (this.accessibilityLive !== v) { this.accessibilityLive = v; this.AccessibilityChanged(); } }

  get IsAccessibilityElement(): boolean { const r = this.AccessibilityRole; return r != null && r !== Aria.RolePresentation; }

  /** Hit rect in canvas pixels used to position the overlay element: the drawn (transformed) bounds, like C# HitBoxWithTransforms. */
  GetAccessibilityPixelRect(): SKRect {
    const r = this.DrawingRect, m = this.RenderTransformMatrix;
    if (!m) return r;
    const pts = Super.CK.Matrix.mapPoints(m, [r.Left, r.Top, r.Right, r.Top, r.Right, r.Bottom, r.Left, r.Bottom]);
    const xs = [pts[0], pts[2], pts[4], pts[6]], ys = [pts[1], pts[3], pts[5], pts[7]];
    return new SKRect(Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys));
  }

  /** Registers on first call, marks the snapshot dirty afterwards. */
  NotifyAccessibility(): void {
    if (!this.IsAccessibilityElement) return;
    const mgr = this.Superview?.AccessibilityManager;
    if (!mgr) return;
    if (!this.registeredWithAccessibility) { mgr.Register(this); this.registeredWithAccessibility = true; }
    else mgr.NotifyUpdated(this);
  }

  UnregisterAccessibility(): void {
    this.Superview?.AccessibilityManager.Unregister(this);
    this.registeredWithAccessibility = false;
  }

  AccessibilityChanged(): void {
    if (!this.IsAccessibilityElement && this.registeredWithAccessibility) this.UnregisterAccessibility();
    else this.NotifyAccessibility();
  }

  OnAccessibilityUnregistered(): void { this.registeredWithAccessibility = false; }

  /** Platform layer (overlay click / Enter): injects a synthetic Tapped at the control's center into the gesture pipeline. */
  OnAccessibilityActivated(): void {
    const r = this.GetAccessibilityPixelRect();
    const center = new SKPoint((r.Left + r.Right) / 2, (r.Top + r.Bottom) / 2);
    const args = new TouchActionEventArgs();
    args.Type = "Released";
    args.Location = center;
    args.StartingLocation = center;
    args.Scale = this.RenderingScale;
    this.OnSkiaGestureEvent(SkiaGesturesParameters.Create("Tapped", args), new GestureEventProcessingInfo(center));
    this.Repaint();
  }

  /** Overlay focus arrives on / leaves this node (inputs may activate their sink here). */
  OnAccessibilityFocused(_focused: boolean): void {}

  NotifyAccessibilityFocused(focused: boolean): void { this.Superview?.AccessibilityManager.NotifyFocused(focused ? this : undefined); }

  /** DrawnUi SmartMax: larger of two sizes, an infinite one loses. */
  protected static SmartMax(a: number, b: number): number {
    if (!isFinite(a) || (isFinite(b) && b > a)) return b;
    return a;
  }

  /** DrawnUi SmartMin: smaller of two sizes, an infinite one loses. */
  protected static SmartMin(a: number, b: number): number {
    if (!isFinite(a) || (isFinite(b) && b < a)) return b;
    return a;
  }

  private static Align(o: LayoutOptions, avail: number, size: number): number {
    if (o === "Center") return (avail - size) / 2;
    if (o === "End") return avail - size;
    return 0;
  }

  /** Called after DrawingRect changed; layouts arrange children here. */
  protected OnLayoutChanged(): void {}

  /** Part of DrawingRect that can actually be seen: intersected with every ancestor (a scroll's box clips its content). */
  GetVisibleViewport(): SKRect {
    const r = this.DrawingRect;
    if (!this.Parent) return r;
    const p = this.Parent.GetVisibleViewport();
    return new SKRect(Math.max(r.Left, p.Left), Math.max(r.Top, p.Top), Math.min(r.Right, p.Right), Math.min(r.Bottom, p.Bottom));
  }

  /** Draws the control: from its cache when it has one, else background + Paint(); overlays always live. */
  /**
   * Pixels the control paints OUTSIDE its DrawingRect (shadows, effects): caches record that much more and the
   * blit is offset accordingly (DrawnUi ComputeEffectsMargin / GetRenderingExpandPixels). Base: nothing.
   */
  ComputeEffectsMargin(_scale: number): Thickness { return Thickness.Zero; }

  private effectsMarginCache?: { scale: number; margin: Thickness };
  /** Cached ComputeEffectsMargin (reset by InvalidateMeasure) — C# AggregatedEffectsMarginPixels. */
  EffectsMargin(scale: number): Thickness {
    if (!this.effectsMarginCache || this.effectsMarginCache.scale !== scale) this.effectsMarginCache = { scale, margin: this.ComputeEffectsMargin(scale) };
    return this.effectsMarginCache.margin;
  }

  /** DrawingRect grown by the effects margin, in pixels — the rect a cache is recorded for. */
  protected ExpandedCacheRect(scale: number): SKRect {
    const r = this.DrawingRect, m = this.EffectsMargin(scale);
    if (m.Left === 0 && m.Top === 0 && m.Right === 0 && m.Bottom === 0) return r;
    return new SKRect(r.Left - Math.ceil(m.Left), r.Top - Math.ceil(m.Top), r.Right + Math.ceil(m.Right), r.Bottom + Math.ceil(m.Bottom));
  }

  Render(ctx: DrawingContext): void {
    if (!this.IsVisible || this.Opacity <= 0) return;
    const canvas = ctx.Context.Canvas;
    const applyOpacity = this.Opacity < 1;
    const needTransform = this.HasTransform;
    let saved = false;
    // same as DrawnUi: opacity = a layer with alpha, transforms = canvas matrix around the whole subtree (cache included)
    if (applyOpacity) {
      const paint = new Super.CK.Paint();
      paint.setAlphaf(Math.max(0, Math.min(1, this.Opacity)));
      canvas.saveLayer(paint);
      paint.delete();
      saved = true;
    } else if (needTransform) {
      canvas.save();
      saved = true;
    }
    if (needTransform) {
      this.RenderTransformMatrix = this.CreateRenderTransformMatrix(this.DrawingRect, ctx.Scale);
      canvas.concat(this.RenderTransformMatrix);
    } else {
      this.RenderTransformMatrix = undefined;
    }
    if (this.IsClippedToBounds) {
      if (!saved) { canvas.save(); saved = true; }
      const c = this.ClipEffects ? this.DrawingRect : this.ExpandedCacheRect(ctx.Scale);
      canvas.clipRect(Super.CK.LTRBRect(c.Left, c.Top, c.Right, c.Bottom), Super.CK.ClipOp.Intersect, true);
    }
    this.RenderContent(ctx);
    if (saved) canvas.restore();
  }

  /** Port of DrawnUi ApplyTransforms: T(-pivot) · rotation · scale/skew · T(pivot) · translation, in canvas pixels. */
  protected CreateRenderTransformMatrix(destination: SKRect, scale: number): number[] {
    const M = Super.CK.Matrix;
    const moveX = this.TranslationX * scale, moveY = this.TranslationY * scale;
    if (this.Rotation === 0 && this.ScaleX === 1 && this.ScaleY === 1 && this.SkewX === 0 && this.SkewY === 0) return M.translated(moveX, moveY);
    const px = destination.Left + destination.Width * this.AnchorX;
    const py = destination.Top + destination.Height * this.AnchorY;
    const kx = this.SkewX !== 0 ? Math.tan((Math.PI * this.SkewX) / 180) : 0;
    const ky = this.SkewY !== 0 ? Math.tan((Math.PI * this.SkewY) / 180) : 0;
    const scaleSkew = [this.ScaleX, kx, 0, ky, this.ScaleY, 0, 0, 0, 1];
    const rotation = this.Rotation !== 0 ? M.rotated((Math.PI * this.Rotation) / 180) : M.identity();
    // multiply(a, b, c) = a·b·c, the right-most applies first
    return M.multiply(M.translated(px + moveX, py + moveY), scaleSkew, rotation, M.translated(-px, -py));
  }

  /** Maps a point given in the parent's space into this control's untransformed space (DrawnUi TransformPointToLocalSpace). */
  TransformPointToLocalSpace(point: SKPoint): SKPoint {
    const m = this.RenderTransformMatrix;
    if (!m) return point;
    const inv = Super.CK.Matrix.invert(m);
    if (!inv) return point;
    const p = Super.CK.Matrix.mapPoints(inv, [point.X, point.Y]);
    return new SKPoint(p[0], p[1]);
  }

  /** Cache blit or live paint, then post animators — the part a transform/opacity layer wraps. */
  private RenderContent(ctx: DrawingContext): void {
    const own: DrawingContext = { ...ctx, Destination: this.DrawingRect };
    const cacheType = this.UsingCacheType;
    if (cacheType === "None") {
      this.DestroyRenderingObject();
      this.PaintContent(own);
    } else {
      const r = this.ExpandedCacheRect(ctx.Scale);
      const stale = this.cacheDirty || !this.RenderObject || this.RenderObject.Type !== cacheType
        || this.RenderObject.Scale !== ctx.Scale
        || Math.round(this.RenderObject.Bounds.Width) !== Math.round(r.Width) || Math.round(this.RenderObject.Bounds.Height) !== Math.round(r.Height);
      if (stale) this.CreateRenderingObject(own, cacheType);
      if (this.RenderObject) this.RenderObject.Draw(ctx.Context.Canvas, r.Left, r.Top);
      else if (this.RenderObjectPrevious) this.RenderObjectPrevious.Draw(ctx.Context.Canvas, r.Left, r.Top); // double buffer: last good frame
      else this.DrawPlaceholder(own);
    }
    this.ExecutePostAnimators(own);
  }

  /** Background + Paint(): the part of the control that a cache captures. */
  protected PaintContent(ctx: DrawingContext): void {
    if (this.BackgroundColor || this.FillGradient || this.PaintsBackgroundWithoutColor()) this.PaintBackground(ctx);
    this.Paint(ctx);
  }
  /** Subclasses whose PaintBackground has something to draw without a BackgroundColor (shape shadows). */
  protected PaintsBackgroundWithoutColor(): boolean { return false; }

  /**
   * ImageDoubleBuffered: the last cache is kept while a new one is produced and shown when producing fails
   * (no surface yet); no background thread in the browser, so recording itself stays synchronous like DrawnUi.Blazor.
   */
  RenderObjectPrevious?: CachedObject;

  /** Drawn when a cache is expected but none exists yet (DrawnUi DrawPlaceholder, ImageDoubleBuffered only). Default: nothing. */
  protected DrawPlaceholder(_ctx: DrawingContext): void {}

  /** Records/renders the content into a new CachedObject for the current DrawingRect. */
  protected CreateRenderingObject(ctx: DrawingContext, cacheType: SkiaCacheType): void {
    const CK = Super.CK;
    const r = this.ExpandedCacheRect(ctx.Scale); // includes what shadows/effects paint outside the box
    const w = Math.max(1, Math.round(r.Width)), h = Math.max(1, Math.round(r.Height));
    if (cacheType === "ImageDoubleBuffered") {
      // keep the previous frame as the fallback until the new cache exists
      if (this.RenderObject) { this.DisposePrevious(); this.RenderObjectPrevious = this.RenderObject; this.RenderObject = undefined; }
    } else {
      this.DestroyRenderingObject();
    }
    if (cacheType === "Operations") {
      const recorder = new CK.PictureRecorder();
      const canvas = recorder.beginRecording(CK.LTRBRect(r.Left, r.Top, r.Right, r.Bottom));
      this.PaintContent({ ...ctx, Context: { ...ctx.Context, Canvas: canvas } });
      const picture = recorder.finishRecordingAsPicture();
      recorder.delete();
      this.RenderObject = new CachedObject("Operations", r, ctx.Scale, picture);
    } else {
      const main = ctx.Context.Surface;
      if (!main) { this.PaintContent(ctx); return; } // no surface to derive from (e.g. recording): draw live
      const offscreen = main.makeSurface({ ...main.imageInfo(), width: w, height: h });
      if (!offscreen) { if (!this.RenderObjectPrevious) this.PaintContent(ctx); return; }
      const canvas = offscreen.getCanvas();
      canvas.clear(CK.TRANSPARENT);
      canvas.translate(-r.Left, -r.Top);
      this.PaintContent({ ...ctx, Context: { Canvas: canvas, Surface: offscreen } });
      const image = offscreen.makeImageSnapshot();
      offscreen.delete();
      this.RenderObject = new CachedObject(cacheType, r, ctx.Scale, undefined, image);
    }
    this.cacheDirty = false;
  }

  private DisposePrevious(): void {
    const old = this.RenderObjectPrevious;
    if (!old) return;
    this.RenderObjectPrevious = undefined;
    const sv = this.Superview;
    if (sv) sv.DisposeObject(old); else old.Dispose();
  }

  /** Drops the cache (disposed after the frame, never mid-draw). */
  DestroyRenderingObject(): void {
    this.DisposePrevious();
    const old = this.RenderObject;
    if (!old) return;
    this.RenderObject = undefined;
    const sv = this.Superview;
    if (sv) sv.DisposeObject(old); else old.Dispose();
  }

  /** Marks the cache stale; re-recorded on the next frame. */
  InvalidateCache(): void { this.cacheDirty = true; }

  /** Draws PostAnimators above content; an effect returning true asks for another frame. */
  ExecutePostAnimators(ctx: DrawingContext): void {
    if (this.PostAnimators.length === 0) return;
    for (const effect of [...this.PostAnimators]) if (effect.Render(ctx, this)) this.Repaint();
  }

  /** Clip path of this control in canvas pixels (rect; shapes override). Caller deletes it. */
  CreateClip(): Path {
    const r = this.DrawingRect;
    const b = new Super.CK.PathBuilder();
    b.addRect(Super.CK.LTRBRect(r.Left, r.Top, r.Right, r.Bottom));
    const path = b.detach();
    b.delete();
    return path;
  }

  /** Fills DrawingRect with BackgroundColor / FillGradient; shapes override for rounded corners etc. */
  protected PaintBackground(ctx: DrawingContext): void {
    const paint = this.CreateBackgroundPaint(ctx.Destination);
    const r = ctx.Destination;
    ctx.Context.Canvas.drawRect(Super.CK.LTRBRect(r.Left, r.Top, r.Right, r.Bottom), paint);
    paint.delete();
  }

  /** Paint for the background: solid BackgroundColor, or FillGradient shader over the given rect. Caller deletes. */
  protected CreateBackgroundPaint(rect: SKRect): import("canvaskit-wasm").Paint {
    const CK = Super.CK;
    const paint = new CK.Paint();
    paint.setAntiAlias(true);
    if (this.BackgroundColor) paint.setColor(Super.ParseColor(this.BackgroundColor));
    const g = this.FillGradient;
    if (g && g.Colors.length > 0) {
      const colors = g.Colors.map((c) => Super.ParseColor(c));
      const shader = CK.Shader.MakeLinearGradient(
        [rect.Left + rect.Width * (g.StartXRatio ?? 0), rect.Top + rect.Height * (g.StartYRatio ?? 0)],
        [rect.Left + rect.Width * (g.EndXRatio ?? 0), rect.Top + rect.Height * (g.EndYRatio ?? 1)],
        colors, null, CK.TileMode.Clamp,
      );
      paint.setShader(shader);
      shader.delete();
    }
    return paint;
  }

  /** Override to draw own content into ctx.Destination. */
  protected Paint(_ctx: DrawingContext): void {}

  // ---- invalidation (same names as DrawnUi) ----

  /** Content changed: cache invalidated + remeasure + redraw (bubbles to every ancestor, whose caches go stale too). */
  Update(): void {
    this.InvalidateMeasure();
  }

  /** Redraw without remeasure and WITHOUT dropping caches (position/overlay changes). */
  Repaint(): void {
    this.Superview?.Update();
  }

  /**
   * Transform / Opacity changed: this control's own cache is still valid (content unchanged) but every ancestor
   * cache holds its composited output, so those are staled before redrawing (DrawnUi RedrawCanvas + parent invalidation).
   */
  RepaintComposition(): void {
    let p = this.Parent;
    while (p) { p.cacheDirty = true; p = p.Parent; }
    this.Repaint();
  }

  /** Size or content may have changed: this control and all ancestors remeasure and re-record. */
  InvalidateMeasure(): void {
    this.NeedMeasure = true;
    this.cacheDirty = true;
    this.effectsMarginCache = undefined;
    if (this.Parent) this.Parent.InvalidateMeasure();
    else this.Superview?.Update();
  }

  // ---- gestures (same names as DrawnUi) ----

  /** Hit rect in pixels (no transforms in this port, so it is DrawingRect). */
  get HitBoxAuto(): SKRect { return this.DrawingRect; }

  HitIsInside(x: number, y: number): boolean {
    const r = this.HitBoxAuto;
    return x >= r.Left && x < r.Right && y >= r.Top && y < r.Bottom;
  }

  IsGestureForChild(child: SkiaControl, point: SKPoint): boolean {
    const local = child.TransformPointToLocalSpace(point);
    return child.HitIsInside(local.X, local.Y);
  }

  /** Children that may receive gestures, top-most LAST (layouts return their Views). */
  protected GetGestureListeners(): readonly SkiaControl[] { return SkiaControl.NoListeners; }
  private static readonly NoListeners: readonly SkiaControl[] = [];

  /** ISkiaGestureListener entry, same as DrawnUi: routes to ProcessGestures. */
  OnSkiaGestureEvent(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    return this.ProcessGestures(args, apply);
  }

  protected CheckChildrenGesturesLocked(type: SkiaGesturesParameters["Type"]): boolean {
    switch (this.LockChildrenGestures) {
      case "Enabled": return true;
      case "PassNone": return true;
      case "PassTap": return type !== "Tapped";
      case "PassTapAndLongPress": return type !== "Tapped" && type !== "LongPressing";
      default: return false;
    }
  }

  /**
   * Port of DrawnUi SkiaControl.ProcessGestures (non-render-tree branch):
   * ConsumeGestures hook -> children top-most first -> own Tapped. Returns the consumer or null.
   */
  ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (!this.Superview) return null;

    const consumedDefault = this.BlockGesturesBelow ? this : null;

    if (this.ConsumeGestures) {
      const sent = new SkiaGesturesInfo(args, apply);
      this.ConsumeGestures(this, sent);
      if (args.Type !== "Up" && sent.Consumed) return this;
    }

    if (this.CheckChildrenGesturesLocked(args.Type)) return consumedDefault;

    let consumed: SkiaControl | null = null;
    const wasConsumed = apply.AlreadyConsumed;
    const point = new SKPoint(apply.MappedLocation.X + apply.ChildOffset.X, apply.MappedLocation.Y + apply.ChildOffset.Y);
    const listeners = this.GetGestureListeners();

    for (let i = listeners.length - 1; i >= 0; i--) {
      const listener = listeners[i];
      if (!listener.IsVisible || listener.InputTransparent) continue;
      if (!this.IsGestureForChild(listener, point)) continue;

      let breakForChild: SkiaControl | null = null;
      if (args.Type === "Tapped") {
        if (this.ChildTapped) { breakForChild = listener; this.ChildTapped(this, new ControlTappedEventArgs(listener, args, apply)); }
      }
      // the child sees the location in its own untransformed space (DrawnUi maps through the inverse RenderTransformMatrix)
      const local = listener.TransformPointToLocalSpace(point);
      consumed = listener.OnSkiaGestureEvent(args, new GestureEventProcessingInfo(local, SKPoint.Empty, apply.ChildOffsetDirect, wasConsumed));
      if (consumed) break;
      if (breakForChild) { consumed = breakForChild; break; }
    }
    if (wasConsumed) consumed = wasConsumed;

    if (args.Type === "Tapped" && !consumed && this.SendTapped(args, apply)) return this;

    return consumed ?? consumedDefault;
  }

  /** Fires Tapped (after AnimationTapped feedback); true when a handler existed (= consumed), like DrawnUi SendTapped. */
  protected SendTapped(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): boolean {
    if (!this.Tapped) return false;
    if (this.AnimationTapped === "Ripple") {
      const pts = this.GetOffsetInsideControlInPoints(apply.MappedLocation, apply.ChildOffset);
      this.PlayRippleAnimation(this.TouchEffectColor, pts.X, pts.Y, true, this.AnimationTappedSpeed);
    }
    this.Tapped(this, new ControlTappedEventArgs(this, args, apply));
    return true;
  }

  /** Canvas-pixel touch location -> points relative to this control's top-left. */
  GetOffsetInsideControlInPoints(location: SKPoint, childOffset: SKPoint): SKPoint {
    return new SKPoint(
      (location.X + childOffset.X - this.DrawingRect.Left) / this.RenderingScale,
      (location.Y + childOffset.Y - this.DrawingRect.Top) / this.RenderingScale,
    );
  }

  // ---- animations (DrawnUi AnimateAsync family, Promise-based; AbortSignal instead of CancellationTokenSource) ----
  private ownAnimations = new Map<string, AbortController>();

  /** Runs callback(0..1) over ms with easing on the canvas frame loop; rejects with AbortError when cancelled. */
  AnimateAsync(callback: (value: number) => void, onCancel?: () => void, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const animator = new SkiaValueAnimator(this);
      animator.mMinValue = 0;
      animator.mMaxValue = 1;
      animator.Speed = ms;
      animator.Easing = easing;
      let cancelled = false;
      const abort = () => { cancelled = true; animator.Stop(); };
      if (cancel?.aborted) { onCancel?.(); reject(new DOMException("Aborted", "AbortError")); return; }
      cancel?.addEventListener("abort", abort, { once: true });
      animator.OnUpdated = (v) => { callback(v); this.RepaintComposition(); };
      animator.OnStop = () => {
        cancel?.removeEventListener("abort", abort);
        if (cancelled) { onCancel?.(); reject(new DOMException("Aborted", "AbortError")); } else resolve();
      };
      animator.Start();
    });
  }

  /** callback(start..end) over ms. */
  AnimateRangeAsync(callback: (value: number) => void, start: number, end: number, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    return this.AnimateAsync((v) => callback(start + (end - start) * v), undefined, ms, easing, cancel);
  }

  /** One running animation per kind, like the per-property CancellationTokenSources in C#. */
  private RunOwnAnimation(kind: string, run: (signal: AbortSignal) => Promise<void>, cancel?: AbortSignal): Promise<void> {
    this.ownAnimations.get(kind)?.abort();
    const controller = new AbortController();
    this.ownAnimations.set(kind, controller);
    cancel?.addEventListener("abort", () => controller.abort(), { once: true });
    return run(controller.signal).finally(() => { if (this.ownAnimations.get(kind) === controller) this.ownAnimations.delete(kind); });
  }

  FadeToAsync(end: number, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    const start = this.Opacity;
    return this.RunOwnAnimation("fade", (s) => this.AnimateAsync((v) => { this.Opacity = start + (end - start) * v; }, undefined, ms, easing, s).then(() => { this.Opacity = end; }), cancel);
  }

  ScaleToAsync(x: number, y: number, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    const sx = this.ScaleX, sy = this.ScaleY;
    return this.RunOwnAnimation("scale", (s) => this.AnimateAsync((v) => { this.ScaleX = sx + (x - sx) * v; this.ScaleY = sy + (y - sy) * v; }, undefined, ms, easing, s).then(() => { this.ScaleX = x; this.ScaleY = y; }), cancel);
  }

  TranslateToAsync(x: number, y: number, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    const tx = this.TranslationX, ty = this.TranslationY;
    return this.RunOwnAnimation("translate", (s) => this.AnimateAsync((v) => { this.TranslationX = tx + (x - tx) * v; this.TranslationY = ty + (y - ty) * v; }, undefined, ms, easing, s).then(() => { this.TranslationX = x; this.TranslationY = y; }), cancel);
  }

  RotateToAsync(end: number, ms = 250, easing: Easing = Easing.Linear, cancel?: AbortSignal): Promise<void> {
    const start = this.Rotation;
    return this.RunOwnAnimation("rotate", (s) => this.AnimateAsync((v) => { this.Rotation = start + (end - start) * v; }, undefined, ms, easing, s).then(() => { this.Rotation = end; }), cancel);
  }

  /** Starts a ripple at (x, y) points inside this control; speedMs 0 = RippleAnimator default. */
  PlayRippleAnimation(color: Color, x: number, y: number, _removePrevious = true, speedMs = 0): void {
    const animation = new RippleAnimator(this);
    animation.Color = color;
    animation.X = x;
    animation.Y = y;
    if (speedMs > 0) animation.Speed = speedMs;
    animation.Start();
  }
}
