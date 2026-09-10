import { useEffect, useRef } from "react";
import * as monaco from "monaco-editor";
// Monaco 0.56 maps `monaco-editor/<path>.js` onto `esm/vs/<path>.js` through its exports map.
import editorWorker from "monaco-editor/editor/editor.worker.js?worker";
import { createHttpLanguageService } from "@nx-lang/language-client";
import { NX_LANGUAGE_ID, registerNxLanguage } from "@nx-lang/monaco";
import type { Diagnostic } from "../compile";
import { SkiaEditor } from "../drawnui/index";
import { API_ROOT } from "../paths";

// Monaco expects to be told where its workers live; Vite supplies them as module workers.
self.MonacoEnvironment = { getWorker: () => new editorWorker() };

/** The one document the playground edits, under the logical URI the language route sees it by. */
const MODEL_URI = monaco.Uri.parse("nx://playground/playground.nx");
const THEME = "github-dark";

/**
 * Highlighting, hover and completion all come from the shared Monaco integration: the grammar is
 * the repository's published one, and hover and completion are answered by the language route the
 * compile server mounts beside the compile route. The route being unreachable is the compile server
 * being down, which the compile pane already reports, so here the providers only fall silent.
 */
const registration = registerNxLanguage(monaco, {
  service: createHttpLanguageService({ baseUrl: `${API_ROOT}/language` }),
  themes: [THEME],
  onError: (error) => console.debug("nx language", error),
});

export interface NxEditorProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
  readonly diagnostics: readonly Diagnostic[];
}

/** The source pane: Monaco, the shared NX integration, and markers for the author's own errors. */
export function NxEditor({ value, onChange, diagnostics }: NxEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const latestChange = useRef(onChange);
  latestChange.current = onChange;

  useEffect(() => {
    let disposed = false;
    let subscription: monaco.IDisposable | undefined;
    let focusSubscription: monaco.IDisposable | undefined;
    void registration.ready.then(() => {
      if (disposed || host.current === null) {
        return;
      }
      const model =
        monaco.editor.getModel(MODEL_URI) ?? monaco.editor.createModel(value, NX_LANGUAGE_ID, MODEL_URI);
      const instance = monaco.editor.create(host.current, {
        model,
        theme: THEME,
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
      // A focused drawn editor (SkiaEditor, in the output pane) takes every keystroke at window
      // level, and DrawnUI lets go of it only when an INPUT, TEXTAREA or contenteditable takes DOM
      // focus. Monaco edits through an EditContext host, which is none of those, so typing into
      // the source pane would land in the drawn editor instead. Monaco taking focus is the signal
      // that the drawn editor should give it up. See FINDINGS.md F23.
      focusSubscription = instance.onDidFocusEditorText(() => {
        if (SkiaEditor.Focused !== undefined) {
          SkiaEditor.Focused.IsFocused = false;
        }
      });
    });
    return () => {
      disposed = true;
      subscription?.dispose();
      focusSubscription?.dispose();
      const model = editor.current?.getModel();
      editor.current?.dispose();
      model?.dispose();
      editor.current = null;
    };
    // The editor owns its text after creation; `value` is only the starting point.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // An example opened from the gallery replaces the whole document.
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
