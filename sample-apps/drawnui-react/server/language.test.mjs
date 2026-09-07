/**
 * Proves the language route answers in the visitor's coordinates, with the catalog's declarations
 * visible, through the same handler `server/index.mjs` mounts.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { LANGUAGE_ROUTE, languageHandler } from "./language.mjs";

const URI = "nx://fiddle/fiddle.nx";

function post(query, body) {
  return languageHandler(
    new Request(`http://localhost${LANGUAGE_ROUTE}${query}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    }),
  );
}

test("hover on a catalog component answers its signature at the visitor's own line and column", async () => {
  const source = 'let root() = {\n  <SkiaLabel Text="hi" />\n}\n';
  const response = await post("hover", {
    documents: [{ uri: URI, source, version: 3 }],
    uri: URI,
    // Inside `SkiaLabel` on the visitor's second line.
    position: { line: 1, character: 5 },
  });
  assert.equal(response.status, 200);
  const hover = await response.json();
  assert.ok(hover !== null, "the catalog component has hover content");
  assert.ok(hover.contents.includes("SkiaLabel"), hover.contents);
  assert.ok(hover.contents.includes("Text"), hover.contents);
  assert.equal(hover.version, 3);
  // The catalog leads the combined module by hundreds of lines; none of them show here.
  assert.deepEqual(hover.range.start, { line: 1, character: 3 });
  assert.deepEqual(hover.range.end, { line: 1, character: 12 });
  assert.equal(hover.range.startByte, source.indexOf("SkiaLabel"));
});

test("completions inside a catalog tag offer its properties not yet supplied", async () => {
  const source = 'let root() = {\n  <SkiaLabel Text="hi"  />\n}\n';
  const response = await post("completions", {
    documents: [{ uri: URI, source }],
    uri: URI,
    position: { line: 1, character: 23 },
  });
  assert.equal(response.status, 200);
  const completions = await response.json();
  const labels = completions.items.map((item) => item.label);
  assert.ok(labels.includes("FontSize"), labels.slice(0, 20).join(","));
  assert.ok(!labels.includes("Text"), "a property already supplied is not offered again");
});

test("diagnostics are positioned in the visitor's source, and the catalog contributes none", async () => {
  const source = "let root() = {\n  <SkiaLabel Text=1.0 />\n}\n";
  const response = await post("diagnostics", { documents: [{ uri: URI, source }], uri: URI });
  assert.equal(response.status, 200);
  const report = await response.json();
  const [document] = report.documents;
  assert.equal(document.uri, URI);
  assert.equal(document.diagnostics.length, 1, JSON.stringify(document.diagnostics));
  assert.equal(document.diagnostics[0].range.start.line, 1);
  assert.deepEqual(report.workspace, []);
});
