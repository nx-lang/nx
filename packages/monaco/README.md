# @nx-lang/monaco

One Monaco integration for NX, shared by every NX web editor: the `nx` language and its
configuration from `@nx-lang/language`, highlighting through [Shiki](https://shiki.style) with that
package's TextMate grammar, hover and completion over an `NxLanguageService`, and a mapping from
protocol diagnostics to editor markers. Framework-free — a host composes it with
`monaco.editor.create` or `@monaco-editor/react` as it likes.

```ts
import * as monaco from "monaco-editor";
import { registerNxLanguage } from "@nx-lang/monaco";
import { createHttpLanguageService } from "@nx-lang/language-client";

const registration = registerNxLanguage(monaco, {
  service: createHttpLanguageService({ baseUrl: "/api/language" }),
});

const editor = monaco.editor.create(container, {
  value: source,
  language: "nx",
  theme: "github-dark",
});
```

## Peer dependencies

The host installs `monaco-editor`, `shiki`, `@shikijs/monaco`, and `@nx-lang/language`, so there is
one Monaco on the page and the host controls versions. Until `@nx-lang/language` is on the
registry, a host consuming this repository links it by path, the way this repository's workspace
does (`"@nx-lang/language": "file:../../src/vscode"`).

## `registerNxLanguage(monaco, options?)`

Registers NX with the given Monaco namespace and returns a handle with `dispose()` and a `ready`
promise that resolves once the highlighter is loaded.

| Option | Default | Meaning |
| --- | --- | --- |
| `service` | none | An `NxLanguageService`. With one, hover and completion providers are registered; without, only highlighting and language configuration. |
| `workspace` | the model alone | `(model) => LanguageDocument[]`: the document set a query about `model` is asked against. |
| `themes` | `["github-light", "github-dark"]` | Shiki themes to load and define in Monaco, by bundled name or theme object. Pick one with the editor's `theme` option or `monaco.editor.setTheme`; without a pick, the first becomes active once the highlighter loads. |
| `onError` | none | Called when a service call rejects or the highlighter fails to load. Nothing is thrown into Monaco. |

**Idempotent.** A repeat call on the same Monaco namespace returns a handle onto the existing
registration rather than registering again, so `@monaco-editor/react`'s `beforeMount` and `onMount`
pair, and React strict mode's double mount, register one set of providers and every completion
appears once. A call without a service registers highlighting only; the first later call that
brings a service adds the hover and completion providers to the existing registration, so a host
that sets up highlighting before its service is ready still gets them. Providers are removed when
the last handle is disposed; the language, its configuration and the highlighter stay, because
Monaco has no way to unregister a language, and a registration made after that reuses them rather
than loading the highlighter again. A React effect that registers on mount and disposes on cleanup
therefore costs one highlighter load, however often it runs.

**Themes.** The highlighter loads after `registerNxLanguage` returns. A theme chosen before `ready`
resolves, through `monaco.editor.setTheme` or an editor's `theme` option, is kept once it has, so
neither example above needs to await `ready` for its theme to hold. A theme that is not one of
`themes` stays the editor's theme, but the highlighter cannot recolor for it and stays on its first.

**Positions.** Monaco's one-based line and column become the protocol's zero-based line and UTF-16
character, and answer ranges come back the other way. The model's text is read at the moment of
the query, so an edit followed by an immediate hover asks about the edited text. Monaco's
cancellation token aborts the service call.

**Hover code blocks** are highlighted by Monaco's own colorizer through the tokens provider Shiki
installs, with the same grammar as the buffer.

## With `@monaco-editor/react`

```tsx
import Editor, { type BeforeMount, type OnMount } from "@monaco-editor/react";
import { registerNxLanguage, setNxMarkers } from "@nx-lang/monaco";
import { createHttpLanguageService } from "@nx-lang/language-client";

const service = createHttpLanguageService({
  baseUrl: "/api/language",
  headers: async () => ({ authorization: `Bearer ${await getToken()}` }),
});

// A multi-file configuration: the model being edited plus its siblings, with unsaved text.
const beforeMount: BeforeMount = (monaco) => {
  registerNxLanguage(monaco, {
    service,
    workspace: (model) => [
      { uri: model.uri.toString(), source: model.getValue(), version: model.getVersionId() },
      ...otherFiles.map((file) => ({ uri: file.uri, source: file.draftText })),
    ],
    onError: (error) => console.warn("nx language", error),
  });
};

const onMount: OnMount = (editor, monaco) => {
  const model = editor.getModel();
  if (model) {
    setNxMarkers(monaco, model, diagnosticsForThisFile);
  }
};

<Editor language="nx" theme="github-dark" beforeMount={beforeMount} onMount={onMount} value={text} />;
```

## Markers

`toMonacoMarkers(diagnostics)` converts protocol diagnostics — or anything with `severity`,
`message`, `code` and a `range` that may be `null` — to `IMarkerData[]`: severity mapped to
Monaco's, columns one-based, a zero-width range widened by one column so an insertion-point
diagnostic (`expected } here`) stays visible, and a range-less diagnostic omitted. `setNxMarkers(monaco,
model, diagnostics, owner = "nx")` sets them on a model.

## Exports

`registerNxLanguage`, `NX_LANGUAGE_ID`, `NX_DEFAULT_LIGHT_THEME`, `NX_DEFAULT_DARK_THEME`,
`NX_COMPLETION_TRIGGER_CHARACTERS`, `defaultWorkspace`, `toMonacoLanguageConfiguration`,
`nxShikiLanguage`, `toProtocolPosition`, `toMonacoRange`, `toMonacoCompletionKind`,
`toMonacoMarkers`, `setNxMarkers`, and the types `MonacoNamespace`, `RegisterNxLanguageOptions`,
`NxLanguageRegistration`, `NxMonacoTheme`, `MarkerDiagnostic`, `NxLanguageService`.
