import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { checkFiles, findNxBlocks, SYNTAX_CODES } from "./check-code-blocks.mjs";

const fixtures = join(dirname(fileURLToPath(import.meta.url)), "fixtures");
let host;

before(async () => {
  host = createNxHost(await loadNxModule());
});

function check(name) {
  return checkFiles(host, [join(fixtures, name)], fixtures).failures;
}

test("a complete example compiles", () => {
  assert.deepEqual(check("complete.md"), []);
});

test("a stale example fails, naming the page and the block's line", () => {
  const failures = check("stale.md");
  assert.equal(failures.length, 1);
  assert.match(failures[0], /^stale\.md:8:1: syntax-error: /);
});

test("a fragment is checked for syntax only", () => {
  assert.deepEqual(check("fragment.mdx"), []);
});

test("a fragment with a syntax error fails", () => {
  assert.match(check("fragment-syntax.md")[0], /^fragment-syntax\.md:2:1: syntax-error: /);
});

test("an example of an error must still be an error", () => {
  assert.deepEqual(check("invalid.md"), [
    "invalid.md:9:1: an `nx invalid` block compiles without errors"
  ]);
});

test("fence words other than the kinds are ignored, and two kinds are refused", () => {
  const [titled, both] = findNxBlocks('```nx title="a.nx"\nlet x = 1\n```\n\n```nx fragment invalid\n```\n');
  assert.equal(titled.kind, "complete");
  assert.deepEqual(both.kinds, ["fragment", "invalid"]);
});

test("the syntax codes are read from the syntax crate", () => {
  assert.ok(SYNTAX_CODES.has("syntax-error"));
  assert.ok(!SYNTAX_CODES.has("value-type-mismatch"));
});
