## Why

Math signatures currently exist only as synthesized compiler declarations, so their NX source has no function documentation or real declaration locations. Authoring the public API in NX makes it readable and lets existing hover/completion documentation work normally.

## What Changes

- Add `export external name(parameters):Result` as an explicitly typed, bodyless runtime-provided function declaration.
- Resolve implementations only through compiler-owned module registry entries; ordinary user/library declarations without a registered implementation are diagnosed.
- Declare all 22 math functions and document both constants in `math.nx`, using ordinary `///` comments and declaration spans.
- Validate the entire source signature against the registered operation; remove synthesized public signatures while keeping existing runtime math bodies and evaluation contracts.
- Update syntax/reference documentation, editor behavior and end-to-end regression tests.

## Capabilities

### New Capabilities

- `external-functions`: Syntax, signature checking and trusted implementation binding for bodyless function declarations.

### Modified Capabilities

- `runtime-standard-functions`: Public standard functions are source-authored documented declarations, bound by trusted identity.
- `standard-libraries`: Standard math declarations use documented external source signatures while imports remain ordinary typed declarations.

## Impact

Tree-sitter grammar/generated parser, HIR lowering, standard-library loading, math source, editor documentation tests and NX reference docs. No new IR node, callback API or runtime backend is introduced. This follows the archived `add-math-standard-library` change.
