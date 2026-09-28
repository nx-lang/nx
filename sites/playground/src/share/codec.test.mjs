/**
 * The address encoding: a fixture made by `node:zlib` decodes to its source, any source survives a
 * round trip, and a payload that is not base64url, not DEFLATE or not UTF-8 is refused by name.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { deflateRawSync } from "node:zlib";
import { test } from "node:test";
import {
  ShareDecodeError,
  decodeSource,
  encodeSource,
  fragmentForPayload,
  payloadFromFragment,
  toBase64Url,
} from "./codec.ts";

const fixture = JSON.parse(readFileSync(new URL("./fixture.json", import.meta.url), "utf8"));

test("the shared fixture decodes to its source exactly", async () => {
  assert.equal(await decodeSource(fixture.payload), fixture.source);
});

test("any source survives a round trip, whatever it holds", async () => {
  const alphabet = "abcXYZ019 \n\t{}<>=\"'/\\@éü日本🎉\u0000";
  let seed = 7;
  const random = () => ((seed = (seed * 1103515245 + 12345) % 2 ** 31) / 2 ** 31);
  for (let run = 0; run < 50; run += 1) {
    const length = Math.floor(random() * 400);
    const source = Array.from({ length }, () => [...alphabet][Math.floor(random() * [...alphabet].length)]).join("");
    assert.equal(await decodeSource(await encodeSource(source)), source);
  }
  assert.equal(await decodeSource(await encodeSource("")), "");
});

test("a payload is unpadded base64url", async () => {
  const payload = await encodeSource("let root() = { 42 }".repeat(10));
  assert.match(payload, /^[A-Za-z0-9_-]+$/);
});

test("a stream from another DEFLATE implementation decodes too", async () => {
  const source = "type Task = { title:string }\n<Task title=\"zlib\" />";
  const payload = toBase64Url(new Uint8Array(deflateRawSync(Buffer.from(source), { level: 1 })));
  assert.equal(await decodeSource(payload), source);
});

test("a payload that is not base64url is refused", async () => {
  await assert.rejects(decodeSource("abc$def"), ShareDecodeError);
  await assert.rejects(decodeSource("abcde"), ShareDecodeError);
  await assert.rejects(decodeSource("ab=="), ShareDecodeError);
});

test("a payload that does not inflate is refused", async () => {
  await assert.rejects(decodeSource(toBase64Url(new Uint8Array([0xff, 0xff, 0xff, 0xff]))), (error) => {
    assert.ok(error instanceof ShareDecodeError);
    assert.match(error.message, /compressed/);
    return true;
  });
});

test("a payload that inflates to something other than UTF-8 is refused", async () => {
  const payload = toBase64Url(new Uint8Array(deflateRawSync(Buffer.from([0xc3, 0x28, 0xff]))));
  await assert.rejects(decodeSource(payload), (error) => {
    assert.ok(error instanceof ShareDecodeError);
    assert.match(error.message, /UTF-8/);
    return true;
  });
});

test("the fragment is #code=<payload>", () => {
  assert.equal(fragmentForPayload("abc"), "#code=abc");
  assert.equal(payloadFromFragment("#code=abc"), "abc");
  assert.equal(payloadFromFragment("code=abc"), "abc");
  assert.equal(payloadFromFragment("#other=abc"), null);
  assert.equal(payloadFromFragment(""), null);
});
