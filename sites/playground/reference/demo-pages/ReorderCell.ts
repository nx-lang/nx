import { Colors, SkiaDynamicDrawnCell, SkiaLabel, SkiaLayer, SkiaShape, SkiaStack, Thickness } from "drawnui-react/core";
import type { SkiaControl, SkiaGesturesInfo, SkiaScroll } from "drawnui-react/core";

export interface ReorderItem { Id: number; Title: string; Tag: string; Color: string }

/** What the page lends every cell so a drag can move the item, scroll the list under it and carry the ghost. */
export interface DragHost {
  Scroll: () => SkiaScroll | undefined;
  IndexOf: (item: ReorderItem) => number;
  /** Writes the new order; false when the target index is outside the list. */
  Move: (from: number, to: number) => boolean;
  /** The item currently being dragged, so every cell knows which row the ghost is standing in for. */
  Dragging: () => ReorderItem | undefined;
  /** Picks the row up: the ghost takes its place under the pointer and the real row goes blank. */
  Lift: (item: ReorderItem, index: number, pointerY: number) => void;
  /** Carries the ghost with the pointer (canvas pixels). */
  Carry: (pointerY: number) => void;
  /** Released: the ghost glides into the slot at `index`, then the real row draws itself again. */
  Drop: (index: number) => void;
  Spacing: number;
}

/** Points from either end of the viewport where a resting pointer keeps the list moving. */
const EDGE_ZONE = 44;
/** Points per frame it moves there. */
const EDGE_STEP = 7;

/**
 * One row of the reorder list: a grip, a caption and the item id.
 *
 * The drag lives on the grip. A vertical pan inside a vertical SkiaScroll belongs to the scroll — it only lets a child
 * take the pan if the child consumes it — so the grip stands the scroll down on Down and gives it back on Up. Travel
 * is counted in CONTENT space: the pointer's own movement plus whatever the list scrolled underneath it, which is what
 * lets a drag reach a position that was off screen when it started. Each step is one `Move` on the items array, which
 * the layout applies to the cells it already has (SkiaLayout.ApplyReorderChange), so heights and the scroll offset
 * survive and the list reorders live under the finger instead of after the drop.
 *
 * The row itself never travels: the page lifts a ghost copy that follows the pointer over the list while this row
 * draws blank, so the gap left behind is the slot the row is going to land in.
 */
export class ReorderCell extends SkiaDynamicDrawnCell {
  private readonly frame = new SkiaShape();
  private readonly grip = new SkiaLayer(); // absolute: a stack would pack the bars at the top of the row
  private readonly title = new SkiaLabel();
  private readonly badge = new SkiaLabel();

  private dragIndex = -1;
  private startIndex = -1;
  private travel = 0;
  private stride = 0;
  private pointerY = 0;
  private lastOffset = 0;
  private scroll?: SkiaScroll;
  private raf = 0;

  constructor(private readonly host: DragHost) {
    super();
    this.Type = "Absolute";
    this.HeightRequest = 44;
    this.HorizontalOptions = "Fill";

    this.frame.Type = "Rectangle";
    this.frame.CornerRadius = 8;
    this.frame.HorizontalOptions = "Fill";
    this.frame.VerticalOptions = "Fill";
    this.frame.BackgroundColor = "#111827";
    this.frame.StrokeWidth = 1;

    this.grip.WidthRequest = 26;
    this.grip.HorizontalOptions = "Start"; // a SkiaStack fills by default, and a Fill child is stretched at arrange
    this.grip.VerticalOptions = "Fill";    // the whole row height is grabbable, the bars just sit in the middle
    this.grip.AccessibilityRole = "button";
    this.grip.Margin = new Thickness(10, 0, 0, 0);
    this.grip.AddSubView(ReorderBars());
    this.grip.ConsumeGestures = (sender, e) => this.OnGrip(sender, e);

    this.title.FontSize = 14;
    this.title.TextColor = Colors.White;
    this.title.VerticalOptions = "Center";
    this.title.Margin = new Thickness(46, 0, 60, 0);

    this.badge.FontSize = 12;
    this.badge.TextColor = "#94A3B8";
    this.badge.HorizontalOptions = "End";
    this.badge.VerticalOptions = "Center";
    this.badge.Margin = new Thickness(0, 0, 14, 0);

    this.AddSubView(this.frame);
    this.AddSubView(this.grip);
    this.AddSubView(this.title);
    this.AddSubView(this.badge);
  }

  /** Re-applies the look when the list itself did not change: a lift blanks one row, the landing brings it back. */
  Refresh(): void {
    if (this.BindingContext) this.SetContent(this.BindingContext);
  }

