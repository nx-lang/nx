/**
 * The main thread's side of the compiler worker, driven with a fake worker so correlation,
 * cancellation and the deadline can be observed exactly.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { createWorkerChannel } from "./channel.ts";

/** A worker that records what it was sent and answers only when the test says so. */
function fakeWorker(record) {
  const worker = {
    sent: [],
    terminated: false,
    onmessage: null,
    onerror: null,
    postMessage(message) {
      worker.sent.push(message);
    },
    terminate() {
      worker.terminated = true;
    },
    ready() {
      worker.onmessage?.({ data: { kind: "ready" } });
    },
    answer(id, value) {
      worker.onmessage?.({ data: { kind: "ok", id, value } });
    },
    fail(id, error) {
      worker.onmessage?.({ data: { kind: "error", id, error } });
    },
  };
  record.started.push(worker);
  return worker;
}

function channelWith(options = {}) {
  const record = { started: [] };
  const channel = createWorkerChannel({ startWorker: () => fakeWorker(record), ...options });
  return {
    channel,
    record,
    /** The worker the channel is currently driving. */
    get worker() {
      return record.started.at(-1);
    },
  };
}

test("correlates answers by id, whatever order they arrive in", async () => {
  const driver = channelWith();
  const { channel } = driver;
  const first = channel.send({ kind: "compile", source: "a" });
  const second = channel.send({ kind: "compile", source: "b" });
  const worker = driver.worker;

  assert.deepEqual(worker.sent.map((message) => message.source), ["a", "b"]);
  const [firstId, secondId] = worker.sent.map((message) => message.id);
  assert.notEqual(firstId, secondId);

  worker.answer(secondId, "B");
  worker.answer(firstId, "A");

  assert.equal(await first, "A");
  assert.equal(await second, "B");
  channel.dispose();
});

test("rejects with the worker's error, keeping the class name", async () => {
  const driver = channelWith();
  const { channel } = driver;
  const pending = channel.send({ kind: "compile", source: "a" });
  const worker = driver.worker;

  worker.fail(worker.sent[0].id, { name: "NxHostCrashedError", message: "it crashed" });

  await assert.rejects(pending, (error) => {
    assert.equal(error.name, "NxHostCrashedError");
    assert.equal(error.message, "it crashed");
    return true;
  });
  channel.dispose();
});

test("a cancelled request rejects at once and its late answer is dropped", async () => {
  const driver = channelWith();
  const { channel } = driver;
  const controller = new AbortController();
  const pending = channel.send({ kind: "compile", source: "a" }, controller.signal);
  const worker = driver.worker;

  controller.abort();
  await assert.rejects(pending, (error) => error.name === "AbortError");

  // The answer arrives afterwards, as it would for a hover the author has already typed past.
  // Nothing is listening for it, and nothing throws.
  worker.answer(worker.sent[0].id, "too late");
  assert.equal(worker.terminated, false);
  channel.dispose();
});

test("an already-cancelled signal is refused without reaching the worker", async () => {
  const { channel, record } = channelWith();
  const controller = new AbortController();
  controller.abort();

  await assert.rejects(
    () => channel.send({ kind: "compile", source: "a" }, controller.signal),
    (error) => error.name === "AbortError",
  );
  assert.equal(record.started.length, 0);
  channel.dispose();
});

test("a request past the deadline terminates the worker, fails everything in flight, and the next request starts a fresh one", async () => {
  const { channel, record } = channelWith({ deadlineMs: 20 });
  const stuck = channel.send({ kind: "compile", source: "a" });
  const behind = channel.send({ kind: "compile", source: "b" });
  const first = record.started[0];
  first.ready();

  await assert.rejects(stuck, (error) => /did not answer within/.test(error.message));
  await assert.rejects(behind, (error) => /did not answer within/.test(error.message));
  assert.equal(first.terminated, true);
  assert.equal(record.started.length, 1);

  const next = channel.send({ kind: "compile", source: "c" });
  assert.equal(record.started.length, 2);
  const replacement = record.started[1];
  replacement.ready();
  replacement.answer(replacement.sent[0].id, "C");
  assert.equal(await next, "C");

  channel.dispose();
});

test("a worker that stops unexpectedly fails what it was carrying", async () => {
  const { channel, record } = channelWith();
  const pending = channel.send({ kind: "compile", source: "a" });
  record.started[0].onerror?.({});

  await assert.rejects(pending, (error) => /stopped unexpectedly/.test(error.message));
  channel.dispose();
});

test("start loads the worker before anything is asked of it", () => {
  const { channel, record } = channelWith();
  assert.equal(record.started.length, 0);
  channel.start();
  assert.equal(record.started.length, 1);
  channel.dispose();
  assert.equal(record.started[0].terminated, true);
});

test("a request sent before the worker is ready waits for the module rather than timing out", async () => {
  const { channel, record } = channelWith({ deadlineMs: 20 });
  const pending = channel.send({ kind: "compile", source: "a" });
  const worker = record.started[0];

  // Long past the deadline, but the worker has not said it is ready: it is still fetching a module
  // that on a slow connection takes many times this budget, and terminating it would start that
  // fetch again from nothing.
  await delay(60);
  assert.equal(worker.terminated, false);
  assert.equal(record.started.length, 1);

  worker.ready();
  worker.answer(worker.sent[0].id, "A");
  assert.equal(await pending, "A");
  channel.dispose();
});

test("the deadline starts when the worker becomes ready, not when the request was sent", async () => {
  const { channel, record } = channelWith({ deadlineMs: 40 });
  const pending = channel.send({ kind: "compile", source: "a" });
  const worker = record.started[0];

  await delay(60);
  worker.ready();

  await assert.rejects(pending, (error) => /did not answer within/.test(error.message));
  assert.equal(worker.terminated, true);
  channel.dispose();
});

test("a worker that will not start rejects its request and leaves no deadline behind", async () => {
  const record = { started: [] };
  let refuse = true;
  const channel = createWorkerChannel({
    deadlineMs: 20,
    startWorker: () => {
      if (refuse) {
        refuse = false;
        // What a content policy that forbids workers does to `new Worker(...)`.
        throw new Error("Worker construction is not allowed here.");
      }
      return fakeWorker(record);
    },
  });

  await assert.rejects(
    channel.send({ kind: "compile", source: "a" }),
    /Worker construction is not allowed/,
  );

  // The refused request must not still be holding a timer: when it fired it would terminate the
  // worker this one is waiting on and fail it with a timeout it had nothing to do with.
  const next = channel.send({ kind: "compile", source: "b" });
  const worker = record.started.at(-1);
  await delay(50);
  worker.ready();
  worker.answer(worker.sent[0].id, "B");

  assert.equal(await next, "B");
  assert.equal(worker.terminated, false);
  channel.dispose();
});

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
