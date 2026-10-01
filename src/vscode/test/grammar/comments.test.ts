// A `//` comment is a comment wherever code can appear, and nothing inside it is code.
import * as fs from 'fs';
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { expectScopes, grammarPath, loadGrammar, scopesAt, tokenizeLines } from './helpers.js';

describe('NX comments', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
  });

  // Each case puts a comment where the enclosing rule also has patterns that match `/` or a bare
  // word, which is where a mis-ordered `#comments` include shows up.
  const trailing: { label: string; lines: string[]; find: string }[] = [
    {
      label: 'a record property',
      lines: ['export type Accessibility = {', '  hidden: boolean = false   // true means decorative / ignored', '}'],
      find: 'hidden'
    },
    {
      label: 'a record property without a default',
      lines: ['type P = {', '  a: int   // 10 / 2 or true', '}'],
      find: 'a: int'
    },
    {
      label: 'a value definition',
      lines: ['let total: int   // true if unset'],
      find: 'total'
    },
    {
      label: 'a value definition with an initializer',
      lines: ['let total: int = 42   // true if unset'],
      find: 'total'
    },
    {
      label: 'a declaration signature',
      lines: ['export external component <Text', '  maxLines?: int   // >= 1 / null; true means unbounded', '/>'],
      find: 'maxLines'
    },
    {
      label: 'a union case',
      lines: ['export type TrackSize =', '  | fraction { value: float64 }   // CSS `fr` units / not true pixels'],
      find: 'fraction'
    }
  ];

  for (const { label, lines, find } of trailing) {
    it(`scopes a trailing comment on ${label}`, function () {
      const result = tokenizeLines(grammar, lines);
      const comment = lines.find(l => l.includes('//'))!;
      const text = comment.slice(comment.indexOf('//'));

      // The whole comment is one token, so no word inside it can carry a code scope.
      const entry = result.find(r => r.line === comment)!;
      const token = entry.tokens.find(t => t.startIndex === comment.indexOf('//'));
      expect(token && comment.slice(token.startIndex, token.endIndex), `${label}: comment span`)
        .to.equal(text);
      expectScopes(scopesAt(result, find, '//'), `${label}: comment`)
        .toInclude('comment.line.double-slash.nx')
        .toNotInclude('keyword.operator.arithmetic.nx');
      expectScopes(scopesAt(result, find, 'true'), `${label}: "true" inside the comment`)
        .toInclude('comment.line.double-slash.nx')
        .toNotInclude('constant.language.boolean.nx', 'entity.name.qualifier.nx');
    });
  }

  it('keeps `//` as literal text inside text content', function () {
    // Comments are not recognized inside text content (nx-grammar-spec.md, "Comments are not
    // recognized inside string literals or text content tokens").
    const result = tokenizeLines(grammar, ['<p:>', '  not a // comment, and true is just a word', '</p>']);
    expectScopes(scopesAt(result, 'not a', '//'), 'text content')
      .toNotInclude('comment.line.double-slash.nx');
  });

  it('lists #comments ahead of any rule that also matches at `//`', function () {
    // A structural guard for the ordering the cases above depend on: TextMate breaks a
    // same-position tie by list order, so `#operators` listed first claims the slashes.
    const greedy = new Set(['#operators', '#qualifiers', '#keywords-core', '#contextual-name', '#types', '#attr-value']);
    // Text-content contexts are exempt: `//` is literal text there, per the spec note above.
    const exempt = new Set(['text-raw-block', 'text-typed-block', 'text-plain-block']);

    const grammarJson = JSON.parse(fs.readFileSync(grammarPath, 'utf8'));
    const offenders: string[] = [];

    const visit = (node: unknown, path: string): void => {
      if (Array.isArray(node)) {
        node.forEach((item, i) => visit(item, `${path}[${i}]`));
        return;
      }
      if (!node || typeof node !== 'object') return;

      const rule = node as Record<string, unknown>;
      if (Array.isArray(rule.patterns) && !exempt.has(path.split('.')[2])) {
        const includes: (string | undefined)[] = rule.patterns.map(p =>
          p && typeof p === 'object' ? (p as Record<string, unknown>).include as string | undefined : undefined
        );
        const commentIndex = includes.indexOf('#comments');
        if (commentIndex > 0) {
          const before = includes.slice(0, commentIndex).filter(i => i !== undefined && greedy.has(i));
          if (before.length > 0) offenders.push(`${path}: #comments after ${before.join(', ')}`);
        }
      }

      for (const [key, value] of Object.entries(rule)) visit(value, `${path}.${key}`);
    };

    visit(grammarJson.repository, '$.repository');
    expect(offenders, `contexts where a comment loses to a code rule:\n${offenders.join('\n')}`)
      .to.deep.equal([]);
  });

  describe('doc comments', function () {
    const DOC = 'comment.line.documentation.nx';
    const LINE = 'comment.line.double-slash.nx';

    it('scopes a leading doc comment as documentation from `///` to the end of the line', function () {
      const line = '/// A text box.';
      const result = tokenizeLines(grammar, [line, 'component <SearchBox />']);
      const tokens = result[0].tokens;
      expect(tokens.map(t => line.slice(t.startIndex, t.endIndex))).to.deep.equal(['///', ' A text box.']);
      for (const token of tokens) {
        expectScopes(token.scopes, 'leading doc comment').toInclude(DOC).toNotInclude(LINE);
      }
      expectScopes(tokens[0].scopes, '///').toInclude('punctuation.definition.comment.nx');
    });

    it('scopes a trailing doc comment on a property and keeps the declaration open', function () {
      const result = tokenizeLines(grammar, [
        'export external component <SearchBox',
        '  placeholder:string   /// Hint text; shown when empty.',
        '  tone: string',
        '/>'
      ]);
      expectScopes(scopesAt(result, 'placeholder', '/// Hint'), 'doc comment start').toInclude(DOC);
      expectScopes(scopesAt(result, 'placeholder', ';'), 'semicolon in the doc comment')
        .toInclude(DOC);
      // The doc comment must not have ended the declaration.
      expectScopes(scopesAt(result, 'tone', 'tone'), 'tone').toInclude('variable.other.property.nx');
    });

    describe('Markdown', function () {
      const LINK = 'markup.underline.link.reference.nx';
      const LINK_TEXT = 'markup.underline.link.text.nx';
      const RAW = 'markup.inline.raw.nx';
      const BOLD = 'markup.bold.nx';
      const ITALIC = 'markup.italic.nx';

      function scopes(line: string, substring: string, occurrence = 1): string[] {
        return scopesAt(tokenizeLines(grammar, [line]), line, substring, occurrence);
      }

      it('scopes a doc link and a member path as a link', function () {
        const line = '/// See [renderNotice] and [LoadState.idle], or [`Grid`].';
        expectScopes(scopes(line, 'renderNotice'), 'name').toInclude(DOC, LINK);
        expectScopes(scopes(line, 'LoadState.idle'), 'member path').toInclude(DOC, LINK);
        expectScopes(scopes(line, '`Grid`'), 'name in backticks').toInclude(LINK);
        expectScopes(scopes('/// Read [Box.aria-label] first.', 'Box.aria-label'), 'hyphenated member')
          .toInclude(LINK);
        expectScopes(scopes(line, '['), 'bracket').toInclude('punctuation.definition.link.title.begin.nx');
      });

      it('scopes a link with a destination, and nothing that is not a link', function () {
        const line = '/// Read [the spec](https://nxlang.org) on [plain words] or [x][y].';
        expectScopes(scopes(line, 'the spec'), 'link text').toInclude(LINK_TEXT).toNotInclude(LINK);
        expectScopes(scopes(line, 'https://nxlang.org'), 'destination').toInclude('markup.underline.link.nx');
        expectScopes(scopes(line, 'plain words'), 'a phrase in brackets').toInclude(DOC).toNotInclude(LINK);
        expectScopes(scopes(line, '[x]'), 'a full reference').toNotInclude(LINK);
      });

      it('scopes a code span as code, with no link inside it', function () {
        const line = '/// Read `items[index]` and ``a ` b`` first.';
        expectScopes(scopes(line, '`items[index]`'), 'code span').toInclude(DOC, RAW).toNotInclude(LINK);
        expectScopes(scopes(line, 'index'), 'brackets in code').toNotInclude(LINK);
        expectScopes(scopes(line, '``a ` b``'), 'double-backtick code span').toInclude(RAW);
      });

      it('scopes strong emphasis and emphasis, keeping the markers', function () {
        const line = '/// Use **only** once, *or* _twice_, **with [Grid]**.';
        expectScopes(scopes(line, '**only**'), 'strong').toInclude(DOC, BOLD);
        expectScopes(scopes(line, '*or*'), 'emphasis').toInclude(DOC, ITALIC).toNotInclude(BOLD);
        expectScopes(scopes(line, '_twice_'), 'underscore emphasis').toInclude(ITALIC);
        expectScopes(scopes(line, 'Grid'), 'a link inside strong').toInclude(BOLD, LINK);
      });

      it('does not read arithmetic or snake_case as emphasis', function () {
        const line = '/// Computes a * b * c for snake_case_name and 2*3.';
        for (const part of ['b', 'case', '3']) {
          expectScopes(scopes(line, part), part).toInclude(DOC).toNotInclude(ITALIC, BOLD);
        }
      });

      it('leaves an ordinary comment plain', function () {
        const line = '// Use **only** [Grid] and `code`.';
        for (const part of ['only', 'Grid', 'code']) {
          expectScopes(scopes(line, part), part).toInclude(LINE).toNotInclude(BOLD, LINK, RAW);
        }
      });
    });

    it('keeps four slashes an ordinary comment', function () {
      for (const line of ['//// Section', '//////////']) {
        const result = tokenizeLines(grammar, [line]);
        expectScopes(result[0].tokens[0].scopes, line).toInclude(LINE).toNotInclude(DOC);
      }
    });

    it('keeps `///` as literal text inside text content', function () {
      const result = tokenizeLines(grammar, ['<p:>', '  a /// b', '</p>']);
      expectScopes(scopesAt(result, 'a /// b', '///'), 'text content')
        .toNotInclude(DOC)
        .toNotInclude(LINE);
    });
  });
});
