import { useEffect, useRef } from "react";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import type { Diagnostic } from "../compile";
import { NX_LANGUAGE_ID, registerNxLanguage } from "./nxLanguage";

// Monaco expects to be told where its workers live; Vite supplies them as module workers.
self.MonacoEnvironment = { getWorker: () => new editorWorker() };

const languageReady = registerNxLanguage();

export interface NxEditorProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
  readonly diagnostics: readonly Diagnostic[];
}

/** The source pane: Monaco, the repository's NX grammar, and markers for the author's own errors. */
export function NxEditor({ value, onChange, diagnostics }: NxEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const latestChange = useRef(onChange);
  latestChange.current = onChange;

  useEffect(() => {
    let disposed = false;
    let subscription: monaco.IDisposable | undefined;
    void languageReady.then(() => {
      if (disposed || host.current === null) {
        return;
      }
      const instance = monaco.editor.create(host.current, {
        value,
        language: NX_LANGUAGE_ID,
        theme: "nx-dark",
        automaticLayout: true,
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        fontSize: 13,
        tabSize: 2,
        renderLineHighlight: "none",
      });
      editor.current = instance;
      subscription = instance.onDidChangeModelContent(() => {
        latestChange.current(instance.getValue());
      });
    });
    return () => {
      disposed = true;
      subscription?.dispose();
      editor.current?.dispose();
      editor.current = null;
    };
    // The editor owns its text after creation; `value` is only the starting point.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // A fiddle loaded from the gallery replaces the whole document.
  useEffect(() => {
    const instance = editor.current;
    if (instance !== null && instance.getValue() !== value) {
      instance.setValue(value);
    }
  }, [value]);

  useEffect(() => {
    const model = editor.current?.getModel();
    if (model === null || model === undefined) {
      return;
    }
    monaco.editor.setModelMarkers(
      model,
      "nx",
      // Only diagnostics that point at the author's own source are marked. A catalog or
      // whole-program fault has no honest position in this document.
      diagnostics
        .filter((diagnostic) => diagnostic.origin === "source" && diagnostic.span !== null)
        .map((diagnostic) => {
          const span = diagnostic.span!;
          // An insertion point has no width — `Expected } here` names the column a token belongs
          // before — and a marker with no width draws nothing. Widening an empty span by one
          // column is a presentation detail; the span itself stays exact.
          const empty = span.startLine === span.endLine && span.startColumn === span.endColumn;
          return {
            message: diagnostic.message,
            severity:
              diagnostic.severity === "warning"
                ? monaco.MarkerSeverity.Warning
                : monaco.MarkerSeverity.Error,
            startLineNumber: span.startLine,
            startColumn: span.startColumn,
            endLineNumber: span.endLine,
            endColumn: empty ? span.endColumn + 1 : span.endColumn,
          };
        }),
    );
  }, [diagnostics]);

  return <div ref={host} style={{ height: "100%", width: "100%" }} />;
}
