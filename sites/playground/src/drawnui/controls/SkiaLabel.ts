import type { Font } from "canvaskit-wasm";
import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { ControlTappedEventArgs, type GestureEventProcessingInfo, type SkiaGesturesParameters } from "../core/Gestures";
import { Super } from "../core/Super";
import {
  type Color, Colors, type DrawTextAlignment, type FontAttributes, type LineBreakMode, ScaledSize, SKRect, type SkiaGradient, type TextAlignment,
  type TextTransform, Thickness,
} from "../core/Types";
import { TextSpan } from "./TextSpan";

/** Resolved faces for one style (the label itself or one span): main font + FontFamilyFallback chain + metrics. */
interface SpanFonts { Key: string; Main: Font; Fallbacks: Font[]; Ascent: number; Descent: number; SizePx: number }
/** A run of text drawn with one font (main, or a fallback for glyphs the main font lacks) and one span style. */
interface TextRun { Text: string; Font: Font; Width: number; Span?: TextSpan; Fonts: SpanFonts }
/** One laid-out line: runs, total advance, max ascent above / descent below the baseline (pixels). */
interface TextLine { Runs: TextRun[]; Width: number; Ascent: number; Descent: number }

/** One code point of the laid-out text: its UTF-16 index/length in Text and its box in pixels relative to DrawingRect. */
export interface GlyphBox { Index: number; Length: number; Line: number; Left: number; Top: number; Width: number; Height: number }
/** A laid-out line: first text index, box in pixels relative to DrawingRect. */
export interface LineBox { Line: number; Start: number; End: number; Left: number; Top: number; Width: number; Height: number }
/** A wrap unit: a word (or glued span fragment) with its style. */
interface Token { Text: string; Fonts: SpanFonts; Span?: TextSpan; SpaceBefore: boolean; TrailingSpace?: boolean }

/**
 * Mirrors DrawnUi SkiaLabel: multi-line text with word wrapping, MaxLines + tail ellipsis, horizontal /
 * vertical alignment, LineSpacing / LineHeight, weights and attributes resolved through the font registry,
 * the opt-in per-codepoint FontFamilyFallback (symbols/emoji missing from the main face) and `Spans`
 * (TextSpan children: per-fragment color/size/weight/italic/underline/strikeout/background/Tapped).
 * Cached as Operations by default, like DrawnUi. Every text property invalidates like a bindable property.
 */
export class SkiaLabel extends SkiaControl {
  private text = "";
  private fontSize = 12;
  private textColor: Color = Colors.GreenYellow;
  private fontFamily = "";
  private fontFamilyFallback = "";
  private fontWeight = 0;
  private fontAttributes: FontAttributes = "None";
  private maxLines = -1;
  private lineBreakMode: LineBreakMode = "TailTruncation";
  private horizontalTextAlignment: DrawTextAlignment = "Start";
  private verticalTextAlignment: TextAlignment = "Start";
  private lineSpacing = 1;
  private lineHeight = 1;
  private textTransform: TextTransform = "None";
  private padding: Thickness = Thickness.Zero;

  /** Styled fragments; when not empty they replace `Text` (same precedence as C#). */
  readonly Spans: TextSpan[] = [];

  /** Set to `Aria.RoleText` to expose every label to screen readers (React extension; C# is opt-in per control). */
  static override DefaultAccessibilityRole?: string;
  /** Like C# OnTextInternalChanged: the spoken label is the text (or the joined spans) unless AccessibilityLabel is set. */
  protected override DefaultAccessibilityLabel(): string | undefined {
    return this.Spans.length > 0 ? this.Spans.map((s) => s.Text).join("") : this.text || undefined;
  }

  constructor() {
    super();
    this.UseCache = "Operations";
  }

  // ---- invalidating accessors (DrawnUi bindable properties) ----
  private Set<K extends keyof this>(key: K, v: this[K]): void { if (this[key] !== v) { this[key] = v; this.Update(); } }

