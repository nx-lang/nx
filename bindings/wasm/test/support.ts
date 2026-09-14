import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { compileNxModule } from "../src/module.js";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

/**
 * The module the package ships, compiled once for the whole test run.
 */
export const nxModule = await compileNxModule(
  await readFile(path.join(packageRoot, "dist", "nx.wasm"))
);

/**
 * The same module built with the crate's `debug-trap` feature, which exports `nx_wasm_trap`.
 *
 * Built by the package's `test` script; the shipped module does not carry the export.
 */
export const nxDebugTrapModule = await compileNxModule(
  await readFile(path.join(packageRoot, "dist", "nx-debug-trap.wasm"))
);
