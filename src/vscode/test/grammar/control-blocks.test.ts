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

describe('NX control blocks', () => {
  let grammar: IGrammar;

  before(async () => {
    grammar = await loadGrammar();
  });

  it('highlights mid-line /if in elements-if single-line block', () => {
    const line = 'if cond: <Spinner/> /if';
    expect(hasScopeAt(line, 'if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '/if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, 'Spinner', ['entity.name.tag.nx'], grammar)).to.equal(true);
  });

  it('highlights property-list if within a start tag and mid-line /if', () => {
    const line = '<UserCard if isLoading: user={user} /if>';
    expect(hasScopeAt(line, 'if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '/if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
  });

  it('highlights property-list switch with mid-line case/default and /switch', () => {
    const line = '<UserCard switch x case 1: a=1 default: b=2 /switch>';
    expect(hasScopeAt(line, 'switch', ['keyword.control.switch.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, 'case', ['keyword.control.switch.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, 'default', ['keyword.control.switch.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '/switch', ['keyword.control.switch.nx'], grammar)).to.equal(true);
  });
});

