import { describe, expect, it } from "vitest";

import {
  NxLanguageSnapshot as NodeLanguageSnapshot,
  NxLibraryRegistry as NodeLibraryRegistry,
  NxProgramArtifact as NodeProgramArtifact,
  NxWorkspace as NodeWorkspace,
  generateNxIrFromSource as generateNxIrWithNode
} from "@nx-lang/sdk-node";

import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

/**
 * Sources chosen to reach the parts the playground uses: a value, a typed function, a component
 * with children, and a program that does not compile.
 */
const sources = {
  value: "let root() = { 42 }",
  typed: "let answer(): int = { 42 }\nlet root() = { answer() }",
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
