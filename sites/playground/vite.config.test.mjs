/**
 * Proves the dev server serves the shell the app needs and the catalog's artifact the renderer
 * imports.
 *
 * The API proxy and the compile-server probe went with the compile server; what the config does
 * now is inject the base and compile the catalog to its artifact at build time.
 */
import { strict as assert } from "node:assert";
import { prepareNxIrModule } from "@nx-lang/ir-runtime";
import { createServer as createHttpServer } from "node:http";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { stop as stopEsbuild } from "esbuild";
import { createServer } from "vite";
import { BASE_HREF } from "./base.mjs";
import { CATALOG_ARTIFACT_MODULE } from "./vite.config.ts";

const appRoot = fileURLToPath(new URL(".", import.meta.url));
const TIMEOUT_MS = 8000;

/** Asks the operating system for a port nothing is using, so a run never collides with a dev server. */
function freePort() {
  return new Promise((fulfil, reject) => {
    const probe = createHttpServer();
    probe.on("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const { port } = probe.address();
      probe.close(() => fulfil(port));
    });
  });
}

let vite;
let origin;

before(async () => {
  const vitePort = await freePort();
  vite = await createServer({
    root: appRoot,
    logLevel: "error",
    // Nothing here needs pre-bundled dependencies, and the scan crawls the whole app — including
    // the compiler worker — which outlives the server this test closes a moment later and leaves
    // esbuild's service process holding the run open.
    optimizeDeps: { noDiscovery: true },
    // The address the requests below go to. Left to Vite, the listener is `localhost`, which Node
    // resolves to `::1` where the hosts file names it (GitHub's runners do), and a fetch to
    // 127.0.0.1 is then refused on a port Vite is listening on.
    // No file watcher: `fs.allow` reaches the repository root, so the watcher crawls the whole
    // tree — the Rust target directory and the WASI sysroot cache included — and outlives the
    // server this test closes, holding the run open. Nothing here edits a file.
    server: { host: "127.0.0.1", port: vitePort, strictPort: true, watch: null },
  });
  await vite.listen();
  origin = `http://127.0.0.1:${vitePort}`;
});

after(async () => {
  await vite?.close();
  // Vite does not stop esbuild's service process, and Node will not exit while it is there. This
  // is not the whole story — the site's `test` script also passes `--test-force-exit`, because a
  // closed Vite server leaves a watcher behind as well — but stopping the one thing that is ours
  // to stop keeps the run from depending entirely on that flag.
  await stopEsbuild();
});

test("the shell carries the site's <base href>, put there by the config rather than index.html", async () => {
  // The examples name images by relative path and the editor's addresses are nested, so a shell
  // without the base would have the browser resolve `images/baboon.jpg` against the route.
  const response = await fetch(`${origin}${BASE_HREF}`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.equal(response.status, 200);
  assert.match(await response.text(), new RegExp(`<base href="${BASE_HREF}"`));
});

test("the catalog's artifact is a module the build emits, so a compile never has to", async () => {
  // What the renderer imports: the catalog compiled on its own, by the wasm module under Node,
  // naming the same identity the worker's compiles name in their module tables.
  const result = await vite.transformRequest(CATALOG_ARTIFACT_MODULE);
  assert.ok(result !== null, "the virtual module resolves");
  // The image as base64 in one string literal, decoded on first use by `render/catalog.ts`.
  const match = /^export default "([A-Za-z0-9+/=]+)";$/.exec(result.code);
  assert.ok(match, "the module is one default export of a base64 string");
  const image = new Uint8Array(Buffer.from(match[1], "base64"));
  assert.equal(new TextDecoder().decode(image.subarray(0, 4)), "NXIR");
  const { artifact } = prepareNxIrModule(image);
  assert.deepEqual(artifact.modules.map((module) => module.identity), ["skia.nx"]);
  const strings = Array.from({ length: artifact.stringCount }, (_, index) => artifact.string(index));
  assert.ok(strings.includes("SkiaLabel"));
  assert.equal(artifact.hasDebug, false, "no debug section: nobody reads the catalog's spans");
});
