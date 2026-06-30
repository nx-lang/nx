import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
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

interface TestIrReference {
  readonly name: string;
  readonly kind: string;
  readonly module: string;
}

interface TestIrTypeRef {
  readonly kind: string;
  readonly display?: string;
  readonly reference?: TestIrReference;
}

interface TestIrRecordField {
  readonly name: string;
  readonly ty: TestIrTypeRef;
}

interface TestIrDeclaration {
  readonly reference: TestIrReference;
  readonly kind: {
    readonly fields?: readonly TestIrRecordField[];
  };
}

interface TestIrDocument {
  readonly programFingerprint: string;
  readonly modules: readonly {
    readonly declarations: readonly TestIrDeclaration[];
  }[];
}

type IrRuntimeModule = typeof import("../../../runtime/typescript/dist/src/index.js");

function irDeclaration(document: TestIrDocument, name: string): TestIrDeclaration {
  const declaration = document.modules
    .flatMap((module) => module.declarations)
    .find((candidate) => candidate.reference.name === name);
  if (declaration === undefined) {
    throw new Error(`Expected IR declaration '${name}'.`);
  }

  return declaration;
}

function irRecordFieldType(declaration: TestIrDeclaration, fieldName: string): TestIrTypeRef {
  const field = (declaration.kind.fields ?? []).find((candidate) => candidate.name === fieldName);
  if (field === undefined) {
    throw new Error(`Expected IR record field '${fieldName}'.`);
  }

  return field.ty;
}

async function importIrRuntime(): Promise<IrRuntimeModule> {
  const runtimeUrl = pathToFileURL(join(process.cwd(), "../../runtime/typescript/dist/src/index.js"));
  return import(runtimeUrl.href) as Promise<IrRuntimeModule>;
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
        const ir = artifact.generateNxIr();
        const document = JSON.parse(ir.json) as TestIrDocument;
        const config = irDeclaration(document, "ChatLinkConfig");
        const questionFlow = irDeclaration(document, "QuestionFlow");
        const questionFlowType = irRecordFieldType(config, "questionFlow");
        const flowStepType = irRecordFieldType(questionFlow, "firstStep");
        const inputType = irRecordFieldType(questionFlow, "input");

        expect(typeof ir.metadata.programFingerprint).toBe("string");
        expect(ir.metadata.programFingerprint).toBe(document.programFingerprint);
        expect(questionFlowType).toMatchObject({
          kind: "nominal",
          display: "QuestionFlow",
          reference: { kind: "record", name: "QuestionFlow" }
        });
        expect(questionFlowType.reference?.module).not.toBe(config.reference.module);
        expect(flowStepType).toMatchObject({
          kind: "nominal",
          display: "FlowStep",
          reference: { kind: "record", name: "FlowStep" }
        });
        expect(flowStepType.reference?.module).not.toBe(questionFlow.reference.module);
        expect(inputType).toMatchObject({
          kind: "nominal",
          display: "TextInput",
          reference: { kind: "component", name: "TextInput" }
        });
        expect(inputType.reference?.module).not.toBe(questionFlow.reference.module);
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
      `export type FlowCompletion = | continue | end { message:string }
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
        const prepared = irRuntime.prepareNxIrProgram(JSON.parse(artifact.generateNxIr().json));
        expect(irRuntime.evaluateFunction(prepared, "root")).toEqual(artifact.evaluateJson());
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
    expect(typeof first.metadata.programFingerprint).toBe("string");
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
