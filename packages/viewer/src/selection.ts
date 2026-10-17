/**
 * Carrying a selection between a host's editor and the reading view: the editor speaks UTF-16
 * offsets into its text, the way JavaScript strings and Monaco count, and the reading view speaks
 * the keys of source tree nodes.
 */
import type { SourceTree, TextPosition } from "@nx-lang/language-protocol";

function compare(a: TextPosition, b: TextPosition): number {
  return a.line - b.line || a.character - b.character;
}

/** A 0-based line and UTF-16 character for a UTF-16 offset into `source`. */
function positionAt(source: string, offset: number): TextPosition {
  const lineStart = source.slice(0, offset).lastIndexOf("\n") + 1;
  let line = 0;
  for (let index = 0; index < lineStart; index += 1) {
    if (source.charCodeAt(index) === 10) {
      line += 1;
    }
  }
  return { line, character: offset - lineStart };
}

/** The UTF-16 offset into `source` of a 0-based line and UTF-16 character. */
function offsetAt(source: string, position: TextPosition): number {
  let offset = 0;
  for (let line = 0; line < position.line; line += 1) {
    const next = source.indexOf("\n", offset);
    if (next === -1) {
      return source.length;
    }
    offset = next + 1;
  }
  return Math.min(offset + position.character, source.length);
}

/**
 * The key of the smallest node of `tree` that holds the text from the UTF-16 offset `start` to
 * `end` in `source`, the text the tree was computed from, or undefined when that text is between
 * nodes. Whitespace at either end of the range is left out, so a selection of whole lines finds
 * what they hold. Without `end`, the range is a cursor, and a cursor just after a word is still in
 * it.
 */
export function keyAtRange(tree: SourceTree, source: string, start: number, end: number = start): string | undefined {
  let from = Math.min(start, end);
  let to = Math.max(start, end);
  while (from < to && /\s/.test(source[from]!)) {
    from += 1;
  }
  while (to > from && /\s/.test(source[to - 1]!)) {
    to -= 1;
  }
  const first = positionAt(source, from);
  const last = positionAt(source, to);
  let found: { key: string; size: number } | undefined;
  for (const node of tree.nodes) {
    if (compare(node.range.start, first) > 0 || compare(last, node.range.end) > 0) {
      continue;
    }
    const size = node.range.endByte - node.range.startByte;
    // Children come after their parents, so a later node of the same size is the deeper one.
    if (found === undefined || size <= found.size) {
      found = { key: node.key, size };
    }
  }
  return found?.key;
}

/** The UTF-16 offsets into `source` where the node with `key` starts and ends, or undefined. */
export function rangeOfKey(tree: SourceTree, source: string, key: string): { start: number; end: number } | undefined {
  const node = tree.nodes.find((candidate) => candidate.key === key);
  if (node === undefined) {
    return undefined;
  }
  return { start: offsetAt(source, node.range.start), end: offsetAt(source, node.range.end) };
}
