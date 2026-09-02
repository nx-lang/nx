// Minimal TextMate grammar tokenization tests for NX (TypeScript)
import { expect } from 'chai';
import type { IGrammar, StateStack } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesAtLine, scopesForSubstring, tokenTextAt, tokenizeLines } from './helpers.js';

describe('NX TextMate grammar', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('highlights control keywords (if)', function () {
    const line = 'if isLoading { result }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, 'if');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('uses value control scopes for inline expressions', function () {
    const ifLine = 'let value = if isLoading { 1 } else { 2 }';
    const ifTokens = grammar.tokenizeLine(ifLine, null).tokens;
    expect(scopesForSubstring(ifLine, ifTokens, 'if')).to.include('meta.control.if.value.nx');
    expect(scopesForSubstring(ifLine, ifTokens, 'else')).to.include('meta.control.if.value.nx');

    const forLine = 'let values = for item in items { item }';
    const forTokens = grammar.tokenizeLine(forLine, null).tokens;
    expect(scopesForSubstring(forLine, forTokens, 'for')).to.include('meta.control.loop.value.nx');
    expect(scopesForSubstring(forLine, forTokens, 'in')).to.include('meta.control.loop.value.nx');
  });

  it('highlights value definitions', function () {
    const line = 'let totalCount: int = 42';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'let')).to.include('keyword.declaration.let.nx');
    expect(scopesForSubstring(line, tokens, 'totalCount')).to.include('entity.name.variable.nx');
    expect(scopesForSubstring(line, tokens, ':')).to.include('punctuation.separator.type.annotation.nx');
    expect(scopesForSubstring(line, tokens, 'int')).to.include('support.type.primitive.nx');
    expect(scopesForSubstring(line, tokens, '=')).to.include('keyword.operator.assignment.nx');
  });

  it('highlights import keywords and visibility modifiers', function () {
    const importLine = 'import { Button as Ui.Button, Input } from "../ui"';
    const importTokens = grammar.tokenizeLine(importLine, null).tokens;
    expect(scopesForSubstring(importLine, importTokens, 'import')).to.include('keyword.control.import.nx');
    expect(scopesForSubstring(importLine, importTokens, 'as')).to.include('keyword.control.import.nx');
    expect(scopesForSubstring(importLine, importTokens, 'from')).to.include('keyword.control.import.nx');

    const typeLine = 'export type Entity = {';
    const typeTokens = grammar.tokenizeLine(typeLine, null).tokens;
    expect(scopesForSubstring(typeLine, typeTokens, 'export')).to.include('storage.modifier.visibility.nx');
    expect(scopesForSubstring(typeLine, typeTokens, 'type')).to.include('keyword.declaration.type.nx');

    const valueLine = 'private let totalCount: int = 42';
    const valueTokens = grammar.tokenizeLine(valueLine, null).tokens;
    expect(scopesForSubstring(valueLine, valueTokens, 'private')).to.include('storage.modifier.visibility.nx');
    expect(scopesForSubstring(valueLine, valueTokens, 'let')).to.include('keyword.declaration.let.nx');
    expect(scopesForSubstring(valueLine, valueTokens, 'totalCount')).to.include('entity.name.variable.nx');

    const actionLine = 'action Save = {';
    const actionTokens = grammar.tokenizeLine(actionLine, null).tokens;
    expect(scopesForSubstring(actionLine, actionTokens, 'action')).to.include('keyword.declaration.type.nx');
  });

  it('highlights component definitions with visibility modifiers', function () {
    const line = 'private component <SearchBox placeholder:string /> = {';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'private')).to.include('storage.modifier.visibility.nx');
    expect(scopesForSubstring(line, tokens, 'component')).to.include('keyword.declaration.component.nx');
    expect(scopesForSubstring(line, tokens, 'SearchBox')).to.include('entity.name.type.nx');
  });

  it('highlights a constant union with a leading pipe', function () {
    const line = 'type Status = | active | pending_review | disabled';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(line, tokens, '|')).to.include('punctuation.separator.union-case.nx');
    expect(scopesForSubstring(line, tokens, 'active')).to.include('entity.name.type.union.case.nx');
  });

  it('highlights a constant union without a leading pipe', function () {
    const line = 'type Color = red | green | blue';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(line, tokens, 'red')).to.include('entity.name.type.union.case.nx');
    expect(scopesForSubstring(line, tokens, 'green')).to.include('entity.name.type.union.case.nx');
    expect(scopesForSubstring(line, tokens, '|')).to.include('punctuation.separator.union-case.nx');
  });

  it('still highlights a type alias as an alias, not a union', function () {
    const line = 'type Alias = Other';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'Other')).to.not.include('entity.name.type.union.case.nx');
  });

  it('does not highlight the removed enum keyword as a declaration', function () {
    const line = 'enum Color = red | green | blue';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'enum')).to.not.include('keyword.declaration.type.nx');
  });

  it('highlights discriminated union definitions and case fields', function () {
    const lines = [
      'export type LoadState =',
      '  | idle',
      '  | failed { message:string retryable:boolean = true }'
    ];

    let ruleStack: StateStack | null = null;

    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    expect(scopesForSubstring(lines[0], start.tokens, 'export')).to.include('storage.modifier.visibility.nx');
    expect(scopesForSubstring(lines[0], start.tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(lines[0], start.tokens, 'LoadState')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(lines[0], start.tokens, '=')).to.include('keyword.operator.assignment.nx');

    const idle = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = idle.ruleStack;
    expect(scopesForSubstring(lines[1], idle.tokens, '|')).to.include('punctuation.separator.union-case.nx');
    expect(scopesForSubstring(lines[1], idle.tokens, 'idle')).to.include('entity.name.type.union.case.nx');

    const failed = grammar.tokenizeLine(lines[2], ruleStack);
    expect(scopesForSubstring(lines[2], failed.tokens, '|')).to.include('punctuation.separator.union-case.nx');
    expect(scopesForSubstring(lines[2], failed.tokens, 'failed')).to.include('entity.name.type.union.case.nx');
    expect(scopesForSubstring(lines[2], failed.tokens, 'message')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[2], failed.tokens, 'string')).to.include('support.type.primitive.nx');
    expect(scopesForSubstring(lines[2], failed.tokens, '=')).to.include('keyword.operator.assignment.nx');
  });

  it('highlights scoped union case constructors', function () {
    const line = 'let state = <LoadState.failed message={"Offline"} />';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'LoadState.failed')).to.include('meta.tag.start.nx');
    expect(scopesForSubstring(line, tokens, 'LoadState')).to.include('entity.name.type.union.nx');
    expect(scopesForSubstring(line, tokens, 'failed')).to.include('entity.name.type.union.case.nx');
    expect(scopesForSubstring(line, tokens, 'message')).to.include('entity.other.attribute-name.nx');
  });

  it('highlights record definitions and fields', function () {
    const lines = ['type User = {', '  name: string', '  profile: models.Profile', '  age: int = 0', '}'];

    let ruleStack: StateStack | null = null;

    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    expect(scopesForSubstring(lines[0], start.tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(lines[0], start.tokens, 'User')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(lines[0], start.tokens, '{')).to.include('punctuation.section.block.begin.nx');

    const fieldOne = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = fieldOne.ruleStack;
    expect(scopesForSubstring(lines[1], fieldOne.tokens, 'name')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[1], fieldOne.tokens, ':')).to.include('punctuation.separator.type.annotation.nx');

    const fieldTwo = grammar.tokenizeLine(lines[2], ruleStack);
    ruleStack = fieldTwo.ruleStack;
    expect(scopesForSubstring(lines[2], fieldTwo.tokens, 'profile')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[2], fieldTwo.tokens, ':')).to.include('punctuation.separator.type.annotation.nx');
    expect(scopesForSubstring(lines[2], fieldTwo.tokens, 'models.Profile')).to.include('entity.name.type.nx');

    const fieldThree = grammar.tokenizeLine(lines[3], ruleStack);
    ruleStack = fieldThree.ruleStack;
    expect(scopesForSubstring(lines[3], fieldThree.tokens, 'age')).to.include('variable.other.property.nx');
    expect(scopesForSubstring(lines[3], fieldThree.tokens, '=')).to.include('keyword.operator.assignment.nx');

    const end = grammar.tokenizeLine(lines[4], ruleStack);
    expect(scopesForSubstring(lines[4], end.tokens, '}')).to.include('punctuation.section.block.end.nx');
  });

  it('highlights abstract and inherited record definitions', function () {
    const lines = [
      'abstract type Entity = {',
      '  id: int',
      '}',
      'type User extends Entity = {',
      '  name: string',
      '}'
    ];

    let ruleStack: StateStack | null = null;

    const root = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = root.ruleStack;
    expect(scopesForSubstring(lines[0], root.tokens, 'abstract')).to.include('storage.modifier.abstract.nx');
    expect(scopesForSubstring(lines[0], root.tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(lines[0], root.tokens, 'Entity')).to.include('entity.name.type.nx');

    grammar.tokenizeLine(lines[1], ruleStack);
    grammar.tokenizeLine(lines[2], ruleStack);

    const derived = grammar.tokenizeLine(lines[3], null);
    expect(scopesForSubstring(lines[3], derived.tokens, 'type')).to.include('keyword.declaration.type.nx');
    expect(scopesForSubstring(lines[3], derived.tokens, 'User')).to.include('entity.name.type.nx');
    expect(scopesForSubstring(lines[3], derived.tokens, 'extends')).to.include('keyword.declaration.extends.nx');
    expect(scopesForSubstring(lines[3], derived.tokens, 'Entity')).to.include('entity.name.type.nx');
  });

  it('highlights inline else within control block', function () {
    const line = 'if user.isAuthenticated { 2 } else { 2 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    const scopes = scopesForSubstring(line, tokens, 'else');
    expect(scopes).to.include('keyword.control.conditional.nx');
  });

  it('highlights match-style value if expressions', function () {
    const line = 'if status is { "active" => 1 "idle" => 2 else => 0 }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights nested control blocks', function () {
    const lines = [
      'if outer {',
      '  for item in items {',
      '    if inner {',
      '      <Item/>',
      '    }',
      '  }',
      '}'
    ];

    let ruleStack: StateStack | null = null;

    const advance = (line: string) => {
      const result = grammar.tokenizeLine(line, ruleStack);
      ruleStack = result.ruleStack;
      return result.tokens;
    };

    advance(lines[0]);
    const forTokens = advance(lines[1]);
    const innerIfTokens = advance(lines[2]);

    expect(scopesForSubstring(lines[1], forTokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(lines[2], innerIfTokens, 'if')).to.include('keyword.control.conditional.nx');
  });

  it('highlights inline if blocks within element content', function () {
    const line = 'render prefix if user.isAuthenticated { <Item/> } else { <Fallback/> } suffix';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights match-style elements if expressions', function () {
    const line = 'render if kind is { "compact" => <Compact/> "full" => <Full/> else => <Fallback/> }';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('does not treat comparison operators as tag starts before elements', function () {
    const lines = [
      '<abc>',
      '  if {',
      '    count < 10 => <span:>Low</span>',
      '  }',
      '</abc>'
    ];

    let ruleStack: StateStack | null = null;

    const advance = (line: string) => {
      const result = grammar.tokenizeLine(line, ruleStack);
      ruleStack = result.ruleStack;
      return result.tokens;
    };

    advance(lines[0]);
    advance(lines[1]);
    const tokens = advance(lines[2]);

    expect(scopesForSubstring(lines[2], tokens, '<')).to.include('keyword.operator.comparison.nx');
    expect(scopesForSubstring(lines[2], tokens, '<')).to.not.include('meta.tag.start.nx');
    expect(scopesForSubstring(lines[2], tokens, '<span')).to.include('meta.tag.start.nx');
    expect(scopesForSubstring(lines[2], tokens, '<span')).to.include('entity.name.tag.nx');
  });

  it('highlights inline for blocks within element content', function () {
    const line = 'render for item in items { item } done';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, 'in')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.block.begin.nx');
    expect(scopesForSubstring(line, tokens, '}')).to.include('punctuation.section.block.end.nx');
  });

  it('highlights the conditional operator', function () {
    const line = 'let result = isReady ? whenReady() : whenNot();';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '?')).to.include('keyword.operator.conditional.nx');
    expect(scopesForSubstring(line, tokens, ':')).to.include('punctuation.separator.conditional.nx');
  });

  it('highlights paren-style function calls', function () {
    const line = 'let message = render(user, index)';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'render')).to.include('source.nx');
    expect(scopesForSubstring(line, tokens, '(')).to.include('source.nx');
  });

  it('highlights every numeric primitive, including int alongside the widths', function () {
    const line = 'let f(a:int, b:int32, c:int64, d:float32, e:float64): boolean = true';
    const { tokens } = grammar.tokenizeLine(line, null);
    for (const name of ['int', 'int32', 'int64', 'float32', 'float64']) {
      expect(scopesForSubstring(line, tokens, name), name).to.include(
        'support.type.primitive.nx',
      );
    }
  });

  it('highlights paren-style function definitions', function () {
    const line = 'let render(title:string, count:int): boolean = title == ""';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'render')).to.include('entity.name.variable.nx');
    expect(scopesForSubstring(line, tokens, 'string')).to.include('support.type.primitive.nx');
    expect(scopesForSubstring(line, tokens, 'int')).to.include('support.type.primitive.nx');
    expect(scopesForSubstring(line, tokens, 'boolean')).to.include('support.type.primitive.nx');
  });

  it('highlights tags and attributes', function () {
    const line = '<Button x=1 y=2/>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'Button')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, 'x')).to.include('entity.other.attribute-name.nx');
    expect(scopesForSubstring(line, tokens, '1')).to.include('constant.numeric.integer.nx');
  });

  it('highlights a module root element after declarations', function () {
    const lines = [
      'type User = string',
      'let greeting = "hi"',
      'let <Greeting name:string /> = <span>{name}</span>',
      '<Greeting name="World" />'
    ];

    let ruleStack: StateStack | null = null;

    lines.forEach((line, index) => {
      const result = grammar.tokenizeLine(line, ruleStack);
      ruleStack = result.ruleStack;

      if (index === lines.length - 1) {
        const scopes = scopesForSubstring(line, result.tokens, 'Greeting');
        expect(scopes).to.include('entity.name.tag.nx');
        expect(scopes).to.include('meta.module.root-element.nx');
      }
    });
  });

  it('highlights braced value expression regions', function () {
    const line = 'class="card {className}"';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Opening brace should be marked as a braced value expression opener.
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.values-braced-expression.begin.nx');
    // Inner identifier should carry the braced value expression meta scope.
    expect(scopesForSubstring(line, tokens, 'className')).to.include('meta.values-braced-expression.nx');
  });

  it('keeps values-braced-expression scopes for multi-value brace sequences', function () {
    const line = '<Button class="{baseClass accentClass <Badge/>}" />';
    const { tokens } = grammar.tokenizeLine(line, null);

    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.values-braced-expression.begin.nx');
    expect(scopesForSubstring(line, tokens, 'baseClass')).to.include('meta.values-braced-expression.nx');
    expect(scopesForSubstring(line, tokens, 'accentClass')).to.include('meta.values-braced-expression.nx');
    expect(scopesForSubstring(line, tokens, '<Badge')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, '<Badge')).to.include('meta.tag.start.nx');
  });

  it('highlights braced value expressions between element children', function () {
    const line = '<Section><Header/>{content}<Footer/></Section>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.values-braced-expression.begin.nx');
    expect(scopesForSubstring(line, tokens, 'content')).to.include('meta.values-braced-expression.nx');
    expect(scopesForSubstring(line, tokens, 'Footer')).to.include('entity.name.tag.nx');
  });

  it('treats escaped braces in markup text as literals', function () {
    const line = '<p>\\{ brace \\}</p>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '{')).to.not.include('punctuation.section.values-braced-expression.begin.nx');
    expect(scopesForSubstring(line, tokens, '}')).to.not.include('punctuation.section.values-braced-expression.end.nx');
  });

  it('highlights elements inside braced value expressions', function () {
    const line = '<div>{ <Inner/> }</div>';
    const { tokens } = grammar.tokenizeLine(line, null);

    expect(scopesForSubstring(line, tokens, '<Inner')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, '<Inner')).to.include('meta.tag.start.nx');
    expect(scopesForSubstring(line, tokens, '/')).to.include('punctuation.definition.tag.self-closing.nx');
  });

  it('highlights inline element as attribute value', function () {
    const line = '<Button prop=<Start/> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Attribute name
    expect(scopesForSubstring(line, tokens, 'prop')).to.include('entity.other.attribute-name.nx');
    // Inline element tag name inside attribute value
    expect(scopesForSubstring(line, tokens, 'Start')).to.include('entity.name.tag.nx');
  });

  it('highlights control blocks inside braced value expressions', function () {
    const line = '{if isActive { "active" } else { "inactive" }}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
  });

  it('highlights sequence type modifiers', function () {
    const line = 'let numbers: int[] = [1, 2]';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, '[]')).to.include('keyword.operator.type-modifier.nx');
  });

  it('scopes an attribute named after a keyword as an attribute', function () {
    // `#keywords-core` and the attribute rule both match at the attribute's name, so list order
    // decides. With the keyword first it consumed the name, and the attribute rule's `(?=name =)`
    // lookahead then missed, leaving the `= value` unscoped as well.
    for (const name of ['type', 'component', 'let', 'state', 'external']) {
      const result = tokenizeLines(grammar, [`<Question ${name} = "multiple" />`]);
      expectScopes(scopesAt(result, '<Question', name), `attribute named \`${name}\``)
        .toInclude('entity.other.attribute-name.nx');
      expectScopes(scopesAt(result, '<Question', '='), `\`=\` after \`${name}\``)
        .toInclude('keyword.operator.assignment.nx');
      expectScopes(scopesAt(result, '<Question', 'multiple'), `value of \`${name}\``)
        .toInclude('string.quoted.double.nx');
    }

    // The same inside a property-list `if` (`samples/survey.nx:15`).
    const inIf = tokenizeLines(grammar, ['<Question', '  if true {', '    type="singleChoice"', '  } >']);
    expectScopes(scopesAtLine(inIf, 2, 'type'), 'attribute named `type` inside a property-list if')
      .toInclude('entity.other.attribute-name.nx')
      .toNotInclude('keyword.declaration.type.nx');

    // A keyword in a non-attribute position keeps its keyword scope.
    const declaration = tokenizeLines(grammar, ['export type Mode = light | dark']);
    expectScopes(scopesAt(declaration, 'Mode', 'type'), '`type` as a declaration keyword')
      .toInclude('keyword.declaration.type.nx');
  });

  it('scopes a parenthesized parameter by the property-definition rule', function () {
    // A paren parameter is a PropertyDefinition, the same production as a signature property, so
    // its name, type, and default are scoped by the same rule rather than falling through to the
    // module-qualifier catch-all.
    const result = tokenizeLines(grammar, [
      'let f(alpha: int, beta: Color? = red, gamma: string = "hi") = alpha'
    ]);

    for (const name of ['alpha', 'beta', 'gamma']) {
      expectScopes(scopesAt(result, 'let f(', name), `parameter ${name}`)
        .toInclude('variable.other.property.nx')
        .toNotInclude('entity.name.qualifier.nx');
    }
    expectScopes(scopesAt(result, 'let f(', 'int'), 'parameter type int')
      .toInclude('support.type.primitive.nx');
    expectScopes(scopesAt(result, 'let f(', 'Color'), 'parameter type Color')
      .toInclude('entity.name.type.nx');
    expectScopes(scopesAt(result, 'let f(', 'red'), 'parameter default red')
      .toInclude('variable.other.enummember.nx')
      .toNotInclude('entity.name.qualifier.nx');
    expectScopes(scopesAt(result, 'let f(', '"hi"'), 'parameter default "hi"')
      .toInclude('string.quoted.double.nx');
  });

  it('scopes the ? and [] type suffixes alike in every annotation position', function () {
    const record = tokenizeLines(grammar, [
      'type Doc = {',
      '  catalogs: CatalogUse[]',
      '  metadata: DocumentMetadata?',
      '  title: string?',
      '  items: string[]?',
      '}'
    ]);

    for (const [property, suffix] of [['catalogs', '[]'], ['metadata', '?'], ['title', '?']]) {
      expectScopes(scopesAt(record, property, suffix), `${suffix} on ${property}`)
        .toInclude('keyword.operator.type-modifier.nx')
        .toNotInclude('keyword.operator.conditional.nx');
    }
    expectScopes(scopesAt(record, 'items', '[]'), '[] on items').toInclude('keyword.operator.type-modifier.nx');
    expectScopes(scopesAt(record, 'items', '?'), '? on items').toInclude('keyword.operator.type-modifier.nx');

    // The same suffix in a declaration signature.
    const signature = tokenizeLines(grammar, ['component <Box', '  size: float64?', '/>']);
    expectScopes(scopesAt(signature, 'size', '?'), '? in a signature')
      .toInclude('keyword.operator.type-modifier.nx');

    // ...and in a value definition, a parameter list, and a function return type.
    const value = tokenizeLines(grammar, [
      'let catalogs: CatalogUse[]? = items',
      'let render(scale: float64?) = scale',
      'let <Row item: string /> : Element? = <HStack />'
    ]);
    for (const [line, suffix] of [['catalogs', '[]?'], ['scale: float64', '?'], ['Element?', '?']]) {
      expectScopes(scopesAt(value, line, suffix), `${suffix} in \`${line}\``)
        .toInclude('keyword.operator.type-modifier.nx')
        .toNotInclude('keyword.operator.conditional.nx');
    }

    // `TypeSuffix*` is a source-ordered repetition, so `?` may precede `[]`.
    const reordered = tokenizeLines(grammar, [
      'component <Box',
      '  tags: string?[]',
      '  swatches: Color[]?[]',
      '/>'
    ]);
    for (const [property, suffix] of [['tags', '?[]'], ['swatches', '[]?[]']]) {
      expectScopes(scopesAt(reordered, property, suffix), `${suffix} on ${property}`)
        .toInclude('keyword.operator.type-modifier.nx');
      expect(tokenTextAt(reordered, property, suffix), `${suffix} on ${property} span`).to.equal(suffix);
    }

    // ...but a ternary is still a ternary.
    const ternary = tokenizeLines(grammar, ['let ratio = ready ? 1 : 2']);
    expectScopes(scopesAt(ternary, 'ratio', '?'), '? in a ternary')
      .toInclude('keyword.operator.conditional.nx')
      .toNotInclude('keyword.operator.type-modifier.nx');
  });

  it('highlights match and for blocks inside braced value expressions', function () {
    const line = '{if state is { "active" => "A" else => "D" } for item in items { item }}';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'is')).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, tokens, 'else')).to.include('keyword.control.conditional.nx');
    expect(scopesForSubstring(line, tokens, 'for')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, 'in')).to.include('keyword.control.loop.nx');
    expect(scopesForSubstring(line, tokens, ' { ')).to.include('punctuation.section.block.begin.nx');
    expect(scopesForSubstring(line, tokens, ' }')).to.include('punctuation.section.block.end.nx');
  });

  it('highlights typed inline content in attribute value', function () {
    const line = '<Button content=<:uitext>Click</> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // Attribute name
    expect(scopesForSubstring(line, tokens, 'content')).to.include('entity.other.attribute-name.nx');
    // Typed tag suffix
    expect(scopesForSubstring(line, tokens, ':uitext')).to.include('support.type.text.nx');
    // Closing fragment tag is recognized
    expect(scopesForSubstring(line, tokens, '</>')).to.include('meta.tag.end.nx');
  });

  it('highlights self-closing slash inside attribute value', function () {
    const line = '<Button prop=<Start/> />';
    const { tokens } = grammar.tokenizeLine(line, null);
    // The slash in the inner self-closing tag should be highlighted
    expect(scopesForSubstring(line, tokens, '/')).to.include('punctuation.definition.tag.self-closing.nx');
  });

  it('highlights self-closing slash not at end-of-line', function () {
    const line = '<Start/> <Next/>';
    const { tokens } = grammar.tokenizeLine(line, null);
    // The first self-closing slash should still be highlighted despite trailing content on the line
    expect(scopesForSubstring(line, tokens, '/')).to.include('punctuation.definition.tag.self-closing.nx');
  });
});
