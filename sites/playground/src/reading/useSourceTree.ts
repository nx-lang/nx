import { useEffect, useState } from "react";
import { languageService, MODEL_URI } from "../editor/NxEditor";
import { NO_ANSWERS, readingOf, type Answers, type Reading } from "./reading.ts";

export type { Reading } from "./reading.ts";

/**
 * The source tree of `source` while `active`, asked of the worker once typing pauses, as evaluation
 * is. The last tree stays shown, marked stale, until the next one arrives; nothing is asked while
 * the reading view is hidden.
 */
export function useSourceTree(source: string | null, active: boolean, debounceMs = 350): Reading {
  const [answers, setAnswers] = useState<Answers>(NO_ANSWERS);

  useEffect(() => {
    if (!active || source === null || source === answers.text) {
      return;
    }
    const abort = new AbortController();
    // The first tree is asked for at once; later ones wait for typing to pause.
    const timer = setTimeout(
      () => {
        languageService
          .sourceTree({ documents: [{ uri: MODEL_URI, source }], uri: MODEL_URI }, abort.signal)
          .then(
            (tree) => {
              if (!abort.signal.aborted) {
                setAnswers({ tree, text: source, failure: null });
              }
            },
            (error: unknown) => {
              if (!abort.signal.aborted) {
                const message = error instanceof Error ? error.message : String(error);
                setAnswers((previous) => ({ ...previous, failure: { source, message } }));
              }
            },
          );
      },
      answers.text === null ? 0 : debounceMs,
    );
    return () => {
      abort.abort();
      clearTimeout(timer);
    };
    // `answers` is read only to skip work already done; a new answer must not ask again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source, active, debounceMs]);

  return readingOf(answers, source);
}
