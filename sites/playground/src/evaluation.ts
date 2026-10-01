/**
 * The output pane's state, and how each evaluation's answer changes it.
 *
 * <para>Kept apart from React so the transitions can be tested under Node: a first source that does
 * not compile, a broken edit after a good one, a compiler that failed.</para>
 */
import type { Diagnostic, EvaluateResult, Outcome } from "./compile";

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

export const initialEvaluation: Evaluation = {
  outcome: null,
  outcomeSource: null,
  stale: false,
  diagnostics: [],
  diagnosticsSource: null,
  failure: null,
  evaluating: true,
};

/** What can happen to an evaluation. */
export type EvaluationEvent =
  /** The source changed, so a new evaluation is on its way. */
  | { readonly kind: "started" }
  /** The worker answered for `source`. */
  | { readonly kind: "answered"; readonly source: string; readonly result: EvaluateResult }
  /** The compiler failed, with a message for the visitor. */
  | { readonly kind: "failed"; readonly message: string };

export function reduceEvaluation(state: Evaluation, event: EvaluationEvent): Evaluation {
  switch (event.kind) {
    case "started":
      return { ...state, evaluating: true };
    case "failed":
      return {
        ...state,
        stale: state.outcome !== null,
        diagnostics: [],
        diagnosticsSource: null,
        failure: event.message,
        evaluating: false,
      };
    case "answered": {
      const { result, source } = event;
      if (result.outcome === null) {
        // It does not compile: the last output stands, marked out of date.
        return {
          ...state,
          stale: state.outcome !== null,
          diagnostics: result.diagnostics,
          diagnosticsSource: source,
          failure: null,
          evaluating: false,
        };
      }
      return {
        outcome: result.outcome,
        outcomeSource: source,
        stale: false,
        diagnostics:
          result.outcome.kind === "error" ? [...result.diagnostics, ...result.outcome.diagnostics] : result.diagnostics,
        diagnosticsSource: source,
        failure: null,
        evaluating: false,
      };
    }
  }
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
