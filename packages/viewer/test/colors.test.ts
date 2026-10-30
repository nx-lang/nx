import assert from "node:assert/strict";
import { test } from "node:test";
import { preserveHexAlpha } from "../src/editors.js";

test("RGB picker preserves alpha, including shorthand and fully transparent colors", () => {
  assert.equal(preserveHexAlpha("#123456", "#f008"), "#12345688");
  assert.equal(preserveHexAlpha("#123456", "#33669980"), "#12345680");
  assert.equal(preserveHexAlpha("#123456", "#fff0"), "#12345600");
  assert.equal(preserveHexAlpha("#123456", "#ffffff"), "#123456");
});
