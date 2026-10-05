import assert from "node:assert/strict";
import { test } from "node:test";

import {
  FUNCTION_TOOL_ANNOTATIONS,
  HTTP_TOOL_METHODS,
  WEB_SEARCH_TOOL_ANNOTATIONS,
  httpToolAnnotations,
  isHttpToolMethod,
} from "../src/annotations.js";

// The spec's values, written out rather than derived, as [readOnly, destructive, idempotent, openWorld].
const expected = {
  function: [true, false, true, false],
  web_search: [true, false, true, true],
  get: [true, false, true, true],
  post: [false, true, false, true],
  put: [false, true, true, true],
  patch: [false, true, false, true],
  delete: [false, true, true, true],
} as const;

function hints(annotations: { readOnlyHint: boolean; destructiveHint: boolean; idempotentHint: boolean; openWorldHint: boolean }): boolean[] {
  return [annotations.readOnlyHint, annotations.destructiveHint, annotations.idempotentHint, annotations.openWorldHint];
}

test("a function tool is read-only, idempotent and closed-world", () => {
  assert.deepEqual(hints(FUNCTION_TOOL_ANNOTATIONS), expected.function);
  assert.deepEqual(Object.keys(FUNCTION_TOOL_ANNOTATIONS), ["readOnlyHint", "destructiveHint", "idempotentHint", "openWorldHint"]);
});

test("a web search tool is read-only and open-world", () => {
  assert.deepEqual(hints(WEB_SEARCH_TOOL_ANNOTATIONS), expected.web_search);
});

test("an HTTP tool's hints follow its method", () => {
  assert.deepEqual([...HTTP_TOOL_METHODS], ["get", "post", "put", "patch", "delete"]);
  for (const method of HTTP_TOOL_METHODS) {
    assert.deepEqual(hints(httpToolAnnotations(method)), expected[method], method);
  }
});

test("the tables cannot be changed", () => {
  assert.throws(() => {
    (FUNCTION_TOOL_ANNOTATIONS as { readOnlyHint: boolean }).readOnlyHint = false;
  }, TypeError);
  assert.throws(() => {
    (httpToolAnnotations("get") as { destructiveHint: boolean }).destructiveHint = true;
  }, TypeError);
});

test("only the five methods are methods", () => {
  for (const value of ["GET", "head", "", undefined, 1]) {
    assert.equal(isHttpToolMethod(value), false, String(value));
  }
});
