// Tokenization tests for visualizer declarations: `visualizer Target = Value` as a module member,
// and `visualizer` as an ordinary name everywhere else.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesAtLine, tokenizeLines } from './helpers.js';

describe('NX visualizer declarations', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('scopes a visualizer declaration, its target and its value', function () {
    const result = tokenizeLines(grammar, ['visualizer SingleChoice = <ElementVisualizer name="Single choice" />']);

    expectScopes(scopesAtLine(result, 0, 'visualizer'), 'visualizer').toInclude('keyword.declaration.visualizer.nx');
    expectScopes(scopesAtLine(result, 0, 'SingleChoice'), 'SingleChoice')
      .toInclude('entity.name.type.nx')
      .toNotInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 0, '='), '=').toInclude('keyword.operator.assignment.nx');
    expectScopes(scopesAtLine(result, 0, 'ElementVisualizer'), 'ElementVisualizer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 0, 'name'), 'name').toInclude('entity.other.attribute-name.nx');
  });

  it('scopes a value written on the following lines as markup', function () {
    const result = tokenizeLines(grammar, [
      'export type ChoiceLayout = list | grid | chips',
      'visualizer ChoiceLayout =',
      '  <UnionVisualizer>',
      '    <CaseVisualizer of=chips label="chips" />',
      '  </UnionVisualizer>',
      'type Next = { label:string }'
    ]);

    // The union ends at the visualizer declaration rather than reading it as a case.
    expectScopes(scopesAtLine(result, 1, 'visualizer'), 'visualizer').toInclude('keyword.declaration.visualizer.nx');
    expectScopes(scopesAtLine(result, 1, 'ChoiceLayout'), 'ChoiceLayout').toInclude('entity.name.type.nx');
    expectScopes(scopesAtLine(result, 2, 'UnionVisualizer'), 'UnionVisualizer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 3, 'CaseVisualizer'), 'CaseVisualizer').toInclude('entity.name.tag.nx');
    expectScopes(scopesAtLine(result, 5, 'type'), 'type').toInclude('keyword.declaration.type.nx');
  });

  it('keeps visualizer as a property name', function () {
    const result = tokenizeLines(grammar, ['type Settings = { visualizer:string }']);
    expectScopes(scopesAtLine(result, 0, 'visualizer'), 'visualizer')
      .toInclude('variable.other.property.nx')
      .toNotInclude('keyword.declaration.visualizer.nx');
  });

  it('keeps visualizer as a property name on its own line of a record', function () {
    const result = tokenizeLines(grammar, ['type Settings = {', '  visualizer:string', '}']);
    expectScopes(scopesAt(result, 'visualizer:', 'visualizer'), 'visualizer')
      .toInclude('variable.other.property.nx')
      .toNotInclude('keyword.declaration.visualizer.nx');
  });

  it('keeps visualizer as a value name', function () {
    const result = tokenizeLines(grammar, ['let visualizer = 1', 'let shown = visualizer']);
    expectScopes(scopesAtLine(result, 0, 'visualizer'), 'visualizer')
      .toInclude('entity.name.variable.nx')
      .toNotInclude('keyword.declaration.visualizer.nx');
    expectScopes(scopesAtLine(result, 1, 'visualizer'), 'visualizer').toNotInclude('keyword.declaration.visualizer.nx');
  });
});
