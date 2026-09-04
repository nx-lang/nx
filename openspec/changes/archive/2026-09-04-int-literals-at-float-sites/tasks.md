## 1. Type checking: accept the literal

- [x] 1.1 Add a float-target test to `crates/nx-types`: given an expected `Type`, return the target
      float primitive when the type strips (through nullability, and through a list element type at
      a scalar-to-list coercion site) to `float32` or `float64`, and `None` for every other expected
      type — `object`, an unresolved type variable, a union, an integer primitive, absent. Verify
      with unit tests covering each of those cases, per design D6.
- [x] 1.2 Add the exactness predicate: an `i64` is acceptable at a float type when it round-trips
      through that type unchanged. Verify with unit tests at the boundaries — 2^53 and 2^53+1 for
      `float64`, 2^24 and 2^24+1 for `float32`, and the negative counterparts — per design D3.
- [x] 1.3 In `check_typed_binding_for` (`crates/nx-types/src/infer.rs`), when the passed `ExprId`
      resolves to `Expr::Literal(Literal::Int(_))` and 1.1 yields a float target, accept the binding
      instead of reporting a mismatch, or report the inexactness diagnostic when 1.2 fails. Verify
      with type-checking tests that `external component <B v:float64 />` accepts `<B v=1 />`,
      accepts `<B v=-1 />`, and rejects `<B v=9007199254740993 />` with a message naming the literal
      and `float64`.
- [x] 1.4 Confirm every binding site named in the spec passes its `ExprId` to
      `check_typed_binding_for` rather than calling `check_typed_binding` with `None`, and thread
      the expression through the sites that do not. Verify with tests covering a property binding,
      a property default (`external component <C x: float64 = 0 />`), a record field default
      (`type Opts = { x: float64 = 1 }`), an annotated let (`let x: float64 = 5`), a float-typed
      argument, and a `float64[]` element.
- [x] 1.5 Verify the boundaries hold: an `int`-typed parameter at a `float64` site is still rejected,
      an integer arithmetic expression at a `float64` site is still rejected, `1.5` and `1.0` at an
      `int` site are still rejected, an integer literal at an `object` site is still `int`, and
      `let n = 42` still infers `int`.

## 2. Type checking: record the conversion

- [x] 2.1 Add a `converted_int_literals: FxHashMap<ExprId, FloatPrimitive>` map to
      `InferenceContext` alongside `resolved_contextual_names`, recorded at the acceptance point in
      1.3, with an accessor mirroring `resolved_contextual_names()`. Verify with a test that
      inspects the map after checking a module containing one converted and one unconverted integer
      literal.
- [x] 2.2 Add the HIR rewrite in `nx-hir` — a sibling of `apply_contextual_name_resolutions` that
      replaces each recorded `Literal::Int(n)` with `Literal::Float(n as f64)` in the prepared
      module. Verify with a unit test that the rewritten expression is `Literal::Float` and its span
      is unchanged.
- [x] 2.3 Drain the map in `crates/nx-types/src/check.rs` next to the contextual-name pass, before
      the module is snapshotted, so no consumer below type checking sees the integer literal. Verify
      by asserting the snapshotted HIR for `<B v=24 />` at a `float64` property contains no
      `Literal::Int`.

## 3. End-to-end equivalence

- [x] 3.1 Add an equivalence test comparing two programs — one written `24`, one written `24.0` at
      the same `float64` property — and assert their emitted NX IR is identical. Verify the test
      fails if the rewrite in 2.2 is disabled.
- [x] 3.2 Extend that equivalence to evaluation and code generation: assert the interpreted value is
      a float value (not `Value::Int`), and that generated C# and TypeScript for a `float64` default
      written `0` match those for one written `0.0`.
- [x] 3.3 Add a `float32` case to 3.1–3.2 confirming a literal at a `float32` site behaves exactly as
      an explicit real literal does there, per design D4. Pin the distinction the artifacts turn
      on: the *binding* takes the declared width at every site (`let x: float32 = 42` gives `x`
      type `float32`), while the type recorded for the literal expression is whatever an explicit
      real literal takes — so both spellings agree at both widths.
