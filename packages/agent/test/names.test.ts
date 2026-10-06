import assert from "node:assert/strict";
import { test } from "node:test";

import { NX_AGENT_TOOL_NAME_PATTERN, isToolName, toSnakeCase } from "../src/index.js";

test("a function name converts to snake_case", () => {
  const cases: readonly (readonly [string, string])[] = [
    ["findPlans", "find_plans"],
    ["HTTPStatusFor", "http_status_for"],
    ["PlanRow", "plan_row"],
    ["lookupOrder2", "lookup_order2"],
    ["lookupOrder2Fast", "lookup_order2_fast"],
    ["find", "find"],
    ["Find", "find"],
    ["ID", "id"],
    ["getHTTP", "get_http"],
    ["parseURLFor", "parse_url_for"],
    ["already_snake", "already_snake"],
  ];
  for (const [name, expected] of cases) {
    assert.equal(toSnakeCase(name), expected, name);
  }
});

test("every character that is not an ASCII letter or digit ends a word", () => {
  assert.equal(toSnakeCase("find-plans"), "find_plans");
  assert.equal(toSnakeCase("User.Update"), "user_update");
  assert.equal(toSnakeCase("find  plans!"), "find_plans");
  assert.equal(toSnakeCase("__find__plans__"), "find_plans");
  assert.equal(toSnakeCase("findPläne"), "find_pl_ne");
});

test("a converted name is still checked against the pattern", () => {
  assert.equal(toSnakeCase("2fast"), "2fast");
  assert.equal(isToolName(toSnakeCase("2fast")), false);
  assert.equal(toSnakeCase("—"), "");
  assert.equal(isToolName(""), false);
});

test("the name pattern accepts snake_case of up to 64 characters", () => {
  for (const name of ["a", "find_plans", "lookup_order2", "shop__lookup_order", `a${"b".repeat(63)}`]) {
    assert.equal(isToolName(name), true, name);
  }
  for (const name of ["", "Find Plans", "findPlans", "find-plans", "_find", "2find", "find.plans", `a${"b".repeat(64)}`, "find_plans\n"]) {
    assert.equal(isToolName(name), false, JSON.stringify(name));
  }
  assert.equal(NX_AGENT_TOOL_NAME_PATTERN.source, "^[a-z][a-z0-9_]{0,63}$");
});
