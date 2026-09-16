import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
  explainNxIr,
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

/** The parts of a schema 3 artifact these tests read. */
type IrRuntimeModule = typeof import("@nx-lang/ir-runtime");

/**
 * The explained text of an image, the way `nxlang ir explain` prints it: every index resolved, a
 * record's fields indented under `record Name`, and a type from another module spelled
 * `identity:Name`.
 */
function explainedLines(image: Buffer): readonly string[] {
  return explainNxIr(image).split("\n");
}

/** The declared type of `fieldName` in the record `name`, as the explained text spells it. */
function irRecordFieldType(lines: readonly string[], name: string, fieldName: string): string {
  const start = lines.findIndex((line) => line.startsWith(`record ${name}`));
  if (start < 0) {
    throw new Error(`Expected IR declaration '${name}'.`);
  }
  for (let index = start + 1; index < lines.length && lines[index]!.startsWith("  "); index += 1) {
    const match = /^  (\w+): (\S+)/.exec(lines[index]!);
    if (match !== null && match[1] === fieldName) {
      return match[2]!;
    }
  }
  throw new Error(`Expected IR record field '${fieldName}'.`);
}

const corpusRoot = join(process.cwd(), "../../specs/ir-conformance");

async function importIrRuntime(): Promise<IrRuntimeModule> {
  return import("@nx-lang/ir-runtime");
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

  it("resolves catalog controls through implicit imports without an import line", () => {
    const { registry, buildContext } = createContext();
    const workspace = new NxWorkspace([
      { identity: "drawnui.nx", source: "export external component <SkiaLabel Text:string />" },
      { identity: "input.nx", source: 'let root() = <SkiaLabel Text="hi" />' }
    ]);

    try {
      expect(workspace.validate(buildContext, { implicitImports: ["drawnui.nx"] })).toEqual([]);
      const artifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: "input.nx",
        implicitImports: ["drawnui.nx"]
      });
      try {
        expect(artifact.evaluateJson()).toMatchObject({ $type: "SkiaLabel", Text: "hi" });
      } finally {
        artifact.dispose();
      }

      const missing = workspace.validate(buildContext, { implicitImports: ["missing.nx"] });
      expect(missing.some((diagnostic) => diagnostic.code === "implicit-import-not-found")).toBe(true);
    } finally {
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

  it("generates NX IR for directory-loaded cross-library type graphs", () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "nx-sdk-node-"));
    const flowStepDir = join(tempRoot, "flow-step");
    const uiDir = join(tempRoot, "ui");
    const questionFlowDir = join(tempRoot, "question-flow");
    const chatLinkDir = join(tempRoot, "chat-link");
    mkdirSync(flowStepDir, { recursive: true });
    mkdirSync(uiDir, { recursive: true });
    mkdirSync(questionFlowDir, { recursive: true });
    mkdirSync(chatLinkDir, { recursive: true });
    writeFileSync(join(flowStepDir, "FlowStep.nx"), "export type FlowStep = { id:string }");
    writeFileSync(join(uiDir, "TextInput.nx"), "export external component <TextInput value:string />");
    writeFileSync(
      join(questionFlowDir, "QuestionFlow.nx"),
      `import { FlowStep } from "../flow-step"
import { TextInput } from "../ui"
export type QuestionFlow = { firstStep:FlowStep input:TextInput }`
    );
    writeFileSync(
      join(chatLinkDir, "ChatLinkConfig.nx"),
      `import { QuestionFlow } from "../question-flow"
export type ChatLinkConfig = { questionFlow:QuestionFlow }`
    );

    const registry = new NxLibraryRegistry();
    registry.loadFromDirectory(questionFlowDir);
    registry.loadFromDirectory(chatLinkDir);
    const buildContext = registry.createBuildContext();
    const workspace = new NxWorkspace([
      {
        identity: "app/main.nx",
        source: `import { ChatLinkConfig } from "../chat-link"
let root() = { "ready" }`
      }
    ]);

    try {
      expect(workspace.validate(buildContext)).toEqual([]);
      const artifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: "app/main.nx"
      });

      try {
        expect(artifact.evaluateJson()).toBe("ready");
        // Every module of the program, each as its own artifact.
        const artifacts = artifact.generateNxIr({ modules: [] });
        const byIdentity = new Map(artifacts.map((entry) => [entry.identity, explainedLines(entry.bytes)]));
        expect(artifacts[0]!.identity).toBe("app/main.nx");
        for (const entry of artifacts) {
          expect(entry.metadata.identity).toBe(entry.identity);
          expect(byIdentity.get(entry.identity)![0]).toBe(
            `module ${entry.identity} fingerprint ${entry.metadata.fingerprint}`
          );
        }

        const chatLink = [...byIdentity.entries()].find(([identity]) => identity.endsWith("ChatLinkConfig.nx"))!;
        const questionFlow = [...byIdentity.entries()].find(([identity]) => identity.endsWith("QuestionFlow.nx"))!;

        // Each nominal type names the module that declares it, so the reference survives a
        // regeneration of that module.
        expect(irRecordFieldType(chatLink[1], "ChatLinkConfig", "questionFlow")).toBe(`${questionFlow[0]}:QuestionFlow`);
        expect(irRecordFieldType(questionFlow[1], "QuestionFlow", "firstStep")).toMatch(/FlowStep\.nx:FlowStep$/);
        expect(irRecordFieldType(questionFlow[1], "QuestionFlow", "input")).toMatch(/TextInput\.nx:TextInput$/);
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

  it("evaluates emitted workspace IR through the TypeScript runtime like native JSON", async () => {
    const tempRoot = mkdtempSync(join(tmpdir(), "nx-sdk-node-"));
    const flowDir = join(tempRoot, "flow");
    const uiDir = join(tempRoot, "ui");
    mkdirSync(flowDir, { recursive: true });
    mkdirSync(uiDir, { recursive: true });
    writeFileSync(
      join(flowDir, "Flow.nx"),
      `export type FlowCompletion = continue | end { message:string }
export type QuestionFlow = {
  completion:FlowCompletion?
  content steps:object
}`
    );
    writeFileSync(join(uiDir, "Panel.nx"), "export external component <Panel content body:object />");

    const registry = new NxLibraryRegistry();
    registry.loadFromDirectory(flowDir);
    registry.loadFromDirectory(uiDir);
    const buildContext = registry.createBuildContext();
    const workspace = new NxWorkspace([
      {
        identity: "app/main.nx",
        source: `import { QuestionFlow } from "../flow"
import { Panel } from "../ui"
let omitted(): QuestionFlow = { <QuestionFlow><Panel><span /></Panel></QuestionFlow> }
let explicit(): QuestionFlow = { <QuestionFlow completion={null}><Panel><span /></Panel></QuestionFlow> }
let root(): QuestionFlow[] = { omitted() explicit() }`
      }
    ]);

    try {
      expect(workspace.validate(buildContext)).toEqual([]);
      const artifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: "app/main.nx"
      });

      try {
        const irRuntime = await importIrRuntime();
        const modules = new Map(
          artifact
            .generateNxIr({ modules: [] })
            .map((entry) => [entry.identity, irRuntime.prepareNxIrModule(entry.bytes)])
        );
        const program = irRuntime.linkNxIrProgram(modules.get("app/main.nx")!, {
          resolve: (identity) => modules.get(identity)
        });
        expect(irRuntime.evaluateFunction(program, "root")).toEqual(artifact.evaluateJson());
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

  it("generates deterministic NX IR images and metadata", () => {
    const source = "let root() = { 42 }";
    const first = generateNxIrFromSource(source);
    const second = generateNxIrFromSource(Buffer.from(source));

    expect(Buffer.isBuffer(first.bytes)).toBe(true);
    expect(first.bytes.subarray(0, 4).toString("latin1")).toBe("NXIR");
    expect(first.bytes.equals(second.bytes)).toBe(true);
    expect(first.metadata.fingerprint).toBe(second.metadata.fingerprint);
    expect(typeof first.metadata.fingerprint).toBe("string");
    expect(first.metadata.runtimeAbi).toContain("nx-ir-runtime");
    expect(first.metadata.functionEntrypoints).toContain("root");
  });

  // `snippet` gives its catalog a version and imports it implicitly; `two-module` has neither.
  it.each([
    ["two-module", ["app/main.nx", "shared/model.nx"]],
    ["snippet", ["drawnui.nx", "input.nx"]]
  ])("emits the conformance corpus's %s images byte for byte", (program, identities) => {
    const dir = join(corpusRoot, program);
    const manifest = JSON.parse(readFileSync(join(dir, "program.json"), "utf8")) as {
      entry: string;
      implicitImports?: string[];
      versions?: Record<string, string>;
    };
    const workspace = new NxWorkspace(
      identities.map((identity) => ({
        identity,
        source: readFileSync(join(dir, identity), "utf8"),
        ...(manifest.versions?.[identity] === undefined ? {} : { version: manifest.versions[identity] })
      }))
    );
    const { registry, buildContext } = createContext();
    try {
      const artifact = NxProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: manifest.entry,
        implicitImports: manifest.implicitImports ?? []
      });
      try {
        for (const debug of [false, true]) {
          for (const entry of artifact.generateNxIr({ modules: [], debug })) {
            const file = `${entry.identity.replaceAll("/", "__")}${debug ? "" : ".stripped"}.nxir`;
            const expected = readFileSync(join(dir, "expected", file));
            expect(entry.bytes.equals(expected), `${file} differs`).toBe(true);
          }
        }
      } finally {
        artifact.dispose();
      }
    } finally {
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("explains an image and refuses bytes that are not one", () => {
    const generated = generateNxIrFromSource("let root() = { 42 }");
    const text = explainNxIr(generated.bytes);
    expect(text).toContain("function root() =\n  42\n");
    expect(explainNxIr(new Uint8Array(generated.bytes))).toBe(text);

    const corpus = join(corpusRoot, "snippet/expected");
    expect(explainNxIr(readFileSync(join(corpus, "input.nx.nxir")))).toBe(
      readFileSync(join(corpus, "input.nx.nxir.txt"), "utf8")
    );

    expect(() => explainNxIr(generated.bytes.subarray(0, 8))).toThrowError(NxEvaluationError);
    expect(() => explainNxIr(Buffer.from("{}"))).toThrowError(/not an NX IR image/);
  });

  it("preserves source labels for IR generation diagnostics", () => {
    const error = captureEvaluationError(() => {
      generateNxIrFromSource(`external component <SearchBox emits { SearchRequested { query:string } } />
action DoSearch = { query:string }
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
