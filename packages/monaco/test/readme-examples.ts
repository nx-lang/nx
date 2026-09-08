/**
 * The README's two examples, typed so they cannot drift from the API. Not executed: Monaco needs
 * a browser. The `@monaco-editor/react` example is typed against the shape of that package's
 * `beforeMount` and `onMount` props rather than the package itself, which this test build does
 * not install.
 */
import type * as Monaco from "monaco-editor";
import type { NxLanguageService } from "@nx-lang/language-protocol";
import { registerNxLanguage, setNxMarkers, type MarkerDiagnostic } from "../src/index.js";

declare const monaco: typeof Monaco;
declare const container: HTMLElement;
declare const source: string;
declare const service: NxLanguageService;
declare const getToken: () => Promise<string>;
declare const otherFiles: { uri: string; draftText: string }[];
declare const diagnosticsForThisFile: MarkerDiagnostic[];

type BeforeMount = (monaco: typeof Monaco) => void;
type OnMount = (editor: Monaco.editor.IStandaloneCodeEditor, monaco: typeof Monaco) => void;

export function directExample(): Monaco.editor.IStandaloneCodeEditor {
  const registration = registerNxLanguage(monaco, { service });
  void registration.ready;
  return monaco.editor.create(container, {
    value: source,
    language: "nx",
    theme: "github-dark",
  });
}

export const beforeMount: BeforeMount = (monaco) => {
  registerNxLanguage(monaco, {
    service,
    workspace: (model) => [
      { uri: model.uri.toString(), source: model.getValue(), version: model.getVersionId() },
      ...otherFiles.map((file) => ({ uri: file.uri, source: file.draftText })),
    ],
    onError: (error) => console.warn("nx language", error),
  });
};

export const onMount: OnMount = (editor, monaco) => {
  const model = editor.getModel();
  if (model) {
    setNxMarkers(monaco, model, diagnosticsForThisFile);
  }
};

void getToken;
