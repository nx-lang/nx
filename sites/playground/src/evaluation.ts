/**
 * The output pane's state, and how each evaluation's answer changes it.
 *
 * <para>Kept apart from React so the transitions can be tested under Node: a first source that does
 * not compile, a broken edit after a good one, a compiler that failed.</para>
 */
import type { Diagnostic, Outcome } from "./compile";

/** The output pane's state, and the diagnostics the editor marks. */
export interface Evaluation {
  /**
   * What to show: the latest outcome, or, while the source does not compile, the last one that
   * did. Null until an evaluation has produced one.
   */
  readonly outcome: Outcome | null;
  /** The source `outcome` came from, so a hover can tell whether its spans still line up. */
  readonly outcomeSource: string | null;
  /** Whether `outcome` is from older source than the editor holds: the edit broke compilation. */
  readonly stale: boolean;
  /** What compiling reported, and a runtime error's own diagnostics, for the editor to mark. */
  readonly diagnostics: readonly Diagnostic[];
  /** The source `diagnostics` were reported against, whose lines and columns their spans count. */
  readonly diagnosticsSource: string | null;
  /** The compiler failed rather than the source: it would not load, crashed or overran. */
  readonly failure: string | null;
  /** An evaluation is waiting for the pause in typing, or on the worker. */
  readonly evaluating: boolean;
}

/** What the output pane says in place of a value, when it has none to show. */
export type OutputNotice =
  /** The first evaluation has not answered yet. */
  | "evaluating"
  /** The source does not compile, and no earlier value stands in for it. */
  | "doesNotCompile"
  /** There is an outcome, or a compiler failure, to show instead. */
  | "none";

export function outputNotice(state: Evaluation): OutputNotice {
  if (state.outcome !== null || state.failure !== null) {
    return "none";
  }
  return state.diagnostics.length > 0 ? "doesNotCompile" : "evaluating";
}

/** Both raw Output and component Preview classify formatting separately from evaluation. */
export function outputErrorHeading(diagnostics: readonly Diagnostic[]): string {
  switch (diagnostics[0]?.code) {
    case "nx-text-unspellable":
      return "The value has no NX spelling";
    case "nx-output-format":
      return "The value could not be displayed";
    default:
      return "Evaluating root failed";
  }
}
