import type { SkiaControl } from "../core/SkiaControl";
import type { GestureEventProcessingInfo, SkiaGesturesParameters } from "../core/Gestures";
import { type Color, Colors } from "../core/Types";
import { SkiaLabel } from "./SkiaLabel";
import { TextSpan } from "./TextSpan";

/** Inline emphasis state while walking markdown (C# isBold / isItalic / isStrikethrough / heading flags). */
interface InlineState { bold: boolean; italic: boolean; strike: boolean; heading: 0 | 1 | 2 | 3; code: boolean }

/**
 * Mirrors DrawnUi SkiaRichLabel: a SkiaLabel whose Text is markdown, rendered as TextSpans.
 * Lightweight parser like the C# one (display-oriented, not full CommonMark): headings, paragraphs, bullet and
 * numbered lists, fenced code blocks, inline code, bold, italic, strikethrough, links (tap -> LinkTapped).
 */
export class SkiaRichLabel extends SkiaLabel {
  // DrawnUi static defaults
  static ColorLink: Color = "#6495ED";
  static ColorCodeBackground: Color = "#696969";
  static ColorCodeBlock: Color = "#222222";
  static ColorCode: Color = Colors.White;
  static ColorStrikeout: Color = Colors.Red;
  static MaskPrefixBullet = "• ";
  static MaskPrefixNumbered = "{0}. ";

  private markdownEnabled = true;
  private linkColor: Color = SkiaRichLabel.ColorLink;
  private codeTextColor: Color = SkiaRichLabel.ColorCode;
  private headingTextColor: Color = SkiaRichLabel.ColorCode;
  private codeBlockBackgroundColor: Color = SkiaRichLabel.ColorCodeBlock;
  private codeBackgroundColor: Color = SkiaRichLabel.ColorCodeBackground;
  private strikeoutColor: Color = SkiaRichLabel.ColorStrikeout;
  private prefixBullet = SkiaRichLabel.MaskPrefixBullet;
  private prefixNumbered = SkiaRichLabel.MaskPrefixNumbered;
  private underlineLink = true;
  private underlineWidth = -1;

  /** Raised when a `[text](url)` span is tapped (C# LinkTapped event). */
  LinkTapped?: (sender: SkiaRichLabel, url: string) => void;

  private Style<K extends keyof this>(key: K, v: this[K]): void { if (this[key] !== v) { this[key] = v; this.RebuildSpans(); } }

  /** false = one literal span, no parsing (unicode/emoji without formatting). */
  get MarkdownEnabled(): boolean { return this.markdownEnabled; }
  set MarkdownEnabled(v: boolean) { this.Style("markdownEnabled" as keyof this, v as this[keyof this]); }
  get LinkColor(): Color { return this.linkColor; }
  set LinkColor(v: Color) { this.Style("linkColor" as keyof this, v as this[keyof this]); }
  get CodeTextColor(): Color { return this.codeTextColor; }
  set CodeTextColor(v: Color) { this.Style("codeTextColor" as keyof this, v as this[keyof this]); }
  get HeadingTextColor(): Color { return this.headingTextColor; }
  set HeadingTextColor(v: Color) { this.Style("headingTextColor" as keyof this, v as this[keyof this]); }
  get CodeBlockBackgroundColor(): Color { return this.codeBlockBackgroundColor; }
  set CodeBlockBackgroundColor(v: Color) { this.Style("codeBlockBackgroundColor" as keyof this, v as this[keyof this]); }
  get CodeBackgroundColor(): Color { return this.codeBackgroundColor; }
  set CodeBackgroundColor(v: Color) { this.Style("codeBackgroundColor" as keyof this, v as this[keyof this]); }
  get StrikeoutColor(): Color { return this.strikeoutColor; }
  set StrikeoutColor(v: Color) { this.Style("strikeoutColor" as keyof this, v as this[keyof this]); }
  get PrefixBullet(): string { return this.prefixBullet; }
  set PrefixBullet(v: string) { this.Style("prefixBullet" as keyof this, v as this[keyof this]); }
  /** `{0}` is replaced by the item number. */
  get PrefixNumbered(): string { return this.prefixNumbered; }
  set PrefixNumbered(v: string) { this.Style("prefixNumbered" as keyof this, v as this[keyof this]); }
  get UnderlineLink(): boolean { return this.underlineLink; }
  set UnderlineLink(v: boolean) { this.Style("underlineLink" as keyof this, v as this[keyof this]); }
  get UnderlineWidth(): number { return this.underlineWidth; }
  set UnderlineWidth(v: number) { this.Style("underlineWidth" as keyof this, v as this[keyof this]); }

