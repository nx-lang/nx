import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { ViewsAdapter } from "../core/ViewsAdapter";
import { type GridLength, type LayoutType, type MeasuringStrategy, type RecyclingTemplate, SKRect, ScaledSize, type ShapeType, Thickness } from "../core/Types";
import { SkiaGridStructure } from "./GridStructure";

/**
 * Mirrors DrawnUi SkiaLayout (Absolute / Column / Row / Wrap / Grid).
 * Column/Row give children an infinite main axis (MAUI stack semantics: Fill on the main axis = auto-sized).
 *
 * Templated mode (ItemsSource + ItemTemplate) is a Column only: cells are created through the ViewsAdapter for
 * the indexes inside the visible viewport (+ VirtualisationInflated), everything else is arithmetic —
 * MeasureFirst measures one cell and assumes uniform size, MeasureAll measures every item once.
 */
export class SkiaLayout extends SkiaControl {
  /** Layout type. SkiaShape redeclares it as ShapeType (any shape value lays out as Absolute), like the C# hidden Type. */
  Type: LayoutType | ShapeType = "Absolute";
  Spacing = 0;
  Padding: Thickness = Thickness.Zero;

  // ---- grid (same names as DrawnUi; definitions as "*, 2*, Auto, 100" or an array) ----
  ColumnDefinitions?: string | GridLength[];
  RowDefinitions?: string | GridLength[];
  /** Track used for columns/rows a child references but no definition declares (C# default: Auto). */
  DefaultColumnDefinition: GridLength = "Auto";
  DefaultRowDefinition: GridLength = "Auto";
  ColumnSpacing = 0;
  RowSpacing = 0;
  /** Structure computed by the last measure; cells are arranged from it. */
  GridStructure?: SkiaGridStructure;

  // ---- templated children (same names as DrawnUi) ----
  RecyclingTemplate: RecyclingTemplate = "Enabled";
  MeasureItemsStrategy: MeasuringStrategy = "MeasureFirst";
  /** Realized views per item (Disabled) or a recycled pool for the visible range (Enabled). */
  readonly ChildrenFactory = new ViewsAdapter(this);
  FirstVisibleIndex = -1;
  LastVisibleIndex = -1;

  private itemsSource?: readonly unknown[];
  private itemTemplate?: () => SkiaControl;
  private structureDirty = true;
  /** Per-item heights in pixels (MeasureAll) or a single uniform height (MeasureFirst). */
  private itemHeights: number[] = [];
  private uniformHeight = 0;
  private measuredWidthPx = 0;

  // ---- MeasureVisible (DrawnUi experimental strategy): measured prefix + average estimate for the rest ----
  /** Items measured per idle slice before the estimate is refreshed (DrawnUi BackgroundMeasurementBatchSize). */
  BackgroundMeasurementBatchSize = 10;
  /** Pixel heights of measured items (0 = not measured yet), any order: visible cells are measured on demand. */
  private mvHeights = new Float64Array(0);
  /** mvPrefix[i] = offset of item i (inside padding, gaps included) for i <= mvMeasured; items 0..mvMeasured-1 are exact. */
  private mvPrefix = new Float64Array(1);
  private mvMeasured = 0;
  private mvSum = 0;
  private mvCount = 0;
  private mvIdle = 0;
  /** Index of the last item measured by the background pass (DrawnUi LastMeasuredIndex); -1 = none. */
  get LastMeasuredIndex(): number { return this.mvMeasured - 1; }

  get ItemsSource(): readonly unknown[] | undefined { return this.itemsSource; }
  set ItemsSource(value: readonly unknown[] | undefined) {
    if (this.itemsSource === value) return;
    const old = this.itemsSource;
    this.itemsSource = value;
    if (value && old && this.IsTemplated && !this.structureDirty && this.ApplyIncrementalChange(old, value)) { this.InvalidateMeasure(); return; }
    if (value && this.IsTemplated) this.ChildrenFactory.UpdateItems(value);
    this.structureDirty = true;
    this.InvalidateMeasure();
  }

  /**
   * Raised after items were inserted at the head and measured, with the inserted extent in pixels; SkiaScroll uses it to
   * keep the visible rows where they are (DrawnUi head-insert rebase).
   */
  ItemsInsertedAtStart?: (sender: SkiaLayout, insertedPx: number) => void;

