// The presence operators: the postfix test `x?`, the step `x?.m` and the fallback `x ?? y`, told
// apart from a property's optional mark (`p?:`) and a type's `?` suffix by position alone. The
// language has no conditional operator and no `null`, so the grammar defines no scope for either.
import * as fs from 'fs';
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import {
  expectScopes,
  flattenTokens,
  grammarPath,
  loadGrammar,
  scopesAt,
  tokenTextAt,
  tokenizeLines
} from './helpers.js';

describe('NX presence operators', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('scopes a presence test as an operator', function () {
    const result = tokenizeLines(grammar, ['let byline = if book.author? { "by" } else { "" }']);

    expectScopes(scopesAt(result, 'author?', '?'), 'the presence test')
      .toInclude('keyword.operator.presence.nx');
    expectScopes(scopesAt(result, 'author?', 'author'), 'the member')
      .toNotInclude('keyword.operator.presence.nx');
    expect(tokenTextAt(result, 'author?', 'author'), 'the member token span').to.not.include('?');
  });

  it('scopes a step as one token', function () {
    const result = tokenizeLines(grammar, ['let name = book.author?.name']);

    expectScopes(scopesAt(result, 'author', '?.'), 'the step')
      .toInclude('keyword.operator.optional-access.nx')
      .toNotInclude('keyword.operator.presence.nx', 'punctuation.separator.dot.nx');
    expect(tokenTextAt(result, 'author', '?.'), 'the step token span').to.equal('?.');
  });

  it('scopes a fallback as one token, inside a concatenation', function () {
    const line = 'let byline = "Author: " + book.author?.name ?? "Anonymous"';
    const result = tokenizeLines(grammar, [line]);

    expectScopes(scopesAt(result, 'byline', '??'), 'the fallback')
      .toInclude('keyword.operator.coalesce.nx')
      .toNotInclude('keyword.operator.presence.nx');
    expect(tokenTextAt(result, 'byline', '??'), 'the fallback token span').to.equal('??');
    expectScopes(scopesAt(result, 'byline', '+'), 'the concatenation')
      .toInclude('keyword.operator.arithmetic.nx');
    for (const literal of ['"Author: "', '"Anonymous"']) {
      expectScopes(scopesAt(result, 'byline', literal), literal).toInclude('string.quoted.double.nx');
    }
  });

  it('tells the three spellings of a question mark apart by position', function () {
    const line = 'let f(p?: Person, q: Person?) = { p?.name ?? q? }';
    const result = tokenizeLines(grammar, [line]);

    expectScopes(scopesAt(result, line, '?', 1), '? after p')
      .toInclude('keyword.operator.optional.nx')
      .toNotInclude('keyword.operator.type-modifier.nx', 'keyword.operator.presence.nx');
    expectScopes(scopesAt(result, line, '?', 2), '? after Person')
      .toInclude('keyword.operator.type-modifier.nx')
      .toNotInclude('keyword.operator.optional.nx', 'keyword.operator.presence.nx');
    expectScopes(scopesAt(result, line, '?.'), '?.').toInclude('keyword.operator.optional-access.nx');
    expectScopes(scopesAt(result, line, '??'), '??').toInclude('keyword.operator.coalesce.nx');
    expectScopes(scopesAt(result, line, '?', -1), 'the final ?')
      .toInclude('keyword.operator.presence.nx')
      .toNotInclude('keyword.operator.optional.nx', 'keyword.operator.type-modifier.nx');
  });

  it('gives a former ternary no conditional scopes', function () {
    const result = tokenizeLines(grammar, ['let ratio = ready ? 1 : 2']);

    for (const { text, scopes } of flattenTokens(result)) {
      expect(scopes, JSON.stringify(text)).to.not.include('keyword.operator.conditional.nx');
      expect(scopes, JSON.stringify(text)).to.not.include('punctuation.separator.conditional.nx');
    }
  });

  it('scopes the word null as the name its position gives it', function () {
    const result = tokenizeLines(grammar, ['let count = { for null in items { null } }', 'let null = 1']);

    expectScopes(scopesAt(result, 'for null', 'null'), 'null as a loop binder')
      .toInclude('variable.other.readwrite.nx')
      .toNotInclude('constant.language.null.nx');
    expectScopes(scopesAt(result, 'let null', 'null'), 'null as a binding name')
      .toInclude('entity.name.variable.nx')
      .toNotInclude('constant.language.null.nx');
  });

  it('defines no scope for a conditional operator or a null literal', function () {
    const source = fs.readFileSync(grammarPath, 'utf8');

    for (const scope of [
      'keyword.operator.conditional.nx',
      'punctuation.separator.conditional.nx',
      'constant.language.null.nx'
    ]) {
      expect(source, scope).to.not.include(scope);
    }
  });
});
