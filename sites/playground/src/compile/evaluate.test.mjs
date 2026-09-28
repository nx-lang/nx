/**
 * The output cap: where a value too long to show is cut.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { MAX_OUTPUT_CHARACTERS, cutPoint, valueOutcome } from "./evaluate.ts";

test("a value within the limit is whole", () => {
  const value = { text: "1\n2", nodes: [] };
  assert.deepEqual(valueOutcome(value), { kind: "value", value, truncated: false });
});

test("a long value is cut after its last whole line", () => {
  const text = Array.from({ length: 20000 }, (_, i) => `line ${i}`).join("\n");
  const cut = cutPoint(text);
  assert.ok(cut <= MAX_OUTPUT_CHARACTERS);
  assert.equal(text[cut], "\n");
});

test("one long line is cut at the limit, never between a surrogate pair's halves", () => {
  const plain = "a".repeat(MAX_OUTPUT_CHARACTERS + 10);
  assert.equal(cutPoint(plain), MAX_OUTPUT_CHARACTERS);
  const emoji = "a".repeat(MAX_OUTPUT_CHARACTERS - 1) + "🎉" + "a".repeat(10);
  assert.equal(cutPoint(emoji), MAX_OUTPUT_CHARACTERS - 1);
  const { value } = valueOutcome({ text: emoji, nodes: [{ start: 0, end: emoji.length, role: "scalar", type: "string" }] });
  assert.equal(value.text.length, MAX_OUTPUT_CHARACTERS - 1);
  assert.equal(value.nodes[0].end, MAX_OUTPUT_CHARACTERS - 1);
});
