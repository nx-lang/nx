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
      lines: ['export external component <Text', '  maxLines: int?   // >= 1 / null; true means unbounded', '/>'],
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
});
