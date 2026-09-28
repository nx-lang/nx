import { useEffect, useRef } from "react";
import "@nx-lang/value-view";
import {
  NX_VALUE_NAVIGATE_EVENT,
  type NxValueDescribe,
  type NxValueElement,
  type NxValueNavigateDetail,
} from "@nx-lang/value-view";
import type { Diagnostic } from "../compile";
import { outputNotice, type Evaluation } from "../evaluation.ts";

export interface OutputPaneProps {
  readonly evaluation: Evaluation;
  /** The hover for a node of the value, from the language service. */
  readonly describe: NxValueDescribe;
  /** Shows a declaration in the source pane. */
  readonly onNavigate: (detail: NxValueNavigateDetail) => void;
}

/**
 * What `root` returns, shown by `<nx-value>`, or why there is nothing to show.
 *
 * <para>The element loads its own Shiki highlighter rather than sharing the editor's. Coloring with
 * a light and a dark theme switches a highlighter's current theme, and the Monaco bridge tokenizes
 * with whatever theme that is, so a shared one would recolor the source from the wrong palette.
 * </para>
 */
export function OutputPane({ evaluation, describe, onNavigate }: OutputPaneProps) {
  const element = useRef<NxValueElement>(null);
  const latestNavigate = useRef(onNavigate);
  latestNavigate.current = onNavigate;

  useEffect(() => {
    const target = element.current;
    if (target === null) {
      return;
    }
    const listener = (event: Event) =>
      latestNavigate.current((event as CustomEvent<NxValueNavigateDetail>).detail);
    target.addEventListener(NX_VALUE_NAVIGATE_EVENT, listener);
    return () => target.removeEventListener(NX_VALUE_NAVIGATE_EVENT, listener);
  });

  const { outcome, failure, stale } = evaluation;
  const value = outcome?.kind === "value" ? outcome : null;
  const notice = outputNotice(evaluation);

  return (
    <section className="pane pane-output" aria-label="Output">
      <div className="pane-title">
        <span>Output</span>
        <span className="pane-note">
          what <code>root</code> returns
        </span>
      </div>
      <div className="output-body">
        {failure !== null && <p className="output-message fault">{failure}</p>}
        {notice === "evaluating" && <p className="output-message quiet">Evaluating…</p>}
        {notice === "doesNotCompile" && (
          <p className="output-message">
            The program does not compile yet, so there is no value to show. The problems are listed under
            the source.
          </p>
        )}
        {outcome?.kind === "noRoot" && (
          <p className={`output-message${stale ? " stale" : ""}`}>
            There is no <code>root</code> to evaluate. Declare <code>let root() = …</code>, or end the
            program with an element, and what it returns shows here.
          </p>
        )}
        {outcome?.kind === "error" && <ErrorMessage diagnostics={outcome.diagnostics} stale={stale} />}
        {value !== null && (
          <nx-value
            ref={element}
            value={value.value}
            truncated={value.truncated}
            stale={stale}
            describe={describe}
          />
        )}
      </div>
    </section>
  );
}

function ErrorMessage({ diagnostics, stale }: { diagnostics: readonly Diagnostic[]; stale: boolean }) {
  const [first] = diagnostics;
  const unspellable = first?.code === "nx-text-unspellable";
  return (
    <div className={`output-message error${stale ? " stale" : ""}`} role="alert">
      <p className="output-heading">
        {unspellable ? "The value has no NX spelling" : "Evaluating root failed"}
      </p>
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
