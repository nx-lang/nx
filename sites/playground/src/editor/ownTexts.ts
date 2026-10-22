/**
 * The texts an editor has reported as changed that no render has passed back as its `value` yet,
 * oldest first. Two changes can land between renders, as two quick edits from Visual do, and a
 * render then passes a `value` the editor has already moved past: that is not a text from outside,
 * and setting it would take back the newer change and clear the undo history.
 */
export interface OwnTexts {
  /** Notes a text the editor reported as changed. */
  reported(text: string): void;
  /**
   * Whether a render's `value` should replace `current`, the editor's text, or null before there is
   * an editor: only a value from outside, never the editor's own, nor one it already holds.
   */
  shouldSet(value: string, current: string | null): boolean;
}

export function createOwnTexts(): OwnTexts {
  let unseen: string[] = [];
  // The last `value` a render passed.
  let rendered: string | undefined;
  return {
    reported(text) {
      if (text === rendered) {
        // Back at what the last render passed, as after an edit and its undo batched into one render,
        // which then passes nothing new: whatever was reported before is behind it.
        unseen = [];
        return;
      }
      unseen.push(text);
    },
    shouldSet(value, current) {
      rendered = value;
      if (current === null || value === current) {
        // The editor and the render agree: whatever it reported before is behind them both.
        unseen = [];
        return false;
      }
      const seen = unseen.indexOf(value);
      if (seen !== -1) {
        // The editor's own text, which it has since moved past.
        unseen = unseen.slice(seen + 1);
        return false;
      }
      unseen = [];
      return true;
    },
  };
}
