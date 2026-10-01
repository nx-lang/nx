import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { evaluateFunction, prepareNxIrModule, prepareNxIrProgram } from "@nx-lang/ir-runtime";
import { beforeAll, describe, expect, it } from "vitest";

import { NxDisposedResourceError, NxEvaluationError } from "../src/errors.js";
import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

describe("program artifacts", () => {
  let host: NxHost;

  beforeAll(() => {
    host = createNxHost(nxModule);
  });

  it("builds a program and emits an NX IR image with metadata", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }", { fileName: "demo.nx" });
    try {
      const artifacts = artifact.generateNxIr();
      expect(artifacts).toHaveLength(1);
      const ir = artifacts[0]!;

      expect(ir.bytes).toBeInstanceOf(Uint8Array);
      expect(new TextDecoder().decode(ir.bytes.subarray(0, 4))).toBe("NXIR");
      expect(new DataView(ir.bytes.buffer, ir.bytes.byteOffset).getUint32(4, true)).toBe(ir.metadata.schemaVersion);
      expect(ir.bytes.byteLength % 4).toBe(0);
      expect(ir.identity).toBe("demo.nx");
      expect(ir.metadata.identity).toBe("demo.nx");
      expect(ir.metadata.schemaVersion).toBe(5);
      expect(ir.metadata.fingerprint).toBeTypeOf("string");
      expect(ir.metadata.runtimeAbi).toBe("nx-ir-runtime-v2");
      expect(ir.metadata.functionEntrypoints).toEqual(["root"]);

      // The image is the caller's: it reads the same after the artifact is released.
      const prepared = prepareNxIrProgram(ir.bytes);
      artifact.dispose();
      expect(evaluateFunction(prepared, "root")).toBe(42);
      expect(evaluateFunction(prepareNxIrProgram(ir.bytes), "root")).toBe(42);
    } finally {
      artifact.dispose();
    }
  });

  it("compiles every property shape and records each field's occurrence", () => {
    // `name:T`, `name?:T`, `name:T+` and `name?:T+`: the four shapes a property slot admits.
    const source = `export type Person = { name:string }
export type Book = { title:string author?:Person tags:string+ extras?:string+ }
let root() = <Book title="A" tags={"x"} />`;
    const artifact = host.buildProgramArtifact(source, { fileName: "shapes.nx" });
    try {
      const [ir] = artifact.generateNxIr();
      expect(ir!.metadata.schemaVersion).toBe(5);
      expect(host.explainNxIr(ir!.bytes)).toContain(
        "record Book\n  title: string required\n  author: Person?\n  tags: string+ required\n  extras: string*\n"
      );

      // The image runs: a lone child at a `+` field is a one-element array, and an omitted optional
      // field is an omitted key, not `null`.
      expect(evaluateFunction(prepareNxIrProgram(ir!.bytes), "root")).toEqual({
        $type: "Book",
        title: "A",
        tags: ["x"]
      });
    } finally {
      artifact.dispose();
    }
  });

  it("emits the debug section on request, and nothing else changes", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }", { fileName: "demo.nx" });
    try {
      const stripped = artifact.generateNxIr()[0]!.bytes;
      const debug = artifact.generateNxIr({ debug: true })[0]!.bytes;
      const prepared = prepareNxIrProgram(debug);
      expect(prepared.entry.module.artifact.source).toBe("let root() = { 42 }");
      expect(prepareNxIrProgram(stripped).entry.module.artifact.hasDebug).toBe(false);
      // Past the header and directory, the debug image is the stripped image's sections followed
      // by the debug section.
      const strippedBody = stripped.subarray(16 + 6 * 12);
      const debugBody = debug.subarray(16 + 7 * 12, 16 + 7 * 12 + strippedBody.byteLength);
      expect(Buffer.compare(Buffer.from(debugBody), Buffer.from(strippedBody))).toBe(0);
    } finally {
      artifact.dispose();
    }
  });

  it("explains an image as the CLI does, and refuses bytes that are not one", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }", { fileName: "demo.nx" });
    try {
      const [ir] = artifact.generateNxIr();
      const text = host.explainNxIr(ir!.bytes);
      expect(text).toContain("module demo.nx fingerprint ");
      expect(text).toContain("function root() =\n  42\n");

      expect(() => host.explainNxIr(ir!.bytes.subarray(0, 8))).toThrowError(NxEvaluationError);
      expect(() => host.explainNxIr(new TextEncoder().encode("{}"))).toThrowError(/not an NX IR image/);
      const old = ir!.bytes.slice();
      new DataView(old.buffer).setUint32(4, 2, true);
      expect(() => host.explainNxIr(old)).toThrowError(/schema version 2/);
      expect(host.crashed).toBe(false);
    } finally {
      artifact.dispose();
    }
  });

  it("explains the corpus snippet exactly as its committed text", () => {
    const corpus = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../specs/ir-conformance/snippet/expected");
    const image = new Uint8Array(readFileSync(path.join(corpus, "input.nx.nxir")));
    const expected = readFileSync(path.join(corpus, "input.nx.nxir.txt"), "utf8");
    expect(host.explainNxIr(image)).toBe(expected);
  });

  const catalog = 'export external component <SkiaLabel Text:string FontSize:int = 14 />';

  it("emits a snippet without the catalog it uses, naming the catalog's version", () => {
    const artifact = host.buildWorkspaceArtifact({
      modules: [
        { identity: "drawnui.nx", source: catalog, version: "9" },
        { identity: "input.nx", source: 'let root() = <SkiaLabel Text="hi" />' }
      ],
      entry: "input.nx",
      implicitImports: ["drawnui.nx"]
    });
    try {
      const artifacts = artifact.generateNxIr();
      expect(artifacts.map((entry) => entry.identity)).toEqual(["input.nx"]);
      const ir = prepareNxIrModule(artifacts[0]!.bytes).artifact;
      expect(ir.modules.map((entry) => entry.identity)).toEqual(["input.nx", "drawnui.nx"]);
      expect(ir.modules[1]!.version).toBe("9");
      const strings = Array.from({ length: ir.stringCount }, (_, index) => ir.string(index));
      expect(strings).not.toContain("FontSize");
      expect(ir.hasDebug).toBe(false);
    } finally {
      artifact.dispose();
    }
  });

  it("emits a catalog on its own", () => {
    const artifact = host.buildWorkspaceArtifact({
      modules: [{ identity: "drawnui.nx", source: catalog, version: "9" }],
      entry: "drawnui.nx"
    });
    try {
      const [ir] = artifact.generateNxIr();
      const parsed = prepareNxIrModule(ir!.bytes).artifact;
      expect(parsed.modules).toEqual([expect.objectContaining({ identity: "drawnui.nx", version: "9" })]);
      expect(ir!.metadata.componentEntrypoints).toEqual(["SkiaLabel"]);
    } finally {
      artifact.dispose();
    }
  });

  it("reports a workspace build failure against the module it belongs to", () => {
    let thrown: unknown;
    try {
      host.buildWorkspaceArtifact({
        modules: [
          { identity: "drawnui.nx", source: catalog },
          { identity: "input.nx", source: 'let root() =\n  <SkiaLabel Text={ 1 - "x" } />' }
        ],
        entry: "input.nx",
        implicitImports: ["drawnui.nx"]
      });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(NxEvaluationError);
    const label = (thrown as NxEvaluationError).diagnostics[0]!.labels[0]!;
    expect(label.file).toBe("input.nx");
    expect(label.span.startLine).toBe(2);
  });

  it("reports a build failure as diagnostics carrying spans against the given file name", () => {
    let thrown: unknown;
    try {
      host.buildProgramArtifact("let broken(): int = { \"oops\" }", { fileName: "broken.nx" });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(NxEvaluationError);
    const diagnostics = (thrown as NxEvaluationError).diagnostics;
    expect(diagnostics.length).toBeGreaterThan(0);

    const first = diagnostics[0]!;
    expect(first.severity).toBe("error");
    expect(first.message).not.toBe("");

    const label = first.labels[0]!;
    expect(label.file).toBe("broken.nx");
    expect(label.span.endByte).toBeGreaterThan(label.span.startByte);
    expect(label.span.startLine).toBeGreaterThan(0);
    expect(label.span.startColumn).toBeGreaterThan(0);
  });

  it("reports the warnings of a build that succeeds, against the given file name", () => {
    const artifact = host.buildProgramArtifact("/// See [Missing].\nlet root() = { 42 }", {
      fileName: "linked.nx"
    });
    try {
      const diagnostics = artifact.diagnostics();
      expect(diagnostics.map((diagnostic) => [diagnostic.severity, diagnostic.code])).toEqual([
        ["warning", "unresolved-doc-link"]
      ]);
      const label = diagnostics[0]!.labels[0]!;
      expect(label.file).toBe("linked.nx");
      expect([label.span.startLine, label.span.startColumn]).toEqual([1, 9]);
      expect(artifact.evaluateNx().text).toBe("42");

      const clean = host.buildProgramArtifact("let root() = { 42 }");
      expect(clean.diagnostics()).toEqual([]);
      clean.dispose();
    } finally {
      artifact.dispose();
    }
  });

  it("throws a disposed-resource error after dispose, and tolerates a second dispose", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }");
    artifact.dispose();

    expect(() => artifact.diagnostics()).toThrowError(NxDisposedResourceError);
    expect(() => artifact.generateNxIr()).toThrowError(NxDisposedResourceError);
    expect(() => artifact.generateNxIr()).toThrowError(/NxProgramArtifact has been disposed/);
    expect(() => artifact.dispose()).not.toThrow();
  });
});

