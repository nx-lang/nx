import assert from "node:assert/strict";
import { test } from "node:test";

import {
  LANGUAGE_QUERIES,
  LanguageServiceHttpError,
  LanguageServiceTimeoutError,
  RESERVED_LANGUAGE_QUERIES,
  UnsupportedQueryError,
  isAbortError,
  isLanguageQueryName,
  isLanguageServiceErrorBody,
  isReservedLanguageQueryName,
  type CompletionList,
  type CompletionsRequest,
  type DiagnosticReport,
  type DiagnosticsRequest,
  type DocumentSymbol,
  type DocumentSymbolsRequest,
  type Hover,
  type HoverRequest,
  type LanguageQueryAnswers,
  type LanguageQueryRequests,
  type NxLanguageService,
} from "../src/index.js";

// The examples the README shows, typed against the protocol so the README cannot drift from it.

const documents = [
  { uri: "nx://tenant/ui.nx", source: "export let <Button label:string /> = <button />\n", version: 3 },
  {
    uri: "nx://tenant/form.nx",
    source: 'import { Button } from "./ui.nx"\n<Button label="😀 go" />\n',
    version: 7,
  },
];

export const hoverRequest: HoverRequest = {
  documents,
  uri: "nx://tenant/form.nx",
  position: { line: 1, character: 3 },
};

export const hoverAnswer: Hover = {
  uri: "nx://tenant/form.nx",
  identity: "tenant/form.nx",
  version: 7,
  range: { start: { line: 1, character: 1 }, end: { line: 1, character: 7 }, startByte: 34, endByte: 40 },
  contents: "```nx\n<Button label:string />\n```",
};

export const completionsRequest: CompletionsRequest = {
  documents,
  uri: "nx://tenant/form.nx",
  // After the emoji: `"😀 go" ` is 8 UTF-16 units, not 7 characters.
  position: { line: 1, character: 23 },
};

export const completionsAnswer: CompletionList = {
  uri: "nx://tenant/form.nx",
  identity: "tenant/form.nx",
  version: 7,
  items: [{ label: "label", kind: "Property", detail: "label:string" }],
};

export const diagnosticsRequest: DiagnosticsRequest = { documents, uri: "nx://tenant/form.nx" };

export const diagnosticsAnswer: DiagnosticReport = {
  documents: [
    { uri: "nx://tenant/ui.nx", identity: "tenant/ui.nx", version: 3, diagnostics: [] },
    {
      uri: "nx://tenant/form.nx",
      identity: "tenant/form.nx",
      version: 7,
      diagnostics: [
        {
          range: { start: { line: 1, character: 8 }, end: { line: 1, character: 13 }, startByte: 41, endByte: 46 },
          severity: "Error",
          code: "type-mismatch",
          message: "Expected string, found int",
          related: [],
        },
      ],
    },
  ],
  workspace: [],
};

export const documentSymbolsRequest: DocumentSymbolsRequest = { documents, uri: "nx://tenant/ui.nx" };

export const documentSymbolsAnswer: DocumentSymbol[] = [
  {
    name: "Button",
    kind: "Component",
    range: { start: { line: 0, character: 0 }, end: { line: 0, character: 47 }, startByte: 0, endByte: 47 },
    selectionRange: { start: { line: 0, character: 12 }, end: { line: 0, character: 18 }, startByte: 12, endByte: 18 },
  },
];

/** An in-process implementation, the shape an editor integration is tested against. */
export const fakeService: NxLanguageService = {
  hover: async () => hoverAnswer,
  completions: async () => completionsAnswer,
  diagnostics: async () => diagnosticsAnswer,
  documentSymbols: async () => documentSymbolsAnswer,
};

test("every example round-trips through JSON without loss", () => {
  for (const example of [
    hoverRequest,
    hoverAnswer,
    completionsRequest,
    completionsAnswer,
    diagnosticsRequest,
    diagnosticsAnswer,
    documentSymbolsRequest,
    documentSymbolsAnswer,
  ]) {
    assert.deepEqual(JSON.parse(JSON.stringify(example)), example);
  }
});

test("the request and answer maps name exactly the defined queries", () => {
  const requests: Record<keyof LanguageQueryRequests, true> = {
    hover: true,
    completions: true,
    diagnostics: true,
    documentSymbols: true,
  };
  const answers: Record<keyof LanguageQueryAnswers, true> = requests;
  assert.deepEqual(Object.keys(answers).sort(), [...LANGUAGE_QUERIES].sort());
});

test("reserved names are distinguishable from defined ones", () => {
  for (const name of LANGUAGE_QUERIES) {
    assert.ok(isLanguageQueryName(name));
    assert.ok(!isReservedLanguageQueryName(name));
  }
  for (const name of RESERVED_LANGUAGE_QUERIES) {
    assert.ok(isReservedLanguageQueryName(name));
    assert.ok(!isLanguageQueryName(name));
  }
  assert.ok(!isLanguageQueryName("format"));
});

test("errors carry what a caller branches on", () => {
  const unsupported = new UnsupportedQueryError("rename");
  assert.equal(unsupported.name, "UnsupportedQueryError");
  assert.equal(unsupported.query, "rename");
  assert.ok(unsupported instanceof Error);

  const http = new LanguageServiceHttpError({
    status: 413,
    url: "http://localhost/api/language/hover",
    bodyExcerpt: '{"error":{"code":"payload-too-large"',
    code: "payload-too-large",
  });
  assert.equal(http.status, 413);
  assert.equal(http.code, "payload-too-large");
  assert.match(http.message, /413/);

  const timeout = new LanguageServiceTimeoutError({ timeoutMs: 5000, url: "http://localhost/api/language/hover" });
  assert.equal(timeout.timeoutMs, 5000);
  assert.match(timeout.message, /5000 ms/);

  assert.ok(isLanguageServiceErrorBody({ error: { code: "unsupported-query", message: "no", query: "rename" } }));
  assert.ok(!isLanguageServiceErrorBody({ message: "no" }));
  assert.ok(!isLanguageServiceErrorBody(null));

  const controller = new AbortController();
  controller.abort();
  assert.ok(isAbortError(controller.signal.reason));
  assert.ok(!isAbortError(new Error("other")));
});
