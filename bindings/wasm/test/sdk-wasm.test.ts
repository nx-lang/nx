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

  it("builds a program and emits NX IR with metadata", () => {
    const artifact = host.buildProgramArtifact("let root() = { 42 }", { fileName: "demo.nx" });
    try {
      const ir = artifact.generateNxIr();

      const parsed = JSON.parse(ir.json) as { schemaVersion: unknown };
      expect(parsed.schemaVersion).toBe(ir.metadata.schemaVersion);
      // Compact: the IR travels inside shares and between threads, where nobody reads the text.
      expect(ir.json).not.toContain("\n");
      expect(ir.metadata.schemaVersion).toBeTypeOf("number");
      expect(ir.metadata.programFingerprint).toBeTypeOf("string");
      expect(ir.metadata.runtimeAbi).toBeTypeOf("string");
      expect(Array.isArray(ir.metadata.functionEntrypoints)).toBe(true);
    } finally {
      artifact.dispose();
    }
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
