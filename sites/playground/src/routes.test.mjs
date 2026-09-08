/**
 * Proves the address scheme: everything under the prefix, the gallery at the prefix itself, and an
 * unknown example landing on the gallery rather than on an error.
 */
import { strict as assert } from "node:assert";
import { test } from "node:test";
import { pathForRoute, routeFromPath } from "./routes.ts";

const ROOT = "/playground";
const isExample = (id) => id === "shapes" || id === "uneven-cells";

test("the prefix, with or without a trailing slash, is the gallery", () => {
  assert.deepEqual(routeFromPath("/playground", ROOT, isExample), { kind: "gallery" });
  assert.deepEqual(routeFromPath("/playground/", ROOT, isExample), { kind: "gallery" });
});

test("a known id under the prefix is that example's editor view", () => {
  assert.deepEqual(routeFromPath("/playground/shapes", ROOT, isExample), { kind: "editor", id: "shapes" });
  assert.deepEqual(routeFromPath("/playground/uneven-cells/", ROOT, isExample), {
    kind: "editor",
    id: "uneven-cells",
  });
});

test("an unknown id resolves to the gallery", () => {
  assert.deepEqual(routeFromPath("/playground/nothing-here", ROOT, isExample), { kind: "gallery" });
});

test("addresses that are not the site's own resolve to the gallery", () => {
  // The server never serves the shell for these, so the answer only matters as a default.
  assert.deepEqual(routeFromPath("/", ROOT, isExample), { kind: "gallery" });
  assert.deepEqual(routeFromPath("/playgrounds/shapes", ROOT, isExample), { kind: "gallery" });
  assert.deepEqual(routeFromPath("/playground/shapes/extra", ROOT, isExample), { kind: "gallery" });
});

test("paths are emitted under the prefix", () => {
  assert.equal(pathForRoute({ kind: "gallery" }, ROOT), "/playground");
  assert.equal(pathForRoute({ kind: "editor", id: "shapes" }, ROOT), "/playground/shapes");
});
