## Why

External components can inherit from an abstract external base, but named-type compatibility and
list inference still behaved like plain records only. That blocked assigning a concrete derived
external component value where the abstract base was expected and made heterogeneous braced lists
of sibling externals fall through instead of typing against their shared contract.

## What Changes

- Type checking treats a concrete external component type as satisfying an expected named type when
  the concrete type extends that contract through the external component inheritance chain (same
  idea as record subtyping, but driven by component contracts and ancestors).
- Type inference for the least upper bound of two named external component types considers the
  abstract external base chain so mixed sibling values can satisfy an explicitly annotated base list
  type (for example `Question[]` holding multiple derived question components).
- The interpreter applies the same contract-aware compatibility when checking that a runtime
  external component record value matches an expected named type, keeping execution consistent with
  the static checker.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `external-components`: Document static and runtime compatibility of derived external component
  values with abstract external base named types.
- `braced-value-sequences`: Clarify that the most specific common type for multi-item braced value
  lists uses external component inheritance (shared abstract external base), not only record
  inheritance, when both operands are external component contract types.

## Impact

- `crates/nx-types` type checker and inference (`infer.rs`).
- `crates/nx-interpreter` runtime value typing checks (`interpreter.rs`).
- Integration-style tests in `crates/nx-types/tests` and interpreter unit tests.
