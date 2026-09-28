/**
 * The website's playground links: they encode a block's text the way the playground decodes it,
 * and only complete programs get one.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { inflateRawSync } from "node:zlib";
import { encodeSource, opensInPlayground, playgroundHref } from "../src/expressive-code/playground-link.mjs";

const fixture = JSON.parse(
  readFileSync(new URL("../../playground/src/share/fixture.json", import.meta.url), "utf8")
);

test("the shared fixture source encodes to the shared fixture payload", () => {
  assert.equal(encodeSource(fixture.source), fixture.payload);
});

test("a link carries the block's text exactly", () => {
  const source = 'type Task = { title:string }\n<Task title="Ünïcødé 🎉" />';
  const href = playgroundHref(source);
  assert.match(href, /^\/play#code=[A-Za-z0-9_-]+$/);
  const payload = href.slice("/play#code=".length);
  assert.equal(inflateRawSync(Buffer.from(payload, "base64url")).toString("utf8"), source);
});

test("complete and invalid blocks get a link; fragments, output and other languages do not", () => {
  assert.equal(opensInPlayground("nx", ""), true);
  assert.equal(opensInPlayground("nx", 'title="tasks.nx"'), true);
  assert.equal(opensInPlayground("nx", "invalid"), true);
  assert.equal(opensInPlayground("nx", "fragment"), false);
  assert.equal(opensInPlayground("nx", 'output title="Evaluates to" wrap'), false);
  assert.equal(opensInPlayground("ts", ""), false);
});
