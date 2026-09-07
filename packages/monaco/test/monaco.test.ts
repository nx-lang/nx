import assert from "node:assert/strict";
import { test } from "node:test";
import type { CompletionsRequest, HoverRequest, NxLanguageService } from "@nx-lang/language-protocol";
import type * as Monaco from "monaco-editor";
import {
  NX_LANGUAGE_ID,
  defaultWorkspace,
  registerNxLanguage,
  toMonacoLanguageConfiguration,
  toMonacoMarkers,
  toProtocolPosition,
} from "../src/index.js";
import { cancellationToken, createFakeMonaco, fakeModel } from "./fake-monaco.js";

const FORM = "inmemory://model/1";

const range = { start: { line: 1, character: 1 }, end: { line: 1, character: 6 }, startByte: 10, endByte: 15 };

/** An in-process service that records requests and answers from a script. */
function fakeService(overrides: Partial<NxLanguageService> = {}): NxLanguageService & { requests: unknown[] } {
  const requests: unknown[] = [];
  const record = <T>(answer: T) =>
    async (request: unknown, signal?: AbortSignal): Promise<T> => {
      requests.push({ request, signal });
      signal?.throwIfAborted();
      return answer;
    };
  return {
    requests,
    hover: record({ uri: FORM, identity: "model/1", version: 1, range, contents: "```nx\n<Panel />\n```" }),
    completions: record({
      uri: FORM,
      identity: "model/1",
      version: 1,
      items: [
        { label: "mode", kind: "Property" as const, detail: "mode:Mode" },
        { label: "Panel", kind: "Component" as const, detail: null },
      ],
    }),
    diagnostics: record({ documents: [], workspace: [] }),
    documentSymbols: record([]),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------------------------

test("registers the language, its published configuration, and the Shiki highlighter", async () => {
  const fake = createFakeMonaco();
  const registration = registerNxLanguage(fake.namespace);
  await registration.ready;

  assert.deepEqual(fake.languages, [NX_LANGUAGE_ID]);
  assert.equal(fake.configurations.length, 1);
  assert.equal(fake.configurations[0]!.comments?.lineComment, "//");
  assert.ok(fake.configurations[0]!.wordPattern instanceof RegExp);
  assert.deepEqual([...fake.themes.keys()], ["github-light", "github-dark"]);
  assert.ok(fake.tokensProviders.has(NX_LANGUAGE_ID), "tokens provider set for nx");
  assert.equal(fake.hoverProviders.length, 0, "no service, no hover provider");
  assert.equal(fake.completionProviders.length, 0, "no service, no completion provider");

  // The tokens provider tokenizes with the published grammar.
  const provider = fake.tokensProviders.get(NX_LANGUAGE_ID)!;
  const tokens = provider.tokenize("let <Panel /> = <div />", provider.getInitialState());
  assert.ok(tokens.tokens.length > 1, JSON.stringify(tokens.tokens));
});

test("loads the themes the host asks for", async () => {
  const fake = createFakeMonaco();
  await registerNxLanguage(fake.namespace, { themes: ["nord"] }).ready;
  assert.deepEqual([...fake.themes.keys()], ["nord"]);
  assert.equal(fake.currentTheme, "nord");
});

test("two registrations register once, and providers go when the last handle is disposed", async () => {
  const fake = createFakeMonaco();
  const service = fakeService();
  const first = registerNxLanguage(fake.namespace, { service });
  const second = registerNxLanguage(fake.namespace, { service });
  await first.ready;

  assert.deepEqual(fake.languages, [NX_LANGUAGE_ID]);
  assert.equal(fake.hoverProviders.length, 1);
  assert.equal(fake.completionProviders.length, 1);

  // What Monaco would collect: each item once.
  const { token } = cancellationToken();
  const model = fakeModel(FORM, "<Panel  />\n");
  const lists = await Promise.all(
    fake.completionProviders.map((provider) =>
      provider.provideCompletionItems(model, { lineNumber: 1, column: 8 } as Monaco.Position, { triggerKind: 0 } as Monaco.languages.CompletionContext, token),
    ),
  );
  const labels = lists.flatMap((list) => (list as Monaco.languages.CompletionList).suggestions.map((item) => item.label));
  assert.deepEqual(labels, ["mode", "Panel"]);

  first.dispose();
  first.dispose();
  assert.equal(fake.hoverProviders.length, 1, "the second handle still holds the providers");
  second.dispose();
  assert.equal(fake.hoverProviders.length, 0);
  assert.equal(fake.completionProviders.length, 0);

  // A later registration on the same namespace re-adds providers without re-registering the
  // language, re-setting its configuration, or loading the highlighter again (which would wrap
  // `setTheme` and `create` a second time).
  const setThemeAfterReady = fake.namespace.editor.setTheme;
  const createAfterReady = fake.namespace.editor.create;
  const third = registerNxLanguage(fake.namespace, { service });
  await third.ready;
  assert.deepEqual(fake.languages, [NX_LANGUAGE_ID]);
  assert.equal(fake.hoverProviders.length, 1);
  assert.equal(fake.configurations.length, 1, "the configuration is set once");
  assert.equal(fake.namespace.editor.setTheme, setThemeAfterReady, "the highlighter is not loaded again");
  assert.equal(fake.namespace.editor.create, createAfterReady);
  third.dispose();
});

test("a theme chosen before ready survives the highlighter loading", async () => {
  // Through `setTheme`, one of the highlighter's themes.
  const viaSetTheme = createFakeMonaco();
  const first = registerNxLanguage(viaSetTheme.namespace);
  viaSetTheme.namespace.editor.setTheme("github-dark");
  assert.equal(viaSetTheme.currentTheme, "github-dark");
  await first.ready;
  assert.equal(viaSetTheme.currentTheme, "github-dark");

  // Through `create`'s `theme` option, which Monaco applies without calling `setTheme`.
  const viaCreate = createFakeMonaco();
  viaCreate.namespace.editor.create = (() => ({})) as unknown as typeof viaCreate.namespace.editor.create;
  const second = registerNxLanguage(viaCreate.namespace);
  viaCreate.namespace.editor.create({} as HTMLElement, { theme: "github-dark" });
  await second.ready;
  assert.equal(viaCreate.currentTheme, "github-dark");

  // A theme the highlighter does not know stays the editor's theme too.
  const builtIn = createFakeMonaco();
  const third = registerNxLanguage(builtIn.namespace);
  builtIn.namespace.editor.setTheme("vs-dark");
  await third.ready;
  assert.equal(builtIn.currentTheme, "vs-dark");

  // The watch is gone once ready: only the bridge's wrapper remains, and it still applies themes.
  viaSetTheme.namespace.editor.setTheme("github-light");
  assert.equal(viaSetTheme.currentTheme, "github-light");
});

test("a highlighting-only registration gains providers from a later call that brings a service", async () => {
  const fake = createFakeMonaco();
  const highlighting = registerNxLanguage(fake.namespace);
  assert.equal(fake.hoverProviders.length, 0);
  assert.equal(fake.completionProviders.length, 0);

  const service = fakeService();
  const withService = registerNxLanguage(fake.namespace, { service });
  assert.deepEqual(fake.languages, [NX_LANGUAGE_ID], "the language is still registered once");
  assert.equal(fake.hoverProviders.length, 1);
  assert.equal(fake.completionProviders.length, 1);

  // The providers answer from the service the second call brought.
  const model = fakeModel(FORM, "<Panel />\n");
  const hover = await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 1, column: 2 } as Monaco.Position, cancellationToken().token);
  assert.ok(hover !== null);
  assert.equal(service.requests.length, 1);

  // A third call with a service adds nothing more.
  const again = registerNxLanguage(fake.namespace, { service: fakeService() });
  assert.equal(fake.hoverProviders.length, 1);

  highlighting.dispose();
  withService.dispose();
  assert.equal(fake.hoverProviders.length, 1, "the third handle still holds the providers");
  again.dispose();
  assert.equal(fake.hoverProviders.length, 0);
  assert.equal(fake.completionProviders.length, 0);
});

