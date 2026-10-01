import { useEffect, useImperativeHandle, useRef, useState, type Ref } from "react";
import * as monaco from "monaco-editor";
// Monaco 0.56 maps `monaco-editor/<path>.js` onto `esm/vs/<path>.js` through its exports map.
import editorWorker from "monaco-editor/editor/editor.worker.js?worker";
import { NX_LANGUAGE_ID, registerNxLanguage } from "@nx-lang/monaco";
import type { Diagnostic, DiagnosticSpan } from "../compile";
import { createWorkerLanguageService } from "../language/worker.ts";
import { EDITOR_THEMES, type Theme } from "../theme.ts";
import { offsetOf, positionAt } from "./positions.ts";
import { openSuggestionDetailsOnce } from "./suggestDetails.ts";

// Monaco expects to be told where its workers live; Vite supplies them as module workers.
self.MonacoEnvironment = { getWorker: () => new editorWorker() };

/** The one document the playground edits, under the logical URI the language service sees it by. */
export const MODEL_URI = "nx://playground/playground.nx";


/** The language service the editor's hover and completion use, and the output pane's hover too. */
export const languageService = createWorkerLanguageService();

/**
 * Highlighting, hover and completion all come from the shared Monaco integration: the grammar is
 * the repository's published one, and hover and completion are answered by the same worker and the
 * same host that evaluate the source, so a hover range and a diagnostic land on the same line. A
 * worker that crashed or overran is reported by the output pane, so here the providers only fall
 * silent.
 */
const registration = registerNxLanguage(monaco, {
  service: languageService,
  themes: [EDITOR_THEMES.dark, EDITOR_THEMES.light],
  onError: (error) => console.debug("nx language", error),
});

/** What the view can ask of the editor. */
export interface NxEditorHandle {
  /** Selects `start` to `end`, UTF-16 offsets into the text, scrolls it into view and focuses it. */
  select(start: number, end: number): void;
}

export interface NxEditorProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** Marked in the text: compile diagnostics, and a runtime error's span. */
  readonly diagnostics: readonly Diagnostic[];
  /** The source `diagnostics` were reported against, whose lines and columns their spans count. */
  readonly diagnosticsSource: string | null;
  readonly theme: Theme;
  readonly ref?: Ref<NxEditorHandle>;
}

