import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { NxValueDescribe, NxValueNavigateDetail, NxValueNode } from "@nx-lang/value-view";
import "./app.css";
import { evaluateInBrowser, type Diagnostic } from "./compile";
import { NxEditor, languageService, MODEL_URI, type EditorHistory, type NxEditorHandle } from "./editor/NxEditor";
import { declaredName, offsetAtPosition, offsetOf, positionAt } from "./editor/positions.ts";
import {
  DEFAULT_EXAMPLE,
  EXAMPLE_GROUPS,
  docsUrl,
  exampleById,
  exampleWithSource,
  type Example,
} from "./examples";
import { SiteHeader } from "./header/SiteHeader";
import { OutputPane, type OutputView } from "./output/OutputPane";
import { PreviewView } from "./preview/PreviewView";
import { usePreview } from "./preview/usePreview.ts";
import { ReadingView, type ReadingCommands } from "./reading/ReadingView";
import { keyAtRange, rangeOfKey, type NxOpenDetail } from "@nx-lang/viewer";
import { createEditQueue } from "./reading/edits.ts";
import { useSourceTree } from "./reading/useSourceTree.ts";
import { SITE_ROOT } from "./paths.ts";
import { createAddressKeeper } from "./address.ts";
import { pathForPayload, routeFromLocation } from "./routes.ts";
import { decodeSource, encodeSource } from "./share/codec.ts";
import type { PreviewReloadNotice, PreviewState } from "./preview/protocol.ts";
import { useSiteTheme } from "./theme.ts";
import { useEvaluation } from "./useEvaluation.ts";
import { startNxWorker } from "./worker/index.ts";

/** What the page shows at the address it was opened or navigated to. */
interface Opened {
  readonly source: string;
  /** Set when the address asked for something that is not there, said once above the editor. */
  readonly notice: string | null;
}

const isExample = (id: string) => exampleById(id) !== undefined;

/** The undo and redo shortcuts as the visitor's platform spells them, for the buttons' tooltips. */
const SHORTCUTS = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "")
  ? { undo: "⌘Z", redo: "⇧⌘Z" }
  : { undo: "Ctrl+Z", redo: "Ctrl+Y" };

/** Reads the current address into source, decoding a `#code=` fragment. */
async function openLocation(): Promise<Opened> {
  const route = routeFromLocation(window.location.pathname, window.location.hash, SITE_ROOT, isExample);
  switch (route.kind) {
    case "code":
      try {
        return { source: await decodeSource(route.payload), notice: null };
      } catch (error) {
        return {
          source: DEFAULT_EXAMPLE.source,
          notice: `The link could not be read (${error instanceof Error ? error.message : String(error)}), so the default example is open.`,
        };
      }
    case "example":
      return { source: exampleById(route.id)!.source, notice: null };
    case "default":
      return {
        source: DEFAULT_EXAMPLE.source,
        notice:
          route.missing === undefined
            ? null
            : `There is no example called “${route.missing}”, so the default example is open. To draw interfaces with NX, try the DrawnUI fiddle.`,
      };
  }
}

/** How the NX pane shows the source: as text in the editor, or visually, as the reading. */
type SourceMode = "source" | "visual";

