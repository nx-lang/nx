import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

import { packageRoot } from "./support.js";

test("the published library types are what nxlang typegen @nx/agent generates", () => {
  // The script regenerates the types with the CLI built from this checkout and compares them with
  // `src/generated`, file by file and byte for byte.
  const result = spawnSync("node", ["scripts/generate-library-types.mjs", "--check"], { cwd: packageRoot, encoding: "utf8" });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});
