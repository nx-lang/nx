/**
 * Enough of the Monaco namespace to register a language, define themes, set a tokens provider,
 * and hold providers, recording every call so a test can see what the integration did.
 */
import type * as Monaco from "monaco-editor";
import type { MonacoNamespace } from "../src/index.js";

export interface FakeMonaco {
  namespace: MonacoNamespace;
  languages: string[];
  configurations: Monaco.languages.LanguageConfiguration[];
  tokensProviders: Map<string, Monaco.languages.TokensProvider>;
  themes: Map<string, Monaco.editor.IStandaloneThemeData>;
  hoverProviders: Monaco.languages.HoverProvider[];
  completionProviders: Monaco.languages.CompletionItemProvider[];
  markers: { owner: string; markers: Monaco.editor.IMarkerData[] }[];
  currentTheme: string | undefined;
}

export function createFakeMonaco(): FakeMonaco {
  const fake: FakeMonaco = {
    namespace: undefined as unknown as MonacoNamespace,
    languages: [],
    configurations: [],
    tokensProviders: new Map(),
    themes: new Map(),
    hoverProviders: [],
    completionProviders: [],
    markers: [],
    currentTheme: undefined,
  };
  const remove = <T>(list: T[], item: T): Monaco.IDisposable => ({
    dispose: () => {
      const index = list.indexOf(item);
      if (index >= 0) {
        list.splice(index, 1);
      }
    },
  });
  const CompletionItemKind = {
    Method: 0, Function: 1, Constructor: 2, Field: 3, Variable: 4, Class: 5, Struct: 6, Interface: 7,
    Module: 8, Property: 9, Event: 10, Operator: 11, Unit: 12, Value: 13, Constant: 14, Enum: 15,
    EnumMember: 16, Keyword: 17, Text: 18, Color: 19, File: 20, Reference: 21, Customcolor: 22,
    Folder: 23, TypeParameter: 24, User: 25, Issue: 26, Snippet: 27,
  };
  fake.namespace = {
    languages: {
      register: (definition: Monaco.languages.ILanguageExtensionPoint) => {
        fake.languages.push(definition.id);
      },
      getLanguages: () => fake.languages.map((id) => ({ id })),
      setLanguageConfiguration: (_id: string, configuration: Monaco.languages.LanguageConfiguration) => {
        fake.configurations.push(configuration);
        return remove(fake.configurations, configuration);
      },
      setTokensProvider: (id: string, provider: Monaco.languages.TokensProvider) => {
        fake.tokensProviders.set(id, provider);
        return { dispose: () => fake.tokensProviders.delete(id) };
      },
      registerHoverProvider: (_id: string, provider: Monaco.languages.HoverProvider) => {
        fake.hoverProviders.push(provider);
        return remove(fake.hoverProviders, provider);
      },
      registerCompletionItemProvider: (_id: string, provider: Monaco.languages.CompletionItemProvider) => {
        fake.completionProviders.push(provider);
        return remove(fake.completionProviders, provider);
      },
      CompletionItemKind,
    },
    editor: {
      defineTheme: (name: string, data: Monaco.editor.IStandaloneThemeData) => {
        fake.themes.set(name, data);
      },
      setTheme: (name: string) => {
        fake.currentTheme = name;
      },
      create: () => {
        throw new Error("the fake cannot create editors");
      },
      setModelMarkers: (_model: unknown, owner: string, markers: Monaco.editor.IMarkerData[]) => {
        fake.markers.push({ owner, markers });
      },
    },
  } as unknown as MonacoNamespace;
  return fake;
}

/** A text model over one string, with the parts of the interface the providers read. */
export function fakeModel(uri: string, text: string, version = 1): Monaco.editor.ITextModel {
  const lines = text.split("\n");
  return {
    uri: { toString: () => uri },
    getValue: () => text,
    getVersionId: () => version,
    getWordUntilPosition: (position: Monaco.IPosition) => {
      const line = lines[position.lineNumber - 1] ?? "";
      const before = line.slice(0, position.column - 1);
      const match = /[A-Za-z0-9_]*$/.exec(before);
      const word = match?.[0] ?? "";
      return { word, startColumn: position.column - word.length, endColumn: position.column };
    },
  } as unknown as Monaco.editor.ITextModel;
}

/** A cancellation token a test can fire. */
export function cancellationToken(): { token: Monaco.CancellationToken; cancel: () => void } {
  const listeners: (() => void)[] = [];
  let requested = false;
  const token = {
    get isCancellationRequested() {
      return requested;
    },
    onCancellationRequested: (listener: () => void) => {
      listeners.push(listener);
      return { dispose: () => undefined };
    },
  } as unknown as Monaco.CancellationToken;
  return {
    token,
    cancel: () => {
      requested = true;
      for (const listener of listeners) {
        listener();
      }
    },
  };
}
