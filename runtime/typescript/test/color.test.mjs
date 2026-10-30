import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { nxStandardColor, nxStandardColorNames } from "../dist/src/color.js";
const corpus = JSON.parse(readFileSync(new URL("../../../crates/nx-value/src/color-cases.json", import.meta.url), "utf8"));
const call = (name, args) => nxStandardColor(nxStandardColorNames.indexOf(name), args);
function compare(actual, expected, context) {
  if (typeof expected === "number") assert.ok(Math.abs(actual - expected) <= 2e-6, `${context}: ${actual} vs ${expected}`);
  else if (expected !== null && typeof expected === "object") {
    for (const key of Object.keys(expected)) compare(actual[key], expected[key], context);
  } else assert.equal(actual, expected, context);
}
for (const [index, entry] of corpus.cases.entries()) {
  const args = entry.args.map(value => value?.color ? call("parse", [value.color]) : value);
  const context = `case ${index}: ${entry.op}`;
  if (entry.error) {
    assert.throws(() => call(entry.op, args), error => error.code === entry.error, context);
    continue;
  }
  let actual = call(entry.op, args);
  if (Array.isArray(entry.expected)) actual = [actual.r, actual.g, actual.b, actual.a];
  if (entry.exact) assert.deepEqual(actual, entry.expected.map(Math.fround), context);
  else compare(actual, entry.expected, context);
}
for (const channels of [[0.123456789, 0.4, 0.8, 0.5], [1, 0, 0, 0], [1e-30, 0.99, 0, 1]]) {
  const value = call("srgb", channels);
  assert.deepEqual(call("parse", [call("toCss", [value])]), value, "CSS preserves exact float32 components");
}
console.log(`${corpus.cases.length} standard color reference cases passed in TypeScript.`);
