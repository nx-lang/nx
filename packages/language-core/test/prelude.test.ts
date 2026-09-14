import assert from "node:assert/strict";
import { test } from "node:test";

import { preludeOffsets, shiftPositionIn, shiftRangeOut, withPrelude } from "../src/prelude.js";

test("prelude offsets add a separating line and shift only lines and bytes", () => {
  const withNewline = preludeOffsets("let a = 1\n");
  assert.equal(withNewline.text, "let a = 1\n\n");
  assert.equal(withNewline.lines, 2);
  assert.equal(withNewline.bytes, 11);

  const withoutNewline = preludeOffsets("let a = 1");
  assert.deepEqual(withoutNewline, withNewline);

  const emoji = preludeOffsets('let s = "😀"');
  assert.equal(emoji.lines, 2);
  // Measured in UTF-8 bytes, not UTF-16 code units: the emoji is four bytes, two units.
  assert.equal(emoji.bytes, new TextEncoder().encode('let s = "😀"\n\n').byteLength);
  assert.equal(emoji.bytes, 'let s = "😀"\n\n'.length + 2);

  assert.deepEqual(shiftPositionIn({ line: 3, character: 7 }, withNewline), { line: 5, character: 7 });
  assert.deepEqual(
    shiftRangeOut({ start: { line: 2, character: 4 }, end: { line: 2, character: 9 }, startByte: 15, endByte: 20 }, withNewline),
    { start: { line: 0, character: 4 }, end: { line: 0, character: 9 }, startByte: 4, endByte: 9 },
  );
  assert.equal(
    shiftRangeOut({ start: { line: 0, character: 4 }, end: { line: 0, character: 5 }, startByte: 4, endByte: 5 }, withNewline),
    null,
  );
});

test("the document's first line is its own whatever the prelude ended with", () => {
  for (const source of ["let a = 1", "let a = 1\n"]) {
    const offsets = preludeOffsets(source);
    const combined = withPrelude(offsets, "<Panel />\n");
    assert.equal(combined.split("\n")[offsets.lines], "<Panel />");
  }
});
