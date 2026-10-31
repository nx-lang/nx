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

describe('NX finite pattern boundaries', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('keeps boolean operands and calls as identifiers after conjunctions', function () {
    for (const line of [
      'let ok(a: boolean, finite: boolean) = { a && finite }',
      'let ok(a: boolean, x: float64) = { a && finite(x) }',
      '<Check value={a && finite} />',
      '<Check value={a && finite(x)} />',
      'let ok(where: boolean, finite: boolean) = { where && finite }',
    ]) {
      const { tokens } = grammar.tokenizeLine(line, null);
      expect(scopesForSubstring(line, tokens, 'finite', -1), line)
        .to.not.include('keyword.other.finite.nx');
    }
  });

  it('keeps boolean conditions as identifiers in condition lists', function () {
    for (const lines of [
      ['let ok(finite: boolean) = { if { finite => "yes" else => "no" } }'],
      ['let ok(finite: boolean) = {', '  if {', '    finite => "yes"', '    else => "no"', '  }', '}'],
    ]) {
      for (const { line, tokens } of tokenizeLines(grammar, lines)) {
        if (line.includes('finite =>')) {
          expect(scopesForSubstring(line, tokens, 'finite'), line)
            .to.not.include('keyword.other.finite.nx');
        }
      }
    }
  });

  it('highlights later arms, comma-separated patterns, and conjunctions', function () {
    for (const line of [
      'let ok(x: float64) = { if x is { < 0.0 => "neg" finite => "ok" else => "no" } }',
      'let ok(x: float64) = { if x is { 0.0, finite => "ok" else => "no" } }',
      'let ok(x: float64) = { if x is { finite && >= 0.0 => "ok" else => "no" } }',
      'let ok(x: float64) = { if x is { >= 0.0 && finite => "ok" else => "no" } }',
      '<Check if x is { finite => value=true else => value=false } />',
    ]) {
      const { tokens } = grammar.tokenizeLine(line, null);
      expect(scopesForSubstring(line, tokens, 'finite'), line)
        .to.include('keyword.other.finite.nx');
    }
  });

  it('keeps identifiers in match bodies out of numeric pattern scopes', function () {
    for (const line of [
      'let ok(x: float64, finite: boolean) = { if x is { 0.0 => a && finite else => false } }',
      'let ok(x: float64, finite: boolean) = { if x is { 0.0 => { a && finite } else => false } }',
      'let ok(x: float64, finite: boolean) = { if x is { 0.0 => if { finite => true else => false } else => false } }',
      '<Check if x is { 0.0 => value={a && finite} else => value=false } />',
    ]) {
      const { tokens } = grammar.tokenizeLine(line, null);
      expect(scopesForSubstring(line, tokens, 'finite', -1), line)
        .to.not.include('keyword.other.finite.nx');
    }
  });

  it('keeps constraints across newlines and stops before subsequent fields and expressions', function () {
    for (const lines of [
      ['type C = float64 where', '  finite'],
      ['type C = float64 where >= 0.0 &&', '  finite'],
      ['type C = float64 where', '  // A pattern can follow trivia.', '  finite'],
      ['type R = {', '  value: float64 where', '    finite', '  finite: boolean', '}'],
      ['let ok(x: float64 where', '  finite): boolean = { a && finite(x) }'],
    ]) {
      for (const { line, tokens } of tokenizeLines(grammar, lines)) {
        if (!line.includes('finite')) continue;
        const actual = scopesForSubstring(line, tokens, 'finite');
        if (line.includes('finite:')) {
          expect(actual, line).to.include('variable.other.property.nx');
          expect(actual, line).to.not.include('keyword.other.finite.nx');
        } else {
          expect(actual, line).to.include('keyword.other.finite.nx');
          if (line.includes('finite(x)')) {
            expect(scopesForSubstring(line, tokens, 'finite', -1), line)
              .to.not.include('keyword.other.finite.nx');
          }
        }
      }
    }
  });
});

describe('NX numeric match context recovery', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('allows comments between is and its arm block', function () {
    for (const comment of ['// Choose an arm.', '/* Choose an arm. */']) {
      const lines = tokenizeLines(grammar, [
        'let classify(x: float64) = {',
        `  if x is ${comment}`,
        '  {',
        '    finite => "yes"',
        '    else => "no"',
        '  }',
        '}',
      ]);
      expect(scopesForSubstring(lines[3].line, lines[3].tokens, 'finite'), comment)
        .to.include('keyword.other.finite.nx');
    }
  });

  it('preserves value control-flow scopes in unindented match bodies', function () {
    const lines = tokenizeLines(grammar, [
      'let result() = {',
      'if x is {',
      '0.0 =>',
      'if y { 1 } else { 2 }',
      'else =>',
      'for row in rows { row }',
      '}',
      '}',
    ]);
    expect(scopesForSubstring(lines[3].line, lines[3].tokens, 'if'))
      .to.include('meta.control.if.value.nx')
      .and.not.include('meta.control.if.elements.nx');
    expect(scopesForSubstring(lines[5].line, lines[5].tokens, 'for'))
      .to.not.include('meta.control.loop.elements.nx');
  });

  it('preserves element control-flow scopes in unindented match bodies', function () {
    const lines = tokenizeLines(grammar, [
      '<Panel>',
      'if x is {',
      '0.0 =>',
      'if y { <A/> } else { <B/> }',
      'else => <C/>',
      '}',
      '</Panel>',
    ]);
    expect(scopesForSubstring(lines[3].line, lines[3].tokens, 'if'))
      .to.include('meta.control.if.elements.nx')
      .and.not.include('meta.control.if.value.nx');
  });
});
