import { callFunction, evaluateFunction, linkNxIrProgram, prepareNxIrModule } from "@nx-lang/ir-runtime";
import {
  NxLanguageSnapshot as NodeLanguageSnapshot,
  NxLibraryRegistry as NodeLibraryRegistry,
  NxProgramArtifact as NodeProgramArtifact,
  NxWorkspace as NodeWorkspace
} from "@nx-lang/sdk-node";
import { afterAll, describe, expect, it } from "vitest";

import { NxEvaluationError } from "../src/errors.js";
import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import type { NxLibraryInput } from "../src/types.js";
import { nxModule } from "./support.js";

const AGENT_ROOT = "@nx/agent";
const AGENT_MODULE = "@nx/agent/agent.nx";

const source = 'import "@nx/agent"\nlet root() = { <Agent name="support">Be brief.</Agent> }';

/** A host library that imports the agent library and extends its abstract `Tool`. */
const chatLink: NxLibraryInput = {
  root: "libraries/chat-link",
  modules: [
    {
      identity: "ChatLink.nx",
      source: [
        'import "@nx/agent"',
        "export type RecordSearchTool extends Tool = { recordKind:string maxResults:int = 5 }",
        "export type AssistantConfig = { enabled?:boolean agent?:Agent }"
      ].join("\n")
    }
  ]
};

const implicitImports = ["libraries/chat-link", AGENT_ROOT];

const tenant = [
  {
    identity: "main.nx",
    source: [
      "/// Finds the plans that fit a team.",
      'let findPlans(teamSize:int): string* = { "Team" }',
      "let root() = {",
      "  <AssistantConfig enabled=true agent={",
      '    <Agent name="support" tools={',
      '      <RecordSearchTool recordKind="company" />',
      "      <FunctionTool function={findPlans} />",
      "    }>Be brief.</Agent>",
      "  } />",
      "}"
    ].join("\n")
  }
];

describe("standard libraries", () => {
  const host: NxHost = createNxHost(nxModule);

  afterAll(() => {
    host.dispose();
  });

  it("resolves a single source's import with no registry or build context", () => {
    const artifact = host.buildProgramArtifact(source);
    try {
      expect(artifact.evaluateNx().text).toBe('<Agent instructions="Be brief." name="support" />');
    } finally {
      artifact.dispose();
    }
  });

  it("leaves the library out of scope without an import", () => {
    expect(() => host.buildProgramArtifact('let root(): Agent = { <Agent name="support">Be brief.</Agent> }')).toThrowError(
      NxEvaluationError
    );
  });

  it("emits the library image beside the entry, and the IR runtime links and evaluates them", () => {
    const artifact = host.buildProgramArtifact(source, { fileName: "main.nx" });
    try {
      const images = artifact.generateNxIr({ modules: ["main.nx", AGENT_MODULE] });
      expect(images.map((image) => image.identity)).toEqual(["main.nx", AGENT_MODULE]);
      expect(images[1]!.metadata.identity).toBe(AGENT_MODULE);
      expect(images[1]!.metadata.requiredFeatures).toContain("function-reference-type-v1");

      const library = prepareNxIrModule(images[1]!.bytes);
      // The version is derived from the library's source, the same in every binding; the entry
      // records the same one, which is what linking compares.
      // Pinned to the value the native build computes (`the_shipped_agent_library_version_is_pinned`).
      expect(library.version).toBe("b06c00c30be50ada");
      const entry = prepareNxIrModule(images[0]!.bytes);
      expect(entry.artifact.modules).toContainEqual(
        expect.objectContaining({ identity: AGENT_MODULE, version: library.version })
      );

      const program = linkNxIrProgram(entry, {
        resolve: (identity) => (identity === AGENT_MODULE ? library : undefined)
      });
      expect(evaluateFunction(program, "root")).toEqual({
        $type: "Agent",
        name: "support",
        instructions: "Be brief."
      });
    } finally {
      artifact.dispose();
    }
  });

  it("lets tenant source use the library through a host's libraries and implicit imports", () => {
    const registry = host.createLibraryRegistry();
    // Nothing loads the agent library: the host library resolves it from an empty registry.
    registry.loadLibrary(chatLink);
    const context = registry.createBuildContext({ implicitImports });
    const artifact = host.buildWorkspaceArtifact({ modules: tenant, entry: "main.nx", buildContext: context });
    try {
      const images = artifact.generateNxIr({ modules: [] });
      expect(images.map((image) => image.identity)).toEqual([
        "main.nx",
        AGENT_MODULE,
        "libraries/chat-link/ChatLink.nx"
      ]);
      const prepared = new Map(images.map((image) => [image.identity, prepareNxIrModule(image.bytes)]));
      const program = linkNxIrProgram(prepared.get("main.nx")!, { resolve: (identity) => prepared.get(identity) });
      const config = evaluateFunction(program, "root") as {
        agent: { tools: { $type: string; function?: unknown; maxResults?: number }[] };
      };
      expect(config.agent.tools.map((tool) => tool.$type)).toEqual(["RecordSearchTool", "FunctionTool"]);
      expect(config.agent.tools[0]!.maxResults).toBe(5);
      expect(callFunction(program, config.agent.tools[1]!.function as never, { teamSize: 4 })).toEqual(["Team"]);
    } finally {
      artifact.dispose();
      context.dispose();
      registry.dispose();
    }
  });

  it("refuses a library load under the reserved root and an unknown standard library", () => {
    const registry = host.createLibraryRegistry();
    try {
      for (const root of ["@nx/agent", "@nx/mine"]) {
        let thrown: unknown;
        try {
          registry.loadLibrary({ root, modules: [{ identity: "Mine.nx", source: "export type Mine = { id:string }" }] });
        } catch (error) {
          thrown = error;
        }
        expect(thrown).toBeInstanceOf(NxEvaluationError);
        const [diagnostic] = (thrown as NxEvaluationError).diagnostics;
        expect(diagnostic!.code).toBe("library-root-reserved");
        expect(diagnostic!.message).toContain(root);
      }
    } finally {
      registry.dispose();
    }

    const diagnostics = host.validateWorkspace({
      modules: [{ identity: "main.nx", source: 'import "@nx/automation"\nlet root() = { 1 }' }]
    });
    expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual(["unknown-standard-library"]);
    expect(diagnostics[0]!.message).toContain("@nx/agent");
  });
});

