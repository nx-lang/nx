import type { Font } from "canvaskit-wasm";
import { type DrawingContext, SkiaControl } from "../core/SkiaControl";
import { ControlTappedEventArgs, type GestureEventProcessingInfo, type SkiaGesturesParameters } from "../core/Gestures";
import { Super } from "../core/Super";
import {
  type Color, Colors, type DrawTextAlignment, type FontAttributes, type LineBreakMode, ScaledSize, SKRect, type TextAlignment,
  type TextTransform, Thickness,
} from "../core/Types";
import { TextSpan } from "./TextSpan";

/** Resolved faces for one style (the label itself or one span): main font + FontFamilyFallback chain + metrics. */
interface SpanFonts { Key: string; Main: Font; Fallbacks: Font[]; Ascent: number; Descent: number; SizePx: number }
/** A run of text drawn with one font (main, or a fallback for glyphs the main font lacks) and one span style. */
interface TextRun { Text: string; Font: Font; Width: number; Span?: TextSpan; Fonts: SpanFonts }
/** One laid-out line: runs, total advance, max ascent above / descent below the baseline (pixels). */
interface TextLine { Runs: TextRun[]; Width: number; Ascent: number; Descent: number }
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
    return ScaledSize.FromPixels(Math.ceil(width) + px, Math.ceil(this.BlockHeight()) + py, scale);
  }

  protected override Paint(ctx: DrawingContext): void {
    if (this.lines.length === 0 || !this.mainFonts) return;
    const scale = ctx.Scale;
    const d = ctx.Destination;
    const p = this.padding;
    const left = d.Left + p.Left * scale, right = d.Right - p.Right * scale;
    const top = d.Top + p.Top * scale, bottom = d.Bottom - p.Bottom * scale;
    const blockH = this.BlockHeight();
    let y = top;
    if (this.verticalTextAlignment === "Center") y = top + (bottom - top - blockH) / 2;
    else if (this.verticalTextAlignment === "End") y = bottom - blockH;

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
    for (const line of this.lines) {
      const lh = this.LineHeightPx(line);
      let x = left;
      if (this.horizontalTextAlignment === "Center") x = left + (right - left - line.Width) / 2;
      else if (this.horizontalTextAlignment === "End") x = right - line.Width;
      const baseline = y + line.Ascent;
      for (const run of line.Runs) {
        const span = run.Span;
        if (span) {
          span.Rects.push(new SKRect(x - d.Left, y - d.Top, x + run.Width - d.Left, y + lh - d.Top));
          if (span.BackgroundColor) canvas.drawRect(CK.LTRBRect(x, y, x + run.Width, y + lh), paintFor(span.BackgroundColor));
        }
        const color = span?.TextColor ?? this.textColor;
        if (run.Text) canvas.drawText(run.Text, x, baseline, paintFor(color), run.Font);
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

  /** Return null to not consume the tap. */
  protected OnSpanTapped(span: TextSpan, args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    span.FireTap(new ControlTappedEventArgs(this, args, apply));
    return this;
  }
}
