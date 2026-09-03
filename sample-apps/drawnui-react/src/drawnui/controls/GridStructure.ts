import type { SkiaControl } from "../core/SkiaControl";
import type { GridLength } from "../core/Types";
import type { SkiaLayout } from "./SkiaLayout";

type GridUnit = "Absolute" | "Auto" | "Star";

/** One row/column track (MAUI DefinitionInfo): its GridLength and the resolved size in points. */
export class DefinitionInfo {
  Unit: GridUnit;
  Value: number;
  Size: number;
  constructor(length: GridLength) {
    if (typeof length === "number") { this.Unit = "Absolute"; this.Value = length; this.Size = length; }
    else if (length === "Auto") { this.Unit = "Auto"; this.Value = 1; this.Size = 0; }
    else { this.Unit = "Star"; this.Value = length === "*" ? 1 : parseFloat(length) || 1; this.Size = 0; }
  }
  get IsAuto(): boolean { return this.Unit === "Auto"; }
  get IsStar(): boolean { return this.Unit === "Star"; }
  get IsAbsolute(): boolean { return this.Unit === "Absolute"; }
  /** Auto tracks grow to the largest measured child. */
  Update(size: number): void { if (size > this.Size) this.Size = size; }
}

/** Parses "*, 2*, Auto, 100" (or an array of the same values) into tracks. */
export function ParseDefinitions(defs: string | GridLength[] | undefined): DefinitionInfo[] {
  if (!defs) return [];
  const list: GridLength[] = typeof defs === "string"
    ? defs.split(",").map((s) => s.trim()).filter(Boolean).map((s) => (/^\d+(\.\d+)?$/.test(s) ? parseFloat(s) : (s as GridLength)))
    : defs;
  return list.map((l) => new DefinitionInfo(l));
}

const enum LengthType { None = 0, Absolute = 1, Star = 2, Auto = 4 }

/** A child placed in the grid with the combined track types it spans (MAUI Cell). */
class Cell {
  constructor(
    public ViewIndex: number, public Row: number, public Column: number, public RowSpan: number, public ColumnSpan: number,
    public ColumnGridLengthType: LengthType, public RowGridLengthType: LengthType,
  ) {}
  get IsColumnSpanAuto(): boolean { return (this.ColumnGridLengthType & LengthType.Auto) === LengthType.Auto; }
  get IsRowSpanAuto(): boolean { return (this.RowGridLengthType & LengthType.Auto) === LengthType.Auto; }
  get IsColumnSpanStar(): boolean { return (this.ColumnGridLengthType & LengthType.Star) === LengthType.Star; }
  get IsRowSpanStar(): boolean { return (this.RowGridLengthType & LengthType.Star) === LengthType.Star; }
  get IsAbsolute(): boolean { return this.ColumnGridLengthType === LengthType.Absolute && this.RowGridLengthType === LengthType.Absolute; }
  /** Any Absolute/Star part means a second measure once Auto tracks are known. */
  get NeedsKnownMeasurePass(): boolean { return ((this.ColumnGridLengthType | this.RowGridLengthType) ^ LengthType.Auto) > 0; }
}

interface GridSpan { Key: string; Start: number; Length: number; IsColumn: boolean; Requested: number }

/**
 * Port of DrawnUi SkiaGridStructure (itself adapted from the MAUI Grid manager). Works in POINTS: constraints come in
 * already reduced by the grid's padding, cells are returned relative to the padded box. Children are measured in
 * pixels through SkiaControl.Measure and their Units sizes drive the tracks.
 */
export class SkiaGridStructure {
  readonly Rows: DefinitionInfo[];
  readonly Columns: DefinitionInfo[];
  readonly ColumnSpacing: number;
  readonly RowSpacing: number;
  private readonly children: SkiaControl[];
  private readonly cells: Cell[] = [];
  private readonly spans = new Map<string, GridSpan>();
  private readonly explicitWidth: number;
  private readonly explicitHeight: number;

