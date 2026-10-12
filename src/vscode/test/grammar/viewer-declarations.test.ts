// Tokenization tests for viewer declarations: `viewer Target = Value` as a module member, and
// `viewer` as an ordinary name everywhere else.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesAtLine, tokenizeLines } from './helpers.js';

describe('NX viewer declarations', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('scopes a viewer declaration, its target and its value', function () {
    const result = tokenizeLines(grammar, ['viewer SingleChoice = <ElementViewer name="Single choice" />']);

    expectScopes(scopesAtLine(result, 0, 'viewer'), 'viewer').toInclude('keyword.declaration.viewer.nx');
    expectScopes(scopesAtLine(result, 0, 'SingleChoice'), 'SingleChoice')
      .toInclude('entity.name.type.nx')
      .toNotInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 0, '='), '=').toInclude('keyword.operator.assignment.nx');
    expectScopes(scopesAtLine(result, 0, 'ElementViewer'), 'ElementViewer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 0, 'name'), 'name').toInclude('entity.other.attribute-name.nx');
  });

  it('scopes a value written on the following lines as markup', function () {
    const result = tokenizeLines(grammar, [
      'export type ChoiceLayout = list | grid | chips',
      'viewer ChoiceLayout =',
      '  <UnionViewer>',
      '    <CaseViewer of=chips label="chips" />',
      '  </UnionViewer>',
      'type Next = { label:string }'
    ]);

    // The union ends at the viewer declaration rather than reading it as a case.
    expectScopes(scopesAtLine(result, 1, 'viewer'), 'viewer').toInclude('keyword.declaration.viewer.nx');
    expectScopes(scopesAtLine(result, 1, 'ChoiceLayout'), 'ChoiceLayout').toInclude('entity.name.type.nx');
    expectScopes(scopesAtLine(result, 2, 'UnionViewer'), 'UnionViewer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 3, 'CaseViewer'), 'CaseViewer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 5, 'type'), 'type').toInclude('keyword.declaration.type.nx');
  });

  it('keeps viewer as a property name', function () {
    const result = tokenizeLines(grammar, ['type Settings = { viewer:string }']);
    expectScopes(scopesAtLine(result, 0, 'viewer'), 'viewer')
      .toInclude('variable.other.property.nx')
      .toNotInclude('keyword.declaration.viewer.nx');
  });

  it('keeps viewer as a property name on its own line of a record', function () {
    const result = tokenizeLines(grammar, ['type Settings = {', '  viewer:string', '}']);
    expectScopes(scopesAt(result, 'viewer:', 'viewer'), 'viewer')
      .toInclude('variable.other.property.nx')
      .toNotInclude('keyword.declaration.viewer.nx');
  });

  it('keeps viewer as a value name', function () {
    const result = tokenizeLines(grammar, ['let viewer = 1', 'let shown = viewer']);
    expectScopes(scopesAtLine(result, 0, 'viewer'), 'viewer')
      .toInclude('entity.name.variable.nx')
      .toNotInclude('keyword.declaration.viewer.nx');
    expectScopes(scopesAtLine(result, 1, 'viewer'), 'viewer').toNotInclude('keyword.declaration.viewer.nx');
  });
});
