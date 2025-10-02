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

async function loadMarkdownGrammarWithNx(): Promise<IGrammar> {
  const wasmPath = cjsRequire.resolve('vscode-oniguruma/release/onig.wasm');
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  await onig.loadWASM(wasmBin);

  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns: string[]) => new onig.OnigScanner(patterns),
      createOnigString: (value: string) => new onig.OnigString(value)
    }),
    loadGrammar: async (scopeName: string) => {
      if (scopeName === 'text.html.markdown') {
        const grammarPath = path.join(__dirname, '..', 'fixtures', 'markdown.test.tmLanguage.json');
        return vsctm.parseRawGrammar(fs.readFileSync(grammarPath, 'utf8'), grammarPath);
      }

      if (scopeName === 'source.nx') {
        const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.tmLanguage.json');
        return vsctm.parseRawGrammar(fs.readFileSync(grammarPath, 'utf8'), grammarPath);
      }

      if (scopeName === 'source.nx.embedded.markdown') {
        const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.markdown.codeblock.tmLanguage.json');
        return vsctm.parseRawGrammar(fs.readFileSync(grammarPath, 'utf8'), grammarPath);
      }

      return null;
    },
    getInjections: (scopeName: string) => {
      if (scopeName === 'text.html.markdown') {
        return ['source.nx.embedded.markdown'];
      }

      return undefined;
    }
  });

  const grammar = await registry.loadGrammar('text.html.markdown');
  if (!grammar) {
    throw new Error('Failed to load Markdown grammar with NX injection');
  }

  return grammar;
}

function scopesForSubstring(line: string, tokens: IToken[], substring: string): string[] {
  const idx = line.indexOf(substring);
  if (idx === -1) return [];
  const pos = idx + Math.floor(substring.length / 2);
  const token = tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? token.scopes : [];
}

describe('Markdown NX fenced code blocks', function () {
  let markdownGrammar: IGrammar;

  before(async function () {
    markdownGrammar = await loadMarkdownGrammarWithNx();
    expect(markdownGrammar).to.exist;
  });

  it('highlights NX syntax inside fenced code blocks', function () {
    const lines = ['```nx', 'if isLoading:', '  <Spinner/>', '```'];
    let ruleStack: any = null;
    let tokensForNxLine: IToken[] = [];

    lines.forEach((line, index) => {
      const result = markdownGrammar.tokenizeLine(line, ruleStack);
      if (index === 1) {
        tokensForNxLine = result.tokens;
      }
      ruleStack = result.ruleStack;
    });

    const scopes = scopesForSubstring(lines[1], tokensForNxLine, 'if');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });
});
