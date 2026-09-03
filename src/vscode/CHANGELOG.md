# Changelog

All notable changes to this project will be documented in this file.

## Unreleased
- Add Rust `nx-lsp` language server integration for diagnostics, symbols, hover, and completions
- Add extension activation code, packaged server path resolution, and `nx.server.path`
- Add package verification for compiled client runtime and native server assets

### Grammar: element-shaped declarations
Component and `let` function declarations written in element form now tokenize as declarations
rather than as element references. Themes and Monaco/Shiki consumers of `@nx-lang/language/grammar`
will see the following scopes change.

New scopes:
- `storage.modifier.external.nx` — the `external` modifier, previously unscoped
- `storage.modifier.content.nx` — the `content` property modifier, previously unscoped
- `keyword.declaration.emits.nx` and `keyword.declaration.state.nx` — previously unscoped, or
  scoped `entity.name.qualifier.nx`
- `meta.declaration.signature.nx`, `meta.declaration.emits.nx`, `meta.declaration.state.nx` —
  container scopes for a declaration signature and its groups
- `meta.definition.function.nx` — an element-shaped `let` function definition

Corrected scopes inside a declaration signature:
- The declared name is `entity.name.type.nx`, not `entity.name.tag.nx`
- A property name is `variable.other.property.nx` and its colon
  `punctuation.separator.type.annotation.nx`, rather than both being folded into
  `support.type.text.nx`
- A property type is scoped over its complete name; a greedy-quantifier backtrack previously split
  the last character off (`TextFormat` as `TextForma` + `t`) and left the default value unscoped
- The `?` and `[]` type suffixes are `keyword.operator.type-modifier.nx`
- The signature's closing `/>` is tag punctuation, not `keyword.operator.arithmetic.nx` +
  `keyword.operator.comparison.nx`
- A `//` comment inside a signature is a comment for the whole line, and no longer closes the
  enclosing tag

Renamed scope (breaking for themes keying on the old name, no alias kept):
- Primitive types (`string`, `int`, `float64`, …) are `support.type.primitive.nx`, previously
  `storage.type.primitive.nx`. `storage.type` is for a type that is itself the declarator, as in
  C's `int x = 5`; NX primitives appear only in annotation position, so they belong in the same
  family as user-defined types, which is what TypeScript does. Most themes colour
  `support.type.primitive.nx` and `entity.name.type.nx` alike, so primitives now read as types
  rather than as keywords

Right-hand sides (`= RhsExpression`):
- Every site that admits an `RhsExpression` — a value definition, a function definition, a record
  property default, a signature property default, and an attribute value — now scopes it by the same
  rule. A bare name is a `ContextualName` and is scoped `variable.other.enummember.nx` in all five
- A record property's default previously fell through to `entity.name.qualifier.nx`, the
  module-qualifier catch-all, and a value definition's right-hand side was left unscoped entirely
- A dotted name in a value position is not scoped as a qualified union case, matching the language
  rule that a `ContextualName` is a single identifier

Element references:
- An attribute named after a keyword is an attribute. In `<Question type = "multiple" />`, `type` is
  `entity.other.attribute-name.nx` rather than `keyword.declaration.type.nx`, and the `=` and
  `"multiple"` are scoped rather than left unscoped. `#keywords-core` outranked the attribute rule
  in `attributes`, and once it consumed the name the attribute rule could no longer match

Control forms:
- A `for` loop's binding variables and its iterable are scoped `variable.other.readwrite.nx`, and
  the separating `,` is `punctuation.separator.comma.nx`. The whole header between `for` and the
  body previously had no token scope at all
- A reserved literal used as a condition is a literal: `if true { … }` in a property list scopes
  `true` as `constant.language.boolean.nx` rather than `entity.name.qualifier.nx`
- A loop header keeps those scopes when it spans a line break (`for item,` / `index in items`) or
  is interrupted by a comment, and every name in a compound iterable is scoped, not just the first:
  `for item in left + right` now scopes `right`. The header is a shared `#loop-header` context
  rather than one single-line match
- A control form used as the iterable is scoped as one: in
  `for item in if ready { items } else { fallback } { … }`, `if` and `else` are
  `keyword.control.conditional.nx` rather than names, the branches are scoped, and the loop body no
  longer escapes the loop at the conditional's `}`