  constructor(private readonly grid: SkiaLayout, private readonly widthConstraint: number, private readonly heightConstraint: number, private readonly scale: number) {
    this.ColumnSpacing = grid.ColumnSpacing;
    this.RowSpacing = grid.RowSpacing;
    this.explicitWidth = grid.WidthRequest;
    this.explicitHeight = grid.HeightRequest;
    this.Rows = this.InitializeTracks(grid.RowDefinitions, grid.DefaultRowDefinition);
    this.Columns = this.InitializeTracks(grid.ColumnDefinitions, grid.DefaultColumnDefinition);
    this.children = grid.Views.filter((v) => v.IsVisible);
    this.InitializeCells();
    this.MeasureCells();
  }

  private InitializeTracks(defs: string | GridLength[] | undefined, fallback: GridLength): DefinitionInfo[] {
    const tracks = ParseDefinitions(defs);
    return tracks.length > 0 ? tracks : [new DefinitionInfo(fallback)];
  }

  private static CreateDefinitionIfMissing(array: DefinitionInfo[], requiredIndex: number, fallback: GridLength): void {
    while (array.length <= requiredIndex) array.push(new DefinitionInfo(fallback));
  }

  private static ToLengthType(d: DefinitionInfo): LengthType {
    return d.IsAbsolute ? LengthType.Absolute : d.IsStar ? LengthType.Star : LengthType.Auto;
  }

  private InitializeCells(): void {
    let maxRow = 0, maxColumn = 0;
    for (const child of this.children) {
      maxRow = Math.max(maxRow, child.Row + child.RowSpan - 1);
      maxColumn = Math.max(maxColumn, child.Column + child.ColumnSpan - 1);
    }
    SkiaGridStructure.CreateDefinitionIfMissing(this.Rows, maxRow, this.grid.DefaultRowDefinition);
    SkiaGridStructure.CreateDefinitionIfMissing(this.Columns, maxColumn, this.grid.DefaultColumnDefinition);
    for (let n = 0; n < this.children.length; n++) {
      const v = this.children[n];
      let ct = LengthType.None, rt = LengthType.None;
      for (let c = v.Column; c < v.Column + v.ColumnSpan; c++) ct |= SkiaGridStructure.ToLengthType(this.Columns[c]);
      for (let r = v.Row; r < v.Row + v.RowSpan; r++) rt |= SkiaGridStructure.ToLengthType(this.Rows[r]);
      this.cells.push(new Cell(n, v.Row, v.Column, v.RowSpan, v.ColumnSpan, ct, rt));
    }
  }

  private static Sum(defs: DefinitionInfo[], spacing: number): number {
    let sum = 0;
    for (let n = 0; n < defs.length; n++) { sum += defs[n].Size; if (n > 0) sum += spacing; }
    return sum;
  }
  GridWidth(): number { return SkiaGridStructure.Sum(this.Columns, this.ColumnSpacing); }
  GridHeight(): number { return SkiaGridStructure.Sum(this.Rows, this.RowSpacing); }

  LeftEdgeOfColumn(column: number): number { let x = 0; for (let n = 0; n < column; n++) x += this.Columns[n].Size + this.ColumnSpacing; return x; }
  TopEdgeOfRow(row: number): number { let y = 0; for (let n = 0; n < row; n++) y += this.Rows[n].Size + this.RowSpacing; return y; }

