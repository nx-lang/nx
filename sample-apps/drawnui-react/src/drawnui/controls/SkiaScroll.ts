import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type RelativePositionType, SKRect, ScaledSize, type ScrollOrientation } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import {
  RubberBandUtils, ScrollFlingAnimator, Spring, SpringWithVelocityAnimator, VelocityAccumulator,
} from "../core/ScrollAnimators";

/** DrawnUi ScaledPoint: same point in points and pixels. */
export interface ScaledPoint { Units: SKPoint; Pixels: SKPoint }

/**
 * Mirrors DrawnUi SkiaScroll (plain content, no header/footer/refresh/virtualization yet).
 * Offsets are in POINTS and <= 0 while inside bounds (content moves up/left as you scroll).
 * Physics ported 1:1: deceleration fling (rate = 1 - FrictionScrolled/100) cut at the content edge,
 * rubber-band overscroll while dragging, spring bounce back on release, wheel = WheelLineSize per notch.
 */
export class SkiaScroll extends SkiaControl {
  static WheelLineSize = 150;
  static ThesholdSwipeOnUp = 20;
  static ScrollVelocityThreshold = 5;

  Orientation: ScrollOrientation = "Vertical";
  Bounces = true;
  RubberDamping = 0.55;
  RubberEffect = 0.55;
  /** 0.1..1, lower = longer fling. */
  FrictionScrolled = 0.3;
  ChangeVelocityScrolled = 1.33;
  ChangeDistancePanned = 1;
  MaxVelocity = 3000;
  MaxBounceVelocity = 500;
  AutoScrollingSpeedMs = 600;
  ScrollingSpeedMs = 400;
  IgnoreWrongDirection = false;
  RespondsToGestures = true;

  /** Offset changed (drag, fling, bounce, ScrollTo). */
  Scrolled?: (sender: SkiaScroll, e: ScaledPoint) => void;

  // ---- LoadMore (DrawnUi SkiaScroll.LoadMoreCommand / LoadMoreTopCommand, callbacks instead of ICommand) ----
  /** Called when scrolling within LoadMoreOffset points of the end (bottom / right), or when the content underfills the viewport. */
  LoadMoreCommand?: (sender: SkiaScroll) => void;
  /** Called when scrolling within LoadMoreTopOffset points of the start (top / left). */
  LoadMoreTopCommand?: (sender: SkiaScroll) => void;
  LoadMoreOffset = 0;
  LoadMoreTopOffset = 0;
  private loadMoreBottomAt = NaN;   // content extent (points) when the bottom command last fired; NaN = armed
  private loadMoreBottomTime = 0;
  /** Top fires only after the user has actually left the top edge and comes back (a fresh list at offset 0 is not a "reached the top"). */
  private loadMoreTopArmed = false;

  /** C# CheckNeedToLoadMore: fires once per content extent near an edge; re-arms when the content grows or the user moves away (>offset+100pt, >2 s). */
  private CheckLoadMore(): void {
    const vertical = this.Orientation !== "Horizontal";
    const offset = vertical ? this.offsetY : this.offsetX;
    const min = vertical ? this.ContentOffsetBounds.Top : this.ContentOffsetBounds.Left; // most negative offset
    const extent = vertical ? this.ContentSize.Units.Height : this.ContentSize.Units.Width;
    const now = performance.now();
    if (this.LoadMoreCommand) {
      const underfills = min >= 0; // nothing to scroll: keep paging until the viewport is filled
      if (!isNaN(this.loadMoreBottomAt) && this.loadMoreBottomAt !== extent) this.loadMoreBottomAt = NaN;
      else if (!isNaN(this.loadMoreBottomAt) && !underfills && offset - min > this.LoadMoreOffset + 100 && now - this.loadMoreBottomTime > 2000) this.loadMoreBottomAt = NaN;
      if (isNaN(this.loadMoreBottomAt) && (underfills || offset <= min + this.LoadMoreOffset)) {
        this.loadMoreBottomAt = extent; this.loadMoreBottomTime = now;
        this.LoadMoreCommand(this);
      }
    }
    if (this.LoadMoreTopCommand) {
      if (-offset > this.LoadMoreTopOffset + 100) this.loadMoreTopArmed = true;
      else if (this.loadMoreTopArmed && min < 0 && offset >= -this.LoadMoreTopOffset) {
        this.loadMoreTopArmed = false;
        this.LoadMoreTopCommand(this);
      }
    }
  }

