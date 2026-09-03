import * as monaco from "monaco-editor";
import { INITIAL, Registry, type IRawGrammar, type IToken, type StateStack } from "vscode-textmate";
import { createOnigScanner, createOnigString, loadWASM } from "vscode-oniguruma";
import onigWasmUrl from "vscode-oniguruma/release/onig.wasm?url";
import nxGrammar from "../../../../src/vscode/syntaxes/nx.tmLanguage.json";

export const NX_LANGUAGE_ID = "nx";
const NX_SCOPE = "source.nx";

/**
 * Highlighting comes from the repository's own TextMate grammar, loaded directly rather than
 * copied, so the fiddle and the VS Code extension cannot drift apart. Monaco has no TextMate
 * support of its own, so the grammar is bridged through a tokens provider: each line is tokenized
 * by `vscode-textmate` and its scopes are handed to Monaco as token names. Monaco matches theme
 * rules by prefix, so a rule for `keyword` colors every `keyword.*` scope the grammar emits.
 */
export async function registerNxLanguage(): Promise<void> {
  if (monaco.languages.getLanguages().some((language) => language.id === NX_LANGUAGE_ID)) {
    return;
  }
  monaco.languages.register({ id: NX_LANGUAGE_ID, extensions: [".nx"] });
  monaco.languages.setLanguageConfiguration(NX_LANGUAGE_ID, {
    comments: { lineComment: "//" },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
  });

  await loadWASM(await fetch(onigWasmUrl));
  const registry = new Registry({
    onigLib: Promise.resolve({ createOnigScanner, createOnigString }),
    loadGrammar: async (scope) => (scope === NX_SCOPE ? (nxGrammar as unknown as IRawGrammar) : null),
  });
  const grammar = await registry.loadGrammar(NX_SCOPE);
  if (grammar === null) {
    return;
  }

  monaco.languages.setTokensProvider(NX_LANGUAGE_ID, {
    getInitialState: () => INITIAL as unknown as monaco.languages.IState,
    tokenize: (line, state) => {
      const result = grammar.tokenizeLine(line, state as unknown as StateStack);
      return {
        tokens: result.tokens.map((token: IToken) => ({
          startIndex: token.startIndex,
          // The most specific scope is the last one, and it is the one worth coloring.
          scopes: token.scopes[token.scopes.length - 1] ?? "",
        })),
        endState: result.ruleStack as unknown as monaco.languages.IState,
      };
    },
  });

  monaco.editor.defineTheme("nx-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6c757d", fontStyle: "italic" },
      { token: "keyword", foreground: "ff8fa3" },
      { token: "storage", foreground: "ff8fa3" },
      { token: "string", foreground: "20c997" },
      { token: "constant", foreground: "ffd93d" },
      { token: "entity.name", foreground: "74c0fc" },
      { token: "support", foreground: "74c0fc" },
      { token: "variable", foreground: "e9ecef" },
      { token: "punctuation", foreground: "868e96" },
    ],
    colors: { "editor.background": "#1b1f24" },
  });
}
