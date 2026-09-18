import path from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

describe("the package's entry points", () => {
  it("resolves the node condition to the node:wasi host", async () => {
    const entry = (await import("@nx-lang/sdk-wasm")) as typeof import("../src/node.js");

    expect(typeof entry.createNxHost).toBe("function");
    expect(typeof entry.compileNxModule).toBe("function");
    // Only the node entry can load the module from the filesystem, so its presence identifies it.
    expect(typeof entry.loadNxModule).toBe("function");

    const host = entry.createNxHost(await entry.loadNxModule());
    try {
      const artifact = host.buildProgramArtifact("let root() = { 42 }");
      expect(artifact.generateNxIr()[0]!.metadata.schemaVersion).toBe(4);
      artifact.dispose();
    } finally {
      host.dispose();
    }
  });

  it("bundles the default entry into a Vite page", async () => {
    const { build } = await import("vite");
    const root = path.join(packageRoot, "test", "fixtures", "vite-page");

    const output = await build({
      root,
      logLevel: "silent",
      build: {
        outDir: path.join(packageRoot, "node_modules", ".vite-page-check"),
        emptyOutDir: true,
        target: "esnext"
      }
    });

    const bundles = Array.isArray(output) ? output : [output];
    const chunks = bundles.flatMap((bundle) => ("output" in bundle ? bundle.output : []));
    const assetNames = chunks.map((chunk) => chunk.fileName);

    expect(assetNames.some((name) => name.endsWith(".js"))).toBe(true);
    expect(assetNames.some((name) => name.endsWith(".wasm"))).toBe(true);
  }, 120_000);
});
