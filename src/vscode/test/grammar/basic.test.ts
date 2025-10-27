// Minimal TextMate grammar tokenization tests for NX (TypeScript)
import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar, IToken, StateStack } from 'vscode-textmate';
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
    const line = 'if isLoading { result }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, 'if');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('uses value control scopes for inline expressions', function () {
    const ifLine = 'let value = if isLoading { 1 } else { 2 }';
    const ifTokens = grammar.tokenizeLine(ifLine, null).tokens;
    expect(scopesForSubstring(ifLine, ifTokens, 'if')).to.include('meta.control.if.value.nx');
    expect(scopesForSubstring(ifLine, ifTokens, 'else')).to.include('meta.control.if.value.nx');

    const forLine = 'let values = for item in items { item }';
    const forTokens = grammar.tokenizeLine(forLine, null).tokens;
    expect(scopesForSubstring(forLine, forTokens, 'for')).to.include('meta.control.loop.value.nx');
    expect(scopesForSubstring(forLine, forTokens, 'in')).to.include('meta.control.loop.value.nx');
  });

  it('highlights value definitions', function () {
    const line = 'let totalCount: int = 42';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'let')).to.include('keyword.declaration.let.nx');
    expect(scopesForSubstring(line, tokens, 'totalCount')).to.include('entity.name.variable.nx');
    expect(scopesForSubstring(line, tokens, ':')).to.include('punctuation.separator.type.annotation.nx');
    expect(scopesForSubstring(line, tokens, 'int')).to.include('storage.type.primitive.nx');
    expect(scopesForSubstring(line, tokens, '=')).to.include('keyword.operator.assignment.nx');
  });

  it('highlights inline else within control block', function () {
    const line = 'if user.isAuthenticated { 2 } else { 2 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, 'else');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('highlights match-style value if expressions', function () {
    const line = 'if status is { "active": 1 "idle": 2 else: 0 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights nested control blocks', function () {
    const lines = [
      'if outer {',
      '  for item in items {',
      '    if inner {',
      '      <Item/>',
      '    }',
      '  }',
      '}'
    ];

    let ruleStack: StateStack | null = null;

    const advance = (line: string) => {
      const result = grammar.tokenizeLine(line, ruleStack);
      ruleStack = result.ruleStack;
      return result.tokens;
    };

    advance(lines[0]);
    const forTokens = advance(lines[1]);
    const innerIfTokens = advance(lines[2]);

    expect(scopesForSubstring(lines[1], forTokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(lines[2], innerIfTokens, 'if')).to.include('keyword.control.conditional.nx');
  });

  it('highlights inline if blocks within element content', function () {
    const line = 'render prefix if user.isAuthenticated { <Item/> } else { <Fallback/> } suffix';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights match-style elements if expressions', function () {
    const line = 'render if kind is { "compact": <Compact/> "full": <Full/> else: <Fallback/> }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights inline for blocks within element content', function () {
    const line = 'render for item in items { item } done';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, 'in')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.block.begin.nx');
    expect(scopesForSubstring(line, tokens, '}')).to.include('punctuation.section.block.end.nx');
  });

  it('highlights the conditional operator', function () {
    const line = 'let result = isReady ? whenReady() : whenNot();';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '?')).to.include('keyword.operator.conditional.nx');
    expect(scopesForSubstring(line, tokens, ':')).to.include('punctuation.separator.conditional.nx');
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

  it('treats escaped braces in markup text as literals', function () {
    const line = '<p>\\{ brace \\}</p>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '{')).to.not.include('punctuation.section.interpolation.begin.nx');
    expect(scopesForSubstring(line, tokens, '}')).to.not.include('punctuation.section.interpolation.end.nx');
  });

  it('highlights inline element as attribute value', function () {
    const line = '<Button prop=<Start/> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Attribute name
    expect(scopesForSubstring(line, tokens, 'prop')).to.include('entity.other.attribute-name.nx');
    // Inline element tag name inside attribute value
    expect(scopesForSubstring(line, tokens, 'Start')).to.include('entity.name.tag.nx');
  });

  it('highlights control blocks inside interpolations', function () {
    const line = '{if isActive { "active" } else { "inactive" }}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights sequence type modifiers', function () {
    const line = 'let numbers: int[] = [1, 2]';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '[]')).to.include('keyword.operator.type-modifier.nx');
  });

  it('highlights match and for blocks inside interpolations', function () {
    const line = '{if state is { "active": "A" else: "D" } for item in items { item }}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, 'in')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, ' { ')).to.include('punctuation.section.block.begin.nx');
    expect(scopesForSubstring(line, tokens, ' }')).to.include('punctuation.section.block.end.nx');
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

  it('highlights self-closing slash inside attribute value', function () {
    const line = '<Button prop=<Start/> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // The slash in the inner self-closing tag should be highlighted
    expect(scopesForSubstring(line, tokens, '/')).to.include('punctuation.definition.tag.self-closing.nx');
  });

  it('highlights self-closing slash not at end-of-line', function () {
    const line = '<Start/> <Next/>';
    const { tokens } = grammar.tokenizeLine(line, null);
    // The first self-closing slash should still be highlighted despite trailing content on the line
    expect(scopesForSubstring(line, tokens, '/')).to.include('punctuation.definition.tag.self-closing.nx');
  });
});