  get Text(): string { return this.text; }
  set Text(v: string) { this.Set("text" as keyof this, v as this[keyof this]); }

  get FontSize(): number { return this.fontSize; }
  set FontSize(v: number) { this.Set("fontSize" as keyof this, v as this[keyof this]); }
  get TextColor(): Color { return this.textColor; }
  set TextColor(v: Color) { this.Set("textColor" as keyof this, v as this[keyof this]); }

  get FontFamily(): string { return this.fontFamily; }
  set FontFamily(v: string) { this.Set("fontFamily" as keyof this, v as this[keyof this]); }
  /**
   * Alias (or comma-separated chain of aliases) tried per codepoint when the main font has no glyph,
   * e.g. "FontSymbols,FontSymbols2" from `AddSymbols()`. Same idea as C# FontFamilyFallback, extended to a chain.
   */
  get FontFamilyFallback(): string { return this.fontFamilyFallback; }
  set FontFamilyFallback(v: string) { this.Set("fontFamilyFallback" as keyof this, v as this[keyof this]); }

  get FontWeight(): number { return this.fontWeight; }
  set FontWeight(v: number) { this.Set("fontWeight" as keyof this, v as this[keyof this]); }
  get FontAttributes(): FontAttributes { return this.fontAttributes; }
  set FontAttributes(v: FontAttributes) { this.Set("fontAttributes" as keyof this, v as this[keyof this]); }

  get MaxLines(): number { return this.maxLines; }
  set MaxLines(v: number) { this.Set("maxLines" as keyof this, v as this[keyof this]); }
  get LineBreakMode(): LineBreakMode { return this.lineBreakMode; }
  set LineBreakMode(v: LineBreakMode) { this.Set("lineBreakMode" as keyof this, v as this[keyof this]); }
  get HorizontalTextAlignment(): DrawTextAlignment { return this.horizontalTextAlignment; }
  set HorizontalTextAlignment(v: DrawTextAlignment) { this.Set("horizontalTextAlignment" as keyof this, v as this[keyof this]); }
  get VerticalTextAlignment(): TextAlignment { return this.verticalTextAlignment; }
  set VerticalTextAlignment(v: TextAlignment) { this.Set("verticalTextAlignment" as keyof this, v as this[keyof this]); }

  get LineSpacing(): number { return this.lineSpacing; }
  set LineSpacing(v: number) { this.Set("lineSpacing" as keyof this, v as this[keyof this]); }
  /** Multiplier applied to the natural line height (ascent + descent). */
  get LineHeight(): number { return this.lineHeight; }
  set LineHeight(v: number) { this.Set("lineHeight" as keyof this, v as this[keyof this]); }
  get TextTransform(): TextTransform { return this.textTransform; }
  set TextTransform(v: TextTransform) { this.Set("textTransform" as keyof this, v as this[keyof this]); }
  get Padding(): Thickness { return this.padding; }
  set Padding(v: Thickness) { this.Set("padding" as keyof this, v as this[keyof this]); }

  /** Number of laid-out lines after the last measure. */
  get LinesCount(): number { return this.lines.length; }

  // ---- spans as children (reconciler AddSubView / InsertSubView / RemoveSubView) ----
  override AddSubView(control: SkiaControl | TextSpan): void { this.InsertSubView(this.Spans.length, control); }
  override InsertSubView(index: number, control: SkiaControl | TextSpan): void {
    if (!(control instanceof TextSpan)) throw new Error("DrawnUi: SkiaLabel children must be <TextSpan>");
    control.Parent = this;
    this.Spans.splice(index, 0, control);
    this.Update();
  }
  override RemoveSubView(control: SkiaControl | TextSpan): void {
    const i = this.Spans.indexOf(control as TextSpan);
    if (i < 0) return;
    this.Spans.splice(i, 1);
    (control as TextSpan).Parent = undefined;
    this.Update();
  }

  // ---- layout ----
  private lines: TextLine[] = [];
  private mainFonts?: SpanFonts;
  private readonly fontsCache = new Map<string, SpanFonts>();
  private readonly runCache = new Map<string, TextRun[]>();

