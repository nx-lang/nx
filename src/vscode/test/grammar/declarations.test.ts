// Tokenization tests for element-shaped declarations: components and `let` functions, their
// modifiers, property lists, emits/state groups, and signature termination.
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, loadGrammar, scopesAt, scopesAtLine, tokenTextAt, tokenizeLines } from './helpers.js';

describe('NX element-shaped declarations', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  describe('are scoped as declarations', function () {
    it('scopes an external component declaration as a declaration', function () {
      const result = tokenizeLines(grammar, [
        'export external component <Text extends UiCommon',
        '  format: TextFormat = plain',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'component', 'export'), 'export').toInclude('storage.modifier.visibility.nx');
      expectScopes(scopesAt(result, 'component', 'external'), 'external').toInclude('storage.modifier.external.nx');
      expectScopes(scopesAt(result, 'component', 'component'), 'component').toInclude('keyword.declaration.component.nx');
      expectScopes(scopesAt(result, 'component', 'Text'), 'Text')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
      expectScopes(scopesAt(result, 'component', 'extends'), 'extends').toInclude('keyword.declaration.extends.nx');
      expectScopes(scopesAt(result, 'component', 'UiCommon'), 'UiCommon').toInclude('entity.name.type.nx');
    });

    it('scopes an abstract external component declaration as a declaration', function () {
      const result = tokenizeLines(grammar, [
        'export abstract external component <UiCommon',
        '  width: float64?',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'component', 'abstract'), 'abstract').toInclude('storage.modifier.abstract.nx');
      expectScopes(scopesAt(result, 'component', 'external'), 'external').toInclude('storage.modifier.external.nx');
      expectScopes(scopesAt(result, 'component', 'UiCommon'), 'UiCommon').toInclude('entity.name.type.nx');
    });

    it('scopes an element-shaped function declaration as a declaration', function () {
      const line = 'export let <Row gap: float64 = 0.0 content child: Element /> : Element = <HStack />';
      const result = tokenizeLines(grammar, [line]);

      expectScopes(scopesAtLine(result, 0, 'let'), 'let').toInclude('keyword.declaration.let.nx');
      expectScopes(scopesAtLine(result, 0, 'Row'), 'Row')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
      // The return type, after the signature's `/>` — the last `Element`, not the property's type.
      expectScopes(scopesAtLine(result, 0, 'Element', -1), 'return type Element')
        .toInclude('entity.name.type.nx');
      expectScopes(scopesAtLine(result, 0, 'Element', 1), 'content property type Element')
        .toInclude('entity.name.type.nx');
    });

    it('keeps the element scope on an element reference', function () {
      const result = tokenizeLines(grammar, ['<Button x=1 y=2/>']);
      expectScopes(scopesAtLine(result, 0, 'Button'), 'Button').toInclude('entity.name.tag.nx');
      expectScopes(scopesAtLine(result, 0, 'x'), 'x').toInclude('entity.other.attribute-name.nx');
    });
  });

  describe('property lists are scoped independent of layout', function () {
    const signature = [
      'export external component <Text extends UiCommon',
      '  format: TextFormat = plain',
      '  letterSpacing: float64 = 0.0',
      '  color: Color?',
      '  items: string[]?',
      '  content text: string',
      '/>'
    ];

    it('scopes a property with a user-defined type and a contextual default', function () {
      const result = tokenizeLines(grammar, signature);

      expectScopes(scopesAt(result, 'format:', 'format'), 'format').toInclude('variable.other.property.nx');
      expectScopes(scopesAt(result, 'format:', ':'), 'annotation colon')
        .toInclude('punctuation.separator.type.annotation.nx');
      expectScopes(scopesAt(result, 'format:', 'TextFormat'), 'TextFormat')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.other.attribute-name.nx');
      // The whole identifier is one token: its last character must not be split off.
      expect(tokenTextAt(result, 'format:', 'TextFormat'), 'TextFormat token span').to.equal('TextFormat');
      expectScopes(scopesAt(result, 'format:', 'plain'), 'plain').toInclude('variable.other.enummember.nx');
    });

    it('scopes a property with a primitive type and a numeric default', function () {
      const result = tokenizeLines(grammar, signature);

      expectScopes(scopesAt(result, 'letterSpacing', 'float64'), 'float64').toInclude('support.type.primitive.nx');
      expect(tokenTextAt(result, 'letterSpacing', 'float64'), 'float64 token span').to.equal('float64');
      expectScopes(scopesAt(result, 'letterSpacing', '='), '=').toInclude('keyword.operator.assignment.nx');
      expectScopes(scopesAt(result, 'letterSpacing', '0.0'), '0.0').toInclude('constant.numeric.float.nx');
    });

    it('scopes type suffixes', function () {
      const result = tokenizeLines(grammar, signature);

      expectScopes(scopesAt(result, 'color:', 'Color'), 'Color').toInclude('entity.name.type.nx');
      expectScopes(scopesAt(result, 'color:', '?'), '? after Color').toInclude('keyword.operator.type-modifier.nx');
      expectScopes(scopesAt(result, 'items:', 'string'), 'string').toInclude('support.type.primitive.nx');
      expectScopes(scopesAt(result, 'items:', '[]'), '[]').toInclude('keyword.operator.type-modifier.nx');
      expectScopes(scopesAt(result, 'items:', '?'), '? after []').toInclude('keyword.operator.type-modifier.nx');
    });

    it('scopes a content-marked property', function () {
      const result = tokenizeLines(grammar, signature);

      expectScopes(scopesAt(result, 'content text', 'content'), 'content').toInclude('storage.modifier.content.nx');
      expectScopes(scopesAt(result, 'content text', 'text'), 'text').toInclude('variable.other.property.nx');
      expectScopes(scopesAt(result, 'content text', 'string'), 'string').toInclude('support.type.primitive.nx');
    });

    it('scopes a single-line signature the same as a multi-line one', function () {
      const result = tokenizeLines(grammar, [
        'export external component <Text extends UiCommon format: TextFormat = plain />'
      ]);

      expectScopes(scopesAtLine(result, 0, 'format'), 'format').toInclude('variable.other.property.nx');
      expectScopes(scopesAtLine(result, 0, 'TextFormat'), 'TextFormat').toInclude('entity.name.type.nx');
      expectScopes(scopesAtLine(result, 0, 'plain'), 'plain').toInclude('variable.other.enummember.nx');
    });
  });

  describe('comments inside a signature', function () {
    it('scopes a trailing comment on a property line', function () {
      const result = tokenizeLines(grammar, [
        'export external component <Text extends UiCommon',
        '  fontWeight: float64?              // 1..1000',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'fontWeight', '// 1..1000'), 'trailing comment')
        .toInclude('comment.line.double-slash.nx');
    });

    it('scopes a comment containing signature-terminating characters', function () {
      const result = tokenizeLines(grammar, [
        'export external component <Text extends UiCommon',
        '  maxLines: int?                    // >= 1; null',
        '  overflow: TextOverflow = clip',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'maxLines', '// >='), 'comment start')
        .toInclude('comment.line.double-slash.nx');
      expectScopes(scopesAt(result, 'maxLines', '/'), 'comment slash')
        .toNotInclude('punctuation.definition.tag.self-closing.nx');
      expectScopes(scopesAt(result, 'maxLines', '>'), 'comment >')
        .toNotInclude('punctuation.definition.tag.end.nx');
      expectScopes(scopesAt(result, 'maxLines', 'null'), 'null in comment')
        .toInclude('comment.line.double-slash.nx')
        .toNotInclude('constant.language.null.nx');

      // The comment must not have ended the declaration.
      expectScopes(scopesAt(result, 'overflow', 'overflow'), 'overflow').toInclude('variable.other.property.nx');
      expectScopes(scopesAt(result, 'overflow', 'TextOverflow'), 'TextOverflow').toInclude('entity.name.type.nx');
    });
  });

  describe('emits and state groups', function () {
    it('scopes an emits group inside a component signature', function () {
      const result = tokenizeLines(grammar, [
        'export component <Input',
        '  value: string = ""',
        '  emits { ValueChanged extends InputAction { value: string } }',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'emits', 'emits'), 'emits').toInclude('keyword.declaration.emits.nx');
      expectScopes(scopesAt(result, 'emits', 'ValueChanged'), 'ValueChanged').toInclude('entity.name.type.nx');
      expectScopes(scopesAt(result, 'emits', 'extends'), 'extends').toInclude('keyword.declaration.extends.nx');
      expectScopes(scopesAt(result, 'emits', 'InputAction'), 'InputAction').toInclude('entity.name.type.nx');
      expectScopes(scopesAt(result, 'emits', 'value'), 'emitted payload property').toInclude('variable.other.property.nx');
    });

    it('scopes a state group inside a component body', function () {
      const result = tokenizeLines(grammar, [
        'component <SearchBox placeholder: string /> = {',
        '  state { query: string = "" }',
        '  <input value={query} />',
        '}'
      ]);

      expectScopes(scopesAt(result, 'state', 'state'), 'state').toInclude('keyword.declaration.state.nx');
      expectScopes(scopesAt(result, 'state', 'query'), 'query').toInclude('variable.other.property.nx');
      expectScopes(scopesAt(result, 'state', '""'), 'empty string').toInclude('string.quoted.double.nx');
    });
  });

  describe('terminate an open construct', function () {
    // The three "a new declaration starts here" lookaheads in the grammar have to agree on the
    // modifier set; a divergence silently swallows the rest of the file, which is how `external`
    // went unnoticed. One open construct per lookahead, times every declaration form.
    const openConstructs: { label: string; lines: string[] }[] = [
      { label: 'multi-line union', lines: ['export type LoadState =', '  | idle', '  | failed { message: string }'] },
      { label: 'brace-form union', lines: ['export type Shape = circle {', '  radius: float64', '}'] },
      { label: 'union case with a record body', lines: ['export type Size =', '  | fixed { value: float64 }'] }
    ];

    const prefixes = ['', 'private ', 'export ', 'abstract ', 'external ', 'export abstract ',
      'export external ', 'export abstract external '];

    const declarations: { label: string; line: string; keyword: string }[] = [
      { label: 'type', line: 'type Next = string', keyword: 'type' },
      { label: 'action', line: 'action Next = {', keyword: 'action' },
      { label: 'component', line: 'component <Next />', keyword: 'component' },
      { label: 'let', line: 'let next = 1', keyword: 'let' }
    ];

    for (const construct of openConstructs) {
      for (const prefix of prefixes) {
        for (const declaration of declarations) {
          const label = `${prefix}${declaration.label} after a ${construct.label}`;
          it(`terminates a ${construct.label} at \`${prefix}${declaration.label}\``, function () {
            const line = prefix + declaration.line;
            const result = tokenizeLines(grammar, [...construct.lines, line]);
            const scopes = scopesAtLine(result, construct.lines.length, declaration.keyword);

            expect(scopes, `${label}: got [${scopes.join(', ')}]`).to.not.satisfy((s: string[]) =>
              s.some(scope => scope.startsWith('meta.definition.type.union'))
            );
          });
        }
      }
    }
  });

  describe('scopes end at the declaration terminator', function () {
    it('scopes the signature terminator as tag punctuation', function () {
      const result = tokenizeLines(grammar, [
        'export external component <Text extends UiCommon',
        '  format: TextFormat = plain',
        '/>'
      ]);

      expectScopes(scopesAt(result, '/>', '/'), 'terminator slash')
        .toInclude('punctuation.definition.tag.self-closing.nx')
        .toNotInclude('keyword.operator.arithmetic.nx');
      expectScopes(scopesAt(result, '/>', '>'), 'terminator >')
        .toInclude('punctuation.definition.tag.end.nx')
        .toNotInclude('keyword.operator.comparison.nx');
    });

    it('scopes the terminator of a single-line signature with a body as tag punctuation', function () {
      const result = tokenizeLines(grammar, ['private component <SearchBox placeholder: string /> = {', '}']);

      expectScopes(scopesAtLine(result, 0, '/>'), 'terminator >')
        .toInclude('punctuation.definition.tag.end.nx')
        .toNotInclude('keyword.operator.comparison.nx');
      expectScopes(scopesAtLine(result, 0, '/'), 'terminator slash')
        .toInclude('punctuation.definition.tag.self-closing.nx')
        .toNotInclude('keyword.operator.arithmetic.nx');
    });

    it('terminates a preceding multi-line union at an external declaration', function () {
      const result = tokenizeLines(grammar, [
        'export type TrackSize =',
        '  | auto',
        '  | fixed { value: float64 }',
        '',
        'export external component <Icon extends UiCommon',
        '  name: string',
        '/>'
      ]);

      const icon = scopesAt(result, 'component', 'Icon');
      expectScopes(icon, 'Icon')
        .toInclude('entity.name.type.nx')
        .toNotInclude('meta.definition.type.union.case.nx');
      expect(
        scopesAt(result, 'name: string', 'name'),
        'a property after a union should not carry union-case scope'
      ).to.not.include('meta.definition.type.union.case.nx');
    });

    it('scopes a declaration the same wherever it appears in a file', function () {
      const declaration = [
        'export external component <Icon extends UiCommon',
        '  name: string',
        '  size: float64 = 16.0',
        '/>'
      ];
      const preamble = [
        'export type TrackSize =',
        '  | auto',
        '  | fixed { value: float64 }',
        '',
        'export type Fit = contain | cover',
        '',
        'export abstract external component <UiCommon',
        '  width: float64?',
        '/>',
        ''
      ];

      const alone = tokenizeLines(grammar, declaration);
      const inFile = tokenizeLines(grammar, [...preamble, ...declaration]).slice(preamble.length);

      for (let i = 0; i < declaration.length; i++) {
        expect(
          inFile[i].tokens.map(t => `${t.startIndex}-${t.endIndex}:${t.scopes.join(' ')}`),
          `line ${i}: ${declaration[i]}`
        ).to.deep.equal(alone[i].tokens.map(t => `${t.startIndex}-${t.endIndex}:${t.scopes.join(' ')}`));
      }
    });

    it('recovers at the next declaration when a signature is never terminated', function () {
      const result = tokenizeLines(grammar, [
        'export component <Broken',
        '  label: string',
        '',
        'export external component <Next />'
      ]);

      expectScopes(scopesAt(result, 'Next', 'external'), 'external after an unterminated signature')
        .toInclude('storage.modifier.external.nx');
      expectScopes(scopesAt(result, 'Next', 'Next'), 'Next after an unterminated signature')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
    });

    it('recovers at the next declaration when a record body is never closed', function () {
      const result = tokenizeLines(grammar, [
        'export type Broken = {',
        '  a: string',
        '',
        'export external component <Next />'
      ]);

      expectScopes(scopesAt(result, 'Next', 'Next'), 'Next after an unclosed record body')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
    });

    it('recovers at the next declaration through an unterminated nested default', function () {
      // A child context that outlives its line blocks every boundary above it, so recovery has to
      // reach the braced expressions, control forms, and element references a default can open.
      const openers: [string, string[]][] = [
        ['braced default', ['component <Broken', '  x: Element = {']],
        ['braced default in a record', ['type Broken = {', '  x: Element = {']],
        ['if inside a default', ['component <Broken', '  x: Element = { if ready {']],
        ['for inside a body', ['component <Broken />', '= { for item in items {']],
        ['embed braced expression', ['component <Broken', '  x: Element = @{']],
        ['function right-hand side', ['let <Row /> = {', '  if ready {']],
        ['value right-hand side', ['let x = {', '  if ready {']],
        ['element reference', ['component <Broken', '  x: Element = { <Foo']],
        ['if in a property list', ['component <Broken />', '= { <Foo', '  if ready {']]
      ];

      // Indentation must not matter: a declaration ends the previous one wherever it is written.
      // Only a *bare* `let` is ambiguous with a nested binding, so it is covered separately below.
      const recoveries = ['export external component <Next />', '  export external component <Next />'];

      for (const [label, opener] of openers) {
        for (const recovery of recoveries) {
          const indented = recovery.startsWith(' ') ? 'indented' : 'column-0';
          const result = tokenizeLines(grammar, [...opener, '', recovery]);
          expectScopes(scopesAt(result, 'Next', 'external'), `external after an unterminated ${label} (${indented})`)
            .toInclude('storage.modifier.external.nx');
          expectScopes(scopesAt(result, 'Next', 'Next'), `Next after an unterminated ${label} (${indented})`)
            .toInclude('entity.name.type.nx')
            .toNotInclude('entity.name.tag.nx');
        }
      }
    });

    it('recovers at an indented declaration of every form that cannot be nested', function () {
      // `type`, `action`, and `component` have no production outside the module top level, and a
      // binding never carries a visibility modifier, so all of these recover at any indentation.
      const declarations: [string, string, string][] = [
        ['type', '  type Next = string', 'Next'],
        ['action', '  action Next = { value: string }', 'Next'],
        ['component', '  export external component <Next />', 'Next'],
        ['modified let', '  export let <Next /> = 1', 'Next']
      ];

      for (const [label, declaration, name] of declarations) {
        const result = tokenizeLines(grammar, ['component <Broken', '  x: Element = {', '', declaration]);
        expectScopes(scopesAt(result, name, name), `${label} declared at an indent`)
          .toInclude('entity.name.type.nx')
          .toNotInclude('entity.name.tag.nx');
      }
    });

    it('does not mistake an indented nested binding for a new declaration', function () {
      // Inside an expression an indented `let` is a nested binding, not a new top-level
      // declaration (`src/vscode/samples/tally-survey.nx:120`). Recovery there requires column 0,
      // so the enclosing braced expression survives.
      const result = tokenizeLines(grammar, [
        'let outer = {',
        '  <Option "Yes"/>',
        '  let myOptions = {',
        '    <Option "No"/>',
        '  }',
        '}'
      ]);

      expectScopes(scopesAtLine(result, 2, 'myOptions'), 'an indented nested binding')
        .toInclude('meta.values-braced-expression.nx');
      expectScopes(scopesAtLine(result, 3, 'Option'), 'an element after a nested binding')
        .toInclude('meta.values-braced-expression.nx');
    });

    it('does not mistake an attribute named after a keyword for a new declaration', function () {
      // Recovery keys on a declaration keyword at the start of a line, and an attribute may be
      // named `type` (`samples/tally-survey.nx:11`). An attribute name is always followed by `=`
      // or `:` and a declaration keyword never is, which is what keeps the two apart.
      const result = tokenizeLines(grammar, [
        '<Question',
        '  type = "multiple"',
        '  label = "x"',
        '/>'
      ]);

      expectScopes(scopesAtLine(result, 1, 'type'), 'an attribute named `type`')
        .toInclude('entity.other.attribute-name.nx')
        .toNotInclude('keyword.declaration.type.nx');
      expectScopes(scopesAtLine(result, 1, '"multiple"'), 'the value of an attribute named `type`')
        .toInclude('string.quoted.double.nx');
      expectScopes(scopesAtLine(result, 2, 'label'), 'attribute after a `type =` line')
        .toInclude('entity.other.attribute-name.nx');
      expectScopes(scopesAtLine(result, 3, '/'), 'terminator after a `type =` line')
        .toInclude('punctuation.definition.tag.self-closing.nx');
    });

    it('scopes a qualified declaration name as a declared type', function () {
      const component = tokenizeLines(grammar, ['component <Ns.Widget value: string />', '= { <Foo /> }']);
      expectScopes(scopesAt(component, 'Ns.Widget', 'Ns.Widget'), 'qualified component name')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
      expect(tokenTextAt(component, 'Ns.Widget', 'Ns.Widget'), 'qualified name span').to.equal('Ns.Widget');
      expectScopes(scopesAt(component, 'Ns.Widget', 'value'), 'property of a qualified component')
        .toInclude('variable.other.property.nx');

      const fn = tokenizeLines(grammar, ['export let <Ns.Row item: string /> : Element = <HStack />']);
      expectScopes(scopesAt(fn, 'Ns.Row', 'Ns.Row'), 'qualified element-function name')
        .toInclude('entity.name.type.nx')
        .toNotInclude('entity.name.tag.nx');
    });
  });

  describe('group properties follow the signature property rules', function () {
    it('scopes a content-marked property in an emits group', function () {
      const result = tokenizeLines(grammar, [
        'component <C',
        '  emits { Changed { content text: string } }',
        '/>'
      ]);

      expectScopes(scopesAt(result, 'emits', 'content'), 'content in an emits group')
        .toInclude('storage.modifier.content.nx')
        .toNotInclude('entity.name.qualifier.nx');
      expectScopes(scopesAt(result, 'emits', 'text'), 'text in an emits group')
        .toInclude('variable.other.property.nx');
    });

    it('scopes a content-marked property in a state group', function () {
      const result = tokenizeLines(grammar, [
        'component <C />',
        '= {',
        '  state { content child: Element }',
        '}'
      ]);

      expectScopes(scopesAt(result, 'state', 'content'), 'content in a state group')
        .toInclude('storage.modifier.content.nx')
        .toNotInclude('entity.name.qualifier.nx');
      expectScopes(scopesAt(result, 'state', 'child'), 'child in a state group')
        .toInclude('variable.other.property.nx');
    });
  });

  describe('scopes end at the declaration terminator, whitespace included', function () {
    it('closes the component at a `/ >` terminator', function () {
      const result = tokenizeLines(grammar, [
        'component <C / > = {',
        '  state { value: string }',
        '}'
      ]);

      expectScopes(scopesAt(result, 'state', 'state'), 'state after a `/ >` terminator')
        .toInclude('keyword.declaration.state.nx');
      expect(
        scopesAt(result, 'state', 'state'),
        'the component scope should not survive its signature'
      ).to.not.include('meta.definition.component.nx');
    });
  });
});
