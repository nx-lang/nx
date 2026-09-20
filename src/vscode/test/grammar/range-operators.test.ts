// The range operators are operators: `..` and `..=` wherever a value expression admits them, and a
// number beside one keeps its numeric scope.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { loadGrammar, scopesForSubstring } from './helpers.js';

describe('NX range operators', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('scopes `..` in a for header as an operator', function () {
    const line = 'let stars = for i in 0..count { i }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '..')).to.include('keyword.operator.range.nx');
    expect(scopesForSubstring(line, tokens, '0')).to.include('constant.numeric.integer.nx');
    expect(scopesForSubstring(line, tokens, 'count')).to.include('variable.other.readwrite.nx');
  });

  it('scopes `..=` as a single operator between two numeric literals', function () {
    const line = 'let r = {1..=5}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '..=')).to.include('keyword.operator.range.nx');
    expect(scopesForSubstring(line, tokens, '1')).to.include('constant.numeric.integer.nx');
    expect(scopesForSubstring(line, tokens, '5')).to.include('constant.numeric.integer.nx');
    // Not a dot followed by an assignment.
    expect(scopesForSubstring(line, tokens, '..=')).to.not.include('keyword.operator.assignment.nx');
  });

  it('keeps an integer literal before `..` an integer, not a real literal', function () {
    const line = 'let r = {1..5}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '1')).to.include('constant.numeric.integer.nx');
    expect(scopesForSubstring(line, tokens, '1')).to.not.include('constant.numeric.float.nx');
    expect(scopesForSubstring(line, tokens, '..')).to.include('keyword.operator.range.nx');
  });

  it('still scopes a real literal beside a range operator as a real literal', function () {
    const line = 'let r = {1.5..2.5}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '1.5')).to.include('constant.numeric.float.nx');
    expect(scopesForSubstring(line, tokens, '2.5')).to.include('constant.numeric.float.nx');
    expect(scopesForSubstring(line, tokens, '..')).to.include('keyword.operator.range.nx');
  });

  // A braced property value and a call argument reach the operator through different pattern sets
  // from the two above: `range={…}` enters through the element's property list, and `f(0..3)`
  // through an argument list. An *unbraced* property value is deliberately not tested — NX's
  // `rhs_expression` admits only an element, a literal, a signed numeric literal, a bare name or a
  // braced expression, so `<Slider range=0..1 />` is a syntax error and has no correct highlighting
  // to pin.
  it('scopes a range in a braced property value and in a call argument', function () {
    const property = 'let s = <Slider range={0..1} />';
    const propertyTokens = grammar.tokenizeLine(property, null).tokens;
    expect(scopesForSubstring(property, propertyTokens, '..')).to.include('keyword.operator.range.nx');
    expect(scopesForSubstring(property, propertyTokens, '0')).to.include('constant.numeric.integer.nx');
    expect(scopesForSubstring(property, propertyTokens, '1')).to.include('constant.numeric.integer.nx');

    const call = 'let a = { f(0..=3) }';
    const callTokens = grammar.tokenizeLine(call, null).tokens;
    expect(scopesForSubstring(call, callTokens, '..=')).to.include('keyword.operator.range.nx');
    expect(scopesForSubstring(call, callTokens, '3')).to.include('constant.numeric.integer.nx');
  });

  it('keeps member accesses beside a range whole, and the operator apart from them', function () {
    const line = 'let r = {page.first..page.last}';
    const { tokens } = grammar.tokenizeLine(line, null);
    // A dotted name is one token in this grammar, so each side keeps its member-access scope and
    // the operator is scoped on its own rather than absorbed into either.
    expect(scopesForSubstring(line, tokens, 'page.first')).to.include('entity.name.qualifier.nx');
    expect(scopesForSubstring(line, tokens, 'page.last')).to.include('entity.name.qualifier.nx');
    expect(scopesForSubstring(line, tokens, '..')).to.include('keyword.operator.range.nx');
    expect(scopesForSubstring(line, tokens, '..')).to.not.include('entity.name.qualifier.nx');
  });
});
