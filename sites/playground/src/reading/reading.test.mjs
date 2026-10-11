import { strict as assert } from "node:assert";
import { test } from "node:test";

import { NO_ANSWERS, readingOf } from "./reading.ts";

const tree = { identity: "playground.nx", nodes: [], declarations: [] };

test("nothing is stale before the first tree", () => {
  assert.deepEqual(readingOf(NO_ANSWERS, "a"), { tree: null, text: null, stale: false, failure: null });
});

test("a tree of other text is stale, and of the same text is not", () => {
  const answers = { tree, text: "a", failure: null };
  assert.equal(readingOf(answers, "b").stale, true);
  // Back to the shown text after a cancelled query: current again, with nothing to clear.
  assert.equal(readingOf(answers, "a").stale, false);
});

test("a failure is shown for its own source, with the older tree marked stale", () => {
  const answers = { tree, text: "a", failure: { source: "b", message: "worker crashed" } };
  assert.deepEqual(readingOf(answers, "b"), { tree, text: "a", stale: true, failure: "worker crashed" });
  assert.equal(readingOf(answers, "a").failure, null);
});
