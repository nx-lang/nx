/**
 * Compiling an example the way the site does, for the scripts that check the corpus.
 *
 * <para>The host is the wasm module the browser loads and the catalog is the one the worker
 * bundles, through `src/compile/catalog.ts` — the same code, so what the examples are checked
 * against is what a visitor gets. The catalog's artifact is emitted here the way the Vite build
 * emits it, and prepared once, the way the page does.</para>
 */
import { readFileSync } from "node:fs";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { compileWithCatalog, emitCatalogArtifact } from "../src/compile/catalog.ts";
import { prepareCatalog } from "../src/render/evaluate.ts";

const catalog = readFileSync(new URL("../catalog/skia.nx", import.meta.url), "utf8");
const host = createNxHost(await loadNxModule());

/** The prepared catalog every compiled example links against. */
export const catalogModule = prepareCatalog(emitCatalogArtifact(host, catalog));

/** Compiles NX source against the site's catalog. Returns `{ ir, diagnostics }`. */
export function compile(source) {
  return compileWithCatalog(host, catalog, source);
}

/** Renders an image as the text `nxlang ir explain` prints. */
export function explain(image) {
  return host.explainNxIr(image);
}

/** Releases the host. A script that runs to completion need not call this; Node exits either way. */
export function dispose() {
  host.dispose();
}
