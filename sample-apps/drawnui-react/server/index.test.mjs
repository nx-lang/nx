/**
 * Serves the app in a real process and probes the request boundary.
 *
 * These are here rather than in `compile.test.mjs` because what they check is not what an answer
 * says but that there is still a process left to answer: one bad request must not end the service
 * for everyone else using it.
 */
import { strict as assert } from "node:assert";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";

const entry = fileURLToPath(new URL("./index.mjs", import.meta.url));

/** Asks the operating system for a port nothing is using, so a run never collides with a dev server. */
function freePort() {
  return new Promise((fulfil, reject) => {
    const probe = createServer();
    probe.on("error", reject);
    probe.listen(0, () => {
      const { port } = probe.address();
      probe.close(() => fulfil(port));
    });
  });
}

let child;
let origin;

/**
 * Proves there is still a process there to answer.
 *
 * The proof is the exchange completing at all: a dead server refuses the connection and `fetch`
 * rejects. The status is deliberately not asserted, because every status is a live answer — the
 * `503` `serveStatic` returns when `dist/` has not been built most of all, since that is the case
 * a fresh checkout runs these tests in.
 */
async function assertStillServing() {
  const response = await fetch(`${origin}/`);
  // Draining the body finishes the exchange, so a half-sent answer fails here rather than passing.
  await response.arrayBuffer();
  assert.ok(
    Number.isInteger(response.status),
    `the server should still be answering, got ${response.status}`,
  );
}

before(async () => {
  const port = await freePort();
  origin = `http://127.0.0.1:${port}`;
  child = spawn(process.execPath, [entry], {
    env: { ...process.env, PORT: String(port) },
    stdio: ["ignore", "pipe", "inherit"],
  });
  await new Promise((fulfil, reject) => {
    const timer = setTimeout(() => reject(new Error("the server did not start")), 20_000);
    child.stdout.on("data", (chunk) => {
      if (chunk.toString().includes("listening")) {
        clearTimeout(timer);
        fulfil();
      }
    });
    child.on("exit", (code) => reject(new Error(`the server exited with ${code} before listening`)));
  });
});

after(() => {
  child?.kill();
});

test("answers a malformed percent-escape and keeps serving", async () => {
  // `decodeURIComponent` throws on `/%ZZ`. Unhandled, that ended the process and every request
  // after it failed to connect at all.
  const malformed = await fetch(`${origin}/%ZZ`);
  assert.equal(malformed.status, 400);

  await assertStillServing();
});

test("answers a stray delimiter at the end of the source and keeps serving", async () => {
  // A source the scanner could not finish scanning used to hang the compile thread, which is the
  // only thread there is: nothing else could be served while it spun.
  const compiled = await fetch(`${origin}/api/compile`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ source: "@" }),
  });
  assert.equal(compiled.status, 200);
  const body = await compiled.json();
  assert.equal(body.ir, null);
  assert.ok(body.diagnostics.length > 0);

  await assertStillServing();
});
