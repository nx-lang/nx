## Why

NX currently treats element body content inconsistently. Some documentation shows a dedicated
`content` slot concept, while the implementation uses an implicit special case around `children`
for some NX-defined declarations. That behavior is not part of the spec, does not apply uniformly
across function-like and record-like declaration forms, and leaks `children` terminology into the
implementation rather than using the source-language concept directly.

Adding an explicit contextual `content` marker makes the declaration contract clear, lets authors
choose the receiving property name, and makes body-content semantics consistent across functions,
components, records, actions, and component state. There is no need for backward compatibility, so
the old implicit `children` convention can be removed outright rather than preserved as a fallback.

## What Changes

- Add a contextual `content` modifier that can appear before at most one property definition in any
  function-like or record-like declaration surface: record types, action records, element-style
  `let` signatures, paren-style `let` parameters, component props, inline component action payload
  fields, and component state fields.
- Allow markup-style invocation body content to bind to the declared content property when
  constructing a record or invoking a function/component declaration.
- Continue allowing the content property to be passed as a normal named property.
- Report a diagnostic when the same content property is provided both as element body content and as
  a named property at the same call site.
- Treat `content` as a contextual keyword only in property-definition position so `content` remains
  a valid symbol name everywhere else.
- Replace the current `children`-based implementation terminology and binding special cases with
  explicit declared content-property semantics throughout the parser, HIR, type checker,
  interpreter, diagnostics, docs, and tests.

## Capabilities

### New Capabilities

- `content-properties`: Define how function-like and record-like declarations mark a content
  property and how markup body content binds to that property during invocation.

### Modified Capabilities

- None.

## Impact

- `crates/nx-syntax` grammar, generated parser artifacts, CST helpers, syntax highlighting, parser
  fixtures, and grammar/reference documentation for all content-capable property-definition sites.
- `crates/nx-hir` lowering and HIR models for property definitions and element invocations.
- `crates/nx-types` element-binding analysis and diagnostics that currently special-case body
  content handling.
- `crates/nx-interpreter` element invocation evaluation for record construction, function calls,
  component calls, and intrinsic/native element content handling.
- `crates/nx-api` exported/imported interface metadata for function, component, and record-like
  declarations.
- Documentation under `docs/src/content/docs/` that currently uses legacy slot examples or refers
  to implicit `children` behavior.
