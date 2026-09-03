import { expect } from 'chai';
import type { IGrammar } from 'vscode-textmate';
import {
  loadGrammar,
  scopesAt,
  scopesForSubstring as scopesForTokens,
  tokenizeLines
} from './helpers.js';

function hasScopeAt(line: string, substr: string, scopes: string[], grammar: IGrammar): boolean {
  const { tokens } = grammar.tokenizeLine(line, null);
  const actual = scopesForTokens(line, tokens, substr);
  return actual.length > 0 && scopes.every((s) => actual.includes(s));
}

function scopesForSubstring(line: string, substr: string, grammar: IGrammar): string[] {
  const { tokens } = grammar.tokenizeLine(line, null);
  return scopesForTokens(line, tokens, substr);
}

describe('NX control blocks', () => {
  let grammar: IGrammar;

  before(async () => {
    grammar = await loadGrammar();
  });

  it('highlights braces in elements-if single-line block', () => {
    const line = 'if cond { <Spinner/> }';
    expect(hasScopeAt(line, 'if', ['keyword.control.conditional.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '{', ['punctuation.section.block.begin.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, '}', ['punctuation.section.block.end.nx'], grammar)).to.equal(true);
    expect(hasScopeAt(line, 'Spinner', ['entity.name.tag.nx'], grammar)).to.equal(true);
  });

  it('highlights property-list if blocks with braces and else', () => {
    const line = '<UserCard if isLoading { user=loading } else { user=loaded }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    expect(scopesForSubstring(line, '{', grammar)).to.include('punctuation.section.block.begin.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
  });

  it('highlights property-list match arms', () => {
    const line = '<UserCard if status is { "active" => icon=ActiveIcon "idle" => icon=IdleIcon else => icon=DefaultIcon }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    const isScopes = scopesForSubstring(line, 'is', grammar);
    expect(isScopes).to.include('keyword.control.match.nx');
    expect(isScopes).to.include('meta.control.if.properties.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
  });

  it('highlights property-list match fragments with union patterns and braced values', () => {
    const line = '<View if state is { LoadState.failed => message={state.message} else => message="" } />';
    expect(scopesForSubstring(line, 'if', grammar)).to.include('meta.control.if.properties.nx');
    expect(scopesForSubstring(line, 'is', grammar)).to.include('keyword.control.match.nx');
    expect(scopesForSubstring(line, 'LoadState', grammar)).to.include('entity.name.type.union.nx');
    expect(scopesForSubstring(line, 'failed', grammar)).to.include('entity.name.type.union.case.nx');
    expect(scopesForSubstring(line, 'message', grammar)).to.include('entity.other.attribute-name.nx');
    expect(scopesForSubstring(line, 'state.message', grammar)).to.include('meta.values-braced-expression.nx');
  });

  it('highlights property-list condition list arms', () => {
    const line = '<UserCard if layout { "compact" => gap=4 "full" => gap=8 else => gap=2 }>';
    const ifScopes = scopesForSubstring(line, 'if', grammar);
    expect(ifScopes).to.include('keyword.control.conditional.nx');
    expect(ifScopes).to.include('meta.control.if.properties.nx');
    const elseScopes = scopesForSubstring(line, 'else', grammar);
    expect(elseScopes).to.include('keyword.control.conditional.nx');
    expect(elseScopes).to.include('meta.control.if.properties.nx');
    expect(scopesForSubstring(line, 'gap', grammar)).to.include('entity.other.attribute-name.nx');
  });

  it('highlights fat arrows in elements match arms', () => {
    const line = 'if status is { "active" => <Span.Active/> else => <Span.Inactive/> }';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });

  it('highlights fat arrows in elements condition-list arms', () => {
    const line = 'if { count == 0 => <span:>Empty</span> }';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });

  it('highlights fat arrows inside attribute value expressions', () => {
    const line = '<UserCard icon=if { isAdmin => <Icon.Admin/> else => <Icon.User/> } />';
    expect(scopesForSubstring(line, '=>', grammar)).to.include('keyword.operator.arrow.nx');
  });

  it('scopes a for loop\'s binding variables and iterable', () => {
    // The header used to be left with no token scope at all: only `for` and `in` were coloured.
    const single = 'let rows = { for item in items { <Row /> } }';
    expect(scopesForSubstring(single, 'item', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(single, 'items', grammar)).to.include('variable.other.readwrite.nx');

    const indexed = 'let rows = { for item, index in list.rows { <Row /> } }';
    expect(scopesForSubstring(indexed, 'item', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(indexed, 'index', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(indexed, ',', grammar)).to.include('punctuation.separator.comma.nx');
    expect(scopesForSubstring(indexed, 'list.rows', grammar)).to.include('variable.other.readwrite.nx');

    // A non-identifier iterable is left to the other patterns.
    const literal = 'let rows = { for filter in ["all", "active"] { <Row /> } }';
    expect(scopesForSubstring(literal, 'filter', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(literal, 'all', grammar)).to.include('string.quoted.double.nx');
  });

  it('keeps a loop header\'s scopes across a line break and a comment', () => {
    // The header used to be one `\\G`-anchored match, so anything that pushed part of it onto a
    // second line — or merely interrupted it with a comment — dropped every scope in it.
    const split = tokenizeLines(grammar, [
      'let rows = {',
      '  for item,',
      '    index in items {',
      '    <Row />',
      '  }',
      '}'
    ]);
    expect(scopesAt(split, 'for item,', 'item')).to.include('variable.other.readwrite.nx');
    expect(scopesAt(split, 'for item,', ',')).to.include('punctuation.separator.comma.nx');
    expect(scopesAt(split, 'index in items', 'index')).to.include('variable.other.readwrite.nx');
    expect(scopesAt(split, 'index in items', 'in ')).to.include('keyword.control.loop.nx');
    expect(scopesAt(split, 'index in items', 'items')).to.include('variable.other.readwrite.nx');

    const commented = 'let rows = { for item /* each */ in items { <Row /> } }';
    expect(scopesForSubstring(commented, 'each', grammar)).to.include('comment.block.nx');
    expect(scopesForSubstring(commented, 'item ', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(commented, 'items', grammar)).to.include('variable.other.readwrite.nx');
  });

  it('scopes a compound iterable, not just a leading identifier', () => {
    const parens = 'let rows = { for item in (items) { <Row /> } }';
    expect(scopesForSubstring(parens, 'items', grammar)).to.include('variable.other.readwrite.nx');

    const binary = 'let rows = { for item in left + right { <Row /> } }';
    expect(scopesForSubstring(binary, 'left', grammar)).to.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(binary, '+', grammar)).to.include('keyword.operator.arithmetic.nx');
    expect(scopesForSubstring(binary, 'right', grammar)).to.include('variable.other.readwrite.nx');
  });

  it('scopes a control-form iterable and still finds the body brace', () => {
    // The iterable is a full `ValueExpression`, so it may itself be an `if`. The header used to
    // list only names and operators, so `if` was scoped as a binding variable, the iterable's
    // branch was left unscoped, and the loop ended at the branch's `}` instead of its own body.
    const result = tokenizeLines(grammar, [
      'let rows = {',
      '  for item in if ready { items } else { fallback } {',
      '    <Row />',
      '  }',
      '}'
    ]);
    expect(scopesAt(result, 'for item in if', 'if')).to.include('keyword.control.conditional.nx');
    expect(scopesAt(result, 'for item in if', 'ready')).to.not.be.empty;
    expect(scopesAt(result, 'for item in if', 'items'))
      .to.include('meta.control.if.value.nx');
    expect(scopesAt(result, 'for item in if', 'else'))
      .to.include('keyword.control.conditional.nx');
    expect(scopesAt(result, 'for item in if', 'fallback')).to.not.be.empty;
    // The loop's own body brace is the last one on the header line, and the body stays inside the
    // loop rather than escaping at the `if`'s closing brace.
    expect(scopesAt(result, 'for item in if', '{', -1))
      .to.include('punctuation.section.block.begin.nx');
    expect(scopesAt(result, '<Row />', 'Row')).to.include('meta.control.loop.value.nx');
  });

  it('scopes a sole binder split from its `in` by a line break', () => {
    // A `begin` sees one line, so `for item` at a line end cannot be told apart from prose that
    // wraps after `for item`. `#loop-header-split-binder` opens anyway and bounds the cost in its
    // `end` instead; the whole header used to go unscoped in this layout.
    const result = tokenizeLines(grammar, [
      'let rows = {',
      '  for item',
      '    in items {',
      '    <Row />',
      '  }',
      '}'
    ]);
    expect(scopesAt(result, 'for item', 'item')).to.include('variable.other.readwrite.nx');
    expect(scopesAt(result, 'in items', 'in ')).to.include('keyword.control.loop.nx');
    expect(scopesAt(result, 'in items', 'items')).to.include('variable.other.readwrite.nx');
    expect(scopesAt(result, '<Row />', 'Row')).to.include('meta.control.loop.value.nx');
  });

  it('carries a split header across blank and comment lines', () => {
    // NX treats blank lines and comments as trivia inside a header, so they must not end one. The
    // split entry's `end` lets them through and pops only at a line that actually stops continuing
    // the header. A block comment spanning lines needs no special case: its own context is on the
    // stack, so this `end` is not tested while it is open.
    const blank = tokenizeLines(grammar, [
      'let rows = {',
      '  for item',
      '',
      '    in items { <Row /> }',
      '}'
    ]);
    expect(scopesAt(blank, 'in items', 'items')).to.include('variable.other.readwrite.nx');

    const commented = tokenizeLines(grammar, [
      'let rows = {',
      '  for item',
      '    // pick each',
      '    in items { <Row /> }',
      '}'
    ]);
    expect(scopesAt(commented, '// pick each', 'pick')).to.include('comment.line.double-slash.nx');
    expect(scopesAt(commented, 'in items', 'items')).to.include('variable.other.readwrite.nx');

    const block = tokenizeLines(grammar, [
      'let rows = {',
      '  for item',
      '  /* why',
      '     not */',
      '  in items { <Row /> }',
      '}'
    ]);
    expect(scopesAt(block, 'not */', 'not')).to.include('comment.block.nx');
    expect(scopesAt(block, 'in items', 'items')).to.include('variable.other.readwrite.nx');
  });

  it('bounds a false loop header in wrapped prose to the line it opened on', () => {
    // The price of the rule above. When prose wraps after `for <word>`, that one word is scoped as
    // a binder — but the header pops at the start of the next line, because it neither begins with
    // `in` nor is trivia, so the rest of the paragraph is untouched. Without that end, every word
    // up to the next brace was scoped as a binding variable.
    const result = tokenizeLines(grammar, [
      'let copy = {',
      '  We are exploring an easier way for neighbors',
      '  to share tools and gear instead of buying them.',
      '  Responses are anonymous unless you add email.',
      '}'
    ]);
    expect(scopesAt(result, 'to share tools', 'share'))
      .to.not.include('variable.other.readwrite.nx');
    expect(scopesAt(result, 'to share tools', 'gear'))
      .to.not.include('variable.other.readwrite.nx');
    expect(scopesAt(result, 'Responses are', 'anonymous'))
      .to.not.include('variable.other.readwrite.nx');
  });

  it('stops a prose continuation that reads as a header at the end of its line', () => {
    // The residual ambiguity, pinned rather than claimed away. `… for neighbors` / `in towns and
    // cities` is a header in every visible respect, so that line is scoped as one. What must hold
    // is that it ends there: the line after it does not continue a header, so the words on it keep
    // their prose scopes instead of running on to the next brace.
    const result = tokenizeLines(grammar, [
      'let copy = {',
      '  We keep a shelf of tools for neighbors',
      '  in towns and cities nearby.',
      '  Everything else is stored away.',
      '}'
    ]);
    expect(scopesAt(result, 'in towns', 'towns')).to.include('variable.other.readwrite.nx');
    expect(scopesAt(result, 'Everything else', 'stored'))
      .to.not.include('variable.other.readwrite.nx');
  });

  it('does not open a loop header on a prose `for`', () => {
    // `for` occurs in prose too. The header only opens on text actually shaped like a header, so a
    // sentence does not get every one of its words scoped as a binding variable.
    const prose = 'let copy = { An easier way for neighbors to share tools }';
    expect(scopesForSubstring(prose, 'neighbors', grammar))
      .to.not.include('variable.other.readwrite.nx');
    expect(scopesForSubstring(prose, 'share', grammar))
      .to.not.include('variable.other.readwrite.nx');
  });

  it('keeps a keyword-spelled name in a condition-list arm out of the keyword scope', () => {
    // The arm list included all of `#attributes`, whose `#keywords-core` fallback claimed any name
    // that was not followed by `=`. Only `true`, `false`, and `null` are reserved.
    const arm = '<Notice if { state => tone="danger" } />';
    expect(scopesForSubstring(arm, 'state', grammar)).to.not.include('keyword.declaration.state.nx');
    expect(scopesForSubstring(arm, '=>', grammar)).to.include('keyword.operator.arrow.nx');
    expect(scopesForSubstring(arm, 'tone', grammar)).to.include('entity.other.attribute-name.nx');

    // The arm list still scopes an assignment and a reserved literal.
    const mixed = '<Notice if { true => type="x" } />';
    expect(scopesForSubstring(mixed, 'true', grammar)).to.include('constant.language.boolean.nx');
    expect(scopesForSubstring(mixed, 'type', grammar)).to.include('entity.other.attribute-name.nx');
  });

  it('ends an arm attribute at the arm\'s closing brace', () => {
    // An attribute ran to end of line, swallowing the `}` that closed its arm. Everything after it
    // then tokenized one context too deep: `examples/nx/component.nx:43-44` scoped the second
    // `density` as a qualifier and the element's `/>` as division and greater-than.
    const result = tokenizeLines(grammar, [
      'let notice = {',
      '  <Notice',
      '    if compact { density="tight" } else { density="normal" }',
      '  />',
      '}'
    ]);
    expect(scopesAt(result, 'if compact', 'density', 1))
      .to.include('entity.other.attribute-name.nx');
    expect(scopesAt(result, 'if compact', 'density', 2))
      .to.include('entity.other.attribute-name.nx');
    expect(scopesAt(result, '/>', '/')).to.include('punctuation.definition.tag.self-closing.nx');
    expect(scopesAt(result, '/>', '>')).to.include('punctuation.definition.tag.end.nx');
  });

  it('scopes a reserved literal in a property-list condition', () => {
    const line = '<Question if true { type = "x" } />';
    expect(scopesForSubstring(line, 'true', grammar)).to.include('constant.language.boolean.nx');
    // ...without claiming identifiers that merely share a keyword's spelling. `state` is a legal
    // parameter name (`examples/nx/component.nx:39`) and `type` a legal attribute name.
    const scrutinee = '<Notice if state is { LoadState.failed => tone="danger" } />';
    expect(scopesForSubstring(scrutinee, 'state', grammar))
      .to.not.include('keyword.declaration.state.nx');
    expect(scopesForSubstring(line, 'type', grammar)).to.include('entity.other.attribute-name.nx');
  });
});
