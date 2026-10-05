import type { NxIrDiagnostic } from "@nx-lang/ir-runtime";

import type { ToolErrorCode, ToolLimit } from "./diagnostics.js";

/**
 * Whether a runtime diagnostic says that what the host passed in is not what the function's
 * parameters admit: `nx-ir-arguments`, or any code of the boundary family, which begins
 * `nx-ir-boundary-`.
 *
 * <para>The family is matched and not a list of its codes. A check a runtime adds at the boundary
 * later, of a value against a constrained type for one, is then input the model can correct, with
 * no change here.</para>
 */
function isBoundaryFailure(diagnostic: NxIrDiagnostic): boolean {
  return diagnostic.code === "nx-ir-arguments" || diagnostic.code.startsWith("nx-ir-boundary-");
}

/**
 * The result code of a call the IR runtime failed, from its diagnostics, so a host does not read
 * IR codes: `resource-limit` when any of them is a limit, with that limit; `invalid-input` when
 * every one is a boundary failure; and `evaluation-failed` otherwise.
 */
export function classifyRuntimeFailure(diagnostics: readonly NxIrDiagnostic[]): { readonly code: ToolErrorCode; readonly limit?: ToolLimit } {
  const limited = diagnostics.find((diagnostic) => diagnostic.code === "nx-ir-resource-limit");
  if (limited !== undefined) {
    return { code: "resource-limit", ...(limited.limit === undefined ? {} : { limit: limited.limit }) };
  }
  return { code: diagnostics.length > 0 && diagnostics.every(isBoundaryFailure) ? "invalid-input" : "evaluation-failed" };
}
