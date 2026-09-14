/**
 * The worker's session, driven under Node against the same module the browser loads.
 *
 * The shell around it (`nx.worker.ts`) only fetches the module and wires `onmessage`; everything
 * with behaviour — compiling against the catalog, answering language queries, and recovering from
 * a crashed host — is here.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { NxHostCrashedError, createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { createNxSession } from "./session.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const module = await loadNxModule();
const uri = "nx://playground/playground.nx";

const session = createNxSession({ module, catalog });
after(() => session.dispose());

test("answers a valid compile with NX IR and no diagnostics", async () => {
  const result = await session.answer({
    kind: "compile",
    id: 1,
    source: 'let root() = { <SkiaLabel Text="hi" /> }',
  });
  assert.deepEqual(result.diagnostics, []);
  assert.equal(result.ir.format, "nx-ir-json");
});

test("answers a compile that fails with diagnostics in the author's own coordinates", async () => {
  const result = await session.answer({
    kind: "compile",
    id: 2,
    source: 'let root() = {\n  <SkiaLabel Text=1.0 />\n}',
  });
  assert.equal(result.ir, null);
  assert.equal(result.diagnostics.length, 1);
  assert.equal(result.diagnostics[0].origin, "source");
  assert.equal(result.diagnostics[0].span.startLine, 2);
});

test("answers language queries through the catalog prelude", async () => {
  const documents = [{ uri, source: '<SkiaLabel Text="hi" />\n', version: 1 }];

  const hover = await session.answer({
    kind: "language",
    id: 3,
    query: "hover",
    request: { documents, uri, position: { line: 0, character: 3 } },
  });
  assert.ok(hover !== null, "the catalog's SkiaLabel should be known");
  assert.match(hover.contents, /SkiaLabel/);
  // The catalog is a prelude, so the range is in the author's own first line.
  assert.equal(hover.range.start.line, 0);

  const symbols = await session.answer({
    kind: "language",
    id: 4,
    query: "documentSymbols",
    request: { documents, uri },
  });
  assert.ok(Array.isArray(symbols));

  const report = await session.answer({
    kind: "language",
    id: 5,
    query: "diagnostics",
    request: { documents, uri },
  });
  assert.deepEqual(report.documents[0].diagnostics, []);
});

test("a crashed host is reported once and replaced, and the next request is answered", async () => {
  // One host that crashes on its first build, then the real thing. That is what a trap looks like
  // from the session's side; the trap itself is proved against a real one in the wasm SDK's tests.
  let crashed = false;
  const crashing = createNxSession({
    module,
    catalog,
    createHost: (compiled) => {
      const host = createNxHost(compiled);
      if (crashed) {
        return host;
      }
      return {
        get crashed() {
          return host.crashed;
        },
        get memoryBytes() {
          return host.memoryBytes;
        },
        buildProgramArtifact: () => {
          crashed = true;
          throw new NxHostCrashedError("nx_wasm_program_build");
        },
        createLanguageSnapshot: (documents) => host.createLanguageSnapshot(documents),
        dispose: () => host.dispose(),
      };
    },
  });

  try {
    await assert.rejects(
      () => crashing.answer({ kind: "compile", id: 6, source: "let root() = { 1 }" }),
      (error) => error.name === "NxHostCrashedError",
    );
    assert.equal(crashing.replacements, 1);

    const result = await crashing.answer({
      kind: "compile",
      id: 7,
      source: 'let root() = { <SkiaLabel Text="hi" /> }',
    });
    assert.deepEqual(result.diagnostics, []);
    assert.equal(result.ir.format, "nx-ir-json");
    assert.equal(crashing.replacements, 1);
  } finally {
    crashing.dispose();
  }
});