test("a highlighter failure is reported, not thrown", async () => {
  const fake = createFakeMonaco();
  const errors: unknown[] = [];
  const registration = registerNxLanguage(fake.namespace, {
    themes: ["no-such-theme" as never],
    onError: (error) => errors.push(error),
  });
  await registration.ready;
  assert.equal(errors.length, 1);
  registration.dispose();
});

// ---------------------------------------------------------------------------------------------
// Hover
// ---------------------------------------------------------------------------------------------

test("hover converts positions, sends the model's current text, and renders the answer", async () => {
  const fake = createFakeMonaco();
  const service = fakeService();
  const registration = registerNxLanguage(fake.namespace, { service });
  const model = fakeModel(FORM, "let a = 1\n<Panel />\n", 7);
  const { token } = cancellationToken();

  const hover = (await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 2, column: 3 } as Monaco.Position, token)) as Monaco.languages.Hover;

  const request = (service.requests[0] as { request: HoverRequest }).request;
  assert.deepEqual(request, {
    documents: [{ uri: FORM, source: "let a = 1\n<Panel />\n", version: 7 }],
    uri: FORM,
    position: { line: 1, character: 2 },
  });
  assert.deepEqual(hover.contents, [{ value: "```nx\n<Panel />\n```" }]);
  assert.deepEqual(hover.range, { startLineNumber: 2, startColumn: 2, endLineNumber: 2, endColumn: 7 });
  registration.dispose();
});

