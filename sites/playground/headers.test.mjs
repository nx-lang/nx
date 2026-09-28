/**
 * Proves `_headers` declares the cache policy the site's requirement asks for. The Worker's tests
 * stub the static assets binding, and the deploy's smoke test reads status and type, so this is
 * what catches a typo in a rule.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { BASE_PATH } from "./base.mjs";

/** The rules in `_headers`, as a map from path pattern to its `Cache-Control` value. */
function cacheRules() {
  const text = readFileSync(new URL("./_headers", import.meta.url), "utf8");
  const rules = new Map();
  let pattern;
  for (const line of text.split("\n")) {
    if (line.trim() === "" || line.startsWith("#")) {
      continue;
    }
    if (!/^\s/.test(line)) {
      pattern = line.trim();
      continue;
    }
    const match = /^\s+Cache-Control:\s*(.+)$/i.exec(line);
    if (match) {
      rules.set(pattern, match[1].trim());
    }
  }
  return rules;
}

const rules = cacheRules();

test("hashed assets are public, immutable and held for a year", () => {
  assert.equal(rules.get(`${BASE_PATH}/assets/*`), "public, max-age=31536000, immutable");
});

test("the shell is revalidated on every use", () => {
  assert.equal(rules.get(`${BASE_PATH}/index.html`), "no-cache");
});
