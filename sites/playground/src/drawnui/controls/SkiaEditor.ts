import type { DrawingContext, SkiaControl } from "../core/SkiaControl";
import { type GestureEventProcessingInfo, SKPoint, type SkiaGesturesParameters } from "../core/Gestures";
import { Super } from "../core/Super";
import { SkiaValueAnimator } from "../core/Animators";
import { type PrebuiltControlStyle, ResolveControlStyle } from "../core/ControlStyle";
import { type InputKey, KeyboardManager } from "../core/KeyboardManager";
import { TextInputProxy } from "../core/TextInputProxy";
import { type Color, Colors, type DrawTextAlignment, type ReturnType, ScaledSize, type SkiaEditorKeyboard, type SkiaGradient, SKRect, type TextAlignment, Thickness } from "../core/Types";
import { type GlyphBox, SkiaLabel } from "./SkiaLabel";
import { SkiaShape } from "./SkiaShape";

const ParagraphBreak = "\n";

/**
 * Mirrors DrawnUi SkiaEditor: a drawn text input. The text is a SkiaLabel inside a shape with padding, a
 * placeholder label, a blinking caret and a selection overlay painted by the editor; single line
 * (`MaxLines` 1, horizontal scrolling to the caret) or multiline (`MaxLines` != 1, `AutoHeight`). Keys come from
 * the window-level `KeyboardManager` like the DrawnUi.Blazor / Wasm heads (no DOM text input: no IME, no mobile
 * soft keyboard) and are applied by the same stub editing methods (`StubTypeText`, `StubBackspace`, arrows,
 * Home / End, Ctrl+A / C / X / V). Taps place the caret, drags select, a long press selects the word, Enter submits
 * (single line, or multiline with `ReturnType` Send).
 */
export class SkiaEditor extends SkiaShape {
  /** The editor currently holding the keyboard, if any. */
  static Focused?: SkiaEditor;
  static DefaultAccessibilityRole?: string = "textbox";

  TextChanged?: (sender: SkiaEditor, text: string) => void;
  FocusChanged?: (sender: SkiaEditor, focused: boolean) => void;
  CursorMoved?: (sender: SkiaEditor) => void;
  TextSubmitted?: (sender: SkiaEditor, text: string) => void;

  // ---- text / look (same names as C#) ----
  private text = "";
  private placeholderText = "";
  PlaceholderColor: Color = "#9AA0A6";
  PlaceholderHorizontalAlignment: DrawTextAlignment = "Start";
  private fontSize = 12;
  private fontFamily = "";
  private fontWeight = 0;
  private textColor: Color = Colors.Black;
  TextGradient?: SkiaGradient;
  private lineHeight = 1;
  private horizontalTextAlignment: DrawTextAlignment = "Start";
  private verticalTextAlignment: TextAlignment = "Start";
  private maxLines = 1;
  AutoHeight = false;
  private isPassword = false;
  /** Enter on a multiline editor: Send submits, anything else inserts a line break. */
  ReturnType: ReturnType = "Done";
  KeyboardType: SkiaEditorKeyboard = "Default";
  private fontFamilyFallback = "";
  /** Fallback font aliases for glyphs the main font lacks (symbols, emoji), forwarded to the text and placeholder labels. */
  get FontFamilyFallback(): string { return this.fontFamilyFallback; }
  set FontFamilyFallback(v: string) { if (this.fontFamilyFallback !== v) { this.fontFamilyFallback = v ?? ""; this.UpdateLabel(); } }
  IsSpellCheckEnabled = true;
  /** Accepted for parity: the React label always renders unicode; markdown formatting while editing is not ported. */
  UseMarkdown = false;
  UseUnicode = true;
  CursorColor: Color = Colors.Black;
  CanShowCursor = true;
  SelectionColor: Color = "#5590CFFE";
  private cursorPosition = 0;
  private selectionLength = 0;
  private isFocused = false;
  private controlStyle: PrebuiltControlStyle = "Unset";

  // ---- children ----
  readonly Label = new SkiaLabel();
  private readonly placeholder = new SkiaLabel();
  private scrollX = 0;
  private scrollY = 0;
  private blink = new SkiaValueAnimator(this);
  private caretVisible = true;
  private subscribed = false;
  private stubSelectionStop = -1;
  private selectionMovingEdge = -1;
  private readonly onKeyDown = (key: InputKey, e: KeyboardEvent) => this.OnKeyDown(key, e);
  private readonly onKeyChar = (ch: string, e: KeyboardEvent) => { e.preventDefault(); this.StubTypeText(ch); };
  private selectionAnchor = -1;
  private dragSelecting = false;
  private styleApplied: Record<string, unknown> = {};
  private layoutVersion = -1;
  private boxes: GlyphBox[] = [];

