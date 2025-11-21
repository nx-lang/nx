import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
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

function hasScopeAt(line: string, substr: string, scopes: string[], grammar: IGrammar): boolean {
  const { tokens } = grammar.tokenizeLine(line, null);
  const i = line.indexOf(substr);
  if (i < 0) return false;
  const mid = i + Math.floor(substr.length / 2);
  const t = tokens.find((t) => t.startIndex <= mid && mid < t.endIndex);
  return !!t && scopes.every((s) => t.scopes.includes(s));
}

function scopesForSubstring(line: string, substr: string, grammar: IGrammar): string[] {
  const { tokens } = grammar.tokenizeLine(line, null);
  const i = line.indexOf(substr);
  if (i < 0) return [];
  const mid = i + Math.floor(substr.length / 2);
  const t = tokens.find((t) => t.startIndex <= mid && mid < t.endIndex);
  return t ? t.scopes : [];
}

describe('NX control blocks', () => {
  let grammar: IGrammar;

  before(async () => {
    grammar = await loadGrammar();
  });

  it('highlights braces in elements-if single-line block', () => {
    const line = 'if cond { <Spinner/> }';
    expect(hasScopeAt(line, 'if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '{', ['punctuation.section.block.begin.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '}', ['punctuation.section.block.end.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, 'Spinner', ['entity.name.tag.nx'], grammar)).to.equal(true);
  });

  it('highlights property-list if blocks with braces and else', () => {
    const line = '<UserCard if isLoading { user=loading } else { user=loaded }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    expect(scopesForSubstring(line, '{', grammar)).to.include('punctuation.section.block.begin.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
  });

  it('highlights property-list match arms', () => {
    const line = '<UserCard if status is { "active" => icon=ActiveIcon "idle" => icon=IdleIcon else => icon=DefaultIcon }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    const isScopes = scopesForSubstring(line, 'is', grammar);
    expect(isScopes).to.include('keyword.control.match.nx');
    expect(isScopes).to.include('meta.control.if.properties.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
  });

  it('highlights property-list condition list arms', () => {
    const line = '<UserCard if layout { "compact" => gap=4 "full" => gap=8 else => gap=2 }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
    expect(scopesForSubstring(line, 'gap', grammar)).to.include('entity.other.attribute-name.nx');
  });

  it('highlights fat arrows in elements match arms', () => {
    const line = 'if status is { "active" => <Span.Active/> else => <Span.Inactive/> }';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });

  it('highlights fat arrows in elements condition-list arms', () => {
    const line = 'if { count == 0 => <span:>Empty</span> }';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });

  it('highlights fat arrows inside attribute value expressions', () => {
    const line = '<UserCard icon=if { isAdmin => <Icon.Admin/> else => <Icon.User/> } />';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });
});
