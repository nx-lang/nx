import assert from "node:assert/strict";
import { test } from "node:test";

import type { NxIrDiagnostic } from "@nx-lang/ir-runtime";

import { classifyRuntimeFailure } from "../src/classify.js";

function diagnostic(code: string, limit?: NxIrDiagnostic["limit"]): NxIrDiagnostic {
  return { severity: "error", code, message: code, ...(limit === undefined ? {} : { limit }) };
}

test("the boundary failures the runtime reports today are invalid input", () => {
  for (const code of ["nx-ir-boundary-type", "nx-ir-boundary-field", "nx-ir-arguments"]) {
    assert.deepEqual(classifyRuntimeFailure([diagnostic(code)]), { code: "invalid-input" }, code);
  }
  assert.deepEqual(
    classifyRuntimeFailure([diagnostic("nx-ir-boundary-type"), diagnostic("nx-ir-arguments"), diagnostic("nx-ir-boundary-field")]),
    { code: "invalid-input" },
  );
});

test("a boundary failure of a kind added later is invalid input", () => {
  // A code this package has no knowledge of: a value outside a constrained type, say.
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-boundary-constraint")]), { code: "invalid-input" });
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-boundary-constraint"), diagnostic("nx-ir-boundary-type")]), { code: "invalid-input" });
});

test("one failure that is not of the boundary makes it an evaluation failure", () => {
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-boundary-constraint"), diagnostic("nx-ir-division-by-zero")]), { code: "evaluation-failed" });
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-division-by-zero")]), { code: "evaluation-failed" });
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-function-value")]), { code: "evaluation-failed" });
});

test("only the family's prefix counts, not a code that merely resembles it", () => {
  for (const code of ["nx-ir-boundary", "nx-ir-boundaries-type", "x-nx-ir-boundary-type", "nx-ir-argument", "nx-ir-arguments-extra", ""]) {
    assert.deepEqual(classifyRuntimeFailure([diagnostic(code)]), { code: "evaluation-failed" }, JSON.stringify(code));
  }
});

test("a limit is a resource limit whatever is beside it, and carries the limit", () => {
  const limit = { name: "maxOperations", value: 100_000 };
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-resource-limit", limit)]), { code: "resource-limit", limit });
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-boundary-type"), diagnostic("nx-ir-resource-limit", limit)]), { code: "resource-limit", limit });
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-resource-limit")]), { code: "resource-limit" });
});

test("a failure with no diagnostics is an evaluation failure", () => {
  assert.deepEqual(classifyRuntimeFailure([]), { code: "evaluation-failed" });
});