  constructor() {
    super();
    this.Type = "Rectangle";
    this.UseCache = "Operations";
    this.HorizontalOptions = "Fill";
    this.Padding = new Thickness(12, 8);
    this.Label.KeepSpacesOnLineBreaks = true;
    this.Label.NeedsGlyphPositions = true;
    this.Label.LineBreakMode = "NoWrap";
    this.Label.UseCache = "None";
    this.placeholder.UseCache = "None";
    this.placeholder.IsVisible = false;
    this.AddSubView(this.placeholder);
    this.AddSubView(this.Label);
    this.blink.mMinValue = 0; this.blink.mMaxValue = 1; this.blink.Speed = 1000; this.blink.Repeat = -1;
    this.blink.OnUpdated = (v) => { const visible = v < 0.5; if (visible !== this.caretVisible) { this.caretVisible = visible; this.InvalidateCache(); this.RepaintComposition(); } };
    this.ApplyControlStyleVisuals();
    this.UpdateLabel();
  }

  // ---- properties ----
  get Text(): string { return this.text; }
  set Text(v: string) {
    const normalized = SkiaEditor.NormalizeLineBreaks(v ?? "");
    const value = this.IsMultiline ? normalized : normalized.replace(/\n/g, " ");
    if (this.text === value) return;
    this.text = value;
    if (this.cursorPosition > value.length) this.cursorPosition = value.length;
    if (this.cursorPosition + this.selectionLength > value.length) this.selectionLength = Math.max(0, value.length - this.cursorPosition);
    this.UpdateLabel();
    TextInputProxy.Sync(this);
    this.TextChanged?.(this, value);
  }
  get PlaceholderText(): string { return this.placeholderText; }
  set PlaceholderText(v: string) { if (this.placeholderText !== v) { this.placeholderText = v ?? ""; this.UpdateLabel(); } }
  get FontSize(): number { return this.fontSize; }
  set FontSize(v: number) { if (this.fontSize !== v) { this.fontSize = v; this.UpdateLabel(); } }
  get FontFamily(): string { return this.fontFamily; }
  set FontFamily(v: string) { if (this.fontFamily !== v) { this.fontFamily = v ?? ""; this.UpdateLabel(); } }
  get FontWeight(): number { return this.fontWeight; }
  set FontWeight(v: number) { if (this.fontWeight !== v) { this.fontWeight = v; this.UpdateLabel(); } }
  get TextColor(): Color { return this.textColor; }
  set TextColor(v: Color) { if (this.textColor !== v) { this.textColor = v; this.UpdateLabel(); } }
  get LineHeight(): number { return this.lineHeight; }
  set LineHeight(v: number) { if (this.lineHeight !== v) { this.lineHeight = v; this.UpdateLabel(); } }
  get HorizontalTextAlignment(): DrawTextAlignment { return this.horizontalTextAlignment; }
  set HorizontalTextAlignment(v: DrawTextAlignment) { if (this.horizontalTextAlignment !== v) { this.horizontalTextAlignment = v; this.UpdateLabel(); } }
  get VerticalTextAlignment(): TextAlignment { return this.verticalTextAlignment; }
  set VerticalTextAlignment(v: TextAlignment) { if (this.verticalTextAlignment !== v) { this.verticalTextAlignment = v; this.UpdateLabel(); } }
  /** 1 = single line (default); anything else = multiline, -1 unbounded. */
  get MaxLines(): number { return this.maxLines; }
  set MaxLines(v: number) { if (this.maxLines !== v) { this.maxLines = v; this.UpdateLabel(); } }
  get IsPassword(): boolean { return this.isPassword; }
  set IsPassword(v: boolean) { if (this.isPassword !== v) { this.isPassword = v; this.UpdateLabel(); } }
  get IsMultiline(): boolean { return this.maxLines !== 1; }
  get HasSelection(): boolean { return this.selectionLength > 0; }
  get ShouldSubmitOnEnter(): boolean { return this.IsMultiline && this.ReturnType === "Send"; }

