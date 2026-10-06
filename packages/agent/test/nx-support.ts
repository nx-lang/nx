import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { evaluateFunction, linkNxIrProgram, prepareNxIrModule, type NxPreparedProgram } from "@nx-lang/ir-runtime";
import { compileNxModule, createNxHost, type NxLibraryInput, type NxProgramArtifact } from "@nx-lang/sdk-wasm";

import type { Agent } from "../src/index.js";

// The compiler, for the tests only: nothing under `src` imports it. The module is read through the
// package's own `./nx.wasm` export and compiled with the API both of its entry points type.
const host = createNxHost(await compileNxModule(await readFile(fileURLToPath(import.meta.resolve("@nx-lang/sdk-wasm/nx.wasm")))));

const disposables: { dispose(): void }[] = [];

/** Releases everything the tests of one file built. Pass it to `after`. */
export function disposeCompiled(): void {
  for (const disposable of disposables.splice(0).reverse()) {
    disposable.dispose();
  }
}

/** One emitted NX IR image: the module's identity and its bytes, which is all the run-time half needs. */
export interface EmittedImage {
  readonly identity: string;
  readonly bytes: Uint8Array;
}

export interface CompiledProgram {
  /** The program artifact, which is a schema source as it stands. */
  readonly artifact: NxProgramArtifact;
  readonly images: readonly EmittedImage[];
  readonly program: NxPreparedProgram;
  /** The entry module's `root`, evaluated by the IR runtime. */
  agent(): Agent;
}

/**
 * A host library the way a host declares one: the tool context it supplies, two tool types of its
 * own, and a second context type that it does not supply.
 */
export const hostLibrary: NxLibraryInput = {
  root: "host",
  modules: [
    {
      identity: "Chat.nx",
      source: [
        'import "@nx/agent"',
        "/// What a chat host supplies to a tool's function.",
        "export type ChatToolContext extends ToolContext = { conversationId:string contactEmail?:string }",
        "export type AuditToolContext extends ToolContext = { actor:string }",
        "export type RecordSearchTool extends Tool = { recordKind:string maxResults:int = 5 }",
        "export type MeetingSchedulerTool extends Tool = { calendar:string }",
      ].join("\n"),
    },
  ],
};

/** The identity of the host library's tool context type. */
export const chatToolContext = { module: "host/Chat.nx", name: "ChatToolContext" } as const;

/** Links emitted images with the IR runtime alone, as a host with no compiler does. */
export function linkImages(images: readonly EmittedImage[], entry = "main.nx"): NxPreparedProgram {
  const prepared = new Map(images.map((image) => [image.identity, prepareNxIrModule(image.bytes)]));
  return linkNxIrProgram(prepared.get(entry)!, { resolve: (identity) => prepared.get(identity) });
}

/**
 * Compiles `source` as `main.nx` with the agent library and the host library imported implicitly,
 * emits every module's image, and links them.
 */
export function compileProgram(source: string): CompiledProgram {
  const registry = host.createLibraryRegistry();
  disposables.push(registry);
  registry.loadLibrary(hostLibrary);
  const buildContext = registry.createBuildContext({ implicitImports: ["host", "@nx/agent"] });
  disposables.push(buildContext);
  const artifact = host.buildWorkspaceArtifact({ modules: [{ identity: "main.nx", source }], entry: "main.nx", buildContext });
  disposables.push(artifact);
  const images = artifact.generateNxIr({ modules: [] }).map((image) => ({ identity: image.identity, bytes: image.bytes }));
  const program = linkImages(images);
  return { artifact, images, program, agent: () => evaluateFunction(program, "root") as unknown as Agent };
}

/** The diagnostics a source fails to compile with, or nothing when it compiles. */
export function compileDiagnostics(source: string): readonly { readonly code?: string; readonly message: string }[] {
  try {
    compileProgram(source);
    return [];
  } catch (error) {
    const diagnostics = (error as { diagnostics?: readonly { code?: string; message: string }[] }).diagnostics;
    if (diagnostics === undefined) {
      throw error;
    }
    return diagnostics;
  }
}

/** The diagnostics a single source fails to compile with on its own, as a program that imports what it uses. */
export function sourceDiagnostics(source: string): readonly { readonly code?: string; readonly message: string }[] {
  try {
    disposables.push(host.buildProgramArtifact(source, { fileName: "main.nx" }));
    return [];
  } catch (error) {
    const diagnostics = (error as { diagnostics?: readonly { code?: string; message: string }[] }).diagnostics;
    if (diagnostics === undefined) {
      throw error;
    }
    return diagnostics;
  }
}
