/**
 * The catalog compile, against the real wasm host under Node.
 *
 * These run against the same module the browser loads, so what the site ships is what is checked.
 * The origin classification and span shifting are `@nx-lang/sdk-wasm`'s and are tested there; what
 * is checked here is that the site's own catalog compiles and the site's input limits hold.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { MAX_SOURCE_BYTES, compileWithCatalog } from "./catalog.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const module = await loadNxModule();
const host = createNxHost(module);

after(() => host.dispose());

/** Compiles against the app's own catalog. */
function compile(source) {
  return compileWithCatalog(host, catalog, source);
}

test("compiles a program that uses the catalog", () => {
  const result = compile('let root() = { <SkiaLabel Text="hi" /> }');
  assert.equal(result.diagnostics.length, 0);
  assert.equal(result.ir.format, "nx-ir-json");
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
