import { useCallback, useEffect, useReducer, useRef } from "react";
import type { NxHostRecord } from "@nx-lang/ir-runtime";

import { compileFailureMessage, retrying } from "../compile/worker.ts";
import { nxWorkerChannel } from "../worker/index.ts";
import {
  callFor,
  initialPreview,
  reducePreview,
  sessionLost,
  type PreviewModel,
} from "./model.ts";
import type { PreviewCall, PreviewState } from "./protocol.ts";

/** Sends one call to the worker's preview. */
export type SendPreview = (call: PreviewCall) => Promise<PreviewState>;

const sendInBrowser: SendPreview = (call) => {
  const send = () =>
    nxWorkerChannel().send({
      kind: "preview",
      call,
      scale: globalThis.devicePixelRatio || 1,
    }) as Promise<PreviewState>;
  // Only source builds retry: dispatching an action twice could change state twice.
  return call.op === "start" || call.op === "reload"
    ? retrying(send)(call.source)
    : send();
};

export interface Preview extends PreviewModel {
  /** Starts a new session for `component` in the current editor source. */
  start(component: string): void;
  /** Opens a different document with fresh component state and no previous component choice. */
  reset(source: string): void;
  dispatch(handler: string, action: NxHostRecord): void;
  goTo(tick: number): void;
  /** Starts again after the worker failed. */
  retry(): void;
}

/**
 * The shared worker output: debounces editor changes, compiles/evaluates them through IR, and
 * retains component state across edits. Both Output and registered previews consume its answer.
 * Opening a different document explicitly resets the previous run, even if the new source is broken.
 */
export function usePreview(
  active: boolean,
  source: string | null,
  send: SendPreview = sendInBrowser,
): Preview {
  const [model, dispatchEvent] = useReducer(reducePreview, initialPreview);
  const epoch = useRef(0);
  const latest = useRef(model);
  latest.current = model;
  const sourceRef = useRef(source);
  sourceRef.current = source;

  const call = useCallback(
    (request: PreviewCall) => {
      const requestEpoch =
        request.op === "start" && request.reset
          ? ++epoch.current
          : epoch.current;
      const before = latest.current.state;
      dispatchEvent({ kind: "asked", call: request });
      send(request).then(
        (state) => {
          if (requestEpoch !== epoch.current) {
            return;
          }
          dispatchEvent({
            kind: "answered",
            epoch: requestEpoch,
            call: request,
            state,
          });
          const source = sourceRef.current;
          if (sessionLost(request, before, state) && source !== null) {
            call({
              op: "start",
              source,
              ...(latest.current.picked === undefined
                ? {}
                : { component: latest.current.picked }),
            });
          }
        },
        (error: unknown) =>
          dispatchEvent({
            kind: "failed",
            epoch: requestEpoch,
            message: compileFailureMessage(error),
          }),
      );
    },
    [send],
  );

  useEffect(() => {
    const next = callFor(model, active, source);
    if (next !== null) {
      const timer = setTimeout(() => call(next), 250);
      return () => clearTimeout(timer);
    }
  }, [model, active, source, call]);

  return {
    ...model,
    reset: (source) => call({ op: "start", source, reset: true }),
    start: (component) => {
      const source = sourceRef.current;
      if (source !== null) {
        call({ op: "start", source, component });
      }
    },
    dispatch: (handler, action) => call({ op: "dispatch", handler, action }),
    goTo: (tick) => call({ op: "goTo", tick }),
    retry: () => {
      const source = sourceRef.current;
      if (source !== null) {
        call({
          op: "start",
          source,
          ...(latest.current.picked === undefined
            ? {}
            : { component: latest.current.picked }),
        });
      }
    },
  };
}
