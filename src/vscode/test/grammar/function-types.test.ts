// Function types: `<function Item:Contact Index:int />: DrawnNode` in every type position, and the
// parenthesized type that lets a suffix apply to the function rather than its result.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { loadGrammar, scopesForSubstring, tokenizeLines } from './helpers.js';

describe('NX TextMate grammar: function types', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  it('scopes a function-typed property of a signature', function () {
    const line = 'external component <List TItem:type ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)? />';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'function')).to.include('keyword.other.function.nx');
    expect(scopesForSubstring(line, tokens, 'Item')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(line, tokens, 'Index')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(line, tokens, 'TItem', 2)).to.include('entity.name.type.nx');
    expect(scopesForSubstring(line, tokens, 'int')).to.include('support.type.primitive.nx');
    expect(scopesForSubstring(line, tokens, 'DrawnNode')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(line, tokens, '(')).to.include('punctuation.definition.type.group.nx');
    expect(scopesForSubstring(line, tokens, ')')).to.include('punctuation.definition.type.group.nx');
    expect(scopesForSubstring(line, tokens, '?', 1)).to.include('keyword.operator.type-modifier.nx');
  });

  it('scopes a function type alias as a function type', function () {
    const line = 'type RowTemplate = <function Item:Contact Index:int />: DrawnNode';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'RowTemplate')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(line, tokens, 'function')).to.include('keyword.other.function.nx');
    expect(scopesForSubstring(line, tokens, 'Item')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(line, tokens, 'Contact')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(line, tokens, 'DrawnNode')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(line, tokens, ':', -1)).to.include('punctuation.separator.type.annotation.nx');
  });

  it('scopes a function type in a parameter, a return annotation, a record field and a value definition', function () {
    const lines = [
      'let invoke(f: <function count:int />: int, n:int): int = <f count={n} />',
      'type Table = { header: (<function />: DrawnNode)? }',
      'let picked: (<function Item:Contact />: DrawnNode)? = null',
    ];
    const tokenized = tokenizeLines(grammar, lines);
    for (const { line, tokens } of tokenized) {
      expect(scopesForSubstring(line, tokens, 'function'), line).to.include('keyword.other.function.nx');
    }
    expect(scopesForSubstring(lines[0], tokenized[0].tokens, 'count')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[2], tokenized[2].tokens, 'Contact')).to.include('entity.name.type.nx');
  });

  it('scopes a function type that spans lines', function () {
    const lines = [
      'type Multi =',
      '  <function',
      '    Item:Contact',
      '    Index:int',
      '  />: DrawnNode',
    ];
    const tokenized = tokenizeLines(grammar, lines);
    expect(scopesForSubstring(lines[1], tokenized[1].tokens, 'function')).to.include('keyword.other.function.nx');
    expect(scopesForSubstring(lines[2], tokenized[2].tokens, 'Item')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[2], tokenized[2].tokens, 'Contact')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(lines[3], tokenized[3].tokens, 'Index')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[4], tokenized[4].tokens, 'DrawnNode')).to.include('entity.name.type.nx');
  });

  it('does not scope the identifier function as a keyword', function () {
    const line = 'let function = 1';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'function')).to.not.include('keyword.other.function.nx');
    expect(scopesForSubstring(line, tokens, 'function')).to.include('entity.name.variable.nx');
    const use = 'let v = {function}';
    const useTokens = grammar.tokenizeLine(use, null).tokens;
    expect(scopesForSubstring(use, useTokens, 'function')).to.not.include('keyword.other.function.nx');
  });
});
