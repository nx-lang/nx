import { strict as assert } from "node:assert";
import { test } from "node:test";
import worker, { isShellPath } from "./index.mjs";

const SHELL = "<!doctype html><title>shell</title>";

/** A stand-in for the static assets binding, which holds only the shell. */
const env = {
  ASSETS: {
    async fetch(request) {
      const { pathname } = new URL(request.url);
      return pathname === "/playground/index.html"
        ? new Response(SHELL, { headers: { "content-type": "text/html", "cache-control": "no-cache" } })
        : new Response("missing", { status: 404 });
    },
  },
};

async function get(path) {
  return worker.fetch(new Request(`https://nxlang.org${path}`), env);
}

for (const path of ["/playground", "/playground/", "/playground/cards", "/playground/cards/"]) {
  test(`${path} is the shell`, async () => {
    const response = await get(path);
    assert.equal(response.status, 200);
    assert.equal(await response.text(), SHELL);
    assert.equal(response.headers.get("cache-control"), "no-cache");
  });
}

for (const path of ["/playground/assets/missing.js", "/playground/api/health", "/playground/missing.js", "/playgrounds", "/"]) {
  test(`${path} is not found, and not the shell`, async () => {
    const response = await get(path);
    assert.equal(response.status, 404);
    assert.notEqual(await response.text(), SHELL);
  });
}

test("a HEAD request for a route is answered like a GET", async () => {
  let seen;
  const response = await worker.fetch(new Request("https://nxlang.org/playground/cards", { method: "HEAD" }), {
    ASSETS: { fetch: async (request) => ((seen = request), new Response(null)) },
  });
  assert.equal(response.status, 200);
  assert.equal(seen.method, "HEAD");
  assert.equal(new URL(seen.url).pathname, "/playground/index.html");
});

test("only one segment below the prefix is a route", () => {
  assert.equal(isShellPath("/playground/a/b"), false);
  assert.equal(isShellPath("/playground//"), false);
});
