// Whole-file regression: a declaration must tokenize the same in isolation as it does in situ.
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import { loadGrammar, tokenizeLines, type TokenizedLine } from './helpers.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const corpusPath = path.join(__dirname, '..', '..', '..', '..', 'docs', 'drawnui-proposal', 'ui', 'ui.nx');

/**
 * The line indexes on which an element-shaped declaration starts.
 *
 * <para>A `component` is always element-shaped, so its signature's `<` need not be on the keyword's
 * line; a `let` is element-shaped only when followed by `<`. Requiring the `<` on the keyword line
 * for both would skip a declaration whose signature opens on the next line.</para>
 */
function declarationStarts(lines: string[]): number[] {
  const opens = /^\s*(?:(?:private|export)\s+)?(?:abstract\s+)?(?:external\s+)?(?:component\b|let\s+<)/;
  return lines.flatMap((line, index) => (opens.test(line) ? [index] : []));
}

/**
 * Line ranges, inclusive of both ends, of each element-shaped declaration.
 *
 * <para>The end comes from the tokenizer's own state rather than from the first textual `/>`, which
 * can be an element-valued property default rather than the signature's terminator.</para>
 */
function declarationRanges(lines: string[], inFile: TokenizedLine[]): { start: number; end: number }[] {
  const declaration = /^meta\.definition\.(?:component|function)\.nx$/;
  const carries = (index: number) =>
    inFile[index].tokens.some(token => token.scopes.some(scope => declaration.test(scope)));

  const starts = declarationStarts(lines);
  return starts.map((start, i) => {
    expect(carries(start), `${corpusPath}:${start + 1} was not scoped as a declaration:\n${lines[start]}`)
      .to.equal(true);
    const limit = i + 1 < starts.length ? starts[i + 1] : lines.length;
    let end = start;
    while (end + 1 < limit && carries(end + 1)) end++;
    return { start, end };
  });
}

function fingerprint(result: TokenizedLine[]): string[] {
  return result.flatMap(({ line, tokens }) =>
    tokens.map(t => `${line.slice(t.startIndex, t.endIndex)} :: ${t.scopes.join(' ')}`)
  );
}

describe('NX grammar corpus regression', function () {
  let grammar: IGrammar;
  let lines: string[] | null = null;

  before(async function () {
    grammar = await loadGrammar();
    if (fs.existsSync(corpusPath)) {
      lines = fs.readFileSync(corpusPath, 'utf8').split(/\r?\n/);
    }
  });

  it('scopes every declaration the same in isolation as in the whole file', function () {
    if (!lines) {
      this.skip();
      return;
    }

    const inFile = tokenizeLines(grammar, lines);
    const ranges = declarationRanges(lines, inFile);
    expect(ranges.length, `no element-shaped declarations found in ${corpusPath}`).to.be.greaterThan(0);
    // A declaration whose range collapsed to its keyword line was not recognized as element-shaped.
    const text = lines;
    const collapsed = ranges.filter(r => r.end === r.start && !text[r.start].includes('/>'));
    expect(collapsed.map(r => `${corpusPath}:${r.start + 1}`), 'declarations that never opened a signature')
      .to.deep.equal([]);

    for (const { start, end } of ranges) {
      const declaration = lines.slice(start, end + 1);
      const alone = fingerprint(tokenizeLines(grammar, declaration));
      const situ = fingerprint(inFile.slice(start, end + 1));
      expect(situ, `${corpusPath}:${start + 1}-${end + 1}\n${declaration.join('\n')}`).to.deep.equal(alone);
    }
  });

  it('leaves no token in the file without a token scope', function () {
    if (!lines) {
      this.skip();
      return;
    }

    // A token whose only scopes are the root and container (`meta.*`) scopes carries no colour of
    // its own: the grammar matched the region but never the token inside it.
    const isContainer = (scope: string) => scope === 'source.nx' || scope.startsWith('meta.');

    const unscoped = tokenizeLines(grammar, lines).flatMap(({ line, tokens }, index) =>
      tokens
        .filter(t => t.scopes.every(isContainer) && line.slice(t.startIndex, t.endIndex).trim().length > 0)
        .map(t => `${index + 1}: ${JSON.stringify(line.slice(t.startIndex, t.endIndex))}`)
    );

    expect(unscoped, `tokens with no token scope:\n${unscoped.join('\n')}`).to.deep.equal([]);
  });
});