/** The source pane: Monaco, the shared NX integration, and markers for the visitor's own errors. */
export function NxEditor({ value, onChange, diagnostics, diagnosticsSource, theme, ref }: NxEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const latestChange = useRef(onChange);
  latestChange.current = onChange;
  const latestTheme = useRef(theme);
  latestTheme.current = theme;
  // The editor is created once the highlighter has loaded, by which time the text to show may have
  // moved on from the text there was at mount: an example chosen, or Back pressed, meanwhile.
  const latestValue = useRef(value);
  latestValue.current = value;
  // Setting the text from outside (an example chosen, a link opened) is not an edit.
  const settingValue = useRef(false);

  useImperativeHandle(ref, () => ({
    select(start, end) {
      const instance = editor.current;
      const model = instance?.getModel();
      if (instance === null || instance === undefined || model === null || model === undefined) {
        return;
      }
      const from = model.getPositionAt(start);
      const to = model.getPositionAt(end);
      const range = new monaco.Range(from.lineNumber, from.column, to.lineNumber, to.column);
      instance.setSelection(range);
      instance.revealRangeInCenterIfOutsideViewport(range);
      instance.focus();
    },
  }));

  useEffect(() => {
    let disposed = false;
    let subscription: monaco.IDisposable | undefined;
    let suggestDetails: monaco.IDisposable | undefined;
    void registration.ready.then(() => {
      if (disposed || host.current === null) {
        return;
      }
      const uri = monaco.Uri.parse(MODEL_URI);
      const model =
        monaco.editor.getModel(uri) ?? monaco.editor.createModel(latestValue.current, NX_LANGUAGE_ID, uri);
      const instance = monaco.editor.create(host.current, {
        model,
        automaticLayout: true,
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        fontSize: 13,
        tabSize: 2,
        renderLineHighlight: "none",
        fixedOverflowWidgets: true,
        // Monaco suggests as you type everywhere but comments by default. The language service
        // answers inside a comment only for a doc link being written, so turning it on there offers
        // link names as they are typed and nothing in an ordinary comment — provided Monaco does
        // not fill the silence with the document's own words, which it does wherever a language
        // offers nothing. The language service offers everything worth offering in NX.
        quickSuggestions: { other: "on", comments: "on", strings: "off" },
        wordBasedSuggestions: "off",
      });
      editor.current = instance;
      suggestDetails = openSuggestionDetailsOnce(instance);
      setEditorReady(true);
      // Through `setTheme`, once the highlighter is installed, so the Shiki bridge tokenizes with the
      // theme Monaco draws with. A theme passed to `create` reaches Monaco without the bridge
      // knowing, and a document set later is then colored from the other theme's palette.
      monaco.editor.setTheme(EDITOR_THEMES[latestTheme.current]);
      subscription = instance.onDidChangeModelContent(() => {
        if (!settingValue.current) {
          latestChange.current(instance.getValue());
        }
      });
    });
    return () => {
      disposed = true;
      subscription?.dispose();
      suggestDetails?.dispose();
      const model = editor.current?.getModel();
      editor.current?.dispose();
      model?.dispose();
      editor.current = null;
    };
    // The editor owns its text after creation; `value` is only the starting point.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // An example chosen, or a link opened, replaces the whole document.
  useEffect(() => {
    const instance = editor.current;
    if (instance !== null && instance.getValue() !== value) {
      settingValue.current = true;
      try {
        instance.setValue(value);
      } finally {
        settingValue.current = false;
      }
    }
  }, [value]);

  useEffect(() => {
    if (editor.current !== null) {
      monaco.editor.setTheme(EDITOR_THEMES[theme]);
    }
  }, [theme]);

  // Markers are set when an evaluation reports, positioned in the text it evaluated. Monaco then
  // moves them with every edit, so they are not set again per keystroke: that would put them back
  // at the old line and column until the next evaluation.
  const [editorReady, setEditorReady] = useState(false);
  useEffect(() => {
    const model = editor.current?.getModel();
    if (model === null || model === undefined || diagnosticsSource === null) {
      return;
    }
    monaco.editor.setModelMarkers(
      model,
      "nx",
      // Only diagnostics that point at the visitor's own source are marked. A whole-program fault
      // has no honest position in this document.
      diagnostics
        .filter((diagnostic) => diagnostic.origin === "source" && diagnostic.span !== null)
        .map((diagnostic) => ({
          message: diagnostic.message,
          severity:
            diagnostic.severity === "warning" ? monaco.MarkerSeverity.Warning : monaco.MarkerSeverity.Error,
          ...markerRange(diagnosticsSource, diagnostic.span!),
        })),
    );
  }, [diagnostics, diagnosticsSource, editorReady]);

  return <div ref={host} className="monaco-host" />;
}

/**
 * A span as Monaco's one-based, UTF-16 range, counted in `source`, the text it was reported against.
 * An insertion point has no width — `Expected } here` names the column a token belongs before — and
 * a marker with no width draws nothing, so an empty span is widened by one column. That is a
 * presentation detail; the span itself stays exact.
 */
function markerRange(source: string, span: DiagnosticSpan) {
  const start = positionAt(source, offsetOf(source, span.startLine, span.startColumn));
  const end = positionAt(source, offsetOf(source, span.endLine, span.endColumn));
  const empty = start.line === end.line && start.character === end.character;
  return {
    startLineNumber: start.line + 1,
    startColumn: start.character + 1,
    endLineNumber: end.line + 1,
    endColumn: end.character + (empty ? 2 : 1),
  };
}
