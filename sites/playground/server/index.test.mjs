/**
 * Serves the site in a real process and probes its address scheme and its request boundary.
 *
 * The address checks are here rather than in `routes.test.mjs` because they are about what the
 * server answers — the redirect, the not-found outside the prefix, the cache headers — and the
 * boundary checks are here because what they check is not what an answer says but that there is
 * still a process left to answer: one bad request must not end the service for everyone else.
 *
 * A stand-in `dist/` is written to a temporary directory so the shell and asset checks do not
 * depend on a build having run.
 */
import { strict as assert } from "node:assert";
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { API_PREFIX, BASE_PATH, HEALTH_PATH } from "../base.mjs";

const entry = fileURLToPath(new URL("./index.mjs", import.meta.url));

const SHELL = "<!doctype html><title>stand-in shell</title>";
const HASHED_ASSET = "assets/index-abc123.js";
const PLAIN_ASSET = "fonts/stand-in.ttf";

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
let dist;

/**
 * Proves there is still a process there to answer.
 *
 * The proof is the exchange completing at all: a dead server refuses the connection and `fetch`
 * rejects. Draining the body finishes the exchange, so a half-sent answer fails here rather than
 * passing.
 */
async function assertStillServing() {
  const response = await fetch(`${origin}${BASE_PATH}`);
  await response.arrayBuffer();
  assert.equal(response.status, 200, "the server should still be answering the gallery");
}

before(async () => {
  dist = mkdtempSync(join(tmpdir(), "playground-dist-"));
  mkdirSync(join(dist, "assets"));
  mkdirSync(join(dist, "fonts"));
  writeFileSync(join(dist, "index.html"), SHELL);
  writeFileSync(join(dist, HASHED_ASSET), "export {};");
  writeFileSync(join(dist, PLAIN_ASSET), "not really a font");

  const port = await freePort();
  origin = `http://127.0.0.1:${port}`;
  child = spawn(process.execPath, [entry], {
    env: { ...process.env, PORT: String(port), PLAYGROUND_DIST: dist },
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
  if (dist !== undefined) {
    rmSync(dist, { recursive: true, force: true });
  }
});

test("the root redirects, temporarily, to the site", async () => {
  const response = await fetch(`${origin}/`, { redirect: "manual" });
  await response.arrayBuffer();
  assert.equal(response.status, 302);
  assert.equal(response.headers.get("location"), BASE_PATH);
});

test("nothing outside the prefix is served, least of all the shell", async () => {
  for (const path of ["/elsewhere", "/playgrounds", "/api/compile", "/index.html"]) {
    const response = await fetch(`${origin}${path}`);
    const body = await response.json();
    assert.equal(response.status, 404, path);
    assert.ok(body.error, path);
  }
});

test("the gallery is the prefix, with or without a trailing slash, and is never cached", async () => {
  for (const path of [BASE_PATH, `${BASE_PATH}/`]) {
    const response = await fetch(`${origin}${path}`);
    assert.equal(response.status, 200, path);
    assert.equal(await response.text(), SHELL, path);
    assert.match(response.headers.get("content-type"), /text\/html/);
    assert.equal(response.headers.get("cache-control"), "no-cache", path);
  }
});

test("the shell is not cached when it is asked for by name either", async () => {
  // `index.html` exists under dist, so the by-name request takes the static-file path; the cache
  // header must still be the shell's, or a proxy would hold a shell naming replaced assets for a day.
  for (const path of [`${BASE_PATH}/index.html`, `${BASE_PATH}/assets/..%2Findex.html`]) {
    const response = await fetch(`${origin}${path}`);
    assert.equal(response.status, 200, path);
    assert.equal(await response.text(), SHELL, path);
    assert.equal(response.headers.get("cache-control"), "no-cache", path);
  }
});

test("an example's address serves the shell for the client router to resolve", async () => {
  const response = await fetch(`${origin}${BASE_PATH}/shapes`);
  assert.equal(response.status, 200);
  assert.equal(await response.text(), SHELL);
  assert.equal(response.headers.get("cache-control"), "no-cache");
});

test("a hashed asset is served as immutable and a plain one is revalidated within a day", async () => {
  const hashed = await fetch(`${origin}${BASE_PATH}/${HASHED_ASSET}`);
  await hashed.arrayBuffer();
  assert.equal(hashed.status, 200);
  assert.equal(hashed.headers.get("cache-control"), "public, max-age=31536000, immutable");
  assert.match(hashed.headers.get("content-type"), /javascript/);

  const plain = await fetch(`${origin}${BASE_PATH}/${PLAIN_ASSET}`);
  await plain.arrayBuffer();
  assert.equal(plain.status, 200);
  assert.equal(plain.headers.get("cache-control"), "public, max-age=86400");
  assert.equal(plain.headers.get("content-type"), "font/ttf");
});

test("a missing asset under the prefix is not found rather than answered with the shell", async () => {
  const response = await fetch(`${origin}${BASE_PATH}/assets/gone.js`);
  const body = await response.json();
  assert.equal(response.status, 404);
  assert.match(body.error, /no such file/);
});

test("a path that climbs out of dist is not found", async () => {
  const response = await fetch(`${origin}${BASE_PATH}/..%2F..%2Fpackage.json`);
  await response.arrayBuffer();
  assert.equal(response.status, 404);
});

test("health answers on GET and HEAD, refuses other methods, and is never stored", async () => {
  const healthy = await fetch(`${origin}${HEALTH_PATH}`);
  assert.equal(healthy.status, 200);
  assert.deepEqual(await healthy.json(), { ok: true });
  assert.equal(healthy.headers.get("cache-control"), "no-store");

  const probed = await fetch(`${origin}${HEALTH_PATH}`, { method: "HEAD" });
  assert.equal(probed.status, 200);
  assert.equal(await probed.text(), "");
  assert.equal(probed.headers.get("cache-control"), "no-store");

  const posted = await fetch(`${origin}${HEALTH_PATH}`, { method: "POST" });
  await posted.arrayBuffer();
  assert.equal(posted.status, 405);
});

test("an API path with no handler is not found rather than the shell", async () => {
  const response = await fetch(`${origin}${API_PREFIX}/nothing`);
  await response.arrayBuffer();
  assert.equal(response.status, 404);
});

test("answers a malformed percent-escape and keeps serving", async () => {
  // `decodeURIComponent` throws on `%ZZ`. Unhandled, that ended the process and every request
  // after it failed to connect at all.
  const malformed = await fetch(`${origin}${BASE_PATH}/%ZZ`);
  await malformed.arrayBuffer();
  assert.equal(malformed.status, 400);

  await assertStillServing();
});

test("answers a stray delimiter at the end of the source and keeps serving", async () => {
  // A source the scanner could not finish scanning used to hang the compile thread, which is the
  // only thread there is: nothing else could be served while it spun.
  const compiled = await fetch(`${origin}${API_PREFIX}/compile`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ source: "@" }),
  });
  assert.equal(compiled.status, 200);
  assert.equal(compiled.headers.get("cache-control"), "no-store");
  const body = await compiled.json();
  assert.equal(body.ir, null);
  assert.ok(body.diagnostics.length > 0);

  await assertStillServing();
});

test("a language answer is never stored either", async () => {
  const response = await fetch(`${origin}${API_PREFIX}/language/hover`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      documents: [{ uri: "nx://playground/playground.nx", source: "let root() = 1\n", version: 1 }],
      uri: "nx://playground/playground.nx",
      position: { line: 0, character: 5 },
    }),
  });
  await response.arrayBuffer();
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("cache-control"), "no-store");
});
