import type { Canvas as SkCanvas, Image, Path, SkPicture, Surface } from "canvaskit-wasm";
import { Super } from "./Super";
import { SkiaStyles } from "./Styles";
import { type Color, Colors, type LayoutOptions, SKRect, ScaledSize, type SkiaCacheType, type SkiaGradient, type SkiaTouchAnimation, Thickness } from "./Types";
import { type IOverlayEffect, RippleAnimator, SkiaValueAnimator } from "./Animators";
import { Easing } from "./Easing";
import type { Canvas } from "./Canvas";
import { Aria } from "./Accessibility";
import { ContextMenuEventArgs, ControlTappedEventArgs, GestureEventProcessingInfo, type LockTouch, SKPoint, SkiaGesturesInfo, SkiaGesturesParameters, TouchActionEventArgs } from "./Gestures";
import { type CachedTexture, type IPostRendererEffect, IsPostRendererEffect, type SkiaEffect } from "./SkiaEffect";

/** Mirrors DrawnUi DrawingContext: ctx.Context.Canvas / Surface, ctx.Destination (pixels), ctx.Scale. */
export interface DrawingContext {
  /**
   * Surface = where the pixels end up (an Image cache surface or the on-screen one), Origin = that surface's top-left
   * in canvas pixels (caches are translated to their own origin), Recording = the canvas records a picture
   * (Operations cache) that is replayed on Surface later.
   */
  Context: { Canvas: SkCanvas; Surface?: Surface; Origin?: { X: number; Y: number }; Recording?: boolean };
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
  /** Paint-time offset in points (C# Left/Top: moves the cached output without a matrix; here a plain translate). */
  Left = 0;
  Top = 0;
  private zIndex = 0;
  /** Drawing order among siblings: higher draws later (on top) and receives gestures first (C# ZIndex). */
  get ZIndex(): number { return this.zIndex; }
  set ZIndex(v: number) { if (this.zIndex !== v) { this.zIndex = v; this.Parent?.InvalidateViewsOrder(); this.RepaintComposition(); } }
  /** A layout re-sorts its children by ZIndex on the next draw (C# _orderedChildren reset). */
  InvalidateViewsOrder(): void {}
  /** Fraction of the available box a Fill control takes (C# DefineAvailableSize); alignment stays inside the full box. */
  HorizontalFillRatio = 1;
  VerticalFillRatio = 1;

