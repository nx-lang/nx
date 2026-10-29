import type { ReactNode } from "react";
import type {
  NxValueDescribe,
  NxValueNavigateDetail,
} from "@nx-lang/value-view";
import type { Diagnostic } from "../compile";
import {
  outputErrorHeading,
  outputNotice,
  type Evaluation,
} from "../evaluation.ts";
import { SessionOutput } from "../preview/PreviewView.tsx";
import type { PreviewRenderedOutput } from "../preview/protocol.ts";
import type { SourceSelection } from "../preview/selection.ts";

export type OutputView = "output" | "preview";

export interface OutputPaneProps {
  readonly evaluation: Evaluation;
  readonly onRetry: () => void;
  readonly view: OutputView;
  readonly available: boolean;
  readonly onViewChange: (view: OutputView) => void;
  readonly preview: ReactNode;
  readonly rendered: PreviewRenderedOutput | null;
  readonly sourceSelection: SourceSelection | null;
  readonly describe: NxValueDescribe;
  readonly onNavigate: (detail: NxValueNavigateDetail, built: boolean) => void;
}

/** A registered preview and the universal structured view of the same current value. */
export function OutputPane({
  evaluation,
  onRetry,
  view,
  available,
  onViewChange,
  preview,
  rendered,
  sourceSelection,
  describe,
  onNavigate,
}: OutputPaneProps) {
  const { outcome, failure, stale } = evaluation;
  const notice = outputNotice(evaluation);
  return (
    <section
      className="pane pane-output"
      aria-label={view === "output" ? "Output" : "Preview"}
    >
      <div className="pane-title">
        <span>{view === "output" ? "Output" : "Preview"}</span>
        <span className="pane-note">
          {evaluation.evaluating
            ? "updating…"
            : view === "output"
              ? "evaluated value"
              : "rendered output"}
        </span>
        <span className="spacer" />
        {available && (
          <span className="mode-switch" role="group" aria-label="Show">
            {(["preview", "output"] as const).map((option) => (
              <button
                key={option}
                type="button"
                aria-pressed={view === option}
                onClick={() => onViewChange(option)}
              >
                {option === "output" ? "Output" : "Preview"}
              </button>
            ))}
          </span>
        )}
      </div>
      <div className="output-body" hidden={view !== "preview"}>
        {preview}
      </div>
      <div className="output-body" hidden={view !== "output"}>
        {failure !== null && (
          <div className="output-message fault" role="alert">
            <p>{failure}</p>
            <button type="button" onClick={onRetry}>
              Start again
            </button>
          </div>
        )}
        {notice === "evaluating" && (
          <p className="output-message quiet">Evaluating…</p>
        )}
        {notice === "doesNotCompile" && (
          <p className="output-message">
            The program does not compile yet. The problems are listed under the
            source.
          </p>
        )}
        {outcome?.kind === "noRoot" && (
          <p className={`output-message${stale ? " stale" : ""}`}>
            End the program with a value or declare <code>let root() = …</code>{" "}
            to see its output.
          </p>
        )}
        {outcome?.kind === "error" && (
          <ErrorMessage diagnostics={outcome.diagnostics} stale={stale} />
        )}
        {rendered !== null && outcome?.kind === "value" && (
          <SessionOutput
            session={rendered}
            sourceSelection={sourceSelection}
            stale={stale}
            describe={describe}
            onPress={() => {}}
            onNavigate={onNavigate}
            interactive={false}
          />
        )}
      </div>
    </section>
  );
}

function ErrorMessage({
  diagnostics,
  stale,
}: {
  diagnostics: readonly Diagnostic[];
  stale: boolean;
}) {
  return (
    <div
      className={`output-message error${stale ? " stale" : ""}`}
      role="alert"
    >
      <p className="output-heading">{outputErrorHeading(diagnostics)}</p>
      {diagnostics.map((diagnostic, index) => (
        <p key={index}>
          {diagnostic.span !== null && (
            <span className="where">
              {diagnostic.span.startLine}:{diagnostic.span.startColumn}
            </span>
          )}
          {diagnostic.message}
        </p>
      ))}
    </div>
  );
}
