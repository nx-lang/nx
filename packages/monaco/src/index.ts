/**
 * The one Monaco integration for NX.
 *
 * Registers the `nx` language and its configuration from `@nx-lang/language`, highlights through
 * Shiki with that package's grammar, and — given an `NxLanguageService` — offers hover and
 * completion. Framework-free: a host composes it with `monaco.editor.create` or
 * `@monaco-editor/react` as it likes.
 */
import nxGrammar from "@nx-lang/language/grammar" with { type: "json" };
import nxLanguageConfiguration from "@nx-lang/language/language-configuration" with { type: "json" };
import {
  isAbortError,
  type CompletionItemKind,
  type DiagnosticSeverity,
  type EditorDiagnostic,
  type EditorRange,
  type LanguageDocument,
  type NxLanguageService,
} from "@nx-lang/language-protocol";
import type * as Monaco from "monaco-editor";
import type { BundledTheme, LanguageInput, ThemeInput } from "shiki";

export type { NxLanguageService } from "@nx-lang/language-protocol";

/**
 * The Monaco namespace, as `import * as monaco from "monaco-editor"` or `@monaco-editor/react`
 * hands it over.
 *
 * Only `editor` and `languages` are named, so a host on a Monaco whose top-level namespace differs
 * from the one this package was compiled against — an older release without the `lsp` or `css`
 * namespaces, say — still type-checks.
 */
export type MonacoNamespace = Pick<typeof Monaco, "editor" | "languages">;

/** The language id the integration registers. */
export const NX_LANGUAGE_ID = "nx";
/** The theme loaded for light hosts when no themes are given. */
export const NX_DEFAULT_LIGHT_THEME = "github-light";
/** The theme loaded for dark hosts when no themes are given. */
export const NX_DEFAULT_DARK_THEME = "github-dark";

/** A Shiki theme to load: a bundled theme's name, or a theme object. */
export type NxMonacoTheme = BundledTheme | ThemeInput;

/** What `registerNxLanguage` accepts. */
export interface RegisterNxLanguageOptions {
  /** Answers hover and completion. Without it, only highlighting and language configuration are registered. */
  service?: NxLanguageService;
  /**
   * The document set a query about `model` is asked against. Defaults to the model alone, with its
   * Monaco URI as the logical URI and its version id as the document version. A host editing one
   * file of a multi-file configuration returns the siblings here too.
   */
  workspace?: (model: Monaco.editor.ITextModel) => LanguageDocument[];
  /** Shiki themes to load and define in Monaco. Default `github-light` and `github-dark`. */
  themes?: readonly NxMonacoTheme[];
  /** Called when a service call rejects, or the highlighter fails to load. Never thrown into Monaco. */
  onError?: (error: unknown) => void;
}

/** What `registerNxLanguage` returns. */
export interface NxLanguageRegistration extends Monaco.IDisposable {
  /** Resolves once the highlighter is loaded and Monaco's tokens provider and themes are set. */
  readonly ready: Promise<void>;
}

interface Language {
  /** Resolves once the highlighter is loaded and Monaco's tokens provider and themes are set. */
  ready: Promise<void>;
}

interface Registration {
  references: number;
  providers: Monaco.IDisposable[];
}

// The language, its configuration and the highlighter are set up once per Monaco namespace and
// never taken down: Monaco has no way to unregister a language, and the Shiki bridge wraps
// `monaco.editor.setTheme` and `create` on every call, so loading it twice would stack wrappers.
const languages = new WeakMap<MonacoNamespace, Language>();

// The providers are reference counted per namespace, however many times a host asks.
// `@monaco-editor/react` calls `beforeMount` and `onMount` on the same instance, and React's strict
// mode mounts twice; a second registration would answer every completion twice.
const registrations = new WeakMap<MonacoNamespace, Registration>();

/**
 * Registers NX with `monaco`.
 *
 * Idempotent per Monaco instance: a repeat call returns a handle onto the existing registration.
 * A registration made without a service is highlighting only; the first later call that brings a
 * service adds the hover and completion providers to it, so a host that sets up highlighting before
 * its service is available is not stuck without them. Disposing every handle removes the providers;
 * the language, its configuration, and the highlighter stay, since Monaco has no way to unregister
 * a language, and a registration made after that reuses them rather than loading them again.
 *
 * A theme the host chooses before `ready` resolves, through `monaco.editor.setTheme` or an
 * editor's `theme` option, is kept once the highlighter is in place. Without a choice, the first
 * of `themes` becomes active.
 */