- [x] 3.4 Run `cargo test --workspace` and confirm no existing test regressed — in particular the
      numeric-compatibility, formatter, and IR-emission suites.

## 4. Formatter and language service

- [x] 4.1 Confirm the value formatter still renders a float `24.0` as `24.0` and add the regression
      test the `unbraced-literal-forms` delta names, so the shortening is never introduced by
      accident (design D5).
- [x] 4.2 Check the language service reports the new acceptance for an integer literal at a float
      site rather than a spurious mismatch, and fix it if it does not. Verify with diagnostics
      tests over both an annotated `let` and the property binding `<B v=1 />` at `v:float64`, and
      over an inexact literal. Surfacing the contextually chosen type in quick-info is out of
      scope: `hover` matches a position against top-level document symbols only and the service
      exposes no expression-level type API, so it is the deferred presentation decision in
      design's Open Questions, not a fix. The proposal's Impact and design's Risks say so too, so
      the deferral is stated everywhere it could be read the other way.

## 5. Corpus: the DrawnUI fiddle

- [x] 5.1 Build the tooling and capture a baseline: `cargo build --workspace`, then from
      `sample-apps/drawnui-react` run `npm run check-examples` and save the emitted NX IR for all 12
      examples to a baseline directory. Verify all 12 compile and evaluate before any edit.
- [x] 5.2 Write a one-off script that rewrites `=N.0` to `=N` in property position in
      `src/examples/nx/*.nx`, leaving fractional literals, strings, colors, and comments untouched.
      Verify by diffing: the count of changed literals matches the count of `=N.0` occurrences and
      no `.5`-style literal moved.
- [x] 5.3 Re-emit the IR and verify it is byte-identical to the 5.1 baseline for every example
      (design D7). Any difference is a bug in 5.2 or in the compiler change, not an acceptable
      variation.
- [x] 5.4 Run `npm test` (compile-service tests plus `check-examples`) and `npm run typecheck`, and
      open the fiddle to confirm the gallery renders as before.
- [x] 5.5 Update `sample-apps/drawnui-react/docs/` where a `.0` appears in an NX snippet, and add a
      line to `docs/FINDINGS.md` recording that the `.0` requirement was the gap and this change
      closed it.

## 6. Corpus: repository examples and documentation

- [x] 6.1 Update the NX snippets under `examples/` and the `.nx` files under `docs/` (the DrawnUI
      proposal trees `docs/drawnui-proposal/`, `docs/displaylist-proposal/`) to drop redundant
      `.0`, excluding `docs/node_modules/`. Verify each edited `.nx` file still compiles: run
      `nxlang typegen` over each proposal library (it type checks the whole library) and require
      no new diagnostic and byte-identical generated output, and for the one tree that emits NX IR
      require the IR to be identical apart from source provenance, per design D7.
- [x] 6.2 Update NX code fences in `docs/src/content/docs/` where they show a whole number at a float
      property, and state the rule where the documentation introduces numeric types. Verify the docs
      site builds. Also update the prose in the DrawnUI proposal documents that states the old
      restriction — `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` and NXE8 in
      `docs/drawn-ui-proposal-nx-enhancements.md` — and the Appendix A fences they describe.
- [x] 6.3 Decide the catalog question from design's Open Questions — whether
      `scripts/generate-catalog.mjs` should emit `0` or `0.0` for float defaults — and either change
      the generator and regenerate `catalog/skia.nx`, or record the decision to leave it. Verify
      `npm run generate-catalog` produces no unintended diff.

## 7. Close out

- [x] 7.1 Run the full workspace test suite and the fiddle's `npm test` together and confirm both are
      green with the corpus edits in place.
- [x] 7.2 Run `openspec validate int-literals-at-float-sites --strict` and confirm the change is
      ready to archive.
