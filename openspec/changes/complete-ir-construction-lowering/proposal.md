## Why

Interpreter retirement exposed two source constructs missing from executable IR lowering:
conditional property fragments and positional record constructors. Preserve their existing source
expectations during retirement, then complete compiler support in this independent follow-up.

## What Changes

- Compile simple conditional, condition-list and match-style property fragments with active-path
  property selection, lexical narrowing, defaults and duplicate validation preserved.
- Compile positional record construction using declared field order and defaults.
- Enable the ignored source-to-IR regression tests retained by `retire-hir-interpreter`.
- Verify Rust and TypeScript IR execution and generated JavaScript agree with handwritten results.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `executable-code-generation`: Require executable support for conditional properties and positional
  record construction, with parity across IR runtimes and generated code.

## Impact

Compiler lowering and conformance tests; no public source-evaluation API is restored.
This change follows interpreter retirement and does not block it. Until this is implemented,
conditional fragments receive an unsupported-codegen diagnostic; positional record calls fail at
runtime. The retirement change documents these accepted temporary limitations.
