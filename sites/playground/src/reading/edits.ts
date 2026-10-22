import type { NxLanguageService, SourceEdit, TextEdit } from "@nx-lang/language-protocol";

/** What a Visual edit asks: the key of the node it is aimed at and the edit. */
export interface VisualEdit {
  readonly key: string;
  readonly edit: SourceEdit;
}

/** The document a queue edits: its text now, and a way to apply text edits to it as one step. */
export interface EditedDocument {
  /** The document's text, or null while there is none. */
  text(): string | null;
  /**
   * Applies `edits` to the text as one undoable step. Every range is in the text as it is now, and
   * none overlap.
   */
  apply(edits: readonly TextEdit[]): void;
}

/**
 * Applies Visual's edits to `document` in the order they are fired, each against the text the one
 * before left: the language service answers each edit for the text as it is when its turn comes,
 * and its text edits are applied unless the text changed while it was asked. A refused edit changes
 * nothing. `onApplied` is told the text after each edit that changed it, and `onUnchanged` of each
 * edit that changed nothing: refused, answered with no edits, answered for text since changed, or
 * failed. Visual shows the reader's value until it is given a tree, so it is given the one it has.
 * `after` runs a step, as an undo, after every edit fired before it: an undo pressed while a field's
 * edit is still being asked takes back that edit, not the one before it.
 */
export function createEditQueue(
  service: Pick<NxLanguageService, "sourceEdit">,
  uri: string,
  document: EditedDocument,
  onApplied: (text: string) => void,
  onUnchanged: () => void = () => undefined,
): EditQueue {
  let last: Promise<void> = Promise.resolve();
  const queue = (edit: VisualEdit): Promise<void> => {
    const next = last.then(async () => {
      const text = document.text();
      if (text === null) {
        onUnchanged();
        return;
      }
      const answer = await service.sourceEdit({ documents: [{ uri, source: text }], uri, key: edit.key, edit: edit.edit });
      // An edit typed in Source meanwhile moved the text the answer counts in.
      if (answer.edits.length === 0 || document.text() !== text) {
        onUnchanged();
        return;
      }
      document.apply(answer.edits);
      const after = document.text();
      if (after !== null) {
        onApplied(after);
      }
    });
    // A failed edit does not hold up the ones after it.
    last = next.catch((error: unknown) => {
      console.debug("nx visual edit", error);
      onUnchanged();
    });
    return next;
  };
  const after = (step: () => void): Promise<void> => {
    const next = last.then(step);
    last = next.catch((error: unknown) => console.debug("nx visual step", error));
    return next;
  };
  return Object.assign(queue, { after });
}

/** Visual's edits, made one after another, and steps to take in their order. */
export interface EditQueue {
  (edit: VisualEdit): Promise<void>;
  /** Runs `step` once every edit fired before it has been made, or refused. */
  after(step: () => void): Promise<void>;
}
