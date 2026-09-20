/**
 * The catalog compile, against the real wasm host under Node.
 *
 * These run against the same module the browser loads, so what the site ships is what is checked:
 * that the site's own catalog compiles as a module, that a document reaches it without an import
 * line and is emitted without it, that a diagnostic is classified by the module it belongs to, and
 * that the input limits hold.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { prepareNxIrModule } from "@nx-lang/ir-runtime";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import {
  CATALOG_IDENTITY,
  MAX_SOURCE_BYTES,
  PRELUDE_IDENTITY,
  classifyDiagnostic,
  compileWithCatalog,
  emitCatalogArtifact,
} from "./catalog.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const module = await loadNxModule();
const host = createNxHost(module);

after(() => host.dispose());

/** Compiles against the app's own catalog. */
function compile(source) {
  return compileWithCatalog(host, catalog, source);
}

/** The opened image of an artifact: its module table and strings, as the runtime reads them. */
function opened(image) {
  const { artifact } = prepareNxIrModule(image);
  return {
    modules: artifact.modules,
    strings: Array.from({ length: artifact.stringCount }, (_, index) => artifact.string(index)),
    hasDebug: artifact.hasDebug,
  };
}

test("compiles a program that uses the catalog to the snippet's artifact alone", () => {
  const result = compile('let root() = { <SkiaLabel Text="hi" /> }');
  assert.equal(result.diagnostics.length, 0);
  assert.ok(result.ir instanceof Uint8Array);
  assert.equal(new TextDecoder().decode(result.ir.subarray(0, 4)), "NXIR");
  const ir = opened(result.ir);
  // The visitor's module, naming the catalog in its table and carrying none of its declarations.
  assert.deepEqual(ir.modules.map((module) => module.identity), ["playground.nx", CATALOG_IDENTITY]);
  assert.equal(ir.strings.includes("SkiaLayer"), false, "the snippet carries none of the catalog");
  assert.equal(ir.hasDebug, false, "the snippet carries no debug section");
  assert.ok(result.ir.byteLength < 4 * 1024, "a one-line snippet's artifact is a few kilobytes");
});

test("emits the catalog's own artifact, the one a compiled snippet names", () => {
  const artifact = opened(emitCatalogArtifact(host, catalog));
  assert.deepEqual(artifact.modules.map((module) => module.identity), [CATALOG_IDENTITY]);
  assert.ok(artifact.strings.includes("SkiaLayer"), "the catalog's declarations are here");
  // The same catalog text on both sides, so the snippet's table names exactly this module.
  const snippet = opened(compile('let root() = { <SkiaLabel Text="hi" /> }').ir);
  assert.deepEqual(snippet.modules[1], artifact.modules[0]);
});

test("exports every declaration the catalog holds", () => {
  // The catalog is reached through an implicit import, which sees only exports: a declaration
  // generated without `export` is a name no document can use, and nothing else would fail.
  const unexported = catalog
    .split("\n")
    .filter((line) => /^(abstract |external |type |let |component )/.test(line));
  assert.deepEqual(unexported, [], "every top-level declaration in catalog/skia.nx must be exported");
});

test("reports a fault in the visitor's document in its own coordinates", () => {
  const result = compile('let root() = {\n  <SkiaLabel Text=1.0 />\n}');
  assert.equal(result.ir, null);
  assert.equal(result.diagnostics.length, 1);
  assert.equal(result.diagnostics[0].origin, "source");
  assert.equal(result.diagnostics[0].span.startLine, 2);
});

test("compiles a source file that is a single trailing element", () => {
  const result = compile("<SkiaLayer VerticalOptions=Fill>\n</SkiaLayer>\n");
  assert.deepEqual(result.diagnostics, []);
  assert.ok(result.ir instanceof Uint8Array);
});

test("compiles a trailing element that has children", () => {
  const result = compile('<SkiaLayer>\n  <SkiaLabel Text="hi" />\n</SkiaLayer>\n');
  assert.deepEqual(result.diagnostics, []);
  assert.ok(result.ir instanceof Uint8Array);
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

test("binds a catalog event as a handler property, inherited events included", () => {
  // `Tapped` is declared on the abstract SkiaControl; SkiaButton reaches it through `extends`.
  const result = compile(`component <Page /> = {
  state { taps:int = 0 }
  <SkiaButton Text="Tap" onTapped=<Update taps={taps + 1} /> />
}
let root() = { <Page /> }`);
  assert.deepEqual(result.diagnostics.map((d) => d.message), []);
});

test("types an event's primitive parameter as a payload field", () => {
  const result = compile(`component <Page /> = {
  state { on:boolean = false }
  <SkiaSwitch onToggled=<Update on={action.value} /> />
}
let root() = { <Page /> }`);
  assert.deepEqual(result.diagnostics.map((d) => d.message), []);
  const mistyped = compile(`component <Page /> = {
  state { label:string = "" }
  <SkiaSwitch onToggled=<Update label={action.value} /> />
}
let root() = { <Page /> }`);
  assert.equal(mistyped.ir, null, "a boolean payload field does not fill a string");
});

test("rejects a handler for an event the control does not have, naming the property", () => {
  const result = compile(`action Log = { }
component <Page /> = { <SkiaButton Text="Tap" onNope=<Log /> /> }
let root() = { <Page /> }`);
  assert.equal(result.ir, null);
  assert.ok(result.diagnostics.some((d) => d.message.includes("onNope")), result.diagnostics.map((d) => d.message).join("\n"));
});

// ------------------------------------------------------------------------------------------------
// The prelude and range expressions
// ------------------------------------------------------------------------------------------------

test("a label inside the prelude is not handed to the editor as the visitor's own span", () => {
  const classified = classifyDiagnostic({
    severity: "error",
    code: "some-code",
    message: "points at a built-in's declaration",
    labels: [
      {
        file: PRELUDE_IDENTITY,
        primary: true,
        span: { startByte: 0, endByte: 5, startLine: 3, startColumn: 3, endLine: 3, endColumn: 8 },
      },
    ],
  });

  assert.equal(classified.origin, "program");
  assert.equal(classified.span, null);
});

test("compiles a range loop against the site's own catalog, with the resolver unchanged", () => {
  const result = compile("let root() = { for i in 0..5 { <SkiaLabel Text=\"star\" /> } }");
  assert.deepEqual(result.diagnostics, []);
  assert.ok(result.ir instanceof Uint8Array);
  // The prelude is in the module table; the renderer's resolver never has to know about it, since
  // the IR runtime supplies the prelude itself.
  const ir = opened(result.ir);
  assert.ok(
    ir.modules.some((module) => module.identity === PRELUDE_IDENTITY),
    `expected the prelude in ${JSON.stringify(ir.modules.map((module) => module.identity))}`,
  );
});

test("a catalog property typed as a range accepts a range expression", () => {
  const withRange = 'export external component <SkiaSlider range:<Range T=float64/> />\n';
  const result = compileWithCatalog(host, withRange, "let root() = { <SkiaSlider range={0..1} /> }");

  assert.deepEqual(result.diagnostics, []);
  assert.ok(result.ir instanceof Uint8Array);
});

test("an incomplete range reports a diagnostic at the visitor's own position", () => {
  const result = compile("let root() = {\n  1..\n}");

  assert.equal(result.ir, null);
  assert.ok(result.diagnostics.length > 0);
  assert.equal(result.diagnostics[0].origin, "source");
  assert.equal(result.diagnostics[0].span.startLine, 2);
});