  /**
   * DrawnUi keeps the structure on ObservableCollection Add/Insert; React apps replace the array, so the relation is
   * detected instead: same items appended (page loaded) or prepended (chat history) keep every measured height,
   * anything else rebuilds. Returns false when a full rebuild is needed.
   */
  private ApplyIncrementalChange(old: readonly unknown[], items: readonly unknown[]): boolean {
    const n = items.length, o = old.length;
    if (n <= o || o === 0) return false;
    const k = n - o;
    const scale = this.RenderingScale, gap = this.Spacing * scale, w = this.measuredWidthPx;
    if (this.MeasureItemsStrategy === "MeasureFirst" && this.uniformHeight <= 0) return false;
    const sameAt = (offset: number) => old[0] === items[offset] && old[o - 1] === items[offset + o - 1] && old[o >> 1] === items[offset + (o >> 1)];
    if (sameAt(0)) {
      // append: heights of the new tail are measured lazily (MeasureAll now, MeasureVisible on demand / in idle time)
      this.ChildrenFactory.UpdateItems(items);
      if (this.MeasureItemsStrategy === "MeasureAll") for (let i = o; i < n; i++) this.itemHeights.push(this.MeasureItem(i, w, scale));
      else if (this.MeasureItemsStrategy === "MeasureVisible") {
        const heights = new Float64Array(n); heights.set(this.mvHeights);
        const prefix = new Float64Array(n + 1); prefix.set(this.mvPrefix);
        this.mvHeights = heights; this.mvPrefix = prefix;
      }
      return true;
    }
    if (sameAt(k)) {
      // prepend: shift indices, measure the new head synchronously (bounded), report the inserted extent
      this.ChildrenFactory.ShiftIndices(k, items);
      let insertedPx = 0;
      if (this.MeasureItemsStrategy === "MeasureAll") {
        const head: number[] = [];
        for (let i = 0; i < k; i++) head.push(this.MeasureItem(i, w, scale));
        this.itemHeights.unshift(...head);
        for (const h of head) insertedPx += h + gap;
      } else if (this.MeasureItemsStrategy === "MeasureVisible") {
        const heights = new Float64Array(n); heights.set(this.mvHeights, k);
        this.mvHeights = heights;
        this.mvPrefix = new Float64Array(n + 1);
        this.mvMeasured = 0;
        const sync = Math.min(k, Math.max(this.BackgroundMeasurementBatchSize, 200));
        for (let i = 0; i < sync; i++) insertedPx += this.MvMeasure(i, w, scale, true) + gap;
        if (sync < k) insertedPx += (k - sync) * this.MvStride(gap); // the rest is estimated until the idle pass reaches it
        this.MvExtendPrefix(gap);
      } else {
        insertedPx = k * (this.uniformHeight + gap);
      }
      this.ItemsInsertedAtStart?.(this, insertedPx);
      return true;
    }
    return false;
  }

  /** Factory creating one cell (DrawnUi DataTemplate). Cells receive the item as BindingContext. */
  get ItemTemplate(): (() => SkiaControl) | undefined { return this.itemTemplate; }
  set ItemTemplate(value: (() => SkiaControl) | undefined) {
    if (this.itemTemplate === value) return;
    this.itemTemplate = value;
    this.ApplyItemsSource();
  }

  get IsTemplated(): boolean { return !!this.itemTemplate && !!this.itemsSource; }

  /** Drops realized cells and rebuilds the structure (DrawnUi ApplyItemsSource). */
  ApplyItemsSource(): void {
    this.ChildrenFactory.Initialize(this.itemTemplate, this.itemsSource ?? [], this.RecyclingTemplate);
    this.structureDirty = true;
    this.InvalidateMeasure();
  }

  /** Diagnostics like DrawnUi DebugString: visible range, realized views, pool. */
  get DebugString(): string {
    if (!this.IsTemplated) return `views ${this.views.length}`;
    const f = this.ChildrenFactory;
    const measured = this.MeasureItemsStrategy === "MeasureVisible" ? ` measured ${this.mvMeasured}/${this.itemsSource!.length}` : "";
    return `items ${this.itemsSource!.length} visible ${this.FirstVisibleIndex}-${this.LastVisibleIndex}${measured} inuse ${f.InUseCount} pool ${f.PoolSize} created ${f.Created}`;
  }

