/**
 * The address policy: an edit reaches the address after a pause, whatever the evaluation does, and
 * choosing an example keeps a waiting edit in the entry Back returns to.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { createAddressKeeper } from "./address.ts";

/** A history that records calls, a timer run by hand, and an encoder that answers when told. */
function harness() {
  const calls = [];
  const timers = new Map();
  let nextTimer = 1;
  const encodings = [];
  const keeper = createAddressKeeper({
    history: {
      pushState: (_data, _unused, url) => calls.push(["push", url]),
      replaceState: (_data, _unused, url) => calls.push(["replace", url]),
    },
    root: "/play",
    encode: (source) =>
      new Promise((resolve) => encodings.push(() => resolve(`enc(${source})`))),
    setTimer: (run) => {
      const id = nextTimer++;
      timers.set(id, run);
      return id;
    },
    clearTimer: (id) => timers.delete(id),
  });
  const fireTimers = () => {
    const runs = [...timers.values()];
    timers.clear();
    runs.forEach((run) => run());
  };
  const finishEncodings = async () => {
    while (encodings.length > 0) {
      encodings.shift()();
      await new Promise((resolve) => setImmediate(resolve));
    }
  };
  return { keeper, calls, fireTimers, finishEncodings, timers };
}

test("an edit reaches the address after the pause, replacing the entry", async () => {
  const { keeper, calls, fireTimers, finishEncodings } = harness();
  keeper.edited("a");
  keeper.edited("ab");
  assert.deepEqual(calls, [], "nothing is written while typing");
  fireTimers();
  await finishEncodings();
  assert.deepEqual(calls, [["replace", "/play#code=enc(ab)"]]);
});

test("choosing an example right after an edit keeps the edit in the entry Back returns to", async () => {
  const { keeper, calls, finishEncodings, timers } = harness();
  keeper.edited("edited");
  const chosen = keeper.choose("records");
  assert.equal(timers.size, 0, "the waiting write is taken over by the choice");
  await finishEncodings();
  await chosen;
  assert.deepEqual(calls, [
    ["replace", "/play#code=enc(edited)"],
    ["push", "/play/records"],
  ]);
});

test("choosing while the edit is being encoded still writes it first", async () => {
  const { keeper, calls, fireTimers, finishEncodings } = harness();
  keeper.edited("edited");
  fireTimers();
  const chosen = keeper.choose("records");
  await finishEncodings();
  await chosen;
  assert.deepEqual(calls, [
    ["replace", "/play#code=enc(edited)"],
    ["push", "/play/records"],
  ]);
});

test("choosing with nothing edited only adds the entry", async () => {
  const { keeper, calls } = harness();
  await keeper.choose("records");
  assert.deepEqual(calls, [["push", "/play/records"]]);
});

test("an edit already in the address is not written again on choosing", async () => {
  const { keeper, calls, fireTimers, finishEncodings } = harness();
  keeper.edited("edited");
  fireTimers();
  await finishEncodings();
  await keeper.choose("records");
  assert.deepEqual(calls, [
    ["replace", "/play#code=enc(edited)"],
    ["push", "/play/records"],
  ]);
});

test("Back drops a waiting edit rather than writing it into the entry it arrived at", async () => {
  const { keeper, calls, fireTimers, finishEncodings } = harness();
  keeper.edited("edited");
  keeper.navigated();
  fireTimers();
  await finishEncodings();
  assert.deepEqual(calls, []);
});
