// Tokenization tests for the `with` expression: a keyword after an operand, a name elsewhere.
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, tokenizeLines } from './helpers.js';

describe('NX with expressions', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('scopes with after an operand', function () {
    const result = tokenizeLines(grammar, [
      'let dark = { light with { accent=#16a34a } }',
      'let both = { first with second }'
    ]);

    expectScopes(scopesAt(result, 'let dark', 'with'), 'with').toInclude('keyword.operator.with.nx');
    expectScopes(scopesAt(result, 'let both', 'with'), 'with').toInclude('keyword.operator.with.nx');
  });

  it('keeps with a name after a dot, as an attribute and as an entry', function () {
    const result = tokenizeLines(grammar, [
      'let a = { x.with }',
      'let b = { <T with=3 /> }',
      'let c = { x with { with=3 } }'
    ]);

    expectScopes(scopesAt(result, 'let a', 'with'), 'with').toNotInclude('keyword.operator.with.nx');
    expectScopes(scopesAt(result, 'let b', 'with'), 'with').toNotInclude('keyword.operator.with.nx');
    expectScopes(scopesAt(result, 'let c', 'with', 2), 'with').toNotInclude('keyword.operator.with.nx');
  });

  it('leaves with in markup text alone', function () {
    const result = tokenizeLines(grammar, ['<p>Text with words</p>']);

    expectScopes(scopesAt(result, '<p>', 'with'), 'with').toNotInclude('keyword.operator.with.nx');
  });
});
