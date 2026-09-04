/**
 * Proves the dev proxy answers rather than hangs when the compile server is missing or stuck.
 *
 * Both paths end in the same place for the app — no compile — but they are different failures, and
 * the point of the config under test is that the client is told which one it hit before its own
 * eight-second deadline. That deadline is what these tests assert against: an answer that arrives
 * after it is, from the app's side, indistinguishable from the hang this replaced.
 */
import { strict as assert } from "node:assert";
import { createServer as createHttpServer } from "node:http";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = fileURLToPath(new URL(".", import.meta.url));

/** The deadline `src/compile/` gives a compile before it stops waiting. */
const CLIENT_DEADLINE_MS = 8000;

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
let compilePort;
let stalling;

/** Posts a compile the way the client does, and fails rather than hangs if nothing answers in time. */
async function postCompile() {
  const started = Date.now();
  const response = await fetch(`${origin}/api/compile`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ source: "component Main { }" }),
    signal: AbortSignal.timeout(CLIENT_DEADLINE_MS),
  });
  const body = await response.json();
  return { response, body, elapsed: Date.now() - started };
}

before(async () => {
  compilePort = await freePort();
  // Read by `server/port.mjs`, which is what the config under test points its probe and proxy at.
  process.env.PORT = String(compilePort);
  const vitePort = await freePort();
  vite = await createServer({
    root: appRoot,
    logLevel: "error",
    server: { port: vitePort, strictPort: true },
  });
  await vite.listen();
  origin = `http://127.0.0.1:${vitePort}`;
});

after(async () => {
  await vite?.close();
  await new Promise((fulfil) => (stalling ? stalling.close(fulfil) : fulfil()));
});

test("names the absent compile server instead of leaving the request to time out", async () => {
  const { response, body, elapsed } = await postCompile();
  assert.equal(response.status, 502);
  assert.match(body.error, /not running/);
  assert.ok(elapsed < CLIENT_DEADLINE_MS, `answered in ${elapsed}ms`);
});

test("answers a compile server that accepts the connection and then never replies", async () => {
  // Listening but silent: the probe sees a live port and hands the request to the proxy, which is
  // the only thing left that can end it.
  stalling = createHttpServer(() => {});
  await new Promise((fulfil) => stalling.listen(compilePort, "127.0.0.1", fulfil));

  const { response, body, elapsed } = await postCompile();
  assert.equal(response.status, 502);
  assert.match(body.error, /mid-request/);
  assert.ok(elapsed < CLIENT_DEADLINE_MS, `answered in ${elapsed}ms`);
});
