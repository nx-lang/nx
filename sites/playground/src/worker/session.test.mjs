/**
 * The worker's session, driven under Node against the same module the browser loads.
 *
 * The shell around it (`nx.worker.ts`) only fetches the module and wires `onmessage`; everything
 * with behaviour — evaluating, answering language queries, and recovering from a crashed host — is
 * here.
 */
import { strict as assert } from "node:assert";
import { after, test } from "node:test";
import { NxHostCrashedError, createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { MAX_OUTPUT_CHARACTERS } from "../compile/evaluate.ts";
import { createNxSession } from "./session.ts";

const module = await loadNxModule();
const uri = "nx://playground/playground.nx";

const session = createNxSession({ module });
after(() => session.dispose());

let id = 0;
const evaluate = (source) => session.answer({ kind: "evaluate", id: ++id, source });

test("answers a program with root's value as annotated NX text", async () => {
  const result = await evaluate('type User = { id:string name:string }\n<User id="1" name="Ada" />');
  assert.deepEqual(result.diagnostics, []);
  assert.equal(result.outcome.kind, "value");
  assert.equal(result.outcome.truncated, false);
  assert.equal(result.outcome.value.text, '<User id="1" name="Ada" />');
  assert.equal(result.outcome.value.nodes[0].type, "User");
  assert.equal(result.outcome.value.nodes[0].declaration.startLine, 1);
});

test("answers a program that does not compile with diagnostics in the visitor's coordinates", async () => {
  const result = await evaluate("let root() = {\n  <User id= />\n}");
  assert.equal(result.outcome, null);
  assert.ok(result.diagnostics.length > 0);
  assert.equal(result.diagnostics[0].origin, "source");
  assert.equal(result.diagnostics[0].span.startLine, 2);
});

test("answers a program with no root with that, rather than an error", async () => {
  const result = await evaluate("type User = { id:string }");
  assert.deepEqual(result.diagnostics, []);
  assert.deepEqual(result.outcome, { kind: "noRoot" });
});

test("answers a runtime error with its span in the visitor's source", async () => {
  const result = await evaluate("let root() = { 1 / 0 }");
  assert.equal(result.outcome.kind, "error");
  const [diagnostic] = result.outcome.diagnostics;
  assert.equal(diagnostic.message, "Division by zero");
  assert.equal(diagnostic.origin, "source");
  assert.equal(diagnostic.span.startColumn, 16);
});

test("answers runaway recursion with the recursion limit, and keeps the host", async () => {
  const result = await evaluate("let f(n:int): int = { f(n + 1) }\nlet root() = { f(0) }");
  assert.equal(result.outcome.kind, "error");
  assert.match(result.outcome.diagnostics[0].message, /recursion depth/);
  assert.equal(session.replacements, 0);
});

test("answers a value with no NX spelling with an error naming the part", async () => {
  const result = await evaluate(`action SearchRequested = { query:string }
action DoSearch = { query:string }
external component <SearchBox emits { SearchRequested } />
let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }`);
  assert.equal(result.outcome.kind, "error");
  assert.equal(result.outcome.diagnostics[0].code, "nx-text-unspellable");
});

test("cuts a very large value, keeping only the nodes that start within the cut", async () => {
  const result = await evaluate(
    "type Item = { n:int }\nlet root(): Item* = { for i in 0..20000 { <Item n={i} /> } }",
  );
  const { value, truncated } = result.outcome;
  assert.equal(truncated, true);
  assert.ok(value.text.length <= MAX_OUTPUT_CHARACTERS);
  // Cut after the last whole line, so the last item is complete.
  assert.match(value.text, /<Item n=\d+ \/>$/);
  assert.ok(value.nodes.length > 0);
  assert.ok(value.nodes.every((node) => node.start < value.text.length && node.end <= value.text.length));
  // The sequence ran past the cut, so it ends at it.
  assert.equal(value.nodes[0].role, "sequence");
  assert.equal(value.nodes[0].end, value.text.length);
});

test("answers hover on the visitor's own declarations, with no catalog behind them", async () => {
  const documents = [{ uri, source: "type User = { id:string name:string }\n<User id=\"1\" name=\"Ada\" />\n", version: 1 }];

  const hover = await session.answer({
    kind: "language",
    id: ++id,
    query: "hover",
    request: { documents, uri, position: { line: 0, character: 6 } },
  });
  assert.ok(hover !== null, "User is declared in the document");
  assert.match(hover.contents, /User/);
  assert.equal(hover.range.start.line, 0);

  const completions = await session.answer({
    kind: "language",
    id: ++id,
    query: "completions",
    request: {
      documents: [{ uri, source: "let <Card title:string note:string /> = <div />\n<Card  />\n", version: 2 }],
      uri,
      position: { line: 1, character: 6 },
    },
  });
  const labels = completions.items.map((item) => item.label);
  assert.deepEqual(labels, ["title", "note"]);

  const report = await session.answer({
    kind: "language",
    id: ++id,
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
        buildWorkspaceArtifact: (options) => host.buildWorkspaceArtifact(options),
        createLanguageSnapshot: (documents, options) => host.createLanguageSnapshot(documents, options),
        explainNxIr: (image) => host.explainNxIr(image),
        dispose: () => host.dispose(),
      };
    },
  });

  try {
    await assert.rejects(
      () => crashing.answer({ kind: "evaluate", id: ++id, source: "let root() = { 1 }" }),
      (error) => error.name === "NxHostCrashedError",
    );
    assert.equal(crashing.replacements, 1);

    const result = await crashing.answer({ kind: "evaluate", id: ++id, source: "let root() = { 42 }" });
    assert.equal(result.outcome.value.text, "42");
    assert.equal(crashing.replacements, 1);
  } finally {
    crashing.dispose();
  }
});

test("a trap that was the stack running out is named, and not retried as a compiler fault", async () => {
  const overflowing = createNxSession({
    module,
    createHost: (compiled) => {
      const host = createNxHost(compiled);
      return {
        get crashed() {
          return host.crashed;
        },
        get memoryBytes() {
          return host.memoryBytes;
        },
        buildProgramArtifact: () => {
          throw new NxHostCrashedError("nx_wasm_program_evaluate_nx", {
            cause: new RangeError("Maximum call stack size exceeded"),
          });
        },
        buildWorkspaceArtifact: (options) => host.buildWorkspaceArtifact(options),
        createLanguageSnapshot: (documents, options) => host.createLanguageSnapshot(documents, options),
        explainNxIr: (image) => host.explainNxIr(image),
        dispose: () => host.dispose(),
      };
    },
  });
  try {
    await assert.rejects(
      () => overflowing.answer({ kind: "evaluate", id: ++id, source: "let root() = { 1 }" }),
      (error) => error.name === "NxStackOverflowError" && /recursed deeper/.test(error.message),
    );
    assert.equal(overflowing.replacements, 1, "the crashed host is still replaced");
  } finally {
    overflowing.dispose();
  }
});
