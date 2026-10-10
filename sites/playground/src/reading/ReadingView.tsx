import { useEffect, useLayoutEffect, useRef } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement } from "@nx-lang/viewer";
import type { Reading } from "./useSourceTree.ts";

export interface ReadingViewProps {
  readonly reading: Reading;
  /** The selected node's key. */
  readonly selection: string | undefined;
  readonly onSelect: (key: string) => void;
}

/** The source pane's Read side: `<nx-viewer>` showing the source tree, or why it cannot. */
export function ReadingView({ reading, selection, onSelect }: ReadingViewProps) {
  const element = useRef<NxViewerElement>(null);
  const latestSelect = useRef(onSelect);
  latestSelect.current = onSelect;

  // The text goes in with no tree shown, so the element renders the new tree once, against the
  // text it was computed from.
  useLayoutEffect(() => {
    const target = element.current;
    if (target === null || reading.tree === null || reading.text === null) {
      return;
    }
    target.tree = undefined;
    target.text = reading.text;
    target.tree = reading.tree;
  }, [reading.tree, reading.text]);

  useLayoutEffect(() => {
    if (element.current !== null && element.current.selection !== selection) {
      element.current.selection = selection;
    }
  }, [selection, reading.tree]);

  useEffect(() => {
    const target = element.current;
    if (target === null) {
      return;
    }
    const listener = (event: Event) => latestSelect.current((event as CustomEvent<NxSelectDetail>).detail.key);
    target.addEventListener(NX_SELECT_EVENT, listener);
    return () => target.removeEventListener(NX_SELECT_EVENT, listener);
  }, [reading.tree === null]);

  return (
    <div className="reading">
      {reading.failure !== null && (
        <p className="reading-message fault" role="alert">
          The reading is unavailable: {reading.failure}
        </p>
      )}
      {reading.tree === null && reading.failure === null && <p className="reading-message quiet">Reading…</p>}
      {reading.tree !== null && <nx-viewer ref={element} stale={reading.stale} />}
    </div>
  );
}
