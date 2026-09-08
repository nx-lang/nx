/**
 * The HTTP implementation of `NxLanguageService`: JSON `POST`s to a base URL, one per query, with
 * host-supplied headers, a per-request timeout, and cancellation.
 */
import {
  LanguageServiceHttpError,
  LanguageServiceTimeoutError,
  UnsupportedQueryError,
  isLanguageServiceErrorBody,
  type CompletionList,
  type CompletionsRequest,
  type DiagnosticReport,
  type DiagnosticsRequest,
  type DocumentSymbol,
  type DocumentSymbolsRequest,
  type Hover,
  type HoverRequest,
  type LanguageQueryAnswers,
  type LanguageQueryName,
  type LanguageQueryRequests,
  type NxLanguageService,
} from "@nx-lang/language-protocol";

export {
  LanguageServiceHttpError,
  LanguageServiceTimeoutError,
  UnsupportedQueryError,
  isAbortError,
  type NxLanguageService,
} from "@nx-lang/language-protocol";

/** What `createHttpLanguageService` accepts. */
export interface HttpLanguageServiceOptions {
  /** Where the handler is mounted, such as `/api/language` or `https://api.example.com/nx/language`. */
  baseUrl: string;
  /** The `fetch` to use. Defaults to the global one. */
  fetch?: typeof fetch;
  /**
   * Headers to send with every request, or a function producing them per request — the place to
   * attach an authorization token. May be async.
   */
  headers?: HeadersInit | ((query: LanguageQueryName) => HeadersInit | Promise<HeadersInit>);
  /** How long to wait for an answer before aborting the request. Default 10 000 ms. */
  timeoutMs?: number;
}

const DEFAULT_TIMEOUT_MS = 10_000;
const BODY_EXCERPT_LENGTH = 200;

/** Creates an `NxLanguageService` that speaks the protocol over HTTP. */
export function createHttpLanguageService(options: HttpLanguageServiceOptions): NxLanguageService {
  const baseUrl = options.baseUrl.replace(/\/+$/, "");
  const fetchImpl = options.fetch ?? globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new TypeError("createHttpLanguageService needs a fetch implementation; none is available globally.");
  }
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;

  async function query<Q extends LanguageQueryName>(
    name: Q,
    request: LanguageQueryRequests[Q],
    signal: AbortSignal | undefined,
  ): Promise<LanguageQueryAnswers[Q]> {
    signal?.throwIfAborted();
    const url = `${baseUrl}/${name}`;

    // One controller for both reasons a request stops early: the caller's signal, and the timeout.
    // Which one fired decides what the caller sees — their own abort, or a timeout error. It covers
    // the whole query — the header hook, the request, and the body — so a hook that never answers
    // or a response whose body never completes ends the same way a request that never connects does.
    const controller = new AbortController();
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, timeoutMs);
    const forwardAbort = () => controller.abort(signal?.reason);
    signal?.addEventListener("abort", forwardAbort, { once: true });
    const stopped = new Promise<never>((_, reject) => {
      controller.signal.addEventListener("abort", () => reject(controller.signal.reason ?? abortError()), {
        once: true,
      });
    });
    stopped.catch(() => undefined);
    const untilStopped = <T>(pending: Promise<T> | T): Promise<T> => Promise.race([pending, stopped]);

    let response: Response;
    let text: string;
    try {
      const headers = new Headers(
        typeof options.headers === "function" ? await untilStopped(options.headers(name)) : options.headers,
      );
      headers.set("content-type", "application/json");
      response = await fetchImpl(url, {
        method: "POST",
        headers,
        body: JSON.stringify(request),
        signal: controller.signal,
      });
      // `fetch` resolves once the headers are in; the body is still arriving. The race is what
      // stops a stalled body; on a real `fetch` the abort also tears the connection down.
      text = await untilStopped(response.text());
    } catch (error) {
      if (timedOut) {
        throw new LanguageServiceTimeoutError({ timeoutMs, url });
      }
      if (signal?.aborted) {
        throw signal.reason instanceof Error ? signal.reason : abortError();
      }
      throw error;
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener("abort", forwardAbort);
    }

    if (!response.ok) {
      let parsed: unknown;
      try {
        parsed = JSON.parse(text);
      } catch {
        parsed = undefined;
      }
      if (isLanguageServiceErrorBody(parsed) && parsed.error.code === "unsupported-query") {
        throw new UnsupportedQueryError(parsed.error.query ?? name, parsed.error.message);
      }
      throw new LanguageServiceHttpError({
        status: response.status,
        url,
        bodyExcerpt: text.slice(0, BODY_EXCERPT_LENGTH),
        ...(isLanguageServiceErrorBody(parsed) ? { code: parsed.error.code } : {}),
      });
    }
    // The answer arrived, but the caller has stopped wanting it.
    signal?.throwIfAborted();
    return JSON.parse(text) as LanguageQueryAnswers[Q];
  }

  return {
    hover: (request: HoverRequest, signal?: AbortSignal): Promise<Hover | null> => query("hover", request, signal),
    completions: (request: CompletionsRequest, signal?: AbortSignal): Promise<CompletionList> =>
      query("completions", request, signal),
    diagnostics: (request: DiagnosticsRequest, signal?: AbortSignal): Promise<DiagnosticReport> =>
      query("diagnostics", request, signal),
    documentSymbols: (request: DocumentSymbolsRequest, signal?: AbortSignal): Promise<DocumentSymbol[]> =>
      query("documentSymbols", request, signal),
  };
}

function abortError(): Error {
  return new DOMException("The language query was aborted.", "AbortError");
}
