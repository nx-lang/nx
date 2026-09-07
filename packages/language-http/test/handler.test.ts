import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import type {
  CompletionList,
  DiagnosticReport,
  DocumentSymbol,
  Hover,
  LanguageDocument,
  LanguageServiceErrorBody,
} from "@nx-lang/language-protocol";
import { NxLibraryRegistry } from "@nx-lang/sdk-node";
import {
  createNxLanguageHandler,
  preludeOffsets,
  shiftPositionIn,
  shiftRangeOut,
  toNodeListener,
  type NxLanguageHandlerOptions,
  type SnapshotLike,
} from "../src/index.js";

const FORM = "nx://tenant/form.nx";
const BASE = "http://localhost/api/language";

const PANEL = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />\n";

function post(query: string, body: unknown, init: RequestInit = {}): Request {
  return new Request(`${BASE}/${query}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: typeof body === "string" ? body : JSON.stringify(body),
    ...init,
  });
}

async function json<T>(response: Response): Promise<T> {
  return (await response.json()) as T;
}

/** The UTF-16 position of `marker` in `source`. */
function positionOf(source: string, marker: string): { line: number; character: number } {
  const offset = source.indexOf(marker);
  assert.ok(offset >= 0, `marker ${marker} not in source`);
  const before = source.slice(0, offset);
  const line = before.split("\n").length - 1;
  return { line, character: before.length - (before.lastIndexOf("\n") + 1) };
}

// ---------------------------------------------------------------------------------------------
// Routing and status paths
// ---------------------------------------------------------------------------------------------

test("routes on the final path segment under any base path", async () => {
  const handler = createNxLanguageHandler();
  const source = `${PANEL}<Panel mode=light title="x" />\n`;
  const response = await handler(
    new Request("http://example.test/some/deep/mount/hover", {
      method: "POST",
      body: JSON.stringify({ documents: [{ uri: FORM, source }], uri: FORM, position: positionOf(source, "Panel mode=") }),
    }),
  );
  assert.equal(response.status, 200);
  const hover = await json<Hover | null>(response);
  assert.ok(hover !== null && hover.contents.includes("<Panel"));
});

test("answers wrong methods, unknown queries and reserved queries with JSON errors", async () => {
  const handler = createNxLanguageHandler();

  const get = await handler(new Request(`${BASE}/hover`, { method: "GET" }));
  assert.equal(get.status, 405);
  assert.equal(get.headers.get("allow"), "POST");
  assert.equal((await json<LanguageServiceErrorBody>(get)).error.code, "method-not-allowed");

  const unknown = await handler(post("format", {}));
  assert.equal(unknown.status, 404);
  const unknownBody = await json<LanguageServiceErrorBody>(unknown);
  assert.equal(unknownBody.error.code, "unknown-query");
  assert.equal(unknownBody.error.query, "format");

  const reserved = await handler(post("rename", {}));
  assert.equal(reserved.status, 501);
  const reservedBody = await json<LanguageServiceErrorBody>(reserved);
  assert.equal(reservedBody.error.code, "unsupported-query");
  assert.equal(reservedBody.error.query, "rename");
});

test("refuses bodies that are not JSON or not a query, naming what is missing", async () => {
  const handler = createNxLanguageHandler();

  const notJson = await handler(post("hover", "{not json"));
  assert.equal(notJson.status, 400);
  assert.equal((await json<LanguageServiceErrorBody>(notJson)).error.code, "invalid-request");

  const noDocuments = await handler(post("diagnostics", { uri: FORM }));
  assert.equal(noDocuments.status, 400);
  assert.match((await json<LanguageServiceErrorBody>(noDocuments)).error.message, /'documents'/);

  const noUri = await handler(post("diagnostics", { documents: [{ uri: FORM, source: "" }] }));
  assert.equal(noUri.status, 400);
  assert.match((await json<LanguageServiceErrorBody>(noUri)).error.message, /'uri'/);

  const noSource = await handler(post("diagnostics", { documents: [{ uri: FORM }], uri: FORM }));
  assert.match((await json<LanguageServiceErrorBody>(noSource)).error.message, /'source'/);

  const noPosition = await handler(post("hover", { documents: [{ uri: FORM, source: "" }], uri: FORM }));
  assert.match((await json<LanguageServiceErrorBody>(noPosition)).error.message, /'position'/);

  const foreignUri = await handler(
    post("diagnostics", { documents: [{ uri: FORM, source: "" }], uri: "nx://tenant/other.nx" }),
  );
  assert.match((await json<LanguageServiceErrorBody>(foreignUri)).error.message, /names no document/);

  const badUri = await handler(post("diagnostics", { documents: [{ uri: "not a uri", source: "" }], uri: "not a uri" }));
  assert.equal(badUri.status, 400);
  assert.match((await json<LanguageServiceErrorBody>(badUri)).error.message, /not a uri/);
});

test("refuses a body over the limit with 413", async () => {
  const handler = createNxLanguageHandler({ maxBodyBytes: 256 });
  const response = await handler(
    post("diagnostics", { documents: [{ uri: FORM, source: "x".repeat(1024) }], uri: FORM }),
  );
  assert.equal(response.status, 413);
  assert.equal((await json<LanguageServiceErrorBody>(response)).error.code, "payload-too-large");

  const declared = await handler(
    new Request(`${BASE}/diagnostics`, {
      method: "POST",
      headers: { "content-length": "999999" },
      body: JSON.stringify({ documents: [{ uri: FORM, source: "" }], uri: FORM }),
    }),
  );
  assert.equal(declared.status, 413);
});

test("contains a service failure as a JSON 500, reports it, and keeps answering", async () => {
  const errors: unknown[] = [];
  let shouldThrow = true;
  const handler = createNxLanguageHandler({
    onError: (error) => errors.push(error),
    createSnapshot: (documents, options) => {
      if (shouldThrow) {
        throw new Error("boom");
      }
      return realFactory(documents, options);
    },
  });
  const body = { documents: [{ uri: FORM, source: "let value = 1\n" }], uri: FORM };

  const failed = await handler(post("diagnostics", body));
  assert.equal(failed.status, 500);
  assert.equal((await json<LanguageServiceErrorBody>(failed)).error.code, "internal-error");
  assert.equal(errors.length, 1);

  shouldThrow = false;
  const ok = await handler(post("diagnostics", body));
  assert.equal(ok.status, 200);
});

// ---------------------------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------------------------

/** The real snapshot factory, so a test can count constructions without faking analysis. */
async function loadRealFactory(): Promise<NonNullable<NxLanguageHandlerOptions["createSnapshot"]>> {
  const { NxLanguageSnapshot } = await import("@nx-lang/sdk-node");
  return (documents, options) => new NxLanguageSnapshot(documents, options);
}
const realFactory = await loadRealFactory();

test("byte-identical document sets construct one snapshot; a one-character change constructs another", async () => {
  let constructed = 0;
  const handler = createNxLanguageHandler({
    createSnapshot: (documents, options) => {
      constructed += 1;
      return realFactory(documents, options);
    },
  });
  const documents: LanguageDocument[] = [{ uri: FORM, source: `${PANEL}<Panel mode=light title="x" />\n`, version: 1 }];

  await handler(post("hover", { documents, uri: FORM, position: { line: 2, character: 2 } }));
  await handler(post("hover", { documents, uri: FORM, position: { line: 2, character: 3 } }));
  await handler(post("completions", { documents, uri: FORM, position: { line: 2, character: 7 } }));
  await handler(post("diagnostics", { documents, uri: FORM }));
  assert.equal(constructed, 1);

  const changed = [{ ...documents[0]!, source: documents[0]!.source.replace("light", "dark ") }];
  await handler(post("diagnostics", { documents: changed, uri: FORM }));
  assert.equal(constructed, 2);
});

test("the same text under a new version reuses the snapshot, and the answer carries the new version", async () => {
  let constructed = 0;
  const handler = createNxLanguageHandler({
    createSnapshot: (documents, options) => {
      constructed += 1;
      return realFactory(documents, options);
    },
  });
  const source = `${PANEL}<Panel mode=light title="x" />\n`;
  const position = { line: 2, character: 2 };

  const first = await json<Hover | null>(await handler(post("hover", { documents: [{ uri: FORM, source, version: 1 }], uri: FORM, position })));
  assert.ok(first !== null);
  assert.equal(first.version, 1);

  const second = await json<Hover | null>(await handler(post("hover", { documents: [{ uri: FORM, source, version: 2 }], uri: FORM, position })));
  assert.ok(second !== null);
  assert.equal(second.version, 2);
  const list = await json<CompletionList>(await handler(post("completions", { documents: [{ uri: FORM, source, version: 3 }], uri: FORM, position: { line: 2, character: 7 } })));
  assert.equal(list.version, 3);
  const report = await json<DiagnosticReport>(await handler(post("diagnostics", { documents: [{ uri: FORM, source, version: 4 }], uri: FORM })));
  assert.deepEqual(report.documents.map((document) => document.version), [4]);
  const unversioned = await json<DiagnosticReport>(await handler(post("diagnostics", { documents: [{ uri: FORM, source }], uri: FORM })));
  assert.deepEqual(unversioned.documents.map((document) => document.version), [null]);
  assert.equal(constructed, 1);
});

test("an evicted snapshot is disposed and the cache never exceeds its size", async () => {
  const disposed: string[] = [];
  const handler = createNxLanguageHandler({
    cacheSize: 2,
    createSnapshot: (documents): SnapshotLike => {
      const source = documents[0]!.source;
      return {
        hover: () => null,
        completions: () => ({ uri: FORM, identity: "tenant/form.nx", version: null, items: [] }),
        diagnostics: () => ({ documents: [], workspace: [] }),
        documentSymbols: () => [],
        dispose: () => disposed.push(source),
      };
    },
  });
  for (const text of ["a", "b", "c"]) {
    await handler(post("diagnostics", { documents: [{ uri: FORM, source: text }], uri: FORM }));
  }
  assert.deepEqual(disposed, ["a"]);
});

// ---------------------------------------------------------------------------------------------
// Build context
// ---------------------------------------------------------------------------------------------

test("a build context makes library components visible; no context leaves them unresolved", async () => {
  const tempRoot = mkdtempSync(join(tmpdir(), "nx-language-http-"));
  const uiDir = join(tempRoot, "ui");
  mkdirSync(uiDir, { recursive: true });
  writeFileSync(join(uiDir, "button.nx"), "export type Size = small | large\nexport let <Button label:string size:Size /> = <button />\n");
  const registry = new NxLibraryRegistry();
  registry.loadFromDirectory(uiDir);
  const buildContext = registry.createBuildContext();
  const source = 'import { Button } from "../ui"\n<Button label="go" size=small />\n';
  const body = { documents: [{ uri: FORM, source }], uri: FORM, position: positionOf(source, "Button label") };
  try {
    const seeing = createNxLanguageHandler({ buildContext });
    const hover = await json<Hover | null>(await seeing(post("hover", body)));
    assert.ok(hover !== null && hover.contents.includes("<Button") && hover.contents.includes("size"), JSON.stringify(hover));

    const blind = createNxLanguageHandler();
    const blindHover = await json<Hover | null>(await blind(post("hover", body)));
    assert.ok(blindHover === null || !blindHover.contents.includes("size"));
    const report = await json<DiagnosticReport>(await blind(post("diagnostics", { documents: body.documents, uri: FORM })));
    const messages = report.documents.flatMap((document) => document.diagnostics.map((diagnostic) => diagnostic.message));
    assert.ok(messages.some((message) => message.includes("Missing workspace module or loaded library")), messages.join("; "));
  } finally {
    buildContext.dispose();
    registry.dispose();
    rmSync(tempRoot, { recursive: true, force: true });
  }
});

// ---------------------------------------------------------------------------------------------
// Prelude
// ---------------------------------------------------------------------------------------------

test("prelude offsets add a separating line and shift only lines and bytes", () => {
  const withNewline = preludeOffsets("let a = 1\n");
  assert.equal(withNewline.text, "let a = 1\n\n");
  assert.equal(withNewline.lines, 2);
  assert.equal(withNewline.bytes, 11);

  const withoutNewline = preludeOffsets("let a = 1");
  assert.deepEqual(withoutNewline, withNewline);

  const emoji = preludeOffsets('let s = "😀"');
  assert.equal(emoji.lines, 2);
  assert.equal(emoji.bytes, Buffer.byteLength('let s = "😀"\n\n'));

  assert.deepEqual(shiftPositionIn({ line: 3, character: 7 }, withNewline), { line: 5, character: 7 });
  assert.deepEqual(
    shiftRangeOut({ start: { line: 2, character: 4 }, end: { line: 2, character: 9 }, startByte: 15, endByte: 20 }, withNewline),
    { start: { line: 0, character: 4 }, end: { line: 0, character: 9 }, startByte: 4, endByte: 9 },
  );
  assert.equal(
    shiftRangeOut({ start: { line: 0, character: 4 }, end: { line: 0, character: 5 }, startByte: 4, endByte: 5 }, withNewline),
    null,
  );
});

const preludeHandler = createNxLanguageHandler({ prelude: { source: PANEL.trimEnd() } });

test("hover through a prelude answers in the document's own coordinates", async () => {
  const source = '<Panel mode=light title="x" />\n';
  const position = positionOf(source, "Panel");
  const hover = await json<Hover | null>(await preludeHandler(post("hover", { documents: [{ uri: FORM, source, version: 5 }], uri: FORM, position })));
  assert.ok(hover !== null, "hover content");
  assert.ok(hover.contents.includes("<Panel"), hover.contents);
  assert.equal(hover.version, 5);
  assert.deepEqual(hover.range.start, { line: 0, character: 1 });
  assert.deepEqual(hover.range.end, { line: 0, character: 6 });
  assert.equal(hover.range.startByte, 1);
  assert.equal(hover.range.endByte, 6);
});

test("completions inside a prelude component's tag offer its properties", async () => {
  const source = "<Panel  />\n";
  const completions = await json<CompletionList>(
    await preludeHandler(post("completions", { documents: [{ uri: FORM, source }], uri: FORM, position: { line: 0, character: 7 } })),
  );
  const labels = completions.items.map((item) => item.label);
  assert.ok(labels.includes("mode") && labels.includes("title"), labels.join(","));
});

test("prelude-internal diagnostics are reported with a prelude origin and no range", async () => {
  const broken = createNxLanguageHandler({ prelude: { source: `${PANEL}let wrong: string = 1` } });
  const source = 'let alsoWrong: string = 2\n<Panel mode=light title="x" />\n';
  const report = await json<DiagnosticReport>(await broken(post("diagnostics", { documents: [{ uri: FORM, source }], uri: FORM })));

  const positioned = report.documents.find((document) => document.uri === FORM)!.diagnostics;
  assert.equal(positioned.length, 1, JSON.stringify(positioned));
  assert.equal(positioned[0]!.range.start.line, 0);
  assert.match(positioned[0]!.message, /alsoWrong/);

  const preludeOrigin = report.workspace.filter((diagnostic) => diagnostic.labels.some((label) => label.identity === "prelude"));
  assert.equal(preludeOrigin.length, 1, JSON.stringify(report.workspace));
  assert.equal(preludeOrigin[0]!.severity, "Error");
});

test("document symbols through a prelude are the document's own, shifted", async () => {
  const source = "let mine = 1\n";
  const symbols = await json<DocumentSymbol[]>(await preludeHandler(post("documentSymbols", { documents: [{ uri: FORM, source }], uri: FORM })));
  assert.deepEqual(symbols.map((symbol) => symbol.name), ["mine"]);
  assert.equal(symbols[0]!.range.start.line, 0);
  assert.equal(symbols[0]!.range.startByte, 0);
});

test("a prelude without a trailing newline still keeps the document's first line its own", async () => {
  const source = "let first = 1\n";
  const report = await json<DiagnosticReport>(await preludeHandler(post("diagnostics", { documents: [{ uri: FORM, source }], uri: FORM })));
  assert.deepEqual(report.documents[0]!.diagnostics, []);
  const symbols = await json<DocumentSymbol[]>(await preludeHandler(post("documentSymbols", { documents: [{ uri: FORM, source }], uri: FORM })));
  assert.deepEqual(symbols[0]!.range.start, { line: 0, character: 0 });
});

test("a sibling's related location that points into the queried document is shifted too", async () => {
  const SIBLING = "nx://tenant/sibling.nx";
  const offsets = preludeOffsets(PANEL);
  const inCombined = { start: { line: offsets.lines + 1, character: 4 }, end: { line: offsets.lines + 1, character: 9 }, startByte: offsets.bytes + 14, endByte: offsets.bytes + 19 };
  const siblingRange = { start: { line: 0, character: 0 }, end: { line: 0, character: 5 }, startByte: 0, endByte: 5 };
  const related = (uri: string, range: typeof inCombined) => ({ uri, identity: uri.slice("nx://".length), range, message: null });
  const handler = createNxLanguageHandler({
    prelude: { source: PANEL },
    createSnapshot: (): SnapshotLike => ({
      hover: () => null,
      completions: () => ({ uri: FORM, identity: "tenant/form.nx", version: null, items: [] }),
      diagnostics: () => ({
        documents: [
          {
            uri: SIBLING,
            identity: "tenant/sibling.nx",
            version: null,
            diagnostics: [
              { severity: "Error", code: "NXE1", message: "conflicts", range: siblingRange, related: [related(FORM, inCombined), related(SIBLING, siblingRange)] },
            ],
          },
          {
            uri: FORM,
            identity: "tenant/form.nx",
            version: null,
            diagnostics: [{ severity: "Error", code: "NXE1", message: "conflicts", range: inCombined, related: [related(SIBLING, siblingRange)] }],
          },
        ],
        workspace: [],
      }),
      documentSymbols: () => [],
      dispose: () => undefined,
    }),
  });

  const report = await json<DiagnosticReport>(
    await handler(post("diagnostics", { documents: [{ uri: FORM, source: "let x = 1\nlet x = 2\n" }, { uri: SIBLING, source: "let x = 3\n" }], uri: FORM })),
  );
  const own = shiftRangeOut(inCombined, offsets);
  const sibling = report.documents.find((document) => document.uri === SIBLING)!;
  assert.deepEqual(sibling.diagnostics[0]!.range, siblingRange, "the sibling's own range is untouched");
  assert.deepEqual(sibling.diagnostics[0]!.related.map((location) => location.range), [own, siblingRange]);
  const form = report.documents.find((document) => document.uri === FORM)!;
  assert.deepEqual(form.diagnostics[0]!.range, own);
  assert.deepEqual(form.diagnostics[0]!.related.map((location) => location.range), [siblingRange]);
});

// ---------------------------------------------------------------------------------------------
// Node adapter
// ---------------------------------------------------------------------------------------------

test("the Node listener answers identically to the handler", async () => {
  const handler = createNxLanguageHandler();
  const server = createServer(toNodeListener(handler));
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.ok(address !== null && typeof address === "object");
  const base = `http://127.0.0.1:${address.port}/api/language`;
  const source = `${PANEL}<Panel mode=light title="x" />\n`;
  const body = { documents: [{ uri: FORM, source, version: 2 }], uri: FORM, position: positionOf(source, "Panel mode=") };
  try {
    const viaHttp = await fetch(`${base}/hover`, { method: "POST", body: JSON.stringify(body) });
    const direct = await handler(post("hover", body));
    assert.equal(viaHttp.status, 200);
    assert.equal(viaHttp.headers.get("content-type"), direct.headers.get("content-type"));
    assert.deepEqual(await viaHttp.json(), await direct.json());

    const wrong = await fetch(`${base}/hover`, { method: "GET" });
    assert.equal(wrong.status, 405);

    const big = await fetch(`${base}/diagnostics`, {
      method: "POST",
      body: JSON.stringify({ documents: [{ uri: FORM, source: "x".repeat(2 * 1024 * 1024) }], uri: FORM }),
    });
    assert.equal(big.status, 413);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});

test("a Host header that is not a valid authority is answered, and the server keeps listening", async () => {
  const handler = createNxLanguageHandler();
  const server = createServer(toNodeListener(handler));
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.ok(address !== null && typeof address === "object");
  const source = `${PANEL}<Panel mode=light title="x" />\n`;
  const body = JSON.stringify({ documents: [{ uri: FORM, source }], uri: FORM, position: positionOf(source, "Panel mode=") });
  try {
    // Node's parser accepts `Host: a b`; `new Request("http://a b/...")` does not.
    const raw = await new Promise<string>((resolve, reject) => {
      const socket = connect(address.port, "127.0.0.1", () => {
        socket.write(
          `POST /api/language/hover HTTP/1.1\r\nHost: a b\r\nContent-Type: application/json\r\n` +
            `Content-Length: ${Buffer.byteLength(body)}\r\nConnection: close\r\n\r\n${body}`,
        );
      });
      let received = "";
      socket.setEncoding("utf8");
      socket.on("data", (chunk: string) => (received += chunk));
      socket.on("end", () => resolve(received));
      socket.on("error", reject);
    });
    assert.match(raw, /^HTTP\/1\.1 200 /, raw.slice(0, 200));
    const hover = JSON.parse(raw.slice(raw.indexOf("\r\n\r\n") + 4)) as Hover | null;
    assert.ok(hover !== null && hover.contents.includes("Panel"), raw);

    const after = await fetch(`http://127.0.0.1:${address.port}/api/language/hover`, { method: "POST", body });
    assert.equal(after.status, 200, "the process and the server survived the odd request");
  } finally {
    await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});