  IsUserPanning = false;
  IsUserFocused = false;
  IsScrolling = false;
  ContentSize: ScaledSize = ScaledSize.Default;
  /** Points: Left/Top = most negative allowed offset, Right/Bottom = 0. */
  ContentOffsetBounds: SKRect = SKRect.Empty;
  OverscrollDistance = SKPoint.Empty;
  get OverScrolled(): boolean { return this.OverscrollDistance.X !== 0 || this.OverscrollDistance.Y !== 0; }

  private content?: SkiaControl;
  get Content(): SkiaControl | undefined { return this.content; }
  set Content(value: SkiaControl | undefined) {
    if (this.content === value) return;
    if (this.content) { this.content.Parent = undefined; if (this.content instanceof SkiaLayout) this.content.ItemsInsertedAtStart = undefined; }
    this.content = value;
    if (value) {
      value.Parent = this;
      // items prepended (chat history): move the offset by the inserted extent so the visible rows stay put
      if (value instanceof SkiaLayout) value.ItemsInsertedAtStart = (_, px) => {
        const d = px / this.RenderingScale;
        if (this.Orientation === "Horizontal") this.offsetX -= d; else this.offsetY -= d;
      };
    }
    this.InvalidateMeasure();
  }

  // ---- offsets (points) ----
  private offsetX = 0;
  private offsetY = 0;
  get ViewportOffsetX(): number { return this.offsetX; }
  set ViewportOffsetX(value: number) { if (this.offsetX !== value) { this.offsetX = value; this.OnScrolled(); } }
  get ViewportOffsetY(): number { return this.offsetY; }
  set ViewportOffsetY(value: number) { if (this.offsetY !== value) { this.offsetY = value; this.OnScrolled(); } }

  private readonly animatorFlingX = new ScrollFlingAnimator(this);
  private readonly animatorFlingY = new ScrollFlingAnimator(this);
  private readonly bounceX = new SpringWithVelocityAnimator(this);
  private readonly bounceY = new SpringWithVelocityAnimator(this);
  private readonly velocity = new VelocityAccumulator();
  private panningCurrentOffsetPts = SKPoint.Empty;
  private panningLastDelta = SKPoint.Empty;
  private hadDown = false;
  private childWasPanning = false;
  private velocityX = 0;
  private velocityY = 0;
  private static readonly MinVelocity = 1.5;
  /** Set when a fling was cut to stop at the edge: the edge offset it lands on (DrawnUi _axis / _changeSpeed). */
  private flingEdgeX: number | null = null;
  private flingEdgeY: number | null = null;

  constructor() {
    super();
    this.animatorFlingX.OnUpdated = (v) => { this.ViewportOffsetX = this.ClampOffset(v, 0, this.ContentOffsetBounds).X; };
    this.animatorFlingY.OnUpdated = (v) => { this.ViewportOffsetY = this.ClampOffset(0, v, this.ContentOffsetBounds).Y; };
    this.bounceX.OnUpdated = (v) => { this.ViewportOffsetX = v; };
    this.bounceY.OnUpdated = (v) => { this.ViewportOffsetY = v; };
    for (const a of [this.animatorFlingX, this.animatorFlingY, this.bounceX, this.bounceY]) {
      a.OnStart = () => { this.IsScrolling = true; };
      a.OnStop = () => { this.IsScrolling = this.animatorFlingX.IsRunning || this.animatorFlingY.IsRunning || this.bounceX.IsRunning || this.bounceY.IsRunning; };
    }
    // DrawnUi OnScrollerStopped: a fling that was cut at the edge hands its remaining velocity to the bounce.
    const flingStop = this.animatorFlingY.OnStop!;
    this.animatorFlingY.OnStop = () => { flingStop(); if (this.animatorFlingY.WasStarted) this.BounceIfNeeded(this.animatorFlingY, false); };
    const flingStopX = this.animatorFlingX.OnStop!;
    this.animatorFlingX.OnStop = () => { flingStopX(); if (this.animatorFlingX.WasStarted) this.BounceIfNeeded(this.animatorFlingX, true); };
  }