  /** A layout paints outside its box whatever its children paint outside theirs (C# aggregated effects margin). */
  override ComputeEffectsMargin(scale: number): Thickness {
    let l = 0, t = 0, r = 0, b = 0;
    const mine = this.DrawingRect;
    for (const v of this.views) {
      if (!v.IsVisible) continue;
      const m = v.EffectsMargin(scale);
      if (m.Left === 0 && m.Top === 0 && m.Right === 0 && m.Bottom === 0) continue;
      const cr = v.DrawingRect; // child overflow beyond this box, if already arranged
      l = Math.max(l, m.Left - Math.max(0, cr.Left - mine.Left)); t = Math.max(t, m.Top - Math.max(0, cr.Top - mine.Top));
      r = Math.max(r, m.Right - Math.max(0, mine.Right - cr.Right)); b = Math.max(b, m.Bottom - Math.max(0, mine.Bottom - cr.Bottom));
    }
    return new Thickness(Math.max(0, l), Math.max(0, t), Math.max(0, r), Math.max(0, b));
  }

  // ---- static children ----

  private readonly views: SkiaControl[] = [];
  /** Read-only live children like DrawnUi Views: realized cells when templated, else static children. */
  get Views(): readonly SkiaControl[] { return this.IsTemplated ? this.ChildrenFactory.GetViewsInUse() : this.views; }
  /** Settable children list like DrawnUi Children (ignored while templated). */
  get Children(): readonly SkiaControl[] { return this.views; }
  set Children(value: readonly SkiaControl[]) {
    for (const v of [...this.views]) this.RemoveSubView(v);
    for (const v of value) this.AddSubView(v);
  }

  protected override GetGestureListeners(): readonly SkiaControl[] { return this.Views; }

  override AddSubView(control: SkiaControl): void { this.InsertSubView(this.views.length, control); }

  override InsertSubView(index: number, control: SkiaControl): void {
    control.Parent = this;
    this.views.splice(index, 0, control);
    this.InvalidateMeasure();
  }

  override RemoveSubView(control: SkiaControl): void {
    const i = this.views.indexOf(control);
    if (i < 0) return;
    this.views.splice(i, 1);
    control.Parent = undefined;
    this.InvalidateMeasure();
  }

  // ---- measure ----

  protected override MeasureAbsolute(widthConstraint: number, heightConstraint: number, scale: number): ScaledSize {
    if (this.IsTemplated) return this.MeasureTemplated(widthConstraint, heightConstraint, scale);
    const px = this.Padding.HorizontalThickness * scale;
    const py = this.Padding.VerticalThickness * scale;
    const w = widthConstraint - px;
    const h = heightConstraint - py;
    const gap = this.Spacing * scale;
    let cw = 0, ch = 0, n = 0;

    if (this.Type === "Wrap") { const s = this.MeasureWrap(w, scale); return ScaledSize.FromPixels(s.w + px, s.h + py, scale); }
    if (this.Type === "Grid") { const s = this.MeasureGrid(w, h, scale); return ScaledSize.FromPixels(s.w + px, s.h + py, scale); }

    for (const v of this.views) {
      if (!v.IsVisible) continue;
      let s: ScaledSize;
      if (this.Type === "Column") { s = v.Measure(w, Infinity, scale); cw = Math.max(cw, s.Pixels.Width); ch += s.Pixels.Height; }
      else if (this.Type === "Row") { s = v.Measure(Infinity, h, scale); cw += s.Pixels.Width; ch = Math.max(ch, s.Pixels.Height); }
      else { s = v.Measure(w, h, scale); cw = Math.max(cw, s.Pixels.Width); ch = Math.max(ch, s.Pixels.Height); }
      n++;
    }
    const gaps = Math.max(0, n - 1) * gap;
    if (this.Type === "Column") ch += gaps;
    if (this.Type === "Row") cw += gaps;
    return ScaledSize.FromPixels(cw + px, ch + py, scale);
  }

  /** Wrap rows computed by the last measure: [childIndex, x, y, w, h] in pixels relative to the padded box. */
  private wrapSlots: { view: SkiaControl; x: number; y: number; w: number; h: number }[] = [];

