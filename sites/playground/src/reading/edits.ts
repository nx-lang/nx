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
  /** Applies `edits` one after another to the text, as one undoable step. */
  apply(edits: readonly TextEdit[]): void;
}

/**
 * Applies Visual's edits to `document` in the order they are fired, each against the text the one
 * before left: the language service answers each edit for the text as it is when its turn comes,
 * and its text edits are applied unless the text changed while it was asked. A refused edit changes
 * nothing. `onApplied` is told the text after each edit that changed it.
 */
export function createEditQueue(
  service: Pick<NxLanguageService, "sourceEdit">,
  uri: string,
  document: EditedDocument,
  onApplied: (text: string) => void,
): (edit: VisualEdit) => Promise<void> {
  let last: Promise<void> = Promise.resolve();
  return (edit) => {
    const next = last.then(async () => {
      const text = document.text();
      if (text === null) {
        return;
      }
      const answer = await service.sourceEdit({ documents: [{ uri, source: text }], uri, key: edit.key, edit: edit.edit });
      // An edit typed in Source meanwhile moved the text the answer counts in.
      if (answer.edits.length === 0 || document.text() !== text) {
        return;
      }
      document.apply(answer.edits);
      const after = document.text();
      if (after !== null) {
        onApplied(after);
      }
    });
    // A failed edit does not hold up the ones after it.
    last = next.catch((error: unknown) => console.debug("nx visual edit", error));
    return next;
  };
}
