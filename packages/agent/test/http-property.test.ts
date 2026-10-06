import assert from "node:assert/strict";
import { test } from "node:test";

import { buildHttpRequest } from "../src/index.js";
import { httpArguments, httpTool } from "./http-support.js";

const CASES = 12_000;
const SEED = 0x6e78_6167;

/** A small seeded generator (mulberry32), so a failing case is the same case on every run. */
function generator(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let mixed = state;
    mixed = Math.imul(mixed ^ (mixed >>> 15), mixed | 1);
    mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), mixed | 61);
    return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
  };
}

/** Pieces a value is assembled from: everything that means something to a URL parser. */
const TOKENS = [
  "/", "//", "..", ".", "../", "/..", "%2e%2e", "%2E", "%2f", "%5c", "%00", "%", "%zz", "?", "#", "@", "\\", "\\\\", ":", ";",
  "&", "=", "+", " ", "\t", "\r\n", "\n", "\u0000", "\u001f", "\u007f", "é", "日本", "𝄞", "‮", "﻿", "\ud83d", "\udc00",
  "https://evil.example", "//evil.example", "evil.example", "user:pw@", ":8080", "{", "}", "{id}", "'", "\"", "<", ">", "`", "^", "|",
  "!", "(", ")", "*", "~", "-", "_", "a", "Z", "0", "orders", "admin", "v1",
];

function randomValue(random: () => number): string {
  const pieces = 1 + Math.floor(random() * 5);
  let value = "";
  for (let index = 0; index < pieces; index += 1) {
    if (random() < 0.8) {
      value += TOKENS[Math.floor(random() * TOKENS.length)]!;
    } else {
      value += String.fromCharCode(Math.floor(random() * 0x3000));
    }
  }
  return value;
}

/**
 * What a value is once lone surrogates, which have no UTF-8 form, are replaced as an encoder
 * replaces them. The decoder keeps a leading U+FEFF, which is data here and which it would
 * otherwise take for a byte order mark and drop.
 */
function wellFormed(value: string): string {
  return new TextDecoder("utf-8", { ignoreBOM: true }).decode(new TextEncoder().encode(value));
}

interface Shape {
  readonly baseUrl: string;
  readonly path: string;
  /** The path's segments after the base path: a literal, or the placeholders whose values fill it in order with its literal text. */
  readonly segments: readonly (string | { readonly prefix: string; readonly names: readonly string[]; readonly suffix: string })[];
}

const SHAPES: readonly Shape[] = [
  { baseUrl: "https://api.example.com", path: "/orders/{a}", segments: ["orders", { prefix: "", names: ["a"], suffix: "" }] },
  {
    baseUrl: "https://api.example.com/v1",
    path: "/accounts/{a}/orders/{b}/lines",
    segments: ["accounts", { prefix: "", names: ["a"], suffix: "" }, "orders", { prefix: "", names: ["b"], suffix: "" }, "lines"],
  },
  {
    baseUrl: "https://api.example.com:8443/api/v2",
    path: "/files/{a}.json",
    segments: ["files", { prefix: "", names: ["a"], suffix: ".json" }],
  },
  { baseUrl: "https://api.example.com/v1", path: "/x/{a}{b}/end", segments: ["x", { prefix: "", names: ["a", "b"], suffix: "" }, "end"] },
  { baseUrl: "https://api.example.com", path: "/x/.{a}", segments: ["x", { prefix: ".", names: ["a"], suffix: "" }] },
  { baseUrl: "https://api.example.com/v1", path: "/x/%2e{a}/y", segments: ["x", { prefix: "%2e", names: ["a"], suffix: "" }, "y"] },
];

test(`over ${CASES} random values, a built URL keeps its origin, its base path and its literal segments`, () => {
  const random = generator(SEED);
  let builtCount = 0;
  const refusals = new Map<string, number>();
  for (let index = 0; index < CASES; index += 1) {
    const shape = SHAPES[index % SHAPES.length]!;
    const values = { a: randomValue(random), b: randomValue(random) };
    const names = shape.path.includes("{b}") ? (["a", "b"] as const) : (["a"] as const);
    const query: [string, string][] = [];
    for (let pair = Math.floor(random() * 3); pair > 0; pair -= 1) {
      query.push([randomValue(random), randomValue(random)]);
    }
    const tool = httpTool("get", shape.path, shape.baseUrl);
    const result = buildHttpRequest(tool, httpArguments(names.map((name) => [name, values[name]] as const), query));
    const context = `case ${index}: ${JSON.stringify({ path: shape.path, values, query })}`;

    if (!result.ok) {
      // A random value is refused only for being a dot segment: values and names here are never
      // empty. The parse-and-compare check is a backstop, and reaching it would mean a rule above
      // it let something through.
      assert.equal(result.error.rule, "dot-segment", `${context}: ${result.error.message}`);
      refusals.set(result.error.rule, (refusals.get(result.error.rule) ?? 0) + 1);
      continue;
    }
    builtCount += 1;
    const { url, method } = result.request;
    const parsed = new URL(url);
    const base = new URL(shape.baseUrl);
    assert.equal(method, "GET", context);
    assert.equal(parsed.href, url, context);
    assert.equal(parsed.origin, base.origin, context);
    assert.equal(parsed.username + parsed.password + parsed.hash, "", context);
    assert.match(url, /^[A-Za-z0-9\-._~:/?&=%]+$/, context);

    const basePath = base.pathname === "/" ? "" : base.pathname;
    assert.ok(parsed.pathname.startsWith(`${basePath}/`), context);
    const segments = parsed.pathname.slice(basePath.length + 1).split("/");
    assert.equal(segments.length, shape.segments.length, context);
    for (const [position, expected] of shape.segments.entries()) {
      const segment = segments[position]!;
      if (typeof expected === "string") {
        assert.equal(segment, expected, context);
      } else {
        // The segment is the literal text around the values, and the values come back out of it.
        assert.ok(segment.startsWith(expected.prefix) && segment.endsWith(expected.suffix), context);
        const filled = segment.slice(expected.prefix.length, segment.length - expected.suffix.length);
        // Each value is encoded on its own: the last half of one and the first half of the next
        // are two lone surrogates, never a pair.
        const expectedText = expected.names.map((name) => wellFormed(values[name as "a" | "b"])).join("");
        assert.equal(decodeURIComponent(filled), expectedText, context);
      }
    }

    const pairs = parsed.search === "" ? [] : parsed.search.slice(1).split("&").map((pair) => pair.split("="));
    assert.deepEqual(
      pairs.map((pair) => pair.map((part) => decodeURIComponent(part))),
      query.map(([name, value]) => [wellFormed(name), wellFormed(value)]),
      context,
    );
  }
  // Both outcomes are reached, so neither branch of the loop is vacuous.
  assert.ok(builtCount > CASES * 0.9, `only ${builtCount} of ${CASES} were built`);
  assert.ok((refusals.get("dot-segment") ?? 0) > 10, `only ${refusals.get("dot-segment") ?? 0} dot segments were generated`);
});
