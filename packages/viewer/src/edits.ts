/**
 * The edits `<nx-viewer>` fires, made in a host's text one after another: what a host whose editor
 * keeps the text, as Monaco does, needs between the element's `nx-edit` and its language service.
 */
import type { NxLanguageService, SourceEdit, TextEdit } from "@nx-lang/language-protocol";

/** What a Visual edit asks: the key of the node it is aimed at and the edit, and whether it is live. */
export interface VisualEdit {
  readonly key: string;
  readonly edit: SourceEdit;
  /** A value the reader is still changing: previewed, replaced by the next, ended by an edit without it. */
  readonly live?: boolean;
}

/** The document a queue edits: its text now, and ways to apply text edits to it. */
export interface EditedDocument {
  /** The document's text, or null while there is none. */
  text(): string | null;
  /**
   * Applies `edits` to the text as one undoable step. Every range is in the text as it is now, and
   * none overlap.
   */
  apply(edits: readonly TextEdit[]): void;
  /**
   * Applies `edits` to the text outside its undo history, as a preview of a value the reader is still
   * changing; every range is in the text as it is now. A document without it ignores live edits.
   */
  preview?(edits: readonly TextEdit[]): void;
}

/** A value going live: the edit's key, the text before its first preview, and the text it left. */
interface Gesture {
  readonly key: string;
  readonly start: string;
  shown: string;
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
 *
 * A live edit is previewed outside the history: answered for the text before the first live edit of
 * its value, and shown in place of the previous preview, a live edit superseded before its turn
 * skipped. The next edit of that value without `live` takes the preview back and is answered for that
 * text and applied as one step, so the whole change is one step of the history, or none when it
 * changes nothing. An edit of another value, or `after`, first records a preview still shown as one
 * step; the commit of a change `after` recorded, coming after the step, changes nothing.
 */
export function createEditQueue(
  service: Pick<NxLanguageService, "sourceEdit">,
  uri: string,
  document: EditedDocument,
  onApplied: (text: string) => void,
  onUnchanged: () => void = () => undefined,
): EditQueue {
  let last: Promise<void> = Promise.resolve();
  let gesture: Gesture | null = null;
  // The value whose change `after` last recorded: the commit that ends it on the viewer's side, still
  // to come, is already a step, and applied again it would bring back what the step undid.
  let recorded: string | null = null;
  // How many edits have been fired: a live edit whose turn comes after a newer one was fired is skipped.
  let fired = 0;

  /** Shows `to` in place of what the document holds, outside its history. */
  const preview = (from: string, to: string): void => {
    if (from !== to) {
      document.preview!(differenceOf(from, to));
    }
  };

  /**
   * Ends the gesture as the document stands: its preview, when still shown, taken back and applied
   * as one step. Answers the text the next edit is asked for.
   */
  const close = (): string | null => {
    const open = gesture;
    gesture = null;
    const text = document.text();
    if (open === null || text === null || text !== open.shown || open.shown === open.start) {
      return text;
    }
    preview(open.shown, open.start);
    document.apply(differenceOf(open.start, open.shown));
    return document.text();
  };

  const ask = (source: string, edit: VisualEdit) =>
    service.sourceEdit({ documents: [{ uri, source }], uri, key: edit.key, edit: edit.edit });

  const live = (edit: VisualEdit, number: number): Promise<void> =>
    last.then(async () => {
      recorded = null;
      if (number !== fired || document.preview === undefined) {
        return;
      }
      let text = document.text();
      if (text === null) {
        return;
      }
      if (gesture !== null && (gesture.key !== edit.key || gesture.shown !== text)) {
        // Another value's change, or the text changed under the preview: what is shown stays.
        text = close();
        if (text === null) {
          return;
        }
      }
      const start = gesture?.start ?? text;
      const answer = await ask(start, edit);
      if (document.text() !== text) {
        onUnchanged();
        return;
      }
      const shown = answer.edits.length === 0 ? start : applied(start, answer.edits);
      gesture = { key: edit.key, start, shown };
      if (shown === text) {
        onUnchanged();
        return;
      }
      preview(text, shown);
      onApplied(shown);
    });

  const committed = (edit: VisualEdit): Promise<void> =>
    last.then(async () => {
      if (recorded === edit.key && gesture === null) {
        recorded = null;
        onUnchanged();
        return;
      }
      recorded = null;
      let text = document.text();
      if (gesture !== null && gesture.key === edit.key && gesture.shown === text) {
        // The change ends: the edit is asked for the text it began from while its preview still
        // shows, and the preview is taken back only as the answer is applied, in one turn, so the
        // text never reads the old value in between.
        const open = gesture;
        const answer = await ask(open.start, edit);
        gesture = null;
        if (document.text() !== open.shown) {
          // An edit typed in Source meanwhile: the text stays as it is.
          onUnchanged();
          return;
        }
        preview(open.shown, open.start);
        if (answer.edits.length > 0) {
          document.apply(answer.edits);
        }
        const after = document.text();
        if (after !== null && after !== open.shown) {
          onApplied(after);
        } else {
          onUnchanged();
        }
        return;
      }
      if (gesture !== null) {
        text = close();
      }
      if (text === null) {
        onUnchanged();
        return;
      }
      const answer = await ask(text, edit);
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

  const queue = (edit: VisualEdit): Promise<void> => {
    // Each edit supersedes the live edits still waiting their turn: a newer live value, or a commit.
    fired += 1;
    const next = edit.live === true ? live(edit, fired) : committed(edit);
    // A failed edit does not hold up the ones after it.
    last = next.catch((error: unknown) => {
      console.debug("nx-viewer: an edit failed", error);
      onUnchanged();
    });
    return next;
  };
  const after = (step: () => void): Promise<void> => {
    const next = last.then(() => {
      // A step of the history sees every change as a step of it.
      recorded = gesture?.key ?? recorded;
      close();
      step();
    });
    last = next.catch((error: unknown) => console.debug("nx-viewer: a step after the edits failed", error));
    return next;
  };
  return Object.assign(queue, { after });
}

/** `text` with `edits` applied, every range in `text`, none overlapping. */
function applied(text: string, edits: readonly TextEdit[]): string {
  const starts = lineStarts(text);
  const offset = ({ line, character }: { line: number; character: number }) => (starts[line] ?? text.length) + character;
  return edits
    .map((edit) => ({ start: offset(edit.range.start), end: offset(edit.range.end), newText: edit.newText }))
    .sort((a, b) => b.start - a.start)
    .reduce((result, { start, end, newText }) => result.slice(0, start) + newText + result.slice(end), text);
}

/** The one text edit that turns `from` into `to`: what lies between their common start and end. */
function differenceOf(from: string, to: string): TextEdit[] {
  let head = 0;
  const shorter = Math.min(from.length, to.length);
  while (head < shorter && from.charCodeAt(head) === to.charCodeAt(head)) {
    head += 1;
  }
  // A range never splits a surrogate pair: the two halves are one character.
  if (head > 0 && isHighSurrogate(from.charCodeAt(head - 1))) {
    head -= 1;
  }
  let tail = 0;
  while (tail < shorter - head && from.charCodeAt(from.length - 1 - tail) === to.charCodeAt(to.length - 1 - tail)) {
    tail += 1;
  }
  if (tail > 0 && isLowSurrogate(from.charCodeAt(from.length - tail))) {
    tail -= 1;
  }
  const starts = lineStarts(from);
  const position = (offset: number) => {
    let line = 0;
    while (line + 1 < starts.length && starts[line + 1]! <= offset) {
      line += 1;
    }
    return { line, character: offset - starts[line]! };
  };
  const bytes = (offset: number) => new TextEncoder().encode(from.slice(0, offset)).length;
  const end = from.length - tail;
  return [
    {
      range: { start: position(head), end: position(end), startByte: bytes(head), endByte: bytes(end) },
      newText: to.slice(head, to.length - tail),
    },
  ];
}

const isHighSurrogate = (unit: number): boolean => unit >= 0xd800 && unit <= 0xdbff;
const isLowSurrogate = (unit: number): boolean => unit >= 0xdc00 && unit <= 0xdfff;

/** Where each line of `text` starts, in UTF-16 offsets. */
function lineStarts(text: string): number[] {
  const starts = [0];
  for (let at = text.indexOf("\n"); at !== -1; at = text.indexOf("\n", at + 1)) {
    starts.push(at + 1);
  }
  return starts;
}

/** Visual's edits, made one after another, and steps to take in their order. */
export interface EditQueue {
  (edit: VisualEdit): Promise<void>;
  /** Runs `step` once every edit fired before it has been made, or refused, and a change previewed recorded. */
  after(step: () => void): Promise<void>;
}
