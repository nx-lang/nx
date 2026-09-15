/**
 * Registration against a Monaco the host loaded through the AMD loader and exposes as a global,
 * shaped like Monaco 0.52: a plain object whose `editor` and `languages` carry only the members
 * that release has. BlazorMonaco 3.3 hands the DrawnUI fiddle exactly such a namespace.
 *
 * Nothing here is imported from `monaco-editor`; the namespace is whatever the host says it is.
 */
import assert from "node:assert/strict";
import { test } from "node:test";
import type { NxLanguageService } from "@nx-lang/language-protocol";
import type * as Monaco from "monaco-editor";
import { registerNxLanguage, setNxMarkers, type MonacoNamespace } from "../src/index.js";
import { cancellationToken, createFakeMonaco, fakeModel } from "./fake-monaco.js";

const URI = "inmemory://editor.nx";

/** The members of `monaco.editor` and `monaco.languages` that Monaco 0.52.2 exports and this package calls. */
const MONACO_0_52_MEMBERS = {
  editor: ["create", "defineTheme", "setTheme", "setModelMarkers"],
  languages: [
    "register",
    "getLanguages",
    "setLanguageConfiguration",
    "setTokensProvider",
    "registerHoverProvider",
    "registerCompletionItemProvider",
    "CompletionItemKind",
  ],
} as const;

/**
 * A global namespace as the AMD loader leaves it: only the members 0.52 has. Reading any other
 * member throws, so an API the integration reaches for beyond that set fails here rather than in a
 * host. Writing is allowed, as on the real global: the Shiki bridge replaces `editor.setTheme` and
 * `editor.create` with wrappers.
 */
function amdGlobal() {
  const fake = createFakeMonaco();
  const source = fake.namespace as unknown as Record<"editor" | "languages", Record<string, unknown>>;
  const pick = (part: "editor" | "languages") => {
    const members: string[] = [...MONACO_0_52_MEMBERS[part]];
    const picked = Object.fromEntries(
      members.map((member) => {
        assert.ok(member in source[part], `the fake has ${part}.${member}`);
        return [member, source[part][member]];
      }),
    );
    return new Proxy(picked, {
      get(target, property, receiver) {
        if (typeof property === "string" && !members.includes(property)) {
          throw new Error(`monaco.${part}.${property} does not exist in Monaco 0.52`);
        }
        return Reflect.get(target, property, receiver);
      },
    });
  };
  const namespace = { editor: pick("editor"), languages: pick("languages") };
  (globalThis as { monaco?: unknown }).monaco = namespace;
  return { fake, namespace: namespace as unknown as MonacoNamespace };
}

test("registers against a 0.52-shaped global namespace and serves hover, completion and markers through it", async () => {
  const { fake, namespace } = amdGlobal();
  try {
    const requests: string[] = [];
    const service: NxLanguageService = {
      hover: async (request) => {
        requests.push("hover");
        return {
          uri: request.uri,
          identity: "editor.nx",
          version: 1,
          range: { start: { line: 0, character: 1 }, end: { line: 0, character: 6 }, startByte: 1, endByte: 6 },
          contents: "```nx\nexternal component <Label Text:string />\n```",
        };
      },
      completions: async (request) => {
        requests.push("completions");
        return { uri: request.uri, identity: "editor.nx", version: 1, items: [{ label: "Label", kind: "Component", detail: null }] };
      },
      diagnostics: async () => ({ documents: [], workspace: [] }),
      documentSymbols: async () => [],
    };

    const registration = registerNxLanguage((globalThis as unknown as { monaco: MonacoNamespace }).monaco, {
      service,
      onError: (error) => {
        throw error;
      },
    });
    await registration.ready;
    assert.equal(fake.languages.length, 1);
    assert.ok(fake.tokensProviders.has("nx"));

    const model = fakeModel(URI, '<Label Text="hi" />\n');
    const { token } = cancellationToken();

    const hover = (await fake.hoverProviders[0]!.provideHover(model, { lineNumber: 1, column: 3 } as Monaco.Position, token)) as Monaco.languages.Hover;
    assert.deepEqual(hover.range, { startLineNumber: 1, startColumn: 2, endLineNumber: 1, endColumn: 7 });

    const list = (await fake.completionProviders[0]!.provideCompletionItems(
      model,
      { lineNumber: 1, column: 2 } as Monaco.Position,
      { triggerKind: 1, triggerCharacter: "<" } as Monaco.languages.CompletionContext,
      token,
    )) as Monaco.languages.CompletionList;
    assert.equal(list.suggestions.length, 1);
    assert.equal(list.suggestions[0]!.kind, namespace.languages.CompletionItemKind.Constructor);

    setNxMarkers(namespace, model, [
      { severity: "Error", code: null, message: "expected }", range: { start: { line: 0, character: 19 }, end: { line: 0, character: 19 }, startByte: 19, endByte: 19 } },
    ]);
    assert.equal(fake.markers.length, 1);
    assert.equal(fake.markers[0]!.markers[0]!.startColumn, 20);

    assert.deepEqual(requests, ["hover", "completions"]);
    registration.dispose();
  } finally {
    delete (globalThis as { monaco?: unknown }).monaco;
  }
});