  private ResolveFonts(family: string, weight: number, italic: boolean, sizePx: number): SpanFonts {
    const key = `${family}|${this.fontFamilyFallback}|${weight}|${italic}|${sizePx}`;
    let f = this.fontsCache.get(key);
    if (f) return f;
    const main = Super.GetFont(family, weight, italic, sizePx);
    const m = main.getMetrics();
    f = {
      Key: key, Main: main, SizePx: sizePx, Ascent: -m.ascent, Descent: m.descent,
      Fallbacks: this.fontFamilyFallback
        ? this.fontFamilyFallback.split(",").map((a) => a.trim()).filter(Boolean).map((a) => Super.GetFont(a, weight, italic, sizePx))
        : [],
    };
    this.fontsCache.set(key, f);
    return f;
  }

  /** The label's own style, and the base every span inherits from. */
  private ResolveMainFonts(scale: number): SpanFonts {
    const bold = this.fontAttributes === "Bold" || this.fontAttributes === "BoldItalic";
    const italic = this.fontAttributes === "Italic" || this.fontAttributes === "BoldItalic";
    const weight = this.fontWeight > 0 ? this.fontWeight : bold ? 700 : 0;
    return this.ResolveFonts(this.fontFamily, weight, italic, this.fontSize * scale);
  }

  private ResolveSpanFonts(span: TextSpan, scale: number): SpanFonts {
    const bold = span.IsBold || this.fontAttributes === "Bold" || this.fontAttributes === "BoldItalic";
    const italic = span.IsItalic || this.fontAttributes === "Italic" || this.fontAttributes === "BoldItalic";
    const weight = span.FontWeight > 0 ? span.FontWeight : span.IsBold ? 700 : this.fontWeight > 0 ? this.fontWeight : bold ? 700 : 0;
    return this.ResolveFonts(span.FontFamily ?? this.fontFamily, weight, italic, (span.FontSize ?? this.fontSize) * scale);
  }

  private static Advance(font: Font, text: string): number {
    let w = 0;
    for (const adv of font.getGlyphWidths(font.getGlyphIDs(text))) w += adv;
    return w;
  }

  /**
   * Splits text into runs by glyph availability: the main font, or the first fallback that has a glyph where the
   * main font has glyph 0. Spaces always stay on the main font (fallback faces often carry very wide spaces).
   */
  private Segment(text: string, fonts: SpanFonts, span?: TextSpan): TextRun[] {
    if (text.length === 0) return [];
    const cacheKey = fonts.Key + " " + text;
    let cached = this.runCache.get(cacheKey);
    if (!cached) {
      const main = fonts.Main;
      const fbs = fonts.Fallbacks;
      cached = [];
      if (fbs.length === 0) {
        cached.push({ Text: text, Font: main, Width: SkiaLabel.Advance(main, text), Fonts: fonts });
      } else {
        const cps = Array.from(text);
        const mainIds = main.getGlyphIDs(text, cps.length);
        const fbIds = fbs.map((f) => f.getGlyphIDs(text, cps.length));
        const fontFor = (i: number): Font => {
          if (cps[i] === " " || mainIds[i] !== 0) return main;
          for (let k = 0; k < fbs.length; k++) if (fbIds[k][i] !== 0) return fbs[k];
          return main;
        };
        let start = 0, current = fontFor(0);
        for (let i = 1; i < cps.length; i++) {
          const f = fontFor(i);
          if (f !== current) {
            const t = cps.slice(start, i).join("");
            cached.push({ Text: t, Font: current, Width: SkiaLabel.Advance(current, t), Fonts: fonts });
            start = i; current = f;
          }
        }
        const t = cps.slice(start).join("");
        cached.push({ Text: t, Font: current, Width: SkiaLabel.Advance(current, t), Fonts: fonts });
      }
      this.runCache.set(cacheKey, cached);
    }
    return span ? cached.map((r) => ({ ...r, Span: span })) : cached;
  }

