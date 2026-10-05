import assert from "node:assert/strict";
import { test } from "node:test";

import { withoutUndefinedMembers } from "../src/json.js";

test("a member that is undefined is left out at any depth, and everything else is kept as it is", () => {
  const record = {
    $type: "ChatToolContext",
    conversationId: "conv_9",
    contactEmail: undefined,
    profile: { $type: "Profile", name: "Ada", nickname: undefined, tags: ["a", { label: "b", note: undefined }], empty: {} },
    count: 0,
    flag: false,
    nothing: null,
    text: "",
  };
  const copy = withoutUndefinedMembers(record, 100);
  assert.deepEqual(copy, {
    $type: "ChatToolContext",
    conversationId: "conv_9",
    profile: { $type: "Profile", name: "Ada", tags: ["a", { label: "b" }], empty: {} },
    count: 0,
    flag: false,
    nothing: null,
    text: "",
  });
  // The keys are in the record's order, and no key holds `undefined`.
  assert.deepEqual(Object.keys(copy as object), ["$type", "conversationId", "profile", "count", "flag", "nothing", "text"]);
  assert.equal(JSON.stringify(copy), JSON.stringify(record));
  // The record given is not changed, and the copy shares nothing with it.
  assert.equal("contactEmail" in record, true);
  assert.notEqual((copy as typeof record).profile, record.profile);
});

test("an item of a list is kept as it is, undefined included, and a value that is not a record or a list is itself", () => {
  assert.deepEqual(withoutUndefinedMembers({ items: [1, undefined, 3] }, 100), { items: [1, undefined, 3] });
  for (const value of ["text", 7, true, null, undefined]) {
    assert.equal(withoutUndefinedMembers(value, 100), value);
  }
});

test("a value that is not a list or a plain record is not entered: it is kept as it is and counts as one", () => {
  class Point {
    public constructor(
      public x: number,
      public y: number | undefined,
    ) {}
  }
  const kept = [new Uint8Array(3_000), Buffer.alloc(3_000), new Date(0), new Map([["a", 1]]), new Set([1]), new Point(1, undefined), /x/, () => 1];
  for (const value of kept) {
    // The record, `extra` and nothing of what `extra` holds: within a bound of two.
    const copy = withoutUndefinedMembers({ extra: value, gone: undefined }, 2) as { extra: unknown };
    assert.deepEqual(Object.keys(copy), ["extra"]);
    assert.equal(copy.extra, value, Object.prototype.toString.call(value));
  }
  // An item of a list likewise.
  const bytes = new Uint8Array(8);
  assert.equal((withoutUndefinedMembers([bytes], 2) as unknown[])[0], bytes);
  // A very large one costs nothing to pass over.
  const large = new Uint8Array(20_000_000);
  const started = performance.now();
  assert.equal((withoutUndefinedMembers({ extra: large }, 2) as { extra: unknown }).extra, large);
  assert.ok(performance.now() - started < 50, "the typed array was read");
  // A caller that knows the value itself is a record says so, and only the value itself is then
  // read that way: an instance below it is still kept as it is.
  const inner = new Point(2, undefined);
  const outer = Object.assign(new Point(1, undefined), { inner });
  assert.equal(withoutUndefinedMembers(outer, 10), outer);
  const read = withoutUndefinedMembers(outer, 10, true) as { x: number; inner: unknown };
  assert.deepEqual(Object.keys(read), ["x", "inner"]);
  assert.equal(Object.getPrototypeOf(read), Object.prototype);
  assert.equal(read.inner, inner);
  // Saying so of a list, or of something that is not an object, changes nothing.
  assert.deepEqual(withoutUndefinedMembers([1, undefined], 10, true), [1, undefined]);
  assert.equal(withoutUndefinedMembers("text", 10, true), "text");
  // A record with no prototype, as `Object.create(null)` and some parsers make, is a record.
  const bare = Object.assign(Object.create(null) as Record<string, unknown>, { a: 1, b: undefined });
  assert.deepEqual(withoutUndefinedMembers(bare, 10), { a: 1 });
});

test("a key named __proto__ is a member of its own, and the copy's prototype is the ordinary one", () => {
  const record = JSON.parse('{"$type":"T","__proto__":{"polluted":true},"n":1}') as object;
  const copy = withoutUndefinedMembers(record, 100) as Record<string, unknown>;
  assert.deepEqual(Object.keys(copy), ["$type", "__proto__", "n"]);
  assert.equal(Object.getPrototypeOf(copy), Object.prototype);
  assert.equal((copy as { polluted?: boolean }).polluted, undefined);
  assert.equal(({} as { polluted?: boolean }).polluted, undefined);
});

test("a value of more values than the bound is refused, exactly at the bound", () => {
  // The record, and its three members.
  const record = { a: 1, b: 2, c: 3 };
  assert.deepEqual(withoutUndefinedMembers(record, 4), record);
  assert.equal(withoutUndefinedMembers(record, 3), undefined);
  // A member that is undefined is not counted: it is not kept.
  assert.deepEqual(withoutUndefinedMembers({ ...record, d: undefined, e: undefined }, 4), record);
  // A list counts its items.
  assert.deepEqual(withoutUndefinedMembers([1, 2, 3], 4), [1, 2, 3]);
  assert.equal(withoutUndefinedMembers([1, 2, 3], 3), undefined);
});

test("a value that holds itself, one nested very deeply and a very large one all end, without the engine's stack", () => {
  const cyclic: Record<string, unknown> = { $type: "ChatToolContext" };
  cyclic["self"] = cyclic;
  assert.equal(withoutUndefinedMembers(cyclic, 2_002), undefined);

  let deep: Record<string, unknown> = { leaf: true };
  for (let depth = 0; depth < 200_000; depth += 1) {
    deep = { inner: deep };
  }
  assert.equal(withoutUndefinedMembers(deep, 2_002), undefined);
  // Under a bound that admits it, the deep value is copied whole.
  let copied = withoutUndefinedMembers(deep, 1_000_000) as Record<string, unknown>;
  let depth = 0;
  while ("inner" in copied) {
    copied = copied["inner"] as Record<string, unknown>;
    depth += 1;
  }
  assert.equal(depth, 200_000);
  assert.deepEqual(copied, { leaf: true });

  // A list of two million items is refused before any of them is read.
  const started = performance.now();
  assert.equal(withoutUndefinedMembers({ items: new Array<number>(2_000_000).fill(1) }, 2_002), undefined);
  assert.ok(performance.now() - started < 100, "the list was walked");
});
