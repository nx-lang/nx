import { nxWorkerChannel } from "../worker/index.ts";

import type { Compile, CompileResult } from "./types.ts";

/**
 * The faults a fresh worker can answer: the channel has already discarded the one that failed.
 *
 * A module that would not load is not among them — the browser refused it or the network did, and
 * asking again in the same breath would fail the same way.
 */
const recoverable = new Set(["TimeoutError", "WorkerStoppedError", "NxHostCrashedError"]);

/**
 * Compiles in the browser, in the app's compiler worker.
 *
 * <para>Everything a failure here can be is an application fault: a module that would not load, a
 * compiler that crashed, or one that overran its deadline. An authoring error is not a failure —
 * it comes back in `diagnostics`. So a rejection is passed through with a readable message and the
 * editor view reports it as a fault.</para>
 *
 * <para>A recoverable fault is retried once, against the replacement worker the channel starts.
 * The editor would recover on its own at the visitor's next keystroke, but a gallery preview
 * compiles once and never again, and would otherwise carry the failure until the page is reloaded.
 * Once only: a crash the source itself causes is reproducible, and a second failure is the
 * visitor's to read.</para>
 */
export const compileInBrowser: Compile = retrying(
  (source) => nxWorkerChannel().send({ kind: "compile", source }) as Promise<CompileResult>
);

/**
 * Wraps one attempt in the retry, over whatever makes the attempt.
 *
 * Exported so a test can drive the retry without a worker; the site has one implementation.
 */
export function retrying(attempt: (source: string) => Promise<CompileResult>): Compile {
  return async (source: string): Promise<CompileResult> => {
    try {
      return await attempt(source);
    } catch (error) {
      if (!(error instanceof Error) || !recoverable.has(error.name)) {
        throw new Error(compileFailureMessage(error));
      }
      try {
        return await attempt(source);
      } catch (retried) {
        throw new Error(compileFailureMessage(retried));
      }
    }
  };
}

/**
 * What the diagnostics pane says when the compiler itself failed.
 *
 * <para>Each fault is named by the layer that produced it — the SDK for a trap, the channel for a
 * deadline, the worker for a module that would not load — so this classifies rather than matches
 * text. Every sentence says what happened and whether the visitor has to do anything: a crash and a
 * timeout are both recovered from by the next compile, and only a module that will not load needs a
 * reload.</para>
 */
export function compileFailureMessage(error: unknown): string {
  if (!(error instanceof Error)) {
    return `The compiler failed: ${String(error)}`;
  }

  switch (error.name) {
    case "NxHostCrashedError":
      return "The compiler crashed on this source. It has been restarted — keep editing and the next compile will run.";
    case "TimeoutError":
      return `${error.message} It has been restarted — keep editing and the next compile will run.`;
    case "WorkerStoppedError":
      return `${error.message} It will be started again on the next compile.`;
    case "NxModuleLoadError":
      return `${error.message} Reload the page to try again.`;
    case "AbortError":
      return "The compile was cancelled.";
    default:
      return `The compiler failed: ${error.message}`;
  }
}
