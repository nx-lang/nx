import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compile, compileWithCatalog, MAX_SOURCE_BYTES } from "./compile.mjs";

test("compiles a program that uses the catalog", () => {
  const result = compile('let root() = { <SkiaLabel Text="hi" /> }');
  assert.equal(result.diagnostics.length, 0);
  assert.equal(result.ir.format, "nx-ir-json");
});

test("reports a source diagnostic at the visitor's own line and column", () => {
  const result = compile('let root() = {\n  <SkiaLabel Text=1.0 />\n}');
  assert.equal(result.ir, null);
  assert.equal(result.diagnostics.length, 1);
  const [diagnostic] = result.diagnostics;
  assert.equal(diagnostic.origin, "source");
  // Line 2, column 14 is the `T` of `Text` — the catalog leads, so the span is shifted back.
  assert.equal(diagnostic.span.startLine, 2);
  assert.equal(diagnostic.span.startColumn, 14);
});

test("compiles a source file that is a single trailing element", () => {
  const result = compile("<SkiaLayer VerticalOptions=Fill>\n</SkiaLayer>\n");
  assert.deepEqual(result.diagnostics, []);
  assert.equal(result.ir.format, "nx-ir-json");
});

test("compiles a trailing element that has children", () => {
  const result = compile('<SkiaLayer>\n  <SkiaLabel Text="hi" />\n</SkiaLayer>\n');
  assert.deepEqual(result.diagnostics, []);
  assert.equal(result.ir.format, "nx-ir-json");
});

const catalogSource = readFileSync(new URL("../catalog/skia.nx", import.meta.url), "utf8");

test("attributes a diagnostic inside the catalog to the catalog", () => {
  const broken = `${catalogSource}\nexternal component <Broken\n`;
  const result = compileWithCatalog(broken, 'let root() = { <SkiaLabel Text="hi" /> }');
  assert.equal(result.ir, null);
  assert.ok(result.diagnostics.length > 0);
  for (const diagnostic of result.diagnostics) {
    assert.equal(diagnostic.origin, "catalog");
    // A catalog fault is never marked in the editor, so it carries no position.
    assert.equal(diagnostic.span, null);
  }
});

test("reports a whole-program failure without a position", () => {
  const broken = `${catalogSource}\nexternal component <Broken value: NoSuchType? />\n`;
  const result = compileWithCatalog(broken, 'let root() = { <SkiaLabel Text="hi" /> }');
  assert.equal(result.ir, null);
  assert.ok(result.diagnostics.length > 0);
  for (const diagnostic of result.diagnostics) {
    assert.equal(diagnostic.origin, "program");
    assert.equal(diagnostic.span, null);
  }
});

test("marks an insertion point, which the compiler reports as an empty span", () => {
  // `Expected } here` names a column and no width. That is a position the author can act on, so it
  // must survive as one rather than be mistaken for a whole-program fault.
  const result = compile('let root() = { <SkiaLabel Text="hi" />');
  assert.equal(result.ir, null);
  assert.equal(result.diagnostics.length, 1);
  const [diagnostic] = result.diagnostics;
  assert.equal(diagnostic.origin, "source");
  assert.equal(diagnostic.span.startLine, 1);
  assert.equal(diagnostic.span.startColumn, diagnostic.span.endColumn);
});

test("answers a stray delimiter at the end of the source", () => {
  // The external scanner used to spin forever on a delimiter with nothing after it, which took the
  // whole single-process service with it. Each of these must come back as a diagnostic.
  for (const source of ["@", "let x = @", "<Doc:string>text@", "<Doc>text&"]) {
    const result = compile(source);
    assert.equal(result.ir, null, `${source} should not compile`);
    assert.ok(result.diagnostics.length > 0, `${source} should report a diagnostic`);
  }
});

test("rejects source larger than the limit", () => {
  assert.throws(() => compile("x".repeat(MAX_SOURCE_BYTES + 1)), RangeError);
});

test("rejects a non-string source", () => {
  assert.throws(() => compile({ not: "source" }), TypeError);
});
