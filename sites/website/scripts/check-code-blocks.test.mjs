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

test("output shown after a block is checked against what the block evaluates to", () => {
  assert.deepEqual(check("output.md"), []);
});

test("stale output fails, naming the page and the output block's line, and showing both texts", () => {
  const failures = check("output-stale.md");
  assert.equal(failures.length, 1);
  assert.match(failures[0], /^output-stale\.md:12:1: the output shown is not what the block before it evaluates to/);
  assert.match(failures[0], /--- shown\n<User id="1" name="Grace" \/>/);
  assert.match(failures[0], /--- evaluated\n<User id="1" name="Ada" \/>/);
});

test("output with no unmarked block before it fails", () => {
  const failures = check("output-orphan.md");
  assert.deepEqual(
    failures.map((failure) => failure.split(": ")[0]),
    ["output-orphan.md:5:1", "output-orphan.md:13:1", "output-orphan.md:21:1"]
  );
  assert.ok(failures.every((failure) => failure.includes("must follow an unmarked `nx` block")));
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
