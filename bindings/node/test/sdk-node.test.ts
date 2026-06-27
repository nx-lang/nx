import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  NxDisposedResourceError,
  NxEvaluationError,
  NxLibraryRegistry,
  NxProgramArtifact,
  NxProgramBuildContext,
  NxWorkspace,
  buildProgramArtifactFromSource,
  evaluateBytesFromSource,
  evaluateJsonFromSource,
  generateNxIrFromSource
} from "../src/index.js";

function createContext(): { registry: NxLibraryRegistry; buildContext: NxProgramBuildContext } {
  const registry = new NxLibraryRegistry();
  return {
    registry,
    buildContext: registry.createBuildContext()
  };
}

function validWorkspace(): NxWorkspace {
  return new NxWorkspace([
    {
      identity: "app/main.nx",
      source: Buffer.from(`import { answer } from "../shared/value.nx"
let root(): int = { answer() }`)
    },
    {
      identity: "shared/value.nx",
      source: "export let answer(): int = { 42 }"
    }
  ]);
}

function captureEvaluationError(callback: () => void): NxEvaluationError {
  let thrown: unknown;
  try {
    callback();
  } catch (error) {
    thrown = error;
  }

  expect(thrown).toBeInstanceOf(NxEvaluationError);
  return thrown as NxEvaluationError;
}

