import * as fs from 'fs';
import * as path from 'path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';
import type { IGrammar, IToken, StateStack } from 'vscode-textmate';

const cjsRequire = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const onig: any = cjsRequire('vscode-oniguruma');
const vsctm: any = cjsRequire('vscode-textmate');

async function loadGrammar(): Promise<IGrammar> {
  const wasmPath = cjsRequire.resolve('vscode-oniguruma/release/onig.wasm');
  const wasmBin = fs.readFileSync(wasmPath).buffer;
  await onig.loadWASM(wasmBin);

  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns: string[]) => new onig.OnigScanner(patterns),
      createOnigString: (s: string) => new onig.OnigString(s)
    }),
    loadGrammar: async (scopeName: string) => {
      if (scopeName !== 'source.nx') return null as any;
      const grammarPath = path.join(__dirname, '..', '..', 'syntaxes', 'nx.tmLanguage.json');
      const content = fs.readFileSync(grammarPath, 'utf8');
      return vsctm.parseRawGrammar(content, grammarPath);
    }
  });

  const grammar = await registry.loadGrammar('source.nx');
  if (!grammar) throw new Error('Failed to load NX grammar');
  return grammar;
}

function scopesForSubstring(line: string, tokens: IToken[], substring: string): string[] {
  const idx = line.indexOf(substring);
  if (idx === -1) return [];
  const pos = idx + Math.floor(substring.length / 2);
  const token = tokens.find(t => t.startIndex <= pos && pos < t.endIndex);
  return token ? token.scopes : [];
}

