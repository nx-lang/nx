import type { NxCanonicalValue } from "@nx-lang/ir-runtime";

/**
 * The reference tokens of a JSON pointer (RFC 6901), unescaped, or `undefined` when the text is no
 * pointer: one that is not empty must start with `/`, and `~` must be followed by `0` or `1`.
 */
export function parsePointer(pointer: string): string[] | undefined {
  if (pointer === "") {
    return [];
  }
  if (!pointer.startsWith("/")) {
    return undefined;
  }
  const tokens: string[] = [];
  for (const raw of pointer.slice(1).split("/")) {
    if (/~(?![01])/.test(raw)) {
      return undefined;
    }
    tokens.push(raw.replaceAll("~1", "/").replaceAll("~0", "~"));
  }
  return tokens;
}

/** A reference token escaped for a JSON pointer. */
export function escapePointerToken(token: string): string {
  return token.replaceAll("~", "~0").replaceAll("/", "~1");
}

/**
 * The value at `pointer` in `value`, or `undefined` when the pointer is malformed or names nothing.
 * An array index is the decimal digits of an item's index, with no leading zero; `-`, which names
 * the item past the end, names nothing here.
 */
export function readPointer(value: NxCanonicalValue, pointer: string): NxCanonicalValue | undefined {
  const tokens = parsePointer(pointer);
  if (tokens === undefined) {
    return undefined;
  }
  let current: NxCanonicalValue | undefined = value;
  for (const token of tokens) {
    if (Array.isArray(current)) {
      if (!/^(?:0|[1-9][0-9]*)$/.test(token)) {
        return undefined;
      }
      current = (current as readonly NxCanonicalValue[])[Number(token)];
    } else if (current !== null && typeof current === "object") {
      const record = current as { readonly [key: string]: NxCanonicalValue };
      current = Object.hasOwn(record, token) ? record[token] : undefined;
    } else {
      return undefined;
    }
    if (current === undefined) {
      return undefined;
    }
  }
  return current;
}
