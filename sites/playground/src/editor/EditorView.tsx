import { useEffect, type ReactNode } from "react";
import type { Compile, Diagnostic } from "../compile";
import { Canvas } from "../drawnui/react/index";
import { pathForRoute } from "../router";
import { useNxDrawing } from "../render/useNxDrawing";
import type { NxObject } from "../render/values";
import { startNxWorker } from "../worker/index.ts";
import { NxEditor } from "./NxEditor";

export interface EditorViewProps {
  readonly title: string;
  readonly source: string;
  readonly onSourceChange: (source: string) => void;
  readonly compile: Compile;
  /** Shown above the source, in the same words the gallery used for this example. */
  readonly coverage?: ReactNode;
  readonly onBack?: () => void;
}

function DiagnosticRow({ diagnostic }: { diagnostic: Diagnostic }) {
  const positioned = diagnostic.origin === "source" && diagnostic.span !== null;
  return (
    <div className={`diagnostic ${positioned ? "error" : "app"}`}>
      <span className="where">
        {positioned
          ? `${diagnostic.span!.startLine}:${diagnostic.span!.startColumn}`
          : diagnostic.origin === "catalog"
            ? "catalog"
            : "program"}
      </span>
      <span className="what">{diagnostic.message}</span>
    </div>
  );
}

/** An action as the visitor wrote it: its name and its fields, `DoSearch search="docs"`. */
function describeAction(action: NxObject): string {
  const fields = Object.entries(action)
    .filter(([name, value]) => name !== "$type" && value !== undefined)
    .map(([name, value]) => `${name}=${JSON.stringify(value)}`);
  return [action.$type ?? "(untyped)", ...fields].join(" ");
}

/** The editor view: NX on the left, what it draws on the right. */
export function EditorView({ title, source, onSourceChange, compile, coverage, onBack }: EditorViewProps) {
  // The compiler module is 2 MB. Fetching and compiling it starts when this view mounts, so the
  // gallery — which never compiles — does not pay for it before its first paint.
  useEffect(startNxWorker, []);

  const drawing = useNxDrawing(source, compile);
  const failures = drawing.failure === null ? [] : [drawing.failure];
  const unknown = drawing.unknownControls;
  const inert = drawing.inertHandlers;
  const effects = drawing.effects;
  const quiet =
    drawing.diagnostics.length === 0 && failures.length === 0 && unknown.length === 0 && inert.length === 0 && effects.length === 0;

  return (
    <div className="app">
      <div className="bar">
        {onBack !== undefined && (
          <a
            className="link"
            // A real address, so the way back works as a link and not only through the handler.
            href={pathForRoute({ kind: "gallery" })}
            onClick={(event) => {
              event.preventDefault();
              onBack();
            }}
          >
            ← Gallery
          </a>
        )}
        <h1>{title}</h1>
        {coverage}
        <span className="spacer" />
        <span className="note">
          Handlers and state run inside components; an action nothing in the tree handles is listed
          below the source.
        </span>
      </div>
      <div className="panes">
        <div className="pane-source">
          <div className="editor">
            <NxEditor value={source} onChange={onSourceChange} diagnostics={drawing.diagnostics} />
          </div>
          <div className="diagnostics">
            {quiet ? <div className="quiet">{drawing.compiling ? "Compiling…" : "No diagnostics."}</div> : null}
            {drawing.diagnostics.map((diagnostic, index) => (
              <DiagnosticRow key={index} diagnostic={diagnostic} />
            ))}
            {unknown.map((type) => (
              <div className="diagnostic app" key={type}>
                <span className="where">renderer</span>
                <span className="what">No DrawnUI control is registered for &lsquo;{type}&rsquo;.</span>
              </div>
            ))}
            {inert.map((where) => (
              <div className="diagnostic app" key={where}>
                <span className="where">renderer</span>
                <span className="what">
                  &lsquo;{where}&rsquo; is bound outside a component, so nothing can run it: handlers run inside
                  a component, and the root function is evaluated, not instantiated.
                </span>
              </div>
            ))}
            {failures.map((failure) => (
              <div className="diagnostic app" key={failure}>
                <span className="where">app</span>
                <span className="what">{failure}</span>
              </div>
            ))}
            {effects.map((effect, index) => (
              <div className="diagnostic effect" key={index}>
                <span className="where">effect</span>
                <span className="what">
                  {describeAction(effect.action)} from &lsquo;{effect.instance}&rsquo;
                </span>
              </div>
            ))}
          </div>
        </div>
        <div className="pane-canvas">
          <Canvas BackgroundColor="#212529" RenderingMode="Accelerated" Gestures="Enabled">
            {drawing.node}
          </Canvas>
        </div>
      </div>
    </div>
  );
}