  private Width(runs: TextRun[]): number { let w = 0; for (const r of runs) w += r.Width; return w; }

  private Transform(text: string): string {
    switch (this.textTransform) {
      case "Uppercase": return text.toUpperCase();
      case "Lowercase": return text.toLowerCase();
      case "Titlecase": return text.replace(/(^|\s)(\S)/g, (_, s, c) => s + c.toUpperCase());
      default: return text;
    }
  }

  /**
   * Paragraphs (split on "\n") of wrap tokens, built from Spans when present, else from Text.
   * A fragment that does not start with a space glues to the previous word (no break opportunity is added).
   */
  private Tokenize(scale: number): Token[][] {
    const paragraphs: Token[][] = [[]];
    const add = (text: string, fonts: SpanFonts, span?: TextSpan) => {
      const parts = this.Transform(text).split("\n");
      for (let p = 0; p < parts.length; p++) {
        if (p > 0) paragraphs.push([]);
        const para = paragraphs[paragraphs.length - 1];
        const words = parts[p].split(" ");
        for (let w = 0; w < words.length; w++) {
          if (words[w] === "") continue;
          para.push({ Text: words[w], Fonts: fonts, Span: span, SpaceBefore: w > 0 || (para.length > 0 && parts[p].startsWith(" ")) });
        }
        if (parts[p].endsWith(" ") && para.length > 0) para[para.length - 1].TrailingSpace = true;
      }
    };
    if (this.Spans.length > 0) {
      for (const s of this.Spans) if (s.IsVisible && s.Text) add(s.Text, this.ResolveSpanFonts(s, scale), s);
    } else if (this.text) {
      add(this.text, this.mainFonts!);
    }
    // a fragment ending with a space puts the space before the NEXT token
    for (const para of paragraphs) {
      for (let i = 0; i < para.length; i++) {
        if (para[i].TrailingSpace && i + 1 < para.length) para[i + 1].SpaceBefore = true;
      }
    }
    return paragraphs;
  }

  private NewLine(): TextLine { return { Runs: [], Width: 0, Ascent: 0, Descent: 0 }; }

  private Append(line: TextLine, runs: TextRun[]): void {
    for (const r of runs) {
      const last = line.Runs[line.Runs.length - 1];
      if (last && last.Font === r.Font && last.Span === r.Span) { last.Text += r.Text; last.Width += r.Width; }
      else line.Runs.push({ ...r });
      line.Width += r.Width;
      if (r.Fonts.Ascent > line.Ascent) line.Ascent = r.Fonts.Ascent;
      if (r.Fonts.Descent > line.Descent) line.Descent = r.Fonts.Descent;
    }
  }

