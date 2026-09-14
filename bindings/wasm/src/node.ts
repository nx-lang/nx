import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

import type { WasiProvider } from "./abi.js";
import { createHost, type NxHost } from "./host.js";
import { compileNxModule } from "./module.js";

export * from "./api.js";

/**
 * Instantiates `module` under Node's built-in WASI and returns a host that answers NX queries.
 *
 * <para>Node prints an experimental-feature warning for `node:wasi`; the package's own tests and
 * the playground's example check run with `--no-warnings`. Nothing in the loader depends on Node
 * internals beyond `WASI` with `version: "preview1"`.</para>
 *
 * @throws NxWasmError when the module's ABI version is not the one this package was built against.
 */
export function createNxHost(module: WebAssembly.Module): NxHost {
  return createHost(module, createWasiProvider);
}

function createWasiProvider(): WasiProvider {
  const wasi = new WASI({ version: "preview1", args: [], env: {} });
  return {
    imports: wasi.getImportObject() as WebAssembly.Imports,
    initialize: (instance) => {
      wasi.initialize(instance);
    }
  };
}

/**
 * Location of the module this package ships, next to its compiled JavaScript.
 */
export const nxModuleUrl = new URL("../nx.wasm", import.meta.url);

/**
 * Compiles the module this package ships, for callers running from the filesystem rather than
 * fetching it as an asset.
 */
export async function loadNxModule(): Promise<WebAssembly.Module> {
  return compileNxModule(await readFile(nxModuleUrl));
}