export function registerNxLanguage(
  monaco: MonacoNamespace,
  options: RegisterNxLanguageOptions = {},
): NxLanguageRegistration {
  let language = languages.get(monaco);
  if (language === undefined) {
    registerLanguage(monaco);
    const ready = loadHighlighter(monaco, options.themes ?? [NX_DEFAULT_LIGHT_THEME, NX_DEFAULT_DARK_THEME]).catch(
      (error: unknown) => {
        options.onError?.(error);
      },
    );
    language = { ready };
    languages.set(monaco, language);
  }

  let registration = registrations.get(monaco);
  if (registration === undefined) {
    registration = { references: 0, providers: [] };
    registrations.set(monaco, registration);
  }
  if (registration.providers.length === 0 && options.service !== undefined) {
    registration.providers.push(...registerProviders(monaco, options.service, options));
  }
  registration.references += 1;

  let disposed = false;
  const current = registration;
  return {
    ready: language.ready,
    dispose() {
      if (disposed) {
        return;
      }
      disposed = true;
      current.references -= 1;
      if (current.references > 0) {
        return;
      }
      for (const provider of current.providers.splice(0)) {
        provider.dispose();
      }
      registrations.delete(monaco);
    },
  };
}

// ---------------------------------------------------------------------------------------------
// Language and highlighting
// ---------------------------------------------------------------------------------------------

function registerLanguage(monaco: MonacoNamespace): void {
  if (!monaco.languages.getLanguages().some((language) => language.id === NX_LANGUAGE_ID)) {
    monaco.languages.register({ id: NX_LANGUAGE_ID, aliases: ["NX"], extensions: [".nx"] });
  }
  monaco.languages.setLanguageConfiguration(NX_LANGUAGE_ID, toMonacoLanguageConfiguration());
}

/** The published language configuration in Monaco's shape: patterns become `RegExp`s, pairs become tuples. */
export function toMonacoLanguageConfiguration(): Monaco.languages.LanguageConfiguration {
  const configuration = nxLanguageConfiguration;
  const pair = ([open, close]: readonly string[]): Monaco.languages.CharacterPair => [open ?? "", close ?? ""];
  return {
    comments: {
      lineComment: configuration.comments.lineComment,
      blockComment: pair(configuration.comments.blockComment),
    },
    brackets: configuration.brackets.map(pair),
    autoClosingPairs: configuration.autoClosingPairs,
    surroundingPairs: configuration.surroundingPairs,
    colorizedBracketPairs: configuration.colorizedBracketPairs.map(pair),
    wordPattern: new RegExp(configuration.wordPattern),
    indentationRules: {
      increaseIndentPattern: new RegExp(configuration.indentationRules.increaseIndentPattern),
      decreaseIndentPattern: new RegExp(configuration.indentationRules.decreaseIndentPattern),
    },
    folding: configuration.folding,
  };
}

/** The published grammar as a Shiki language, under the id Monaco knows. */
export function nxShikiLanguage(): LanguageInput {
  return { ...(nxGrammar as object), name: NX_LANGUAGE_ID, aliases: ["NX"] } as LanguageInput;
}

async function loadHighlighter(monaco: MonacoNamespace, themes: readonly NxMonacoTheme[]): Promise<void> {
  // The bridge ends by activating the first theme it defined, which would silently replace a theme
  // the host picked while the highlighter was loading. Watch what the host picks until the bridge
  // is about to install, then put the choice back once it has.
  const choice = watchThemeChoice(monaco);
  let install: () => readonly string[];
  try {
    install = await prepareHighlighter(monaco, themes);
  } finally {
    choice.stop();
  }
  const loaded = install();
  const chosen = choice.theme();
  if (chosen === undefined) {
    return;
  }
  if (loaded.includes(chosen)) {
    monaco.editor.setTheme(chosen);
  } else {
    // Not one of the highlighter's themes, so the bridge cannot recolor for it: the highlighter
    // stays on its first theme and the editor gets back the theme the host asked for.
    choice.setThemeDirectly(chosen);
  }
}

interface ThemeChoice {
  /** The last theme the host asked for while watched, if any. */
  theme(): string | undefined;
  /** Ends the watch, restoring `setTheme` and `create` when nothing else wrapped them meanwhile. */
  stop(): void;
  /** Monaco's own `setTheme`, as it was before anyone wrapped it. */
  setThemeDirectly(theme: string): void;
}

