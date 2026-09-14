import { describe, expect, it } from "vitest";

import { abiVersion } from "../src/abi.js";
import { NxWasmError } from "../src/errors.js";
import { createNxHost } from "../src/node.js";
import { nxModule } from "./support.js";

describe("the wasm loader", () => {
  it("checks the module's ABI version and names both when it disagrees", async () => {
    const stub = await WebAssembly.compile(abiVersionOnlyModule(abiVersion + 1));

    expect(() => createNxHost(stub)).toThrowError(NxWasmError);
    expect(() => createNxHost(stub)).toThrowError(
      new RegExp(`ABI version ${abiVersion + 1}\\b[\\s\\S]*ABI version ${abiVersion}\\b`)
    );
  });

  it("refuses a module that does not implement the ABI at all", async () => {
    const empty = await WebAssembly.compile(Uint8Array.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0, 0, 0]));

    expect(() => createNxHost(empty)).toThrowError(/nx_wasm_abi_version/);
  });

  it("imports nothing but WASI preview 1, so any host with a WASI shim can run it", () => {
    // Named rather than counted: a new crate dependency that reaches for an `env` symbol would
    // otherwise surface as an opaque LinkError at instantiation, and only on the path that uses it.
    const foreign = WebAssembly.Module.imports(nxModule)
      .filter((entry) => entry.module !== "wasi_snapshot_preview1")
      .map((entry) => `${entry.module}.${entry.name}`);

    expect(foreign).toEqual([]);
  });

  it("releases every result, so repeated calls do not grow the module's memory", () => {
    const host = createNxHost(nxModule);
    try {
      const snapshot = host.createLanguageSnapshot([
        { uri: "nx://demo/input.nx", source: "let root() = { 42 }" }
      ]);

      // One pass first, so the lazy analysis and the allocator's first growth are behind us.
      snapshot.documentSymbols("nx://demo/input.nx");
      const settled = host.memoryBytes;

      for (let index = 0; index < 1000; index += 1) {
        snapshot.documentSymbols("nx://demo/input.nx");
      }

      expect(host.memoryBytes).toBe(settled);
      snapshot.dispose();
    } finally {
      host.dispose();
    }
  });
});

/**
 * A module exporting nothing but `nx_wasm_abi_version`, so the loader's version check can be
 * exercised without building a second copy of the real one.
 */
function abiVersionOnlyModule(version: number): Uint8Array<ArrayBuffer> {
  const body = [0x41, ...signedLeb128(version), 0x0b];
  const name = [...new TextEncoder().encode("nx_wasm_abi_version")];

  return Uint8Array.from([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    // type section: one type, () -> i32
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
    // function section: one function of type 0
    0x03, 0x02, 0x01, 0x00,
    // export section: the function, under its ABI name
    0x07, name.length + 4, 0x01, name.length, ...name, 0x00, 0x00,
    // code section: one body with no locals
    0x0a, body.length + 3, 0x01, body.length + 1, 0x00, ...body
  ]);
}

function signedLeb128(value: number): number[] {
  const bytes: number[] = [];
  let remaining = value;
  for (;;) {
    const byte = remaining & 0x7f;
    remaining >>= 7;
    const signBitSet = (byte & 0x40) !== 0;
    if ((remaining === 0 && !signBitSet) || (remaining === -1 && signBitSet)) {
      bytes.push(byte);
      return bytes;
    }
    bytes.push(byte | 0x80);
  }
}