describe("standard library parity with the Node SDK", () => {
  const host: NxHost = createNxHost(nxModule);

  afterAll(() => {
    host.dispose();
  });

  it("emits byte-identical entry and library images, and equal diagnostics", () => {
    const broken = [{ identity: "main.nx", source: 'let root() = { <Agent name="support" /> }' }];

    const nodeRegistry = new NodeLibraryRegistry();
    nodeRegistry.loadLibraries([chatLink]);
    const nodeContext = nodeRegistry.createBuildContext();
    const nodeWorkspace = new NodeWorkspace(tenant);
    const nodeBroken = new NodeWorkspace(broken);
    const fromNode = NodeProgramArtifact.buildWorkspace(nodeWorkspace, {
      buildContext: nodeContext,
      entryIdentity: "main.nx",
      implicitImports
    });

    const wasmRegistry = host.createLibraryRegistry();
    wasmRegistry.loadLibraries([chatLink]);
    const wasmContext = wasmRegistry.createBuildContext({ implicitImports });
    const fromWasm = host.buildWorkspaceArtifact({ modules: tenant, entry: "main.nx", buildContext: wasmContext });
    try {
      const nodeArtifacts = fromNode.generateNxIr({ modules: [] });
      const wasmArtifacts = fromWasm.generateNxIr({ modules: [] });
      expect(wasmArtifacts.map((entry) => entry.identity)).toEqual(nodeArtifacts.map((entry) => entry.identity));
      expect(wasmArtifacts.map((entry) => entry.identity)).toContain(AGENT_MODULE);
      wasmArtifacts.forEach((entry, index) => {
        expect(Buffer.compare(Buffer.from(entry.bytes), nodeArtifacts[index]!.bytes)).toBe(0);
        expect(entry.metadata).toEqual(nodeArtifacts[index]!.metadata);
      });

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

  it("answers hover and completions identically, with no build context", () => {
    const uri = "nx://demo/agent.nx";
    const text = 'import "@nx/agent"\n<Agent name="support" />\n';
    const documents = [{ uri, source: text, identity: "demo/agent.nx", version: 3 }];
    const nodeSnapshot = new NodeLanguageSnapshot(documents);
    const wasmSnapshot = host.createLanguageSnapshot(documents);
    try {
      expect(wasmSnapshot.diagnostics()).toEqual(nodeSnapshot.diagnostics());

      const onTag = { line: 1, character: 3 };
      const hover = wasmSnapshot.hover(uri, onTag);
      expect(hover?.contents).toContain("(standard library @nx/agent)");
      expect(hover).toEqual(nodeSnapshot.hover(uri, onTag));

      const inTag = { line: 1, character: 22 };
      const completions = wasmSnapshot.completions(uri, inTag);
      expect(completions.items.map((item) => item.label)).toEqual(
        expect.arrayContaining(["model", "tools", "instructions"])
      );
      expect(completions).toEqual(nodeSnapshot.completions(uri, inTag));
    } finally {
      wasmSnapshot.dispose();
      nodeSnapshot.dispose();
    }
  });
});
