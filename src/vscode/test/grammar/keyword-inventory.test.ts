// Keeps the TextMate grammar's keyword set in step with the tree-sitter grammar, which is the
// authoritative token inventory. `external` was absent from the TextMate grammar for as long as it
// existed in tree-sitter; this check is what catches the next such gap.
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import { grammarPath } from './helpers.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const treeSitterGrammarPath = path.join(
  __dirname, '..', '..', '..', '..', 'crates', 'nx-syntax', 'grammar.js'
);

/** The lowercase keyword literals the tree-sitter grammar matches as terminals. */
function treeSitterKeywords(source: string): string[] {
  const withoutComments = source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/[^\n]*/g, '');

  const terminalsOnly = withoutComments
    // `field('name', …)` and the grammar's own `name:` are metadata, not terminals.
    .replace(/field\(\s*'[^']*'\s*,/g, 'field(')
    .replace(/\bname:\s*'[^']*'/g, 'name:');

  const keywords = new Set<string>();
  for (const [, literal] of terminalsOnly.matchAll(/'([a-z][a-z0-9_]*)'/g)) {
    keywords.add(literal);
  }

  return [...keywords].sort();
}

/**
 * Every regex the TextMate grammar assigns a scope with.
 *
 * <para>Only `match` and `begin` of a rule that names a scope counts. An `end` lookahead mentioning
 * a keyword proves only that the grammar knows where a construct stops, not that it colours the
 * keyword — the exact gap `external` sat in.</para>
 */
function scopeAssigningPatterns(node: unknown, out: string[] = []): string[] {
  if (Array.isArray(node)) {
    for (const item of node) scopeAssigningPatterns(item, out);
    return out;
  }

  if (!node || typeof node !== 'object') {
    return out;
  }

  const rule = node as Record<string, unknown>;
  const assignsScope = 'name' in rule || 'captures' in rule || 'beginCaptures' in rule;

  for (const [key, value] of Object.entries(rule)) {
    if (key === 'match' || key === 'begin') {
      if (assignsScope && typeof value === 'string') out.push(value);
    } else if (key !== 'end' && key !== 'name' && key !== 'comment') {
      scopeAssigningPatterns(value, out);
    }
  }

  return out;
}

describe('NX grammar keyword inventory', function () {
  it('matches every keyword the tree-sitter grammar defines', function () {
    if (!fs.existsSync(treeSitterGrammarPath)) {
      this.skip();
      return;
    }

    const keywords = treeSitterKeywords(fs.readFileSync(treeSitterGrammarPath, 'utf8'));
    expect(keywords.length, 'no keywords extracted from the tree-sitter grammar').to.be.greaterThan(20);

    // `\\b` reads as a word character to the boundary check below, so drop the escapes first.
    const patterns = scopeAssigningPatterns(JSON.parse(fs.readFileSync(grammarPath, 'utf8')))
      .map(pattern => pattern.replace(/\\./g, ' '));

    const missing = keywords.filter(keyword => {
      const asWord = new RegExp(`(?<![A-Za-z0-9_])${keyword}(?![A-Za-z0-9_])`);
      return !patterns.some(pattern => asWord.test(pattern));
    });

    expect(missing, `keywords the TextMate grammar never matches: ${missing.join(', ')}`).to.deep.equal([]);
  });
});
