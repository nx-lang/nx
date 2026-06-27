import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { NxNativeError } from "./errors.js";

export interface NativeWorkspaceModule {
  readonly identity: string;
  readonly source: string | Buffer;
}

export interface NativeNxWorkspace {
  validate(buildContext: NativeNxProgramBuildContext): string;
  dispose(): void;
}

export interface NativeNxWorkspaceConstructor {
  new (modules: readonly NativeWorkspaceModule[]): NativeNxWorkspace;
}

export interface NativeNxLibraryRegistry {
  loadLibraryFromDirectory(rootPath: string): void;
  createBuildContext(): NativeNxProgramBuildContext;
  dispose(): void;
}

export interface NativeNxLibraryRegistryConstructor {
  new (): NativeNxLibraryRegistry;
}

export interface NativeNxProgramBuildContext {
  buildSourceProgramArtifact(source: string | Buffer, fileName?: string): NativeNxProgramArtifact;
  buildWorkspaceProgramArtifact(
    workspace: NativeNxWorkspace,
    entryIdentity: string
  ): NativeNxProgramArtifact;
  dispose(): void;
}

export interface NativeNxProgramArtifact {
  readonly entryIdentity: string;
  generateNxIr(): string;
  evaluateJson(): string;
  evaluateBytes(outputFormat?: string): Buffer;
  dispose(): void;
}

export interface NativeBinding {
  readonly NativeNxWorkspace: NativeNxWorkspaceConstructor;
  readonly NativeNxLibraryRegistry: NativeNxLibraryRegistryConstructor;
}

let binding: NativeBinding | undefined;

export function loadNativeBinding(): NativeBinding {
  if (binding !== undefined) {
    return binding;
  }

  const require = createRequire(import.meta.url);
  const here = dirname(fileURLToPath(import.meta.url));
  const candidates = [
    resolve(here, "../../native/nx_sdk_node.node"),
    resolve(here, "../native/nx_sdk_node.node"),
    resolve(here, "./nx_sdk_node.node")
  ];
  const failures: string[] = [];

  for (const candidate of candidates) {
    if (!existsSync(candidate)) {
      failures.push(`${candidate}: file not found`);
      continue;
    }

    try {
      binding = require(candidate) as NativeBinding;
      return binding;
    } catch (error) {
      failures.push(`${candidate}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  throw new NxNativeError(
    [
      "NX native Node binding could not be loaded.",
      "Run `npm install` and `npm run build:native` from `bindings/node`, or use a future published prebuild that matches this Node and platform.",
      ...failures
    ].join("\n")
  );
}
