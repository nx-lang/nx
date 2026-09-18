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
      expect(ir.metadata.schemaVersion).toBe(4);
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

  it("throws a disposed-resource error after dispose, and tolerates a second dispose", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }");
    artifact.dispose();

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
