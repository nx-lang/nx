import {
  createSnapshotLanguageService,
  type SnapshotLanguageService
} from "@nx-lang/language-core";
import type { LanguageDocument } from "@nx-lang/language-protocol";

import type { NxHost } from "./host.js";
import type { NxLanguageDocumentInput } from "./types.js";

/** What `createLanguageService` accepts beyond the host. */
export interface NxLanguageServiceOptions {
  /**
   * Documents the host always includes alongside the queried set, such as a catalog of external
   * components. A diagnostic in one is reported against that document's URI.
   */
  readonly documents?: readonly NxLanguageDocumentInput[];

  /**
   * Identities every queried document imports implicitly, as if it began with a wildcard import
   * of each; typically the identities of `documents`. A listed document imports nothing
   * implicitly itself.
   */
  readonly implicitImports?: readonly string[];

  /** How many analyzed document sets to keep. Default 8. */
  readonly cacheSize?: number;
}

/**
 * An `NxLanguageService` that answers from `host`'s snapshots, in this thread.
 *
 * <para>The answering and the cache are `@nx-lang/language-core`, the same code the HTTP handler
 * runs, so a hover answered here and the same hover answered by a server agree on every line and
 * column. The host's context reaches the service as documents of its own, made visible through an
 * implicit import, so the queried document's text is analyzed as it is and every answer is already
 * in its coordinates.</para>
 *
 * <para>Every query enters the module, so a caller that must stay responsive — the playground's
 * editor, for one — runs this in a worker. A trap inside the module surfaces as
 * `NxHostCrashedError` from the query that caused it; the caller replaces the host and builds a new
 * service over it.</para>
 */
export function createLanguageService(
  host: NxHost,
  options: NxLanguageServiceOptions = {}
): SnapshotLanguageService {
  // The host's documents and implicit imports are the core's own host context, so joining the sets
  // and refusing a query that would replace a context document happen in one place.
  return createSnapshotLanguageService({
    createSnapshot: (documents: readonly LanguageDocument[], snapshotOptions) =>
      host.createLanguageSnapshot(documents, { implicitImports: snapshotOptions.implicitImports }),
    context: {
      documents: Array.from(options.documents ?? []) as readonly LanguageDocument[],
      implicitImports: Array.from(options.implicitImports ?? [])
    },
    ...(options.cacheSize === undefined ? {} : { cacheSize: options.cacheSize })
  });
}
