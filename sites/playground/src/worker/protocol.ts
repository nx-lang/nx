/**
 * What the main thread and the compiler worker say to each other.
 *
 * <para>One channel carries both compilation and language queries, because both are answered by
 * the same host inside the worker and a second worker would mean a second copy of the module and
 * a second analysis of the same text.</para>
 *
 * <para>There is no cancel message. Messages are delivered in order and the worker answers them one
 * at a time, so a cancel posted after the request it cancels arrives after that request has already
 * been answered — it could never save the work. Cancellation is therefore the main thread's: the
 * caller's promise settles at once and the answer, when it comes, is dropped.</para>
 */
import type { LanguageQueryName } from "@nx-lang/language-protocol";

/** A request from the main thread, before the channel gives it its id. */
export type WorkerCall =
  | { readonly kind: "compile"; readonly source: string }
  | { readonly kind: "language"; readonly query: LanguageQueryName; readonly request: unknown };

/** A request from the main thread, as the worker receives it. */
export type WorkerRequest = WorkerCall & { readonly id: number };

/** An error as it crosses the channel: structured clone does not preserve error classes. */
export interface WorkerError {
  readonly name: string;
  readonly message: string;
}

/** An answer from the worker. Exactly one arrives per request. */
export type WorkerResponse =
  | { readonly kind: "ok"; readonly id: number; readonly value: unknown }
  | { readonly kind: "error"; readonly id: number; readonly error: WorkerError };

/**
 * The worker's one unsolicited message: its module is compiled and it can start working.
 *
 * <para>It exists so the main thread can tell waiting for the compiler from waiting for the
 * download. The module is two megabytes and arrives over the visitor's connection, which on a slow
 * one takes far longer than any healthy request; a request's deadline starts when this arrives, so
 * a slow load is waited for rather than cut off and started over.</para>
 */
export interface WorkerReady {
  readonly kind: "ready";
}

/** Everything the worker posts: an answer to a request, or its one readiness notice. */
export type WorkerMessage = WorkerResponse | WorkerReady;

/** Describes an error for the channel, keeping the class name the caller checks. */
export function toWorkerError(error: unknown): WorkerError {
  return error instanceof Error
    ? { name: error.name, message: error.message }
    : { name: "Error", message: String(error) };
}

/** Rebuilds an error from what crossed the channel, so `error.name` still identifies it. */
export function fromWorkerError(error: WorkerError): Error {
  const rebuilt = new Error(error.message);
  rebuilt.name = error.name;
  return rebuilt;
}