  /** Sets ScaleX and ScaleY together (MAUI Scale). */
  get Scale(): number { return this.ScaleX; }
  set Scale(v: number) { this.ScaleX = v; this.ScaleY = v; }
  get HasTransform(): boolean {
    return this.TranslationX !== 0 || this.TranslationY !== 0 || this.Left !== 0 || this.Top !== 0 || this.Rotation !== 0 || this.ScaleX !== 1 || this.ScaleY !== 1 || this.SkewX !== 0 || this.SkewY !== 0;
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
      case "ImageComposite": case "ImageCompositeGPU": return "ImageComposite";
      default: return "Image"; // Image, GPU -> single offscreen image
    }
  }
  /** C# IsCacheComposite: the offscreen surface is kept and only the changed children are re-recorded. */
  get IsCacheComposite(): boolean { return this.UsingCacheType === "ImageComposite"; }
  /** Children reported dirty since the last composite record (C# DirtyChildrenInternal, filled by RepaintComposition). */
  readonly DirtyChildrenInternal = new Set<SkiaControl>();
  /** True while a composite re-record paints only the dirty children (C# IsRenderingWithComposition). */
  IsRenderingWithComposition = false;
  /** Composite surface kept across records; its size follows the expanded cache rect. */
  private compositeSurface?: Surface;
  /** Own content or structure changed: the next composite record is a full one. */
  private compositeFull = true;
  /** Canvas-pixel bounds this control covered when its parent last recorded it (composite erase region). */
  private lastCompositeBounds?: SKRect;
  /** What the last composite record did (diagnostics): "full" or "partial", the number of children re-recorded and which ones. */
  LastCompositeRecord: { Mode: "full" | "partial"; Children: number; Dirty: readonly SkiaControl[] } = { Mode: "full", Children: 0, Dirty: [] };
  /** The children a composite record can re-record individually; layouts return their views. */
  protected GetCompositeChildren(): readonly SkiaControl[] { return []; }
  /** C# TrackChildAsDirty (DirtyChildrenTracker): the child changed without a remeasure of this control. */
  TrackChildAsDirty(child: SkiaControl): void { if (this.IsCacheComposite) this.DirtyChildrenInternal.add(child); }
  /** Drawn bounds in canvas pixels including transforms and effects margins (C# GetTransformedDirtyBounds). */
  GetTransformedDirtyBounds(): SKRect {
    const r = this.ExpandedCacheRect(this.RenderingScale), m = this.RenderTransformMatrix;
    if (!m) return r;
    const pts = Super.CK.Matrix.mapPoints(m, [r.Left, r.Top, r.Right, r.Top, r.Right, r.Bottom, r.Left, r.Bottom]);
    const xs = [pts[0], pts[2], pts[4], pts[6]], ys = [pts[1], pts[3], pts[5], pts[7]];
    return new SKRect(Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys));
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

  // ---- visual effects (C# VisualEffects: attached SkiaEffect objects) ----
  private visualEffects: SkiaEffect[] = [];
  /** Post-renderer effects among VisualEffects (C# EffectPostRenderers). */
  EffectPostRenderers: (SkiaEffect & IPostRendererEffect)[] = [];
  /** Skip every attached effect (C# DisableEffects). */
  DisableEffects = false;
  /** Effects attached to this control; assign a new array to change them (they are attached/detached here). */
  get VisualEffects(): readonly SkiaEffect[] { return this.visualEffects; }
  set VisualEffects(v: readonly SkiaEffect[] | undefined) {
    const next = v ? [...v] : [];
    for (const e of this.visualEffects) if (!next.includes(e)) e.Dettach();
    for (const e of next) if (e.Parent !== this) e.Attach(this);
    this.visualEffects = next;
    this.OnVisualEffectsChanged();
  }
  protected OnVisualEffectsChanged(): void {
    this.EffectPostRenderers = this.visualEffects.filter(IsPostRendererEffect);
    this.InvalidateEffectsMargin();
    this.InvalidateCache();
    this.RepaintComposition();
  }
  /** Post renderers that can render this frame (compiled shaders); an unready effect leaves the control drawn plainly. */
  private ActivePostRenderers(): (SkiaEffect & IPostRendererEffect)[] {
    if (this.DisableEffects || this.EffectPostRenderers.length === 0) return [];
    return this.EffectPostRenderers.filter((e) => e.NeedApply);
  }
  /** C# CachedImage: the cached texture and the canvas rect it was rasterized over (Image caches only). */
  get CachedImage(): CachedTexture | undefined {
    const c = this.RenderObject;
    return c?.Image ? { Image: c.Image, Bounds: c.Bounds } : undefined;
  }
  /** Clip overlay effects to the control's shape (CreateClip). */
  ClipEffects = true;

  // ---- gesture events (single handler each; C# events map to one callback prop) ----
  Tapped?: (sender: SkiaControl, e: ControlTappedEventArgs) => void;
  /**
   * Context-menu request over this control (right click, long press on touch, Menu key). Return true to handle it:
   * the browser's own canvas menu is suppressed. Routed deepest child first, then parents, then Canvas.ContextMenu.
   * Not in DrawnUi.Net (a web concept).
   */
  ContextMenu?: (sender: SkiaControl, e: ContextMenuEventArgs) => boolean | void;
  ChildTapped?: (sender: SkiaControl, e: ControlTappedEventArgs) => void;
  /** Raw gesture hook: set e.Consumed = true to stop propagation (not for Up). */
  ConsumeGestures?: (sender: SkiaControl, e: SkiaGesturesInfo) => void;

  // ---- tree ----
  Parent?: SkiaControl;
  /** Containers override; a leaf control cannot host children. */
  private styledInitially = false;

  /**
   * C# ApplyInitialStyles: takes the property defaults registered with `ConfigureStyles` for this control's class.
   * The reconciler calls it with `atConstruction` right after `new`, before the JSX props, so props win; a control
   * built in code-behind is styled on its first measure and keeps whatever the code already set.
   */
  ApplyInitialStyles(atConstruction = false): void {
    this.styledInitially = true;
    if (!SkiaStyles.IsEmpty) SkiaStyles.Apply(this, atConstruction);
  }

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
    if (!this.styledInitially) this.ApplyInitialStyles(false);
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
    if (this.HorizontalFillRatio !== 1) availW = Math.ceil(availW * this.HorizontalFillRatio);
    if (this.VerticalFillRatio !== 1) availH = Math.ceil(availH * this.VerticalFillRatio);

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
  /**
   * Whether the mouse at this point (pixels, relative to DrawingRect's origin) is over something tappable, for the
   * host's pointer cursor. Default: the control as a whole (AccessibilityCanInteract); SkiaLabel adds its tappable spans.
   */
  WantsPointerCursor(_x: number, _y: number): boolean { return this.AccessibilityCanInteract; }

  /** aria-pressed for toggles; undefined = not a toggle. */
  get AccessibilityIsPressed(): boolean | undefined { return this.accessibilityIsPressed; }
  set AccessibilityIsPressed(v: boolean | undefined) { if (this.accessibilityIsPressed !== v) { this.accessibilityIsPressed = v; this.AccessibilityChanged(); } }

  /** aria-live: `Aria.LivePolite` / `Aria.LiveAssertive`; changes are announced immediately. */
  get AccessibilityLive(): string | undefined { return this.accessibilityLive; }
  set AccessibilityLive(v: string | undefined) { if (this.accessibilityLive !== v) { this.accessibilityLive = v; this.AccessibilityChanged(); } }

  get IsAccessibilityElement(): boolean { const r = this.AccessibilityRole; return r != null && r !== Aria.RolePresentation; }

  private accessibilityTextSelectable = false;
  /**
   * Opt-in (React extension, off by default): the control's text is rendered as real, invisible DOM text in the
   * accessibility overlay with pointer events, so the browser selects / copies it like HTML. Pointer input over the
   * text then goes to the selection, not to the drawn control — never enable it on gesture-driven controls.
   */
  get AccessibilityTextSelectable(): boolean { return this.accessibilityTextSelectable; }
  set AccessibilityTextSelectable(v: boolean) { if (this.accessibilityTextSelectable !== v) { this.accessibilityTextSelectable = v; this.AccessibilityChanged(); } }
  /** Text lines for AccessibilityTextSelectable in CSS px relative to the node (SkiaLabel implements it). */
  GetAccessibilityTextLines(_scale: number): import("./Accessibility").AccessibilityTextLine[] { return []; }

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
    if (!this.effectsMarginCache || this.effectsMarginCache.scale !== scale) {
      let m = this.ComputeEffectsMargin(scale);
      if (!this.DisableEffects) for (const e of this.visualEffects) { // C# ComputeEffectsMargin: per-side max over the attached effects
        const em = e.GetEffectMargin(scale);
        if (em.Left > m.Left || em.Top > m.Top || em.Right > m.Right || em.Bottom > m.Bottom) m = new Thickness(Math.max(m.Left, em.Left), Math.max(m.Top, em.Top), Math.max(m.Right, em.Right), Math.max(m.Bottom, em.Bottom));
      }
      this.effectsMarginCache = { scale, margin: m };
    }
    return this.effectsMarginCache.margin;
  }

  /** DrawingRect grown by the effects margin, in pixels — the rect a cache is recorded for. */
  protected ExpandedCacheRect(scale: number): SKRect {
    const r = this.DrawingRect, m = this.EffectsMargin(scale);
    if (m.Left === 0 && m.Top === 0 && m.Right === 0 && m.Bottom === 0) return r;
    return new SKRect(r.Left - Math.ceil(m.Left), r.Top - Math.ceil(m.Top), r.Right + Math.ceil(m.Right), r.Bottom + Math.ceil(m.Bottom));
  }

  Render(ctx: DrawingContext): void {
    if (!this.IsVisible || this.Opacity <= 0 || this.IsDisposed) return;
    const canvas = ctx.Context.Canvas;
    const applyOpacity = this.Opacity < 1;
    // C# Left/Top: a cached control is blitted at an offset, no matrix and no save/restore around the subtree
    const offsetOnly = (this.Left !== 0 || this.Top !== 0) && this.UsingCacheType !== "None" && this.TranslationX === 0 && this.TranslationY === 0
      && this.Rotation === 0 && this.ScaleX === 1 && this.ScaleY === 1 && this.SkewX === 0 && this.SkewY === 0;
    const needTransform = this.HasTransform && !offsetOnly;
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
    let dx = 0, dy = 0;
    if (needTransform) {
      this.RenderTransformMatrix = this.CreateRenderTransformMatrix(this.DrawingRect, ctx.Scale);
      canvas.concat(this.RenderTransformMatrix);
    } else if (offsetOnly) {
      dx = this.Left * ctx.Scale; dy = this.Top * ctx.Scale;
      this.RenderTransformMatrix = Super.CK.Matrix.translated(dx, dy); // gestures / accessibility still map through it
    } else {
      this.RenderTransformMatrix = undefined;
    }
    if (this.IsClippedToBounds) {
      if (!saved) { canvas.save(); saved = true; }
      const c = this.ClipEffects ? this.DrawingRect : this.ExpandedCacheRect(ctx.Scale);
      canvas.clipRect(Super.CK.LTRBRect(c.Left + dx, c.Top + dy, c.Right + dx, c.Bottom + dy), Super.CK.ClipOp.Intersect, true);
    }
    this.RenderContent(ctx, dx, dy);
    if (saved) canvas.restore();
  }

  /** Port of DrawnUi ApplyTransforms: T(-pivot) · rotation · scale/skew · T(pivot) · translation, in canvas pixels. */
  protected CreateRenderTransformMatrix(destination: SKRect, scale: number): number[] {
    const M = Super.CK.Matrix;
    const moveX = (this.TranslationX + this.Left) * scale, moveY = (this.TranslationY + this.Top) * scale;
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

  /**
   * Image caches are rasterized over whole device pixels: the expanded rect snapped outward to integers, so the blit
   * lands 1:1 (no sub-pixel resample) and effects sampling the cache at `fragCoord - iOffset` hit texel centers.
   */
  private AlignedCacheRect(scale: number): SKRect {
    const r = this.ExpandedCacheRect(scale);
    return new SKRect(Math.floor(r.Left), Math.floor(r.Top), Math.ceil(r.Right), Math.ceil(r.Bottom));
  }

  /** Cache blit or live paint, then post animators — the part a transform/opacity layer wraps. */
  private RenderContent(ctx: DrawingContext, dx = 0, dy = 0): void {
    const dest = dx !== 0 || dy !== 0 ? new SKRect(this.DrawingRect.Left + dx, this.DrawingRect.Top + dy, this.DrawingRect.Right + dx, this.DrawingRect.Bottom + dy) : this.DrawingRect;
    const own: DrawingContext = { ...ctx, Destination: dest };
    const cacheType = this.UsingCacheType;
    const post = this.ActivePostRenderers();
    if (cacheType === "None") {
      this.DestroyRenderingObject();
      this.PaintContent(own);
      // C# DrawDirectInternal: post renderers run after the direct paint, snapshotting what was painted
      for (const e of post) e.Render(own);
    } else {
      const r = cacheType === "Operations" ? this.ExpandedCacheRect(ctx.Scale) : this.AlignedCacheRect(ctx.Scale);
      const stale = this.cacheDirty || !this.RenderObject || this.RenderObject.Type !== cacheType
        || this.RenderObject.Scale !== ctx.Scale
        || Math.round(this.RenderObject.Bounds.Width) !== Math.round(r.Width) || Math.round(this.RenderObject.Bounds.Height) !== Math.round(r.Height);
      if (stale) this.CreateRenderingObject({ ...ctx, Destination: this.DrawingRect }, cacheType);
      if (this.RenderObject) {
        // C# DrawRenderObject: with post renderers an Image cache is not blitted, the effects sample it (CachedImage)
        // and paint the result; a picture cache has no texture, so it is replayed first and snapshotted by the effect
        if (post.length === 0 || !this.RenderObject.Image) this.RenderObject.Draw(ctx.Context.Canvas, r.Left + dx, r.Top + dy);
        for (const e of post) e.Render(own);
      }
      else if (this.RenderObjectPrevious) this.RenderObjectPrevious.Draw(ctx.Context.Canvas, r.Left + dx, r.Top + dy); // double buffer: last good frame
      else this.DrawPlaceholder(own);
    }
    this.ExecutePostAnimators(own);
  }

  /** Background + Paint(): the part of the control that a cache captures. */
  protected PaintContent(ctx: DrawingContext): void {
    if (this.BackgroundColor || (this.FillGradient && this.FillGradientPaintsBackground()) || this.PaintsBackgroundWithoutColor()) this.PaintBackground(ctx);
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
    // includes what shadows/effects paint outside the box; image caches on whole pixels
    const r = cacheType === "Operations" ? this.ExpandedCacheRect(ctx.Scale) : this.AlignedCacheRect(ctx.Scale);
    const w = Math.max(1, Math.round(r.Width)), h = Math.max(1, Math.round(r.Height));
    if (cacheType === "ImageDoubleBuffered") {
      // keep the previous frame as the fallback until the new cache exists
      if (this.RenderObject) { this.DisposePrevious(); this.RenderObjectPrevious = this.RenderObject; this.RenderObject = undefined; }
    } else if (cacheType !== "ImageComposite") { // a composite keeps its previous object to patch it
      this.DestroyRenderingObject();
    }
    if (cacheType === "Operations") {
      const recorder = new CK.PictureRecorder();
      const canvas = recorder.beginRecording(CK.LTRBRect(r.Left, r.Top, r.Right, r.Bottom));
      this.PaintContent({ ...ctx, Context: { ...ctx.Context, Canvas: canvas, Recording: true } });
      const picture = recorder.finishRecordingAsPicture();
      recorder.delete();
      this.RenderObject = new CachedObject("Operations", r, ctx.Scale, picture);
    } else if (cacheType === "ImageComposite") {
      this.CreateCompositeRenderingObject(ctx, r, w, h);
    } else {
      const main = ctx.Context.Surface;
      if (!main) { this.PaintContent(ctx); return; } // no surface to derive from (e.g. recording): draw live
      const offscreen = main.makeSurface({ ...main.imageInfo(), width: w, height: h });
      if (!offscreen) { if (!this.RenderObjectPrevious) this.PaintContent(ctx); return; }
      const canvas = offscreen.getCanvas();
      canvas.clear(CK.TRANSPARENT);
      canvas.translate(-r.Left, -r.Top);
      this.PaintContent({ ...ctx, Context: { Canvas: canvas, Surface: offscreen, Origin: { X: r.Left, Y: r.Top } } });
      const image = offscreen.makeImageSnapshot();
      offscreen.delete();
      this.RenderObject = new CachedObject(cacheType, r, ctx.Scale, undefined, image);
    }
    this.cacheDirty = false;
  }

  /**
   * C# ImageComposite (SetupRenderingWithComposition + CreateRenderingObject reuse): the offscreen surface survives
   * between records; when only some children changed (RepaintComposition from a child, no remeasure of this control)
   * their old and new bounds — plus every sibling they overlap — are erased and just those children are painted
   * again. Anything else (own content, structure, size, scale) records fully.
   */
  private CreateCompositeRenderingObject(ctx: DrawingContext, r: SKRect, w: number, h: number): void {
    const CK = Super.CK;
    const main = ctx.Context.Surface;
    if (!main) { this.PaintContent(ctx); return; }
    let surface = this.compositeSurface;
    const prev = this.RenderObject;
    const sameGeometry = !!surface && !!prev && prev.Type === "ImageComposite" && prev.Scale === ctx.Scale
      && Math.round(prev.Bounds.Width) === w && Math.round(prev.Bounds.Height) === h;
    if (!sameGeometry) { surface?.delete(); surface = main.makeSurface({ ...main.imageInfo(), width: w, height: h }) ?? undefined; this.compositeSurface = surface; }
    if (!surface) { this.PaintContent(ctx); return; }
    const canvas = surface.getCanvas();
    const children = this.GetCompositeChildren();
    const partial = sameGeometry && !this.compositeFull && this.DirtyChildrenInternal.size > 0 && children.length > 0;
    const offset = prev && sameGeometry ? { X: r.Left - prev.Bounds.Left, Y: r.Top - prev.Bounds.Top } : { X: 0, Y: 0 };
    const saved = canvas.save();
    canvas.translate(-r.Left, -r.Top);
    if (partial) {
      // dirty = reported children + siblings intersecting their old or new bounds (C# makes intersecting children dirty too)
      const dirty = new Set<SkiaControl>();
      const rects: SKRect[] = [];
      const boundsOf = (c: SkiaControl): SKRect[] => {
        const out = [c.GetTransformedDirtyBounds()];
        if (c.lastCompositeBounds) out.push(new SKRect(c.lastCompositeBounds.Left + offset.X, c.lastCompositeBounds.Top + offset.Y, c.lastCompositeBounds.Right + offset.X, c.lastCompositeBounds.Bottom + offset.Y));
        return out;
      };
      for (const c of this.DirtyChildrenInternal) if (children.includes(c)) { dirty.add(c); rects.push(...boundsOf(c)); }
      let grew = true;
      while (grew) {
        grew = false;
        for (const c of children) {
          if (dirty.has(c) || !c.IsVisible) continue;
          const own = boundsOf(c);
          if (rects.some((d) => own.some((o) => o.Left < d.Right && d.Left < o.Right && o.Top < d.Bottom && d.Top < o.Bottom))) { dirty.add(c); rects.push(...own); grew = true; }
        }
      }
      const clip = new CK.PathBuilder();
      const erase = new CK.Paint(); erase.setBlendMode(CK.BlendMode.Clear);
      for (const d of rects) { const l = Math.floor(d.Left), t = Math.floor(d.Top), rr = Math.ceil(d.Right), b = Math.ceil(d.Bottom); clip.addRect(CK.LTRBRect(l, t, rr, b)); canvas.drawRect(CK.LTRBRect(l, t, rr, b), erase); }
      erase.delete();
      const path = clip.detach(); clip.delete();
      canvas.clipPath(path, CK.ClipOp.Intersect, false);
      path.delete();
      this.IsRenderingWithComposition = true;
      this.DirtyChildrenInternal.clear();
      for (const c of dirty) this.DirtyChildrenInternal.add(c);
      this.PaintContent({ ...ctx, Context: { Canvas: canvas, Surface: surface, Origin: { X: r.Left, Y: r.Top } } });
      this.IsRenderingWithComposition = false;
      this.LastCompositeRecord = { Mode: "partial", Children: dirty.size, Dirty: [...dirty] };
    } else {
      canvas.clear(CK.TRANSPARENT);
      this.PaintContent({ ...ctx, Context: { Canvas: canvas, Surface: surface, Origin: { X: r.Left, Y: r.Top } } });
      this.LastCompositeRecord = { Mode: "full", Children: children.length, Dirty: children };
    }
    canvas.restoreToCount(saved);
    for (const c of children) c.lastCompositeBounds = c.IsVisible ? c.GetTransformedDirtyBounds() : undefined;
    this.DirtyChildrenInternal.clear();
    this.compositeFull = false;
    const image = surface.makeImageSnapshot();
    const old = this.RenderObject;
    this.RenderObject = new CachedObject("ImageComposite", r, ctx.Scale, undefined, image);
    if (old) { const sv = this.Superview; if (sv) sv.DisposeObject(old); else old.Dispose(); }
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

  /** Marks the cache stale; re-recorded on the next frame (a composite records fully: its own content changed). */
  InvalidateCache(): void { this.cacheDirty = true; this.compositeFull = true; }
  /** Effects margin recomputed on the next use (C# InvalidateEffectsMargin). */
  InvalidateEffectsMargin(): void { this.effectsMarginCache = undefined; }

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
    if (this.FillGradient && this.FillGradientPaintsBackground()) this.SetupGradient(paint, this.FillGradient, rect);
    return paint;
  }

  /** Whether FillGradient fills the background (C# base: yes, even without BackgroundColor; SkiaLabel: only the glyphs unless BackgroundColor is set). */
  protected FillGradientPaintsBackground(): boolean { return true; }

  /** Shaders built for a gradient object, keyed by the rect they were built for (C# cached SetupGradient overload). */
  private gradientShaders?: Map<SkiaGradient, Map<string, import("canvaskit-wasm").Shader>>;

  /** C# SetupGradient: white base color, the gradient's BlendMode and a (cached) shader on the paint. */
  SetupGradient(paint: import("canvaskit-wasm").Paint, gradient: SkiaGradient, rect: SKRect): boolean {
    const CK = Super.CK;
    const key = `${rect.Left},${rect.Top},${rect.Width},${rect.Height}|${this.Value1},${this.Value2}`;
    this.gradientShaders ??= new Map();
    let byRect = this.gradientShaders.get(gradient);
    if (!byRect) {
      // a new gradient object (React re-render literal) replaces the old ones: drop their shaders
      for (const m of this.gradientShaders.values()) for (const s of m.values()) s.delete();
      this.gradientShaders.clear();
      byRect = new Map();
      this.gradientShaders.set(gradient, byRect);
    }
    let shader = byRect.get(key);
    if (!shader) {
      const made = this.CreateGradient(rect, gradient);
      if (!made) return false;
      if (byRect.size >= 32) { for (const s of byRect.values()) s.delete(); byRect.clear(); } // GradientByLines on long text
      byRect.set(key, made);
      shader = made;
    }
    paint.setColor(CK.WHITE);
    const blend = gradient.BlendMode ? (CK.BlendMode as unknown as Record<string, import("canvaskit-wasm").BlendMode>)[gradient.BlendMode] : undefined;
    if (blend) paint.setBlendMode(blend);
    paint.setShader(shader);
    return true;
  }

  /** Port of C# CreateGradient: Linear / Circular / Oval / Sweep shader over the rect (pixels); null for None. */
  CreateGradient(rect: SKRect, g: SkiaGradient): import("canvaskit-wasm").Shader | null {
    const type = g.Type ?? "Linear";
    if (type === "None" || !g.Colors || g.Colors.length === 0) return null;
    const CK = Super.CK;
    const light = g.Light ?? 1, opacity = g.Opacity ?? 1;
    const colors = g.Colors.map((c) => {
      const p = Super.ParseColor(c);
      let rgb: [number, number, number] = [p[0], p[1], p[2]];
      if (light !== 1) rgb = SkiaControl.AdjustLightness(rgb, light);
      return Float32Array.of(rgb[0], rgb[1], rgb[2], p[3] * opacity);
    });
    const positions = g.ColorPositions && g.ColorPositions.length === colors.length ? g.ColorPositions : null;
    const tile = (CK.TileMode as unknown as Record<string, import("canvaskit-wasm").TileMode>)[g.TileMode ?? "Clamp"] ?? CK.TileMode.Clamp;
    switch (type) {
      case "Sweep":
        return CK.Shader.MakeSweepGradient(rect.Left + rect.Width / 2, rect.Top + rect.Height / 2, colors, positions, tile, null, 0, this.Value1, this.Value1 + (this.Value2 || 360));
      case "Circular": case "Conical": case "Oval": {
        const cx = rect.Left + (g.StartXRatio ?? 0) * rect.Width, cy = rect.Top + (g.StartYRatio ?? 0) * rect.Height;
        if (type !== "Oval") return CK.Shader.MakeRadialGradient([cx, cy], Math.min(rect.Width / 2, rect.Height / 2), colors, positions, tile);
        const scaleX = rect.Width >= rect.Height ? 1 : rect.Width / rect.Height;
        const scaleY = rect.Height >= rect.Width ? 1 : rect.Height / rect.Width;
        return CK.Shader.MakeRadialGradient([cx, cy], Math.max(rect.Width / 2, rect.Height / 2), colors, positions, tile, CK.Matrix.scaled(scaleX, scaleY, cx, cy));
      }
      default: {
        let sx = g.StartXRatio ?? 0, sy = g.StartYRatio ?? 0, ex = g.EndXRatio ?? 0, ey = g.EndYRatio ?? 1;
        if (g.Angle != null) [sx, sy, ex, ey] = SkiaControl.LinearGradientAngleToPoints(g.Angle);
        return CK.Shader.MakeLinearGradient([rect.Left + rect.Width * sx, rect.Top + rect.Height * sy], [rect.Left + rect.Width * ex, rect.Top + rect.Height * ey], colors, positions, tile);
      }
    }
  }

  /** C# SkiaGradient.LinearGradientAngleToPoints (CSS angle -> start/end ratios). */
  static LinearGradientAngleToPoints(direction: number): [number, number, number, number] {
    direction -= 90;
    if (direction < 0) direction = 360 + direction;
    if (direction > 360) direction = 360;
    const eps = Math.pow(2, -52);
    const angle = direction % 360;
    const rad = (d: number) => (d * Math.PI) / 180;
    let sx = Math.cos(rad(180 - angle)), sy = Math.sin(rad(180 - angle)), ex = Math.cos(rad(360 - angle)), ey = Math.sin(rad(360 - angle));
    if (sx <= 0 || Math.abs(sx) <= eps) sx = 0;
    if (sy <= 0 || Math.abs(sy) <= eps) sy = 0;
    if (ex <= 0 || Math.abs(ex) <= eps) ex = 0;
    if (ey <= 0 || Math.abs(ey) <= eps) ey = 0;
    return [sx, sy, ex, ey];
  }

  /** C# MakeDarker / MakeLighter: HSL lightness scaled down (light < 1) or pushed toward white (light > 1). */
  static AdjustLightness(rgb: [number, number, number], light: number): [number, number, number] {
    const [r, g, b] = rgb;
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h = 0, s = 0;
    let l = (max + min) / 2;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      if (max === r) h = (g - b) / d + (g < b ? 6 : 0); else if (max === g) h = (b - r) / d + 2; else h = (r - g) / d + 4;
      h /= 6;
    }
    l = light < 1 ? l * light : l + (1 - l) * (light - 1);
    l = Math.max(0, Math.min(1, l));
    if (s === 0) return [l, l, l];
    const hue = (p: number, q: number, t: number) => { if (t < 0) t += 1; if (t > 1) t -= 1; if (t < 1 / 6) return p + (q - p) * 6 * t; if (t < 1 / 2) return q; if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6; return p; };
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    return [hue(p, q, h + 1 / 3), hue(p, q, h), hue(p, q, h - 1 / 3)];
  }

  /** Override to draw own content into ctx.Destination. */
  protected Paint(_ctx: DrawingContext): void {}

  // ---- invalidation (same names as DrawnUi) ----

  // ---- disposal (C# Dispose / OnDisposing) ----
  IsDisposed = false;

  /**
   * Frees what the control owns: caches, gradient shaders, running animations and overlay effects, the accessibility
   * node, then the children. Called by the React renderer when the element leaves the tree (detachDeletedInstance).
   */
  Dispose(): void {
    if (this.IsDisposed) return;
    this.IsDisposed = true;
    this.OnDisposing();
    for (const c of this.ownAnimations.values()) c.abort();
    this.ownAnimations.clear();
    for (const e of [...this.PostAnimators]) (e as { Stop?: () => void }).Stop?.();
    this.PostAnimators.length = 0;
    this.DestroyRenderingObject();
    this.compositeSurface?.delete(); this.compositeSurface = undefined;
    for (const e of this.visualEffects) e.Dispose(); // C# disposes attached effects with the control
    this.visualEffects = []; this.EffectPostRenderers = [];
    if (this.gradientShaders) { for (const m of this.gradientShaders.values()) for (const s of m.values()) s.delete(); this.gradientShaders.clear(); }
    this.UnregisterAccessibility();
    this.DisposeChildren();
    this.Parent = undefined;
  }
  /** Subclasses release their own native objects here (before the base frees caches and children). */
  protected OnDisposing(): void {}
  /** Layouts dispose their children here. */
  protected DisposeChildren(): void {}

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
    let child: SkiaControl = this, p = this.Parent;
    while (p) { p.cacheDirty = true; p.TrackChildAsDirty(child); child = p; p = p.Parent; }
    this.Repaint();
  }

  /** Size or content may have changed: this control and all ancestors remeasure and re-record. */
  InvalidateMeasure(): void {
    this.NeedMeasure = true;
    this.cacheDirty = true;
    this.compositeFull = true; // structure may change: a composite cannot patch it
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

  /**
   * Routes a context-menu request like a tap: visible, non-transparent children under the point first (top-most
   * first, each in its own transformed space), then this control's ContextMenu handler. True = handled.
   */
  ProcessContextMenu(point: SKPoint, e: ContextMenuEventArgs): boolean {
    const listeners = this.GetGestureListeners();
    for (let i = listeners.length - 1; i >= 0; i--) {
      const listener = listeners[i];
      if (!listener.IsVisible || listener.InputTransparent || !this.IsGestureForChild(listener, point)) continue;
      if (listener.ProcessContextMenu(listener.TransformPointToLocalSpace(point), e)) return true;
    }
    if (!this.ContextMenu) return false;
    e.Control = this;
    e.Local = new SKPoint(point.X - this.DrawingRect.Left, point.Y - this.DrawingRect.Top);
    return this.ContextMenu(this, e) === true;
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

    // C# EffectsGestureProcessors: attached effects that process gestures (ISkiaGestureProcessor) see them first
    if (!this.DisableEffects) for (const e of this.visualEffects) {
      const p = (e as unknown as { ProcessGestures?: (a: SkiaGesturesParameters, i: GestureEventProcessingInfo) => SkiaControl | null }).ProcessGestures;
      if (typeof p === "function" && p.call(e, args, apply) && args.Type !== "Up") return this;
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
