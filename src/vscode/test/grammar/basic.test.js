/* Minimal TextMate grammar tokenization tests for NX */
const fs = require('fs');
const path = require('path');
const { expect } = require('chai');
const vsctm = require('vscode-textmate');
const onig = require('vscode-oniguruma');

async function loadGrammar() {
  const wasmPath = require.resolve('vscode-oniguruma/release/onig.wasm');
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  await onig.loadWASM(wasmBin);

  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new onig.OnigScanner(patterns),
      createOnigString: (s) => new onig.OnigString(s)
    }),
    loadGrammar: async (scopeName) => {
      if (scopeName !== 'source.nx') return null;
      const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.tmLanguage.json');
      const content = fs.readFileSync(grammarPath, 'utf8');
      return vsctm.parseRawGrammar(content, grammarPath);
    }
  });

  return registry.loadGrammar('source.nx');
}

function scopesForSubstring(line, tokens, substring) {
  const idx = line.indexOf(substring);
  if (idx === -1) return [];
  const pos = idx + Math.floor(substring.length / 2);
  const token = tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? token.scopes : [];
}

describe('NX TextMate grammar', function () {
  let grammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('highlights control keywords (if)', function () {
    const line = 'if isLoading:';
    const { tokens } = grammar.tokenizeLine(line);
    const scopes = scopesForSubstring(line, tokens, 'if');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('highlights tags and attributes', function () {
    const line = '<Button x=1 y=2/>';
    const { tokens } = grammar.tokenizeLine(line);
    expect(scopesForSubstring(line, tokens, 'Button')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, 'x')).to.include('entity.other.attribute-name.nx');
    expect(scopesForSubstring(line, tokens, '1')).to.include('constant.numeric.integer.nx');
  });

  it('highlights interpolation regions', function () {
    const line = 'class="card {className}"';
    const { tokens } = grammar.tokenizeLine(line);
    // Opening brace should be marked as interpolation begin
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.interpolation.begin.nx');
    // Inner identifier should carry the interpolation meta scope
    expect(scopesForSubstring(line, tokens, 'className')).to.include('meta.interpolation.nx');
  });
});

