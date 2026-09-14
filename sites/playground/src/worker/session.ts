/**
 * The compiler and language service the worker runs, and its recovery from a crashed host.
 *
 * <para>Kept apart from the worker shell so it can be driven directly by a test under Node with the
 * same module the browser loads: the shell is the four lines that fetch the module and wire
 * `onmessage`, and everything with behaviour is here.</para>
 */
import type { SnapshotLanguageService } from "@nx-lang/language-core";
import type { LanguageQueryName } from "@nx-lang/language-protocol";
import { NxHostCrashedError, createLanguageService, createNxHost, type NxHost } from "@nx-lang/sdk-wasm";

import { compileWithCatalog } from "../compile/catalog.ts";
import type { CompileResult } from "../compile/types.ts";
import type { WorkerRequest } from "./protocol.ts";

/** What `createNxSession` accepts. */
export interface NxSessionOptions {
  /** The compiled module. A replacement host is created from it, so it is never fetched twice. */
  readonly module: WebAssembly.Module;
  /** The DrawnUI catalog, prepended to every compile and to every language query. */
  readonly catalog: string;
  /** Creates a host from the module. A test replaces this to force a crash. */
  readonly createHost?: (module: WebAssembly.Module) => NxHost;
}

/** The worker's compiler and language service, with one host behind both. */
export interface NxSession {
  /** Answers one request. Rejects with the failure the caller should see. */
  answer(request: WorkerRequest): Promise<unknown>;
  /** How many times a crashed host has been replaced. Observed by the session's tests. */
  readonly replacements: number;
  /** Releases the current host and its analyses. */
  dispose(): void;
}

/**
 * Creates the session: a host over `module`, a language service over that host's snapshots, and
 * the catalog prepended to both.
 */
export function createNxSession(options: NxSessionOptions): NxSession {
  const createHost = options.createHost ?? createNxHost;
  let host: NxHost = createHost(options.module);
  let language: SnapshotLanguageService = createLanguageService(host, {
    prelude: { source: options.catalog }
  });
  let replacements = 0;

  /**
   * Abandons the crashed host and starts a fresh one from the module already compiled.
   *
   * The analyses cached by the old service pointed into memory that no longer exists, so the
   * service is rebuilt rather than rebound.
   */
  function replaceHost(): void {
    replacements += 1;
    host = createHost(options.module);
    language = createLanguageService(host, { prelude: { source: options.catalog } });
  }

  return {
    async answer(request: WorkerRequest): Promise<unknown> {
      try {
        return request.kind === "compile"
          ? (compileWithCatalog(host, options.catalog, request.source) satisfies CompileResult)
          : await answerLanguage(language, request.query, request.request);
      } catch (error) {
        // A trap ended the instance. The caller is told what happened to its request, and the next
        // request is answered by a replacement, so one crash costs one request rather than the
        // session.
        if (error instanceof NxHostCrashedError) {
          replaceHost();
        }
        throw error;
      }
    },
    get replacements(): number {
      return replacements;
    },
    dispose(): void {
      language.dispose();
      host.dispose();
    }
  };
}

function answerLanguage(
  service: SnapshotLanguageService,
  query: LanguageQueryName,
  request: unknown
): Promise<unknown> {
  switch (query) {
    case "hover":
      return service.hover(request as Parameters<SnapshotLanguageService["hover"]>[0]);
    case "completions":
      return service.completions(request as Parameters<SnapshotLanguageService["completions"]>[0]);
    case "diagnostics":
      return service.diagnostics(request as Parameters<SnapshotLanguageService["diagnostics"]>[0]);
    case "documentSymbols":
      return service.documentSymbols(
        request as Parameters<SnapshotLanguageService["documentSymbols"]>[0]
      );
    default:
      return Promise.reject(new Error(`Unknown language query '${String(query)}'.`));
  }
}
