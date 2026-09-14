import { describe, expect, it } from "vitest";

import type { NxWasmExports } from "../src/abi.js";
import { NxHostCrashedError } from "../src/errors.js";
import type { NxHost } from "../src/host.js";
import { createNxHost } from "../src/node.js";
import { nxDebugTrapModule } from "./support.js";

/**
 * Calls the trap export the `debug-trap` module carries, through the loader's own wrapper, so the
 * test exercises the path a real trap takes rather than a simulated one.
 */
function trap(host: NxHost): void {
  (host as unknown as { call(operation: string, run: (exports: NxWasmExports) => number): unknown }).call(
    "nx_wasm_trap",
    (exports) => {
      exports.nx_wasm_trap?.();
      return 0;
    }
  );
}

describe("a trapped host", () => {
  it("reports the trap, refuses later calls, and is replaced from the same module", () => {
    const host = createNxHost(nxDebugTrapModule);

    expect(host.crashed).toBe(false);
    expect(() => trap(host)).toThrowError(NxHostCrashedError);
    expect(() => trap(host)).toThrowError(/nx_wasm_trap/);
    expect(host.crashed).toBe(true);

    // The next call is refused by the host itself; the operation named stays the one that trapped,
    // which is how the caller can tell it never entered the module again.
    let refused: unknown;
    try {
      host.buildProgramArtifact("let root() = { 42 }");
    } catch (error) {
      refused = error;
    }
    expect(refused).toBeInstanceOf(NxHostCrashedError);
    expect((refused as NxHostCrashedError).operation).toBe("nx_wasm_trap");

    // Disposing resources on a crashed host is tolerated, so cleanup paths do not mask the crash.
    expect(() => host.dispose()).not.toThrow();

    const replacement = createNxHost(nxDebugTrapModule);
    try {
      const artifact = replacement.buildProgramArtifact("let root() = { 42 }");
      expect(artifact.generateNxIr().json).toContain("\"schemaVersion\"");
      artifact.dispose();
      expect(replacement.crashed).toBe(false);
    } finally {
      replacement.dispose();
    }
  });

  it("leaves resources built before the trap reporting the crash rather than a stale answer", () => {
    const host = createNxHost(nxDebugTrapModule);
    const snapshot = host.createLanguageSnapshot([
      { uri: "nx://demo/input.nx", source: "let root() = { 42 }" }
    ]);

    expect(() => snapshot.documentSymbols("nx://demo/input.nx")).not.toThrow();
    expect(() => trap(host)).toThrowError(NxHostCrashedError);
    expect(() => snapshot.documentSymbols("nx://demo/input.nx")).toThrowError(NxHostCrashedError);
    expect(() => snapshot.dispose()).not.toThrow();
  });
});