  /** Caret index (UTF-16 units) = the left edge of the selection when one exists. */
  get CursorPosition(): number { return this.cursorPosition; }
  set CursorPosition(v: number) {
    const p = Math.max(0, Math.min(this.text.length, v | 0));
    if (p === this.cursorPosition) return;
    this.cursorPosition = p;
    this.OnCursorMoved();
  }
  get SelectionLength(): number { return this.selectionLength; }
  set SelectionLength(v: number) {
    const l = Math.max(0, Math.min(this.text.length - this.cursorPosition, v | 0));
    if (l === this.selectionLength) return;
    this.selectionLength = l;
    this.OnCursorMoved();
  }
  private OnCursorMoved(): void {
    this.ScrollToCaret();
    this.caretVisible = true;
    TextInputProxy.Sync(this);
    this.CursorMoved?.(this);
    this.InvalidateCache(); this.RepaintComposition();
  }

  get IsFocused(): boolean { return this.isFocused; }
  set IsFocused(v: boolean) {
    if (this.isFocused === v) return;
    this.isFocused = v;
    if (v) {
      if (SkiaEditor.Focused && SkiaEditor.Focused !== this) SkiaEditor.Focused.IsFocused = false;
      SkiaEditor.Focused = this;
      this.SetFocusNative(true);
      this.caretVisible = true;
      this.blink.Start();
    } else {
      if (SkiaEditor.Focused === this) SkiaEditor.Focused = undefined;
      this.selectionLength = 0;
      this.dragSelecting = false;
      this.stubSelectionStop = -1; this.selectionMovingEdge = -1;
      this.blink.Stop();
      this.SetFocusNative(false);
    }
    this.FocusChanged?.(this, v);
    this.NotifyAccessibility();
    this.InvalidateCache(); this.RepaintComposition();
  }

  get ControlStyle(): PrebuiltControlStyle { return this.controlStyle; }
  set ControlStyle(v: PrebuiltControlStyle) { if (this.controlStyle !== v) { this.controlStyle = v; this.ApplyControlStyleVisuals(); this.Update(); } }

  /** C# ApplyControlStyleVisuals: background, corners, border, cursor per style, applied only over values the app has not changed. */
  private ApplyControlStyleVisuals(): void {
    const style = ResolveControlStyle(this.controlStyle);
    const look = style === "Cupertino" ? { bg: "#FFFFFF", corner: 10, stroke: "#C7C7CC", strokeW: 1, cursor: "#007AFF" }
      : style === "Material" ? { bg: "#EFEBF4", corner: 4, stroke: Colors.Transparent, strokeW: 0, cursor: "#2196F3" }
      : style === "Material3" ? { bg: "#E6E0E9", corner: 4, stroke: Colors.Transparent, strokeW: 0, cursor: "#6750A4" }
      : style === "Windows" ? { bg: "#FFFFFF", corner: 2, stroke: "#8A8A8A", strokeW: 1, cursor: "#0078D4" }
      : { bg: "#F2F3F5", corner: 8, stroke: Colors.Transparent, strokeW: 0, cursor: "#DC143C" };
    const apply = <K extends string>(key: K, current: unknown, next: unknown, set: () => void) => {
      if (!(key in this.styleApplied) || this.styleApplied[key] === current) { set(); this.styleApplied[key] = next; }
    };
    apply("bg", this.BackgroundColor, look.bg, () => { this.BackgroundColor = look.bg; });
    apply("corner", this.CornerRadius, look.corner, () => { this.CornerRadius = look.corner; });
    apply("stroke", this.StrokeColor, look.stroke, () => { this.StrokeColor = look.stroke; });
    apply("strokeW", this.StrokeWidth, look.strokeW, () => { this.StrokeWidth = look.strokeW; });
    apply("cursor", this.CursorColor, look.cursor, () => { this.CursorColor = look.cursor; });
  }

  // ---- label ----
  private static NormalizeLineBreaks(v: string): string { return v.replace(/\r\n/g, "\n").replace(/\r/g, "\n").replace(/\u2029/g, ParagraphBreak); }

