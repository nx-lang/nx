import { useEffect, useState } from "react";
import type { SourceTree } from "@nx-lang/language-protocol";
import { languageService, MODEL_URI } from "../editor/NxEditor";

/** The reading view's tree, the text it was computed from, and whether a newer one is coming. */
export interface Reading {
  readonly tree: SourceTree | null;
  /** The source `tree` describes; null until the first tree arrives. */
  readonly text: string | null;
  /** Set while a tree for newer source is being computed. */
  readonly stale: boolean;
  /** Why the latest query failed, or null. */
  readonly failure: string | null;
}

const NOTHING_YET: Reading = { tree: null, text: null, stale: false, failure: null };

/**
 * The source tree of `source` while `active`, asked of the worker once typing pauses, as evaluation
 * is. The last tree stays shown, marked stale, until the next one arrives; nothing is asked while
 * the reading view is hidden.
 */
export function useSourceTree(source: string | null, active: boolean, debounceMs = 350): Reading {
  const [reading, setReading] = useState<Reading>(NOTHING_YET);

  useEffect(() => {
    if (!active || source === null || (source === reading.text && reading.failure === null)) {
      return;
    }
    setReading((previous) => (previous.stale ? previous : { ...previous, stale: true }));
    const abort = new AbortController();
    // The first tree is asked for at once; later ones wait for typing to pause.
    const timer = setTimeout(
      () => {
        languageService
          .sourceTree({ documents: [{ uri: MODEL_URI, source }], uri: MODEL_URI }, abort.signal)
          .then(
            (tree) => {
              if (!abort.signal.aborted) {
                setReading({ tree, text: source, stale: false, failure: null });
              }
            },
            (error: unknown) => {
              if (!abort.signal.aborted) {
                const failure = error instanceof Error ? error.message : String(error);
                setReading((previous) => ({ ...previous, stale: false, failure }));
              }
            },
          );
      },
      reading.text === null ? 0 : debounceMs,
    );
    return () => {
      abort.abort();
      clearTimeout(timer);
    };
    // `reading` is read only to skip work already done; a new answer must not ask again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source, active, debounceMs]);

  return reading;
}
