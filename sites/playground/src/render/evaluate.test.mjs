/**
 * The renderer's preparation: a compiled snippet linked against the catalog prepared once.
 *
 * The catalog's artifact is emitted here the way the Vite build emits it, so what is checked is
 * the link the page performs: the snippet's own artifact, a few kilobytes naming the catalog,
 * against the catalog's declarations it did not carry.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { CATALOG_IDENTITY, compileWithCatalog, emitCatalogArtifact } from "../compile/catalog.ts";
import { evaluateRoot, prepare, prepareCatalog } from "./evaluate.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const host = createNxHost(await loadNxModule());
after(() => host.dispose());

const snippet = compileWithCatalog(host, catalog, 'let root() = { <SkiaLabel Text="hi" FontSize=12 /> }').ir;

test("a snippet links against the catalog prepared from its own artifact and evaluates", () => {
  const prepared = prepareCatalog(emitCatalogArtifact(host, catalog));
  assert.equal(prepared.identity, CATALOG_IDENTITY);
  const program = prepare(snippet, prepared);
  const root = evaluateRoot(program);
  assert.equal(root.$type, "SkiaLabel");
  assert.equal(root.Text, "hi");
  assert.equal(root.FontSize, 12);
  // The catalog's declarations came from the prepared module, not the snippet.
  assert.equal(program.modulesByIdentity.get(CATALOG_IDENTITY)?.module, prepared);
});

test("one prepared catalog serves every compile", () => {
  const prepared = prepareCatalog(emitCatalogArtifact(host, catalog));
  const other = compileWithCatalog(host, catalog, 'let root() = { <SkiaLayer><SkiaLabel Text="a" /></SkiaLayer> }').ir;
  const first = prepare(snippet, prepared);
  const second = prepare(other, prepared);
  assert.equal(first.modulesByIdentity.get(CATALOG_IDENTITY)?.module, second.modulesByIdentity.get(CATALOG_IDENTITY)?.module);
  assert.equal(evaluateRoot(second).$type, "SkiaLayer");
});

test("a catalog that lacks a control the snippet names refuses to link, naming the control", () => {
  const without = prepareCatalog(emitCatalogArtifact(host, "export abstract external component <DrawnNode />\n"));
  assert.throws(() => prepare(snippet, without), /SkiaLabel/);
});

/// A range loop renders one item per integer, and the renderer's resolver is unchanged: it answers
/// for the catalog alone, and the IR runtime supplies the prelude the snippet links against.
test("a range loop renders one item per integer with the resolver unchanged", () => {
  const prepared = prepareCatalog(emitCatalogArtifact(host, catalog));
  const stars = compileWithCatalog(
    host,
    catalog,
    'let root() = { <SkiaLayer>for i in 0..5 { <SkiaLabel Text="star" /> }</SkiaLayer> }',
  );
  assert.deepEqual(stars.diagnostics, []);

  const root = evaluateRoot(prepare(stars.ir, prepared));
  assert.equal(root.$type, "SkiaLayer");
  assert.equal(root.Children.length, 5);
  assert.deepEqual(
    root.Children.map((child) => child.$type),
    ["SkiaLabel", "SkiaLabel", "SkiaLabel", "SkiaLabel", "SkiaLabel"],
  );
});
