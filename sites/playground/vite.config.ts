import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import react from "@vitejs/plugin-react";
import { copyFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { type Plugin, defineConfig } from "vite";
import { BASE_HREF } from "./base.mjs";
import { emitCatalogArtifact } from "./src/compile/catalog.ts";

/** The virtual module `src/render/catalog.ts` imports the catalog's image from, as base64. */
export const CATALOG_ARTIFACT_MODULE = "virtual:nx-catalog-artifact";

const catalogPath = fileURLToPath(new URL("./catalog/skia.nx", import.meta.url));

/**
 * Compiles the catalog to its NX IR artifact at build time and serves it as a module.
 *
 * The compiler is the same wasm module the browser loads, run here under Node, so the artifact the
 * bundle carries is what the worker's compiles name in their module tables. Emitting it here rather
 * than in the worker means a compile sends the visitor's module alone — a few kilobytes — and the
 * catalog is prepared once per page rather than parsed on every keystroke. Nothing is committed:
 * the catalog text stays the one source, and in development an edit to it re-emits the artifact.
 */
function catalogArtifact(): Plugin {
  const resolved = `\0${CATALOG_ARTIFACT_MODULE}`;
  return {
    name: "nx-catalog-artifact",
    resolveId: (id) => (id === CATALOG_ARTIFACT_MODULE ? resolved : undefined),
    async load(id) {
      if (id !== resolved) {
        return undefined;
      }
      this.addWatchFile(catalogPath);
      const host = createNxHost(await loadNxModule());
      try {
        const artifact = emitCatalogArtifact(host, readFileSync(catalogPath, "utf8"));
        // The image as base64 in a string literal, decoded on first use by `render/catalog.ts`: a
        // string this size costs the JavaScript parser nothing, and the bundle stays one file with
        // no asset to fetch before the first drawing.
        return `export default ${JSON.stringify(Buffer.from(artifact).toString("base64"))};`;
      } finally {
        host.dispose();
      }
    },
  };
}

/**
 * Puts the site's `<base href>` into the shell from the same constant Vite's `base` comes from.
 *
 * Editor addresses are nested (`/playground/shapes`), and DrawnUI loads the images the examples
 * name by relative path; without a `<base>` they would resolve against the route. Injecting it
 * here rather than writing it in `index.html` keeps `base.mjs` the only place the prefix is spelled.
 */
function siteBase(): Plugin {
  return {
    name: "site-base",
    transformIndexHtml: () => [{ tag: "base", attrs: { href: BASE_HREF }, injectTo: "head-prepend" }],
  };
}

/**
 * Where the build goes. Files land under `dist/playground/`, so the path of each file under `dist/`
 * is the path it is served at, and `dist/` is the directory the Cloudflare Worker serves.
 */
const outDir = fileURLToPath(new URL("./dist/playground", import.meta.url));

/**
 * Copies the cache policy into the root of the served directory, where Cloudflare's static assets
 * read `_headers` from. It is not in `public/`, which would put it under the prefix and serve it as a
 * file.
 */
function cacheHeaders(): Plugin {
  return {
    name: "cache-headers",
    apply: "build",
    closeBundle() {
      copyFileSync(
        fileURLToPath(new URL("./_headers", import.meta.url)),
        fileURLToPath(new URL("./dist/_headers", import.meta.url)),
      );
    },
  };
}

/**
 * drawnui-react is built for Vite: its `dist` imports the CanvasKit wasm binary with `?url`, and
 * the fonts it is configured with load from `publicDir`. The `build` and `fs` settings below mirror
 * `samples/vite.shared.ts` upstream, so the package runs as it does there.
 *
 * `base` is the site's prefix: every asset URL Vite emits and `import.meta.env.BASE_URL` in the
 * client carry it, and the dev server serves the app at the same address production does, so a
 * path written against the origin's root breaks locally before it ships.
 *
 * There is no API proxy. The compiler and the language service are a WebAssembly module the client
 * loads into a worker, so `pnpm run dev` alone is the whole development setup.
 */
export default defineConfig({
  base: BASE_HREF,
  plugins: [react(), siteBase(), catalogArtifact(), cacheHeaders()],
  build: { target: "esnext", outDir, emptyOutDir: true },
  server: {
    // The shared packages are workspace links into the repository, so dev needs to be allowed to
    // read above the app root.
    fs: { allow: [".", "../.."] },
  },
});