  /** DrawnUi BounceIfNeeded: after an edge-cut fling finishes, bounce with the velocity it still had. */
  private BounceIfNeeded(animator: ScrollFlingAnimator, horizontal: boolean): void {
    const edge = horizontal ? this.flingEdgeX : this.flingEdgeY;
    if (!this.Bounces || edge === null || !animator.SelfFinished || !animator.Parameters) return;
    if (horizontal) this.flingEdgeX = null; else this.flingEdgeY = null;
    const remaining = animator.Parameters.VelocityAt(animator.Speed);
    const velocity = Math.sign(remaining) * Math.min(Math.abs(remaining), this.MaxBounceVelocity);
    if (Math.abs(velocity) <= SkiaScroll.ThesholdSwipeOnUp * this.RenderingScale) return;
    if (horizontal) this.Bounce(this.bounceX, this.animatorFlingX, this.offsetX, edge, velocity);
    else this.Bounce(this.bounceY, this.animatorFlingY, this.offsetY, edge, velocity);
  }

  // ---- tree (single Content) ----
  override AddSubView(control: SkiaControl): void { this.Content = control; }
  override InsertSubView(_index: number, control: SkiaControl): void { this.Content = control; }
  override RemoveSubView(control: SkiaControl): void { if (this.content === control) this.Content = undefined; }
  protected override GetGestureListeners(): readonly SkiaControl[] { return this.content ? [this.content] : []; }

  // ---- measure / arrange ----