/** The site: the header, the toolbar, and the source and output panes. */
export function App() {
  const theme = useSiteTheme();
  const [source, setSource] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [shareState, setShareState] = useState<"idle" | "copied" | "failed">("idle");
  const editor = useRef<NxEditorHandle>(null);
  // Only typed text is written back into the address, so opening an example keeps its clean path.
  const address = useMemo(
    () => createAddressKeeper({ history: window.history, root: SITE_ROOT, encode: encodeSource }),
    [],
  );
  const sourceRef = useRef<string | null>(null);
  sourceRef.current = source;

  useEffect(startNxWorker, []);

  // The address decides the source on first load and on Back and Forward.
  useEffect(() => {
    let current = true;
    const open = () => {
      void openLocation().then((opened) => {
        if (current) {
          // Another document opens on its own folding, as choosing an example does.
          setSource(opened.source);
          setFolded(undefined);
          setNotice(opened.notice);
        }
      });
    };
    const onPop = () => {
      address.navigated();
      open();
    };
    open();
    window.addEventListener("popstate", onPop);
    return () => {
      current = false;
      window.removeEventListener("popstate", onPop);
    };
  }, [address]);

  const example = source === null ? undefined : exampleWithSource(source);

  useEffect(() => {
    document.title = example === undefined ? "NX Playground" : `${example.title} — NX Playground`;
  }, [example]);

  const evaluation = useEvaluation(source, evaluateInBrowser);
  // The source the output came from, read by the hover callback without re-creating it.
  const evaluationSourceRef = useRef<string | null>(null);
  evaluationSourceRef.current = evaluation.outcomeSource;

  const chooseExample = (chosen: Example) => {
    // A new history entry, so Back returns to the source that was there, edits and all.
    void address.choose(chosen.id);
    setSource(chosen.source);
    setFolded(undefined);
    setNotice(null);
  };

  const share = async () => {
    if (source === null) {
      return;
    }
    try {
      const payload = await encodeSource(source);
      const link = new URL(pathForPayload(payload, SITE_ROOT), window.location.origin).href;
      await navigator.clipboard.writeText(link);
      setShareState("copied");
    } catch {
      setShareState("failed");
    }
    setTimeout(() => setShareState("idle"), 2000);
  };

  const [view, setView] = useState<OutputView>("output");
  const preview = usePreview(view === "preview", evaluation.outcomeSource);
  // The source the preview's spans index, read by its hover callback.
  const previewSourceRef = useRef<string | null>(null);
  previewSourceRef.current = preview.state?.source ?? null;
  // The preview is behind the editor: the edit does not compile, or the session could not run it
  // (its props, or its program, failed) and kept what it had.
  const previewStale = evaluation.stale || preview.notice?.kind === "failed";

  /** The output's hover: the language service's hover at the node's declaration, while it lines up. */
  const describe = useCallback<NxValueDescribe>(
    (node) => hoverAt(node, sourceRef.current, evaluationSourceRef.current),
    [],
  );
  /** The preview's hover, over the source the session's spans index. */
  const describePreview = useCallback<NxValueDescribe>(
    (node) => hoverAt(node, sourceRef.current, previewSourceRef.current),
    [],
  );

  const [mode, setMode] = useState<SourceMode>("visual");
  const [selection, setSelection] = useState<string | undefined>(undefined);
  // What the reader folded in Visual, kept while they switch to Source and back; until they fold
  // anything, Visual folds the source's supporting declarations itself.
  const [folded, setFolded] = useState<readonly string[] | undefined>(undefined);
  const readingCommands = useRef<ReadingCommands | null>(null);
  // The text an edit made in Visual left, whose tree is asked for at once rather than after a pause.
  const immediateTree = useRef<string | null>(null);
  // Raised by an edit made in Visual that changed nothing, or by an undo or redo, to give Visual its
  // tree again.
  const [visualRevision, setVisualRevision] = useState(0);
  const reading = useSourceTree(source, mode === "visual", undefined, immediateTree);
  // Visual's edits, made in the editor's text one after another, each one step of its undo history.
  const applyVisualEdit = useMemo(
    () =>
      createEditQueue(
        languageService,
        MODEL_URI,
        { text: () => editor.current?.text() ?? null, apply: (edits) => editor.current?.applyEdits(edits) },
        (text) => {
          immediateTree.current = text;
        },
        () => setVisualRevision((revision) => revision + 1),
      ),
    [],
  );
  // Whether the editor's history holds anything to undo and to redo, for the title bar's buttons.
  const [history, setHistory] = useState<EditorHistory>({ canUndo: false, canRedo: false });
  // A field open in Visual holding a change, and edits fired and not yet in the editor's history:
  // Undo takes them back too, after committing the field, so it is offered for them as well.
  const [visualDraft, setVisualDraft] = useState(false);
  const [editsInFlight, setEditsInFlight] = useState(0);
  const canUndo = history.canUndo || (mode === "visual" && (visualDraft || editsInFlight > 0));
  // When Visual last closed a field's change without committing it, as leaving a field holding an
  // entry it cannot take does, and when the Undo button was last pressed: a press that closed one
  // took that change back, which is all the visitor asked Undo for.
  const visualDraftRef = useRef(false);
  const draftDroppedAt = useRef(Number.NEGATIVE_INFINITY);
  const undoPressedAt = useRef(Number.POSITIVE_INFINITY);
  const onVisualDraft = (changed: boolean) => {
    if (!changed && visualDraftRef.current) {
      draftDroppedAt.current = performance.now();
    }
    visualDraftRef.current = changed;
    setVisualDraft(changed);
  };
  /** Undo or redo, from Visual or the title bar: the editor's history, the same as in Source. */
  const visualHistory = (step: "undo" | "redo") => {
    // After any edit still being made, as a field's that pressing the button committed.
    void applyVisualEdit.after(() => {
      editor.current?.[step]();
      immediateTree.current = editor.current?.text() ?? null;
      // The step can take the text back to what Visual's tree already describes, as Undo does with a
      // field open, whose edit it commits and takes back at once. No newer tree comes then, so Visual
      // gets its tree again, and drops the value it shows as pending.
      setVisualRevision((revision) => revision + 1);
    });
  };
  // The editor's cursor, carried into Visual until a tree of the same text can say what it is on.
  const cursorToSelect = useRef<number | null>(null);
  // The selected node's start, carried into Source until the editor is shown again.
  const cursorToPlace = useRef<number | null>(null);
  // The node the cursor selected on entering Visual: leaving with it still selected leaves the cursor be.
  const selectedOnEntry = useRef<string | undefined>(undefined);
  /** Shows a value Visual cannot edit in Source, with the cursor at its start. */
  const openInSource = (detail: NxOpenDetail) => {
    const { text } = reading;
    if (text !== null && text === sourceRef.current) {
      cursorToPlace.current = offsetAtPosition(text, detail.range.start);
    }
    setSelection(detail.key);
    setMode("source");
  };

  // The named values a property can take, asked of the text whose tree Visual's keys are from.
  const readingText = useRef<string | null>(null);
  readingText.current = reading.text;
  const namedValuesOf = useMemo(
    () => (key: string, property: string) => {
      const text = readingText.current;
      return text === null
        ? Promise.resolve([])
        : languageService.namedValues({ documents: [{ uri: MODEL_URI, source: text }], uri: MODEL_URI, key, property });
    },
    [],
  );

  useEffect(() => {
    const offset = cursorToSelect.current;
    const { tree, text } = reading;
    if (mode === "source" || offset === null || tree === null || text === null || text !== source) {
      return;
    }
    cursorToSelect.current = null;
    const key = keyAtRange(tree, text, offset);
    selectedOnEntry.current = key;
    setSelection(key);
  }, [mode, reading.tree, reading.text, source]);

  useEffect(() => {
    const offset = cursorToPlace.current;
    if (mode === "source" && offset !== null) {
      cursorToPlace.current = null;
      editor.current?.placeCursor(offset);
    }
  }, [mode]);

  const switchTo = (next: SourceMode) => {
    if (next === mode) {
      return;
    }
    if (next === "visual") {
      cursorToSelect.current = editor.current?.cursor() ?? null;
      selectedOnEntry.current = undefined;
    } else {
      const { tree, text } = reading;
      const moved = selection !== selectedOnEntry.current;
      if (moved && selection !== undefined && tree !== null && text !== null && text === source) {
        cursorToPlace.current = rangeOfKey(tree, text, selection)?.start ?? null;
      }
    }
    setMode(next);
  };

  const navigate = (detail: NxValueNavigateDetail) => {
    const current = sourceRef.current;
    if (current === null || current !== evaluation.outcomeSource) {
      return;
    }
    const name = declaredName(detail.node, current);
    if (name !== null) {
      editor.current?.select(name.start, name.end);
    }
  };

  /** Selects where a preview's record was built, or a node's declaration when it was not. */
  const navigatePreview = (detail: NxValueNavigateDetail, built: boolean) => {
    const current = sourceRef.current;
    if (current === null || current !== preview.state?.source) {
      return;
    }
    const span = detail.declaration;
    const range = built
      ? {
          start: offsetOf(current, span.startLine, span.startColumn),
          end: offsetOf(current, span.endLine, span.endColumn),
        }
      : declaredName(detail.node, current);
    if (range !== null) {
      editor.current?.select(range.start, range.end);
    }
  };

  // While Preview shows, the editor also marks where a failed step or start points.
  const previewDiagnostics = view === "preview" ? previewMarks(preview.state, preview.notice) : [];
  const marked =
    previewDiagnostics.length > 0 && preview.state?.source === evaluation.diagnosticsSource
      ? [...evaluation.diagnostics, ...previewDiagnostics]
      : evaluation.diagnostics;

  return (
    <div className="app">
      <SiteHeader theme={theme} />
      <div className="toolbar">
        <label className="examples">
          <span className="examples-label">Examples</span>
          <select
            value={example?.id ?? ""}
            onChange={(event) => {
              const chosen = exampleById(event.target.value);
              if (chosen !== undefined) {
                chooseExample(chosen);
              }
            }}
          >
            {example === undefined && (
              <option value="" disabled>
                Edited
              </option>
            )}
            {EXAMPLE_GROUPS.map((group) => (
              <optgroup key={group.topic} label={group.topic}>
                {group.examples.map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.title}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </label>
        {example !== undefined && (
          <a className="docs-link" href={docsUrl(example)}>
            About this in the docs
          </a>
        )}
        <span className="spacer" />
        <button type="button" className="share" onClick={() => void share()} disabled={source === null}>
          {shareState === "copied" ? "Link copied" : shareState === "failed" ? "Could not copy" : "Share"}
        </button>
        <a className="fiddle-link" href="https://fiddle.drawnui.net">
          Draw with NX: DrawnUI fiddle
        </a>
      </div>
      {notice !== null && (
        <div className="notice" role="status">
          <span>{notice}</span>
          <button type="button" aria-label="Dismiss" onClick={() => setNotice(null)}>
            ×
          </button>
        </div>
      )}
      <main className="panes">
        <section className="pane pane-source" aria-label="NX">
          <div className="pane-title">
            <span>NX</span>
            <span className="spacer" />
            <span className="history-actions">
              <button
                type="button"
                aria-label="Undo"
                title={`Undo (${SHORTCUTS.undo})`}
                disabled={!canUndo}
                onPointerDown={(event) => {
                  undoPressedAt.current = event.timeStamp;
                }}
                onClick={() => {
                  const dropped = draftDroppedAt.current >= undoPressedAt.current;
                  undoPressedAt.current = Number.POSITIVE_INFINITY;
                  if (!dropped) {
                    visualHistory("undo");
                  }
                }}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M9 14 4 9l5-5" />
                  <path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11" />
                </svg>
              </button>
              <button
                type="button"
                aria-label="Redo"
                title={`Redo (${SHORTCUTS.redo})`}
                disabled={!history.canRedo}
                onClick={() => visualHistory("redo")}
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="m15 14 5-5-5-5" />
                  <path d="M20 9H9.5A5.5 5.5 0 0 0 4 14.5A5.5 5.5 0 0 0 9.5 20H13" />
                </svg>
              </button>
            </span>
            {mode === "visual" && (
              <span className="fold-actions">
                <button type="button" aria-label="Collapse all" title="Collapse all" onClick={() => readingCommands.current?.collapseAll()}>
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m7 20 5-5 5 5" />
                    <path d="m7 4 5 5 5-5" />
                  </svg>
                  <span>Collapse all</span>
                </button>
                <button type="button" aria-label="Expand all" title="Expand all" onClick={() => readingCommands.current?.expandAll()}>
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m7 15 5 5 5-5" />
                    <path d="m7 9 5-5 5 5" />
                  </svg>
                  <span>Expand all</span>
                </button>
              </span>
            )}
            <span className="mode-switch" role="group" aria-label="Show the source as">
              {(["visual", "source"] as const).map((option) => (
                <button
                  key={option}
                  type="button"
                  aria-pressed={mode === option}
                  onClick={() => switchTo(option)}
                >
                  {option === "source" ? "Source" : "Visual"}
                </button>
              ))}
            </span>
          </div>
          {mode === "visual" && (
            <ReadingView
              reading={reading}
              selection={selection}
              onSelect={setSelection}
              folded={folded}
              onFold={(keys, kept) => {
                setFolded(keys);
                setSelection(kept);
              }}
              commands={readingCommands}
              onEdit={(detail) => {
                // The field's change was committed, not dropped: Undo takes back its edit.
                draftDroppedAt.current = Number.NEGATIVE_INFINITY;
                setEditsInFlight((count) => count + 1);
                void applyVisualEdit(detail).finally(() => setEditsInFlight((count) => count - 1));
              }}
              onDraft={onVisualDraft}
              onOpen={openInSource}
              namedValues={namedValuesOf}
              onUndo={() => visualHistory("undo")}
              onRedo={() => visualHistory("redo")}
              revision={visualRevision}
            />
          )}
          {/* The editor stays mounted while hidden, keeping its undo history and scroll position. */}
          <div className="editor" hidden={mode === "visual"}>
            {source !== null && (
              <NxEditor
                ref={editor}
                value={source}
                onChange={(next) => {
                  address.edited(next);
                  setSource(next);
                }}
                diagnostics={marked}
                diagnosticsSource={evaluation.diagnosticsSource}
                theme={theme}
                onHistory={setHistory}
              />
            )}
          </div>
          <Diagnostics diagnostics={evaluation.diagnostics} evaluating={evaluation.evaluating} />
        </section>
        <OutputPane
          evaluation={evaluation}
          view={view}
          onViewChange={setView}
          preview={
            <PreviewView
              preview={preview}
              doesNotCompile={evaluation.outcomeSource === null && evaluation.diagnostics.length > 0}
              stale={previewStale}
              describe={describePreview}
              onNavigate={navigatePreview}
            />
          }
          describe={describe}
          onNavigate={navigate}
        />
      </main>
    </div>
  );
}

/**
 * The hover at a node's declaration, from the language service, while the editor still holds the
 * source whose spans the node carries.
 */
async function hoverAt(node: NxValueNode, current: string | null, spansSource: string | null) {
  if (node.declaration === undefined || current === null || current !== spansSource) {
    return undefined;
  }
  const name = declaredName(node, current);
  if (name === null) {
    return undefined;
  }
  const hover = await languageService.hover({
    documents: [{ uri: MODEL_URI, source: current }],
    uri: MODEL_URI,
    position: positionAt(current, name.start),
  });
  return hover?.contents;
}

/** The diagnostics of the preview's failed start, step or reload that point into the source. */
function previewMarks(state: PreviewState | null, notice: PreviewReloadNotice | null): Diagnostic[] {
  const problem = state?.problem;
  const diagnostics = [
    ...(problem?.kind === "startFailed" || problem?.kind === "dispatchFailed" || problem?.kind === "programFailed"
      ? problem.diagnostics
      : []),
    ...(notice === null ? [] : notice.diagnostics),
  ];
  return diagnostics.filter((diagnostic) => diagnostic.span !== null);
}

/** The compile diagnostics, listed under the editor as well as marked in it. */
function Diagnostics({
  diagnostics,
  evaluating,
}: {
  diagnostics: readonly Diagnostic[];
  evaluating: boolean;
}) {
  const compile = diagnostics.filter((diagnostic) => diagnostic.code !== "runtime-error" && diagnostic.code !== "nx-text-unspellable");
  if (compile.length === 0) {
    return null;
  }
  return (
    <ul className={`diagnostics${evaluating ? " evaluating" : ""}`} aria-label="Problems">
      {compile.map((diagnostic, index) => (
        <li key={index} className={diagnostic.severity === "warning" ? "warning" : "error"}>
          {diagnostic.span !== null && (
            <span className="where">
              {diagnostic.span.startLine}:{diagnostic.span.startColumn}
            </span>
          )}
          <span className="what">{diagnostic.message}</span>
        </li>
      ))}
    </ul>
  );
}
