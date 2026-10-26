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

/**
 * The key of what a change to the text from the UTF-16 offset `start` to `end` in `source` touched,
 * as a reader selects it: the property or the element holding that text, or what holds the place
 * of text the change took out. Undefined when the change lies outside every node, as between
 * declarations.
 */
export function keyOfChange(tree: SourceTree, source: string, start: number, end: number = start): string | undefined {
  // Where text was taken out, the place is a point between what is left: what holds it is what lost
  // the text, never the node that ends just before it, as a cursor there would be in.
  const key = start === end ? keyAround(tree, source, start) : keyAtRange(tree, source, start, end);
  let at = key === undefined ? undefined : tree.nodes.findIndex((node) => node.key === key);
  while (at !== undefined && at >= 0) {
    const node = tree.nodes[at]!;
    if (node.role === "attribute" || node.role === "element") {
      return node.key;
    }
    at = node.parent;
  }
  return key;
}

/** The key of the smallest node of `tree` that holds the UTF-16 offset `at` strictly inside it. */
function keyAround(tree: SourceTree, source: string, at: number): string | undefined {
  let found: { key: string; size: number } | undefined;
  for (const node of tree.nodes) {
    const start = offsetAt(source, node.range.start);
    const end = offsetAt(source, node.range.end);
    if (start < at && at < end && (found === undefined || end - start <= found.size)) {
      found = { key: node.key, size: end - start };
    }
  }
  return found?.key;
}

/**
 * Where a set of changes made at once, as one undo step makes them, lies in the text they left: from
 * the first change's start to the last one's end, in UTF-16 offsets. Each change is given as an
 * editor reports it, by its offset and length in the text before any of them, and the text that
 * replaced that. Undefined for no changes.
 */
export function spanOfChanges(
  changes: readonly { rangeOffset: number; rangeLength: number; text: string }[]
): { start: number; end: number } | undefined {
  let span: { start: number; end: number } | undefined;
  let shift = 0;
  for (const change of [...changes].sort((a, b) => a.rangeOffset - b.rangeOffset)) {
    const start = change.rangeOffset + shift;
    const end = start + change.text.length;
    span = span === undefined ? { start, end } : { start: span.start, end };
    shift += change.text.length - change.rangeLength;
  }
  return span;
}