  /** The markdown source; spans are rebuilt on every change (C# SetTextInternal). */
  override get Text(): string { return super.Text; }
  override set Text(v: string) { super.Text = v; this.RebuildSpans(); }

  // headings scale from the label's FontSize, so rebuild when it changes
  override get FontSize(): number { return super.FontSize; }
  override set FontSize(v: number) { super.FontSize = v; this.RebuildSpans(); }

  /** C# OnLinkTapped: raises LinkTapped; override to open the url yourself. */
  OnLinkTapped(url: string, _text: string): void { this.LinkTapped?.(this, url); }

  protected override OnSpanTapped(span: TextSpan, args: SkiaGesturesParameters, apply: GestureEventProcessingInfo): SkiaControl | null {
    if (span.Tag) { this.OnLinkTapped(span.Tag, span.Text); return this; }
    return super.OnSpanTapped(span, args, apply);
  }

  // ---- markdown -> spans ----
  private hadParagraph = false;

  private RebuildSpans(): void {
    for (const s of this.Spans) s.Parent = undefined;
    this.Spans.length = 0;
    this.hadParagraph = false;
    const text = super.Text;
    if (text) {
      if (!this.markdownEnabled) this.AddTextSpan(text, SkiaRichLabel.Plain());
      else this.RenderDocument(text);
    }
    this.Update();
  }

  private static Plain(): InlineState { return { bold: false, italic: false, strike: false, heading: 0, code: false }; }

  /** C# AddTextSpan + SpanWithAttributes: one span per fragment carrying the current inline state. */
  private AddTextSpan(text: string, state: InlineState, modify?: (span: TextSpan) => void): TextSpan | undefined {
    if (!text) return undefined;
    const span = new TextSpan();
    span.Parent = this;
    span.Text = text;
    span.IsBold = state.bold;
    span.IsItalic = state.italic;
    span.Strikeout = state.strike;
    if (state.strike) span.StrikeoutColor = this.strikeoutColor;
    if (state.heading === 1) { span.IsBold = true; span.FontSize = this.FontSize + 9; span.TextColor = this.headingTextColor; }
    else if (state.heading === 2) { span.IsBold = true; span.FontSize = this.FontSize + 4; span.TextColor = this.headingTextColor; }
    else if (state.heading === 3) { span.IsBold = true; span.FontSize = this.FontSize + 2; span.TextColor = this.headingTextColor; }
    modify?.(span);
    this.Spans.push(span);
    return span;
  }

  /** C# RenderInline(LineBreak): a "\n" appended to the last span (or a span of its own). */
  private LineBreak(): void {
    const last = this.Spans[this.Spans.length - 1];
    if (last) last.Text += "\n";
    else this.AddTextSpan("\n", SkiaRichLabel.Plain());
  }