  /** C# UpdateLabel: pushes the editor's text props into the content and placeholder labels. */
  UpdateLabel(): void {
    let display = this.isPassword && this.text ? "•".repeat(this.text.length) : this.text;
    if (display.length === 0 || (this.IsMultiline && display.endsWith(ParagraphBreak))) display += "\u200B"; // C#: keeps the empty (last) line for the caret
    const l = this.Label;
    l.Text = display; l.FontFamily = this.fontFamily; l.FontFamilyFallback = this.fontFamilyFallback; l.FontSize = this.fontSize; l.TextColor = this.textColor; l.FontWeight = this.fontWeight;
    l.FillGradient = this.TextGradient; l.HorizontalTextAlignment = this.horizontalTextAlignment; l.VerticalTextAlignment = this.verticalTextAlignment; l.LineHeight = this.lineHeight;
    l.MaxLines = -1; l.LineBreakMode = this.IsMultiline ? "WordWrap" : "NoWrap"; l.HorizontalOptions = this.IsMultiline ? "Fill" : "Start";
    const p = this.placeholder;
    p.Text = this.placeholderText; p.TextColor = this.PlaceholderColor; p.HorizontalTextAlignment = this.PlaceholderHorizontalAlignment; p.FontFamily = this.fontFamily; p.FontFamilyFallback = this.fontFamilyFallback; p.FontSize = this.fontSize; p.FontWeight = this.fontWeight; p.LineHeight = this.lineHeight;
    p.MaxLines = this.IsMultiline ? -1 : 1; p.HorizontalOptions = "Fill";
    p.IsVisible = this.text.length === 0 && this.placeholderText.length > 0;
    this.Update();
  }

  /** Visual height of one line in points (C# GetSingleLineHeightPts). */
  GetSingleLineHeightPts(): number {
    const scale = this.RenderingScale || 1;
    const px = this.Label.MeasuredLineHeight || this.placeholder.MeasuredLineHeight;
    if (px > 0) return Math.ceil(px / scale);
    return Math.ceil((this.fontSize > 0 ? this.fontSize : 20) * 1.2 * Math.max(this.lineHeight, 1));
  }

  // ---- layout ----
  private Inner(rect: SKRect, scale: number): SKRect {
    const p = this.Padding;
    return new SKRect(rect.Left + p.Left * scale, rect.Top + p.Top * scale, rect.Right - p.Right * scale, rect.Bottom - p.Bottom * scale);
  }

  protected override MeasureAbsolute(w: number, h: number, scale: number): ScaledSize {
    const px = this.Padding.HorizontalThickness * scale, py = this.Padding.VerticalThickness * scale;
    const innerW = isFinite(w) ? Math.max(0, w - px) : Infinity;
    const labelSize = this.Label.Measure(this.IsMultiline ? innerW : Infinity, Infinity, scale);
    if (this.placeholder.IsVisible) this.placeholder.Measure(innerW, Infinity, scale);
    const lineH = this.Label.MeasuredLineHeight || this.placeholder.MeasuredLineHeight || this.GetSingleLineHeightPts() * scale;
    let lines = this.maxLines > 0 ? this.maxLines : 1;
    if (this.AutoHeight && this.IsMultiline) { const actual = Math.max(1, this.Label.LinesCount); lines = this.maxLines > 0 ? Math.min(actual, this.maxLines) : actual; }
    const contentH = lineH * lines;
    const width = isFinite(w) ? w : labelSize.Pixels.Width + px;
    return ScaledSize.FromPixels(width, Math.ceil(contentH) + py, scale);
  }

  protected override OnLayoutChanged(): void {
    const scale = this.RenderingScale;
    const inner = this.Inner(this.DrawingRect, scale);
    this.ClampScroll(inner);
    const l = this.Label.MeasuredSize.Pixels;
    const lw = this.IsMultiline ? inner.Width : Math.max(l.Width, inner.Width);
    const lh = Math.max(l.Height, inner.Height);
    this.Label.Arrange(SKRect.Create(inner.Left - this.scrollX, inner.Top - this.scrollY, lw, lh), -1, -1, scale);
    if (this.placeholder.IsVisible) this.placeholder.Arrange(inner, -1, -1, scale);
  }

  private ClampScroll(inner: SKRect): void {
    const l = this.Label.MeasuredSize.Pixels;
    this.scrollX = Math.max(0, Math.min(this.scrollX, l.Width - inner.Width));
    this.scrollY = Math.max(0, Math.min(this.scrollY, l.Height - inner.Height));
  }

