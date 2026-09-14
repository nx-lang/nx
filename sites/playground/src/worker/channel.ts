/**
 * The main thread's side of the compiler worker: one channel, request ids, cancellation and a
 * deadline.
 *
 * <para>The worker answers one request at a time, so everything the editor asks for shares a
 * queue: a compile, then the hover that follows it. That is what makes a deadline necessary — a
 * compile that never returns would take every later query with it — and what makes cancellation
 * worth having: an abandoned hover cannot be stopped inside the worker, but its answer can be
 * dropped so the caller is not left waiting on it.</para>
 *
 * <para>When a request overruns the deadline the worker is terminated: a wasm instance mid-compile
 * cannot be interrupted any other way. Everything in flight is rejected, and the next request
 * starts a fresh worker, which recompiles the module the browser has already cached.</para>
 *
 * <para>The deadline measures the worker's own work, not the wait for its module. A request sent
 * before the worker is ready waits without a timer, and its deadline starts when the worker says it
 * is ready. Terminating a worker that is still downloading would throw away the partial download
 * and start the same slow fetch again, which on a connection slow enough to need it can never
 * finish.</para>
 */
import {
  fromWorkerError,
  type WorkerCall,
  type WorkerMessage,
  type WorkerRequest,
  type WorkerResponse
} from "./protocol.ts";

/**
 * How long one request may take, once the worker is ready, before it is assumed stuck.
 *
 * A cold compile of the catalog and an example takes tens of milliseconds; this is far above
 * anything healthy, so reaching it means the module is not coming back. The module's own download
 * is outside this budget — see the note above.
 */
export const DEFAULT_DEADLINE_MS = 10_000;

/** Anything the channel can drive: a real `Worker`, or a fake in a test. */
export interface WorkerLike {
  postMessage(message: WorkerRequest): void;
  terminate(): void;
  onmessage: ((event: MessageEvent<WorkerMessage>) => void) | null;
  onerror: ((event: never) => void) | null;
}

/** What `createWorkerChannel` accepts. */
export interface WorkerChannelOptions {
  /** Starts a worker. Called again after a deadline or a worker error, never for a single request. */
  readonly startWorker: () => WorkerLike;
  /** How long one request may take once the worker is ready. Default 10 seconds. */
  readonly deadlineMs?: number;
}

/** The one channel the compile seam and the language service both go through. */
export interface WorkerChannel {
  /**
   * Sends `request` and resolves with the worker's answer.
   *
   * @throws the error the worker reported, an abort error when `signal` is cancelled first, or a
   * timeout error when the deadline passes.
   */
  send(call: WorkerCall, signal?: AbortSignal): Promise<unknown>;

  /** Starts the worker without asking it anything, so the module is loading before it is needed. */
  start(): void;

  /** Terminates the worker and rejects everything in flight. */
  dispose(): void;
}

interface Pending {
  readonly settle: (response: WorkerResponse) => void;
  readonly fail: (error: Error) => void;
  /** Starts this request's deadline: when it is sent to a ready worker, or when that worker is. */
  readonly arm: () => void;
  /** Stops the deadline, armed or not. */
  readonly disarm: () => void;
}

/** Creates the channel. The worker starts on `start()`, or on the first request. */
export function createWorkerChannel(options: WorkerChannelOptions): WorkerChannel {
  const deadlineMs = options.deadlineMs ?? DEFAULT_DEADLINE_MS;
  const pending = new Map<number, Pending>();
  let worker: WorkerLike | null = null;
  let ready = false;
  let nextId = 1;
  let disposed = false;

  function ensureWorker(): WorkerLike {
    if (worker !== null) {
      return worker;
    }

    ready = false;
    const started = options.startWorker();
    started.onmessage = (event) => {
      const message = event.data;
      if (message.kind === "ready") {
        markReady();
        return;
      }
      // An answer with no pending entry is the late answer to a cancelled request. Dropping it is
      // the whole of cancellation on this side.
      pending.get(message.id)?.settle(message);
    };
    started.onerror = () => {
      replaceWorker(named("WorkerStoppedError", "The compiler worker stopped unexpectedly."));
    };
    worker = started;
    return started;
  }

  /** Terminates the current worker and fails everything in flight with `reason`. */
  function replaceWorker(reason: Error): void {
    worker?.terminate();
    worker = null;
    ready = false;
    for (const entry of [...pending.values()]) {
      entry.fail(reason);
    }
    pending.clear();
  }

  /**
   * The worker's module is compiled, so everything waiting on it is now waiting on the compiler.
   *
   * Requests sent while it was loading were held without a timer; each starts its deadline here, so
   * all of them get the same budget for the work itself however long the module took to arrive.
   */
  function markReady(): void {
    if (ready) {
      return;
    }
    ready = true;
    for (const entry of pending.values()) {
      entry.arm();
    }
  }

  return {
    start(): void {
      if (!disposed) {
        ensureWorker();
      }
    },

    send(call: WorkerCall, signal?: AbortSignal): Promise<unknown> {
      if (disposed) {
        return Promise.reject(new Error("The compiler worker channel has been disposed."));
      }
      if (signal?.aborted === true) {
        return Promise.reject(abortError());
      }

      const id = nextId;
      nextId += 1;

      return new Promise<unknown>((resolve, reject) => {
        let timer: ReturnType<typeof setTimeout> | null = null;

        const finish = () => {
          entry.disarm();
          pending.delete(id);
          signal?.removeEventListener("abort", onAbort);
        };

        const onAbort = () => {
          finish();
          reject(abortError());
        };

        const entry: Pending = {
          settle: (response) => {
            finish();
            if (response.kind === "ok") {
              resolve(response.value);
            } else {
              reject(fromWorkerError(response.error));
            }
          },
          fail: (error) => {
            entry.disarm();
            signal?.removeEventListener("abort", onAbort);
            reject(error);
          },
          // A request past its deadline has taken the worker with it, so the worker goes and
          // everything queued behind it fails too; the next request starts a fresh one.
          arm: () => {
            timer ??= setTimeout(() => {
              replaceWorker(
                named(
                  "TimeoutError",
                  `The compiler did not answer within ${Math.round(deadlineMs / 1000)}s.`
                )
              );
            }, deadlineMs);
          },
          disarm: () => {
            if (timer !== null) {
              clearTimeout(timer);
              timer = null;
            }
          }
        };

        pending.set(id, entry);
        signal?.addEventListener("abort", onAbort, { once: true });
        try {
          ensureWorker().postMessage({ ...call, id });
        } catch (error) {
          // Starting a worker can fail outright — a content policy that forbids one, say. The entry
          // and its timer would otherwise outlive the rejection and take the next request's worker
          // with them when the timer fired.
          finish();
          throw error;
        }
        if (ready) {
          entry.arm();
        }
      });
    },

    dispose(): void {
      disposed = true;
      replaceWorker(new Error("The compiler worker channel has been disposed."));
    }
  };
}

function abortError(): Error {
  return named("AbortError", "The request was cancelled.");
}

/**
 * An error the caller classifies by `name` rather than by matching its text.
 *
 * The names survive the trip from the worker, because the channel rebuilds errors from `name` and
 * `message`; the diagnostics pane turns each into a sentence a visitor can act on.
 */
function named(name: string, message: string): Error {
  const error = new Error(message);
  error.name = name;
  return error;
}