  /** Flow children left to right, wrapping when the row overflows; Spacing applies between items and rows. */
  private MeasureWrap(width: number, scale: number): { w: number; h: number } {
    const gap = this.Spacing * scale;
    this.wrapSlots = [];
    let x = 0, y = 0, rowH = 0, maxW = 0;
    const row: typeof this.wrapSlots = [];
    const finishRow = () => { for (const s of row) s.h = rowH; row.length = 0; };
    for (const v of this.views) {
      if (!v.IsVisible) continue;
      const s = v.Measure(width, Infinity, scale);
      const cw = s.Pixels.Width, ch = s.Pixels.Height;
      if (x > 0 && isFinite(width) && x + cw > width) { finishRow(); y += rowH + gap; x = 0; rowH = 0; }
      const slot = { view: v, x, y, w: cw, h: ch };
      this.wrapSlots.push(slot); row.push(slot);
      x += cw + gap; rowH = Math.max(rowH, ch); maxW = Math.max(maxW, x - gap);
    }
    finishRow();
    return { w: maxW, h: this.wrapSlots.length ? y + rowH : 0 };
  }

  /** Port of C# MeasureGrid: build the structure in points, stretch the last track when the grid fills, remeasure at final cells. */
  private MeasureGrid(widthPx: number, heightPx: number, scale: number): { w: number; h: number } {
    const wPts = widthPx / scale, hPts = heightPx / scale;
    const g = new SkiaGridStructure(this, wPts, hPts, scale);
    g.DecompressStars(wPts, hPts);
    const needAutoWidth = this.WidthRequest < 0 && this.HorizontalOptions !== "Fill";
    const needAutoHeight = this.HeightRequest < 0 && this.VerticalOptions !== "Fill";
    if (!needAutoWidth && g.Columns.length > 0 && isFinite(wPts) && g.GridWidth() < wPts) g.Columns[g.Columns.length - 1].Size += wPts - g.GridWidth();
    if (!needAutoHeight && g.Rows.length > 0 && isFinite(hPts) && g.GridHeight() < hPts) g.Rows[g.Rows.length - 1].Size += hPts - g.GridHeight();
    g.RemeasureChildrenAtFinalCells();
    this.GridStructure = g;
    return { w: g.GridWidth() * scale, h: g.GridHeight() * scale };
  }

  /** Templated Column: content = padding + sum of item heights + gaps; width = the constraint (cells fill). */
  private MeasureTemplated(widthConstraint: number, _heightConstraint: number, scale: number): ScaledSize {
    const items = this.itemsSource!;
    const px = this.Padding.HorizontalThickness * scale;
    const py = this.Padding.VerticalThickness * scale;
    const w = isFinite(widthConstraint) ? widthConstraint - px : 0;
    const gap = this.Spacing * scale;

    if (this.structureDirty || this.measuredWidthPx !== w) {
      this.measuredWidthPx = w;
      this.itemHeights = [];
      this.uniformHeight = 0;
      this.MvReset(items.length);
      if (items.length > 0) {
        if (this.MeasureItemsStrategy === "MeasureAll") {
          for (let i = 0; i < items.length; i++) this.itemHeights.push(this.MeasureItem(i, w, scale));
        } else if (this.MeasureItemsStrategy === "MeasureVisible") {
          // initial pass: enough items to fill the viewport (at least one batch), the rest is estimated
          const viewportH = this.GetVisibleViewport().Height;
          for (let i = 0; i < items.length; i++) {
            this.MvMeasure(i, w, scale, true);
            this.MvExtendPrefix(gap);
            if (i + 1 >= this.BackgroundMeasurementBatchSize && this.mvPrefix[this.mvMeasured] >= viewportH + this.VirtualisationInflated * scale) break;
          }
        } else {
          this.uniformHeight = this.MeasureItem(0, w, scale);
        }
      }
      this.structureDirty = false;
    }

    let total = 0;
    if (this.MeasureItemsStrategy === "MeasureAll") {
      while (this.itemHeights.length < items.length) this.itemHeights.push(this.MeasureItem(this.itemHeights.length, w, scale));
      for (const h of this.itemHeights) total += h; total += Math.max(0, items.length - 1) * gap;
    }
    else if (this.MeasureItemsStrategy === "MeasureVisible") { total = this.MvContentHeight(gap); this.MvScheduleBackground(); }
    else total = this.uniformHeight * items.length + Math.max(0, items.length - 1) * gap;
    return ScaledSize.FromPixels(w + px, total + py, scale);
  }

