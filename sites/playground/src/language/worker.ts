/**
 * Hover, completions, diagnostics and document symbols, answered in the app's compiler worker.
 *
 * <para>The same host that compiles answers these, over the same catalog prelude, so a hover range
 * and a compile diagnostic land on the same line of the author's own text. Each query carries the
 * editor's `AbortSignal`: an answer the editor has typed past is dropped rather than waited for.
 * </para>
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
  LanguageQueryName,
  NxLanguageService
} from "@nx-lang/language-protocol";

import { nxWorkerChannel } from "../worker/index.ts";

/** Creates the language service the Monaco integration is registered with. */
export function createWorkerLanguageService(): NxLanguageService {
  const ask = <T>(query: LanguageQueryName, request: unknown, signal?: AbortSignal): Promise<T> =>
    nxWorkerChannel().send({ kind: "language", query, request }, signal) as Promise<T>;

  return {
    hover: (request: HoverRequest, signal?: AbortSignal) =>
      ask<Hover | null>("hover", request, signal),
    completions: (request: CompletionsRequest, signal?: AbortSignal) =>
      ask<CompletionList>("completions", request, signal),
    diagnostics: (request: DiagnosticsRequest, signal?: AbortSignal) =>
      ask<DiagnosticReport>("diagnostics", request, signal),
    documentSymbols: (request: DocumentSymbolsRequest, signal?: AbortSignal) =>
      ask<DocumentSymbol[]>("documentSymbols", request, signal)
  };
}
