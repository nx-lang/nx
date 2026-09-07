import assert from "node:assert/strict";
import { createServer } from "node:http";
import { after, before, test } from "node:test";
import type { NxLanguageService } from "@nx-lang/language-protocol";
import { createNxLanguageHandler, toNodeListener } from "@nx-lang/language-http";
import { UnsupportedQueryError, createHttpLanguageService } from "../src/index.js";

const FORM = "nx://tenant/form.nx";
const UI = "nx://tenant/ui.nx";
const documents = [
  { uri: UI, source: "export type Fit = fill | contain | cover\nexport let <Img fit:Fit /> = <img />\n", version: 3 },
  { uri: FORM, source: 'import { Img } from "./ui.nx"\nlet wrong: string = 1\n<Img fit=fill />\n', version: 7 },
];
/** The same set with the value slot empty, the position member completions are asked from. */
const halfTyped = [documents[0]!, { ...documents[1]!, source: 'import { Img } from "./ui.nx"\n<Img fit= />\n' }];

const server = createServer(toNodeListener(createNxLanguageHandler()));
let service: NxLanguageService;
let baseUrl: string;

before(async () => {
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.ok(address !== null && typeof address === "object");
  baseUrl = `http://127.0.0.1:${address.port}/api/language`;
  service = createHttpLanguageService({ baseUrl });
});

after(async () => {
  await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
});

test("hover through the client and handler resolves a sibling document's component", async () => {
  const hover = await service.hover({ documents, uri: FORM, position: { line: 2, character: 2 } });
  assert.ok(hover !== null);
  assert.equal(hover.uri, FORM);
  assert.equal(hover.version, 7);
  assert.ok(hover.contents.includes("<Img"), hover.contents);
  assert.deepEqual(hover.range.start, { line: 2, character: 1 });
});

test("completions through the client and handler offer member values from the sibling", async () => {
  const completions = await service.completions({ documents: halfTyped, uri: FORM, position: { line: 1, character: 9 } });
  const labels = completions.items.map((item) => item.label);
  assert.ok(labels.includes("cover") && labels.includes("contain"), labels.join(","));
  assert.equal(completions.version, 7);
});

test("diagnostics through the client and handler cover every document", async () => {
  const report = await service.diagnostics({ documents, uri: FORM });
  assert.deepEqual(report.documents.map((document) => document.uri).sort(), [FORM, UI]);
  const form = report.documents.find((document) => document.uri === FORM)!;
  assert.equal(form.version, 7);
  assert.equal(form.diagnostics.length, 1);
  assert.equal(form.diagnostics[0]!.range.start.line, 1);
  assert.equal(form.diagnostics[0]!.severity, "Error");
});

test("document symbols through the client and handler", async () => {
  const symbols = await service.documentSymbols({ documents, uri: UI });
  const kinds = symbols.map((symbol) => `${symbol.name}:${symbol.kind}`);
  assert.ok(kinds.includes("Fit:Union"), kinds.join(","));
  assert.ok(kinds.includes("Img:Component"), kinds.join(","));
  assert.deepEqual(symbols[0]!.range.start, { line: 0, character: 0 });
});

test("identical queries are answered identically regardless of what came between", async () => {
  const first = await service.hover({ documents, uri: FORM, position: { line: 2, character: 2 } });
  await service.diagnostics({ documents: [{ uri: FORM, source: "let other = 2\n" }], uri: FORM });
  const third = await service.hover({ documents, uri: FORM, position: { line: 2, character: 2 } });
  assert.deepEqual(third, first);
});

test("a reserved query is an UnsupportedQueryError on the wire, not a transport failure", async () => {
  // The client has no method for reserved queries yet, so the request is steered onto one by
  // rewriting the URL on its way to the real server.
  const steered = createHttpLanguageService({
    baseUrl,
    fetch: (input, init) => fetch(String(input).replace(/\/hover$/, "/definition"), init),
  });
  await assert.rejects(
    steered.hover({ documents, uri: FORM, position: { line: 0, character: 0 } }),
    (error: unknown) => error instanceof UnsupportedQueryError && error.query === "definition",
  );
});
