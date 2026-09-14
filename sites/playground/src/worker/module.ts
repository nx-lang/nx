/**
 * Fetching and compiling the NX WebAssembly module, with the one bound that belongs to a download.
 *
 * <para>A request's deadline measures the compiler's work and deliberately does not cover this
 * wait: the module is two megabytes, and on a slow connection it takes many times any healthy
 * request, so terminating the worker mid-download would discard the partial fetch and start the
 * same slow one over. What a slow load still needs is an end. A connection that accepts and then
 * stalls settles nothing — `compileStreaming` neither resolves nor rejects, the worker stays
 * healthy, and without a bound here the editor waits forever with nothing on screen. So the load
 * carries its own budget, far above any real download, and reports a module that never arrived the
 * same way as one that would not load: a fault the visitor can read, answered by a reload.</para>
 */

/**
 * How long the module may take to arrive before the load is treated as stalled rather than slow.
 *
 * Well above the 73.5 s a throttled measurement took against an origin serving it uncompressed and
 * uncached — that load has to succeed — and far below waiting forever.
 */
export const DEFAULT_LOAD_DEADLINE_MS = 120_000;

/** What `loadNxModule` accepts. Everything but the URL exists so a test can drive the wait. */
export interface LoadNxModuleOptions {
  /** How long the module may take to arrive. Default two minutes. */
  readonly deadlineMs?: number;
  /** Fetches the module. Must honour `signal`, as the platform's own `fetch` does. */
  readonly fetch?: (url: string, init: { signal: AbortSignal }) => Promise<Response>;
  /** Compiles the response as it streams. */
  readonly compileStreaming?: (response: Promise<Response>) => Promise<WebAssembly.Module>;
}

/**
 * Fetches `url` and compiles it as it streams.
 *
 * @throws an error named `NxModuleLoadError` when the module cannot be fetched, when the browser
 * refuses it, or when it has not arrived within the load budget. A network failure and a module
 * the browser refuses look the same to the caller and need the same answer — reload — so they are
 * reported as one named failure rather than as whatever fetch or WebAssembly happened to throw.
 */
export async function loadNxModule(
  url: string,
  options: LoadNxModuleOptions = {}
): Promise<WebAssembly.Module> {
  const deadlineMs = options.deadlineMs ?? DEFAULT_LOAD_DEADLINE_MS;
  const fetchModule = options.fetch ?? ((target, init) => fetch(target, init));
  const compile = options.compileStreaming ?? ((response) => WebAssembly.compileStreaming(response));

  const controller = new AbortController();
  // Aborting is what ends a stalled download: it rejects the fetch, or errors the body stream the
  // compile is still reading, and frees the connection either way.
  const stalled = setTimeout(() => controller.abort(), deadlineMs);

  try {
    return await compile(fetchModule(url, { signal: controller.signal }));
  } catch (error) {
    throw controller.signal.aborted
      ? loadError(`the download did not finish within ${Math.round(deadlineMs / 1000)}s`)
      : loadError(error instanceof Error ? error.message : String(error));
  } finally {
    clearTimeout(stalled);
  }
}

/** The one failure this module reports, named so the diagnostics pane can word it. */
function loadError(reason: string): Error {
  const failure = new Error(`The compiler could not be loaded: ${reason}.`);
  failure.name = "NxModuleLoadError";
  return failure;
}