describe('NX TextMate text elements', function () {
  let grammar: IGrammar;

  before(async function () {
    grammar = await loadGrammar();
    expect(grammar).to.exist;
  });

  it('highlights raw keyword and TextType in <tag:text raw> blocks', function () {
    const lines = [
      '<style:text raw>',
      '  Hello {world} \\{escaped\\}',
      '</style>'
    ];

    let ruleStack: StateStack | null = null;

    const tokens0 = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = tokens0.ruleStack;

    expect(scopesForSubstring(lines[0], tokens0.tokens, ':text')).to.include('support.type.text.nx');
    expect(scopesForSubstring(lines[0], tokens0.tokens, 'raw')).to.include('keyword.other.raw.nx');
  });

  it('scopes the TextType when present', function () {
    const line = '<markdown:text content="*Hello*"></markdown>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, ':text')).to.include('support.type.text.nx');
  });

  it('tokenizes @{ } interpolation inside typed text content', function () {
    const lines = [
      '<markdown:text>',
      'Hello @{user}!',
      '</markdown>'
    ];

    let ruleStack: StateStack | null = null;
    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    const tokens = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = tokens.ruleStack;

    expect(scopesForSubstring(lines[1], tokens.tokens, '@{')).to.include('punctuation.section.interpolation.begin.nx');
    expect(scopesForSubstring(lines[1], tokens.tokens, 'user')).to.include('meta.interpolation.nx');
  });

  it('treats <tag: ...> blocks without TextType as plain text content with { } interpolation', function () {
    const lines = [
      '<message: prop="one">',
      '  Plain text {name} \\{escaped\\}',
      '</message>'
    ];

    let ruleStack: StateStack | null = null;
    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    const tokens = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = tokens.ruleStack;

    expect(scopesForSubstring(lines[1], tokens.tokens, '{')).to.include('punctuation.section.interpolation.begin.nx');
    expect(scopesForSubstring(lines[1], tokens.tokens, '\\{')).to.include('constant.character.escape.nx');
  });

  it('treats \\@ as an escape inside typed text content', function () {
    const lines = [
      '<markdown:text>',
      'Email \\@\\{user\\}@example.com',
      '</markdown>'
    ];

    let ruleStack: StateStack | null = null;
    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    const tokens = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = tokens.ruleStack;

    expect(scopesForSubstring(lines[1], tokens.tokens, '\\@')).to.include('constant.character.escape.nx');
    expect(scopesForSubstring(lines[1], tokens.tokens, '\\{')).to.include('constant.character.escape.nx');
  });

  it('scopes the colon when TextType is omitted', function () {
    const line = '<style: raw prop="value"></style>';
    const { tokens } = grammar.tokenizeLine(line, null);
    expect(scopesForSubstring(line, tokens, ':')).to.include('support.type.text.nx');
  });

  it('recognizes the closing tag after typed text content', function () {
    const lines = [
      '<p:ss>',
      '  This is some text',
      '</p>'
    ];

    let ruleStack: StateStack | null = null;
    const start = grammar.tokenizeLine(lines[0], ruleStack);
    ruleStack = start.ruleStack;
    const middle = grammar.tokenizeLine(lines[1], ruleStack);
    ruleStack = middle.ruleStack;
    const end = grammar.tokenizeLine(lines[2], ruleStack);

    const scopes = scopesForSubstring(lines[2], end.tokens, '</p>');
    expect(scopes).to.include('meta.tag.end.nx');
    expect(scopes).to.include('entity.name.tag.nx');
  });

  it('highlights attributes on text elements', function () {
    const line = '<textelement: val="3">slkdjfsdlkfj</textelement>';
    const { tokens } = grammar.tokenizeLine(line, null);

    expect(scopesForSubstring(line, tokens, 'val')).to.include('entity.other.attribute-name.nx');
    expect(scopesForSubstring(line, tokens, '=')).to.include('keyword.operator.assignment.nx');
    expect(scopesForSubstring(line, tokens, '"3"')).to.include('string.quoted.double.nx');
  });

  it('handles trailing backslash in text content', function () {
    const line = '<p:>Hello \\</p>';
    const { tokens } = grammar.tokenizeLine(line, null);
    // The backslash should be treated as a literal or escape, but definitely not break the tokenizer
    // In this specific case, since it's at the end of content before a tag, it might be tricky.
    // Let's just ensure the tag is still recognized.
    expect(scopesForSubstring(line, tokens, '</p>')).to.include('meta.tag.end.nx');
  });

  // ============================================================================
  // Text Child Element Tests
  // ============================================================================

  it('highlights child elements inside text content', function () {
    const line = '<p:>Hello <b>world</b>!</p>';
    const { tokens } = grammar.tokenizeLine(line, null);

    // The <b> tag should be recognized
    expect(scopesForSubstring(line, tokens, '<b>')).to.include('entity.name.tag.nx');
    // The closing </b> should be part of the child element with proper tag scope
    expect(scopesForSubstring(line, tokens, '</b>')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(line, tokens, '</b>')).to.include('meta.text.child-element.nx');
    // The outer closing tag should still work
    expect(scopesForSubstring(line, tokens, '</p>')).to.include('meta.tag.end.nx');
  });

  it('highlights self-closing child elements in text content', function () {
    const line = '<p:>Line<br />break</p>';
    const { tokens } = grammar.tokenizeLine(line, null);

    // The <br /> should be recognized as a tag
    expect(scopesForSubstring(line, tokens, 'br')).to.include('entity.name.tag.nx');
  });

  it('highlights nested child elements in text content', function () {
    const lines = [
      '<p:>Start <b>bold <i>italic</i> bold</b> end</p>'
    ];

    const { tokens } = grammar.tokenizeLine(lines[0], null);

    // Both b and i should be recognized as tags
    expect(scopesForSubstring(lines[0], tokens, '<b>')).to.include('entity.name.tag.nx');
    expect(scopesForSubstring(lines[0], tokens, '<i>')).to.include('entity.name.tag.nx');
  });

  it('highlights child elements with attributes in text content', function () {
    const line = '<p:>Click <a href="link">here</a></p>';
    const { tokens } = grammar.tokenizeLine(line, null);

    // The <a> tag should be recognized
    expect(scopesForSubstring(line, tokens, '<a')).to.include('entity.name.tag.nx');
    // The href attribute should be highlighted
    expect(scopesForSubstring(line, tokens, 'href')).to.include('entity.other.attribute-name.nx');
    // The string value should be highlighted
    expect(scopesForSubstring(line, tokens, '"link"')).to.include('string.quoted.double.nx');
  });

  it('handles interpolation inside child elements in text content', function () {
    const line = '<p:>Hello <b>{name}</b>!</p>';
    const { tokens } = grammar.tokenizeLine(line, null);

    // Interpolation should work inside child elements
    expect(scopesForSubstring(line, tokens, '{')).to.include('punctuation.section.interpolation.begin.nx');
    expect(scopesForSubstring(line, tokens, 'name')).to.include('meta.interpolation.nx');
  });
});
