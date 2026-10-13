import { useEffect, useLayoutEffect, useRef, useState } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement, type NxViewerView } from "@nx-lang/viewer";
import { colorize, loadHighlighter, type NxValueHighlighter } from "@nx-lang/value-view";
import { EDITOR_THEMES } from "../theme.ts";
import type { Reading } from "./reading.ts";

export interface ReadingViewProps {
  readonly reading: Reading;
  /** The source's own text with its nodes marked, or the reading that draws cards. */
  readonly view: NxViewerView;
  /** The selected node's key. */
  readonly selection: string | undefined;
  readonly onSelect: (key: string) => void;
}

/** The source pane's Read side: `<nx-viewer>` showing the source tree, or why it cannot. */
export function ReadingView({ reading, view, selection, onSelect }: ReadingViewProps) {
  const element = useRef<NxViewerElement>(null);
  // The highlighter the output pane colors with, loaded with the editor's themes, so the source
  // reading takes the colors the editor shows.
  const [highlighter, setHighlighter] = useState<NxValueHighlighter | null>(null);
  useEffect(() => {
    let current = true;
    loadHighlighter(EDITOR_THEMES).then(
      (loaded) => {
        if (current) {
          setHighlighter(loaded);
        }
      },
      // Without a highlighter the text still reads, uncolored.
      () => undefined
    );
    return () => {
      current = false;
    };
  }, []);
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
    target.view = view;
    target.colors = view === "source" && highlighter !== null ? colorize(highlighter, reading.text, EDITOR_THEMES) : [];
    target.tree = reading.tree;
  }, [reading.tree, reading.text, view, highlighter]);

  useLayoutEffect(() => {
    if (element.current !== null && element.current.selection !== selection) {
      element.current.selection = selection;
    }
  }, [selection, reading.tree, view, highlighter]);

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
