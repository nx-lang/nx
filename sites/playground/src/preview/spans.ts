/**
 * Spans of the visitor's source from the UTF-8 byte offsets the IR runtime reports, in the form NX
 * diagnostics use: byte offsets, and 1-based lines and columns counted in Unicode scalar values.
 */
import type { NxTextSpan } from "@nx-lang/sdk-wasm";

/** Finds the line and column of any byte offset of one source. */
export class SourceIndex {
  /** The byte offset of each scalar value, and one past the last. */
  readonly #bytes: number[] = [];
  readonly #lines: number[] = [];
  readonly #columns: number[] = [];

  /** The source the offsets index. */
  readonly source: string;

  constructor(source: string) {
    this.source = source;
    let bytes = 0;
    let line = 1;
    let column = 1;
    for (const character of source) {
      this.#bytes.push(bytes);
      this.#lines.push(line);
      this.#columns.push(column);
      const code = character.codePointAt(0)!;
      bytes += code < 0x80 ? 1 : code < 0x800 ? 2 : code < 0x10000 ? 3 : 4;
      if (character === "\n") {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
    }
    this.#bytes.push(bytes);
    this.#lines.push(line);
    this.#columns.push(column);
  }

  /** The span from byte `start` to byte `end`, or null when either is outside the source. */
  span(start: number, end: number): NxTextSpan | null {
    const from = this.#at(start);
    const to = this.#at(end);
    if (from === undefined || to === undefined || end < start) {
      return null;
    }
    return {
      startByte: start,
      endByte: end,
      startLine: this.#lines[from]!,
      startColumn: this.#columns[from]!,
      endLine: this.#lines[to]!,
      endColumn: this.#columns[to]!
    };
  }

  /** The index of the scalar value starting at `byte`. */
  #at(byte: number): number | undefined {
    let low = 0;
    let high = this.#bytes.length - 1;
    while (low <= high) {
      const middle = (low + high) >>> 1;
      const value = this.#bytes[middle]!;
      if (value === byte) {
        return middle;
      }
      if (value < byte) {
        low = middle + 1;
      } else {
        high = middle - 1;
      }
    }
    return undefined;
  }
}
