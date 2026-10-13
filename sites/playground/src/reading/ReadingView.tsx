import { useEffect, useLayoutEffect, useRef, useState } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement, type NxViewerView } from "@nx-lang/viewer";
import { colorize, loadHighlighter, type NxValueHighlighter } from "@nx-lang/value-view";
import { EDITOR_THEMES } from "../theme.ts";
import type { Reading } from "./reading.ts";
// The faces the source reading is being tried in. Latin only, in the weights and styles a theme asks for.
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-400-italic.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-400-italic.css";
import "@fontsource/ibm-plex-mono/latin-700.css";
import "@fontsource/ia-writer-quattro/latin-400.css";
import "@fontsource/ia-writer-quattro/latin-400-italic.css";
import "@fontsource/ia-writer-quattro/latin-700.css";

/**
 * The fonts the source reading can be set in while they are compared: the editor's monospace, the
 * system's proportional face for prose with its monospace one for code, one bundled proportional
 * face throughout, a bundled proportional face for prose with its monospace one for code, and a
 * duospace face.
 */
export const READING_FONTS = ["monospace", "hybrid-system", "proportional-system", "proportional", "hybrid", "duospace"] as const;
export type ReadingFont = (typeof READING_FONTS)[number];
export const READING_FONT_LABELS: Record<ReadingFont, string> = {
  monospace: "Monospace (as Edit)",
  "hybrid-system": "Hybrid, system fonts (Segoe UI prose, Consolas code)",
  "proportional-system": "Proportional, system font (Segoe UI)",
  proportional: "Proportional (IBM Plex Sans)",
  hybrid: "Hybrid (Plex Sans prose, Plex Mono code)",
  duospace: "Duospace (iA Writer Quattro)",
};

export interface ReadingViewProps {
  readonly reading: Reading;
  /** The source's own text with its nodes marked, or the reading that draws cards. */
  readonly view: NxViewerView;
  /** The font the source reading is set in. */
  readonly font: ReadingFont;
  /** The selected node's key. */
  readonly selection: string | undefined;
  readonly onSelect: (key: string) => void;
}

/** The source pane's Read side: `<nx-viewer>` showing the source tree, or why it cannot. */
export function ReadingView({ reading, view, font, selection, onSelect }: ReadingViewProps) {
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
      {reading.tree !== null && <nx-viewer ref={element} stale={reading.stale} className={`reading-font-${font}`} />}
    </div>
  );
}