  /** Word-wraps tokens into lines that fit maxWidth (Infinity = no wrap), applies MaxLines with a tail ellipsis. */
  private LayoutLines(maxWidth: number, scale: number): TextLine[] {
    const wrap = this.lineBreakMode !== "NoWrap" && isFinite(maxWidth);
    const out: TextLine[] = [];
    const empty = (fonts: SpanFonts): TextLine => { const l = this.NewLine(); l.Ascent = fonts.Ascent; l.Descent = fonts.Descent; return l; };

    for (const para of this.Tokenize(scale)) {
      let line = this.NewLine();
      let lastFonts = this.mainFonts!;
      for (let i = 0; i < para.length; i++) {
        const tok = para[i];
        lastFonts = tok.Fonts;
        // glued fragments (no space between spans) wrap as one word
        let word = this.Segment(tok.Text, tok.Fonts, tok.Span);
        while (i + 1 < para.length && !para[i + 1].SpaceBefore) { i++; word = word.concat(this.Segment(para[i].Text, para[i].Fonts, para[i].Span)); }
        const space = tok.SpaceBefore && line.Runs.length > 0 ? this.Segment(" ", tok.Fonts, tok.Span) : [];
        const spaceW = this.Width(space), wordW = this.Width(word);
        if (!wrap || line.Width + spaceW + wordW <= maxWidth || (line.Runs.length === 0 && wordW <= maxWidth)) {
          this.Append(line, space); this.Append(line, word);
          continue;
        }
        if (line.Runs.length > 0) { out.push(line); line = this.NewLine(); }
        if (wordW <= maxWidth) { this.Append(line, word); continue; }
        // word longer than the line: break by code points
        for (const r of word) {
          for (const ch of Array.from(r.Text)) {
            const run = this.Segment(ch, r.Fonts, r.Span);
            const chW = this.Width(run);
            if (line.Runs.length > 0 && line.Width + chW > maxWidth) { out.push(line); line = this.NewLine(); }
            this.Append(line, run);
          }
        }
      }
      out.push(line.Runs.length > 0 ? line : empty(lastFonts));
    }

    if (this.maxLines > 0 && out.length > this.maxLines) {
      out.length = this.maxLines;
      const truncates = this.lineBreakMode === "TailTruncation" || this.lineBreakMode === "HeadTruncation" || this.lineBreakMode === "MiddleTruncation";
      if (truncates) {
        const last = out[this.maxLines - 1];
        const tailRun = last.Runs[last.Runs.length - 1];
        const fonts = tailRun ? tailRun.Fonts : this.mainFonts!;
        const ell = this.Segment("…", fonts, tailRun?.Span);
        const ellW = this.Width(ell);
        while (last.Runs.length > 0 && last.Width + ellW > maxWidth) {
          const r = last.Runs[last.Runs.length - 1];
          const cps = Array.from(r.Text);
          cps.pop();
          while (cps.length > 0 && cps[cps.length - 1] === " ") cps.pop();
          last.Width -= r.Width;
          if (cps.length === 0) { last.Runs.pop(); continue; }
          r.Text = cps.join(""); r.Width = SkiaLabel.Advance(r.Font, r.Text);
          last.Width += r.Width;
        }
        this.Append(last, ell);
      }
    }
    return out;
  }

  private LineHeightPx(line: TextLine): number { return (line.Ascent + line.Descent) * this.lineHeight; }

  private BlockHeight(): number {
    let h = 0;
    for (let i = 0; i < this.lines.length; i++) {
      const lh = this.LineHeightPx(this.lines[i]);
      h += i < this.lines.length - 1 ? lh * this.lineSpacing : lh;
    }
    return h;
  }

  protected override MeasureAbsolute(widthConstraint: number, _heightConstraint: number, scale: number): ScaledSize {
    this.mainFonts = this.ResolveMainFonts(scale);
    const px = this.padding.HorizontalThickness * scale, py = this.padding.VerticalThickness * scale;
    this.lines = this.text || this.Spans.length > 0 ? this.LayoutLines(widthConstraint - px, scale) : [];
    let width = 0;
    for (const l of this.lines) width = Math.max(width, l.Width);
    const extra = this.EffectsExtra(scale);
    this.LayoutVersion++;
    return ScaledSize.FromPixels(Math.ceil(width + extra.W) + px, Math.ceil(this.BlockHeight() + extra.H) + py, scale);
  }

  /** C# GradientByLines: FillGradient spans each line's bounds (default) instead of the whole text block. */
  GradientByLines = true;
  /** C# KeepSpacesOnLineBreaks / NeedsGlyphPositions: accepted; glyph boxes are always available through GetGlyphBoxes. */
  KeepSpacesOnLineBreaks = false;
  NeedsGlyphPositions = false;
  /** Bumped on every measure: consumers cache GetGlyphBoxes() against it. */
  LayoutVersion = 0;
  /** Height of the first laid-out line in pixels (C# MeasuredLineHeight); 0 before the first measure. */
  get MeasuredLineHeight(): number { return this.lines.length ? this.LineHeightPx(this.lines[0]) : 0; }

