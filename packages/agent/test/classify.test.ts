import assert from "node:assert/strict";
import { test } from "node:test";

import type { NxIrDiagnostic } from "@nx-lang/ir-runtime";

import { classifyRuntimeFailure } from "../src/classify.js";

/** A diagnostic with `code`, naming `argument` when one is given. */
function diagnostic(code: string, argument?: string, limit?: NxIrDiagnostic["limit"]): NxIrDiagnostic {
  return {
    severity: "error",
    code,
    message: code,
    ...(argument === undefined ? {} : { argument }),
    ...(limit === undefined ? {} : { limit }),
  };
}

/** The tool's function takes `context` and `session` from the host; the model sends the rest. */
const contextParameters = new Set(["context", "session"]);

function classify(...diagnostics: NxIrDiagnostic[]): ReturnType<typeof classifyRuntimeFailure> {
  return classifyRuntimeFailure(diagnostics, contextParameters);
}

test("a boundary failure in an argument the model sent is invalid input", () => {
  for (const code of ["nx-ir-boundary-type", "nx-ir-boundary-field", "nx-ir-arguments"]) {
    assert.deepEqual(classify(diagnostic(code, "teamSize")), { code: "invalid-input" }, code);
  }
  assert.deepEqual(
    classify(diagnostic("nx-ir-boundary-type", "teamSize"), diagnostic("nx-ir-arguments", "note"), diagnostic("nx-ir-boundary-field", "teamSize")),
    { code: "invalid-input" },
  );
  // With no context parameters every named argument is the model's.
  assert.deepEqual(classifyRuntimeFailure([diagnostic("nx-ir-boundary-type", "context")], new Set()), { code: "invalid-input" });
});

test("a boundary failure in a context parameter is invalid context", () => {
  for (const code of ["nx-ir-boundary-type", "nx-ir-boundary-field", "nx-ir-arguments"]) {
    assert.deepEqual(classify(diagnostic(code, "context")), { code: "invalid-context" }, code);
  }
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-field", "session")), { code: "invalid-context" });
});

test("one failure in the context beside one in the model's input is invalid context", () => {
  // The model cannot make the call succeed while the host's record is wrong.
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-type", "teamSize"), diagnostic("nx-ir-boundary-field", "context")), { code: "invalid-context" });
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-field", "context"), diagnostic("nx-ir-boundary-type", "teamSize")), { code: "invalid-context" });
});

test("a boundary failure that names no argument is an evaluation failure", () => {
  // It was found inside the function: in a default, or in the result.
  for (const code of ["nx-ir-boundary-type", "nx-ir-boundary-field", "nx-ir-arguments", "nx-ir-boundary-constraint"]) {
    assert.deepEqual(classify(diagnostic(code)), { code: "evaluation-failed" }, code);
  }
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-type", "teamSize"), diagnostic("nx-ir-boundary-type")), { code: "evaluation-failed" });
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-type", "context"), diagnostic("nx-ir-boundary-type")), { code: "evaluation-failed" });
});

test("a boundary failure of a kind added later is invalid input when it names an argument", () => {
  // A code this package has no knowledge of: a value outside a constrained type, say.
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-constraint", "teamSize")), { code: "invalid-input" });
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-constraint", "teamSize"), diagnostic("nx-ir-boundary-type", "note")), { code: "invalid-input" });
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-constraint", "context")), { code: "invalid-context" });
});

test("one failure that is not of the boundary makes it an evaluation failure", () => {
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-constraint", "teamSize"), diagnostic("nx-ir-division-by-zero")), { code: "evaluation-failed" });
  assert.deepEqual(classify(diagnostic("nx-ir-boundary-type", "context"), diagnostic("nx-ir-division-by-zero")), { code: "evaluation-failed" });
  assert.deepEqual(classify(diagnostic("nx-ir-division-by-zero")), { code: "evaluation-failed" });
  // A function record that names no function is the runtime's refusal of an argument, and names
  // it, but its code is not of the boundary family: a tool's function takes no function from a model.
  assert.deepEqual(classify(diagnostic("nx-ir-function-value", "step")), { code: "evaluation-failed" });
  assert.deepEqual(classify(diagnostic("nx-ir-function-value", "context")), { code: "evaluation-failed" });
});

test("only the family's prefix counts, not a code that merely resembles it", () => {
  for (const code of ["nx-ir-boundary", "nx-ir-boundaries-type", "x-nx-ir-boundary-type", "nx-ir-argument", "nx-ir-arguments-extra", ""]) {
    assert.deepEqual(classify(diagnostic(code, "teamSize")), { code: "evaluation-failed" }, JSON.stringify(code));
  }
});

test("a limit is a resource limit whatever is beside it, and carries the limit", () => {
  const limit = { name: "maxOperations", value: 100_000 };
  assert.deepEqual(classify(diagnostic("nx-ir-resource-limit", undefined, limit)), { code: "resource-limit", limit });
  for (const beside of [
    diagnostic("nx-ir-boundary-type", "teamSize"),
    diagnostic("nx-ir-boundary-type", "context"),
    diagnostic("nx-ir-boundary-type"),
    diagnostic("nx-ir-division-by-zero"),
  ]) {
    assert.deepEqual(classify(beside, diagnostic("nx-ir-resource-limit", undefined, limit)), { code: "resource-limit", limit });
    assert.deepEqual(classify(diagnostic("nx-ir-resource-limit", undefined, limit), beside), { code: "resource-limit", limit });
  }
  assert.deepEqual(classify(diagnostic("nx-ir-resource-limit")), { code: "resource-limit" });
});

test("a failure with no diagnostics is an evaluation failure", () => {
  assert.deepEqual(classify(), { code: "evaluation-failed" });
});