  /** Keeps the caret inside the padded box by scrolling the label (C#: SkiaScroll under the label). */
  private ScrollToCaret(): void {
    const rect = this.CaretRectLabelPx();
    if (!rect) return;
    const inner = this.Inner(this.DrawingRect, this.RenderingScale);
    if (inner.Width <= 0) return;
    const caretW = 2 * this.RenderingScale;
    if (rect.Left - this.scrollX < 0) this.scrollX = rect.Left;
    else if (rect.Left + caretW - this.scrollX > inner.Width) this.scrollX = rect.Left + caretW - inner.Width;
    if (rect.Top - this.scrollY < 0) this.scrollY = rect.Top;
    else if (rect.Bottom - this.scrollY > inner.Height) this.scrollY = rect.Bottom - inner.Height;
    this.ClampScroll(inner);
    this.OnLayoutChanged();
  }

  // ---- caret / selection geometry (pixels relative to the label's DrawingRect) ----
  private Boxes(): GlyphBox[] {
    if (this.layoutVersion !== this.Label.LayoutVersion) { this.boxes = this.Label.GetGlyphBoxes(); this.layoutVersion = this.Label.LayoutVersion; }
    return this.boxes;
  }

  /** Rect of the caret before character `index` (label pixels), or undefined before the first layout. */
  private CaretRectLabelPx(index = this.cursorPosition + this.selectionLength): SKRect | undefined {
    const boxes = this.Boxes();
    const lines = this.Label.GetLineBoxes();
    if (lines.length === 0) return undefined;
    const caretW = 2 * this.RenderingScale;
    const at = boxes.find((b) => b.Index === index);
    if (at) return SKRect.Create(at.Left, at.Top, caretW, at.Height);
    let prev: GlyphBox | undefined;
    for (const b of boxes) { if (b.Index < index) prev = b; else break; }
    if (prev) return SKRect.Create(prev.Left + prev.Width, prev.Top, caretW, prev.Height);
    const first = lines[0];
    return SKRect.Create(first.Left, first.Top, caretW, first.Height);
  }

  /** C# GetCursorPosition: the text index nearest to a point in label pixels. */
  private HitTestIndex(x: number, y: number): number {
    const lines = this.Label.GetLineBoxes();
    if (lines.length === 0) return this.text.length;
    let line = lines[lines.length - 1];
    for (const l of lines) if (y < l.Top + l.Height) { line = l; break; }
    const boxes = this.Boxes().filter((b) => b.Line === line.Line);
    if (boxes.length === 0) return Math.min(line.Start, this.text.length);
    if (x <= boxes[0].Left) return boxes[0].Index;
    for (const b of boxes) if (x < b.Left + b.Width) return x < b.Left + b.Width / 2 ? b.Index : b.Index + b.Length;
    const last = boxes[boxes.length - 1];
    return Math.min(last.Index + last.Length, this.text.length);
  }

  private LabelPoint(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo, starting = false): SKPoint {
    const loc = starting ? args.Event.StartingLocation : apply.MappedLocation;
    const r = this.Label.DrawingRect;
    return new SKPoint(loc.X + apply.ChildOffset.X - r.Left, loc.Y + apply.ChildOffset.Y - r.Top);
  }

  // ---- painting: selection under the text, caret above ----
  protected override Paint(ctx: DrawingContext): void {
    const CK = Super.CK;
    const canvas = ctx.Context.Canvas;
    const inner = this.Inner(ctx.Destination, ctx.Scale);
    const clip = CK.LTRBRect(inner.Left, inner.Top, inner.Right, inner.Bottom);
    const lr = this.Label.DrawingRect;
    if (this.isFocused && this.selectionLength > 0) {
      const saved = canvas.save();
      canvas.clipRect(clip, CK.ClipOp.Intersect, true);
      const paint = new CK.Paint(); paint.setColor(Super.ParseColor(this.SelectionColor));
      const end = this.cursorPosition + this.selectionLength;
      for (const b of this.Boxes()) if (b.Index >= this.cursorPosition && b.Index < end) canvas.drawRect(CK.LTRBRect(lr.Left + b.Left, lr.Top + b.Top, lr.Left + b.Left + Math.max(b.Width, 2), lr.Top + b.Top + b.Height), paint);
      paint.delete();
      canvas.restoreToCount(saved);
    }
    super.Paint(ctx);
    if (this.isFocused && this.CanShowCursor && this.caretVisible) {
      const rect = this.CaretRectLabelPx();
      if (rect) {
        const saved = canvas.save();
        canvas.clipRect(clip, CK.ClipOp.Intersect, true);
        const paint = new CK.Paint(); paint.setColor(Super.ParseColor(this.CursorColor));
        canvas.drawRect(CK.LTRBRect(lr.Left + rect.Left, lr.Top + rect.Top, lr.Left + rect.Right, lr.Top + rect.Bottom), paint);
        paint.delete();
        canvas.restoreToCount(saved);
      }
    }
  }