describe("@nx-lang/sdk-node", () => {
  it("validates valid and invalid workspaces with structured diagnostics", () => {
    const { registry, buildContext } = createContext();
    const workspace = validWorkspace();
    const invalid = new NxWorkspace([
      {
        identity: "app/main.nx",
        source: `import { answer } from "../shared/missing.nx"
let root(): int = { answer }`
      },
      {
        identity: "shared/value.nx",
        source: `let broken(): int = { "oops" }`
      }
    ]);

    try {
      expect(workspace.validate(buildContext)).toEqual([]);

      const diagnostics = invalid.validate(buildContext);
      expect(diagnostics.some((diagnostic) => diagnostic.message.includes("shared/missing.nx"))).toBe(true);
      expect(diagnostics.some((diagnostic) => diagnostic.code === "return-type-mismatch")).toBe(true);
      expect(diagnostics.every((diagnostic) => diagnostic.severity === "error")).toBe(true);
    } finally {
      invalid.dispose();
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("rejects duplicate normalized module identities and missing workspace entries", () => {
    const duplicateError = captureEvaluationError(() => {
      new NxWorkspace([
        { identity: "lib/config.nx", source: "let root() = { 1 }" },
        { identity: "lib/./config.nx", source: "let root() = { 2 }" }
      ]);
    });
    expect(duplicateError.diagnostics.some((diagnostic) => diagnostic.message.includes("lib/config.nx"))).toBe(true);

    const { registry, buildContext } = createContext();
    const workspace = new NxWorkspace([{ identity: "main.nx", source: "let root() = { 42 }" }]);

    try {
      const missingEntryError = captureEvaluationError(() => {
        NxProgramArtifact.buildWorkspace(workspace, {
          buildContext,
          entryIdentity: "missing.nx"
        });
      });
      expect(missingEntryError.diagnostics.some((diagnostic) => diagnostic.message.includes("missing.nx"))).toBe(true);
    } finally {
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("builds source and workspace artifacts and evaluates JSON and bytes", () => {
    const { registry, buildContext } = createContext();
    const workspace = validWorkspace();

    try {
      const sourceArtifact = buildProgramArtifactFromSource("let root() = { \"source\" }", {
        buildContext
      });
      const workspaceArtifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: "app/main.nx"
      });

      try {
        expect(sourceArtifact.evaluateJson()).toBe("source");
        expect(workspaceArtifact.evaluateJson()).toBe(42);
        expect(Buffer.isBuffer(workspaceArtifact.evaluateBytes())).toBe(true);
        expect(workspaceArtifact.evaluateBytes({ outputFormat: "json" }).toString("utf8")).toBe("42");
      } finally {
        sourceArtifact.dispose();
        workspaceArtifact.dispose();
      }
    } finally {
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("uses supplied build contexts and keeps artifacts usable after context disposal", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "nx-sdk-node-"));
    const appDir = join(tempRoot, "app");
    const uiDir = join(tempRoot, "ui");
    mkdirSync(appDir, { recursive: true });
    mkdirSync(uiDir, { recursive: true });
    writeFileSync(join(uiDir, "answer.nx"), "export let answer(): int = { 42 }");

    const registry = new NxLibraryRegistry();
    registry.loadFromDirectory(uiDir);
    const buildContext = registry.createBuildContext();
    const source = `import { answer } from "../ui"
let root(): int = { answer() }`;
    const fileName = join(appDir, "main.nx");
    const workspace = new NxWorkspace([{ identity: "app/main.nx", source }]);

    try {
      expect(evaluateJsonFromSource(source, { buildContext, fileName })).toBe(42);

      const artifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: "app/main.nx"
      });
      buildContext.dispose();
      registry.dispose();

      try {
        expect(artifact.evaluateJson()).toBe(42);
      } finally {
        artifact.dispose();
      }
    } finally {
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
      rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  it("generates deterministic NX IR JSON and metadata", () => {
    const source = "let root() = { 42 }";
    const first = generateNxIrFromSource(source);
    const second = generateNxIrFromSource(Buffer.from(source));

    expect(first.json).toBe(second.json);
    expect(first.metadata.programFingerprint).toBe(second.metadata.programFingerprint);
    expect(first.metadata.runtimeAbi).toContain("nx-ir-runtime");
    expect(first.metadata.functionEntrypoints.some((entrypoint) => entrypoint.name === "root")).toBe(true);
  });

  it("preserves source labels for IR generation diagnostics", () => {
    const error = captureEvaluationError(() => {
      generateNxIrFromSource(`external component <SearchBox emits { SearchRequested { query:string } } />
let DoSearch(query:string) = { query }
let root() = { <SearchBox onSearchRequested=<DoSearch query={action.query} /> /> }`);
    });
    const diagnostic = error.diagnostics.find((item) => item.code === "codegen-unsupported-construct");
    if (diagnostic === undefined) {
      throw new Error("Expected codegen unsupported diagnostic.");
    }

    const label = diagnostic.labels[0];
    if (label === undefined) {
      throw new Error("Expected codegen diagnostic to include a source label.");
    }

    expect(label.file).toBe("input.nx");
    expect(label.primary).toBe(true);
    expect(label.span.endByte).toBeGreaterThan(label.span.startByte);
  });

  it("exposes source convenience evaluation and diagnostic failures", () => {
    expect(evaluateJsonFromSource("let root() = { 42 }")).toBe(42);
    expect(evaluateBytesFromSource("let root() = { 42 }", { outputFormat: "json" }).toString("utf8")).toBe("42");

    expect(() => evaluateJsonFromSource("let helper() = { 42 }")).toThrowError(NxEvaluationError);
    expect(() => evaluateJsonFromSource("let root(): int = { \"oops\" }")).toThrowError(NxEvaluationError);
  });

  it("does not fall back to JavaScript-side named entrypoint lookup", () => {
    const artifact = NxProgramArtifact.buildSource("let helper() = { 42 }\nlet root() = { 1 }");
    try {
      expect(() => artifact.evaluateJson({ entrypoint: "helper" })).toThrowError(NxEvaluationError);
    } finally {
      artifact.dispose();
    }
  });

  it("rejects disposed artifact use predictably", () => {
    const artifact = NxProgramArtifact.buildSource("let root() = { 42 }");
    artifact.dispose();

    expect(() => artifact.evaluateJson()).toThrowError(NxDisposedResourceError);
  });
});
