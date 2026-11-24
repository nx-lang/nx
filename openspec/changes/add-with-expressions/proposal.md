## Why

A theme can only be applied to a whole program: `provide brand.theme` is allowed at the top of the
main module and nowhere else, and a section can change single values only. A marketing banner in
another brand, or a settings page with denser spacing, has no way to take a bundle of overrides.
`add-design-tokens` (#75) records this as later work.

The record `with` expression was proposed in the design-system tokens design
(https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG) as sugar for `apply`, and deferred. Today a
patched copy of a record names its update type, `apply(light, <SemanticColors.Update accent=… />)`,
and a nested field needs one `apply` per level. Both needs are the same idea, "this value, with
these changes on top", so this change adds one word for both: `with`.

## What Changes

- **`with` on records.** `value with { field=… }` is the record with those fields replaced, as
  `apply` gives it, without naming the update type:
  - `light with { accent=#16a34a }`;
  - a path field patches a nested record and keeps the rest: `colors with { text.muted=… }`;
  - an update value works on the right too: `saved with rename`, where `rename` is a `T.Update`;
  - `with` chains, left to right: `base with brand with { radius=4 }`.
- **`with` on update records and bundles.** `first with second` on two `T.Update` values is
  `merge(first, second)`. On an `env.Update` bundle it is a bundle with the second's overrides
  applied after the first's, so `brand with { radius.md=4 }` derives a theme from a theme.
- **Bundles for one section.** `provide env with brand { … }` applies a bundle to its body. It mixes
  with single overrides, in the order written: `provide env with marketing env.colorMode=dark { … }`.
  The bundle can be named or written inline. It patches what is already there: values it doesn't set
  keep what the place around it has.
- **One evaluation rule.** Every environment value is computed where it is read: defaults,
  main-module provides, `env.` attributes, `provide` overrides and bundles alike. Inside one, a read
  of the same value sees the sources further out. Until now local overrides were computed where
  written; they now follow a value overridden nearer the reader, so a theme that varies by mode
  follows an `env.colorMode` set inside the section. Descriptors carry an override that reads the
  environment as a formula (its code and captured locals); plain values stay as they are.
- **App-wide form.** The main module's `provide brand.theme` becomes `provide env with brand.theme`,
  so a bundle is applied with one syntax everywhere. The old form is removed (no compatibility is
  needed yet).
- **Reserved word.** `with` can't name a declaration, parameter or binding (`reserved-name`); a
  record field named `with`, read as `x.with`, keeps working.
- **Diagnostics:** `with-target` (the left side is not a record, update record or bundle) and
  `with-path` (a path step that is not a required record field). A wrong-typed field gets the usual
  mismatch.
- **NX IR:** record `with` lowers to the existing `apply` intrinsic, so it needs nothing new. Bundle
  `with` adds a base operand to `envUpdate`; `envProvide` gains bundle overrides and formulas, and
  `$env` entries gain `formula` and `bundle`, behind a new required feature, `environment-v2`.
- **Runtimes and generated code:** the Rust and TypeScript IR runtimes and generated JavaScript and
  TypeScript evaluate the new forms. Generated code keeps its "values evaluated at load" limit (see
  design).
- **Editor:** completion of field names and paths inside `with { … }`, and hover on `with` showing
  the record type.

## Capabilities

### New Capabilities

- `with-expressions`: the `with` expression on records, update records and `env.Update` bundles,
  its field lists and paths, typing, precedence and diagnostics.

### Modified Capabilities

- `environment-values`: `provide env with bundle` in `provide` blocks and at the top of the main
  module (replacing `provide bundle`), and one rule for every source: computed where it is read.
- `nx-ir-format`: formulas and bundles in `envProvide` and `$env`, their lookup and their cost.

## Impact

- `crates/nx-syntax` (grammar, highlight queries), `crates/nx-hir` (lowering, `reserved-name`),
  `crates/nx-types` (typing and diagnostics), `crates/nx-ir` (`envUpdate` base, bundle references,
  `environment-v2`), `crates/nx-ir-runtime`, `runtime/typescript`, `crates/nx-codegen`,
  `crates/nx-language-service`.
- Conformance corpus: new programs for record `with`, bundles in blocks and formulas; `environment` programs
  move to `provide env with`.
- Docs: `reference/syntax/expressions.md` (new `with` section), `reference/syntax/environment.md`,
  `reference/syntax/functions.md` (point from `apply` to `with`), the design-tokens tutorial and
  reference if #75 has merged, and `specs/future.md`.
- If #75 merges first, its uses of `provide brand.theme` and its "a bundle can't be applied to one
  section" notes are updated here.
