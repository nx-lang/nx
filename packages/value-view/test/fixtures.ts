/**
 * Values as `evaluateNx()` returns them, from `test/fixtures/values.json`, which
 * `scripts/update-fixtures.mjs` writes from `test/fixtures/sources.json`. `fixtures.test.ts` fails
 * when the compiler's output drifts from what is committed.
 */
import { readFileSync } from "node:fs";

import type { NxValueText } from "../src/types.js";

const values = JSON.parse(
  readFileSync(new URL("../../test/fixtures/values.json", import.meta.url), "utf8")
) as Record<string, NxValueText>;

function fixture(name: string): NxValueText {
  const value = values[name];
  if (value === undefined) {
    throw new Error(`No fixture '${name}' in test/fixtures/values.json.`);
  }
  return value;
}

/** `<User id="1" name="Ada" />`, the example the specs use. */
export const user = fixture("user");

/** A record with an optional property. */
export const card = fixture("card");

/** One record spanning several lines, with a record nested in a property. */
export const person = fixture("person");

/** A sequence of three `Task` records that each span several lines. */
export const tasks = fixture("tasks");

/** Records nested in sequences nested in properties, several deep, as markup evaluates to. */
export const nested = fixture("nested");

/** A top-level string, which has no declaration. */
export const greeting = fixture("greeting");
