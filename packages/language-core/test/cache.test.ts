import assert from "node:assert/strict";
import { test } from "node:test";
import type { LanguageDocument } from "@nx-lang/language-protocol";

import { SnapshotCache, documentSetKey, type SnapshotLike } from "../src/cache.js";

const FORM = "nx://tenant/form.nx";

function fakeSnapshot(onDispose: () => void): SnapshotLike {
  return {
    hover: () => null,
    completions: () => ({ uri: FORM, identity: "tenant/form.nx", version: null, items: [] }),
    diagnostics: () => ({ documents: [], workspace: [] }),
    documentSymbols: () => [],
    dispose: onDispose,
  };
}

test("the key changes with text, URI or identity, and ignores version", () => {
  const base: LanguageDocument[] = [{ uri: FORM, source: "let a = 1\n", version: 1 }];
  const key = documentSetKey(base);

  assert.equal(documentSetKey([{ ...base[0]!, version: 99 }]), key);
  assert.notEqual(documentSetKey([{ ...base[0]!, source: "let a = 2\n" }]), key);
  assert.notEqual(documentSetKey([{ ...base[0]!, uri: "nx://tenant/other.nx" }]), key);
  assert.notEqual(documentSetKey([{ ...base[0]!, identity: "tenant/form.nx" }]), key);
});

test("no two different document sets share a key", () => {
  // Without the length prefixes these two would join into the same string.
  const first = documentSetKey([{ uri: "a", source: "bc" }]);
  const second = documentSetKey([{ uri: "ab", source: "c" }]);
  assert.notEqual(first, second);

  const oneDocument = documentSetKey([{ uri: FORM, source: "x\ny" }]);
  const twoDocuments = documentSetKey([
    { uri: FORM, source: "x" },
    { uri: FORM, source: "y" },
  ]);
  assert.notEqual(oneDocument, twoDocuments);
});

test("the cache returns the held snapshot and evicts the least recently used", () => {
  const disposed: string[] = [];
  const cache = new SnapshotCache<SnapshotLike>(2);

  const first = cache.getOrCreate("a", () => fakeSnapshot(() => disposed.push("a")));
  assert.equal(cache.getOrCreate("a", () => assert.fail("should not rebuild")), first);

  cache.getOrCreate("b", () => fakeSnapshot(() => disposed.push("b")));
  // Touching "a" makes "b" the least recently used, so "b" is what the third entry evicts.
  cache.getOrCreate("a", () => assert.fail("should not rebuild"));
  cache.getOrCreate("c", () => fakeSnapshot(() => disposed.push("c")));

  assert.deepEqual(disposed, ["b"]);
  assert.equal(cache.size, 2);
});

test("clear disposes everything held", () => {
  const disposed: string[] = [];
  const cache = new SnapshotCache<SnapshotLike>(4);
  for (const key of ["a", "b"]) {
    cache.getOrCreate(key, () => fakeSnapshot(() => disposed.push(key)));
  }

  cache.clear();
  assert.deepEqual(disposed.sort(), ["a", "b"]);
  assert.equal(cache.size, 0);
});

test("a cache size that is not a positive integer is refused", () => {
  assert.throws(() => new SnapshotCache<SnapshotLike>(0), RangeError);
  assert.throws(() => new SnapshotCache<SnapshotLike>(1.5), RangeError);
});
