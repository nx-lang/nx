import { useEffect, useLayoutEffect, useRef, useState } from "react";
import "@nx-lang/viewer";
import { NX_SELECT_EVENT, type NxSelectDetail, type NxViewerElement, type NxViewerIcons } from "@nx-lang/viewer";
import { colorize, loadHighlighter, type NxValueHighlighter } from "@nx-lang/value-view";
import { EDITOR_THEMES, type EditorThemes } from "../theme.ts";
import type { Reading } from "./reading.ts";
// The reading's faces: Geist for prose and Geist Mono for code. Latin only, in the weights and
// styles the theme asks for.
import "@fontsource/geist/latin-400.css";
import "@fontsource/geist/latin-400-italic.css";
import "@fontsource/geist/latin-700.css";
import "@fontsource/geist-mono/latin-400.css";
import "@fontsource/geist-mono/latin-400-italic.css";
import "@fontsource/geist-mono/latin-700.css";

export interface ReadingViewProps {
  readonly reading: Reading;
  /** The selected node's key. */
  readonly selection: string | undefined;
  readonly onSelect: (key: string) => void;
}

/**
 * Draws Lucide icons by name, for the source reading to mark elements with. The whole set loads
 * once, apart from the rest of the playground, when the reading first needs it.
 */
let lucideIcons: Promise<NxViewerIcons> | undefined;
function loadIcons(): Promise<NxViewerIcons> {
  lucideIcons ??= import("lucide").then(({ icons, createElement }) => {
    const drawn = new Map<string, Element | undefined>();
    return (name: string) => {
      if (!drawn.has(name)) {
        // Lucide names its icons in kebab case and exports them in Pascal case.
        const exported = name.replace(/(^|-)([a-z0-9])/g, (_match, _dash: string, letter: string) => letter.toUpperCase());
        const node = (icons as Record<string, Parameters<typeof createElement>[0] | undefined>)[exported];
        drawn.set(name, node === undefined ? undefined : createElement(node));
      }
      return drawn.get(name);
    };
  });
  return lucideIcons;
}

/** The source pane's Read side: `<nx-viewer>` showing the source tree, or why it cannot. */
export function ReadingView({ reading, selection, onSelect }: ReadingViewProps) {
  const element = useRef<NxViewerElement>(null);
  // The highlighter the output pane colors with, loaded with the editor's themes, so the source
  // reading takes the colors the editor shows.
  const themes = EDITOR_THEMES;
  // A highlighter is loaded for one pair of themes, so it is kept with the pair it colors with.
  const [highlighter, setHighlighter] = useState<{ value: NxValueHighlighter; themes: EditorThemes } | null>(null);
  useEffect(() => {
    let current = true;
    loadHighlighter(themes).then(
      (loaded) => {
        if (current) {
          setHighlighter({ value: loaded, themes });
        }
      },
      // Without a highlighter the text still reads, uncolored.
      () => undefined
    );
    return () => {
      current = false;
    };
  }, [themes]);
  const [icons, setIcons] = useState<NxViewerIcons | undefined>(undefined);
  useEffect(() => {
    let current = true;
    loadIcons().then(
      (loaded) => {
        if (current) {
          setIcons(() => loaded);
        }
      },
      // Without icons the reading is the same, unmarked.
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
    target.icons = icons;
    target.colors = highlighter !== null ? colorize(highlighter.value, reading.text, highlighter.themes) : [];
    target.tree = reading.tree;
  }, [reading.tree, reading.text, highlighter, icons]);

  useLayoutEffect(() => {
    if (element.current !== null && element.current.selection !== selection) {
      element.current.selection = selection;
    }
  }, [selection, reading.tree, highlighter]);

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
