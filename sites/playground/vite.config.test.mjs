/**
 * Proves the dev server serves the shell under the site's prefix, the address production serves it
 * at.
 */
import { strict as assert } from "node:assert";
import { createServer as createHttpServer } from "node:http";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { stop as stopEsbuild } from "esbuild";
import { createServer } from "vite";
import { BASE_HREF } from "./base.mjs";

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
/** A stand-in for the website's dev server, which the playground's forwards paths outside `/play` to. */
let website;

before(async () => {
  const websitePort = await freePort();
  website = createHttpServer((request, response) => {
    response.writeHead(200, { "content-type": "text/plain" });
    response.end(`website: ${request.url}`);
  });
  await new Promise((resolve) => website.listen(websitePort, "127.0.0.1", resolve));
  // Read by vite.config.ts when the server below loads it.
  process.env.NX_WEBSITE_DEV_URL = `http://127.0.0.1:${websitePort}`;

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
  website?.close();
  await vite?.close();
  // Vite does not stop esbuild's service process, and Node will not exit while it is there. This
  // is not the whole story — the site's `test` script also passes `--test-force-exit`, because a
  // closed Vite server leaves a watcher behind as well — but stopping the one thing that is ours
  // to stop keeps the run from depending entirely on that flag.
  await stopEsbuild();
});

test("the dev server serves the shell at the site's prefix", async () => {
  const response = await fetch(`${origin}${BASE_HREF}`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.equal(response.status, 200);
  const shell = await response.text();
  assert.match(shell, /<div id="root">/);
  // Asset URLs carry the prefix, so the shell works at a nested address such as `/play/records`.
  assert.match(shell, new RegExp(`src="${BASE_HREF}src/main.tsx"`));
});

test("the dev server serves the shell at the prefix without its slash, as a shared link names it", async () => {
  const response = await fetch(`${origin}${BASE_HREF.replace(/\/$/, "")}`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.equal(response.status, 200);
  assert.match(await response.text(), /<div id="root">/);
});

test("a path outside the prefix is forwarded to the website's dev server, as the domain splits it", async () => {
  const response = await fetch(`${origin}/tutorials/getting-started/`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.equal(response.status, 200);
  assert.equal(await response.text(), "website: /tutorials/getting-started/");
  // The playground's own paths are not forwarded.
  const shell = await fetch(`${origin}${BASE_HREF}`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.match(await shell.text(), /<div id="root">/);
});

test("with the website's dev server gone, a forwarded path says how to start it", async () => {
  await new Promise((resolve) => website.close(resolve));
  const response = await fetch(`${origin}/tutorials/getting-started/`, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  assert.equal(response.status, 502);
  assert.match(await response.text(), /pnpm run dev` in sites\/website/);
});
