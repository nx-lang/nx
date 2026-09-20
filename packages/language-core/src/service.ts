/**
 * An in-process implementation of the protocol's `NxLanguageService`.
 *
 * <para>It is the same answering the HTTP handler does, without a request or a response: the caller
 * hands it a way to build a snapshot and it answers queries over a bounded cache of analyses. The
 * playground's worker uses it with the wasm SDK's snapshots; a Node service could use it with the
 * Node SDK's.</para>
 */
import type {
  CompletionList,
  CompletionsRequest,
  DiagnosticReport,
  DiagnosticsRequest,
  DocumentSymbol,
  DocumentSymbolsRequest,
  Hover,
  HoverRequest,
  LanguageDocument,
  LanguageQueryName,
  NxLanguageService
} from "@nx-lang/language-protocol";

import {
  answerQuery,
  contextCollision,
  contextCollisionMessage,
  type AnswerRequest,
  type HostContext
} from "./answer.js";
import { SnapshotCache, documentSetKey, type SnapshotLike } from "./cache.js";

/** How many analyzed document sets a service keeps by default. */
export const DEFAULT_CACHE_SIZE = 8;

/** What `createSnapshotLanguageService` accepts. */
export interface SnapshotLanguageServiceOptions {
  /**
   * Analyzes a document set. Called once per distinct set, and its result is cached and disposed.
   *
   * <para>The identities of `context.implicitImports` are passed as `implicitImports`, so the factory
   * hands them to whichever SDK's snapshot it builds.</para>
   */
  createSnapshot: (
    documents: readonly LanguageDocument[],
    options: { implicitImports: readonly string[] }
  ) => SnapshotLike;
  /**
   * The host's own declarations, served with every query: documents that join every query's set, and
   * the identities every queried document imports implicitly.
   *
   * <para>Nothing is prepended and no position is rewritten, so every position in a query and every
   * range in an answer is in the queried document's own coordinates.</para>
   */
  context?: HostContext;
  /** How many analyzed document sets to keep. Default 8. */
  cacheSize?: number;
}

/**
 * A service backed by snapshots, with the caller's own snapshot factory.
 *
 * Alongside the protocol's four queries it exposes `dispose`, which releases every cached
 * analysis — a worker that is being replaced should call it.
 */
export interface SnapshotLanguageService extends NxLanguageService {
  /** Disposes every cached snapshot. The service answers again, analyzing afresh, after this. */
  dispose(): void;
}

/** Creates an in-process language service over `options.createSnapshot`. */
export function createSnapshotLanguageService(
  options: SnapshotLanguageServiceOptions
): SnapshotLanguageService {
  if ("prelude" in options) {
    throw new TypeError(
      "createSnapshotLanguageService no longer accepts `prelude`. Pass `context: { documents, implicitImports }` instead: the host's declarations join every query's document set and are named as implicit imports, and no position is shifted."
    );
  }
  const cache = new SnapshotCache<SnapshotLike>(options.cacheSize ?? DEFAULT_CACHE_SIZE);
  const context = options.context;
  const implicitImports = Array.from(context?.implicitImports ?? []);

  function answer<T>(
    query: LanguageQueryName,
    request: AnswerRequest,
    signal: AbortSignal | undefined
  ): Promise<T> {
    // Analysis and the answer are produced synchronously on this thread, so a cancellation can
    // only be observed before the work starts: nothing yields in between for the signal to fire.
    // A caller that cancels while an answer is in flight is served by whoever runs the service off
    // the caller's thread — the playground's worker channel drops the late answer.
    if (signal?.aborted === true) {
      return Promise.reject(abortError());
    }

    const collision = contextCollision(request.documents, context);
    if (collision !== undefined) {
      return Promise.reject(new TypeError(contextCollisionMessage(collision, "query")));
    }

    try {
      return Promise.resolve(
        answerQuery(query, request, context, (documents) =>
          cache.getOrCreate(documentSetKey(documents), () =>
            options.createSnapshot(documents, { implicitImports })
          )
        ) as T
      );
    } catch (error) {
      return Promise.reject(error);
    }
  }

  return {
    hover(request: HoverRequest, signal?: AbortSignal): Promise<Hover | null> {
      return answer("hover", toAnswerRequest(request, request.position), signal);
    },
    completions(request: CompletionsRequest, signal?: AbortSignal): Promise<CompletionList> {
      return answer("completions", toAnswerRequest(request, request.position), signal);
    },
    diagnostics(request: DiagnosticsRequest, signal?: AbortSignal): Promise<DiagnosticReport> {
      return answer("diagnostics", toAnswerRequest(request, undefined), signal);
    },
    documentSymbols(request: DocumentSymbolsRequest, signal?: AbortSignal): Promise<DocumentSymbol[]> {
      return answer("documentSymbols", toAnswerRequest(request, undefined), signal);
    },
    dispose(): void {
      cache.clear();
    }
  };
}

function toAnswerRequest(
  request: { documents: LanguageDocument[]; uri: string },
  position: AnswerRequest["position"]
): AnswerRequest {
  return { documents: request.documents, uri: request.uri, position };
}

/**
 * The rejection a cancelled query produces, recognized by the protocol's `isAbortError`.
 */
function abortError(): Error {
  const error = new Error("The language query was cancelled.");
  error.name = "AbortError";
  return error;
}
