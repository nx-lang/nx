import {
  createSnapshotLanguageService,
  type SnapshotLanguageService
} from "@nx-lang/language-core";
import type { LanguageDocument } from "@nx-lang/language-protocol";

import type { NxHost } from "./host.js";

/** What `createLanguageService` accepts beyond the host. */
export interface NxLanguageServiceOptions {
  /**
   * NX source placed ahead of the queried document before analysis, for hosts whose context
   * declarations cannot yet be imported without loss. Positions and ranges are shifted so the
   * caller sees only its own coordinates.
   */
  readonly prelude?: { source: string };

  /** How many analyzed document sets to keep. Default 8. */
  readonly cacheSize?: number;
}

/**
 * An `NxLanguageService` that answers from `host`'s snapshots, in this thread.
 *
 * <para>The answering, the prelude arithmetic and the cache are `@nx-lang/language-core`, the same
 * code the HTTP handler runs, so a hover answered here and the same hover answered by a server
 * agree on every line and column.</para>
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
  return createSnapshotLanguageService({
    createSnapshot: (documents: readonly LanguageDocument[]) =>
      host.createLanguageSnapshot(documents),
    ...(options.prelude === undefined ? {} : { prelude: options.prelude }),
    ...(options.cacheSize === undefined ? {} : { cacheSize: options.cacheSize })
  });
}
