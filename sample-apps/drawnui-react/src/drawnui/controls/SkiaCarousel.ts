import type { SkiaControl } from "../core/SkiaControl";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import { SKRect, ScaledSize, Thickness } from "../core/Types";
import { SnappingLayout } from "./SnappingLayout";

/**
 * Mirrors DrawnUi SkiaCarousel (static children): every child is a full-size cell laid along the axis,
 * `SidesOffset` peeks the neighbours, `Spacing` separates cells; swipe / tap-release snaps to the nearest
 * cell by velocity, `SelectedIndex` drives and reports the position, `GoNext`/`GoPrev`/`ScrollTo`.
 */
export class SkiaCarousel extends SnappingLayout {
  IsVertical = false;
  /** Side padding in points so the previous/next cells peek in. */
  SidesOffset = 0;
  IsRightToLeft = false;
  SelectedIndexChanged?: (sender: SkiaCarousel, index: number) => void;

  private selectedIndex = 0;
  private lastCellSize = { W: 0, H: 0 };
  private panningOffset = SKPoint.Empty;
  private panningStart = SKPoint.Empty;
  private wrongDirection = false;
  private snapIfNoPanOnUp = false;
  private hadDown = false;

  constructor() {
    super();
    this.HorizontalOptions = "Fill";
    this.IsClippedToBounds = true; // C# WillClipBounds => true
  }

  get SelectedIndex(): number { return this.selectedIndex; }
  set SelectedIndex(v: number) {
    if (this.selectedIndex === v) return;
    this.selectedIndex = v;
    this.SelectedIndexChanged?.(this, v);
    this.NotifyAccessibility();
    if (this.SnapPoints.length > 0 && !this.IsUserPanning) this.ApplyIndex(false);
  }
  get MaxIndex(): number { return this.Views.length - 1; }
  get ChildrenTotal(): number { return this.Views.length; }
  get IsAtStart(): boolean { return this.selectedIndex === 0; }
  get IsAtEnd(): boolean { return this.selectedIndex === this.MaxIndex; }
  /** 0..1 progress of the content along all cells (C# ScrollProgress). */
  get ScrollProgress(): number {
    const last = this.SnapPoints[this.SnapPoints.length - 1];
    if (!last) return 0;
    const max = this.IsVertical ? last.Y : last.X;
    return max === 0 ? 0 : (this.IsVertical ? this.CurrentPosition.Y : this.CurrentPosition.X) / max;
  }

  GoNext(): void { if (this.selectedIndex < this.MaxIndex) this.SelectedIndex = this.selectedIndex + 1; }
  GoPrev(): void { if (this.selectedIndex > 0) this.SelectedIndex = this.selectedIndex - 1; }
  /** C# ScrollTo(index, animate). */
  ScrollTo(index: number, animate = true): void {
    const i = Math.max(0, Math.min(this.MaxIndex, index));
    if (i === this.selectedIndex) { this.ApplyIndex(!animate); return; }
    if (!animate) { this.selectedIndex = i; this.SelectedIndexChanged?.(this, i); this.ApplyIndex(true); return; }
    this.SelectedIndex = i;
  }

  /** Step between cells in points: cell size + Spacing - 2 * SidesOffset (C# InitializeChildren). */
  private Step(): number { const size = this.IsVertical ? this.lastCellSize.H : this.lastCellSize.W; return size + this.Spacing - 2 * this.SidesOffset; }

  private ApplyIndex(instant: boolean): void {
    const snap = this.SnapPoints[this.selectedIndex];
    if (!snap) return;
    this.ScrollToOffset(snap, SKPoint.Empty, !instant && this.CanAnimate && this.Animated);
  }

  override UpdateReportedPosition(): void {
    if (this.SnapPoints.length === 0) return;
    const i = this.SnapPoints.findIndex((p) => Math.abs(p.X - this.CurrentSnap.X) <= 1 && Math.abs(p.Y - this.CurrentSnap.Y) <= 1);
    if (i >= 0 && i !== this.selectedIndex) { this.selectedIndex = i; this.SelectedIndexChanged?.(this, i); this.NotifyAccessibility(); }
  }

  override ApplyPosition(position: SKPoint): void {
    super.ApplyPosition(position);
    this.RepaintComposition();
  }

