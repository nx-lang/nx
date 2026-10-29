/**
 * The faults a fresh worker can answer: the channel has already discarded the one that failed.
 *
 * A module that would not load is not among them — the browser refused it or the network did, and
 * asking again in the same breath would fail the same way.
 */
const recoverable = new Set([
  "TimeoutError",
  "WorkerStoppedError",
  "NxHostCrashedError",
]);

/**
 * Retries recoverable worker failures once, preserving the original error for the caller to
 * classify. Source builds can retry safely; action dispatches must never use this wrapper.
 */
export function retrying<T>(
  attempt: (source: string) => Promise<T>,
): (source: string) => Promise<T> {
  return async (source: string): Promise<T> => {
    try {
      return await attempt(source);
    } catch (error) {
      if (!(error instanceof Error) || !recoverable.has(error.name)) {
        throw error;
      }
      return await attempt(source);
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
    case "NxStackOverflowError":
      return `${error.message} Look for a function that calls itself without stopping, or nests calls less deeply.`;
    case "AbortError":
      return "The compile was cancelled.";
    default:
      return `The compiler failed: ${error.message}`;
  }
}
