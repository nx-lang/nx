/**
 * Proves the address scheme: everything under the prefix, the default example at the prefix
 * itself, examples one segment below it, and a `#code=` fragment winning over the path.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { pathForExample, pathForPayload, routeFromLocation } from "./routes.ts";

const ROOT = "/play";
const isExample = (id) => id === "records" || id === "for-ranges";
const route = (path, hash = "") => routeFromLocation(path, hash, ROOT, isExample);

test("the prefix, with or without a trailing slash, is the default example", () => {
  assert.deepEqual(route("/play"), { kind: "default" });
  assert.deepEqual(route("/play/"), { kind: "default" });
});

test("a known id under the prefix is that example", () => {
  assert.deepEqual(route("/play/records"), { kind: "example", id: "records" });
  assert.deepEqual(route("/play/for-ranges/"), { kind: "example", id: "for-ranges" });
});

test("an unknown id is the default example, with the id it asked for", () => {
  assert.deepEqual(route("/play/shapes"), { kind: "default", missing: "shapes" });
});

test("the fragment wins, whatever the path names", () => {
  assert.deepEqual(route("/play", "#code=abc"), { kind: "code", payload: "abc" });
  assert.deepEqual(route("/play/records", "#code=abc"), { kind: "code", payload: "abc" });
  assert.deepEqual(route("/play/shapes", "#code=abc"), { kind: "code", payload: "abc" });
});

test("a fragment that is not #code= is ignored", () => {
  assert.deepEqual(route("/play/records", "#top"), { kind: "example", id: "records" });
});

test("addresses that are not the site's own resolve to the default example", () => {
  // The Worker never serves the shell for these, so the answer only matters as a default.
  assert.deepEqual(route("/"), { kind: "default" });
  assert.deepEqual(route("/plays/records"), { kind: "default" });
  assert.deepEqual(route("/play/records/extra"), { kind: "default" });
});

test("addresses are emitted under the prefix", () => {
  assert.equal(pathForExample("records", ROOT), "/play/records");
  assert.equal(pathForPayload("abc", ROOT), "/play#code=abc");
});