  private MvReset(count: number): void {
    if (this.mvIdle) { (window.cancelIdleCallback ?? clearTimeout)(this.mvIdle); this.mvIdle = 0; }
    this.mvHeights = new Float64Array(count);
    this.mvPrefix = new Float64Array(count + 1);
    this.mvMeasured = 0; this.mvSum = 0; this.mvCount = 0;
  }

  /** Average item stride (height + gap) from everything measured so far. */
  private MvStride(gap: number): number { return (this.mvCount > 0 ? this.mvSum / this.mvCount : 0) + gap; }

  /** DrawnUi MeasureList estimate: exact prefix + average × remaining. */
  private MvContentHeight(gap: number): number {
    const n = this.mvHeights.length;
    if (n === 0) return 0;
    return this.mvPrefix[this.mvMeasured] + (n - this.mvMeasured) * this.MvStride(gap) - gap;
  }

  /** Measures one item (binding it through the adapter); release = give the cell back to the pool right away. */
  private MvMeasure(index: number, widthPx: number, scale: number, release: boolean): number {
    const known = this.mvHeights[index];
    if (known > 0) return known;
    const view = this.ChildrenFactory.GetOrCreateViewForIndex(index);
    if (!view) return 0;
    const h = view.Measure(widthPx, Infinity, scale).Pixels.Height;
    if (release && this.RecyclingTemplate === "Enabled") this.ChildrenFactory.ReleaseViewAt(index);
    this.mvHeights[index] = h;
    this.mvSum += h; this.mvCount++;
    return h;
  }

  /** Grows the exact prefix over every already-measured item that follows it. */
  private MvExtendPrefix(gap: number): void {
    const n = this.mvHeights.length;
    while (this.mvMeasured < n && this.mvHeights[this.mvMeasured] > 0) {
      this.mvPrefix[this.mvMeasured + 1] = this.mvPrefix[this.mvMeasured] + this.mvHeights[this.mvMeasured] + gap;
      this.mvMeasured++;
    }
  }

  /** Background measurement in idle time (DrawnUi StartBackgroundMeasurement): time-sliced, then the estimate is refreshed once. */
  private MvScheduleBackground(): void {
    if (this.mvIdle || this.mvMeasured >= this.mvHeights.length || !this.Superview) return;
    const run = (deadline?: IdleDeadline) => {
      this.mvIdle = 0;
      if (this.MeasureItemsStrategy !== "MeasureVisible" || !this.IsTemplated || !this.Superview) return;
      const scale = this.RenderingScale, gap = this.Spacing * scale, w = this.measuredWidthPx;
      const before = this.MvContentHeight(gap);
      const started = performance.now();
      const budget = () => (deadline ? deadline.timeRemaining() > 1 : performance.now() - started < 8);
      let measured = 0;
      while (this.mvMeasured < this.mvHeights.length && (measured < this.BackgroundMeasurementBatchSize || budget())) {
        this.MvMeasure(this.mvMeasured, w, scale, true);
        this.MvExtendPrefix(gap);
        measured++;
      }
      if (Math.abs(this.MvContentHeight(gap) - before) > 0.5) this.InvalidateMeasure(); // parents (the scroll) pick up the new extent
      if (this.mvMeasured < this.mvHeights.length) this.MvScheduleBackground();
    };
    this.mvIdle = window.requestIdleCallback ? window.requestIdleCallback(run, { timeout: 200 }) : window.setTimeout(() => run(), 16);
  }

  /** Binds a cell to items[index] (through the adapter, so MeasureFirst's cell 0 stays realized) and measures it. */
  private MeasureItem(index: number, widthPx: number, scale: number): number {
    const view = this.ChildrenFactory.GetOrCreateViewForIndex(index);
    if (!view) return 0;
    const h = view.Measure(widthPx, Infinity, scale).Pixels.Height;
    if (this.MeasureItemsStrategy === "MeasureAll" && this.RecyclingTemplate === "Enabled") this.ChildrenFactory.ReleaseViewAt(index);
    return h;
  }

  /** Pixel offset of item index from the top of the layout content (inside padding). */
  GetItemOffsetPixels(index: number): number {
    const scale = this.RenderingScale;
    const gap = this.Spacing * scale;
    let y = this.Padding.Top * scale;
    if (this.MeasureItemsStrategy === "MeasureAll") for (let i = 0; i < index && i < this.itemHeights.length; i++) y += this.itemHeights[i] + gap;
    else if (this.MeasureItemsStrategy === "MeasureVisible") y += index <= this.mvMeasured ? this.mvPrefix[index] : this.mvPrefix[this.mvMeasured] + (index - this.mvMeasured) * this.MvStride(gap);
    else y += index * (this.uniformHeight + gap);
    return y;
  }

