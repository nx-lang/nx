// Shared grammar-test infrastructure: grammar loading, scope lookup, and multi-line tokenization.
import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar, IToken, StateStack } from 'vscode-textmate';

const cjsRequire = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const onig: any = cjsRequire('vscode-oniguruma');
const vsctm: any = cjsRequire('vscode-textmate');

export const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.tmLanguage.json');

let wasmLoaded: Promise<void> | null = null;

export function loadOniguruma(): Promise<void> {
  if (!wasmLoaded) {
    const wasmPath = cjsRequire.resolve('vscode-oniguruma/release/onig.wasm');
    const wasmBin = fs.readFileSync(wasmPath).buffer;
    wasmLoaded = onig.loadWASM(wasmBin);
  }
  return wasmLoaded as Promise<void>;
}

/** Loads the Oniguruma WASM binary once per process. */

/** Creates a registry that resolves `source.nx` from the repository's grammar file. */
export function createRegistry(): any {
  return new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns: string[]) => new onig.OnigScanner(patterns),
      createOnigString: (s: string) => new onig.OnigString(s)
    }),
    loadGrammar: async (scopeName: string) => {
      if (scopeName !== 'source.nx') return null as any;
      const content = fs.readFileSync(grammarPath, 'utf8');
      return vsctm.parseRawGrammar(content, grammarPath);
    }
  });
}

/** Loads the NX TextMate grammar. */
export async function loadGrammar(): Promise<IGrammar> {
  await loadOniguruma();
  const grammar = await createRegistry().loadGrammar('source.nx');
  if (!grammar) throw new Error('Failed to load NX grammar');
  return grammar;
}

/**
 * Returns the index at which `occurrence` of `substring` starts in `line`, or -1.
 *
 * <para>`occurrence` is 1-based; a negative value counts from the end, so -1 is the last
 * occurrence. Several assertions are about the second or last occurrence of a name on a line — a
 * function's return type repeats a property's type, for one — and resolving only the first silently
 * checks the wrong token.</para>
 */
export function occurrenceIndex(line: string, substring: string, occurrence = 1): number {
  if (occurrence === 0) throw new Error('occurrence is 1-based; 0 is not a position');
  const starts: number[] = [];
  for (let from = line.indexOf(substring); from !== -1; from = line.indexOf(substring, from + 1)) {
    starts.push(from);
  }
  const index = occurrence > 0 ? occurrence - 1 : starts.length + occurrence;
  return index >= 0 && index < starts.length ? starts[index] : -1;
}

/** Returns the scopes of the token covering the middle of `substring`'s `occurrence` in `line`. */
export function scopesForSubstring(
  line: string,
  tokens: IToken[],
  substring: string,
  occurrence = 1
): string[] {
  const idx = occurrenceIndex(line, substring, occurrence);
  if (idx === -1) return [];
  const pos = idx + Math.floor(substring.length / 2);
  const token = tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? token.scopes : [];
}

/** One tokenized line: its text and the tokens the grammar produced for it. */
export interface TokenizedLine {
  line: string;
  tokens: IToken[];
}

/**
 * Tokenizes `lines` as a contiguous document, carrying the rule stack from one line to the next.
 *
 * <para>Single-line tokenization from a fresh rule stack cannot observe defects that depend on the
 * state a preceding line leaves behind, which is what most multi-line declaration scoping is.</para>
 */
export function tokenizeLines(grammar: IGrammar, lines: string[]): TokenizedLine[] {
  let ruleStack: StateStack | null = null;
  return lines.map(line => {
    const result = grammar.tokenizeLine(line, ruleStack);
    ruleStack = result.ruleStack;
    return { line, tokens: result.tokens };
  });
}

/** Returns the scopes of `substring` on the line of `result` whose text contains `lineSubstring`. */
export function scopesAt(
  result: TokenizedLine[],
  lineSubstring: string,
  substring: string,
  occurrence = 1
): string[] {
  const entry = result.find(r => r.line.includes(lineSubstring));
  if (!entry) {
    throw new Error(`No tokenized line contains ${JSON.stringify(lineSubstring)}`);
  }
  return scopesForSubstring(entry.line, entry.tokens, substring, occurrence);
}

/** Returns the text of the token that `scopesAt` would report on, for span assertions. */
export function tokenTextAt(
  result: TokenizedLine[],
  lineSubstring: string,
  substring: string,
  occurrence = 1
): string {
  const entry = result.find(r => r.line.includes(lineSubstring));
  if (!entry) {
    throw new Error(`No tokenized line contains ${JSON.stringify(lineSubstring)}`);
  }
  const idx = occurrenceIndex(entry.line, substring, occurrence);
  const pos = idx + Math.floor(substring.length / 2);
  const token = entry.tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? entry.line.slice(token.startIndex, token.endIndex) : '';
}

/** Returns the scopes of `substring` on line `index` of `result`. */
export function scopesAtLine(
  result: TokenizedLine[],
  index: number,
  substring: string,
  occurrence = 1
): string[] {
  const entry = result[index];
  if (!entry) {
    throw new Error(`No tokenized line at index ${index}`);
  }
  return scopesForSubstring(entry.line, entry.tokens, substring, occurrence);
}

/** Returns every token of `result`, flattened, each paired with the text it covers. */
export function flattenTokens(result: TokenizedLine[]): { text: string; scopes: string[] }[] {
  return result.flatMap(({ line, tokens }) =>
    tokens.map(token => ({ text: line.slice(token.startIndex, token.endIndex), scopes: token.scopes }))
  );
}

/**
 * Asserts on a token's scopes.
 *
 * <para>`toNotInclude` gives the negative form the spec's "SHALL NOT be scoped" scenarios need; a
 * plain `expect(...).to.not.include(...)` on an empty scope array passes vacuously, so this form
 * also requires that a token was actually found.</para>
 */
export function expectScopes(scopes: string[], label: string) {
  return {
    toInclude(...expected: string[]) {
      for (const scope of expected) {
        expect(scopes, `${label} should be scoped ${scope}, got [${scopes.join(', ')}]`).to.include(scope);
      }
      return this;
    },
    toNotInclude(...unexpected: string[]) {
      expect(scopes, `${label} matched no token`).to.not.be.empty;
      for (const scope of unexpected) {
        expect(scopes, `${label} should not be scoped ${scope}, got [${scopes.join(', ')}]`).to.not.include(scope);
      }
      return this;
    }
  };
}
