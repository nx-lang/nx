import type { SourceTree } from "@nx-lang/language-protocol";

/** The reading view's tree, the text it was computed from, and whether a newer one is coming. */
export interface Reading {
  readonly tree: SourceTree | null;
  /** The source `tree` describes; null until the first tree arrives. */
  readonly text: string | null;
  /** Whether `tree` describes other text than the current source. */
  readonly stale: boolean;
  /** Why the query for the current source failed, or null. */
  readonly failure: string | null;
}

/** What the hook remembers: the last tree it got, and the last failure and the source it was for. */
export interface Answers {
  readonly tree: SourceTree | null;
  readonly text: string | null;
  readonly failure: { readonly source: string; readonly message: string } | null;
}

export const NO_ANSWERS: Answers = { tree: null, text: null, failure: null };

/**
 * The reading of `source` from what the hook has: stale whenever the tree is of other text, which
 * a query being computed, cancelled or failed all leave it, and a failure only for this source.
 */
export function readingOf(answers: Answers, source: string | null): Reading {
  return {
    tree: answers.tree,
    text: answers.text,
    stale: answers.tree !== null && answers.text !== source,
    failure: answers.failure !== null && answers.failure.source === source ? answers.failure.message : null,
  };
}