  protected override SetContent(ctx: unknown): void {
    const item = ctx as ReorderItem;
    this.grip.AccessibilityLabel = `Reorder ${item.Title}`;
    this.title.Text = item.Title;
    this.badge.Text = item.Tag;
    this.frame.StrokeColor = item.Color;

    // the ghost is standing in for this row: leave the gap it is going to drop into
    const opacity = this.host.Dragging() === item ? 0 : 1;
    if (this.Opacity !== opacity) { this.Opacity = opacity; this.RepaintComposition(); }
  }

  private OnGrip(_sender: SkiaControl, e: SkiaGesturesInfo): void {
    const type = e.Args.Type;

    if (type === "Down") {
      this.EndDrag(); // a previous drag that never saw its Up
      const item = this.BindingContext as ReorderItem | undefined;
      this.dragIndex = item ? this.host.IndexOf(item) : -1;
      e.Consumed = this.dragIndex >= 0;
      if (!e.Consumed || !item) return;

      this.startIndex = this.dragIndex;
      this.travel = 0;
      this.stride = this.MeasuredSize.Units.Height + this.host.Spacing;
      this.pointerY = e.Args.Event.Location.Y;
      this.scroll = this.host.Scroll();
      this.lastOffset = this.scroll?.ViewportOffsetY ?? 0;
      if (this.scroll) this.scroll.RespondsToGestures = false; // the pan is ours for the whole drag
      this.host.Lift(item, this.dragIndex, this.pointerY);
      if (!this.raf) this.raf = requestAnimationFrame(this.Tick);
      return;
    }

    if (this.dragIndex < 0) return;

    if (type === "Panning") {
      this.pointerY = e.Args.Event.Location.Y;
      this.travel += e.Args.Event.Distance.Delta.Y / this.RenderingScale;
      e.Consumed = true;
      return;
    }

    if (type === "Up" || type === "Touch") {
      // released outside the list is a cancel: the row goes back to where it was picked up from
      const viewport = this.scroll?.DrawingRect;
      const p = e.Args.Event.Location;
      const outside = !!viewport
        && (p.Y < viewport.Top || p.Y > viewport.Bottom || p.X < viewport.Left || p.X > viewport.Right);
      if (outside && this.dragIndex !== this.startIndex && this.host.Move(this.dragIndex, this.startIndex)) {
        this.dragIndex = this.startIndex;
      }
      this.host.Drop(this.dragIndex);
      this.EndDrag();
    }
  }

  /** One frame of the drag: hold at an edge and the list keeps moving, so the row keeps advancing. */
  private readonly Tick = (): void => {
    this.raf = 0;
    if (this.dragIndex < 0) return;

    const scroll = this.scroll;
    if (scroll) {
      const scale = scroll.RenderingScale || 1;
      const zone = EDGE_ZONE * scale;
      const direction = this.pointerY < scroll.DrawingRect.Top + zone ? 1
        : this.pointerY > scroll.DrawingRect.Bottom - zone ? -1 : 0;
      if (direction !== 0) scroll.ScrollTo(scroll.ViewportOffsetX, scroll.ViewportOffsetY + direction * EDGE_STEP, 0, true);

      const offset = scroll.ViewportOffsetY;
      this.travel += this.lastOffset - offset; // the list moving under the pointer advances the row as well
      this.lastOffset = offset;
    }

    this.host.Carry(this.pointerY);

    while (this.stride > 0 && Math.abs(this.travel) >= this.stride) {
      const step = Math.sign(this.travel);
      if (!this.host.Move(this.dragIndex, this.dragIndex + step)) { this.travel = 0; break; } // hit an end
      this.dragIndex += step;
      this.travel -= step * this.stride;
    }

    this.raf = requestAnimationFrame(this.Tick);
  };

  /** Ends a drag: stops the ticker and gives the scroll its gestures back. Settling the ghost is the page's job. */
  private EndDrag(): void {
    if (this.raf) { cancelAnimationFrame(this.raf); this.raf = 0; }
    this.dragIndex = -1;
    this.startIndex = -1;
    if (this.scroll) { this.scroll.RespondsToGestures = true; this.scroll = undefined; }
  }

  override Dispose(): void {
    this.EndDrag();
    super.Dispose();
  }
}

/** The three grip bars, shared by a row and by the ghost that stands in for it. */
function ReorderBars(): SkiaStack {
  const bars = new SkiaStack();
  bars.Spacing = 3;
  bars.VerticalOptions = "Center";
  bars.HorizontalOptions = "Fill";
  for (let i = 0; i < 3; i++) {
    const bar = new SkiaShape();
    bar.Type = "Rectangle";
    bar.CornerRadius = 1;
    bar.HeightRequest = 2;
    bar.WidthRequest = 16;
    bar.BackgroundColor = "#94A3B8";
    bar.HorizontalOptions = "Center";
    bars.AddSubView(bar);
  }
  return bars;
}