  /** Content gets an infinite constraint along the scroll axis; the scroll itself takes the box it is given. */
  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    const c = this.content;
    if (c && c.IsVisible) {
      const w = this.Orientation === "Horizontal" || this.Orientation === "Both" ? Infinity : widthConstraint;
      const h = this.Orientation === "Vertical" || this.Orientation === "Both" ? Infinity : heightConstraint;
      this.ContentSize = c.Measure(w, h, scale);
    } else this.ContentSize = ScaledSize.Default;
    const rw = isFinite(widthConstraint) ? widthConstraint : this.ContentSize.Pixels.Width;
    const rh = isFinite(heightConstraint) ? heightConstraint : this.ContentSize.Pixels.Height;
    return ScaledSize.FromPixels(rw, rh, scale);
  }

  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const viewportW = this.DrawingRect.Width / scale, viewportH = this.DrawingRect.Height / scale;
    const width = Math.max(0, this.ContentSize.Units.Width - viewportW);
    const height = Math.max(0, this.ContentSize.Units.Height - viewportH);
    this.ContentOffsetBounds = new SKRect(-width, -height, 0, 0);
    this.ArrangeContent();
    this.CheckLoadMore();
  }

  /** Places Content at the current offset; called on layout and every frame while the offset moves. */
  private ArrangeContent(): void {
    const c = this.content;
    if (!c) return;
    const scale = this.RenderingScale;
    const r = this.DrawingRect;
    const alongX = this.Orientation === "Horizontal" || this.Orientation === "Both";
    const alongY = this.Orientation === "Vertical" || this.Orientation === "Both";
    const w = alongX ? Math.max(this.ContentSize.Pixels.Width, r.Width) : r.Width;
    const h = alongY ? Math.max(this.ContentSize.Pixels.Height, r.Height) : r.Height;
    // Offsets are snapped to whole device pixels: a fractional offset would rasterize every glyph at a
    // different sub-pixel phase each frame (text shimmer while scrolling). DrawnUi gets the same result from
    // nearest-sampled cached cells; here the snap applies to the whole content, cached or not.
    const x = r.Left + (alongX ? Math.round(this.offsetX * scale) : 0);
    const y = r.Top + (alongY ? Math.round(this.offsetY * scale) : 0);
    c.Arrange(SKRect.Create(x, y, w, h), c.WidthRequest, c.HeightRequest, scale);
    this.OverscrollDistance = this.CalculateOverscrollDistance(this.offsetX, this.offsetY);
  }

  protected override Paint(ctx: DrawingContext): void {
    const c = this.content;
    if (!c) return;
    this.ArrangeContent();
    const canvas = ctx.Context.Canvas;
    const d = ctx.Destination;
    const saved = canvas.save();
    canvas.clipRect(Super.CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), Super.CK.ClipOp.Intersect, true);
    c.Render(ctx);
    canvas.restoreToCount(saved);
  }

  private OnScrolled(): void {
    this.CheckLoadMore();
    this.Repaint();
    this.Scrolled?.(this, { Units: new SKPoint(this.offsetX, this.offsetY), Pixels: new SKPoint(this.offsetX * this.RenderingScale, this.offsetY * this.RenderingScale) });
  }

  // ---- clamping ----

  ClampOffset(x: number, y: number, bounds: SKRect, strict = false): { X: number; Y: number } {
    if (!this.Bounces || strict) {
      return { X: Math.max(bounds.Left, Math.min(bounds.Right, x)), Y: Math.max(bounds.Top, Math.min(bounds.Bottom, y)) };
    }
    const scale = this.RenderingScale;
    return RubberBandUtils.ClampOnTrack(x, y, bounds, this.RubberEffect, this.DrawingRect.Width / scale, this.DrawingRect.Height / scale);
  }

  private CalculateOverscrollDistance(x: number, y: number): SKPoint {
    const b = this.ContentOffsetBounds;
    const dx = x < b.Left ? x - b.Left : x > b.Right ? x - b.Right : 0;
    const dy = y < b.Top ? y - b.Top : y > b.Bottom ? y - b.Bottom : 0;
    return new SKPoint(dx, dy);
  }

  // ---- programmatic scrolling ----

  /** Scroll to an offset in points; maxSpeedSecs > 0 animates along the deceleration curve. */
  /**
   * Scrolls every SkiaScroll ancestor of `control` so its DrawingRect lies inside the viewport (React extension,
   * used by the accessibility overlay when keyboard focus lands on an off-screen node). Offsets are points.
   */
  static EnsureVisible(control: SkiaControl, maxTimeSecs = 0.25, paddingPts = 8): void {
    let p = control.Parent;
    while (p) {
      if (p instanceof SkiaScroll) {
        const scale = p.RenderingScale;
        const r = control.DrawingRect, v = p.DrawingRect;
        let dx = 0, dy = 0;
        if (r.Top < v.Top) dy = (v.Top - r.Top) / scale + paddingPts;
        else if (r.Bottom > v.Bottom) dy = -((r.Bottom - v.Bottom) / scale + paddingPts);
        if (r.Left < v.Left) dx = (v.Left - r.Left) / scale + paddingPts;
        else if (r.Right > v.Right) dx = -((r.Right - v.Right) / scale + paddingPts);
        if (dx !== 0 || dy !== 0) p.ScrollTo(p.offsetX + dx, p.offsetY + dy, maxTimeSecs);
      }
      p = p.Parent;
    }
  }

  ScrollTo(x: number, y: number, maxSpeedSecs: number, clamp = true): void {
    this.StopAnimators(); // also forgets any pending edge bounce: a programmatic scroll never bounces
    let tx = x, ty = y;
    if (clamp) { const c = this.ClampOffset(x, y, this.ContentOffsetBounds, true); tx = c.X; ty = c.Y; }
    const rate = 1 - this.DecelerationRatio;
    if (maxSpeedSecs > 0) {
      if (this.Orientation !== "Vertical" && tx !== this.offsetX) { this.animatorFlingX.InitializeWithDestination(this.offsetX, tx, maxSpeedSecs, rate); this.animatorFlingX.Start(); }
      if (this.Orientation !== "Horizontal" && ty !== this.offsetY) { this.animatorFlingY.InitializeWithDestination(this.offsetY, ty, maxSpeedSecs, rate); this.animatorFlingY.Start(); }
    } else {
      this.ViewportOffsetX = tx;
      this.ViewportOffsetY = ty;
    }
  }

  ScrollToTop(maxTimeSecs: number): void { this.ScrollTo(this.Orientation === "Horizontal" ? 0 : this.offsetX, this.Orientation === "Horizontal" ? this.offsetY : 0, maxTimeSecs); }

  ScrollToBottom(maxTimeSecs: number): void {
    if (this.Orientation === "Horizontal") this.ScrollTo(this.ContentOffsetBounds.Left, this.offsetY, maxTimeSecs);
    else this.ScrollTo(this.offsetX, this.ContentOffsetBounds.Top, maxTimeSecs);
  }

  StopScrolling(): void { this.StopAnimators(); this.IsUserPanning = false; }

  /**
   * Scrolls so that item `index` is at the viewport start (or end). Like DrawnUi, Content must BE the
   * templated layout (the recycled list is the scroll's only child; a header goes above the scroll or in Header).
   */
  ScrollToIndex(index: number, animate: boolean, option: RelativePositionType = "Start"): void {
    const layout = this.content instanceof SkiaLayout && this.content.IsTemplated ? this.content : undefined;
    const items = layout?.ItemsSource;
    if (!layout || !items || items.length === 0 || !this.content) return;
    const i = Math.max(0, Math.min(items.length - 1, index));
    const scale = this.RenderingScale;
    const layoutTopPts = (layout.DrawingRect.Top - this.content.DrawingRect.Top) / scale;
    let target = layoutTopPts + layout.GetItemOffsetPixels(i) / scale;
    if (option === "End") target -= this.DrawingRect.Height / scale - layout.GetItemOffsetPixels(i + 1) / scale + layout.GetItemOffsetPixels(i) / scale;
    else if (option === "Center") target -= this.DrawingRect.Height / scale / 2;
    this.ScrollTo(this.offsetX, -target, animate ? this.ScrollingSpeedMs / 1000 : 0, true);
  }

  private StopAnimators(): void {
    this.flingEdgeX = null; this.flingEdgeY = null;
    this.animatorFlingX.Stop(); this.animatorFlingY.Stop(); this.bounceX.Stop(); this.bounceY.Stop();
  }

  /** DrawnUi DecelerationRatio: FrictionScrolled / 100, floored at 0.1 friction. */
  private get DecelerationRatio(): number { return Math.max(0.1, this.FrictionScrolled) / 100; }

  // ---- gestures ----

  private ResetPan(): void {
    this.IsUserFocused = true;
    this.IsUserPanning = false;
    this.childWasPanning = false;
    this.velocity.Clear();
    this.panningLastDelta = SKPoint.Empty;
    this.panningCurrentOffsetPts = new SKPoint(this.offsetX, this.offsetY);
  }

  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    const consumedDefault = this.BlockGesturesBelow ? this : null;
    const scale = this.RenderingScale;
    const e = args.Event;

    if (args.Type === "Down") {
      this.hadDown = true;
      if (this.RespondsToGestures) { this.StopAnimators(); this.ResetPan(); }
      return super.ProcessGestures(args, apply) ?? consumedDefault; // children see Down (buttons press)
    }

    if (args.Type === "Wheel") {
      if (!this.RespondsToGestures) return super.ProcessGestures(args, apply);
      this.ApplyWheelScroll(e.Wheel.Delta);
      return this;
    }

    this.velocityY = e.Distance.Velocity.Y / scale;
    this.velocityX = e.Distance.Velocity.X / scale;
    const wrongDirection =
      (this.Orientation === "Vertical" && Math.abs(e.Distance.Total.X) > Math.abs(e.Distance.Total.Y) && Math.abs(e.Distance.Total.X) > SkiaScroll.ScrollVelocityThreshold * scale) ||
      (this.Orientation === "Horizontal" && Math.abs(e.Distance.Total.Y) > Math.abs(e.Distance.Total.X) && Math.abs(e.Distance.Total.Y) > SkiaScroll.ScrollVelocityThreshold * scale);

    if (args.Type === "Panning" && this.RespondsToGestures && this.hadDown) {
      if (!this.IsUserPanning) {
        // A child may own the pan (slider, nested horizontal scroll); ask once before taking over.
        const childConsumed = super.ProcessGestures(args, apply);
        if (childConsumed && childConsumed !== this) { this.childWasPanning = true; return childConsumed; }
        if (this.childWasPanning) return consumedDefault;
        if (this.IgnoreWrongDirection && wrongDirection) { this.IsUserFocused = false; return consumedDefault; }
        const v = this.Orientation === "Vertical" ? this.velocityY : this.Orientation === "Horizontal" ? this.velocityX : Math.max(Math.abs(this.velocityX), Math.abs(this.velocityY));
        if (Math.abs(v) <= SkiaScroll.ScrollVelocityThreshold) return consumedDefault;
      }
      if (!this.IsUserFocused) this.ResetPan();
      this.IsUserPanning = true;
      this.velocity.CaptureVelocity(this.velocityX, this.velocityY, args.ArrivedTimeNanos);

      const movedX = (e.Distance.Delta.X / scale) * this.ChangeDistancePanned;
      const movedY = (e.Distance.Delta.Y / scale) * this.ChangeDistancePanned;
      const ix = this.panningLastDelta.X + (movedX - this.panningLastDelta.X) * 0.85;
      const iy = this.panningLastDelta.Y + (movedY - this.panningLastDelta.Y) * 0.85;
      this.panningLastDelta = new SKPoint(ix, iy);
      this.panningCurrentOffsetPts = new SKPoint(this.panningCurrentOffsetPts.X + ix, this.panningCurrentOffsetPts.Y + iy);
      const clamped = this.ClampOffset(this.panningCurrentOffsetPts.X, this.panningCurrentOffsetPts.Y, this.ContentOffsetBounds);
      if (this.Orientation !== "Horizontal") this.ViewportOffsetY = clamped.Y;
      if (this.Orientation !== "Vertical") this.ViewportOffsetX = clamped.X;
      return this;
    }

    if (args.Type === "Up") {
      const childConsumed = super.ProcessGestures(args, apply); // children release (buttons)
      this.hadDown = false;
      if (!this.RespondsToGestures || this.childWasPanning) { this.IsUserPanning = false; return childConsumed ?? consumedDefault; }
      const wasPanning = this.IsUserPanning;
      this.IsUserPanning = false;
      this.IsUserFocused = false;
      if (!wasPanning && !this.OverScrolled) return childConsumed ?? consumedDefault;

      const finalVelocity = this.velocity.CalculateFinalVelocity(this.MaxVelocity);
      const swipeThreshold = SkiaScroll.ThesholdSwipeOnUp * scale;
      let vx = finalVelocity.X * this.ChangeVelocityScrolled;
      let vy = finalVelocity.Y * this.ChangeVelocityScrolled;
      let fling = false;

      if (this.OverScrolled) {
        const rest = this.ClampOffset(this.offsetX, this.offsetY, this.ContentOffsetBounds, true);
        const bvx = Math.sign(vx) * Math.min(Math.abs(vx), this.MaxBounceVelocity);
        const bvy = Math.sign(vy) * Math.min(Math.abs(vy), this.MaxBounceVelocity);
        if (this.OverscrollDistance.Y !== 0) { this.Bounce(this.bounceY, this.animatorFlingY, this.offsetY, rest.Y, bvy); fling = true; }
        if (this.OverscrollDistance.X !== 0) { this.Bounce(this.bounceX, this.animatorFlingX, this.offsetX, rest.X, bvx); fling = true; }
        return this;
      }

      const swipe = Math.abs(vx) > swipeThreshold || Math.abs(vy) > swipeThreshold;
      if (swipe) {
        if (this.Orientation !== "Vertical" && Math.abs(vx) > SkiaScroll.MinVelocity) { this.bounceX.Stop(); fling = this.StartToFlingFrom(this.animatorFlingX, this.offsetX, vx, true) || fling; }
        if (this.Orientation !== "Horizontal" && Math.abs(vy) > SkiaScroll.MinVelocity) { this.bounceY.Stop(); fling = this.StartToFlingFrom(this.animatorFlingY, this.offsetY, vy, false) || fling; }
      }
      this.Repaint();
      return fling || wasPanning ? this : childConsumed ?? consumedDefault;
    }

    if (args.Type === "Tapped" && this.IsUserPanning) return this; // a drag never ends in a tap
    return super.ProcessGestures(args, apply) ?? consumedDefault;
  }

  private Bounce(animator: SpringWithVelocityAnimator, fling: ScrollFlingAnimator, offsetFrom: number, offsetTo: number, velocity: number): void {
    const displacement = offsetFrom - offsetTo;
    if (displacement === 0 && velocity === 0) return;
    if (fling.IsRunning) fling.Stop();
    const spring = new Spring(1 * (1 + this.RubberDamping), 200, 0.5 * (1 + this.RubberDamping));
    animator.Initialize(offsetTo, displacement, velocity, spring);
    animator.Start();
  }

  /** DrawnUi StartToFlingFrom + PrepareToFlingAfterInitialized: cut the curve so it stops exactly at the edge. */
  private StartToFlingFrom(animator: ScrollFlingAnimator, from: number, velocity: number, horizontal: boolean): boolean {
    animator.InitializeWithVelocity(from, velocity, 1 - this.DecelerationRatio);
    const p = animator.Parameters!;
    const b = this.ContentOffsetBounds;
    const min = horizontal ? b.Left : b.Top, max = horizontal ? b.Right : b.Bottom;
    const destination = p.Destination;
    let edge: number | null = null;
    if (destination < min || destination > max) {
      edge = Math.max(min, Math.min(max, destination));
      animator.Speed = p.DurationToValue(edge);
    }
    if (horizontal) this.flingEdgeX = edge; else this.flingEdgeY = edge;
    if (animator.Speed <= 0) return false;
    animator.Start();
    return true;
  }

  /**
   * One wheel notch = WheelLineSize points, hard-clamped, animated over AutoScrollingSpeedMs.
   * Notches arriving while the previous one is still animating add onto its target, so a fast wheel
   * spin travels N steps instead of restarting from the barely-moved current offset.
   */
  private ApplyWheelScroll(delta: number): void {
    const step = SkiaScroll.WheelLineSize * -Math.sign(delta); // wheel down = content up = more negative offset
    const horizontal = this.Orientation === "Horizontal";
    const running = horizontal ? this.animatorFlingX : this.animatorFlingY;
    const base = running.IsRunning && running.Parameters ? running.Parameters.Destination : horizontal ? this.offsetX : this.offsetY;
    let x = this.offsetX, y = this.offsetY;
    if (horizontal) x = base + step; else y = base + step;
    this.ScrollTo(x, y, this.AutoScrollingSpeedMs / 1000, true);
  }
}
