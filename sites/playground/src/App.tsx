import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { NxValueDescribe, NxValueNavigateDetail } from "@nx-lang/value-view";
import "./app.css";
import { evaluateInBrowser, type Diagnostic } from "./compile";
import { NxEditor, languageService, MODEL_URI, type NxEditorHandle } from "./editor/NxEditor";
import { declaredName, positionAt } from "./editor/positions.ts";
import {
  DEFAULT_EXAMPLE,
  EXAMPLE_GROUPS,
  docsUrl,
  exampleById,
  exampleWithSource,
  type Example,
} from "./examples";
import { SiteHeader } from "./header/SiteHeader";
import { OutputPane } from "./output/OutputPane";
import { SITE_ROOT } from "./paths.ts";
import { createAddressKeeper } from "./address.ts";
import { pathForPayload, routeFromLocation } from "./routes.ts";
import { decodeSource, encodeSource } from "./share/codec.ts";
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

  /** The output's hover: the language service's hover at the node's declaration, while it lines up. */
  const describe = useCallback<NxValueDescribe>(
    async (node) => {
      const current = sourceRef.current;
      if (node.declaration === undefined || current === null || current !== evaluationSourceRef.current) {
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
    },
    [],
  );

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
          </div>
          <div className="editor">
            {source !== null && (
              <NxEditor
                ref={editor}
                value={source}
                onChange={(next) => {
                  address.edited(next);
                  setSource(next);
                }}
                diagnostics={evaluation.diagnostics}
                diagnosticsSource={evaluation.diagnosticsSource}
                theme={theme}
              />
            )}
          </div>
          <Diagnostics diagnostics={evaluation.diagnostics} evaluating={evaluation.evaluating} />
        </section>
        <OutputPane evaluation={evaluation} describe={describe} onNavigate={navigate} />
      </main>
    </div>
  );
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
