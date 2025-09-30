// Minimal TextMate grammar tokenization tests for NX (TypeScript)
import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar, IToken } from 'vscode-textmate';
// Use CommonJS require via createRequire to avoid ESM interop issues

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

describe('NX TextMate grammar', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('highlights control keywords (if)', function () {
    const line = 'if isLoading:';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, 'if');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('highlights tags and attributes', function () {
    const line = '<Button x=1 y=2/>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'Button')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, 'x')).to.include('entity.other.attribute-name.nx');
    expect(scopesForSubstring(line, tokens, '1')).to.include('constant.numeric.integer.nx');
  });

  it('highlights interpolation regions', function () {
    const line = 'class="card {className}"';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Opening brace should be marked as interpolation begin
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.interpolation.begin.nx');
    // Inner identifier should carry the interpolation meta scope
    expect(scopesForSubstring(line, tokens, 'className')).to.include('meta.interpolation.nx');
  });

  it('highlights inline element as attribute value', function () {
    const line = '<Button prop=<Start/> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Attribute name
    expect(scopesForSubstring(line, tokens, 'prop')).to.include('entity.other.attribute-name.nx');
    // Inline element tag name inside attribute value
    expect(scopesForSubstring(line, tokens, 'Start')).to.include('entity.name.tag.nx');
  });

  it('highlights typed inline content in attribute value', function () {
    const line = '<Button content=<:uitext>Click</> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Attribute name
    expect(scopesForSubstring(line, tokens, 'content')).to.include('entity.other.attribute-name.nx');
    // Typed tag suffix
    expect(scopesForSubstring(line, tokens, ':uitext')).to.include('support.type.text.nx');
    // Closing fragment tag is recognized
    expect(scopesForSubstring(line, tokens, '</>')).to.include('meta.tag.end.nx');
  });
});
