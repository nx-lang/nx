// Tokenization tests for unbraced property values: contextual names and signed literals.
import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar, IToken } from 'vscode-textmate';

const cjsRequire = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const onig: any = cjsRequire('vscode-oniguruma');
const vsctm: any = cjsRequire('vscode-textmate');

async function loadGrammar(): Promise<IGrammar> {
  const wasmPath = cjsRequire.resolve('vscode-oniguruma/release/onig.wasm');
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  await onig.loadWASM(wasmBin);

  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns: string[]) => new onig.OnigScanner(patterns),
      createOnigString: (s: string) => new onig.OnigString(s)
    }),
    loadGrammar: async (scopeName: string) => {
      if (scopeName !== 'source.nx') return null as any;
      const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.tmLanguage.json');
      const content = fs.readFileSync(grammarPath, 'utf8');
      return vsctm.parseRawGrammar(content, grammarPath);
    }
  });

  const grammar = await registry.loadGrammar('source.nx');
  if (!grammar) throw new Error('Failed to load NX grammar');
  return grammar;
}

function scopesForSubstring(line: string, tokens: IToken[], substring: string): string[] {
  const idx = line.indexOf(substring);
  if (idx === -1) return [];
  const pos = idx + Math.floor(substring.length / 2);
  const token = tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? token.scopes : [];
}

describe('NX unbraced property values', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('scopes a bare value as a union case, distinctly from a string', function () {
    const line = '<Img fit=cover alt="cover" />';
    const { tokens } = grammar.tokenizeLine(line, null);

    expect(scopesForSubstring(line, tokens, 'cover')).to.include(
      'variable.other.enummember.nx'
    );
    expect(scopesForSubstring(line, tokens, '"cover"')).to.not.include(
      'variable.other.enummember.nx'
    );
  });

  it('keeps true, false, and null as literals rather than bare names', function () {
    const line = '<C flag=true other=false opt=null />';
    const { tokens } = grammar.tokenizeLine(line, null);

    for (const literal of ['true', 'false', 'null']) {
      expect(scopesForSubstring(line, tokens, literal)).to.not.include(
        'variable.other.enummember.nx'
      );
    }
  });

  it('scopes a signed numeric literal as a number', function () {
    const line = '<C x=-1.5 />';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, '-1.5');
    expect(scopes.some(scope => scope.startsWith('constant.numeric'))).to.equal(true);
  });
});
