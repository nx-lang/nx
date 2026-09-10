import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { Super } from "../core/Super";
import { type Color, type RelativePositionType, SKRect, ScaledSize, type ScrollOrientation, type SnapToChildrenType } from "../core/Types";
import { SkiaLayout } from "./SkiaLayout";
import { type IScrollBar, type ScrollBarVisibility, SkiaScrollBar } from "./SkiaScrollBar";
import type { IRefreshIndicator } from "./RefreshIndicator";
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

  // ---- header / footer (C# SkiaScroll.Header / Footer) ----
  private header?: SkiaControl;
  private footer?: SkiaControl;
  /** A view before the content along the scroll axis; `Tag="Header"` on a JSX child sets it. */
  get Header(): SkiaControl | undefined { return this.header; }
  set Header(v: SkiaControl | undefined) { if (this.header === v) return; if (this.header) this.header.Parent = undefined; this.header = v; if (v) { v.Parent = this; v.ZIndex = 1; } this.InvalidateMeasure(); }
  /** A view after the content along the scroll axis; `Tag="Footer"` on a JSX child sets it. */
  get Footer(): SkiaControl | undefined { return this.footer; }
  set Footer(v: SkiaControl | undefined) { if (this.footer === v) return; if (this.footer) this.footer.Parent = undefined; this.footer = v; if (v) v.Parent = this; this.InvalidateMeasure(); }
  /** The header stays at the viewport start while the content scrolls under it (drawn over the content unless HeaderBehind). */
  HeaderSticky = false;
  /** The header is drawn behind the content (the content covers it as it scrolls, parallax covers). */
  HeaderBehind = false;
  /** 1 = the header scrolls with the content, 0.5 = at half speed (parallax), 0 = stays. */
  HeaderParallaxRatio = 1;
  /** Apply the parallax while overscrolling at the start too (false: the header follows the content there). */
  ParallaxOverscrollEnabled = true;
  /** Extra points between a HeaderBehind / HeaderSticky header and the content (C# ContentOffset). */
  ContentOffset = 0;
  private headerSize: ScaledSize = ScaledSize.Default;
  private footerSize: ScaledSize = ScaledSize.Default;

  // ---- scroll bars (C# ScrollBar / ScrollBarHorizontal / ScrollBarsVisibility) ----
  private scrollBar?: IScrollBar & SkiaControl;
  private scrollBarHorizontal?: IScrollBar & SkiaControl;
  private scrollBarsVisibility: ScrollBarVisibility = "None";
  /** Creates default SkiaScrollBar overlays for the given axes (matched against Orientation). */
  get ScrollBarsVisibility(): ScrollBarVisibility { return this.scrollBarsVisibility; }
  set ScrollBarsVisibility(v: ScrollBarVisibility) { if (this.scrollBarsVisibility === v) return; this.scrollBarsVisibility = v; this.ApplyScrollBarsVisibility(); }
  /** Custom vertical bar (IScrollBar control); `Tag="ScrollBar"` on a JSX child sets it. */
  get ScrollBar(): (IScrollBar & SkiaControl) | undefined { return this.scrollBar; }
  set ScrollBar(v: (IScrollBar & SkiaControl) | undefined) { if (this.scrollBar === v) return; if (this.scrollBar) { this.scrollBar.Parent = undefined; this.scrollBar.Dispose(); } this.scrollBar = v; if (v) { v.Parent = this; this.ApplyScrollBarColors(v); } this.scrollBarLast = undefined; this.Repaint(); }
  get ScrollBarHorizontal(): (IScrollBar & SkiaControl) | undefined { return this.scrollBarHorizontal; }
  set ScrollBarHorizontal(v: (IScrollBar & SkiaControl) | undefined) { if (this.scrollBarHorizontal === v) return; if (this.scrollBarHorizontal) { this.scrollBarHorizontal.Parent = undefined; this.scrollBarHorizontal.Dispose(); } this.scrollBarHorizontal = v; if (v) { v.Parent = this; this.ApplyScrollBarColors(v); } this.scrollBarHLast = undefined; this.Repaint(); }
  private scrollBarThumbColor?: Color;
  private scrollBarTrackColor?: Color;
  /** Colors applied to the default SkiaScrollBar (C# ScrollBarThumbColor / ScrollBarTrackColor). */
  get ScrollBarThumbColor(): Color | undefined { return this.scrollBarThumbColor; }
  set ScrollBarThumbColor(v: Color | undefined) { this.scrollBarThumbColor = v; if (this.scrollBar) this.ApplyScrollBarColors(this.scrollBar); if (this.scrollBarHorizontal) this.ApplyScrollBarColors(this.scrollBarHorizontal); }
  get ScrollBarTrackColor(): Color | undefined { return this.scrollBarTrackColor; }
  set ScrollBarTrackColor(v: Color | undefined) { this.scrollBarTrackColor = v; if (this.scrollBar) this.ApplyScrollBarColors(this.scrollBar); if (this.scrollBarHorizontal) this.ApplyScrollBarColors(this.scrollBarHorizontal); }
  private scrollBarLast?: string;
  private scrollBarHLast?: string;
  private ApplyScrollBarColors(bar: SkiaControl): void {
    if (!(bar instanceof SkiaScrollBar)) return;
    if (this.scrollBarThumbColor) bar.ThumbColor = this.scrollBarThumbColor;
    if (this.scrollBarTrackColor) bar.TrackColor = this.scrollBarTrackColor;
  }
  /** C# ApplyScrollBarsVisibility: a default bar per wanted axis, none otherwise. */
  protected ApplyScrollBarsVisibility(): void {
    const wantV = this.scrollBarsVisibility === "Vertical" || this.scrollBarsVisibility === "Both";
    const wantH = this.scrollBarsVisibility === "Horizontal" || this.scrollBarsVisibility === "Both";
    if (wantV && !this.scrollBar) this.ScrollBar = new SkiaScrollBar();
    else if (!wantV && this.scrollBar instanceof SkiaScrollBar) this.ScrollBar = undefined;
    if (wantH && !this.scrollBarHorizontal) this.ScrollBarHorizontal = new SkiaScrollBar();
    else if (!wantH && this.scrollBarHorizontal instanceof SkiaScrollBar) this.ScrollBarHorizontal = undefined;
  }
  /** C# UpdateScrollBarIndicator: pushes progress / thumb ratio / overscroll / scrolling state when any changed. */
  protected UpdateScrollBarIndicator(): void {
    const isScrolling = this.IsScrolling || this.IsUserPanning;
    const scale = this.RenderingScale;
    const viewportH = this.DrawingRect.Height / scale, viewportW = this.DrawingRect.Width / scale;
    const extentH = this.ContentSize.Units.Height + this.HeaderExtentPts + this.FooterExtentPts, extentW = this.ContentSize.Units.Width + this.HeaderExtentPts + this.FooterExtentPts;
    if (this.scrollBar && this.Orientation !== "Horizontal") {
      const min = this.ContentOffsetBounds.Top;
      const progress = min !== 0 ? this.offsetY / min : 0;
      const ratio = extentH > 0 ? viewportH / extentH : 1;
      const key = `${progress.toFixed(4)}|${ratio.toFixed(4)}|${this.OverscrollDistance.Y.toFixed(1)}|${isScrolling}`;
      if (key !== this.scrollBarLast) { this.scrollBarLast = key; this.scrollBar.SetScrollProgress("Vertical", progress, ratio, this.OverscrollDistance.Y, isScrolling); }
    }
    if (this.scrollBarHorizontal && this.Orientation !== "Vertical") {
      const min = this.ContentOffsetBounds.Left;
      const progress = min !== 0 ? this.offsetX / min : 0;
      const ratio = extentW > 0 ? viewportW / extentW : 1;
      const key = `${progress.toFixed(4)}|${ratio.toFixed(4)}|${this.OverscrollDistance.X.toFixed(1)}|${isScrolling}`;
      if (key !== this.scrollBarHLast) { this.scrollBarHLast = key; this.scrollBarHorizontal.SetScrollProgress("Horizontal", progress, ratio, this.OverscrollDistance.X, isScrolling); }
    }
  }

  // ---- pull to refresh (C# RefreshEnabled / RefreshIndicator / RefreshCommand) ----
  RefreshEnabled = false;
  /** Called when the pull passed RefreshDistanceLimit (C# RefreshCommand); set IsRefreshing = false when done. */
  RefreshCommand?: (sender: SkiaScroll) => void;
  /** Pull distance (points) that triggers the refresh. */
  RefreshDistanceLimit = 150;
  /** Distance (points) where the indicator stops moving and stays while refreshing. */
  RefreshShowDistance = 50;
  private refreshIndicator?: IRefreshIndicator & SkiaControl;
  /** The pull-to-refresh view (IRefreshIndicator control); `Tag="RefreshIndicator"` on a JSX child sets it. */
  get RefreshIndicator(): (IRefreshIndicator & SkiaControl) | undefined { return this.refreshIndicator; }
  set RefreshIndicator(v: (IRefreshIndicator & SkiaControl) | undefined) { if (this.refreshIndicator === v) return; if (this.refreshIndicator) this.refreshIndicator.Parent = undefined; this.refreshIndicator = v; if (v) v.Parent = this; this.InvalidateMeasure(); }
  private isRefreshing = false;
  private wasRefreshing = false;
  private scrollLocked = false;
  get IsRefreshing(): boolean { return this.isRefreshing; }
  set IsRefreshing(v: boolean) { if (this.isRefreshing === v) return; this.SetIsRefreshing(v, false); }
  private get UsingRefreshDistanceLimit(): number { return Math.max(this.RefreshDistanceLimit, this.RefreshShowDistance); }
  /** C# SetIsRefreshing: true locks the scroll at RefreshShowDistance and runs RefreshCommand; false hides the indicator. */
  SetIsRefreshing(state: boolean, initial: boolean): void {
    this.isRefreshing = state;
    if (state) {
      this.wasRefreshing = true; this.scrollLocked = true;
      this.ShowRefreshIndicatorForced();
      this.RefreshCommand?.(this);
    } else {
      this.scrollLocked = false;
      if (initial || (this.offsetX === 0 && this.offsetY === 0)) this.HideRefreshIndicator();
      else { this.wasRefreshing = false; this.ScrollTo(this.Orientation === "Horizontal" ? 0 : this.offsetX, this.Orientation === "Horizontal" ? this.offsetY : 0, this.AutoScrollingSpeedMs / 1000, false); }
    }
    this.Repaint();
  }
  protected HideRefreshIndicator(): void {
    this.refreshIndicator?.SetDragRatio(0, 0, this.RefreshShowDistance, this.RefreshDistanceLimit);
    this.scrollLocked = false; this.wasRefreshing = false;
  }
  /** C# ShowRefreshIndicatorForced: parks the offset at RefreshShowDistance and shows the indicator fully. */
  protected ShowRefreshIndicatorForced(): void {
    const ind = this.refreshIndicator;
    if (!ind) return;
    ind.IsVisible = true;
    if (this.Orientation === "Horizontal") { if (this.offsetX < this.RefreshShowDistance) this.ViewportOffsetX = this.RefreshShowDistance; ind.SetDragRatio(1, this.offsetX, this.RefreshShowDistance, this.RefreshDistanceLimit); }
    else { if (this.offsetY < this.RefreshShowDistance) this.ViewportOffsetY = this.RefreshShowDistance; ind.SetDragRatio(1, this.offsetY, this.RefreshShowDistance, this.RefreshDistanceLimit); }
  }
  /** C# CheckNeedRefresh + ApplyScrollPositionToRefreshViewUnsafe: called from every scroll change. */
  protected CheckNeedRefresh(): void {
    const ind = this.refreshIndicator;
    if (this.isRefreshing) { if (ind && !ind.IsVisible) this.ShowRefreshIndicatorForced(); return; }
    if (!this.RefreshEnabled || !ind) return;
    const horizontal = this.Orientation === "Horizontal";
    const over = horizontal ? this.OverscrollDistance.X : this.OverscrollDistance.Y;
    const offset = horizontal ? this.offsetX : this.offsetY;
    if (over > 0) {
      const ratio = over / this.RefreshShowDistance;
      ind.SetDragRatio(ratio, offset, this.RefreshShowDistance, this.RefreshDistanceLimit);
      const canRefresh = offset > this.UsingRefreshDistanceLimit;
      if (this.IsUserPanning && canRefresh && !this.isRefreshing && this.RefreshCommand && !this.wasRefreshing && !this.scrollLocked) this.IsRefreshing = true;
    } else if (ind.IsVisible && !this.wasRefreshing) this.HideRefreshIndicator();
    else if (over <= 0 && this.wasRefreshing && !this.isRefreshing) { this.wasRefreshing = false; this.HideRefreshIndicator(); }
  }

  // ---- snapping / index tracking (C# SnapToChildren / TrackIndexPosition) ----
  /** Snap to the child at the tracked position after scrolling stops: Center = centered in the viewport, Side = aligned to the viewport start (or end when TrackIndexPosition is End). */
  SnapToChildren: SnapToChildrenType = "Disabled";
  /** The viewport position whose child index is reported as CurrentIndex (None = off). */
  TrackIndexPosition: RelativePositionType = "None";
  /** Points added to the tracked position (C# TrackIndexPositionOffset). */
  TrackIndexPositionOffset = 8;
  /** Index of the content child at the tracked position, -1 when none. */
  CurrentIndex = -1;
  CurrentIndexChanged?: (sender: SkiaScroll, index: number) => void;
  private isSnapping = false;
  private snapped = false;
  /** The content child covering the tracked viewport point (C# CurrentIndexHit): index, its rect and the point, canvas px. */
  protected GetIndexHit(position: RelativePositionType): { Index: number; Area: SKRect; Point: SKPoint } | undefined {
    const layout = this.content instanceof SkiaLayout ? this.content : undefined;
    if (!layout || position === "None") return undefined;
    const r = this.DrawingRect, scale = this.RenderingScale, off = this.TrackIndexPositionOffset * scale;
    const horizontal = this.Orientation === "Horizontal";
    const along = position === "Start" ? off : position === "End" ? (horizontal ? r.Width : r.Height) - off : (horizontal ? r.Width : r.Height) / 2;
    const point = horizontal ? new SKPoint(r.Left + along, r.Top + r.Height / 2) : new SKPoint(r.Left + r.Width / 2, r.Top + along);
    const views = layout.Views;
    for (let i = 0; i < views.length; i++) {
      const v = views[i], a = v.DrawingRect;
      if (!v.IsVisible || a.Width <= 0 || a.Height <= 0) continue;
      const inside = horizontal ? point.X >= a.Left && point.X < a.Right : point.Y >= a.Top && point.Y < a.Bottom;
      if (inside) return { Index: v.ContextIndex >= 0 ? v.ContextIndex : i, Area: a, Point: point };
    }
    return undefined;
  }
  private TrackIndex(): void {
    if (this.TrackIndexPosition === "None") return;
    const hit = this.GetIndexHit(this.TrackIndexPosition);
    const index = hit ? hit.Index : -1;
    if (index !== this.CurrentIndex) { this.CurrentIndex = index; this.CurrentIndexChanged?.(this, index); }
  }
  /** C# CheckNeedToSnap: after the fling stopped, not panning, no bounce, no ordered scroll. */
  protected CheckNeedToSnap(): boolean {
    return !(this.isSnapping || this.snapped || this.IsUserFocused || this.SnapToChildren === "Disabled" || this.bounceX.IsRunning || this.bounceY.IsRunning || this.animatorFlingX.IsRunning || this.animatorFlingY.IsRunning);
  }
  /** C# Snap: scroll so the tracked child is centered / aligned to the side. */
  Snap(maxTimeSecs: number): void {
    if (this.isSnapping) return;
    this.isSnapping = true;
    try {
      const position = this.SnapToChildren === "Center" ? "Center" : this.TrackIndexPosition === "End" ? "End" : "Start";
      const hit = this.GetIndexHit(position);
      if (!hit) return;
      const horizontal = this.Orientation === "Horizontal", scale = this.RenderingScale, r = this.DrawingRect, content = this.content!;
      // geometry from the last paint (child and content rects are consistent there): where the child's anchor sits from the content start
      const contentStart = horizontal ? content.DrawingRect.Left : content.DrawingRect.Top;
      const anchor = this.SnapToChildren === "Center" ? (horizontal ? hit.Area.Left + hit.Area.Width / 2 : hit.Area.Top + hit.Area.Height / 2) : position === "End" ? (horizontal ? hit.Area.Right : hit.Area.Bottom) : (horizontal ? hit.Area.Left : hit.Area.Top);
      const relPts = (anchor - contentStart) / scale;
      const pointPts = ((horizontal ? hit.Point.X - r.Left : hit.Point.Y - r.Top)) / scale;
      const target = pointPts - this.HeaderExtentPts - relPts; // content start + header + rel must land on the tracked point
      const current = horizontal ? this.offsetX : this.offsetY;
      if (Math.abs(target - current) * scale <= scale * 2) return;
      this.snapped = true;
      this.StopAnimators();
      this.ScrollTo(horizontal ? target : this.offsetX, horizontal ? this.offsetY : target, maxTimeSecs, true);
    } finally { this.isSnapping = false; }
  }

  private get HeaderExtentPts(): number {
    if (!this.header) return 0;
    const size = this.Orientation === "Horizontal" ? this.headerSize.Units.Width : this.headerSize.Units.Height;
    return size + (this.HeaderBehind || this.HeaderSticky ? this.ContentOffset : 0);
  }
  private get FooterExtentPts(): number { return this.footer ? (this.Orientation === "Horizontal" ? this.footerSize.Units.Width : this.footerSize.Units.Height) : 0; }

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

  protected override OnDisposing(): void {
    this.animatorFlingX.Stop(); this.animatorFlingY.Stop(); this.bounceX.Stop(); this.bounceY.Stop();
  }
  protected override DisposeChildren(): void {
    this.content?.Dispose(); this.content = undefined;
    this.header?.Dispose(); this.header = undefined;
    this.footer?.Dispose(); this.footer = undefined;
    this.refreshIndicator?.Dispose(); this.refreshIndicator = undefined;
    this.scrollBar?.Dispose(); this.scrollBar = undefined;
    this.scrollBarHorizontal?.Dispose(); this.scrollBarHorizontal = undefined;
  }
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
      a.OnStop = () => {
        this.IsScrolling = this.animatorFlingX.IsRunning || this.animatorFlingY.IsRunning || this.bounceX.IsRunning || this.bounceY.IsRunning;
        if (!this.IsScrolling) { this.Repaint(); queueMicrotask(() => { if (this.CheckNeedToSnap()) this.Snap(this.AutoScrollingSpeedMs / 1000); }); }
      };
    }
    // DrawnUi OnScrollerStopped: a fling that was cut at the edge hands its remaining velocity to the bounce.
    const flingStop = this.animatorFlingY.OnStop!;
    this.animatorFlingY.OnStop = () => { this.LandScrollTo(this.animatorFlingY, false); flingStop(); if (this.animatorFlingY.WasStarted) this.BounceIfNeeded(this.animatorFlingY, false); };
    const flingStopX = this.animatorFlingX.OnStop!;
    this.animatorFlingX.OnStop = () => { this.LandScrollTo(this.animatorFlingX, true); flingStopX(); if (this.animatorFlingX.WasStarted) this.BounceIfNeeded(this.animatorFlingX, true); };
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

  // ---- tree: one Content, plus tagged extras (JSX children with Tag Header / Footer / RefreshIndicator / ScrollBar / ScrollBarHorizontal) ----
  override AddSubView(control: SkiaControl): void {
    switch (control.Tag) {
      case "Header": this.Header = control; return;
      case "Footer": this.Footer = control; return;
      case "RefreshIndicator": this.RefreshIndicator = control as IRefreshIndicator & SkiaControl; return;
      case "ScrollBar": this.ScrollBar = control as IScrollBar & SkiaControl; return;
      case "ScrollBarHorizontal": this.ScrollBarHorizontal = control as IScrollBar & SkiaControl; return;
    }
    this.Content = control;
  }
  override InsertSubView(_index: number, control: SkiaControl): void { this.AddSubView(control); }
  override RemoveSubView(control: SkiaControl): void {
    if (this.content === control) this.Content = undefined;
    else if (this.header === control) this.Header = undefined;
    else if (this.footer === control) this.Footer = undefined;
    else if (this.refreshIndicator === control) this.RefreshIndicator = undefined;
    else if (this.scrollBar === control) this.scrollBar = undefined;
    else if (this.scrollBarHorizontal === control) this.scrollBarHorizontal = undefined;
  }
  /** Draw order: behind header, content, flow / sticky header, footer (the base walks it top-most first for gestures). */
  protected override GetGestureListeners(): readonly SkiaControl[] {
    const list: SkiaControl[] = [];
    if (this.header && this.HeaderBehind) list.push(this.header);
    if (this.content) list.push(this.content);
    if (this.header && !this.HeaderBehind) list.push(this.header);
    if (this.footer) list.push(this.footer);
    return list;
  }

  // ---- measure / arrange ----

  /** Content gets an infinite constraint along the scroll axis; the scroll itself takes the box it is given. */
  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    const c = this.content;
    if (c && c.IsVisible) {
      const w = this.Orientation === "Horizontal" || this.Orientation === "Both" ? Infinity : widthConstraint;
      const h = this.Orientation === "Vertical" || this.Orientation === "Both" ? Infinity : heightConstraint;
      this.ContentSize = c.Measure(w, h, scale);
    } else this.ContentSize = ScaledSize.Default;
    // C#: header / footer measured with the scroll's constraints (they span the viewport across the axis)
    this.headerSize = this.header && this.header.IsVisible ? this.header.Measure(widthConstraint, heightConstraint, scale) : ScaledSize.Default;
    this.footerSize = this.footer && this.footer.IsVisible ? this.footer.Measure(widthConstraint, heightConstraint, scale) : ScaledSize.Default;
    if (this.refreshIndicator) this.refreshIndicator.Measure(widthConstraint, heightConstraint, scale);
    const extra = (this.HeaderExtentPts + this.FooterExtentPts) * scale;
    const rw = isFinite(widthConstraint) ? widthConstraint : this.ContentSize.Pixels.Width + (this.Orientation === "Horizontal" ? extra : 0);
    const rh = isFinite(heightConstraint) ? heightConstraint : this.ContentSize.Pixels.Height + (this.Orientation !== "Horizontal" ? extra : 0);
    return ScaledSize.FromPixels(rw, rh, scale);
  }

  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const viewportW = this.DrawingRect.Width / scale, viewportH = this.DrawingRect.Height / scale;
    const extra = this.HeaderExtentPts + this.FooterExtentPts;
    const width = Math.max(0, this.ContentSize.Units.Width + (this.Orientation === "Horizontal" ? extra : 0) - viewportW);
    const height = Math.max(0, this.ContentSize.Units.Height + (this.Orientation !== "Horizontal" ? extra : 0) - viewportH);
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
    const headerPx = Math.round(this.HeaderExtentPts * scale);
    const x = r.Left + (alongX ? Math.round(this.offsetX * scale) + (this.Orientation === "Horizontal" ? headerPx : 0) : 0);
    const y = r.Top + (alongY ? Math.round(this.offsetY * scale) + (this.Orientation !== "Horizontal" ? headerPx : 0) : 0);
    c.Arrange(SKRect.Create(x, y, w, h), c.WidthRequest, c.HeightRequest, scale);
    this.OverscrollDistance = this.CalculateOverscrollDistance(this.offsetX, this.offsetY);
    this.ArrangeHeaderFooter(r, scale);
  }

  /** Header: sticky = pinned at the viewport start, else scrolled at HeaderParallaxRatio of the offset; footer after the content. */
  private ArrangeHeaderFooter(r: SKRect, scale: number): void {
    const horizontal = this.Orientation === "Horizontal";
    const offset = horizontal ? this.offsetX : this.offsetY;
    if (this.header) {
      let pos = 0;
      if (!this.HeaderSticky) {
        const overscrollStart = (horizontal ? this.OverscrollDistance.X : this.OverscrollDistance.Y) > 0;
        pos = !this.ParallaxOverscrollEnabled && overscrollStart ? offset : offset * this.HeaderParallaxRatio;
      }
      const hw = horizontal ? this.headerSize.Pixels.Width : r.Width, hh = horizontal ? r.Height : this.headerSize.Pixels.Height;
      const rect = horizontal ? SKRect.Create(r.Left + Math.round(pos * scale), r.Top, hw, hh) : SKRect.Create(r.Left, r.Top + Math.round(pos * scale), hw, hh);
      this.header.Arrange(rect, this.header.WidthRequest, this.header.HeightRequest, scale);
    }
    if (this.footer) {
      const contentEnd = (this.HeaderExtentPts + (horizontal ? this.ContentSize.Units.Width : this.ContentSize.Units.Height)) * scale;
      const fw = horizontal ? this.footerSize.Pixels.Width : r.Width, fh = horizontal ? r.Height : this.footerSize.Pixels.Height;
      const rect = horizontal ? SKRect.Create(r.Left + Math.round(offset * scale) + contentEnd, r.Top, fw, fh) : SKRect.Create(r.Left, r.Top + Math.round(offset * scale) + contentEnd, fw, fh);
      this.footer.Arrange(rect, this.footer.WidthRequest, this.footer.HeightRequest, scale);
    }
    if (this.refreshIndicator) {
      const ind = this.refreshIndicator;
      const m = ind.MeasuredSize.Pixels;
      const rect = horizontal ? SKRect.Create(r.Left, r.Top, m.Width, r.Height) : SKRect.Create(r.Left, r.Top, r.Width, m.Height);
      ind.Arrange(rect, ind.WidthRequest, ind.HeightRequest, scale);
    }
    for (const bar of [this.scrollBar, this.scrollBarHorizontal]) if (bar) { bar.Measure(r.Width, r.Height, scale); bar.Arrange(r, bar.WidthRequest, bar.HeightRequest, scale); }
  }

  protected override Paint(ctx: DrawingContext): void {
    const c = this.content;
    if (!c) return;
    this.ArrangeContent();
    const canvas = ctx.Context.Canvas;
    const d = ctx.Destination;
    const saved = canvas.save();
    canvas.clipRect(Super.CK.LTRBRect(d.Left, d.Top, d.Right, d.Bottom), Super.CK.ClipOp.Intersect, true);
    const header = this.header, onScreen = (v: SkiaControl) => { const a = v.DrawingRect; return a.Right > d.Left && a.Left < d.Right && a.Bottom > d.Top && a.Top < d.Bottom; };
    // C# order: a HeaderBehind header first (under the content), the content, a flow / sticky header over it, the footer
    if (header && header.IsVisible && this.HeaderBehind && onScreen(header)) header.Render(ctx);
    c.Render(ctx);
    if (header && header.IsVisible && !this.HeaderBehind && onScreen(header)) header.Render(ctx);
    if (this.footer && this.footer.IsVisible && onScreen(this.footer)) this.footer.Render(ctx);
    if (this.RefreshEnabled && this.refreshIndicator && this.refreshIndicator.IsVisible && (this.OverScrolled || this.isRefreshing)) this.refreshIndicator.Render(ctx);
    this.UpdateScrollBarIndicator();
    for (const bar of [this.scrollBar, this.scrollBarHorizontal]) if (bar && bar.IsVisible && bar.Opacity > 0) bar.Render(ctx);
    canvas.restoreToCount(saved);
  }

  private OnScrolled(): void {
    this.CheckLoadMore();
    this.OverscrollDistance = this.CalculateOverscrollDistance(this.offsetX, this.offsetY);
    this.CheckNeedRefresh();
    this.TrackIndex();
    this.RepaintComposition(); // a scroll inside a cached parent: that parent must re-record (its own cache is None by default)
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

  /** Destination of the running ScrollTo per axis: the deceleration curve stops within its threshold, the exact value is applied when it finishes. */
  private scrollToTargetX: number | null = null;
  private scrollToTargetY: number | null = null;
  private LandScrollTo(animator: ScrollFlingAnimator, horizontal: boolean): void {
    const target = horizontal ? this.scrollToTargetX : this.scrollToTargetY;
    if (target === null) return;
    if (horizontal) this.scrollToTargetX = null; else this.scrollToTargetY = null;
    if (!animator.SelfFinished) return;
    if (horizontal) this.ViewportOffsetX = target; else this.ViewportOffsetY = target;
  }

  ScrollTo(x: number, y: number, maxSpeedSecs: number, clamp = true): void {
    this.StopAnimators(); // also forgets any pending edge bounce: a programmatic scroll never bounces
    let tx = x, ty = y;
    if (clamp) { const c = this.ClampOffset(x, y, this.ContentOffsetBounds, true); tx = c.X; ty = c.Y; }
    const rate = 1 - this.DecelerationRatio;
    this.scrollToTargetX = maxSpeedSecs > 0 && this.Orientation !== "Vertical" && tx !== this.offsetX ? tx : null;
    this.scrollToTargetY = maxSpeedSecs > 0 && this.Orientation !== "Horizontal" && ty !== this.offsetY ? ty : null;
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
    this.scrollToTargetX = null; this.scrollToTargetY = null;
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
      this.snapped = false;
      if (this.RespondsToGestures) { this.StopAnimators(); this.ResetPan(); }
      return super.ProcessGestures(args, apply) ?? consumedDefault; // children see Down (buttons press)
    }

    if (args.Type === "Wheel") {
      // a nested scroll under the pointer takes the wheel first; the outer one scrolls when the inner is at its edge
      const child = super.ProcessGestures(args, apply);
      if (child && child !== this) return child;
      if (!this.RespondsToGestures) return child ?? consumedDefault;
      return this.ApplyWheelScroll(e.Wheel.Delta) ? this : consumedDefault;
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
        // refreshing: the bounce settles at RefreshShowDistance so the indicator stays in view (C# ScrollLocked)
        if (this.isRefreshing && this.scrollLocked) { if (this.Orientation === "Horizontal") rest.X = this.RefreshShowDistance; else rest.Y = this.RefreshShowDistance; }
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
      if (!fling && wasPanning && this.CheckNeedToSnap()) this.Snap(this.AutoScrollingSpeedMs / 1000);
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
  private ApplyWheelScroll(delta: number): boolean {
    const step = SkiaScroll.WheelLineSize * -Math.sign(delta); // wheel down = content up = more negative offset
    const horizontal = this.Orientation === "Horizontal";
    const running = horizontal ? this.animatorFlingX : this.animatorFlingY;
    const base = running.IsRunning && running.Parameters ? running.Parameters.Destination : horizontal ? this.offsetX : this.offsetY;
    const b = this.ContentOffsetBounds;
    const min = horizontal ? b.Left : b.Top, max = horizontal ? b.Right : b.Bottom;
    if ((step < 0 && base <= min) || (step > 0 && base >= max)) return false; // at the edge: let an outer scroll take it
    let x = this.offsetX, y = this.offsetY;
    if (horizontal) x = base + step; else y = base + step;
    this.ScrollTo(x, y, this.AutoScrollingSpeedMs / 1000, true);
    return true;
  }
}