- A loop header whose sole binder is separated from its `in` by a line break is scoped: `for item`
  then `in items` scopes the binder, the `in`, and the iterable, where the whole header was
  previously unscoped, and blank lines or comments between the two do not change that. Because a
  header opens only on text shaped like one — `for` also occurs in prose — this layout has its own
  rule, which closes at the first line that does not continue the header. Prose that wraps after
  `for <word>` therefore mis-scopes that one word, and prose whose next line begins with `in` is
  indistinguishable from a header, so that line's names are scoped as well. In both cases the
  misreading ends at the first line that does not continue the header, rather than running to the
  next brace
- A name in a property-list condition arm that merely shares a keyword's spelling keeps its
  positional scope: `state` in `if { state => tone="danger" }` is no longer
  `keyword.declaration.state.nx`
- An attribute inside a condition arm ends at the arm's `}` instead of running to end of line.
  Everything after such an arm previously tokenized one context too deep — in
  `if compact { density="tight" } else { density="normal" }` the second `density` was
  `entity.name.qualifier.nx`, and the element's `/>` was scoped as
  `keyword.operator.arithmetic.nx` and `keyword.operator.comparison.nx`
- New `#literals` rule (`true`, `false`, `null`), split out of `#keywords-core`, which now includes
  it. New `#attribute-spread`, `#attribute-assignment`, and `#attributes-condition` rules, split out
  of `#attributes`. No scope names changed

Comments and type suffixes outside a signature:
- A `//` comment trailing a record property or a value definition is a comment. It was previously
  tokenized as code — the slashes as `keyword.operator.arithmetic.nx`, and words inside it as
  `constant.language.boolean.nx`, `keyword.control.*`, or `entity.name.qualifier.nx`
- `?` in a type position is `keyword.operator.type-modifier.nx` in a record property as well as a
  signature, matching `[]`. It was `keyword.operator.conditional.nx`, which most themes leave
  unstyled. The ternary `?` keeps that scope

Declaration recovery, qualified names, and remaining annotation positions:
- A declaration whose signature or record body is never terminated no longer swallows the rest of
  the file. Every context that can hold a declaration open — the signature, a property definition,
  a record or action body, a union case body, an `emits` or `state` group, a braced or embed
  expression, an `if`/`for` form, a start tag, and a definition's right-hand side — now recovers at
  the next top-level declaration, so the declaration after a typo is scoped as itself. Recovery
  ignores an attribute named after a keyword (`type = "multiple"`) and an indented nested binding
  (`let x = { … }` inside a braced expression), neither of which starts a declaration. Indentation
  is otherwise irrelevant: `type`, `action`, `component`, and a modifier-carrying `let` all recover
  wherever they are written, since none of them can appear inside an expression
- A qualified declaration name (`component <Ns.Widget … />`) is recognized as a declaration.
  Previously the whole signature was left unscoped because only an unqualified name was matched
- A signature terminated `/ >` now closes its component. The signature accepted the whitespace but
  the component did not, leaving the body unscoped until the next declaration
- The `content` modifier is `storage.modifier.content.nx` inside an `emits` or `state` group as
  well as in a signature; it was `entity.name.qualifier.nx` there
- A value definition's type, a parenthesized parameter's type, and a function's return type are all
  scoped by the same rule as a signature or record property. In those three positions `?` was
  `keyword.operator.conditional.nx`
- A parenthesized function parameter is scoped by the property-definition rule, the same production
  it is in the language: its name is `variable.other.property.nx` and its default is scoped by the
  `RhsExpression` rule. Both previously fell through to `entity.name.qualifier.nx`, the
  module-qualifier catch-all
- Type suffixes are scoped in source order, so `string?[]` and `Color[]?[]` scope every suffix.
  Only `[]`-then-`?` was recognized before, which left a trailing `[]` unscoped

A declaration is now scoped independently of what precedes it in the file: `external` was missing
from the lookaheads that terminate a union or record body, so a multi-line union nested every
following declaration inside a stale union-case scope.

## 0.1.0
- Initial release
- TextMate grammar and language configuration
- Snippets and sample file