  private ItemHeightPixels(index: number): number {
    if (this.MeasureItemsStrategy === "MeasureAll") return this.itemHeights[index] ?? 0;
    if (this.MeasureItemsStrategy === "MeasureVisible") return this.mvHeights[index] || this.MvStride(this.Spacing * this.RenderingScale) - this.Spacing * this.RenderingScale;
    return this.uniformHeight;
  }

  // ---- arrange ----

  protected override OnLayoutChanged(): void {
    if (this.IsTemplated) return; // cells are arranged per frame for the visible range only
    const scale = this.RenderingScale;
    const p = this.Padding;
    const r = this.DrawingRect;
    const inner = new SKRect(r.Left + p.Left * scale, r.Top + p.Top * scale, r.Right - p.Right * scale, r.Bottom - p.Bottom * scale);
    const gap = this.Spacing * scale;
    let cursor = this.Type === "Row" ? inner.Left : inner.Top;

    if (this.Type === "Wrap") {
      for (const s of this.wrapSlots) s.view.Arrange(SKRect.Create(inner.Left + s.x, inner.Top + s.y, s.w, s.h), s.view.WidthRequest, s.view.HeightRequest, scale);
      return;
    }
    if (this.Type === "Grid") {
      const g = this.GridStructure;
      if (!g) return;
      for (const v of this.views) {
        if (!v.IsVisible) continue;
        const c = g.GetCellBoundsFor(v, inner.Left / scale, inner.Top / scale);
        v.Arrange(SKRect.Create(c.Left * scale, c.Top * scale, c.Width * scale, c.Height * scale), v.WidthRequest, v.HeightRequest, scale);
      }
      return;
    }

    for (const v of this.views) {
      if (!v.IsVisible) continue;
      if (this.Type === "Column") {
        const h = v.MeasuredSize.Pixels.Height;
        v.Arrange(new SKRect(inner.Left, cursor, inner.Right, cursor + h), v.WidthRequest, v.HeightRequest, scale);
        cursor += h + gap;
      } else if (this.Type === "Row") {
        const w = v.MeasuredSize.Pixels.Width;
        v.Arrange(new SKRect(cursor, inner.Top, cursor + w, inner.Bottom), v.WidthRequest, v.HeightRequest, scale);
        cursor += w + gap;
      } else {
        v.Arrange(inner, v.WidthRequest, v.HeightRequest, scale);
      }
    }
  }

  protected override Paint(ctx: DrawingContext): void {
    if (this.IsTemplated) { this.PaintTemplated(ctx); return; }
    for (const v of this.views) v.Render(ctx);
  }

  /** Realizes, binds, arranges and draws only the cells intersecting the visible viewport (+ inflation). */
  private PaintTemplated(ctx: DrawingContext): void {
    const items = this.itemsSource!;
    const scale = this.RenderingScale;
    const r = this.DrawingRect;
    const p = this.Padding;
    const gap = this.Spacing * scale;
    const left = r.Left + p.Left * scale;
    const width = r.Width - p.HorizontalThickness * scale;
    const viewport = this.GetVisibleViewport();
    const inflate = this.VirtualisationInflated * scale;
    const visTop = viewport.Top - inflate, visBottom = viewport.Bottom + inflate;

    if (this.MeasureItemsStrategy === "MeasureVisible") { this.PaintMeasureVisible(ctx, visTop, visBottom, left, width); return; }

    let first = -1, last = -1;
    if (items.length > 0 && visBottom > visTop) {
      if (this.MeasureItemsStrategy === "MeasureAll") {
        let y = r.Top + p.Top * scale;
        for (let i = 0; i < items.length; i++) {
          const h = this.itemHeights[i] ?? 0;
          if (y + h >= visTop && y <= visBottom) { if (first < 0) first = i; last = i; } else if (first >= 0) break;
          y += h + gap;
        }
      } else {
        const stride = this.uniformHeight + gap;
        if (stride > 0) {
          first = Math.max(0, Math.floor((visTop - (r.Top + p.Top * scale)) / stride));
          last = Math.min(items.length - 1, Math.floor((visBottom - (r.Top + p.Top * scale)) / stride));
        }
      }
    }
    this.FirstVisibleIndex = first;
    this.LastVisibleIndex = last;
    if (first < 0) { this.ChildrenFactory.ReleaseOutside(1, 0); return; }

    this.ChildrenFactory.ReleaseOutside(first, last);
    for (let i = first; i <= last; i++) {
      const view = this.ChildrenFactory.GetOrCreateViewForIndex(i);
      if (!view) continue;
      const top = r.Top + this.GetItemOffsetPixels(i);
      const h = this.ItemHeightPixels(i);
      view.Measure(width, Infinity, scale); // recycled cells carry new content; height stays the structure's
      view.Arrange(SKRect.Create(left, top, width, h), view.WidthRequest, view.HeightRequest, scale);
      view.Render(ctx);
    }
  }