  // ---- gestures (C# ProcessGestures: tap places the caret, drag selects, long press selects the word) ----
  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    switch (args.Type) {
      case "Down": {
        const p = this.LabelPoint(args, apply);
        const pos = this.HitTestIndex(p.X, p.Y);
        this.dragSelecting = true;
        if (KeyboardManager.IsShiftPressed && this.isFocused) {
          // C#: shift + tap extends the selection from the far edge of the current one
          const left = this.cursorPosition, right = this.cursorPosition + this.selectionLength;
          const anchor = this.selectionLength > 0 ? (pos - left <= right - pos ? right : left) : this.cursorPosition;
          this.selectionAnchor = anchor;
          this.cursorPosition = Math.min(anchor, pos); this.selectionLength = Math.abs(anchor - pos);
          this.stubSelectionStop = anchor; this.selectionMovingEdge = pos;
        } else {
          this.selectionAnchor = pos;
          this.selectionLength = 0;
          this.cursorPosition = pos;
          this.stubSelectionStop = -1; this.selectionMovingEdge = -1;
        }
        if (!this.isFocused) this.IsFocused = true;
        this.OnCursorMoved();
        return this;
      }
      case "Panning": {
        if (!this.dragSelecting) return this;
        const p = this.LabelPoint(args, apply);
        const pos = this.HitTestIndex(p.X, p.Y);
        const start = Math.min(pos, this.selectionAnchor), end = Math.max(pos, this.selectionAnchor);
        this.cursorPosition = start; this.selectionLength = end - start;
        this.OnCursorMoved();
        return this;
      }
      case "LongPressing": {
        const p = this.LabelPoint(args, apply, true);
        this.SelectWord(this.HitTestIndex(p.X, p.Y));
        this.dragSelecting = true;
        this.selectionAnchor = this.cursorPosition;
        if (!this.isFocused) this.IsFocused = true;
        return this;
      }
      case "Up": this.dragSelecting = false; return this;
      case "Tapped": return this;
    }
    return this;
  }

  override OnAccessibilityActivated(): void { this.IsFocused = true; }
  protected override DefaultAccessibilityLabel(): string | undefined { return this.text || this.placeholderText || undefined; }

  // ---- editing API (C# names) ----
  Submit(): void {
    if (this.IsMultiline && !this.ShouldSubmitOnEnter) { this.InsertAtCursor(ParagraphBreak); return; }
    this.ExecuteSubmit(!this.IsMultiline);
  }
  private ExecuteSubmit(clearFocus: boolean): void {
    if (clearFocus) this.IsFocused = false;
    this.TextSubmitted?.(this, this.text);
  }
  SelectAll(): void { this.SetSelection(0, this.text.length); }
  SetSelection(start: number, end: number): void {
    const s = Math.max(0, Math.min(this.text.length, Math.min(start, end))), e = Math.max(0, Math.min(this.text.length, Math.max(start, end)));
    this.cursorPosition = s; this.selectionLength = e - s;
    this.OnCursorMoved();
  }
  /** Selects the word around a character index (C# SelectWord). */
  SelectWord(index: number): void {
    const t = this.text;
    if (t.length === 0) return;
    const i = Math.max(0, Math.min(t.length - 1, index));
    const isWord = (c: string) => /[\p{L}\p{N}_]/u.test(c);
    if (!isWord(t[i])) { this.SetSelection(i, i + 1); return; }
    let s = i, e = i + 1;
    while (s > 0 && isWord(t[s - 1])) s--;
    while (e < t.length && isWord(t[e])) e++;
    this.SetSelection(s, e);
  }
  GetSelectedText(): string { return this.text.substr(this.cursorPosition, this.selectionLength); }
  DeleteSelection(): void { if (this.selectionLength > 0) this.ReplaceSelection(""); }
  InsertAtCursor(value: string): void { this.ReplaceSelection(value ?? ""); }
  CutSelection(): void { const s = this.GetSelectedText(); if (s) { void navigator.clipboard?.writeText(s); this.DeleteSelection(); } }
  CopySelection(): void { const s = this.GetSelectedText(); if (s) void navigator.clipboard?.writeText(s); }
  PasteFromClipboard(): void { void navigator.clipboard?.readText().then((t) => { if (t) this.ReplaceSelection(t); }); }

  private ReplaceSelection(value: string): void {
    const start = this.cursorPosition, end = start + this.selectionLength;
    const next = this.text.slice(0, start) + value + this.text.slice(end);
    this.selectionLength = 0;
    this.cursorPosition = start + value.length;
    this.stubSelectionStop = -1; this.selectionMovingEdge = -1;
    this.Text = next;
    this.OnCursorMoved();
  }

  // ---- keyboard (port of SkiaEditor.Blazor.cs: KeyboardManager subscription + stub editing) ----
  private SetFocusNative(focus: boolean): void {
    if (focus === this.subscribed) return;
    this.subscribed = focus;
    if (focus) { KeyboardManager.Subscribe(this.onKeyDown, this.onKeyChar); KeyboardManager.BlurExternalTextInput(); TextInputProxy.Attach(this); }
    else { TextInputProxy.Detach(this); KeyboardManager.Unsubscribe(this.onKeyDown, this.onKeyChar); }
  }

  private OnKeyDown(key: InputKey, e: KeyboardEvent): void {
    const shift = KeyboardManager.IsShiftPressed, ctrl = KeyboardManager.IsControlPressed, alt = KeyboardManager.IsAltPressed;
    const handled = () => e.preventDefault();
    // with the hidden textarea focused these keys become input / clipboard events there and are applied from those
    if (TextInputProxy.IsActive(this) && TextInputProxy.IsProxyTarget(e.target) && (key === "Backspace" || key === "Delete" || key === "Enter" || key === "NumpadEnter" || (ctrl && (key === "KeyC" || key === "KeyX" || key === "KeyV")))) return;
    switch (key) {
      case "Backspace": handled(); this.StubBackspace(); break;
      case "Delete": handled(); this.StubDelete(); break;
      case "Enter": case "NumpadEnter": handled(); this.StubPressEnter(alt, shift); break;
      case "ArrowLeft": handled(); this.StubMoveCursor(-1, shift); break;
      case "ArrowRight": handled(); this.StubMoveCursor(1, shift); break;
      case "ArrowUp": if (this.IsMultiline) { handled(); this.HandleVerticalArrow(true); } break;
      case "ArrowDown": if (this.IsMultiline) { handled(); this.HandleVerticalArrow(false); } break;
      case "Home": handled(); this.StubMoveCursor(-this.cursorPosition, shift); break;
      case "End": handled(); this.StubMoveCursor(this.text.length - this.cursorPosition, shift); break;
      case "Escape": handled(); this.IsFocused = false; break;
      case "KeyA": if (ctrl) { handled(); this.StubSelectAll(); } break;
      case "KeyC": if (ctrl) { handled(); this.CopySelection(); } break;
      case "KeyX": if (ctrl) { handled(); this.CutSelection(); } break;
      case "KeyV": if (ctrl) { handled(); this.PasteFromClipboard(); } break;
    }
  }

  StubTypeText(value: string): void { if (value) this.ReplaceSelection(value); }

  StubPressEnter(splitLine = false, shift = false): void {
    if (this.IsMultiline) {
      if (!splitLine && !shift && this.ShouldSubmitOnEnter) { this.ExecuteSubmit(false); return; }
      this.ReplaceSelection(ParagraphBreak);
      return;
    }
    this.ExecuteSubmit(false);
  }

  StubBackspace(count = 1): void {
    if (count <= 0) return;
    if (this.HasSelection) { this.ReplaceSelection(""); return; }
    if (this.text.length === 0) return;
    const remove = SkiaEditor.CodeUnitsBeforeCaret(this.text, this.cursorPosition, count);
    if (remove === 0) return;
    const start = this.cursorPosition - remove;
    this.cursorPosition = start; this.selectionLength = 0;
    this.Text = this.text.slice(0, start) + this.text.slice(start + remove);
    this.OnCursorMoved();
  }

  StubDelete(count = 1): void {
    if (count <= 0) return;
    if (this.HasSelection) { this.ReplaceSelection(""); return; }
    if (this.text.length === 0 || this.cursorPosition >= this.text.length) return;
    const remove = SkiaEditor.CodeUnitsAfterCaret(this.text, this.cursorPosition, count);
    if (remove === 0) return;
    this.selectionLength = 0;
    this.Text = this.text.slice(0, this.cursorPosition) + this.text.slice(this.cursorPosition + remove);
    this.OnCursorMoved();
  }

  /** Arrow / Home / End: the anchor stays, the moving edge follows (shift), or the selection collapses in the move direction. */
  StubMoveCursor(delta: number, extendSelection = false): void {
    const len = this.text.length;
    if (extendSelection) {
      if (!this.HasSelection) { this.stubSelectionStop = this.cursorPosition; this.selectionMovingEdge = this.cursorPosition; }
      this.selectionMovingEdge = Math.max(0, Math.min(len, this.selectionMovingEdge + delta));
      this.selectionLength = Math.abs(this.selectionMovingEdge - this.stubSelectionStop);
      this.cursorPosition = Math.min(this.selectionMovingEdge, this.stubSelectionStop);
      this.OnCursorMoved();
      return;
    }
    if (this.HasSelection) this.cursorPosition = delta < 0 ? this.cursorPosition : this.cursorPosition + this.selectionLength;
    else this.cursorPosition = Math.max(0, Math.min(len, this.cursorPosition + delta));
    this.selectionLength = 0;
    this.stubSelectionStop = -1; this.selectionMovingEdge = -1;
    this.OnCursorMoved();
  }

  StubSelectRange(start: number, length: number): void {
    const len = this.text.length;
    const s = Math.max(0, Math.min(len, start)), l = Math.max(0, Math.min(len - s, length));
    this.cursorPosition = s; this.selectionLength = l;
    this.stubSelectionStop = s; this.selectionMovingEdge = s + l;
    this.OnCursorMoved();
  }
  StubSelectAll(): void { this.StubSelectRange(0, this.text.length); }

  /** Up / Down keep the caret x on the neighbouring line (C# HandleVerticalArrow). */
  protected HandleVerticalArrow(up: boolean): void {
    const lines = this.Label.GetLineBoxes();
    if (lines.length <= 1) return;
    const caret = this.CaretRectLabelPx(this.cursorPosition);
    if (!caret) return;
    const current = lines.findIndex((l) => caret.Top >= l.Top && caret.Top < l.Top + l.Height);
    const target = (current < 0 ? 0 : current) + (up ? -1 : 1);
    if (target < 0 || target >= lines.length) return;
    const line = lines[target];
    this.selectionLength = 0; this.stubSelectionStop = -1; this.selectionMovingEdge = -1;
    this.cursorPosition = this.HitTestIndex(caret.Left, line.Top + line.Height / 2);
    this.OnCursorMoved();
  }

  /** UTF-16 units of `count` grapheme clusters before the caret (C# CodeUnitsBeforeCaret). */
  static CodeUnitsBeforeCaret(text: string, caret: number, count: number): number {
    if (!text || caret <= 0 || count <= 0) return 0;
    caret = Math.min(caret, text.length);
    const starts: number[] = [];
    for (const seg of SkiaEditor.Graphemes(text)) { if (seg.index >= caret) break; starts.push(seg.index); }
    if (starts.length === 0) return caret;
    const idx = starts.length - count;
    return caret - (idx > 0 ? starts[idx] : 0);
  }
  static CodeUnitsAfterCaret(text: string, caret: number, count: number): number {
    if (!text || caret >= text.length || count <= 0) return 0;
    let pos = Math.max(0, caret), n = 0;
    for (const seg of SkiaEditor.Graphemes(text.slice(pos))) { if (n++ >= count) break; pos = caret + seg.index + seg.segment.length; }
    return pos - caret;
  }
  private static Graphemes(text: string): { index: number; segment: string }[] {
    const Seg = (Intl as unknown as { Segmenter?: new (l?: string, o?: { granularity: string }) => { segment(t: string): Iterable<{ index: number; segment: string }> } }).Segmenter;
    if (Seg) return [...new Seg(undefined, { granularity: "grapheme" }).segment(text)];
    const out: { index: number; segment: string }[] = []; let i = 0;
    for (const cp of Array.from(text)) { out.push({ index: i, segment: cp }); i += cp.length; }
    return out;
  }

  protected override OnDisposing(): void {
    this.blink.Stop();
    this.blink.Dispose();
    if (SkiaEditor.Focused === this) SkiaEditor.Focused = undefined;
    this.SetFocusNative(false);
  }
}
