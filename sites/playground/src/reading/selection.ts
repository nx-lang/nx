import type { SourceTree, TextPosition } from "@nx-lang/language-protocol";
import { positionAt } from "../editor/positions.ts";

/**
 * Carrying a selection between the editor and the reading view: the editor speaks UTF-16 offsets
 * into its text, and the reading view speaks the keys of source tree nodes.
 */

function compare(a: TextPosition, b: TextPosition): number {
  return a.line - b.line || a.character - b.character;
}

/**
 * The key of the smallest node of `tree` that holds the UTF-16 `offset` into `source`, the text
 * the tree was computed from, or undefined when the offset is between nodes. A cursor just after
 * a word is still in it.
 */
export function keyAtOffset(tree: SourceTree, source: string, offset: number): string | undefined {
  const cursor = positionAt(source, offset);
  let found: { key: string; size: number } | undefined;
  for (const node of tree.nodes) {
    if (compare(node.range.start, cursor) > 0 || compare(cursor, node.range.end) > 0) {
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

/** The UTF-16 offset into `source` where the node with `key` starts, or undefined. */
export function offsetOfKey(tree: SourceTree, source: string, key: string): number | undefined {
  const node = tree.nodes.find((candidate) => candidate.key === key);
  if (node === undefined) {
    return undefined;
  }
  let offset = 0;
  for (let line = 0; line < node.range.start.line; line += 1) {
    const next = source.indexOf("\n", offset);
    if (next === -1) {
      return source.length;
    }
    offset = next + 1;
  }
  return Math.min(offset + node.range.start.character, source.length);
}
