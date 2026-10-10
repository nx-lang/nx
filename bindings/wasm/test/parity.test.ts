import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
  NxLanguageSnapshot as NodeLanguageSnapshot,
  NxLibraryRegistry as NodeLibraryRegistry,
  NxProgramArtifact as NodeProgramArtifact,
  NxWorkspace as NodeWorkspace,
  generateNxIrFromSource as generateNxIrWithNode
} from "@nx-lang/sdk-node";

import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import { corpus } from "./schema-corpus.js";
import { nxModule } from "./support.js";

/**
 * Sources chosen to reach the parts the playground uses: a value, a typed function, records, a
 * sequence, a union case, a component with children, and a program that does not compile.
 */
const sources = {
  value: "let root() = { 42 }",
  typed: "let answer(): int = { 42 }\nlet root() = { answer() }",
  record: `type Address = { city:string zip?:string }
type Person = { name:string home:Address tags?:string+ }
<Person name="Ada" home=<Address city="London" /> tags={"a" "b"} />`,
  sequence: `type Task = { title:string done:boolean = false }
let titles:string+ = { "Write" "Ship" }
let root(): Task+ = { for title in titles { <Task title={title} /> } }`,
  union: "type Status = active | retired\nlet root(): Status = retired",
  text: `type Label = { content text:string }
let <Total count:int /> = <Label>Total: {count}</Label>
<Total count=3 />`,
  component: `let <Greeting
  name:string
/> =
  <div class="greeting">
    <p:>Hello {name}</p>
  </div>

let <Page /> =
  <Greeting name="world" />`,
  broken: "let broken(): int = { \"oops\" }"
} as const;

const host: NxHost = createNxHost(nxModule);

