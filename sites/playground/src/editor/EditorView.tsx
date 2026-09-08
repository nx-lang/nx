import type { ReactNode } from "react";
import type { Compile, Diagnostic } from "../compile";
import { Canvas } from "../drawnui/react/index";
import { pathForRoute } from "../router";
import { useNxDrawing } from "../render/useNxDrawing";
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

/** The editor view: NX on the left, what it draws on the right. */
export function EditorView({ title, source, onSourceChange, compile, coverage, onBack }: EditorViewProps) {
  const drawing = useNxDrawing(source, compile);
  const failures = drawing.failure === null ? [] : [drawing.failure];
  const unknown = drawing.unknownControls;

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
          Authored interaction is not supported yet — no event handlers, no animation, no state.
          Scrolling and DrawnUI's own gestures still work.
        </span>
      </div>
      <div className="panes">
        <div className="pane-source">
          <div className="editor">
            <NxEditor value={source} onChange={onSourceChange} diagnostics={drawing.diagnostics} />
          </div>
          <div className="diagnostics">
            {drawing.diagnostics.length === 0 && failures.length === 0 && unknown.length === 0 ? (
              <div className="quiet">{drawing.compiling ? "Compiling…" : "No diagnostics."}</div>
            ) : null}
            {drawing.diagnostics.map((diagnostic, index) => (
              <DiagnosticRow key={index} diagnostic={diagnostic} />
            ))}
            {unknown.map((type) => (
              <div className="diagnostic app" key={type}>
                <span className="where">renderer</span>
                <span className="what">No DrawnUI control is registered for &lsquo;{type}&rsquo;.</span>
              </div>
            ))}
            {failures.map((failure) => (
              <div className="diagnostic app" key={failure}>
                <span className="where">app</span>
                <span className="what">{failure}</span>
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
