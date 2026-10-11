import { useCallback, useEffect, useReducer, useRef } from "react";
import type { NxHostRecord } from "@nx-lang/ir-runtime";

import { compileFailureMessage } from "../compile/worker.ts";
import { nxWorkerChannel } from "../worker/index.ts";
import { callFor, initialPreview, reducePreview, sessionLost, type PreviewModel } from "./model.ts";
import type { PreviewCall, PreviewState } from "./protocol.ts";

/** Sends one call to the worker's preview. */
export type SendPreview = (call: PreviewCall) => Promise<PreviewState>;

const sendInBrowser: SendPreview = (call) => nxWorkerChannel().send({ kind: "preview", call }) as Promise<PreviewState>;

export interface Preview extends PreviewModel {
  /** Starts a new session for `component` on the latest source that compiled. */
  start(component: string): void;
  dispatch(handler: string, action: NxHostRecord): void;
  goTo(tick: number): void;
  /** Starts again after the worker failed. */
  retry(): void;
}

/**
 * The preview the output pane shows: started when the pane switches to Preview, and reloaded with
 * each later source that compiles. `compiled` is the latest source that compiled, which is the build
 * the worker keeps. Calls queue in the worker in the order they are made, so the last answer is
 * always the newest state.
 */
export function usePreview(active: boolean, compiled: string | null, send: SendPreview = sendInBrowser): Preview {
  const [model, dispatchEvent] = useReducer(reducePreview, initialPreview);
  const latest = useRef(model);
  latest.current = model;
  const compiledRef = useRef(compiled);
  compiledRef.current = compiled;

  const call = useCallback(
    (request: PreviewCall) => {
      const before = latest.current.state;
      dispatchEvent({ kind: "asked", call: request });
      send(request).then(
        (state) => {
          dispatchEvent({ kind: "answered", call: request, state });
          const source = compiledRef.current;
          if (sessionLost(request, before, state) && source !== null) {
            call({ op: "start", source, ...(latest.current.picked === undefined ? {} : { component: latest.current.picked }) });
          }
        },
        (error: unknown) => dispatchEvent({ kind: "failed", message: compileFailureMessage(error) })
      );
    },
    [send]
  );

  useEffect(() => {
    const next = callFor(model, active, compiled);
    if (next !== null) {
      call(next);
    }
  }, [model, active, compiled, call]);

  return {
    ...model,
    start: (component) => {
      const source = compiledRef.current;
      if (source !== null) {
        call({ op: "start", source, component });
      }
    },
    dispatch: (handler, action) => call({ op: "dispatch", handler, action }),
    goTo: (tick) => call({ op: "goTo", tick }),
    retry: () => {
      const source = compiledRef.current;
      if (source !== null) {
        call({ op: "start", source, ...(latest.current.picked === undefined ? {} : { component: latest.current.picked }) });
      }
    }
  };
}