describe("NX IR parity with the Node SDK", () => {
  for (const [name, source] of Object.entries(sources)) {
    if (name === "broken") {
      continue;
    }

    it(`emits identical IR and metadata for ${name}`, () => {
      const fileName = `${name}.nx`;
      const fromNode = generateNxIrWithNode(source, { fileName });

      const artifact = host.buildProgramArtifact(source, { fileName });
      try {
        const [fromWasm] = artifact.generateNxIr();

        expect(Buffer.compare(Buffer.from(fromWasm!.bytes), fromNode.bytes)).toBe(0);
        expect(fromWasm!.metadata).toEqual(fromNode.metadata);
      } finally {
        artifact.dispose();
      }
    });
  }

  it("records a module's version identically", () => {
    const modules = [
      { identity: "drawnui.nx", source: "export external component <SkiaLabel Text:string />", version: "9" },
      { identity: "input.nx", source: 'let root() = <SkiaLabel Text="hi" />' }
    ];
    const implicitImports = ["drawnui.nx"];
    const registry = new NodeLibraryRegistry();
    const buildContext = registry.createBuildContext();
    const workspace = new NodeWorkspace(modules);
    const fromNode = NodeProgramArtifact.buildWorkspace(workspace, {
      buildContext,
      entryIdentity: "input.nx",
      implicitImports
    });
    const fromWasm = host.buildWorkspaceArtifact({ modules, entry: "input.nx", implicitImports });
    try {
      const nodeArtifacts = fromNode.generateNxIr({ modules: [] });
      const wasmArtifacts = fromWasm.generateNxIr({ modules: [] });
      expect(wasmArtifacts.map((entry) => entry.identity)).toEqual(nodeArtifacts.map((entry) => entry.identity));
      wasmArtifacts.forEach((entry, index) => {
        expect(Buffer.compare(Buffer.from(entry.bytes), nodeArtifacts[index]!.bytes)).toBe(0);
        expect(entry.metadata).toEqual(nodeArtifacts[index]!.metadata);
      });
    } finally {
      fromWasm.dispose();
      fromNode.dispose();
      workspace.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("emits identical entry and library images, and equal diagnostics, for in-memory libraries", () => {
    const libraries = [
      {
        root: "libraries/chat-link",
        modules: [
          {
            identity: "ChatLinkConfig.nx",
            source:
              'import "../question-flow"\nexport type ChatLinkConfig = { title:string questionFlow:QuestionFlow }'
          }
        ]
      },
      {
        root: "libraries/question-flow",
        version: "3",
        modules: [
          { identity: "Step.nx", source: "export type Step = { id:string label?:string }" },
          { identity: "QuestionFlow.nx", source: "export type QuestionFlow = { firstStep:Step steps?:Step+ }" }
        ]
      },
      {
        // Not imported by the tenant: here only for the warning its load answers with.
        root: "libraries/warns",
        modules: [{ identity: "Warns.nx", source: "export let f(n:int) = { n ?? 0 }" }]
      }
    ];
    const implicitImports = ["libraries/chat-link", "libraries/question-flow"];
    const tenant = [
      {
        identity: "chat-link.nx",
        source:
          'let root() = <ChatLinkConfig title="Hi" questionFlow={<QuestionFlow firstStep={<Step id="a" />} />} />\nlet key() = {Step.Property.label}'
      }
    ];
    const broken = [{ identity: "chat-link.nx", source: "let root() = <ChatLinkConfig title={1} />" }];

    const nodeRegistry = new NodeLibraryRegistry();
    const nodeLoadDiagnostics = nodeRegistry.loadLibraries(libraries);
    const nodeContext = nodeRegistry.createBuildContext();
    const nodeWorkspace = new NodeWorkspace(tenant);
    const nodeBroken = new NodeWorkspace(broken);
    const fromNode = NodeProgramArtifact.buildWorkspace(nodeWorkspace, {
      buildContext: nodeContext,
      entryIdentity: "chat-link.nx",
      implicitImports
    });

    const wasmRegistry = host.createLibraryRegistry();
    const wasmLoadDiagnostics = wasmRegistry.loadLibraries(libraries);
    const wasmContext = wasmRegistry.createBuildContext({ implicitImports });
    const fromWasm = host.buildWorkspaceArtifact({
      modules: tenant,
      entry: "chat-link.nx",
      buildContext: wasmContext
    });
    try {
      const nodeArtifacts = fromNode.generateNxIr({ modules: [] });
      const wasmArtifacts = fromWasm.generateNxIr({ modules: [] });
      expect(wasmArtifacts.map((entry) => entry.identity)).toEqual(nodeArtifacts.map((entry) => entry.identity));
      expect(wasmArtifacts.map((entry) => entry.identity)).toContain("libraries/question-flow/Step.nx");
      wasmArtifacts.forEach((entry, index) => {
        expect(Buffer.compare(Buffer.from(entry.bytes), nodeArtifacts[index]!.bytes)).toBe(0);
        expect(entry.metadata).toEqual(nodeArtifacts[index]!.metadata);
      });

      expect(wasmLoadDiagnostics.map((diagnostic) => diagnostic.code)).toEqual(["fallback-never-taken"]);
      expect(wasmLoadDiagnostics).toEqual(nodeLoadDiagnostics);

      const nodeDiagnostics = nodeBroken.validate(nodeContext, { implicitImports });
      const wasmDiagnostics = host.validateWorkspace({ modules: broken, buildContext: wasmContext });
      expect(wasmDiagnostics.length).toBeGreaterThan(0);
      expect(wasmDiagnostics).toEqual(nodeDiagnostics);
    } finally {
      fromWasm.dispose();
      wasmContext.dispose();
      wasmRegistry.dispose();
      fromNode.dispose();
      nodeWorkspace.dispose();
      nodeBroken.dispose();
      nodeContext.dispose();
      nodeRegistry.dispose();
    }
  });

  it("reports the same diagnostics for source that does not compile", () => {
    const fileName = "broken.nx";

    const fromNode = captureDiagnostics(() => generateNxIrWithNode(sources.broken, { fileName }));
    const fromWasm = captureDiagnostics(() =>
      host.buildProgramArtifact(sources.broken, { fileName })
    );

    expect(fromWasm).toEqual(fromNode);
    expect(fromWasm.length).toBeGreaterThan(0);
  });
});

describe("language answer parity with the Node SDK", () => {
  const uri = "nx://demo/input.nx";
  const documents = [{ uri, source: sources.component, identity: "demo/input.nx", version: 7 }];

  const nodeSnapshot = new NodeLanguageSnapshot(documents);
  const wasmSnapshot = host.createLanguageSnapshot(documents);

  it("answers diagnostics identically", () => {
    expect(wasmSnapshot.diagnostics()).toEqual(nodeSnapshot.diagnostics());
  });

  it("answers document symbols identically", () => {
    expect(wasmSnapshot.documentSymbols(uri)).toEqual(nodeSnapshot.documentSymbols(uri));
  });

  it("answers the source tree identically for every source of the corpus", () => {
    for (const [name, source] of Object.entries(sources)) {
      const corpus = [{ uri, source, identity: "demo/input.nx", version: 7 }];
      const fromNode = new NodeLanguageSnapshot(corpus);
      const fromWasm = host.createLanguageSnapshot(corpus);
      try {
        expect(JSON.parse(JSON.stringify(fromWasm.sourceTree(uri))), name).toEqual(
          fromNode.sourceTree(uri)
        );
      } finally {
        fromWasm.dispose();
        fromNode.dispose();
      }
    }
  });

  it("answers hover and completions identically at every line start", () => {
    const lineCount = sources.component.split("\n").length;

    for (let line = 0; line < lineCount; line += 1) {
      for (const character of [0, 2, 5]) {
        const position = { line, character };
        expect(wasmSnapshot.hover(uri, position), `hover at ${line}:${character}`).toEqual(
          nodeSnapshot.hover(uri, position)
        );
        expect(
          wasmSnapshot.completions(uri, position),
          `completions at ${line}:${character}`
        ).toEqual(nodeSnapshot.completions(uri, position));
      }
    }
  });
});

function captureDiagnostics(run: () => unknown): readonly unknown[] {
  try {
    run();
  } catch (error) {
    const diagnostics = (error as { diagnostics?: readonly unknown[] }).diagnostics;
    if (diagnostics === undefined) {
      throw error;
    }
    return diagnostics;
  }

  throw new Error("Expected the source not to compile.");
}

describe("declaration schema parity with the Node SDK", () => {
  for (const entry of corpus) {
    it(`answers identical schemas for ${entry.identity}`, () => {
      const modules = [{ identity: entry.identity, source: entry.source }];
      const wasmArtifact = host.buildWorkspaceArtifact({ modules, entry: entry.identity });
      const registry = new NodeLibraryRegistry();
      const buildContext = registry.createBuildContext();
      const workspace = new NodeWorkspace(modules);
      const nodeArtifact = NodeProgramArtifact.buildWorkspace(workspace, {
        buildContext,
        entryIdentity: entry.identity
      });
      try {
        for (const name of Object.keys(entry.golden)) {
          const fromWasm = wasmArtifact.functionSchema({ name });
          const fromNode = nodeArtifact.functionSchema({ name });
          // Schema documents, documentation, parameters and diagnostics alike, as JSON text.
          expect(JSON.stringify(fromWasm), `${entry.identity} ${name}`).toBe(JSON.stringify(fromNode));
        }
      } finally {
        wasmArtifact.dispose();
        nodeArtifact.dispose();
        workspace.dispose();
        buildContext.dispose();
        registry.dispose();
      }
    });
  }

  it("answers identically with host-supplied types", () => {
    const source = [
      'import "@nx/agent"',
      "type ChatContext extends ToolContext = { conversationId:string }",
      "type Request = { context:ChatContext }",
      "let lookupOrder(orderId:string, context:ChatContext): HttpArguments = { <HttpArguments /> }",
      "let send(contexts:ChatContext+, request:Request): string = { \"\" }"
    ].join("\n");
    const options = { hostSuppliedTypes: [{ module: "@nx/agent/agent.nx", name: "ToolContext" }] };
    const wasmArtifact = host.buildProgramArtifact(source, { fileName: "tools.nx" });
    const registry = new NodeLibraryRegistry();
    const buildContext = registry.createBuildContext();
    const nodeArtifact = NodeProgramArtifact.buildSource(source, { fileName: "tools.nx", buildContext });
    try {
      const fromWasm = wasmArtifact.functionSchema({ name: "lookupOrder" }, options);
      expect(fromWasm.parameters[1]!.hostSupplied).toEqual(options.hostSuppliedTypes[0]);
      expect(JSON.stringify(fromWasm)).toBe(
        JSON.stringify(nodeArtifact.functionSchema({ name: "lookupOrder" }, options))
      );
      // And for parameters that hold a listed type without being one.
      const heldFromWasm = wasmArtifact.functionSchema({ name: "send" }, options);
      expect(heldFromWasm.parameters.map((parameter) => parameter.hostSuppliedWithin)).toEqual([
        options.hostSuppliedTypes,
        options.hostSuppliedTypes
      ]);
      expect(JSON.stringify(heldFromWasm)).toBe(JSON.stringify(nodeArtifact.functionSchema({ name: "send" }, options)));
    } finally {
      wasmArtifact.dispose();
      nodeArtifact.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });

  it("answers identical type schemas in both directions, and for a type with no JSON form", () => {
    const source = [
      "type Booking = { host:string durationMinutes:int = 30 }",
      "type Row = { template:<function Item:string />: string }"
    ].join("\n");
    const wasmArtifact = host.buildProgramArtifact(source, { fileName: "types.nx" });
    const registry = new NodeLibraryRegistry();
    const buildContext = registry.createBuildContext();
    const nodeArtifact = NodeProgramArtifact.buildSource(source, { fileName: "types.nx", buildContext });
    try {
      for (const name of ["Booking", "Row"]) {
        for (const direction of ["input", "output"] as const) {
          expect(JSON.stringify(wasmArtifact.typeSchema({ name }, { direction }))).toBe(
            JSON.stringify(nodeArtifact.typeSchema({ name }, { direction }))
          );
        }
      }
      expect(wasmArtifact.typeSchema({ name: "Row" }).diagnostics[0]!.code).toBe(
        "schema-inexpressible-type"
      );
    } finally {
      wasmArtifact.dispose();
      nodeArtifact.dispose();
      buildContext.dispose();
      registry.dispose();
    }
  });
});

describe("evaluation parity with the command line", () => {
  const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
  let nxlang: string;
  let directory: string;

  beforeAll(() => {
    nxlang = buildCommandLine(repositoryRoot);
    directory = mkdtempSync(path.join(tmpdir(), "nx-parity-"));
  }, 600_000);

  afterAll(() => {
    rmSync(directory, { recursive: true, force: true });
  });

  for (const [name, source] of Object.entries(sources)) {
    if (!/\blet root\(|^<|\n</.test(source)) {
      continue;
    }

    it(`prints what nxlang run prints for ${name}`, () => {
      const file = path.join(directory, `${name}.nx`);
      writeFileSync(file, source);
      const printed = execFileSync(nxlang, ["run", file], { encoding: "utf8" });

      const artifact = host.buildProgramArtifact(source, { fileName: `${name}.nx` });
      try {
        // `nxlang run` ends its output with a newline; the text is the value alone.
        expect(artifact.evaluateNx().text).toBe(printed.replace(/\n$/, ""));
      } finally {
        artifact.dispose();
      }
    });
  }
});

describe("recursion past the module's limit", () => {
  const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

  it("is an error here and a value from nxlang run, which allows deeper calls", () => {
    const source = "let f(n:int): int = { if n == 0 { 0 } else { f(n - 1) } }\nlet root() = { f(250) }";
    const directory = mkdtempSync(path.join(tmpdir(), "nx-parity-"));
    try {
      const file = path.join(directory, "deep.nx");
      writeFileSync(file, source);
      expect(execFileSync(buildCommandLine(repositoryRoot), ["run", file], { encoding: "utf8" })).toBe("0\n");
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }

    const artifact = host.buildProgramArtifact(source);
    try {
      expect(() => artifact.evaluateNx()).toThrow(/recursion depth 200 exceeded/);
    } finally {
      artifact.dispose();
    }
  }, 600_000);
});

/**
 * Builds `nxlang` with cargo, the binary `cargo test` builds, and returns its path, read from
 * cargo's own report so a custom target directory is honored.
 */
function buildCommandLine(repositoryRoot: string): string {
  const report = execFileSync(
    "cargo",
    ["build", "-q", "-p", "nx-cli", "--bin", "nxlang", "--message-format=json"],
    { cwd: repositoryRoot, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 }
  );
  for (const line of report.split("\n")) {
    if (!line.startsWith("{")) {
      continue;
    }
    const message = JSON.parse(line) as { reason?: string; executable?: string | null };
    if (message.reason === "compiler-artifact" && typeof message.executable === "string") {
      return message.executable;
    }
  }
  throw new Error("cargo built nxlang but did not report where.");
}
