// Hover content is markdown whose NX fragments are fenced ```nx, and some of those fragments are
// not NX: a parameter, a field, and a union case have no standalone spelling in the language, so
// hover writes them with a parenthesized kind. Without a rule for that shape the line falls
// outside every declaration context the grammar scopes inside — the names go unhighlighted, and
// the annotation colon and `?` are scoped as a ternary's.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesForSubstring } from './helpers.js';

function scopes(grammar: IGrammar, line: string, substring: string, occurrence = 1): string[] {
  const { tokens } = grammar.tokenizeLine(line, null);
  return scopesForSubstring(line, tokens, substring, occurrence);
}

describe('NX hover annotations', () => {
  let grammar: IGrammar;

  before(async () => {
    grammar = await loadGrammar();
  });

  it('scopes the owner, the property, and its type', () => {
    const line = '(property) ShapeCommon.shadows: Shadow[]?';

    expectScopes(scopes(grammar, line, 'property'), 'the kind').toInclude(
      'meta.annotation.hover.kind.nx'
    );
    expectScopes(scopes(grammar, line, 'ShapeCommon'), 'the owner').toInclude(
      'entity.name.type.nx'
    );
    expectScopes(scopes(grammar, line, 'shadows'), 'the property').toInclude(
      'variable.other.property.nx'
    );
    expectScopes(scopes(grammar, line, 'Shadow'), 'the declared type').toInclude(
      'entity.name.type.nx'
    );
    expectScopes(scopes(grammar, line, '[]?'), 'the type suffixes').toInclude(
      'keyword.operator.type-modifier.nx'
    );
  });

  // The colon of an annotation and the `?` of a nullable type are not a ternary's. Outside a
  // declaration context they were scoped as one, which is the defect this rule exists to fix.
  it('does not scope the annotation colon or the nullable suffix as a ternary', () => {
    const line = '(property) ShapeCommon.shadows: Shadow[]?';

    expectScopes(scopes(grammar, line, ':'), 'the annotation colon')
      .toInclude('punctuation.separator.type.annotation.nx')
      .toNotInclude('punctuation.separator.conditional.nx');
    expectScopes(scopes(grammar, line, '[]?'), 'the nullable suffix').toNotInclude(
      'keyword.operator.conditional.nx'
    );
  });

  it('scopes a parameter and its primitive type', () => {
    const line = '(parameter) count: int';

    expectScopes(scopes(grammar, line, 'count'), 'the parameter').toInclude(
      'variable.other.property.nx'
    );
    expectScopes(scopes(grammar, line, 'int'), 'the primitive type').toInclude(
      'support.type.primitive.nx'
    );
  });

  it('scopes a union case payload field under its case', () => {
    const line = '(property) LoadState.failed.message: string';

    expectScopes(scopes(grammar, line, 'LoadState.failed.'), 'the owner').toInclude(
      'entity.name.type.nx'
    );
    expectScopes(scopes(grammar, line, 'message'), 'the field').toInclude(
      'variable.other.property.nx'
    );
  });

  it('scopes a union case the way source spells one', () => {
    const line = '(case) Role.admin';

    expectScopes(scopes(grammar, line, 'Role'), 'the union').toInclude(
      'entity.name.type.union.nx'
    );
    expectScopes(scopes(grammar, line, 'admin'), 'the case').toInclude(
      'entity.name.type.union.case.nx'
    );
  });

  // `type` inside the kind was being scoped as the `type` declaration keyword, so the prefix
  // highlighted as if a declaration started there.
  it('does not scope the word type inside a kind as a declaration keyword', () => {
    const line = '(primitive type) int';

    expectScopes(scopes(grammar, line, 'primitive type'), 'the kind')
      .toInclude('meta.annotation.hover.kind.nx')
      .toNotInclude('keyword.declaration.type.nx');
    expectScopes(scopes(grammar, line, 'int'), 'the primitive type').toInclude(
      'support.type.primitive.nx'
    );
  });

  it('scopes a built-in type name', () => {
    const line = '(built-in type) Element';

    expectScopes(scopes(grammar, line, 'Element'), 'the built-in type').toInclude(
      'entity.name.type.nx'
    );
  });

  // Every shape is a line of hover content, so a theme keying off the container scope — to tint
  // hover content, or to suppress a rule inside it — sees all three.
  it('marks every shape as a hover annotation line', () => {
    for (const line of [
      '(property) ShapeCommon.shadows: Shadow[]?',
      '(case) Role.admin',
      '(built-in type) Element'
    ]) {
      expectScopes(scopes(grammar, line, ')'), `the line ${JSON.stringify(line)}`).toInclude(
        'meta.annotation.hover.nx'
      );
    }
  });

  // The rule is bounded by the literal kind words, which is the only boundary a line-based grammar
  // can draw here. A line in the same shape whose first word is not one of them is left alone.
  it('leaves a line whose first word is not a kind alone', () => {
    const line = '(counted) items: many';

    expect(scopes(grammar, line, 'items')).to.not.include('variable.other.property.nx');
    expect(scopes(grammar, line, 'counted')).to.not.include('meta.annotation.hover.kind.nx');
  });

  // The cost of that boundary, pinned so it is visible rather than discovered: element text is
  // prose, and a line of prose that opens with a kind word in this shape is scoped as hover
  // content. Highlighting every fragment hover emits is worth this; see the rule's comment.
  it('does claim element text that reads like a hover annotation', () => {
    const annotated = '(parameter) rights: reserved';
    expect(scopes(grammar, annotated, 'rights')).to.include('variable.other.property.nx');

    // Every kind claims prose, not only the ones carrying an annotation.
    const dotted = '(case) All.rights';
    expect(scopes(grammar, dotted, 'All')).to.include('entity.name.type.union.nx');
  });

  it('leaves a parenthesized expression in source alone', () => {
    const line = 'let value = { (count) }';

    expect(scopes(grammar, line, 'let')).to.include('keyword.declaration.let.nx');
    expect(scopes(grammar, line, 'count')).to.not.include('meta.annotation.hover.kind.nx');
  });
});