  /** Cell rectangle in points relative to the padded box (+ offsets). */
  GetCellBoundsFor(view: SkiaControl, xOffset = 0, yOffset = 0): { Left: number; Top: number; Width: number; Height: number } {
    const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));
    const firstColumn = clamp(view.Column, 0, this.Columns.length - 1);
    const columnSpan = clamp(view.ColumnSpan, 1, this.Columns.length - firstColumn);
    const firstRow = clamp(view.Row, 0, this.Rows.length - 1);
    const rowSpan = clamp(view.RowSpan, 1, this.Rows.length - firstRow);
    let width = 0, height = 0;
    for (let n = firstColumn; n < firstColumn + columnSpan; n++) width += this.Columns[n].Size;
    for (let n = firstRow; n < firstRow + rowSpan; n++) height += this.Rows[n].Size;
    width += (columnSpan - 1) * this.ColumnSpacing;
    height += (rowSpan - 1) * this.RowSpacing;
    return { Left: this.LeftEdgeOfColumn(firstColumn) + xOffset, Top: this.TopEdgeOfRow(firstRow) + yOffset, Width: width, Height: height };
  }

  // ---- measure passes (same order as C#) ----
  private MeasureCells(): void {
    for (const cell of this.cells) this.MeasureChild(cell);
    this.ResolveStarColumns(this.widthConstraint);
    this.ResolveStarRows(this.heightConstraint);
    this.MeasureKnownCells();
    this.ResolveSpans();
    this.ApplyMinimumDimensionsFromFillChildren();
    this.CompressStarMeasurements();
  }

  /** Points -> pixels for Measure; negative/NaN = unconstrained. */
  private MeasurePx(child: SkiaControl, wPts: number, hPts: number) {
    const w = wPts < 0 ? Infinity : wPts * this.scale;
    const h = hPts < 0 ? Infinity : Math.round(hPts * this.scale);
    return child.Measure(w, h, this.scale).Units;
  }

  private MeasureChild(cell: Cell): void {
    if (cell.IsAbsolute) return;
    const child = this.children[cell.ViewIndex];
    let availableWidth = this.AvailableWidth(cell);
    let availableHeight = this.AvailableHeight(cell);
    const finiteW = availableWidth, finiteH = availableHeight;
    // a Fill child in an Auto track is measured unconstrained (content-sized track, MAUI desired-size semantics)
    if (cell.IsColumnSpanAuto && child.HorizontalOptions === "Fill") availableWidth = Infinity;
    if (cell.IsRowSpanAuto && child.VerticalOptions === "Fill") availableHeight = Infinity;
    const m = this.MeasurePx(child, availableWidth, availableHeight);
    let mw = m.Width, mh = m.Height;
    // ...but clamped to the finite grid so a wrapping label / scroll cannot inflate the track
    if (availableWidth === Infinity && isFinite(finiteW) && finiteW >= 0 && mw > finiteW) mw = finiteW;
    if (availableHeight === Infinity && isFinite(finiteH) && finiteH >= 0 && mh > finiteH) mh = finiteH;
    if (cell.IsColumnSpanAuto) {
      if (cell.ColumnSpan === 1) this.Columns[cell.Column].Update(mw);
      else this.TrackSpan({ Key: `c${cell.Column}:${cell.ColumnSpan}`, Start: cell.Column, Length: cell.ColumnSpan, IsColumn: true, Requested: mw });
    }
    if (cell.IsRowSpanAuto) {
      if (cell.RowSpan === 1) this.Rows[cell.Row].Update(mh);
      else this.TrackSpan({ Key: `r${cell.Row}:${cell.RowSpan}`, Start: cell.Row, Length: cell.RowSpan, IsColumn: false, Requested: mh });
    }
  }

  private TrackSpan(span: GridSpan): void {
    const other = this.spans.get(span.Key);
    if (!other || span.Requested > other.Requested) this.spans.set(span.Key, span);
  }

  private ResolveSpans(): void {
    for (const span of this.spans.values()) {
      if (span.IsColumn) SkiaGridStructure.ResolveSpan(this.Columns, span.Start, span.Length, this.ColumnSpacing, span.Requested);
      else SkiaGridStructure.ResolveSpan(this.Rows, span.Start, span.Length, this.RowSpacing, span.Requested);
    }
  }

  private static ResolveSpan(defs: DefinitionInfo[], start: number, length: number, spacing: number, requested: number): void {
    let current = 0;
    const end = start + length;
    for (let n = start; n < end; n++) { current += defs[n].Size; if (n > start) current += spacing; }
    if (requested <= current) return;
    const required = requested - current;
    let autoCount = 0;
    for (let n = start; n < end; n++) { if (defs[n].IsAuto) autoCount++; else if (defs[n].IsStar) return; }
    if (autoCount === 0) return;
    const distribution = required / autoCount;
    for (let n = start; n < end; n++) if (defs[n].IsAuto) defs[n].Size += distribution;
  }

  private ApplyMinimumDimensionsFromFillChildren(): void {
    for (const cell of this.cells) {
      const child = this.children[cell.ViewIndex];
      if (child.HorizontalOptions === "Fill" && child.MinimumWidthRequest >= 0) {
        const min = (child.MinimumWidthRequest + child.Margin.HorizontalThickness) / cell.ColumnSpan;
        for (let c = cell.Column; c < cell.Column + cell.ColumnSpan; c++) if (this.Columns[c].Size < min) this.Columns[c].Size = min;
      }
      if (child.VerticalOptions === "Fill" && child.MinimumHeightRequest >= 0) {
        const min = (child.MinimumHeightRequest + child.Margin.VerticalThickness) / cell.RowSpan;
        for (let r = cell.Row; r < cell.Row + cell.RowSpan; r++) if (this.Rows[r].Size < min) this.Rows[r].Size = min;
      }
    }
  }

  private ResolveStars(defs: DefinitionInfo[], availableSpace: number, cellCheck: (c: Cell) => boolean, dimension: (c: SkiaControl) => number): void {
    let starCount = 0;
    for (const d of defs) if (d.IsStar) starCount += d.Value;
    if (starCount === 0) return;
    let starSize = 0;
    if (!isFinite(availableSpace)) {
      // unbounded axis: stars take the largest star cell's desired size
      for (const cell of this.cells) if (cellCheck(cell)) starSize = Math.max(starSize, dimension(this.children[cell.ViewIndex]));
    } else {
      starSize = availableSpace / starCount;
    }
    for (const d of defs) if (d.IsStar) d.Size = starSize * d.Value;
  }
  private ResolveStarColumns(widthConstraint: number): void {
    this.ResolveStars(this.Columns, widthConstraint - this.GridWidth(), (c) => c.IsColumnSpanStar, (v) => v.MeasuredSize.Units.Width);
  }
  private ResolveStarRows(heightConstraint: number): void {
    this.ResolveStars(this.Rows, heightConstraint - this.GridHeight(), (c) => c.IsRowSpanStar, (v) => v.MeasuredSize.Units.Height);
  }

  private MeasureKnownCells(): void {
    for (const cell of this.cells) {
      if (!cell.NeedsKnownMeasurePass) continue;
      let width = 0, height = 0;
      for (let n = cell.Row; n < cell.Row + cell.RowSpan; n++) height += this.Rows[n].Size;
      for (let n = cell.Column; n < cell.Column + cell.ColumnSpan; n++) width += this.Columns[n].Size;
      if (width === 0 || height === 0) continue;
      const child = this.children[cell.ViewIndex];
      const rect = this.GetCellBoundsFor(child);
      this.MeasurePx(child, rect.Width, rect.Height);
      if (cell.IsColumnSpanStar && cell.ColumnSpan > 1) this.TrackSpan({ Key: `c${cell.Column}:${cell.ColumnSpan}`, Start: cell.Column, Length: cell.ColumnSpan, IsColumn: true, Requested: rect.Width });
      if (cell.IsRowSpanStar && cell.RowSpan > 1) this.TrackSpan({ Key: `r${cell.Row}:${cell.RowSpan}`, Start: cell.Row, Length: cell.RowSpan, IsColumn: false, Requested: rect.Height });
    }
  }

  private AvailableWidth(cell: Cell): number {
    let w = 0, absolute = true;
    for (let c = cell.Column; c < cell.Column + cell.ColumnSpan; c++) { w += this.Columns[c].Size; if (!this.Columns[c].IsAbsolute) absolute = false; }
    w += (cell.ColumnSpan - 1) * this.ColumnSpacing;
    if (absolute) return w;
    return this.widthConstraint - this.GridWidth() + w;
  }
  private AvailableHeight(cell: Cell): number {
    let h = 0, absolute = true;
    for (let r = cell.Row; r < cell.Row + cell.RowSpan; r++) { h += this.Rows[r].Size; if (!this.Rows[r].IsAbsolute) absolute = false; }
    h += (cell.RowSpan - 1) * this.RowSpacing;
    if (absolute) return h;
    return this.heightConstraint - this.GridHeight() + h;
  }

  /** Re-resolves stars against the final size when the grid has an explicit size request (C# DecompressStars). */
  DecompressStars(targetWidth: number, targetHeight: number): void {
    if (this.explicitHeight >= 0) { SkiaGridStructure.ZeroOutStars(this.Rows); this.ResolveStarRows(targetHeight); }
    if (this.explicitWidth >= 0) { SkiaGridStructure.ZeroOutStars(this.Columns); this.ResolveStarColumns(targetWidth); }
  }

  private CompressStarMeasurements(): void {
    this.CompressStars(this.Rows, (c) => c.IsRowSpanStar, (c) => c.Row, (c) => c.RowSpan, this.heightConstraint, (v) => v.MeasuredSize.Units.Height);
    this.CompressStars(this.Columns, (c) => c.IsColumnSpanStar, (c) => c.Column, (c) => c.ColumnSpan, this.widthConstraint, (v) => v.MeasuredSize.Units.Width);
  }

  /** Stars shrink to what their cells actually need (so an auto-sized grid does not take the whole constraint). */
  private CompressStars(defs: DefinitionInfo[], isStar: (c: Cell) => boolean, start: (c: Cell) => number, span: (c: Cell) => number, constraint: number, dimension: (v: SkiaControl) => number): void {
    const copy = defs.map((d) => { const c = new DefinitionInfo(d.Unit === "Absolute" ? d.Value : d.Unit === "Auto" ? "Auto" : `${d.Value}*`); c.Size = d.IsStar ? 0 : d.Size; return c; });
    for (const cell of this.cells) {
      if (!isStar(cell)) continue;
      const needed = Math.min(constraint, dimension(this.children[cell.ViewIndex]));
      SkiaGridStructure.ExpandStarsInSpan(needed, defs, copy, start(cell), start(cell) + span(cell));
    }
    for (let n = 0; n < copy.length; n++) if (copy[n].IsStar) defs[n].Size = copy[n].Size;
  }

  private static ExpandStarsInSpan(spaceNeeded: number, original: DefinitionInfo[], updated: DefinitionInfo[], start: number, end: number): void {
    for (let n = start; n < end; n++) if (original[n].IsAbsolute || original[n].IsAuto) spaceNeeded -= original[n].Size;
    let spaceAvailable = 0, starCount = 0;
    for (let n = start; n < end; n++) if (updated[n].IsStar) { starCount++; spaceAvailable += updated[n].Size; }
    if (starCount > 0 && spaceAvailable < spaceNeeded) {
      const toAdd = (spaceNeeded - spaceAvailable) / starCount;
      for (let n = start; n < end; n++) if (updated[n].IsStar) updated[n].Size += toAdd;
    }
  }

  private static ZeroOutStars(defs: DefinitionInfo[]): void { for (const d of defs) if (d.IsStar) d.Size = 0; }

  /** Tracks are final only now: measure every child at its final cell so its internal layout matches the arranged box. */
  RemeasureChildrenAtFinalCells(): void {
    for (const cell of this.cells) {
      const child = this.children[cell.ViewIndex];
      const rect = this.GetCellBoundsFor(child);
      if (rect.Width <= 0 || rect.Height <= 0) continue;
      const m = this.MeasurePx(child, rect.Width, rect.Height);
      if (cell.ColumnSpan === 1 && this.Columns[cell.Column].IsAuto && m.Width > this.Columns[cell.Column].Size) this.Columns[cell.Column].Update(m.Width);
      if (cell.RowSpan === 1 && this.Rows[cell.Row].IsAuto && m.Height > this.Rows[cell.Row].Size) this.Rows[cell.Row].Update(m.Height);
    }
  }
}
