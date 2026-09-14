import assert from "node:assert/strict";
import { test } from "node:test";
import type {
  CompletionList,
  DiagnosticReport,
  DocumentSymbol,
  EditorRange,
  Hover,
  LanguageDocument,
} from "@nx-lang/language-protocol";
import { isAbortError } from "@nx-lang/language-protocol";

import { PRELUDE_ORIGIN } from "../src/answer.js";
import type { SnapshotLike } from "../src/cache.js";
import { preludeOffsets } from "../src/prelude.js";
import { createSnapshotLanguageService } from "../src/service.js";

const FORM = "nx://tenant/form.nx";
const PANEL = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />";
const OFFSETS = preludeOffsets(PANEL);

/** A range in the combined text, so a fake snapshot can answer where the real one would. */
function range(line: number, startCharacter: number, endCharacter: number, startByte: number): EditorRange {
  return {
    start: { line, character: startCharacter },
    end: { line, character: endCharacter },
    startByte,
    endByte: startByte + (endCharacter - startCharacter),
  };
}

/**
 * A snapshot that answers from the combined text it was given, so the service's own shifting is
 * what the test observes rather than the compiler's.
 */
function fakeSnapshot(
  documents: readonly LanguageDocument[],
  record: { analyses: number; sources: string[] },
): SnapshotLike {
  record.analyses += 1;
  record.sources.push(documents[0]!.source);
  const combined = documents[0]!.source;
  const panelLine = combined.split("\n").findIndex((line) => line.includes("<Panel mode=light"));

  return {
    hover: (): Hover => ({
      uri: FORM,
      identity: "tenant/form.nx",
      version: 1,
      contents: "```nx\nlet <Panel mode:Mode title:string />\n```",
      range: range(panelLine, 1, 6, combined.indexOf("<Panel mode=light") + 1),
    }),
    completions: (): CompletionList => ({
      uri: FORM,
      identity: "tenant/form.nx",
      version: 1,
      items: [{ label: "mode", kind: "Property", detail: null }],
    }),
    diagnostics: (): DiagnosticReport => ({
      documents: [
        {
          uri: FORM,
          identity: "tenant/form.nx",
          version: 1,
          diagnostics: [
            {
              severity: "Error",
              code: "prelude-problem",
              message: "the prelude itself is broken",
              // Line 0 is inside the prelude, so this diagnostic belongs to no document line.
              range: range(0, 0, 4, 0),
              related: [],
            },
            {
              severity: "Warning",
              code: "document-problem",
              message: "the document has a problem",
              range: range(panelLine, 1, 6, combined.indexOf("<Panel mode=light") + 1),
              related: [],
            },
          ],
        },
      ],
      workspace: [],
    }),
    documentSymbols: (): DocumentSymbol[] => [
      {
        name: "Panel",
        kind: "Component",
        range: range(panelLine, 0, 30, combined.indexOf("<Panel mode=light")),
        selectionRange: range(panelLine, 1, 6, combined.indexOf("<Panel mode=light") + 1),
      },
    ],
    dispose: () => {},
  };
}

function serviceOver(record: { analyses: number; sources: string[] }) {
  return createSnapshotLanguageService({
    createSnapshot: (documents) => fakeSnapshot(documents, record),
    prelude: { source: PANEL },
  });
}

const SOURCE = '<Panel mode=light title="x" />\n';

test("hover through a prelude answers in the document's own coordinates", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);

  const hover = await service.hover({
    documents: [{ uri: FORM, source: SOURCE, version: 5 }],
    uri: FORM,
    position: { line: 0, character: 3 },
  });

  assert.ok(hover !== null);
  assert.ok(hover.contents.includes("<Panel"), hover.contents);
  assert.deepEqual(hover.range.start, { line: 0, character: 1 });
  assert.deepEqual(hover.range.end, { line: 0, character: 6 });
  assert.equal(hover.range.startByte, 1);
  assert.equal(hover.version, 5);

  // The snapshot saw the prelude ahead of the document, and the queried position was shifted into
  // the combined text before it was asked.
  assert.ok(record.sources[0]!.startsWith(OFFSETS.text));
});

test("a diagnostic inside the prelude is reported with a prelude origin and no range", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);

  const report = await service.diagnostics({
    documents: [{ uri: FORM, source: SOURCE, version: 2 }],
    uri: FORM,
  });

  assert.deepEqual(
    report.workspace.map((diagnostic) => [diagnostic.code, diagnostic.labels[0]?.identity]),
    [["prelude-problem", PRELUDE_ORIGIN]],
  );
  assert.deepEqual(
    report.documents[0]!.diagnostics.map((diagnostic) => diagnostic.code),
    ["document-problem"],
  );
  assert.deepEqual(report.documents[0]!.diagnostics[0]!.range.start, { line: 0, character: 1 });
});

test("document symbols through a prelude are the document's own, shifted", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);

  const symbols = await service.documentSymbols({
    documents: [{ uri: FORM, source: SOURCE }],
    uri: FORM,
  });

  assert.deepEqual(
    symbols.map((symbol) => [symbol.name, symbol.range.start.line, symbol.selectionRange.start.character]),
    [["Panel", 0, 1]],
  );
});

test("repeated queries over unchanged text analyze once, and each answer carries its own version", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);
  const position = { line: 0, character: 3 };

  const first = await service.hover({ documents: [{ uri: FORM, source: SOURCE, version: 1 }], uri: FORM, position });
  const second = await service.hover({ documents: [{ uri: FORM, source: SOURCE, version: 2 }], uri: FORM, position });
  const completions = await service.completions({
    documents: [{ uri: FORM, source: SOURCE, version: 3 }],
    uri: FORM,
    position,
  });
  const report = await service.diagnostics({ documents: [{ uri: FORM, source: SOURCE, version: 4 }], uri: FORM });
  const unversioned = await service.diagnostics({ documents: [{ uri: FORM, source: SOURCE }], uri: FORM });

  assert.equal(record.analyses, 1);
  assert.equal(first!.version, 1);
  assert.equal(second!.version, 2);
  assert.equal(completions.version, 3);
  assert.deepEqual(report.documents.map((document) => document.version), [4]);
  assert.deepEqual(unversioned.documents.map((document) => document.version), [null]);

  // A one-character change is a different document set, so it is analyzed afresh.
  await service.diagnostics({ documents: [{ uri: FORM, source: SOURCE.replace("light", "dark ") }], uri: FORM });
  assert.equal(record.analyses, 2);
});

test("a cancelled signal rejects with an abort error and asks nothing of the snapshot", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);
  const controller = new AbortController();
  controller.abort();

  await assert.rejects(
    () =>
      service.hover(
        { documents: [{ uri: FORM, source: SOURCE }], uri: FORM, position: { line: 0, character: 3 } },
        controller.signal,
      ),
    (error: unknown) => isAbortError(error),
  );
  await assert.rejects(
    () => service.diagnostics({ documents: [{ uri: FORM, source: SOURCE }], uri: FORM }, controller.signal),
    (error: unknown) => isAbortError(error),
  );
  assert.equal(record.analyses, 0);
});

test("dispose releases the cached analyses and the service answers again afterwards", async () => {
  const record = { analyses: 0, sources: [] as string[] };
  const service = serviceOver(record);
  const request = { documents: [{ uri: FORM, source: SOURCE }], uri: FORM };

  await service.diagnostics(request);
  await service.diagnostics(request);
  assert.equal(record.analyses, 1);

  service.dispose();
  await service.diagnostics(request);
  assert.equal(record.analyses, 2);
});
