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
import "@fontsource/atkinson-hyperlegible-next/latin-400.css";
import "@fontsource/atkinson-hyperlegible-next/latin-400-italic.css";
import "@fontsource/atkinson-hyperlegible-next/latin-700.css";
import "@fontsource/atkinson-hyperlegible-mono/latin-400.css";
import "@fontsource/atkinson-hyperlegible-mono/latin-400-italic.css";
import "@fontsource/atkinson-hyperlegible-mono/latin-700.css";
// Fira Mono has no italic of its own; the browser slants the upright.
import "@fontsource/geist/latin-400.css";
import "@fontsource/geist/latin-400-italic.css";
import "@fontsource/geist/latin-700.css";
import "@fontsource/geist-mono/latin-400.css";
import "@fontsource/geist-mono/latin-400-italic.css";
import "@fontsource/geist-mono/latin-700.css";
import "@fontsource/source-sans-3/latin-400.css";
import "@fontsource/source-sans-3/latin-400-italic.css";
import "@fontsource/source-sans-3/latin-700.css";
import "@fontsource/source-code-pro/latin-400.css";
import "@fontsource/source-code-pro/latin-400-italic.css";
import "@fontsource/source-code-pro/latin-700.css";
import "@fontsource/fira-sans/latin-400.css";
import "@fontsource/fira-sans/latin-400-italic.css";
import "@fontsource/fira-sans/latin-700.css";
import "@fontsource/fira-mono/latin-400.css";
import "@fontsource/fira-mono/latin-700.css";
// Variable faces: Inter by weight, and Recursive with every axis, its monospace one among them.
import "@fontsource-variable/inter/wght.css";
import "@fontsource-variable/inter/wght-italic.css";
import "@fontsource-variable/recursive/full.css";

/**
 * The fonts the source reading can be set in while they are compared: the editor's monospace, the
 * system's proportional face for prose with its monospace one for code, one bundled proportional
 * face throughout, a bundled proportional face for prose with its monospace one for code, and a
 * duospace face.
 */
export const READING_FONTS = [
  "monospace",
  "hybrid-system",
  "proportional-system",
  "verdana",
  "inter",
  "atkinson",
  "recursive",
  "geist",
  "source",
  "fira",
  "proportional",
  "atkinson-hybrid",
  "recursive-hybrid",
  "geist-hybrid",
  "source-hybrid",
  "fira-hybrid",
  "hybrid",
  "duospace",
] as const;
export type ReadingFont = (typeof READING_FONTS)[number];
export const READING_FONT_LABELS: Record<ReadingFont, string> = {
  monospace: "Monospace (as Edit)",
  "hybrid-system": "Hybrid, system: Segoe UI prose, Consolas code",
  "proportional-system": "Proportional, system: Segoe UI",
  verdana: "Proportional, system: Verdana",
  inter: "Proportional: Inter",
  atkinson: "Proportional: Atkinson Hyperlegible",
  recursive: "Proportional: Recursive",
  geist: "Proportional: Geist",
  source: "Proportional: Source Sans 3",
  fira: "Proportional: Fira Sans",
  proportional: "Proportional: IBM Plex Sans",
  "atkinson-hybrid": "Hybrid: Atkinson Hyperlegible, with its Mono for code",
  "recursive-hybrid": "Hybrid: Recursive, monospace for code",
  "geist-hybrid": "Hybrid: Geist, with Geist Mono for code (default)",
  "source-hybrid": "Hybrid: Source Sans 3, with Source Code Pro for code",
  "fira-hybrid": "Hybrid: Fira Sans, with Fira Mono for code",
  hybrid: "Hybrid: IBM Plex Sans, with Plex Mono for code",
  duospace: "Duospace: iA Writer Quattro",
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
