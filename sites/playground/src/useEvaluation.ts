import { useEffect, useState } from "react";
import type { Evaluate } from "./compile";
import { initialEvaluation, reduceEvaluation, type Evaluation, type EvaluationEvent } from "./evaluation.ts";

export type { Evaluation } from "./evaluation.ts";

/**
 * Evaluates `source` once typing pauses, rather than per keystroke: each evaluation crosses to the
 * worker, and an output that re-renders mid-word flickers through half-written states. Nothing is
 * evaluated while `source` is null, which it is until the address has been read. How each answer
 * changes the state is `reduceEvaluation`'s, in `evaluation.ts`.
 */
export function useEvaluation(source: string | null, evaluate: Evaluate, debounceMs = 350): Evaluation {
  const [evaluation, setEvaluation] = useState<Evaluation>(initialEvaluation);

  useEffect(() => {
    if (source === null) {
      return;
    }
    let cancelled = false;
    const dispatch = (event: EvaluationEvent) => {
      if (!cancelled) {
        setEvaluation((previous) => reduceEvaluation(previous, event));
      }
    };
    dispatch({ kind: "started" });

    const timer = setTimeout(() => {
      evaluate(source).then(
        (result) => dispatch({ kind: "answered", source, result }),
        (error: unknown) => dispatch({ kind: "failed", message: error instanceof Error ? error.message : String(error) }),
      );
    }, debounceMs);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source, evaluate, debounceMs]);

  return evaluation;
}
