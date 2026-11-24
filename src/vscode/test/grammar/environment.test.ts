// Tokenization tests for environment values: `environment` declarations, `env.` reads and
// attributes, and `provide` declarations and blocks.
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, tokenizeLines } from './helpers.js';

describe('NX environment values', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('scopes a declaration and a read', function () {
    const result = tokenizeLines(grammar, [
      'export environment space: int = 4',
      'let gap() = { env.space * 2 }'
    ]);

    expectScopes(scopesAt(result, 'export environment', 'export'), 'export').toInclude('storage.modifier.visibility.nx');
    expectScopes(scopesAt(result, 'export environment', 'environment'), 'environment').toInclude(
      'keyword.declaration.environment.nx'
    );
    expectScopes(scopesAt(result, 'export environment', 'space'), 'space').toInclude(
      'variable.other.declaration.environment.nx'
    );
    expectScopes(scopesAt(result, 'export environment', 'int'), 'int').toNotInclude('keyword.declaration.environment.nx');
    expectScopes(scopesAt(result, 'let gap', 'env'), 'env').toInclude('support.variable.env.nx');
  });

  it('scopes a provide block and an attribute', function () {
    const result = tokenizeLines(grammar, [
      'let page() = { provide env.colorMode=dark { <Header /> } }',
      'let section() = { <Section env.colorMode=dark /> }'
    ]);

    expectScopes(scopesAt(result, 'let page', 'provide'), 'provide').toInclude('keyword.control.provide.nx');
    expectScopes(scopesAt(result, 'let page', 'env'), 'env').toInclude('support.variable.env.nx');
    expectScopes(scopesAt(result, 'let section', 'env'), 'env').toInclude('support.variable.env.nx');
    expectScopes(scopesAt(result, 'let section', '.colorMode'), '.colorMode').toInclude('entity.other.attribute-name.nx');
  });

  it('scopes a top-level provide and ends it at the next declaration', function () {
    const result = tokenizeLines(grammar, [
      'provide env.space = 8',
      'provide env with brand.theme',
      'let gap() = { 1 }'
    ]);

    expectScopes(scopesAt(result, 'provide env', 'provide'), 'provide').toInclude('keyword.control.provide.nx');
    expectScopes(scopesAt(result, 'provide env', 'env'), 'env').toInclude('support.variable.env.nx');
    expectScopes(scopesAt(result, 'provide env with', 'provide'), 'provide').toInclude('keyword.control.provide.nx');
    expectScopes(scopesAt(result, 'provide env with', 'with'), 'with').toInclude('keyword.operator.with.nx');
    expectScopes(scopesAt(result, 'let gap', 'let'), 'let')
      .toInclude('keyword.declaration.let.nx')
      .toNotInclude('meta.definition.provide.nx');
  });

  it('scopes a bundle in a provide block', function () {
    const result = tokenizeLines(grammar, [
      'let page() = { provide env with marketing with { radius=4 } { <Banner /> } }'
    ]);

    expectScopes(scopesAt(result, 'let page', 'provide'), 'provide').toInclude('keyword.control.provide.nx');
    expectScopes(scopesAt(result, 'let page', 'env'), 'env').toInclude('support.variable.env.nx');
    expectScopes(scopesAt(result, 'let page', 'with'), 'with').toInclude('keyword.operator.with.nx');
  });

  it('keeps provide and environment as names elsewhere', function () {
    const result = tokenizeLines(grammar, [
      'type Plan = { provide:string environment:string }'
    ]);

    expectScopes(scopesAt(result, 'type Plan', 'provide'), 'provide').toNotInclude('keyword.control.provide.nx');
    expectScopes(scopesAt(result, 'type Plan', 'environment'), 'environment').toNotInclude(
      'keyword.declaration.environment.nx'
    );
  });
});
