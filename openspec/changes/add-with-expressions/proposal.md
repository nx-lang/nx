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
  - `with` chains, left to right: `base with brand with { radius=4 }`. It binds tighter than `??` and
    arithmetic and looser than member access.
- **`with` on update records and bundles.** `first with second` on two `T.Update` values is
  `merge(first, second)`. On an `env.Update` bundle it is a bundle with the second's overrides
  applied after the first's, so `brand with { radius.md=4 }` derives a theme from a theme.
- **Bundles for one section.** `provide env with brand { … }` applies a bundle to its body. It mixes
  with single overrides, in the order written: `provide env with marketing env.colorMode=dark { … }`.
  The bundle can be named, written inline, or passed in a prop or local: an `env.Update` value keeps
  its entries as formulas, so it means the same wherever it is applied. It patches what is already
  there: values it doesn't set keep what the place around it has.
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
  `with-path` (a path step that is not a required record field). `environment-cycle` also covers an
  override of `V` that reads a value whose default reads `V`, and a runtime
  `nx-ir-environment-cycle` covers the case only placement reveals. A wrong-typed field gets the
  usual mismatch.
- **NX IR:** record `with` lowers to the existing `apply` intrinsic, so it needs nothing new. Bundle
  `with` adds a base operand to `envUpdate`; `envProvide` gains bundle overrides and formulas, and
  `$env` entries gain `formula` and `bundle`, behind a new required feature, `environment-v2`.
- **Runtimes and generated code:** both IR runtimes accept `environment-v2` and evaluate formulas
  with the program at the read, and the instance tree tracks reads made through them. Generated code
  carries a formula as a closure, which also fixes the generated-code theme divergence from #75.
- **Editor:** completion of field names and paths inside `with { … }`, hover on `with` showing the
  patched type, the bundle hover on `provide env with`, and `with` in both syntax grammars.

## Capabilities

### New Capabilities

- `with-expressions`: the `with` expression on records, update records and `env.Update` bundles,
  its field lists and paths, typing, precedence and diagnostics.

### Modified Capabilities

- `environment-values`: `provide env with bundle` in `provide` blocks and at the top of the main
  module (replacing `provide bundle`), bundles that keep formulas, the cycle check for overrides, and
  one rule for every source: computed where it is read.
- `nx-ir-format`: the formula flag, bundle overrides, `envUpdate`'s `base`, formulas and bundles in
  `$env`, `environment-v2`, the lookup and the cost.
- `executable-code-generation`: formulas as closures in generated code.
- `rust-ir-runtime`, `typescript-ir-runtime`: `environment-v2`, and formulas evaluated with the
  program at the read.
- `component-instance-tree`: reads made through formulas and bundle entries are recorded.
- `editor-language-service`: the bundle hover on `provide env with`, and `with` completion and hover.
- `editor-syntax-highlighting`: `with` as a keyword after an operand.

## Impact

- `crates/nx-syntax` (grammar, highlight queries), `crates/nx-hir` (lowering, `reserved-name`),
  `crates/nx-types` (typing and diagnostics), `crates/nx-ir` (`envUpdate` base, bundle references,
  `environment-v2`), `crates/nx-ir-runtime`, `runtime/typescript`, `crates/nx-codegen`,
  `crates/nx-language-service`.
- Conformance corpus: new programs for record `with`, bundles in blocks and formulas; `environment`
  programs move to `provide env with`.
- Docs: `reference/syntax/expressions.md` (new `with` section), `reference/syntax/environment.md`,
  `reference/syntax/functions.md` (point from `apply` to `with`), the design-tokens tutorial and
  reference if #75 has merged, and `specs/future.md`.
- #75 has merged: its uses of `provide brand.theme`, its "a bundle can't be applied to one section"
  notes and its generated-code divergence are updated here.