/**
 * Records the host's theme choices: calls to `monaco.editor.setTheme`, and the `theme` option of
 * `monaco.editor.create`, which Monaco applies internally without going through `setTheme`.
 */
function watchThemeChoice(monaco: MonacoNamespace): ThemeChoice {
  const editor = monaco.editor;
  const originalSetTheme = editor.setTheme;
  const originalCreate = editor.create;
  let chosen: string | undefined;
  const watchingSetTheme: typeof editor.setTheme = (theme) => {
    chosen = theme;
    return originalSetTheme.call(editor, theme);
  };
  const watchingCreate: typeof editor.create = (element, options, override) => {
    if (typeof options?.theme === "string") {
      chosen = options.theme;
    }
    return originalCreate.call(editor, element, options, override);
  };
  editor.setTheme = watchingSetTheme;
  editor.create = watchingCreate;
  return {
    theme: () => chosen,
    stop: () => {
      if (editor.setTheme === watchingSetTheme) {
        editor.setTheme = originalSetTheme;
      }
      if (editor.create === watchingCreate) {
        editor.create = originalCreate;
      }
    },
    setThemeDirectly: (theme) => originalSetTheme.call(editor, theme),
  };
}

/**
 * Loads Shiki and builds the highlighter, and returns the synchronous step that installs it into
 * Monaco. Installing is separate so the caller can stop watching theme choices just before the
 * bridge wraps `setTheme` and `create`, with no gap in which a choice could be missed.
 */
async function prepareHighlighter(
  monaco: MonacoNamespace,
  themes: readonly NxMonacoTheme[],
): Promise<() => readonly string[]> {
  // Shiki's core rather than its full bundle: the full bundle's registry references every
  // bundled language, which a bundler turns into a chunk per language for one grammar that is
  // supplied here directly. Themes still come from the bundled set, loaded by name on demand.
  const [{ createHighlighterCore }, { createOnigurumaEngine }, { bundledThemes }, { shikiToMonaco }] =
    await Promise.all([
      import("shiki/core"),
      import("shiki/engine/oniguruma"),
      import("shiki/themes"),
      import("@shikijs/monaco"),
    ]);
  const themeInputs: ThemeInput[] = themes.map((theme) => {
    if (typeof theme !== "string") {
      return theme;
    }
    const bundled = bundledThemes[theme];
    if (bundled === undefined) {
      throw new Error(`'${theme}' is not a Shiki bundled theme.`);
    }
    return bundled;
  });
  const highlighter = await createHighlighterCore({
    themes: themeInputs,
    langs: [nxShikiLanguage()],
    engine: createOnigurumaEngine(import("shiki/wasm")),
  });
  return () => {
    // The bridge is typed against monaco-editor-core; the namespace is a superset of it.
    shikiToMonaco(highlighter, monaco as unknown as Parameters<typeof shikiToMonaco>[1]);
    return highlighter.getLoadedThemes();
  };
}

// ---------------------------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------------------------

/** The completion trigger characters, matching the NX language server's advertisement. */
export const NX_COMPLETION_TRIGGER_CHARACTERS: readonly string[] = ["<", ":"];

function registerProviders(
  monaco: MonacoNamespace,
  service: NxLanguageService,
  options: RegisterNxLanguageOptions,
): Monaco.IDisposable[] {
  const workspace = options.workspace ?? defaultWorkspace;
  const report = (error: unknown): void => {
    if (!isAbortError(error)) {
      options.onError?.(error);
    }
  };

  const hover = monaco.languages.registerHoverProvider(NX_LANGUAGE_ID, {
    async provideHover(model, position, token) {
      const controller = abortWith(token);
      try {
        const answer = await service.hover(
          { documents: workspace(model), uri: model.uri.toString(), position: toProtocolPosition(position) },
          controller.signal,
        );
        if (answer === null) {
          return null;
        }
        return { contents: [{ value: answer.contents }], range: toMonacoRange(answer.range) };
      } catch (error) {
        report(error);
        return null;
      }
    },
  });

  const completion = monaco.languages.registerCompletionItemProvider(NX_LANGUAGE_ID, {
    triggerCharacters: [...NX_COMPLETION_TRIGGER_CHARACTERS],
    async provideCompletionItems(model, position, _context, token) {
      const controller = abortWith(token);
      try {
        const answer = await service.completions(
          { documents: workspace(model), uri: model.uri.toString(), position: toProtocolPosition(position) },
          controller.signal,
        );
        const word = model.getWordUntilPosition(position);
        const range: Monaco.IRange = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };
        return {
          suggestions: answer.items.map((item) => ({
            label: item.label,
            kind: toMonacoCompletionKind(monaco, item.kind),
            insertText: item.label,
            range,
            ...(item.detail === null ? {} : { detail: item.detail }),
          })),
        };
      } catch (error) {
        report(error);
        return { suggestions: [] };
      }
    },
  });

  return [hover, completion];
}