  private RenderDocument(text: string): void {
    const lines = text.replace(/\r\n?/g, "\n").split("\n");
    let i = 0;
    const paragraph: string[] = [];
    const flushParagraph = (prefix?: string) => {
      if (paragraph.length === 0) return;
      this.BeginBlock();
      if (prefix) this.AddTextSpan(prefix, SkiaRichLabel.Plain());
      this.RenderInlines(paragraph.join("\n"), SkiaRichLabel.Plain());
      paragraph.length = 0;
    };
    while (i < lines.length) {
      const line = lines[i];
      const fence = /^\s*```/.exec(line);
      const heading = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
      const bullet = /^\s*[-*+]\s+(.*)$/.exec(line);
      const numbered = /^\s*(\d+)[.)]\s+(.*)$/.exec(line);
      if (fence) {
        flushParagraph();
        const code: string[] = [];
        i++;
        while (i < lines.length && !/^\s*```/.test(lines[i])) code.push(lines[i++]);
        i++; // closing fence
        this.BeginBlock();
        code.forEach((l, n) => {
          if (n > 0) this.LineBreak();
          this.AddTextSpan(l, { ...SkiaRichLabel.Plain(), code: true }, (s) => { s.TextColor = this.codeTextColor; s.BackgroundColor = this.codeBlockBackgroundColor; });
        });
        continue;
      }
      if (heading) {
        flushParagraph();
        this.BeginBlock();
        const level = Math.min(3, heading[1].length) as 1 | 2 | 3;
        this.RenderInlines(heading[2], { ...SkiaRichLabel.Plain(), heading: level });
        i++;
        continue;
      }
      if (bullet || numbered) {
        flushParagraph();
        let itemNumber = 1;
        while (i < lines.length) {
          const b = /^\s*[-*+]\s+(.*)$/.exec(lines[i]);
          const n = /^\s*(\d+)[.)]\s+(.*)$/.exec(lines[i]);
          if (!b && !n) break;
          this.BeginBlock();
          this.AddTextSpan(b ? this.prefixBullet : this.prefixNumbered.replace("{0}", String(n ? itemNumber++ : 0)), SkiaRichLabel.Plain());
          this.RenderInlines(b ? b[1] : n![2], SkiaRichLabel.Plain());
          i++;
          // continuation lines of the same item (indented, not blank, not a new item)
          while (i < lines.length && /^\s+\S/.test(lines[i]) && !/^\s*([-*+]|\d+[.)])\s+/.test(lines[i])) { this.LineBreak(); this.RenderInlines(lines[i].trim(), SkiaRichLabel.Plain()); i++; }
        }
        continue;
      }
      if (line.trim() === "") { flushParagraph(); i++; continue; }
      paragraph.push(line);
      i++;
    }
    flushParagraph();
  }

  /** Blocks are separated by one line break once something was emitted (C# hadParagraph). */
  private BeginBlock(): void {
    if (this.hadParagraph) this.LineBreak();
    this.hadParagraph = true;
  }

  /** Inline markdown: `code`, **bold**, __bold__, *italic*, _italic_, ~~strike~~, [text](url), backslash escapes. */
  private RenderInlines(text: string, state: InlineState): void {
    let literal = "";
    const flush = () => { if (literal) { this.AddTextSpan(literal, state); literal = ""; } };
    let i = 0;
    const n = text.length;
    const closes = (marker: string, from: number): number => {
      let k = text.indexOf(marker, from);
      while (k > 0 && text[k - 1] === "\\") k = text.indexOf(marker, k + 1);
      return k;
    };
    while (i < n) {
      const ch = text[i];
      if (ch === "\\" && i + 1 < n) { literal += text[i + 1]; i += 2; continue; }
      if (ch === "`") {
        const end = text.indexOf("`", i + 1);
        if (end > i) { flush(); this.AddTextSpan(text.slice(i + 1, end), { ...state, code: true }, (s) => { s.TextColor = this.codeTextColor; s.BackgroundColor = this.codeBackgroundColor; }); i = end + 1; continue; }
      }
      if (ch === "[") {
        const m = /^\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/.exec(text.slice(i));
        if (m) {
          flush();
          this.AddTextSpan(m[1] || m[2], state, (s) => { s.Tag = m[2]; s.TextColor = this.linkColor; s.Underline = this.underlineLink; s.UnderlineWidth = this.underlineWidth; s.ForceCaptureInput = true; });
          i += m[0].length; continue;
        }
      }
      const two = text.slice(i, i + 2);
      if (two === "**" || two === "__" || two === "~~") {
        const end = closes(two, i + 2);
        if (end > i + 2) {
          flush();
          const inner = text.slice(i + 2, end);
          this.RenderInlines(inner, two === "~~" ? { ...state, strike: true } : { ...state, bold: true });
          i = end + 2; continue;
        }
      }
      if ((ch === "*" || ch === "_") && i + 1 < n && !/\s/.test(text[i + 1])) {
        const end = closes(ch, i + 1);
        if (end > i + 1 && !/\s/.test(text[end - 1])) {
          flush();
          this.RenderInlines(text.slice(i + 1, end), { ...state, italic: true });
          i = end + 1; continue;
        }
      }
      literal += ch;
      i++;
    }
    flush();
  }
}