  /**
   * MeasureVisible frame: the first visible index comes from the exact prefix (binary search) or the estimate,
   * then cells are laid out contiguously with their REAL measured heights (measured on demand, kept for the prefix).
   */
  private PaintMeasureVisible(ctx: DrawingContext, visTop: number, visBottom: number, left: number, width: number): void {
    const n = this.mvHeights.length;
    const scale = this.RenderingScale, gap = this.Spacing * scale;
    const top0 = this.DrawingRect.Top + this.Padding.Top * scale;
    let first = -1, last = -1;
    if (n > 0 && visBottom > visTop) {
      const rel = visTop - top0;
      if (rel <= this.mvPrefix[this.mvMeasured]) {
        let lo = 0, hi = this.mvMeasured; // largest i with prefix[i] <= rel
        while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (this.mvPrefix[mid] <= rel) lo = mid; else hi = mid - 1; }
        first = Math.min(lo, n - 1);
      } else {
        const stride = this.MvStride(gap);
        first = Math.min(n - 1, this.mvMeasured + (stride > 0 ? Math.floor((rel - this.mvPrefix[this.mvMeasured]) / stride) : 0));
      }
      first = Math.max(0, first);
    }
    if (first < 0) { this.FirstVisibleIndex = this.LastVisibleIndex = -1; this.ChildrenFactory.ReleaseOutside(1, 0); return; }

    let y = top0 + this.GetItemOffsetPixels(first) - this.Padding.Top * scale;
    const drawn: { view: SkiaControl; top: number; h: number }[] = [];
    for (let i = first; i < n && y <= visBottom; i++) {
      const h = this.MvMeasure(i, width, scale, false);
      const view = this.ChildrenFactory.GetOrCreateViewForIndex(i);
      if (view && y + h >= visTop) drawn.push({ view, top: y, h });
      last = i;
      y += h + gap;
    }
    this.MvExtendPrefix(gap);
    this.FirstVisibleIndex = first;
    this.LastVisibleIndex = last;
    this.ChildrenFactory.ReleaseOutside(first, last);
    for (const d of drawn) {
      d.view.Measure(width, Infinity, scale);
      d.view.Arrange(SKRect.Create(left, d.top, width, d.h), d.view.WidthRequest, d.view.HeightRequest, scale);
      d.view.Render(ctx);
    }
    this.MvScheduleBackground();
  }
}

/** SkiaLayout Type=Column + HorizontalOptions=Fill (DrawnUi alias). */
export class SkiaStack extends SkiaLayout {
  constructor() { super(); this.Type = "Column"; this.HorizontalOptions = "Fill"; }
}

/** SkiaLayout Type=Row (DrawnUi alias). */
export class SkiaRow extends SkiaLayout {
  constructor() { super(); this.Type = "Row"; }
}

/** SkiaLayout Type=Absolute + HorizontalOptions=Fill (DrawnUi alias). */
export class SkiaLayer extends SkiaLayout {
  constructor() { super(); this.Type = "Absolute"; this.HorizontalOptions = "Fill"; }
}

/** SkiaLayout Type=Grid + HorizontalOptions=Fill (DrawnUi alias): MAUI Grid alternative. */
export class SkiaGrid extends SkiaLayout {
  constructor() { super(); this.Type = "Grid"; this.HorizontalOptions = "Fill"; }
}

/** SkiaLayout Type=Wrap + HorizontalOptions=Fill (DrawnUi alias): responsive flow of fixed-size children. */
export class SkiaWrap extends SkiaLayout {
  constructor() { super(); this.Type = "Wrap"; this.HorizontalOptions = "Fill"; }
}
