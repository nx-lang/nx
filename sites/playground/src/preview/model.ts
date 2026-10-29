/**
 * The page's side of the preview: what it last heard from the worker, and which call to make next.
 *
 * <para>Kept apart from React so the transitions can be tested under Node. The worker holds the
 * session; the page holds only the last answer, the reload notice that answer brought, and the
 * source it last asked the worker to run, so it can tell when a newer build needs a reload.</para>
 */
import type {
  PreviewCall,
  PreviewReloadNotice,
  PreviewState,
} from "./protocol.ts";

export interface PreviewModel {
  /** Responses from a document opened before this epoch cannot replace its state. */
  readonly epoch: number;
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
  epoch: 0,
  state: null,
  notice: null,
  asked: null,
  picked: undefined,
  pending: null,
  failure: null,
};

export type PreviewEvent =
  | { readonly kind: "asked"; readonly call: PreviewCall }
  | {
      readonly kind: "answered";
      readonly epoch?: number;
      readonly call: PreviewCall;
      readonly state: PreviewState;
    }
  | {
      readonly kind: "failed";
      readonly message: string;
      readonly epoch?: number;
    };

export function reducePreview(
  model: PreviewModel,
  event: PreviewEvent,
): PreviewModel {
  if (
    event.kind !== "asked" &&
    event.epoch !== undefined &&
    event.epoch !== model.epoch
  ) {
    return model;
  }
  switch (event.kind) {
    case "asked": {
      const { call } = event;
      return {
        ...model,
        pending: call,
        epoch:
          call.op === "start" && call.reset ? model.epoch + 1 : model.epoch,
        ...(call.op === "start" && call.reset
          ? { state: null, notice: null, failure: null }
          : {}),
        ...(call.op === "start" || call.op === "reload"
          ? { asked: call.source }
          : {}),
        ...(call.op === "start" ? { picked: call.component } : {}),
      };
    }
    case "answered": {
      const { call, state } = event;
      // A newer document request supersedes answers queued for an older source.
      if (
        (call.op === "start" || call.op === "reload") &&
        model.asked !== null &&
        call.source !== model.asked
      ) {
        return model;
      }
      const ran = call.op === "start" || call.op === "reload";
      return {
        ...model,
        state,
        notice: ran ? state.notice : model.notice,
        pending: null,
        failure: null,
      };
    }
    case "failed":
      // Keep the last answer for stale Output and Preview. The worker may have replaced only its
      // compiler host and retained its IR session. A later edit attempts reload before restart.
      return {
        ...model,
        pending: null,
        failure: event.message,
      };
  }
}

/**
 * The call that brings the worker's preview up to `source`, the current editor source, or
 * null when it already runs it or the preview is not shown.
 */
export function callFor(
  model: PreviewModel,
  active: boolean,
  source: string | null,
): PreviewCall | null {
  if (!active || source === null || source === model.asked) {
    return null;
  }
  return model.state === null
    ? {
        op: "start",
        source: source,
        ...(model.picked === undefined ? {} : { component: model.picked }),
      }
    : {
        op: "reload",
        source: source,
        ...(model.picked === undefined ? {} : { component: model.picked }),
      };
}

/**
 * Whether an answer says the worker no longer holds the session the page showed: the worker was
 * replaced since, so a step or a move along the timeline found nothing to act on.
 */
export function sessionLost(
  call: PreviewCall,
  before: PreviewState | null,
  after: PreviewState,
): boolean {
  return (
    (call.op === "dispatch" || call.op === "goTo") &&
    before?.session != null &&
    after.session === null &&
    after.problem === null
  );
}
