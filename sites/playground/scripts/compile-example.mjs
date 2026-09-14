/**
 * Compiling an example the way the site does, for the scripts that check the corpus.
 *
 * <para>The host is the wasm module the browser loads and the catalog is the one the worker
 * bundles, through `src/compile/catalog.ts` — the same code, so what the examples are checked
 * against is what a visitor gets.</para>
 */
import { readFileSync } from "node:fs";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { compileWithCatalog } from "../src/compile/catalog.ts";

const catalog = readFileSync(new URL("../catalog/skia.nx", import.meta.url), "utf8");
const host = createNxHost(await loadNxModule());

/** Compiles NX source against the site's catalog. Returns `{ ir, diagnostics }`. */
export function compile(source) {
  return compileWithCatalog(host, catalog, source);
}

/** Releases the host. A script that runs to completion need not call this; Node exits either way. */
export function dispose() {
  host.dispose();
}
