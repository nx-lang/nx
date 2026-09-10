import type { DrawingContext, SkiaControl } from "../core/SkiaControl";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import { Easing } from "../core/Easing";
import { Spring } from "../core/ScrollAnimators";
import { SKRect, ScaledSize, Thickness } from "../core/Types";
import { SnappingLayout } from "./SnappingLayout";

export interface SnapPoint { Id: number; Location: SKPoint }
interface ChildPosition { Offset: SKPoint; OnScreen: boolean; NextToScreen: boolean }

/**
 * Mirrors DrawnUi SkiaCarousel: every child (static `Children` or a recycled `ItemsSource` + `ItemTemplate` cell)
 * is a full-size slide laid along the axis, `SidesOffset` peeks the neighbours, `Spacing` separates slides; a
 * swipe / release snaps to the nearest slide by velocity, `SelectedIndex` drives and reports the position,
 * `IsLooped` wraps around through virtual anchors, `PreloadNeighboors` draws the hidden neighbours,
 * `DynamicSize` sizes an auto-sized carousel from the selected slide, `SwipeSpeed` / `LinearSpeedMs` tune the snap.
 */
export class SkiaCarousel extends SnappingLayout {
  IsVertical = false;
  /** Side padding in points so the previous/next slides peek in. */
  SidesOffset = 0;
  /** Accepted for parity; C# marks it TODO. */
  IsRightToLeft = false;
  /** Wrap from the last slide to the first and back. */
  IsLooped = false;
  /** Draw the hidden neighbour slides too (preloads images etc.), C# default true. */
  PreloadNeighboors = true;
  /** An auto-sized carousel takes the size of the selected slide instead of the largest one. */
  DynamicSize = false;
  /** Multiplies the snap velocity / spring stiffness (C# SwipeSpeed). */
  SwipeSpeed = 1;
  /** Milliseconds a whole slide takes when snapping without Bounces; 0 = derived from the velocity (C# LinearSpeedMs). */
  LinearSpeedMs = 0;
  SelectedIndexChanged?: (sender: SkiaCarousel, index: number) => void;
  ItemAppearing?: (sender: SkiaCarousel, index: number) => void;
  ItemDisappearing?: (sender: SkiaCarousel, index: number) => void;

