import type { Evaluation } from "../evaluation.ts";
import type { PreviewModel } from "../preview/model.ts";

/** Both presentations inspect the worker's one current IR result, including component steps. */
export function outputEvaluation(
  preview: PreviewModel,
  source: string | null,
): Evaluation {
  const state = preview.state;
  const rendered = state?.session ?? state?.structured;
  const problem = state?.problem;
  const diagnostics = [
    ...(state?.diagnostics ?? []),
    ...(problem != null && "diagnostics" in problem ? problem.diagnostics : []),
    ...(state?.drawing?.diagnostics ?? []),
    ...(preview.notice?.diagnostics ?? []),
  ];
  const outcome =
    rendered === null || rendered === undefined
      ? state?.hasRoot === false || problem?.kind === "noComponent"
        ? { kind: "noRoot" as const }
        : problem != null && "diagnostics" in problem
          ? { kind: "error" as const, diagnostics: problem.diagnostics }
          : null
      : "diagnostics" in rendered.output
        ? { kind: "error" as const, diagnostics: rendered.output.diagnostics }
        : { kind: "value" as const, ...rendered.output };
  const evaluating =
    source !== null &&
    ((state === null && preview.failure === null) ||
      preview.asked !== source ||
      preview.pending !== null);
  return {
    outcome,
    outcomeSource: state?.source ?? null,
    stale:
      (state !== null &&
        (state.diagnosticsSource ?? state.source) !== state.source) ||
      preview.notice?.kind === "failed" ||
      preview.failure !== null,
    diagnostics: diagnostics.filter(
      (item, index) =>
        diagnostics.findIndex(
          (other) =>
            other.code === item.code &&
            other.message === item.message &&
            JSON.stringify(other.span) === JSON.stringify(item.span),
        ) === index,
    ),
    diagnosticsSource: state?.diagnosticsSource ?? state?.source ?? null,
    failure: preview.failure,
    evaluating,
  };
}
