import { strict as assert } from "node:assert";
import { test } from "node:test";
import { createOwnTexts } from "./ownTexts.ts";

test("a value the editor reported is not set again, even once the editor has moved past it", () => {
  const own = createOwnTexts();
  own.reported("one");
  own.reported("two");
  assert.equal(own.shouldSet("one", "two"), false, "a render one step behind");
  assert.equal(own.shouldSet("two", "two"), false, "and the next");
});

test("a value from outside is set", () => {
  const own = createOwnTexts();
  own.reported("typed");
  assert.equal(own.shouldSet("example", "typed"), true);
});

test("a value the editor already holds is not set", () => {
  const own = createOwnTexts();
  assert.equal(own.shouldSet("same", "same"), false);
});

test("texts left behind by changes that ended where they began do not hide a value from outside", () => {
  const own = createOwnTexts();
  assert.equal(own.shouldSet("start", "start"), false);
  // An edit and its undo, batched into one render that passes nothing new, so no render follows.
  own.reported("edited");
  own.reported("start");
  assert.equal(own.shouldSet("edited", "start"), true, "an outside value that happens to match a forgotten text");
});

test("an outside value forgets what the editor reported before it", () => {
  const own = createOwnTexts();
  own.reported("a");
  assert.equal(own.shouldSet("example", "a"), true);
  assert.equal(own.shouldSet("a", "example"), true, "the old text, from outside again");
});

test("before there is an editor, nothing is set and nothing is kept", () => {
  const own = createOwnTexts();
  own.reported("a");
  assert.equal(own.shouldSet("b", null), false);
  assert.equal(own.shouldSet("a", "b"), true, "a reported text is forgotten once a render passes another");
});
