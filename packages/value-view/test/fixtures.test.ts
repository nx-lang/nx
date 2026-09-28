import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

// @ts-expect-error: a plain script, not part of the package's typed sources.
import { evaluateFixtures } from "../../scripts/update-fixtures.mjs";

test("the committed fixtures are what evaluateNx() returns", async () => {
  const committed = JSON.parse(readFileSync(new URL("../../test/fixtures/values.json", import.meta.url), "utf8"));
  assert.deepEqual(
    await evaluateFixtures(),
    committed,
    "the compiler's output changed; run `pnpm run update-fixtures` and review the diff"
  );
});
