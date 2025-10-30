import { expect } from 'chai';
import type { IGrammar, IToken, StateStack } from 'vscode-textmate';
import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

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

function tokensInSlice(line: string, tokens: IToken[], startAfter: string, endAt: string): IToken[] {
  const startIdx = line.indexOf(startAfter);
  const endIdx = line.indexOf(endAt, startIdx >= 0 ? startIdx + startAfter.length : 0);
  if (startIdx < 0 || endIdx < 0) return [];
  const start = startIdx + startAfter.length;
  const end = endIdx;
  return tokens.filter(t => t.startIndex >= start && t.endIndex <= end);
}

describe('NX if-forms (scrutinee vs condition-list)', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('value if: match form (scrutinee required) highlights scrutinee and keywords', function () {
    const line = 'let result = if status is { "active": 1 "idle": 2 else: 0 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('meta.control.if.value.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('value if: condition-list form has no scrutinee between if and {', function () {
    const line = 'let result = if { x > 0: 1 else: 0 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const slice = tokensInSlice(line, tokens, 'if', '{');
    const hasQualifier = slice.some(t => t.scopes.includes('entity.name.qualifier.nx'));
    expect(hasQualifier).to.equal(false);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('meta.control.if.value.nx');
  });

  it('elements if: match form highlights scrutinee and keywords', function () {
    const line = 'if kind is { "compact": <C/> "full": <F/> else: <X/> }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('elements if: condition-list form has no scrutinee between if and {', function () {
    const line = 'if { cond: <A/> else: <B/> }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const slice = tokensInSlice(line, tokens, 'if', '{');
    const hasQualifier = slice.some(t => t.scopes.includes('entity.name.qualifier.nx'));
    expect(hasQualifier).to.equal(false);
  });

  it('properties if: match form highlights scrutinee and keywords', function () {
    const line = '<Card if status is { "ok": icon=Ok "fail": icon=No else: icon=Default } />';
    const { tokens } = grammar.tokenizeLine(line, null);
    const ifScopes = scopesForSubstring(line, tokens, 'if');
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    const isScopes = scopesForSubstring(line, tokens, 'is');
    expect(isScopes).to.include('keyword.control.match.nx');
    expect(isScopes).to.include('meta.control.if.properties.nx');
  });

  it('properties if: condition-list form has no scrutinee between if and {', function () {
    const line = '<Card if { status == "ok": icon=Ok else: icon=Default } />';
    const { tokens } = grammar.tokenizeLine(line, null);
    const slice = tokensInSlice(line, tokens, 'if', '{');
    const hasQualifier = slice.some(t => t.scopes.includes('entity.name.qualifier.nx'));
    expect(hasQualifier).to.equal(false);
  });

  it('interpolation: value condition-list form carries interpolation and value-if meta', function () {
    const line = 'class="btn {if { active: \"on\" else: \"off\" }}"';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.interpolation.begin.nx');
    expect(scopesForSubstring(line, tokens, 'if')).to.include('meta.interpolation.nx');
    expect(scopesForSubstring(line, tokens, 'if')).to.include('meta.control.if.value.nx');
  });

  it('tokens still highlight for ill-formed match without scrutinee (syntax legality is parser responsibility)', function () {
    const line = 'if is { "x": 1 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
  });
});

