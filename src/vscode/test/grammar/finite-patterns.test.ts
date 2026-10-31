import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { loadGrammar, scopesForSubstring, tokenizeLines } from './helpers.js';

describe('NX finite patterns', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('highlights the contextual finite term in constraints and numeric arms', function () {
    for (const line of [
      'type Coordinate = float64 where finite',
      'type Extent = float64 where >= 0.0 && finite',
      'let classify(x: float64) = { if x is { finite => "yes" else => "no" } }',
    ]) {
      const { tokens } = grammar.tokenizeLine(line, null);
      expect(scopesForSubstring(line, tokens, 'finite'), line).to.include('keyword.other.finite.nx');
    }
  });

  it('highlights a numeric arm across line breaks', function () {
    const lines = tokenizeLines(grammar, [
      'let classify(x: float64) = {',
      '  if x is {',
      '    finite => "yes"',
      '    else => "no"',
      '  }',
      '}',
    ]);
    expect(scopesForSubstring(lines[2].line, lines[2].tokens, 'finite'))
      .to.include('keyword.other.finite.nx');
  });

  it('keeps fields, functions, and qualified references named finite as identifiers', function () {
    for (const line of [
      'type Sample = { finite: boolean }',
      'let finite(x: float64): boolean = { x == x }',
      'let result = { limits.finite }',
    ]) {
      const { tokens } = grammar.tokenizeLine(line, null);
      expect(scopesForSubstring(line, tokens, 'finite'), line).to.not.include('keyword.other.finite.nx');
    }
  });
});
