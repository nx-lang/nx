import { ConsoleStdout, File, OpenFile, WASI } from "@bjorn3/browser_wasi_shim";

import type { WasiProvider } from "./abi.js";
import { createHost, type NxHost } from "./host.js";

export * from "./api.js";

/**
 * Instantiates `module` in a browser and returns a host that answers NX queries.
 *
 * <para>The module's only imports are WASI preview 1, supplied here by a shim with no preopened
 * directories: NX's file-backed fallbacks see "not a file" and the language service works entirely
 * from the documents it is given. Standard output and error go to the console, where a panic
 * message from inside the module is worth reading.</para>
 *
 * @throws NxWasmError when the module's ABI version is not the one this package was built against.
 */
export function createNxHost(module: WebAssembly.Module): NxHost {
  return createHost(module, createWasiProvider);
}

function createWasiProvider(): WasiProvider {
  const wasi = new WASI(
    [],
    [],
    [
      new OpenFile(new File([])),
      ConsoleStdout.lineBuffered((line) => console.log(`[nx-wasm] ${line}`)),
      ConsoleStdout.lineBuffered((line) => console.warn(`[nx-wasm] ${line}`))
    ]
  );

  return {
    imports: { wasi_snapshot_preview1: wasi.wasiImport },
    initialize: (instance) => {
      wasi.initialize(instance as unknown as Parameters<typeof wasi.initialize>[0]);
    }
  };
}