test("hover is silent where the service is, and a failing service reports rather than throws", async () => {
  const fake = createFakeMonaco();
  const errors: unknown[] = [];
  const service = fakeService({ hover: async () => null });
  const registration = registerNxLanguage(fake.namespace, { service, onError: (error) => errors.push(error) });
  const model = fakeModel(FORM, "let a = 1\n");
  const { token } = cancellationToken();

  assert.equal(await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 1, column: 1 } as Monaco.Position, token), null);
  registration.dispose();

  const failing = registerNxLanguage(fake.namespace, {
    service: fakeService({ hover: async () => { throw new Error("route down"); } }),
    onError: (error) => errors.push(error),
  });
  assert.equal(await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 1, column: 1 } as Monaco.Position, token), null);
  assert.equal(errors.length, 1);
  assert.match(String(errors[0]), /route down/);
  failing.dispose();
});

test("cancelling Monaco's token aborts the service call, and the abort is not an error", async () => {
  const fake = createFakeMonaco();
  const errors: unknown[] = [];
  let seenSignal: AbortSignal | undefined;
  const service = fakeService({
    hover: (_request, signal) =>
      new Promise((_, reject) => {
        seenSignal = signal;
        signal?.addEventListener("abort", () => reject(signal.reason));
      }),
  });
  const registration = registerNxLanguage(fake.namespace, { service, onError: (error) => errors.push(error) });
  const { token, cancel } = cancellationToken();
  const pending = fake.hoverProviders[0]!.provideHover(fakeModel(FORM, ""), { lineNumber: 1, column: 1 } as Monaco.Position, token);
  cancel();
  assert.equal(await pending, null);
  assert.ok(seenSignal?.aborted);
  assert.equal(errors.length, 0);
  registration.dispose();
});

// ---------------------------------------------------------------------------------------------
// Completion
// ---------------------------------------------------------------------------------------------

test("completions carry trigger characters, map kinds, and insert over the word being typed", async () => {
  const fake = createFakeMonaco();
  const service = fakeService();
  const registration = registerNxLanguage(fake.namespace, { service });
  const provider = fake.completionProviders[0]!;
  assert.deepEqual(provider.triggerCharacters, ["<", ":"]);

  const model = fakeModel(FORM, "<Panel mo />\n");
  const { token } = cancellationToken();
  const list = (await provider.provideCompletionItems(model, { lineNumber: 1, column: 10 } as Monaco.Position, { triggerKind: 0 } as Monaco.languages.CompletionContext, token)) as Monaco.languages.CompletionList;

  const request = (service.requests[0] as { request: CompletionsRequest }).request;
  assert.deepEqual(request.position, { line: 0, character: 9 });
  assert.equal(list.suggestions.length, 2);
  const [mode, panel] = list.suggestions as [Monaco.languages.CompletionItem, Monaco.languages.CompletionItem];
  assert.equal(mode.label, "mode");
  assert.equal(mode.kind, fake.namespace.languages.CompletionItemKind.Property);
  assert.equal(mode.detail, "mode:Mode");
  assert.equal(mode.insertText, "mode");
  assert.deepEqual(mode.range, { startLineNumber: 1, endLineNumber: 1, startColumn: 8, endColumn: 10 });
  assert.equal(panel.kind, fake.namespace.languages.CompletionItemKind.Constructor);
  assert.equal(panel.detail, undefined);
  registration.dispose();
});