  /** Line origins relative to DrawingRect, same alignment math as Paint. */
  private LineGeometry(): { x: number; y: number; w: number; h: number; text: string }[] {
    const scale = this.RenderingScale, d = this.DrawingRect, p = this.padding;
    const left = p.Left * scale, right = d.Width - p.Right * scale, top = p.Top * scale, bottom = d.Height - p.Bottom * scale;
    const extra = this.EffectsExtra(scale);
    const blockH = this.BlockHeight() + extra.H;
    let y = top;
    if (this.verticalTextAlignment === "Center") y = top + (bottom - top - blockH) / 2;
    else if (this.verticalTextAlignment === "End") y = bottom - blockH;
    y += extra.Stroke / 2;
    const out: { x: number; y: number; w: number; h: number; text: string }[] = [];
    for (const line of this.lines) {
      const lh = this.LineHeightPx(line);
      const lineW = line.Width + extra.W;
      let x = left;
      if (this.horizontalTextAlignment === "Center") x = left + (right - left - lineW) / 2;
      else if (this.horizontalTextAlignment === "End") x = right - lineW;
      x += extra.Stroke / 2;
      out.push({ x, y, w: line.Width, h: lh, text: line.Runs.map((r) => r.Text).join("") });
      y += lh * this.lineSpacing;
    }
    return out;
  }

  /** Lines as CSS text for the accessibility overlay (AccessibilityTextSelectable): font alias chain, weight and px size. */
  override GetAccessibilityTextLines(scale: number): import("../core/Accessibility").AccessibilityTextLine[] {
    const family = [this.fontFamily || Super.DefaultFontAlias, ...this.fontFamilyFallback.split(",").map((f) => f.trim()).filter(Boolean)].filter(Boolean).join(", ");
    const weight = this.fontWeight > 0 ? this.fontWeight : this.fontAttributes === "Bold" || this.fontAttributes === "BoldItalic" ? 600 : 400;
    const size = this.fontSize;
    return this.LineGeometry().map((g) => ({ Text: g.text, Left: g.x / scale, Top: g.y / scale, Width: g.w / scale, Height: g.h / scale, FontFamily: family, FontWeight: weight, FontSize: size }));
  }

  /** Laid-out lines with their text index range (a wrapped-away space or line break sits between two lines). */
  GetLineBoxes(): LineBox[] {
    const geo = this.LineGeometry();
    const out: LineBox[] = [];
    let cursor = 0;
    geo.forEach((g, i) => {
      let start = g.text.length ? this.text.indexOf(g.text, cursor) : cursor;
      if (start < 0) start = cursor;
      const end = start + g.text.length;
      out.push({ Line: i, Start: start, End: end, Left: g.x, Top: g.y, Width: g.w, Height: g.h });
      cursor = end;
    });
    return out;
  }

  /** Every code point of the laid-out text with its box (editor caret / selection / hit testing). */
  GetGlyphBoxes(): GlyphBox[] {
    const out: GlyphBox[] = [];
    const geo = this.LineGeometry();
    let cursor = 0;
    this.lines.forEach((line, li) => {
      const g = geo[li];
      let start = g.text.length ? this.text.indexOf(g.text, cursor) : cursor;
      if (start < 0) start = cursor;
      let x = g.x, index = start;
      for (const run of line.Runs) {
        const cps = Array.from(run.Text);
        const widths = run.Font.getGlyphWidths(run.Font.getGlyphIDs(run.Text, cps.length));
        cps.forEach((cp, k) => {
          const w = widths[k] ?? 0;
          out.push({ Index: index, Length: cp.length, Line: li, Left: x, Top: g.y, Width: w, Height: g.h });
          x += w; index += cp.length;
        });
      }
      cursor = start + g.text.length;
    });
    return out;
  }
  /** Outline around the glyphs, drawn under the fill (C# StrokeColor / StrokeWidth in points; Transparent = none). */
  StrokeColor: Color = Colors.Transparent;
  StrokeWidth = 1;
  StrokeGradient?: SkiaGradient;
  /** Drop shadow: a stroked copy of the text offset by DropShadowOffsetX/Y points (C# DropShadow*). */
  DropShadowColor: Color = Colors.Transparent;
  DropShadowSize = 2;
  DropShadowOffsetX = 2;
  DropShadowOffsetY = 2;

