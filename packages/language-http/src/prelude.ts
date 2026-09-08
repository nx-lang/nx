import type { EditorRange, TextPosition } from "@nx-lang/language-protocol";

/**
 * Where a document's own text begins once a prelude is placed ahead of it.
 *
 * `text` is the prelude as it is actually prepended: ending in a newline, with one blank line after
 * it, so the document's first line is always its own line whatever the prelude ended with. `lines`
 * and `bytes` are the size of that text, and are what every shift adds or subtracts.
 */
export interface PreludeOffsets {
  text: string;
  lines: number;
  bytes: number;
}

/** Computes the offsets a prelude introduces. */
export function preludeOffsets(preludeSource: string): PreludeOffsets {
  const normalized = preludeSource.endsWith("\n") ? preludeSource : `${preludeSource}\n`;
  const text = `${normalized}\n`;
  let lines = 0;
  for (const character of text) {
    if (character === "\n") {
      lines += 1;
    }
  }
  return { text, lines, bytes: Buffer.byteLength(text, "utf8") };
}

/** Places the prelude ahead of `source`. */
export function withPrelude(offsets: PreludeOffsets, source: string): string {
  return `${offsets.text}${source}`;
}

/**
 * Moves a position from the document's coordinates into the combined text.
 *
 * Only the line moves: a column is relative to its line's start and the prelude contributes whole
 * lines.
 */
export function shiftPositionIn(position: TextPosition, offsets: PreludeOffsets): TextPosition {
  return { line: position.line + offsets.lines, character: position.character };
}

/**
 * Moves a range from the combined text back into the document's coordinates, or returns `null`
 * when the range starts inside the prelude and so belongs to no document position.
 */
export function shiftRangeOut(range: EditorRange, offsets: PreludeOffsets): EditorRange | null {
  if (range.start.line < offsets.lines) {
    return null;
  }
  return {
    start: { line: range.start.line - offsets.lines, character: range.start.character },
    end: { line: range.end.line - offsets.lines, character: range.end.character },
    startByte: range.startByte - offsets.bytes,
    endByte: range.endByte - offsets.bytes,
  };
}
