// Self-tests for the shared grammar-test infrastructure in helpers.ts.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesAtLine, tokenizeLines } from './helpers.js';

describe('grammar test helpers', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('carries the rule stack across lines', function () {
    const result = tokenizeLines(grammar, ['type User = {', '  name: string', '}']);

    // Reachable only from the rule stack the first line leaves behind.
    expect(scopesAtLine(result, 1, 'name')).to.include('variable.other.property.nx');
    expect(scopesAt(result, 'name:', 'string')).to.include('support.type.primitive.nx');
  });

  it('supports negative scope assertions', function () {
    const result = tokenizeLines(grammar, ['<Button x=1 />']);
    expectScopes(scopesAtLine(result, 0, 'Button'), 'Button').toNotInclude('entity.name.type.nx');
    expectScopes(scopesAtLine(result, 0, 'Button'), 'Button').toInclude('entity.name.tag.nx');
  });

  it('fails a negative assertion when no token was found', function () {
    expect(() => expectScopes([], 'nothing').toNotInclude('any.scope.nx')).to.throw();
  });
});