  private HasStroke(): boolean { return this.StrokeWidth > 0 && this.StrokeColor !== Colors.Transparent; }
  private HasDropShadow(): boolean { return this.DropShadowSize > 0 && this.DropShadowColor !== Colors.Transparent; }
  /** C# measurement: stroke adds StrokeWidth*2 on each axis, the shadow adds DropShadowSize + offset (pixels). */
  private EffectsExtra(scale: number): { W: number; H: number; Stroke: number } {
    const stroke = this.HasStroke() ? this.StrokeWidth * 2 * scale : 0;
    const sw = this.HasDropShadow() ? (this.DropShadowSize + this.DropShadowOffsetX) * scale : 0;
    const sh = this.HasDropShadow() ? (this.DropShadowSize + this.DropShadowOffsetY) * scale : 0;
    return { W: stroke + sw, H: stroke + sh, Stroke: stroke };
  }
  /** C# SkiaLabel.SetupBackgroundPaint: no BackgroundColor = no background, the gradient goes on the glyphs. */
  protected override FillGradientPaintsBackground(): boolean { return !!this.BackgroundColor; }

  protected override Paint(ctx: DrawingContext): void {
    if (this.lines.length === 0 || !this.mainFonts) return;
    const scale = ctx.Scale;
    const d = ctx.Destination;
    const p = this.padding;
    const left = d.Left + p.Left * scale, right = d.Right - p.Right * scale;
    const top = d.Top + p.Top * scale, bottom = d.Bottom - p.Bottom * scale;
    const extra = this.EffectsExtra(scale);
    const blockH = this.BlockHeight() + extra.H;
    let y = top;
    if (this.verticalTextAlignment === "Center") y = top + (bottom - top - blockH) / 2;
    else if (this.verticalTextAlignment === "End") y = bottom - blockH;
    // C#: the glyphs sit strokeOffset in from the left/top of the inflated box, the shadow band stays below/right
    y += extra.Stroke / 2;
    const CKp = Super.CK;
    let strokePaint: InstanceType<typeof CKp.Paint> | undefined, shadowPaint: InstanceType<typeof CKp.Paint> | undefined;
    if (this.HasStroke()) { strokePaint = new CKp.Paint(); strokePaint.setAntiAlias(true); strokePaint.setStyle(CKp.PaintStyle.Stroke); strokePaint.setStrokeWidth(this.StrokeWidth * 2 * scale); strokePaint.setColor(Super.ParseColor(this.StrokeColor)); }
    if (this.HasDropShadow()) { shadowPaint = new CKp.Paint(); shadowPaint.setAntiAlias(true); shadowPaint.setStyle(CKp.PaintStyle.Stroke); shadowPaint.setStrokeWidth(this.DropShadowSize * 2 * scale); shadowPaint.setColor(Super.ParseColor(this.DropShadowColor)); }
    const shadowDx = Math.trunc(this.DropShadowOffsetX * scale), shadowDy = Math.trunc(this.DropShadowOffsetY * scale);

    for (const s of this.Spans) s.Rects.length = 0;

    const CK = Super.CK;
    const paints = new Map<string, InstanceType<typeof CK.Paint>>();
    const paintFor = (color: Color, stroke = 0) => {
      const key = color + "|" + stroke;
      let paint = paints.get(key);
      if (!paint) {
        paint = new CK.Paint();
        paint.setColor(Super.ParseColor(color));
        paint.setAntiAlias(true);
        if (stroke > 0) { paint.setStyle(CK.PaintStyle.Stroke); paint.setStrokeWidth(stroke); }
        paints.set(key, paint);
      }
      return paint;
    };
    const canvas = ctx.Context.Canvas;
    const gradient = this.FillGradient;
    const textRect = new SKRect(left, top, right, bottom);
    for (const line of this.lines) {
      const lh = this.LineHeightPx(line);
      const lineW = line.Width + extra.W;
      let x = left;
      if (this.horizontalTextAlignment === "Center") x = left + (right - left - lineW) / 2;
      else if (this.horizontalTextAlignment === "End") x = right - lineW;
      x += extra.Stroke / 2;
      const baseline = y + line.Ascent;
      // C# paintDefault gets the gradient over rectDraw, or over line.Bounds per line (GradientByLines)
      const gradientRect = gradient ? (this.GradientByLines ? new SKRect(x, y, x + line.Width, y + lh) : textRect) : undefined;
      if (strokePaint && this.StrokeGradient) this.SetupGradient(strokePaint, this.StrokeGradient, gradientRect ?? new SKRect(x, y, x + line.Width, y + lh));
      for (const run of line.Runs) {
        const span = run.Span;
        if (span) {
          span.Rects.push(new SKRect(x - d.Left, y - d.Top, x + run.Width - d.Left, y + lh - d.Top));
          if (span.BackgroundColor) canvas.drawRect(CK.LTRBRect(x, y, x + run.Width, y + lh), paintFor(span.BackgroundColor));
        }
        const color = span?.TextColor ?? this.textColor;
        if (run.Text) {
          // C# DrawText order: drop shadow, stroke, fill
          if (shadowPaint) canvas.drawText(run.Text, x + shadowDx, baseline + shadowDy, shadowPaint, run.Font);
          if (strokePaint) canvas.drawText(run.Text, x, baseline, strokePaint, run.Font);
          const paint = paintFor(color);
          if (gradient && gradientRect) this.SetupGradient(paint, gradient, gradientRect);
          canvas.drawText(run.Text, x, baseline, paint, run.Font);
        }
        if (span?.HasDecorations) {
          // same geometry as C# DrawSpanDecorations; CanvasKit exposes no underline/strikeout/x-height metrics,
          // so the C# fallbacks apply: underline 1 scaled px under the baseline, strikeout at half an estimated x-height
          if (span.Underline && span.UnderlineWidth !== 0) {
            const w = span.UnderlineWidth > 0 ? span.UnderlineWidth * scale : -span.UnderlineWidth;
            const yl = Math.round(baseline + scale);
            canvas.drawLine(x, yl, x + run.Width, yl, paintFor(color, w));
          }
          if (span.Strikeout) {
            const yl = Math.round(baseline - (run.Fonts.SizePx * 0.52) / 2);
            canvas.drawLine(x, yl, x + run.Width, yl, paintFor(span.StrikeoutColor, span.StrikeoutWidth * scale));
          }
        }
        x += run.Width;
      }
      y += lh * this.lineSpacing;
    }
    for (const paint of paints.values()) paint.delete();
    strokePaint?.delete(); shadowPaint?.delete();
  }

  // ---- span taps (port of C# SkiaLabel.ProcessGestures) ----
  override ProcessGestures(args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (args.Type === "Tapped" && this.Spans.length > 0) {
      const x = apply.MappedLocation.X + apply.ChildOffset.X - this.DrawingRect.Left;
      const y = apply.MappedLocation.Y + apply.ChildOffset.Y - this.DrawingRect.Top;
      for (const span of this.Spans) {
        if (span.HasTapHandler && span.HitIsInside(x, y)) {
          this.PlayRippleAnimation(this.TouchEffectColor, x / this.RenderingScale, y / this.RenderingScale);
          return this.OnSpanTapped(span, args, apply);
        }
      }
    }
    return super.ProcessGestures(args, apply);
  }

  /** Pointer cursor over a tappable span (the label itself may not be tappable). */
  override WantsPointerCursor(x: number, y: number): boolean {
    if (super.WantsPointerCursor(x, y)) return true;
    for (const span of this.Spans) if (span.HasTapHandler && span.HitIsInside(x, y)) return true;
    return false;
  }

  /** Return null to not consume the tap. */
  protected OnSpanTapped(span: TextSpan, args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    span.FireTap(new ControlTappedEventArgs(this, args, apply));
    return this;
  }
}
