// Tokenization tests for unbraced property values: contextual names and signed literals.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesForSubstring, tokenizeLines } from './helpers.js';

describe('NX unbraced property values', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('scopes a bare value as a union case, distinctly from a string', function () {
    const line = '<Img fit=cover alt="cover" />';
    const { tokens } = grammar.tokenizeLine(line, null);

    expect(scopesForSubstring(line, tokens, 'cover')).to.include(
      'variable.other.enummember.nx'
    );
    expect(scopesForSubstring(line, tokens, '"cover"')).to.not.include(
      'variable.other.enummember.nx'
    );
  });

  it('keeps true, false, and null as literals rather than bare names', function () {
    const line = '<C flag=true other=false opt=null />';
    const { tokens } = grammar.tokenizeLine(line, null);

    for (const literal of ['true', 'false', 'null']) {
      expect(scopesForSubstring(line, tokens, literal)).to.not.include(
        'variable.other.enummember.nx'
      );
    }
  });

  it('scopes a signed numeric literal as a number', function () {
    const line = '<C x=-1.5 />';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, '-1.5');
    expect(scopes.some(scope => scope.startsWith('constant.numeric'))).to.equal(true);
  });

  it('scopes a bare union case alike in every unbraced value position', function () {
    const record = tokenizeLines(grammar, ['type Stroke = {', '  lineCap: LineCap = butt', '}']);
    expectScopes(scopesAt(record, 'lineCap', 'butt'), 'record property default')
      .toInclude('variable.other.enummember.nx')
      .toNotInclude('entity.name.qualifier.nx');

    const signature = tokenizeLines(grammar, ['component <S lineJoin: LineJoin = miter />']);
    expectScopes(scopesAt(signature, 'lineJoin', 'miter'), 'signature property default')
      .toInclude('variable.other.enummember.nx');

    const attribute = tokenizeLines(grammar, ['<Img fit=cover />']);
    expectScopes(scopesAt(attribute, 'fit', 'cover'), 'attribute value')
      .toInclude('variable.other.enummember.nx');
  });

  it('does not resolve a dotted name in a value position as a union case', function () {
    // nx-grammar.md: a ContextualName is a single Identifier and deliberately never a
    // QualifiedName, since admitting `fit=Fit.cover` would also admit `fit=obj.field`.
    for (const [label, lines, find] of [
      ['record property default', ['type S = {', '  b: LineCap = LineCap.butt', '}'], 'b:'],
      ['attribute value', ['<Img fit=LineCap.butt />'], 'fit']
    ] as [string, string[], string][]) {
      const result = tokenizeLines(grammar, lines);
      expectScopes(scopesAt(result, find, 'LineCap.'), `${label}: dotted name`)
        .toNotInclude('entity.name.type.union.nx');
    }
  });

  it('keeps a module-qualified name out of the union-case scopes', function () {
    const result = tokenizeLines(grammar, ['type U = {', '  profile: models.Profile', '}']);
    expectScopes(scopesAt(result, 'profile', 'models.Profile'), 'a module-qualified type')
      .toInclude('entity.name.type.nx')
      .toNotInclude('variable.other.enummember.nx');
  });

  it('scopes the same RhsExpression alike at every `= RhsExpression` site', function () {
    // The production appears at a value definition, a function definition, a record property
    // default, a signature property default, a parenthesized parameter default, and an attribute
    // value. All six must agree.
    const sites: [string, string[], string][] = [
      ['value definition', ['export let lineJoin: LineJoin = RHS'], 'lineJoin'],
      ['function definition', ['let <Row gap: float64 /> : LineJoin = RHS'], 'Row'],
      ['record property default', ['type S = {', '  lineJoin: LineJoin = RHS', '}'], 'lineJoin'],
      ['signature property default', ['component <S lineJoin: LineJoin = RHS />'], 'lineJoin'],
      ['parameter default', ['let stroke(lineJoin: LineJoin = RHS) = lineJoin'], 'stroke('],
      ['attribute value', ['<Img lineJoin=RHS />'], 'lineJoin']
    ];

    const scopesFor = (rhs: string, expected: string) => {
      for (const [label, template, find] of sites) {
        const lines = template.map(l => l.replace('RHS', rhs));
        const result = tokenizeLines(grammar, lines);
        expectScopes(scopesAt(result, find, rhs), `${label}: ${rhs}`).toInclude(expected);
      }
    };

    scopesFor('miter', 'variable.other.enummember.nx');   // ContextualName
    scopesFor('42', 'constant.numeric.integer.nx');       // Literal
    scopesFor('1.5', 'constant.numeric.float.nx');        // Literal
    scopesFor('true', 'constant.language.boolean.nx');    // Literal
    scopesFor('"hi"', 'string.quoted.double.nx');         // Literal
    scopesFor('-7', 'constant.numeric.integer.nx');       // SignedNumericLiteral

    // Element and ValuesBracedExpression, checked on the token inside them.
    for (const [label, template, find] of sites) {
      const element = tokenizeLines(grammar, template.map(l => l.replace('RHS', '<Foo />')));
      expectScopes(scopesAt(element, find, 'Foo'), `${label}: element RHS`)
        .toInclude('entity.name.tag.nx');

      const braced = tokenizeLines(grammar, template.map(l => l.replace('RHS', '{ compute }')));
      expectScopes(scopesAt(braced, find, 'compute'), `${label}: braced RHS`)
        .toInclude('meta.values-braced-expression.nx');
    }
  });
});