  // ---- layout: every child is one cell ----
  protected override MeasureAbsolute(w: number, h: number, scale: number): ScaledSize {
    const px = this.Padding.HorizontalThickness * scale, py = this.Padding.VerticalThickness * scale;
    const cw = isFinite(w) ? w - px : 0, ch = isFinite(h) ? h - py : 0;
    let maxH = 0, maxW = 0;
    for (const v of this.Views) {
      if (!v.IsVisible) continue;
      v.HorizontalOptions = "Fill"; v.VerticalOptions = "Fill";
      v.Margin = this.IsVertical ? new Thickness(0, this.SidesOffset, 0, this.SidesOffset) : new Thickness(this.SidesOffset, 0, this.SidesOffset, 0);
      const s = v.Measure(cw, ch, scale);
      maxW = Math.max(maxW, s.Pixels.Width); maxH = Math.max(maxH, s.Pixels.Height);
    }
    return ScaledSize.FromPixels((isFinite(w) ? w : maxW + px), (isFinite(h) ? h : maxH + py), scale);
  }

  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const p = this.Padding;
    const r = this.DrawingRect;
    const inner = new SKRect(r.Left + p.Left * scale, r.Top + p.Top * scale, r.Right - p.Right * scale, r.Bottom - p.Bottom * scale);
    const cellW = inner.Width / scale, cellH = inner.Height / scale;
    if (cellW !== this.lastCellSize.W || cellH !== this.lastCellSize.H || this.SnapPoints.length !== this.Views.length) {
      this.lastCellSize = { W: cellW, H: cellH };
      this.SnapPoints = this.Views.map((_, i) => (this.IsVertical ? new SKPoint(0, -i * this.Step()) : new SKPoint(-i * this.Step(), 0)));
      this.ContentOffsetBounds = this.BoundsFromSnapPoints();
      if (this.selectedIndex > this.MaxIndex) this.selectedIndex = Math.max(0, this.MaxIndex);
      this.CurrentSnap = new SKPoint(-1, -1);
      this.ApplyIndex(true);
    }
    const step = this.Step() * scale;
    const ox = this.CurrentPosition.X * scale, oy = this.CurrentPosition.Y * scale;
    for (let i = 0; i < this.Views.length; i++) {
      const v = this.Views[i];
      if (!v.IsVisible) continue;
      const cell = this.IsVertical
        ? SKRect.Create(inner.Left, inner.Top + i * step + oy, inner.Width, inner.Height)
        : SKRect.Create(inner.Left + i * step + ox, inner.Top, inner.Width, inner.Height);
      v.Arrange(cell, v.WidthRequest, v.HeightRequest, scale);
    }
  }

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
      this.snapIfNoPanOnUp = this.InTransition;
      this.StopSnapAnimators(); this.velocityAccumulator.Clear();
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
          const v = new SKPoint(Math.abs(final.X) < 100 ? 0 : final.X, Math.abs(final.Y) < 100 ? 0 : final.Y);
          this.CurrentSnap = this.CurrentPosition;
          this.ScrollToNearestAnchorByDrag(v);
          this.IsUserPanning = false; this.IsUserFocused = false; this.snapIfNoPanOnUp = false;
        } else if (this.snapIfNoPanOnUp) {
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

  /** No velocity: a drag past SnapDistanceRatio of a step still moves to the next cell (C# SelectNextAnchor drag branch). */
  private ScrollToNearestAnchorByDrag(velocity: SKPoint): void {
    if (velocity.X === 0 && velocity.Y === 0) {
      const delta = this.IsVertical ? this.CurrentPosition.Y - this.panningStart.Y : this.CurrentPosition.X - this.panningStart.X;
      const step = this.Step();
      if (step > 0 && Math.abs(delta) >= step * this.SnapDistanceRatio) velocity = this.IsVertical ? new SKPoint(0, Math.sign(delta)) : new SKPoint(Math.sign(delta), 0);
    }
    this.ScrollToNearestAnchor(this.CurrentPosition, velocity);
  }

  protected override DefaultAccessibilityLabel(): string | undefined { return `${this.selectedIndex + 1} / ${this.ChildrenTotal}`; }
}
