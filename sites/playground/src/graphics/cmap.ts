/** Reads only Unicode cmap formats 4/12 from the supplied pinned TrueType font. No system fallback. */
export function glyphCoverage(
  bytes: Uint8Array,
): (codepoint: number) => boolean {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const u16 = (at: number) => view.getUint16(at, false);
  const u32 = (at: number) => view.getUint32(at, false);
  let cmap = -1;
  for (let i = 0; i < u16(4); i++) {
    const at = 12 + i * 16;
    if (u32(at) === 0x636d6170) {
      cmap = u32(at + 8);
      break;
    }
  }
  if (cmap < 0) {
    throw new Error("The supplied font has no Unicode cmap.");
  }
  const tables: number[] = [];
  for (let i = 0; i < u16(cmap + 2); i++) {
    const at = cmap + 4 + i * 8;
    const platform = u16(at);
    const encoding = u16(at + 2);
    if (
      platform === 0 ||
      (platform === 3 && (encoding === 1 || encoding === 10))
    ) {
      tables.push(cmap + u32(at + 4));
    }
  }
  const table =
    tables.find((at) => u16(at) === 12) ?? tables.find((at) => u16(at) === 4);
  if (table === undefined) {
    throw new Error("The supplied font has no supported Unicode cmap.");
  }
  if (u16(table) === 12) {
    const count = u32(table + 12);
    return (code) => {
      let lo = 0,
        hi = count - 1;
      while (lo <= hi) {
        const mid = (lo + hi) >>> 1;
        const at = table + 16 + mid * 12;
        const start = u32(at);
        const end = u32(at + 4);
        if (code < start) {
          hi = mid - 1;
        } else if (code > end) {
          lo = mid + 1;
        } else {
          return u32(at + 8) + code - start !== 0;
        }
      }
      return false;
    };
  }
  const count = u16(table + 6) / 2;
  const ends = table + 14;
  const starts = ends + count * 2 + 2;
  const deltas = starts + count * 2;
  const ranges = deltas + count * 2;
  return (code) => {
    if (code > 0xffff) {
      return false;
    }
    for (let i = 0; i < count; i++) {
      if (code > u16(ends + i * 2)) {
        continue;
      }
      if (code < u16(starts + i * 2)) {
        return false;
      }
      const delta = u16(deltas + i * 2);
      const range = u16(ranges + i * 2);
      if (range === 0) {
        return ((code + delta) & 0xffff) !== 0;
      }
      const glyph = u16(
        ranges + i * 2 + range + 2 * (code - u16(starts + i * 2)),
      );
      return glyph !== 0 && ((glyph + delta) & 0xffff) !== 0;
    }
    return false;
  };
}