test("all six protocol kinds map to a Monaco kind", async () => {
  const fake = createFakeMonaco();
  const kinds = ["Keyword", "Type", "Declaration", "Component", "Property", "Member"] as const;
  const service = fakeService({
    completions: async () => ({ uri: FORM, identity: "m", version: null, items: kinds.map((kind) => ({ label: kind, kind, detail: null })) }),
  });
  const registration = registerNxLanguage(fake.namespace, { service });
  const list = (await fake.completionProviders[0]!.provideCompletionItems(fakeModel(FORM, ""), { lineNumber: 1, column: 1 } as Monaco.Position, { triggerKind: 0 } as Monaco.languages.CompletionContext, cancellationToken().token)) as Monaco.languages.CompletionList;
  const mapped = list.suggestions.map((item) => item.kind);
  assert.equal(new Set(mapped).size, kinds.length, "distinct kinds stay distinct");
  assert.ok(mapped.every((kind) => typeof kind === "number"));
  registration.dispose();
});

test("a failing completion reports and offers nothing", async () => {
  const fake = createFakeMonaco();
  const errors: unknown[] = [];
  const registration = registerNxLanguage(fake.namespace, {
    service: fakeService({ completions: async () => { throw new Error("nope"); } }),
    onError: (error) => errors.push(error),
  });
  const list = (await fake.completionProviders[0]!.provideCompletionItems(fakeModel(FORM, ""), { lineNumber: 1, column: 1 } as Monaco.Position, { triggerKind: 0 } as Monaco.languages.CompletionContext, cancellationToken().token)) as Monaco.languages.CompletionList;
  assert.deepEqual(list.suggestions, []);
  assert.equal(errors.length, 1);
  registration.dispose();
});

// ---------------------------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------------------------

test("the default workspace is the model; a host callback supplies siblings", async () => {
  const model = fakeModel("inmemory://model/2", "<Img />", 3);
  assert.deepEqual(defaultWorkspace(model), [{ uri: "inmemory://model/2", source: "<Img />", version: 3 }]);

  const fake = createFakeMonaco();
  const service = fakeService();
  const siblings = [
    { uri: "nx://tenant/a.nx", source: "export let a = 1" },
    { uri: "nx://tenant/b.nx", source: "export let b = 2" },
  ];
  const registration = registerNxLanguage(fake.namespace, {
    service,
    workspace: (current) => [...defaultWorkspace(current), ...siblings],
  });
  await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 1, column: 2 } as Monaco.Position, cancellationToken().token);
  const request = (service.requests[0] as { request: HoverRequest }).request;
  assert.equal(request.documents.length, 3);
  assert.equal(request.uri, "inmemory://model/2");
  assert.deepEqual(request.documents.slice(1), siblings);
  registration.dispose();
});

// ---------------------------------------------------------------------------------------------
// Markers and conversions
// ---------------------------------------------------------------------------------------------

test("diagnostics map to markers: severity, one-based columns, widened insertion points, no range omitted", () => {
  const markers = toMonacoMarkers([
    { severity: "Warning", code: "unused", message: "unused", range: { start: { line: 0, character: 4 }, end: { line: 0, character: 9 }, startByte: 4, endByte: 9 } },
    { severity: "Error", code: null, message: "expected }", range: { start: { line: 2, character: 3 }, end: { line: 2, character: 3 }, startByte: 30, endByte: 30 } },
    { severity: "Info", code: null, message: "whole program", range: null },
    { severity: "Hint", code: "h", message: "hint", range: { start: { line: 1, character: 0 }, end: { line: 1, character: 1 }, startByte: 10, endByte: 11 } },
  ]);
  assert.deepEqual(markers, [
    { message: "unused", severity: 4, startLineNumber: 1, startColumn: 5, endLineNumber: 1, endColumn: 10, code: "unused" },
    { message: "expected }", severity: 8, startLineNumber: 3, startColumn: 4, endLineNumber: 3, endColumn: 5 },
    { message: "hint", severity: 1, startLineNumber: 2, startColumn: 1, endLineNumber: 2, endColumn: 2, code: "h" },
  ]);
  assert.deepEqual(toProtocolPosition({ lineNumber: 3, column: 1 }), { line: 2, character: 0 });
  assert.deepEqual(toMonacoLanguageConfiguration().brackets, [["{", "}"], ["[", "]"], ["(", ")"]]);
});