describe("language snapshots", () => {
  let host: NxHost;

  beforeAll(() => {
    host = createNxHost(nxModule);
  });

  it("answers the four queries over in-memory documents", () => {
    const uri = "nx://demo/input.nx";
    const snapshot = host.createLanguageSnapshot([{ uri, source: "let root() = { 42 }", version: 3 }]);
    try {
      expect(snapshot.diagnostics()).toHaveProperty("documents");
      expect(snapshot.completions(uri, { line: 0, character: 0 })).toHaveProperty("items");
      expect(Array.isArray(snapshot.documentSymbols(uri))).toBe(true);

      const hover = snapshot.hover(uri, { line: 0, character: 4 });
      expect(hover === null || typeof hover === "object").toBe(true);
    } finally {
      snapshot.dispose();
    }
  });

  it("reports an unparseable URI as an evaluation error naming it", () => {
    let thrown: unknown;
    try {
      host.createLanguageSnapshot([{ uri: ":::", source: "" }]);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(NxEvaluationError);
    expect((thrown as NxEvaluationError).diagnostics[0]?.code).toBe(
      "language-snapshot-input-error"
    );
    expect((thrown as Error).message).toContain(":::");
  });

  it("throws a disposed-resource error after dispose, and tolerates a second dispose", () => {
    const uri = "nx://demo/input.nx";
    const snapshot = host.createLanguageSnapshot([{ uri, source: "let root() = { 42 }" }]);
    snapshot.dispose();

    expect(() => snapshot.diagnostics()).toThrowError(NxDisposedResourceError);
    expect(() => snapshot.hover(uri, { line: 0, character: 0 })).toThrowError(
      /NxLanguageSnapshot has been disposed/
    );
    expect(() => snapshot.dispose()).not.toThrow();
  });
});

describe("evaluating root to NX text", () => {
  let host: NxHost;

  beforeAll(() => {
    host = createNxHost(nxModule);
  });

  function evaluate(source: string) {
    const artifact = host.buildProgramArtifact(source);
    try {
      return artifact.evaluateNx();
    } finally {
      artifact.dispose();
    }
  }

  function failure(source: string): NxEvaluationError {
    const artifact = host.buildProgramArtifact(source);
    try {
      artifact.evaluateNx();
    } catch (error) {
      expect(error).toBeInstanceOf(NxEvaluationError);
      return error as NxEvaluationError;
    } finally {
      artifact.dispose();
    }
    throw new Error("evaluateNx() should have thrown");
  }

  const userSource = `type User = { id:string name:string }
<User id="1" name="Ada" />`;

  it("spells a record as NX", () => {
    expect(evaluate(userSource).text).toBe('<User id="1" name="Ada" />');
  });

  it("annotates the record and its properties with types and declarations", () => {
    const value = evaluate(userSource);
    const [user, ...properties] = value.nodes;

    expect(user).toMatchObject({ start: 0, end: value.text.length, role: "record", type: "User" });
    expect(user!.parent).toBeUndefined();
    expect(user!.declaration).toMatchObject({ startLine: 1, startColumn: 1 });

    expect(properties.map((node) => node.name)).toEqual(["id", "name"]);
    for (const property of properties) {
      expect(property).toMatchObject({ role: "property", type: "string", parent: 0 });
      expect(property.declaration?.startLine).toBe(1);
    }
    expect(value.text.slice(properties[0]!.start, properties[0]!.end)).toBe('id="1"');
    expect(value.text.slice(properties[1]!.start, properties[1]!.end)).toBe('name="Ada"');
  });

  it("gives a sequence its item type and count, and parents each item", () => {
    const value = evaluate(`type User = { id:string }
let root(): User* = { <User id="1" /> <User id="2" /> <User id="3" /> }`);
    expect(value.nodes[0]).toMatchObject({
      role: "sequence",
      type: "User*",
      count: 3,
      start: 0,
      end: value.text.length
    });
    const records = value.nodes.filter((node) => node.role === "record");
    expect(records).toHaveLength(3);
    expect(records.every((node) => node.parent === 0)).toBe(true);
  });

  it("gives a property its declared type and marks it optional", () => {
    const value = evaluate(`type Card = { title:string subtitle?:string }
<Card title="a" subtitle="b" />`);
    const subtitle = value.nodes.find((node) => node.name === "subtitle");
    expect(subtitle).toMatchObject({ type: "string", optional: true });
    expect(value.nodes.find((node) => node.name === "title")?.optional).toBeUndefined();
  });

  it("counts offsets in UTF-16 code units", () => {
    const value = evaluate(`type Note = { a:string b:int }
<Note a="😀" b=1 />`);
    const b = value.nodes.find((node) => node.name === "b")!;
    expect(value.text.slice(b.start, b.end)).toBe("b=1");
  });

  it("reports a missing root", () => {
    const error = failure("type User = { id:string }");
    expect(error.diagnostics[0]?.code).toBe("no-root");
  });

  it("reports a runtime error at the expression that failed", () => {
    const error = failure("let root() = { 1 / 0 }");
    const diagnostic = error.diagnostics[0]!;
    expect(diagnostic.code).toBe("runtime-error");
    expect(diagnostic.message).toBe("Division by zero");
    expect(diagnostic.labels[0]?.span).toMatchObject({ startByte: 15, endByte: 20, startLine: 1 });
  });

  it("reports runaway recursion as the interpreter's limit, without trapping", () => {
    const error = failure("let f(n:int): int = { f(n + 1) }\nlet root() = { f(0) }");
    expect(error.diagnostics[0]?.code).toBe("runtime-error");
    expect(error.message).toMatch(/recursion depth/);
    expect(host.crashed).toBe(false);
    expect(evaluate("let root() = { 42 }").text).toBe("42");
  });

  it("reports a value with no NX spelling rather than returning part of it", () => {
    const error = failure(`action SearchRequested = { query:string }
action DoSearch = { query:string }
external component <SearchBox emits { SearchRequested } />
let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }`);
    expect(error.diagnostics[0]?.code).toBe("nx-text-unspellable");
    expect(error.message).toContain("action handler");
  });

  it("leaves the artifact usable after a failed evaluation", () => {
    const artifact = host.buildProgramArtifact("let root() = { 1 / 0 }");
    try {
      expect(() => artifact.evaluateNx()).toThrow(NxEvaluationError);
      expect(artifact.generateNxIr()).toHaveLength(1);
      expect(() => artifact.evaluateNx()).toThrow(NxEvaluationError);
    } finally {
      artifact.dispose();
    }
  });

  it("refuses a disposed artifact", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }");
    artifact.dispose();
    expect(() => artifact.evaluateNx()).toThrow(NxDisposedResourceError);
  });
});