  private selectedIndex = 0;
  private lastIndex = -1;
  private childrenInitialized = false;
  private layoutKey = "";
  private lastLooped = false;
  private cellSize = { W: 0, H: 0 };
  /** Slide size in points (C# CellSize). */
  protected get CellSize(): { W: number; H: number } { return this.cellSize; }
  private panningOffset = SKPoint.Empty;
  private panningStart = SKPoint.Empty;
  private wrongDirection = false;
  private snapIfNoPanOnUp = false;
  private hadDown = false;
  private itemsVisibility: boolean[] = [];
  private snapPointsVirtual?: SnapPoint[];
  private scrollAmount = 0;
  /** Slides drawn on screen by the last frame: the only gesture targets (C# rendering tree). */
  private visibleViews: SkiaControl[] = [];

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.IsClippedToBounds = true; // C# WillClipBounds => true
    this.SnapEasing = Easing.SinOut;
  }

  // ---- state ----
  get SelectedIndex(): number { return this.selectedIndex; }
  set SelectedIndex(v: number) {
    if (this.selectedIndex === v) return;
    this.InterruptSnapping();
    this.lastIndex = this.selectedIndex;
    this.selectedIndex = v;
    this.OnSelectedIndexChanged();
    if (this.SnapPoints.length > 0 && !this.IsUserPanning) this.ApplyIndex(false);
  }
  /** Previously selected index (C# LastIndex). */
  get LastIndex(): number { return this.lastIndex; }
  get MaxIndex(): number { return this.ChildrenTotal - 1; }
  get ChildrenTotal(): number { return this.IsTemplated ? (this.ItemsSource?.length ?? 0) : this.Children.length; }
  get ChildrenCount(): number { return this.ChildrenTotal; }
  get IsAtStart(): boolean { return this.selectedIndex === 0; }
  get IsAtEnd(): boolean { return this.selectedIndex === this.MaxIndex; }
  /** 0..1 progress of the content along all slides (C# ScrollProgress). */
  get ScrollProgress(): number {
    const last = this.SnapPoints[this.SnapPoints.length - 1];
    if (!last) return 0;
    const max = this.IsVertical ? last.Y : last.X;
    return max === 0 ? 0 : (this.IsVertical ? this.CurrentPosition.Y : this.CurrentPosition.X) / max;
  }
  /** Scroll amount of the selected slide, 0..1 of the track (C# ScrollAmount). */
  get ScrollAmount(): number { return this.scrollAmount; }
  /** Fraction of the current transition between two slides (C# TransitionProgress). */
  get TransitionProgress(): number {
    if (this.MaxIndex < 1) return 0;
    const scaled = this.ScrollProgress * this.MaxIndex;
    return scaled - Math.floor(scaled);
  }

  private OnSelectedIndexChanged(): void {
    this.SelectedIndexChanged?.(this, this.selectedIndex);
    this.NotifyAccessibility();
    if (this.DynamicSize) this.InvalidateMeasure();
  }

  /** Index reached by scrolling: state only, no snap (C# SelectedIndex set from UpdateReportedPosition). */
  private ReportIndex(i: number): void {
    if (i === this.selectedIndex) return;
    this.lastIndex = this.selectedIndex;
    this.selectedIndex = i;
    this.OnSelectedIndexChanged();
  }

  GoNext(): void {
    if (this.selectedIndex < this.MaxIndex) this.SelectedIndex = this.selectedIndex + 1;
    else if (this.IsLooped) this.SelectedIndex = 0;
  }
  GoPrev(): void {
    if (this.selectedIndex > 0) this.SelectedIndex = this.selectedIndex - 1;
    else if (this.IsLooped) this.SelectedIndex = this.MaxIndex;
  }
  /** C# ScrollTo(index, animate). */
  ScrollTo(index: number, animate = true): void {
    const i = Math.max(0, Math.min(this.MaxIndex, index));
    if (i === this.selectedIndex) { this.ApplyIndex(!animate); return; }
    if (!animate) { this.InterruptSnapping(); this.lastIndex = this.selectedIndex; this.selectedIndex = i; this.OnSelectedIndexChanged(); this.ApplyIndex(true); return; }
    this.SelectedIndex = i;
  }

  private get ApplyLoopedLogic(): boolean { return this.IsLooped && this.Animated && this.CanAnimate && this.SnapPoints.length > 1; }

  /** Step between slides in points: slide size + Spacing - 2 * SidesOffset (C# InitializeChildren). */
  private Step(): number { const size = this.IsVertical ? this.cellSize.H : this.cellSize.W; return size + this.Spacing - 2 * this.SidesOffset; }

  /**
   * C# InterruptSnapping: stops an in-flight snap before a programmatic index change and continues from the visual
   * position; for IsLooped the position is shifted by whole strips into the neighbourhood of the current index.
   */
  private InterruptSnapping(): void {
    if (!this.IsSnapAnimating) return;
    this.StopSnapAnimators();
    let pos = this.CurrentPosition;
    if (this.IsLooped && this.SnapPoints.length > 1 && this.selectedIndex >= 0 && this.selectedIndex < this.SnapPoints.length) {
      const n = this.SnapPoints.length;
      const strip = new SKPoint((this.SnapPoints[1].X - this.SnapPoints[0].X) * n, (this.SnapPoints[1].Y - this.SnapPoints[0].Y) * n);
      const target = this.SnapPoints[this.selectedIndex];
      const d = (p: SKPoint) => SnappingLayout.Dist(p, target);
      while (d(new SKPoint(pos.X + strip.X, pos.Y + strip.Y)) < d(pos)) pos = new SKPoint(pos.X + strip.X, pos.Y + strip.Y);
      while (d(new SKPoint(pos.X - strip.X, pos.Y - strip.Y)) < d(pos)) pos = new SKPoint(pos.X - strip.X, pos.Y - strip.Y);
    }
    this.CurrentPosition = pos;
    this.CurrentSnap = pos;
  }

  private ApplyIndex(instant: boolean): void {
    if (this.selectedIndex < 0 || this.selectedIndex >= this.SnapPoints.length) return;
    let target = this.SnapPoints[this.selectedIndex];
    if (!instant && this.ApplyLoopedLogic) {
      if (this.selectedIndex === 0 && this.lastIndex === this.MaxIndex) target = this.GetVirtualSnapPoints().find((p) => p.Id === -2)!.Location;
      else if (this.selectedIndex === this.MaxIndex && this.lastIndex === 0) target = this.GetVirtualSnapPoints().find((p) => p.Id === -1)!.Location;
    }
    this.ScrollToOffset(target, SKPoint.Empty, !instant && this.CanAnimate && this.Animated);
  }

  override UpdateReportedPosition(): void {
    if (this.SnapPoints.length === 0) return;
    if (this.IsLooped && this.SnapPoints.length > 1) {
      const c = this.GetVirtualAnchor(this.CurrentSnap);
      this.ReportIndex(c.Id === -1 ? this.MaxIndex : c.Id === -2 ? 0 : c.Id);
    } else {
      const i = this.SnapPoints.findIndex((p) => Math.abs(p.X - this.CurrentSnap.X) <= 1 && Math.abs(p.Y - this.CurrentSnap.Y) <= 1);
      if (i >= 0 && i < this.ChildrenTotal) this.ReportIndex(i);
    }
  }

  override ApplyPosition(position: SKPoint): void {
    super.ApplyPosition(position);
    this.OnScrollProgressChanged();
    this.RepaintComposition();
  }
  /** The position changed (C# OnScrollProgressChanged); SkiaShaderCarousel drives its transition here. */
  protected OnScrollProgressChanged(): void {}
  /** Snap points were (re)built (C# OnChildrenInitialized). */
  protected OnChildrenInitialized(): void {}
  /** Where a drawn slide goes for its scroll offset; SkiaShaderCarousel keeps every slide in place (C# AnimateVisibleChild). */
  protected SlideOffset(offset: SKPoint): SKPoint { return offset; }
  /** An already realized slide (C# ChildrenFactory.GetViewForIndex): no cell is created. */
  protected GetExistingView(index: number): SkiaControl | undefined { return this.IsTemplated ? this.ChildrenFactory.GetViewForIndex(index) : this.Children[index]; }

  // ---- looped: virtual anchors one step before the first and after the last slide ----
  private GetVirtualSnapPoints(): SnapPoint[] {
    if (!this.snapPointsVirtual) {
      const s = this.SnapPoints, n = s.length;
      const dx = s[1].X - s[0].X, dy = s[1].Y - s[0].Y;
      this.snapPointsVirtual = [
        { Id: -1, Location: new SKPoint(s[0].X - dx, s[0].Y - dy) },
        { Id: -2, Location: new SKPoint(s[n - 1].X + dx, s[n - 1].Y + dy) },
        ...s.map((p, i) => ({ Id: i, Location: p })),
      ];
    }
    return this.snapPointsVirtual;
  }

  protected GetVirtualAnchor(current: SKPoint): SnapPoint {
    let best = this.GetVirtualSnapPoints()[0], d = Infinity;
    for (const p of this.GetVirtualSnapPoints()) { const dd = SnappingLayout.Dist(p.Location, current); if (dd < d) { d = dd; best = p; } }
    return best;
  }

  protected override FindNearestAnchorInternal(current: SKPoint, velocity: SKPoint): SKPoint {
    if (this.IsLooped && this.SnapPoints.length > 1) {
      const c = this.GetVirtualAnchor(current);
      if (c.Id === -1) return this.SnapPoints[this.SnapPoints.length - 1];
      if (c.Id === -2) return this.SnapPoints[0];
      return c.Location;
    }
    return super.FindNearestAnchorInternal(current, velocity);
  }

  /** C# SkiaCarousel.ScrollToNearestAnchor: velocity below 100 counts as none; looped snaps around virtual anchors. */
  override ScrollToNearestAnchor(location: SKPoint, velocity: SKPoint): void {
    velocity = new SKPoint(Math.abs(velocity.X) < 100 ? 0 : velocity.X, Math.abs(velocity.Y) < 100 ? 0 : velocity.Y);
    if (this.ApplyLoopedLogic) {
      const origin = this.FindNearestAnchorInternal(location, velocity);
      const target = this.SelectNextAnchor(origin, velocity);
      if (SnappingLayout.Dist(location, target) >= 0.5) this.ScrollToOffset(target, velocity, this.CanAnimate);
      else this.UpdateReportedPosition();
      return;
    }
    super.ScrollToNearestAnchor(location, velocity);
  }

  /** C# SkiaCarousel.SelectNextAnchor: base choice, plus a wrap to the virtual anchor at the strip borders when looped. */
  override SelectNextAnchor(origin: SKPoint, velocity: SKPoint): SKPoint {
    if (!this.ApplyLoopedLogic) return super.SelectNextAnchor(origin, velocity);
    const baseTarget = super.SelectNextAnchor(origin, velocity);
    let originSnap = this.FindNearestAnchorInternal(origin, velocity);
    let originIndex = this.SnapPoints.indexOf(originSnap);
    if (originIndex < 0) { originSnap = this.FindNearestAnchor(origin); originIndex = this.SnapPoints.indexOf(originSnap); }
    if (originSnap.X !== baseTarget.X || originSnap.Y !== baseTarget.Y) return baseTarget;

    const axis = (p: SKPoint) => (this.IsVertical ? p.Y : p.X);
    const stepAxis = this.SnapPoints.length > 1 ? Math.abs(axis(this.SnapPoints[1]) - axis(this.SnapPoints[0])) : 0;
    const disp = axis(this.CurrentPosition) - axis(this.panningStart);
    let dirSign = Math.sign(axis(velocity));
    if (dirSign === 0) {
      // tiny velocity at finger-up: infer from the pan displacement, past SnapDistanceRatio of a step
      const minDisplacement = stepAxis > 0 ? stepAxis * this.SnapDistanceRatio : 0;
      dirSign = stepAxis <= 0 || Math.abs(disp) < minDisplacement ? 0 : Math.sign(disp);
    }
    if (dirSign === 0) return baseTarget;
    // zero velocity needs stronger intent (>= 50% of a step) before wrapping
    if (velocity.X === 0 && velocity.Y === 0 && stepAxis > 0 && Math.abs(disp) < stepAxis * 0.5) return baseTarget;
    if (dirSign < 0 && originIndex === this.MaxIndex) return this.GetVirtualSnapPoints().find((p) => p.Id === -2)!.Location;
    if (dirSign > 0 && originIndex === 0) return this.GetVirtualSnapPoints().find((p) => p.Id === -1)!.Location;
    return baseTarget;
  }

  /** C# FixIndex: a virtual anchor becomes the real one, the position keeps its offset (visually identical, looped drawing wraps). */
  private FixIndex(): void {
    if (this.IsLooped && this.SnapPoints.length > 1 && this.selectedIndex >= 0 && this.selectedIndex < this.SnapPoints.length) {
      const snap = this.GetVirtualAnchor(this.CurrentPosition);
      const ox = this.CurrentPosition.X - snap.Location.X, oy = this.CurrentPosition.Y - snap.Location.Y;
      const real = this.SnapPoints[this.selectedIndex];
      this.CurrentSnap = real;
      this.CurrentPosition = new SKPoint(real.X + ox, real.Y + oy);
    }
  }

  private FixPosition(): void {
    if (this.IsLooped && this.SnapPoints.length > 1) {
      const v = this.GetVirtualAnchor(this.CurrentSnap);
      if (v.Id === -1 || v.Id === -2) this.FixIndex();
    }
  }

  protected override OnTransitionChanged(): void {
    if (!this.InTransition) this.FixPosition();
    super.OnTransitionChanged();
  }

  // ---- snap speed (C# SkiaCarousel.ScrollToOffset) ----
  protected override ScaleSnapVelocity(velocity: SKPoint): SKPoint { const k = this.SwipeSpeed / 2; return new SKPoint(velocity.X * k, velocity.Y * k); }
  protected override CreateSnapSpring(): Spring { return new Spring(1 * (1 + this.RubberDamping), 200 * this.SwipeSpeed, 0.5 * (1 + this.RubberDamping)); }
  protected override SpringVelocity(velocity: SKPoint): SKPoint { return new SKPoint(velocity.X * this.SwipeSpeed, velocity.Y * this.SwipeSpeed); }
  protected override GetSnapDurationSeconds(start: SKPoint, end: SKPoint, velocity: SKPoint, displacement: SKPoint): number {
    const speedK = this.SwipeSpeed / 2;
    const maxSpeed = 0.25 / speedK;
    const h = this.cellSize.H || 1; // C# normalizes both axes by Height
    let speed = maxSpeed;
    if (this.IsVertical) { if (velocity.Y !== 0) speed = Math.abs(displacement.Y / velocity.Y) * (Math.abs(end.Y - start.Y) / h); }
    else if (velocity.X !== 0) speed = Math.abs(displacement.X / velocity.X) * (Math.abs(end.X - start.X) / h);
    if (speed > maxSpeed) speed = maxSpeed;
    if (this.LinearSpeedMs > 0) {
      const denom = this.IsVertical ? this.cellSize.H : this.cellSize.W;
      const delta = this.IsVertical ? Math.abs(end.Y - start.Y) : Math.abs(end.X - start.X);
      if (denom > 0) speed = (delta / denom) * (this.LinearSpeedMs / 1000);
    }
    return speed;
  }

  // ---- children ----
  private InnerRect(): SKRect {
    const scale = this.RenderingScale, p = this.Padding, r = this.DrawingRect;
    return new SKRect(r.Left + p.Left * scale, r.Top + p.Top * scale, r.Right - p.Right * scale, r.Bottom - p.Bottom * scale);
  }

  /** C# AdaptTemplate: slides fill the carousel (Start only when the carousel auto-sizes on that axis), SidesOffset as margin. */
  private AdaptTemplate(view: SkiaControl): void {
    const vo = this.HeightRequest < 0 && this.VerticalOptions === "Start" ? "Start" : "Fill";
    const ho = this.WidthRequest < 0 && this.HorizontalOptions === "Start" ? "Start" : "Fill";
    if (view.VerticalOptions !== vo) view.VerticalOptions = vo;
    if (view.HorizontalOptions !== ho) view.HorizontalOptions = ho;
    const m = this.SidesOffset, cur = view.Margin;
    const want = this.IsVertical ? new Thickness(0, m, 0, m) : new Thickness(m, 0, m, 0);
    if (cur.Left !== want.Left || cur.Top !== want.Top || cur.Right !== want.Right || cur.Bottom !== want.Bottom) view.Margin = want;
  }

  private GetChild(index: number): SkiaControl | undefined {
    if (this.IsTemplated) { const v = this.ChildrenFactory.GetOrCreateViewForIndex(index); if (v) this.AdaptTemplate(v); return v; }
    return this.Children[index];
  }

  protected override MeasureAbsolute(w: number, h: number, scale: number): ScaledSize {
    const px = this.Padding.HorizontalThickness * scale, py = this.Padding.VerticalThickness * scale;
    const cw = isFinite(w) ? w - px : Infinity, ch = isFinite(h) ? h - py : Infinity;
    let maxW = 0, maxH = 0;
    if (!this.IsTemplated) {
      this.Children.forEach((v, i) => {
        if (!v.IsVisible) return;
        this.AdaptTemplate(v);
        const s = v.Measure(cw, ch, scale);
        if (this.DynamicSize) { if (i === this.selectedIndex) { maxW = s.Pixels.Width; maxH = s.Pixels.Height; } }
        else { maxW = Math.max(maxW, s.Pixels.Width); maxH = Math.max(maxH, s.Pixels.Height); }
      });
    } else if (this.ChildrenTotal > 0) {
      // C# measures one template; DynamicSize follows the selected slide (ApplyDynamicSize)
      const view = this.GetChild(this.DynamicSize ? Math.max(0, Math.min(this.MaxIndex, this.selectedIndex)) : 0);
      if (view) { const s = view.Measure(cw, ch, scale); maxW = s.Pixels.Width; maxH = s.Pixels.Height; }
    }
    return ScaledSize.FromPixels(isFinite(w) ? w : maxW + px, isFinite(h) ? h : maxH + py, scale);
  }

  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const inner = this.InnerRect();
    const cellW = inner.Width / scale, cellH = inner.Height / scale;
    // snap points depend on the axis size, orientation, Spacing and SidesOffset; a cross-axis change (DynamicSize) keeps them
    const key = `${this.IsVertical}|${this.Spacing}|${this.SidesOffset}|${this.IsVertical ? cellH : cellW}`;
    this.cellSize = { W: cellW, H: cellH };
    if (key !== this.layoutKey || !this.childrenInitialized || this.SnapPoints.length !== this.ChildrenTotal) { this.layoutKey = key; this.InitializeChildren(); }
    else if (this.IsLooped !== this.lastLooped) { this.lastLooped = this.IsLooped; this.snapPointsVirtual = undefined; this.ContentOffsetBounds = this.GetContentOffsetBounds(); }
  }

  /** C# InitializeChildren: snap points from the slide size, bounds, and the current index applied instantly. */
  private InitializeChildren(): void {
    const count = this.ChildrenTotal;
    if (this.IsTemplated && !this.ItemTemplate) return;
    this.childrenInitialized = true;
    this.lastLooped = this.IsLooped;
    this.itemsVisibility = new Array(count).fill(false);
    const step = this.Step();
    this.SnapPoints = Array.from({ length: count }, (_, i) => (this.IsVertical ? new SKPoint(0, -i * step) : new SKPoint(-i * step, 0)));
    this.snapPointsVirtual = undefined;
    this.ContentOffsetBounds = this.GetContentOffsetBounds();
    this.CurrentSnap = new SKPoint(-1, -1);
    if (count > 0 && (this.selectedIndex < 0 || this.selectedIndex > this.MaxIndex)) {
      this.lastIndex = this.selectedIndex; this.selectedIndex = 0; this.OnSelectedIndexChanged();
    }
    this.OnChildrenInitialized();
    this.ApplyIndex(true);
  }

  /** C# GetContentOffsetBounds: the snap extent, or unbounded along the axis when looped. */
  private GetContentOffsetBounds(): SKRect {
    if (this.SnapPoints.length === 0) return SKRect.Empty;
    const b = this.BoundsFromSnapPoints();
    if (!this.IsLooped || this.SnapPoints.length < 2) return b;
    const far = 1e9;
    return this.IsVertical ? new SKRect(b.Left, -far, b.Right, far) : new SKRect(-far, b.Top, far, b.Bottom);
  }

  /** C# CalculateChildPosition: slide offset in points, visibility, and "next to screen"; looped draws the edge slides on the other side. */
  private CalculateChildPosition(current: SKPoint, index: number, count: number): ChildPosition {
    const W = this.cellSize.W, H = this.cellSize.H, S = this.SidesOffset, gap = this.Spacing;
    const nextToScreenOffset = 10;
    const size = this.IsVertical ? H : W;
    const snap = this.SnapPoints[index];
    let pos = this.IsVertical ? current.Y + Math.abs(snap.Y) : current.X + Math.abs(snap.X);
    const test = (p: number) => ({ visible: p + S * 2 <= size && p + size >= 0, next: Math.abs(p) - size - S - gap <= nextToScreenOffset });
    let { visible, next } = test(pos);
    if (this.IsLooped && count > 1) {
      const step = this.SnapPoints.length >= 2 ? Math.abs((this.IsVertical ? this.SnapPoints[1].Y : this.SnapPoints[1].X) - (this.IsVertical ? this.SnapPoints[0].Y : this.SnapPoints[0].X)) : size + gap - S * 2;
      if (index === 0 || index === count - 1) {
        const alt = index === 0 ? pos + step * count : pos - step * count; // first after last / last before first
        const t = test(alt);
        if ((t.visible || t.next) && !visible) { pos = alt; visible = t.visible; next = t.next; }
      }
    }
    return { Offset: this.IsVertical ? new SKPoint(0, pos) : new SKPoint(pos, 0), OnScreen: visible, NextToScreen: next };
  }

  private SendVisibility(index: number, state: boolean): void {
    if (this.itemsVisibility[index] === state) return;
    this.itemsVisibility[index] = state;
    if (state) this.ItemAppearing?.(this, index); else this.ItemDisappearing?.(this, index);
  }

  /** C# RenderViewsList: position every slide, draw the visible ones (+ neighbours when PreloadNeighboors), release the rest. */
  protected override Paint(ctx: DrawingContext): void {
    const count = this.ChildrenTotal;
    if (this.SnapPoints.length === 0 || count === 0 || (this.IsTemplated && !this.ItemTemplate)) { this.visibleViews = []; return; }
    const scale = this.RenderingScale;
    const inner = this.InnerRect();
    const keep = new Set<number>();
    const cells: { index: number; visible: boolean; offset: SKPoint }[] = [];
    for (let i = 0; i < count; i++) {
      const p = this.CalculateChildPosition(this.CurrentPosition, i, count);
      if (p.OnScreen || p.NextToScreen) { cells.push({ index: i, visible: p.OnScreen, offset: p.Offset }); keep.add(i); }
      else if (!this.IsTemplated) { // static slides keep a truthful (off-screen) box for accessibility / hit tests
        const v = this.Children[i];
        if (v?.IsVisible) v.Arrange(this.SlideRect(inner, p.Offset, scale), v.WidthRequest, v.HeightRequest, scale);
      }
      this.SendVisibility(i, p.OnScreen);
    }
    if (this.IsTemplated) this.ChildrenFactory.ReleaseExcept(keep);

    const track = (this.IsVertical ? inner.Height : inner.Width) - this.SidesOffset * scale;
    const visible: SkiaControl[] = [];
    for (const cell of cells) {
      if (!cell.visible && !this.PreloadNeighboors) continue;
      const view = this.GetChild(cell.index);
      if (!view || !view.IsVisible) continue;
      if (cell.index === this.selectedIndex && track > 0) this.scrollAmount = ((this.IsVertical ? cell.offset.Y : cell.offset.X) * scale) / track;
      if (this.IsTemplated) view.Measure(inner.Width, inner.Height, scale); // recycled cells carry new content
      view.Arrange(this.SlideRect(inner, this.SlideOffset(cell.offset), scale), view.WidthRequest, view.HeightRequest, scale);
      view.Render(ctx);
      if (cell.visible) visible.push(view);
    }
    this.visibleViews = visible;
  }

  private SlideRect(inner: SKRect, offset: SKPoint, scale: number): SKRect {
    return this.IsVertical
      ? SKRect.Create(inner.Left, inner.Top + offset.Y * scale, inner.Width, inner.Height)
      : SKRect.Create(inner.Left + offset.X * scale, inner.Top, inner.Width, inner.Height);
  }

  protected override GetGestureListeners(): readonly SkiaControl[] { return this.visibleViews; }

  // ---- gestures (port of C# SkiaCarousel.ProcessGestures) ----
  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    const consumedDefault = this.BlockGesturesBelow ? this : null;
    let passed = false;
    const passToChildren = () => { passed = true; return super.ProcessGestures(args, apply); };
    let consumed: SkiaControl | null = null;
    if (!this.IsUserPanning || !this.RespondsToGestures || args.Type === "Tapped") {
      consumed = passToChildren();
      if (consumed === this) consumed = null;
      if (consumed && !(args.Type === "Up" && this.snapIfNoPanOnUp)) return consumed;
    }
    if (!this.RespondsToGestures || this.ChildrenTotal < 2) return consumedDefault;
    const scale = this.RenderingScale;
    const e = args.Event;
    const resetPan = () => {
      this.wrongDirection = false; this.IsUserFocused = true; this.IsUserPanning = false;
      this.snapIfNoPanOnUp = this.IsSnapAnimating || this.InTransition;
      this.StopSnapAnimators(); this.velocityAccumulator.Clear();
      this.FixPosition();
      this.panningOffset = this.CurrentPosition; this.panningStart = this.CurrentPosition;
    };
    switch (args.Type) {
      case "Down":
        this.hadDown = true;
        resetPan();
        consumed = this;
        break;
      case "Panning": {
        if (!this.hadDown || this.wrongDirection) return consumedDefault;
        if (!this.IsUserPanning) {
          const movex = Math.abs(e.Distance.Total.X), movey = Math.abs(e.Distance.Total.Y);
          const along = this.IsVertical ? movey : movex, across = this.IsVertical ? movex : movey;
          if (along < scale * 2 || across > along) { this.wrongDirection = true; return consumedDefault; }
        }
        if (!this.IsUserFocused) resetPan();
        this.IsUserPanning = true; this.snapIfNoPanOnUp = false;
        const x = this.panningOffset.X + e.Distance.Delta.X / scale, y = this.panningOffset.Y + e.Distance.Delta.Y / scale;
        const vx = this.IsVertical ? 0 : e.Distance.Velocity.X / scale, vy = this.IsVertical ? e.Distance.Velocity.Y / scale : 0;
        this.velocityAccumulator.CaptureVelocity(vx, vy, args.ArrivedTimeNanos);
        this.panningOffset = new SKPoint(this.IsVertical ? 0 : x, this.IsVertical ? y : 0);
        this.ApplyPosition(this.ClampOffset(this.panningOffset.X, this.panningOffset.Y, this.Bounces));
        consumed = this;
        break;
      }
      case "Up":
        this.hadDown = false;
        if (this.IsUserPanning) {
          consumed = this;
          const final = this.velocityAccumulator.CalculateFinalVelocity(500);
          this.CurrentSnap = this.CurrentPosition;
          this.ScrollToNearestAnchor(this.CurrentSnap, new SKPoint(final.X, final.Y));
          this.IsUserPanning = false; this.IsUserFocused = false; this.snapIfNoPanOnUp = false;
        } else if (this.snapIfNoPanOnUp) {
          // Down interrupted a snap but no carousel pan followed: still settle on the nearest slide, leave Up to the children
          this.CurrentSnap = this.CurrentPosition;
          this.ScrollToNearestAnchor(this.CurrentSnap, SKPoint.Empty);
          this.IsUserFocused = false; this.IsUserPanning = false; this.snapIfNoPanOnUp = false;
        }
        break;
    }
    if (consumed || this.IsUserPanning) return consumed ?? (args.Type !== "Up" ? this : consumedDefault);
    if (!passed) return passToChildren();
    return consumedDefault;
  }

  protected override DefaultAccessibilityLabel(): string | undefined { return `${this.selectedIndex + 1} / ${this.ChildrenTotal}`; }
}
