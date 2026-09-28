import type { NxValueNode } from "@nx-lang/value-view";

/**
 * Converting between the positions NX reports and the ones editors use.
 *
 * <para>NX spans give 1-based lines and 1-based columns counted in Unicode scalar values. Monaco and
 * the language protocol count columns in UTF-16 code units, the way JavaScript strings do, so a line
 * holding a character outside the Basic Multilingual Plane puts the two out of step.</para>
 */

/** The UTF-16 offset into `source` of a 1-based line and scalar-value column. */
export function offsetOf(source: string, line: number, column: number): number {
  let offset = 0;
  for (let current = 1; current < line; current += 1) {
    const next = source.indexOf("\n", offset);
    if (next === -1) {
      return source.length;
    }
    offset = next + 1;
  }
  let scalars = 1;
  while (scalars < column && offset < source.length && source[offset] !== "\n") {
    const code = source.codePointAt(offset)!;
    offset += code > 0xffff ? 2 : 1;
    scalars += 1;
  }
  return offset;
}

/** A 0-based line and UTF-16 character for a UTF-16 offset into `source`. */
export function positionAt(source: string, offset: number): { line: number; character: number } {
  const before = source.slice(0, offset);
  const lineStart = before.lastIndexOf("\n") + 1;
  let line = 0;
  for (let index = 0; index < lineStart; index += 1) {
    if (source.charCodeAt(index) === 10) {
      line += 1;
    }
  }
  return { line, character: offset - lineStart };
}

/**
 * The first whole-word occurrence of `name` in `source` between two UTF-16 offsets, as a range of
 * offsets, or null. A declaration's span starts at its keyword (`type`, `let`), so this finds the
 * name the declaration gives, which is what hover answers for and what a reader wants selected.
 */
export function findName(
  source: string,
  name: string,
  start: number,
  end: number
): { start: number; end: number } | null {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(?<![\\p{L}\\p{N}_])${escaped}(?![\\p{L}\\p{N}_])`, "u");
  const match = pattern.exec(source.slice(start, end));
  return match === null ? null : { start: start + match.index, end: start + match.index + name.length };
}

/**
 * Where the name a node's declaration gives sits in `source`: a record's or function's name, a
 * property's, a case's. A declaration's span starts at its keyword, and hover and selection both
 * want the name.
 */
export function declaredName(node: NxValueNode, source: string): { start: number; end: number } | null {
  const span = node.declaration;
  if (span === undefined) {
    return null;
  }
  const start = offsetOf(source, span.startLine, span.startColumn);
  const end = offsetOf(source, span.endLine, span.endColumn);
  const name =
    node.role === "property" || node.role === "function"
      ? node.name
      : node.role === "case"
        ? undefined
        : node.type?.split(".").at(-1);
  if (name !== undefined) {
    return findName(source, name, start, end) ?? { start, end };
  }
  // A case's own span is just the case, so its first word is its name.
  const word = /[\p{L}_][\p{L}\p{N}_]*/u.exec(source.slice(start, end));
  return word === null ? { start, end } : { start: start + word.index, end: start + word.index + word[0].length };
}
