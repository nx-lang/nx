import type { NxIrDiagnostic } from "@nx-lang/ir-runtime";

import type { ToolErrorCode, ToolLimit } from "./diagnostics.js";

/**
 * Whether a runtime diagnostic says that what the host passed in is not what the function's
 * parameters admit: `nx-ir-arguments`, or any code of the boundary family, which begins
 * `nx-ir-boundary-`.
 *
 * <para>The family is matched and not a list of its codes. A check a runtime adds at the boundary
 * later, of a value against a constrained type for one, is then input the model can correct, with
 * no change here, as long as the runtime names the argument as it does for the rest.</para>
 */
function isBoundaryFailure(diagnostic: NxIrDiagnostic): boolean {
  return diagnostic.code === "nx-ir-arguments" || diagnostic.code.startsWith("nx-ir-boundary-");
}

/**
 * The result code of a call the IR runtime failed, from its diagnostics, so a host does not read
 * IR codes. `contextParameters` holds the names of the parameters of the tool's function that the
 * host fills in.
 *
 * <para>`resource-limit` when any diagnostic is a limit, with that limit. When every diagnostic is
 * a boundary failure that names the argument it is in: `invalid-context` if any of those is a
 * context parameter, since the model cannot make the call succeed while the host's record is
 * wrong, and otherwise `invalid-input`, the one failure the model can correct. `evaluation-failed`
 * for anything else, a boundary failure that names no argument included: it was found inside the
 * function, in a default or in the result, and not in what the call was given.</para>
 */
export function classifyRuntimeFailure(
  diagnostics: readonly NxIrDiagnostic[],
  contextParameters: ReadonlySet<string>,
): { readonly code: ToolErrorCode; readonly limit?: ToolLimit } {
  const limited = diagnostics.find((diagnostic) => diagnostic.code === "nx-ir-resource-limit");
  if (limited !== undefined) {
    return { code: "resource-limit", ...(limited.limit === undefined ? {} : { limit: limited.limit }) };
  }
  const named = diagnostics.map((diagnostic) => (isBoundaryFailure(diagnostic) ? diagnostic.argument : undefined));
  if (named.length === 0 || named.includes(undefined)) {
    return { code: "evaluation-failed" };
  }
  return { code: named.some((argument) => contextParameters.has(argument!)) ? "invalid-context" : "invalid-input" };
}
