import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { NxValueDescribe, NxValueNavigateDetail, NxValueNode } from "@nx-lang/value-view";
import "./app.css";
import { evaluateInBrowser, type Diagnostic } from "./compile";
import { NxEditor, languageService, MODEL_URI, type NxEditorHandle } from "./editor/NxEditor";
import { declaredName, offsetOf, positionAt } from "./editor/positions.ts";
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
import { ReadingView } from "./reading/ReadingView";
import { keyAtOffset, offsetOfKey } from "./reading/selection.ts";
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

/** Whether the source pane shows the editor or the reading view. */
/** How the source pane shows the source: the editor, or the reading. */
type SourceMode = "edit" | "read";

const MODE_LABELS: Record<SourceMode, string> = { edit: "Edit", read: "Read" };

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
          setSource(opened.source);
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

  const [mode, setMode] = useState<SourceMode>("edit");
  const [selection, setSelection] = useState<string | undefined>(undefined);
  const reading = useSourceTree(source, mode !== "edit");
  // The editor's cursor, carried into Read until a tree of the same text can say what it is on.
  const cursorToSelect = useRef<number | null>(null);
  // The selected node's start, carried into Edit until the editor is shown again.
  const cursorToPlace = useRef<number | null>(null);
  // The node the cursor selected on entering Read: leaving with it still selected leaves the cursor be.
  const selectedOnEntry = useRef<string | undefined>(undefined);

  useEffect(() => {
    const offset = cursorToSelect.current;
    const { tree, text } = reading;
    if (mode === "edit" || offset === null || tree === null || text === null || text !== source) {
      return;
    }
    cursorToSelect.current = null;
    const key = keyAtOffset(tree, text, offset);
    selectedOnEntry.current = key;
    setSelection(key);
  }, [mode, reading.tree, reading.text, source]);

  useEffect(() => {
    const offset = cursorToPlace.current;
    if (mode === "edit" && offset !== null) {
      cursorToPlace.current = null;
      editor.current?.placeCursor(offset);
    }
  }, [mode]);

  const switchTo = (next: SourceMode) => {
    if (next === mode) {
      return;
    }
    if (next !== "edit") {
      cursorToSelect.current = editor.current?.cursor() ?? null;
      selectedOnEntry.current = undefined;
    } else {
      const { tree, text } = reading;
      const moved = selection !== selectedOnEntry.current;
      if (moved && selection !== undefined && tree !== null && text !== null && text === source) {
        cursorToPlace.current = offsetOfKey(tree, text, selection) ?? null;
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
        <section className="pane pane-source" aria-label="Source">
          <div className="pane-title">
            <span>Source</span>
            <span className="pane-note">NX</span>
            <span className="spacer" />
            <span className="mode-switch" role="group" aria-label="Show the source as">
              {(["edit", "read"] as const).map((option) => (
                <button
                  key={option}
                  type="button"
                  aria-pressed={mode === option}
                  onClick={() => switchTo(option)}
                >
                  {MODE_LABELS[option]}
                </button>
              ))}
            </span>
          </div>
          {mode !== "edit" && (
            <ReadingView reading={reading} selection={selection} onSelect={setSelection} />
          )}
          {/* The editor stays mounted while hidden, keeping its undo history and scroll position. */}
          <div className="editor" hidden={mode !== "edit"}>
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
