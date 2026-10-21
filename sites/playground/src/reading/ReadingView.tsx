import { useEffect, useLayoutEffect, useRef, useState, type MutableRefObject } from "react";
import "@nx-lang/viewer";
import {
  NX_EDIT_EVENT,
  NX_FOLD_EVENT,
  NX_OPEN_EVENT,
  NX_SELECT_EVENT,
  type NxEditDetail,
  type NxFoldDetail,
  type NxOpenDetail,
  type NxSelectDetail,
  type NxViewerElement,
  type NxViewerIcons,
} from "@nx-lang/viewer";
import { colorize, loadHighlighter, type NxValueHighlighter } from "@nx-lang/value-view";
import { EDITOR_THEMES } from "../theme.ts";
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
  /** The keys of the groups and declarations folded, kept by the host across Source and Visual. */
  readonly folded: readonly string[] | undefined;
  /** Also hands back the selection, which a fold that hides it moves or clears. */
  readonly onFold: (folded: readonly string[], selection: string | undefined) => void;
  /** Filled in with what the pane's title bar can ask of the reading. */
  readonly commands: MutableRefObject<ReadingCommands | null>;
  /** An edit the visitor made in Visual, for the source; with it, Visual edits values in place. */
  readonly onEdit?: (detail: NxEditDetail) => void;
  /** A value Visual cannot edit, to show in Source. */
  readonly onOpen?: (detail: NxOpenDetail) => void;
  /** Ctrl or Cmd+Z, and Ctrl+Y or Ctrl or Cmd+Shift+Z, pressed in Visual, for the editor's history. */
  readonly onUndo?: () => void;
  readonly onRedo?: () => void;
  /**
   * Raised when an edit made in Visual changed nothing in the source, so no new tree comes: the
   * reading is given its tree again, and stops showing the value the visitor left.
   */
  readonly revision?: number;
}

/** What the NX pane's title bar can ask of the Visual view. */
export interface ReadingCommands {
  collapseAll(): void;
  expandAll(): void;
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

/** The NX pane's Visual view: `<nx-viewer>` showing the source tree, or why it cannot. */
export function ReadingView({
  reading,
  selection,
  onSelect,
  folded,
  onFold,
  commands,
  onEdit,
  onOpen,
  onUndo,
  onRedo,
  revision,
}: ReadingViewProps) {
  const element = useRef<NxViewerElement>(null);
  const latestFold = useRef(onFold);
  latestFold.current = onFold;
  const latest = useRef({ onEdit, onOpen, onUndo, onRedo });
  latest.current = { onEdit, onOpen, onUndo, onRedo };
  const editable = onEdit !== undefined;
  useEffect(() => {
    commands.current = {
      collapseAll: () => element.current?.collapseAll(),
      expandAll: () => element.current?.expandAll(),
    };
    return () => {
      commands.current = null;
    };
  }, [commands]);
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
    target.folded = folded;
    target.icons = icons;
    target.editable = editable;
    target.colors = highlighter !== null ? colorize(highlighter, reading.text, EDITOR_THEMES) : [];
    target.tree = reading.tree;
    // `revision` gives the tree again after an edit that changed nothing.
  }, [reading.tree, reading.text, highlighter, icons, revision]);

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
    const foldListener = (event: Event) => {
      const { folded, selection } = (event as CustomEvent<NxFoldDetail>).detail;
      latestFold.current(folded, selection);
    };
    const editListener = (event: Event) => latest.current.onEdit?.((event as CustomEvent<NxEditDetail>).detail);
    const openListener = (event: Event) => latest.current.onOpen?.((event as CustomEvent<NxOpenDetail>).detail);
    // The reading leaves undo and redo to the host, which keeps one history for Source and Visual.
    // Inside one of the reading's own fields, they are the field's, and do not reach here.
    const keyListener = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) {
        return;
      }
      const key = event.key.toLowerCase();
      const redo = key === "y" || (key === "z" && event.shiftKey);
      if (key !== "z" && key !== "y") {
        return;
      }
      const handler = redo ? latest.current.onRedo : latest.current.onUndo;
      if (handler !== undefined) {
        event.preventDefault();
        handler();
      }
    };
    target.addEventListener(NX_SELECT_EVENT, listener);
    target.addEventListener(NX_FOLD_EVENT, foldListener);
    target.addEventListener(NX_EDIT_EVENT, editListener);
    target.addEventListener(NX_OPEN_EVENT, openListener);
    target.addEventListener("keydown", keyListener);
    return () => {
      target.removeEventListener(NX_SELECT_EVENT, listener);
      target.removeEventListener(NX_FOLD_EVENT, foldListener);
      target.removeEventListener(NX_EDIT_EVENT, editListener);
      target.removeEventListener(NX_OPEN_EVENT, openListener);
      target.removeEventListener("keydown", keyListener);
    };
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
