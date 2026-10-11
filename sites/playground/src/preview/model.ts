/**
 * The page's side of the preview: what it last heard from the worker, and which call to make next.
 *
 * <para>Kept apart from React so the transitions can be tested under Node. The worker holds the
 * session; the page holds only the last answer, the reload notice that answer brought, and the
 * source it last asked the worker to run, so it can tell when a newer build needs a reload.</para>
 */
import type { PreviewCall, PreviewReloadNotice, PreviewState } from "./protocol.ts";

export interface PreviewModel {
  /** The worker's last answer, or null before the first. */
  readonly state: PreviewState | null;
  /** What the last start or reload did, kept until the next one. */
  readonly notice: PreviewReloadNotice | null;
  /** The source the page last asked the worker to start or reload with. */
  readonly asked: string | null;
  /** The component the visitor picked, carried into every start. */
  readonly picked: string | undefined;
  /** The call the page waits on. */
  readonly pending: PreviewCall | null;
  /** The worker failed rather than the program: it crashed, overran or would not load. */
  readonly failure: string | null;
}

export const initialPreview: PreviewModel = {
  state: null,
  notice: null,
  asked: null,
  picked: undefined,
  pending: null,
  failure: null
};

export type PreviewEvent =
  | { readonly kind: "asked"; readonly call: PreviewCall }
  | { readonly kind: "answered"; readonly call: PreviewCall; readonly state: PreviewState }
  | { readonly kind: "failed"; readonly message: string };

export function reducePreview(model: PreviewModel, event: PreviewEvent): PreviewModel {
  switch (event.kind) {
    case "asked": {
      const { call } = event;
      return {
        ...model,
        pending: call,
        ...(call.op === "start" || call.op === "reload" ? { asked: call.source } : {}),
        ...(call.op === "start" && call.component !== undefined ? { picked: call.component } : {})
      };
    }
    case "answered": {
      const { call, state } = event;
      const ran = call.op === "start" || call.op === "reload";
      return {
        ...model,
        state,
        notice: ran ? state.notice : model.notice,
        pending: null,
        failure: null
      };
    }
    case "failed":
      // The worker went with its session. The next source that compiles starts one again; the same
      // source is not asked for twice, so a worker that keeps failing is not called in a loop.
      return { ...model, state: null, notice: null, pending: null, failure: event.message };
  }
}

/**
 * The call that brings the worker's preview up to `compiled`, the latest source that compiled, or
 * null when it already runs it or the preview is not shown.
 */
export function callFor(model: PreviewModel, active: boolean, compiled: string | null): PreviewCall | null {
  if (!active || compiled === null || compiled === model.asked) {
    return null;
  }
  return model.state?.session == null
    ? { op: "start", source: compiled, ...(model.picked === undefined ? {} : { component: model.picked }) }
    : { op: "reload", source: compiled };
}

/**
 * Whether an answer says the worker no longer holds the session the page showed: the worker was
 * replaced since, so a step or a move along the timeline found nothing to act on.
 */
export function sessionLost(call: PreviewCall, before: PreviewState | null, after: PreviewState): boolean {
  return (call.op === "dispatch" || call.op === "goTo") && before?.session != null && after.session === null && after.problem === null;
}
