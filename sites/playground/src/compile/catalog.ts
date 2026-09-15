/**
 * Turns NX source into NX IR against the DrawnUI catalog, wherever the compiler is running.
 *
 * <para>The build itself — the catalog placed ahead of the visitor's text, and every diagnostic
 * classified as the visitor's, the catalog's or the program's with spans shifted back into the
 * visitor's own coordinates — is `@nx-lang/sdk-wasm`'s `buildProgramWithPrelude`. What this module
 * adds is the site's policy: the input limits, and the IR parsed for the renderer.</para>
 *
 * <para>Nothing here is browser-only or Node-only: the worker calls it with a host over the wasm
 * module, and `scripts/check-examples.mjs` calls it with a host over the same module under Node, so
 * what the examples are checked against is what the site ships.</para>
 */
import { buildProgramWithPrelude, type NxHost } from "@nx-lang/sdk-wasm";

import type { CompileResult } from "./types.ts";

const encoder = new TextEncoder();

/** Source larger than this is rejected before it reaches the compiler. */
export const MAX_SOURCE_BYTES = 256 * 1024;

/** The logical file name the combined module compiles under. */
const FILE_NAME = "playground.nx";

/**
 * Compiles `source` against `catalog` through `host`.
 *
 * Returns `{ ir, diagnostics }`. `ir` is null when compilation failed; each diagnostic carries an
 * `origin` saying whether it points at the visitor's source or at the catalog behind it.
 *
 * @throws TypeError when `source` is not a string, RangeError when it is over the size limit, and
 * `NxHostCrashedError` when the module traps — a crash is the caller's to recover from by replacing
 * the host, not something to report as an authoring error.
 */
export function compileWithCatalog(host: NxHost, catalog: string, source: string): CompileResult {
  if (typeof source !== "string") {
    throw new TypeError("source must be a string");
  }
  if (encoder.encode(source).byteLength > MAX_SOURCE_BYTES) {
    throw new RangeError(`source exceeds ${MAX_SOURCE_BYTES} bytes`);
  }

  const result = buildProgramWithPrelude(host, catalog, source, { fileName: FILE_NAME });
  return {
    ir: result.ir === null ? null : (JSON.parse(result.ir.json) as unknown),
    diagnostics: result.diagnostics,
  };
}
