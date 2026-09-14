/**
 * Loading the compiler module: a slow download is waited for, a stalled one is not waited for
 * forever, and both kinds of failure reach the visitor as the same readable fault.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { loadNxModule } from "./module.ts";

/** Stands in for the compiled module; nothing here inspects it. */
const compiled = { module: true };

/** A fetch that never answers on its own, and rejects when the load gives up on it. */
function stalling(record = {}) {
  return (url, init) =>
    new Promise((_, reject) => {
      record.url = url;
      init.signal.addEventListener("abort", () => {
        record.aborted = true;
        reject(new Error("The operation was aborted."));
      });
    });
}

test("a module that arrives is compiled as it streams", async () => {
  const module = await loadNxModule("/nx.wasm", {
    fetch: () => Promise.resolve("a response"),
    compileStreaming: async (response) => {
      assert.equal(await response, "a response");
      return compiled;
    },
  });

  assert.equal(module, compiled);
});

test("a download slower than a request's deadline is still waited for", async () => {
  // 60 ms is many times the 10 ms budget below, the way a real download is many times a compile's.
  const module = await loadNxModule("/nx.wasm", {
    deadlineMs: 1_000,
    fetch: () => new Promise((resolve) => setTimeout(() => resolve("a slow response"), 60)),
    compileStreaming: async (response) => {
      await response;
      return compiled;
    },
  });

  assert.equal(module, compiled);
});

test("a download that stalls is given up on, and says so in words the pane can use", async () => {
  const record = {};

  await assert.rejects(
    loadNxModule("/nx.wasm", { deadlineMs: 20, fetch: stalling(record), compileStreaming: (r) => r }),
    (error) => {
      assert.equal(error.name, "NxModuleLoadError");
      assert.match(error.message, /could not be loaded/);
      // The budget, not whatever the abort happened to say: "The operation was aborted" tells a
      // visitor nothing about what to do.
      assert.match(error.message, /did not finish within 0s/);
      return true;
    },
  );

  // The connection is released rather than left hanging on a body that is not coming.
  assert.equal(record.aborted, true);
  assert.equal(record.url, "/nx.wasm");
});

test("a download that fails keeps the reason it failed for", async () => {
  await assert.rejects(
    loadNxModule("/nx.wasm", {
      fetch: () => Promise.reject(new Error("Failed to fetch")),
      compileStreaming: (response) => response,
    }),
    (error) => {
      assert.equal(error.name, "NxModuleLoadError");
      assert.match(error.message, /could not be loaded: Failed to fetch/);
      return true;
    },
  );
});

test("a module the browser refuses is reported the same way as one that never arrived", async () => {
  await assert.rejects(
    loadNxModule("/nx.wasm", {
      fetch: () => Promise.resolve("a response"),
      compileStreaming: () => Promise.reject(new TypeError("Incorrect response MIME type.")),
    }),
    (error) => {
      assert.equal(error.name, "NxModuleLoadError");
      assert.match(error.message, /could not be loaded: Incorrect response MIME type/);
      return true;
    },
  );
});