/** The model alone: its Monaco URI as the logical URI, its version id as the version. */
export function defaultWorkspace(model: Monaco.editor.ITextModel): LanguageDocument[] {
  return [{ uri: model.uri.toString(), source: model.getValue(), version: model.getVersionId() }];
}

function abortWith(token: Monaco.CancellationToken): AbortController {
  const controller = new AbortController();
  if (token.isCancellationRequested) {
    controller.abort();
  } else {
    token.onCancellationRequested(() => controller.abort());
  }
  return controller;
}

// ---------------------------------------------------------------------------------------------
// Coordinates and kinds
// ---------------------------------------------------------------------------------------------

/** Monaco's one-based line and column to the protocol's zero-based line and UTF-16 character. */
export function toProtocolPosition(position: Monaco.IPosition): { line: number; character: number } {
  return { line: position.lineNumber - 1, character: position.column - 1 };
}

/** A protocol range to Monaco's one-based range. */
export function toMonacoRange(range: EditorRange): Monaco.IRange {
  return {
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: range.end.character + 1,
  };
}

const completionKinds: Record<CompletionItemKind, keyof typeof Monaco.languages.CompletionItemKind> = {
  Keyword: "Keyword",
  Type: "Class",
  Declaration: "Variable",
  Component: "Constructor",
  Property: "Property",
  Member: "EnumMember",
};

/** A protocol completion kind to Monaco's. */
export function toMonacoCompletionKind(
  monaco: MonacoNamespace,
  kind: CompletionItemKind,
): Monaco.languages.CompletionItemKind {
  return monaco.languages.CompletionItemKind[completionKinds[kind] ?? "Text"];
}

// ---------------------------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------------------------

/** A diagnostic as a marker maps it: a protocol diagnostic, or one whose range is unknown. */
export type MarkerDiagnostic = Pick<EditorDiagnostic, "severity" | "message" | "code"> & {
  range: EditorRange | null;
};

// Monaco's `MarkerSeverity` values, spelled out so a host that has not loaded the namespace yet
// can still map diagnostics.
const markerSeverities: Record<DiagnosticSeverity, Monaco.MarkerSeverity> = {
  Hint: 1 as Monaco.MarkerSeverity,
  Info: 2 as Monaco.MarkerSeverity,
  Warning: 4 as Monaco.MarkerSeverity,
  Error: 8 as Monaco.MarkerSeverity,
};

/**
 * Protocol diagnostics as Monaco markers.
 *
 * Severity maps to Monaco's; columns become one-based; a zero-width range is widened by one column
 * so an insertion-point diagnostic (`expected } here`) draws; a diagnostic without a range is
 * omitted, since a marker with no honest position would be a lie about where the problem is.
 */
export function toMonacoMarkers(diagnostics: readonly MarkerDiagnostic[]): Monaco.editor.IMarkerData[] {
  const markers: Monaco.editor.IMarkerData[] = [];
  for (const diagnostic of diagnostics) {
    if (diagnostic.range === null) {
      continue;
    }
    const range = toMonacoRange(diagnostic.range);
    const empty = range.startLineNumber === range.endLineNumber && range.startColumn === range.endColumn;
    markers.push({
      message: diagnostic.message,
      severity: markerSeverities[diagnostic.severity],
      startLineNumber: range.startLineNumber,
      startColumn: range.startColumn,
      endLineNumber: range.endLineNumber,
      endColumn: empty ? range.endColumn + 1 : range.endColumn,
      ...(diagnostic.code === null ? {} : { code: diagnostic.code }),
    });
  }
  return markers;
}

/** Sets `diagnostics` as the markers `owner` holds on `model`. */
export function setNxMarkers(
  monaco: MonacoNamespace,
  model: Monaco.editor.ITextModel,
  diagnostics: readonly MarkerDiagnostic[],
  owner = "nx",
): void {
  monaco.editor.setModelMarkers(model, owner, toMonacoMarkers(diagnostics));
}
