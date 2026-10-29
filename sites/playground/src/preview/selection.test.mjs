import { test } from "node:test";
import assert from "node:assert/strict";
import { authoredSites, selectedTargets } from "./selection.ts";
const module = "playground.nx";
const site = (start, end) => ({ module, start, end });
const targets = [
  { id: "Question", sources: [site(10, 90)] },
  { id: "Choice/0", sources: [site(30, 45)] },
  { id: "Choice/1", sources: [site(50, 65)] },
  { id: "Choice/0-copy", sources: [site(30, 45)] },
];
test("generic Question selection includes its rendered choices and repeated output", () => {
  assert.deepEqual(
    selectedTargets(targets, { source: "survey", start: 10, end: 90 }, module),
    targets.map((t) => t.id),
  );
  assert.deepEqual(
    selectedTargets(targets, { source: "survey", start: 30, end: 45 }, module),
    ["Choice/0", "Choice/0-copy"],
  );
});
test("a selected property selects the nearest enclosing output, while foreign modules do not match", () => {
  assert.deepEqual(
    selectedTargets(targets, { source: "survey", start: 32, end: 36 }, module),
    ["Choice/0", "Choice/0-copy"],
  );
  assert.deepEqual(
    selectedTargets(
      targets,
      { source: "survey", start: 32, end: 36 },
      "other.nx",
    ),
    [],
  );
  assert.deepEqual(selectedTargets(targets, null, module), []);
});
test("authorship is separate from construction and follows the innermost local call site", () => {
  const construction = {
    module: "@lib/lib.nx",
    start: 1,
    end: 9,
    callSites: [
      { module: "@lib/lib.nx", start: 12, end: 15 },
      site(30, 45),
      site(10, 90),
    ],
  };
  assert.deepEqual(authoredSites(construction, module), [
    site(30, 45),
    site(10, 90),
  ]);
  assert.deepEqual(authoredSites(null, module), []);
});
