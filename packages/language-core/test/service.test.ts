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

import type { SnapshotLike } from "../src/cache.js";
import { createSnapshotLanguageService } from "../src/service.js";

const FORM = "nx://tenant/form.nx";
const CATALOG = "nx://tenant/catalog.nx";
const PANEL = "type Mode = light | dark\nlet <Panel mode:Mode title:string /> = <div />\n";

/** A range in the queried document's own text, which is the only coordinate space there is. */
function range(line: number, startCharacter: number, endCharacter: number, startByte: number): EditorRange {
  return {
    start: { line, character: startCharacter },
    end: { line, character: endCharacter },
    startByte,
    endByte: startByte + (endCharacter - startCharacter),
  };
}

/**
 * A snapshot that answers from the document set it was given, so what the test observes is the
 * service's own dispatch — which documents it analyzed, and under which URI each answer came
 * back — rather than the compiler's.
 */
function fakeSnapshot(
  documents: readonly LanguageDocument[],
  record: { analyses: number; sources: string[]; implicitImports: string[][] },
  options: { implicitImports: readonly string[] },
): SnapshotLike {
  record.analyses += 1;
  record.sources.push(documents.map((document) => document.uri).join(","));
  record.implicitImports.push(Array.from(options.implicitImports));
  // The queried document is analyzed as the client sent it, so its own line is line 0 and the
  // answer needs no arithmetic.
  const queried = documents.find((document) => document.uri === FORM)!.source;
  const panelLine = queried.split("\n").findIndex((line) => line.includes("<Panel mode=light"));

  return {
    hover: (): Hover => ({
      uri: FORM,
      identity: "tenant/form.nx",
      version: 1,
      contents: "```nx\nlet <Panel mode:Mode title:string />\n```",
      range: range(panelLine, 1, 6, queried.indexOf("<Panel mode=light") + 1),
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
          uri: CATALOG,
          identity: "tenant/catalog.nx",
          version: null,
          diagnostics: [
            {
              severity: "Error",
              code: "context-problem",
              message: "the context document itself is broken",
              range: range(1, 0, 4, 0),
              related: [],
            },
          ],
        },
        {
          uri: FORM,
          identity: "tenant/form.nx",
          version: 1,
          diagnostics: [
            {
              severity: "Warning",
              code: "document-problem",
              message: "the document has a problem",
              range: range(panelLine, 1, 6, queried.indexOf("<Panel mode=light") + 1),
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
        range: range(panelLine, 0, 30, queried.indexOf("<Panel mode=light")),
        selectionRange: range(panelLine, 1, 6, queried.indexOf("<Panel mode=light") + 1),
      },
    ],
    dispose: () => {},
  };
}

type Record_ = { analyses: number; sources: string[]; implicitImports: string[][] };

function newRecord(): Record_ {
  return { analyses: 0, sources: [], implicitImports: [] };
}

function serviceOver(record: Record_) {
  return createSnapshotLanguageService({
    createSnapshot: (documents, options) => fakeSnapshot(documents, record, options),
    context: {
      documents: [{ uri: CATALOG, identity: "tenant/catalog.nx", source: PANEL }],
      implicitImports: ["tenant/catalog.nx"],
    },
  });
}

const SOURCE = '<Panel mode=light title="x" />\n';

test("hover through host context answers in the queried document's own coordinates", async () => {
  const record = newRecord();
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

  // The context document joined the set without the client sending it, its identity was named as an
  // implicit import, and the queried document's own text went through unchanged.
  assert.equal(record.sources[0], `${CATALOG},${FORM}`);
  assert.deepEqual(record.implicitImports[0], ["tenant/catalog.nx"]);
});

test("completions through host context offer the context component's properties", async () => {
  const record = newRecord();
  const service = serviceOver(record);

  const completions = await service.completions({
    documents: [{ uri: FORM, source: SOURCE, version: 1 }],
    uri: FORM,
    position: { line: 0, character: 7 },
  });

  assert.deepEqual(
    completions.items.map((item) => item.label),
    ["mode"],
  );
});

test("a context document's own error is reported under its URI, not the queried document's", async () => {
  const record = newRecord();
  const service = serviceOver(record);

  const report = await service.diagnostics({
    documents: [{ uri: FORM, source: SOURCE, version: 2 }],
    uri: FORM,
  });

  assert.deepEqual(
    report.documents.map((document) => [document.uri, document.diagnostics.map((diagnostic) => diagnostic.code)]),
    [
      [CATALOG, ["context-problem"]],
      [FORM, ["document-problem"]],
    ],
  );
  // The queried document's diagnostic is in its own coordinates, and nothing is a workspace
  // diagnostic: no position was ever rewritten.
  const queried = report.documents.find((document) => document.uri === FORM)!;
  assert.deepEqual(queried.diagnostics[0]!.range.start, { line: 0, character: 1 });
  assert.deepEqual(report.workspace, []);
  assert.equal(queried.version, 2);
});

test("a client cannot supply a document with a context document's URI", async () => {
  const service = serviceOver(newRecord());

  await assert.rejects(
    () =>
      service.hover({
        documents: [{ uri: CATALOG, source: "let x = 1\n" }],
        uri: CATALOG,
        position: { line: 0, character: 0 },
      }),
    /A document with the uri 'nx:\/\/tenant\/catalog\.nx' is part of the host's context/,
  );
});

// The context document is declared the way both READMEs declare one — a URI and a source, with no
// identity — so its identity is the derived `tenant/catalog.nx`. A client naming that identity under
// a URI of its own would replace the host's declarations, which is the refusal this asserts.
test("a client cannot supply a document with a context document's derived identity", async () => {
  const service = serviceOver(newRecord());

  await assert.rejects(
    () =>
      service.hover({
        documents: [{ uri: "nx://other.nx", identity: "tenant/catalog.nx", source: "let x = 1\n" }],
        uri: "nx://other.nx",
        position: { line: 0, character: 0 },
      }),
    /A document with the identity 'tenant\/catalog\.nx' is part of the host's context/,
  );
});

test("passing the removed prelude option fails at construction, naming context", () => {
  assert.throws(
    () =>
      createSnapshotLanguageService({
        createSnapshot: () => {
          throw new Error("not reached");
        },
        // @ts-expect-error the option was removed; a host still passing it must be told so.
        prelude: { source: PANEL },
      }),
    /context/,
  );
});

test("document symbols through host context are the document's own", async () => {
  const record = newRecord();
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
  const record = newRecord();
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
  // Only the client's own documents take this request's versions; a context document has none.
  assert.equal(report.documents.find((document) => document.uri === FORM)!.version, 4);
  assert.equal(unversioned.documents.find((document) => document.uri === FORM)!.version, null);

  // A one-character change is a different document set, so it is analyzed afresh.
  await service.diagnostics({ documents: [{ uri: FORM, source: SOURCE.replace("light", "dark ") }], uri: FORM });
  assert.equal(record.analyses, 2);
});

test("a cancelled signal rejects with an abort error and asks nothing of the snapshot", async () => {
  const record = newRecord();
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
  const record = newRecord();
  const service = serviceOver(record);
  const request = { documents: [{ uri: FORM, source: SOURCE }], uri: FORM };

  await service.diagnostics(request);
  await service.diagnostics(request);
  assert.equal(record.analyses, 1);

  service.dispose();
  await service.diagnostics(request);
  assert.equal(record.analyses, 2);
});
