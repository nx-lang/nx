/**
 * The compile seam in the browser: what it retries, and the sentences the diagnostics pane shows
 * when the compiler itself failed.
 *
 * Each fault is named by the layer that produced it, so these are what the visitor reads for a
 * crash, a deadline, a worker that died and a module that would not load.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { compileFailureMessage, retrying } from "./worker.ts";

/** An error as the channel rebuilds it from what crossed from the worker. */
function named(name, message) {
  const error = new Error(message);
  error.name = name;
  return error;
}

test("a crashed compiler says it restarted and that editing continues", () => {
  const message = compileFailureMessage(named("NxHostCrashedError", "trap in nx_wasm_program_build"));
  assert.match(message, /crashed/);
  assert.match(message, /restarted/);
});

test("a deadline keeps the channel's own wording and says the next compile will run", () => {
  const message = compileFailureMessage(
    named("TimeoutError", "The compiler did not answer within 10s."),
  );
  assert.match(message, /did not answer within 10s/);
  assert.match(message, /next compile will run/);
});

test("a module that would not load asks for a reload", () => {
  const message = compileFailureMessage(
    named("NxModuleLoadError", "The compiler could not be loaded: Failed to fetch"),
  );
  assert.match(message, /could not be loaded/);
  assert.match(message, /Reload the page/);
});

test("a worker that stopped says it will be started again", () => {
  const message = compileFailureMessage(
    named("WorkerStoppedError", "The compiler worker stopped unexpectedly."),
  );
  assert.match(message, /started again/);
});

test("anything else is still readable", () => {
  assert.match(compileFailureMessage(new Error("something odd")), /The compiler failed: something odd/);
  assert.match(compileFailureMessage("not an error"), /The compiler failed: not an error/);
});

test("a recoverable fault is retried once, so a preview that never edits still draws", async () => {
  const attempts = [];
  const compile = retrying((source) => {
    attempts.push(source);
    return attempts.length === 1
      ? Promise.reject(named("TimeoutError", "The compiler did not answer within 10s."))
      : Promise.resolve({ ir: { ok: true }, diagnostics: [] });
  });

  const result = await compile("let root() = { 42 }");

  assert.deepEqual(result.ir, { ok: true });
  assert.equal(attempts.length, 2);
});

test("a fault that will not get better is not retried", async () => {
  let attempts = 0;
  const compile = retrying(() => {
    attempts += 1;
    return Promise.reject(named("NxModuleLoadError", "The compiler could not be loaded: Failed to fetch"));
  });

  await assert.rejects(compile("let root() = { 42 }"), /Reload the page/);
  assert.equal(attempts, 1);
});

test("a fault that repeats is reported rather than retried again", async () => {
  let attempts = 0;
  const compile = retrying(() => {
    attempts += 1;
    return Promise.reject(named("NxHostCrashedError", "trap in nx_wasm_program_build"));
  });

  await assert.rejects(compile("let root() = { 42 }"), /crashed/);
  assert.equal(attempts, 2);
});
