# Review: update-fiddle-to-nx-occurrences

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/drawnui-nx-catalog, specs/editor-language-service,
specs/fiddle-nx-language, specs/optional-properties, specs/playground  
**Reviewed code:**
- NX (`refine-sequences-and-optionals`, `main..HEAD`): `fa04fa9` (the language-service and type-checker fixes),
  `c6d313d` (the playground on the published `drawnui-react`), `f160bc7` (`specs/future.md`).
- Fiddle (`~/src/DrawnUi.FiddleEngine`, `nx-occurrences`, `origin/main..HEAD`): `08fe972`, `c13888e`, `98d5411`,
  `c34425c`, `e20bc2a`. These cover `nx/generate-catalog.mjs`, `nx/catalog/*`, `nx/src/values.ts`, `nx/test/*`,
  the fixtures, `FiddlePresetsNx.cs`, `README-NX.md` and `nx/CATALOG.md`.

Tests run during the review, all green:
- `cargo test -p nx-types --test optional_properties`;
- the new language-service completion-detail test;
- the playground's `pnpm test` (20 examples checked);
- the fiddle's `npm run test:nx -- --nx ../nx` (99 pass).

Also checked:
- The playground's generator differs from the fiddle's only in paths, header and `contentProperty`.
- `skia.nx` and `drawnui.nx` are identical below their headers.
- The playground's component list is unchanged by the move.
- `materialize.ts`'s `applyProps` matches upstream's `applyProps` at `v0.1.0-preview.12` for a new control.

Section 8 and 9 tasks are open, as the change intends (each waits on the user's go-ahead).

## Findings

### ✅ Verified - RF1 A parenthesized function parameter written `b:int?` is still reported a second time at every call
- **Severity:** Medium
- **Evidence:** The requirement "A property admits zero only through a mark on its name" names parenthesized
  function parameters among the declaration sites. `fa04fa9` reads a rejected `name: T?` as marked for element
  properties (`crates/nx-types/src/infer.rs:4780`) and record fields (`crates/nx-types/src/infer.rs:2952`). It does
  not do so for a function's parameters: the omissible count at `crates/nx-types/src/infer.rs:776` looks only at
  each parameter's `omissible` flag. Reproduced with the CLI at HEAD:
  ```
  let f(a:int, b:int?) = { a }
  let root() = { f(1) + f(2) }
  ```
  gives the declaration's fix-it plus `Function expects 2 arguments, got 1` at both calls. That is the same
  one-mistake-many-errors pattern the fix was meant to remove. `infer_call` then returns `Type::Error`, so the
  arguments are not type-checked either.
- **Recommendation:** When computing `required` at `infer.rs:776`, treat a parameter whose declared type admits zero
  as omissible, as `infer.rs:4780` does for element properties. Add the `let f(a:int, b:int?)` case to
  `a_property_rejected_in_the_type_slot_is_not_reported_again_where_it_is_left_out` in
  `crates/nx-types/tests/optional_properties.rs`. Optionally, name it in the delta scenario too.
- **Fix:** `declared_function_callee` (`crates/nx-types/src/infer.rs`) now reads a parameter whose declared type
  admits zero as omissible, resolving it quietly in the declaring module as the element and record paths do, for
  local and imported functions alike. The `let f(a:int, b:int?)` case, with `f(1) + f(2)`, is in
  `a_property_rejected_in_the_type_slot_is_not_reported_again_where_it_is_left_out`, and failed without the fix
  (the declaration plus two "Function expects 2 arguments"). The delta scenario names it too.
- **Verification:** Confirmed with the CLI rebuilt from the working tree: `let f(a:int, b:int?)` with
  `f(1) + f(2)` now reports only the declaration. It gave three errors at HEAD. A type alias that carries zero
  (`s:Maybe`) is also read as omissible. A non-trailing `a:int?` before a required `b` still reports the call to
  `f(1)`, which is correct, because a positional call cannot skip it. The test case is in `optional_properties.rs`,
  the delta scenario names `let f(a:int, b:int?)`, and `cargo test -p nx-types` and `-p nx-language-service` pass.
  See RF7 for the one half of that scenario that functions do not meet.

### ✅ Verified - RF2 Nothing checks that the copied assets come from the tag of the pinned version
- **Severity:** Low
- **Evidence:** The playground delta requires the recorded upstream tag to "be the release of the `drawnui-react`
  version the site pins". `sites/playground/scripts/catalog-version.test.mjs` compares only `catalog-meta.json`
  with the pin.
  - The README's pin move (`sites/playground/README.md:112`) is three commands: `pnpm add`, `generate-catalog`,
    `sync-drawnui`.
  - A maintainer who skips the third passes `pnpm test` with `docs/UPSTREAM.md` still naming the old tag, and with
    fonts, images and shaders from the old release.
- **Recommendation:** Extend `catalog-version.test.mjs` to read the `Package` or `Tag` row of `docs/UPSTREAM.md` and
  compare it with the pin. Its failure message should name `pnpm run sync-drawnui`.
- **Fix:** `sites/playground/scripts/catalog-version.test.mjs` gained a second test that reads the `Package` row of
  `docs/UPSTREAM.md` and compares it with the pin, failing with both versions and `pnpm run sync-drawnui`. Checked:
  it passes as committed and fails with the row set to preview.11. The README's pin section says `pnpm test` checks
  both records.
- **Verification:** The new test reads the `Package` row (`drawnui-react <version>`), which
  `sync-drawnui.mjs` writes in exactly that form. I changed the row to preview.11 and the test failed, naming both
  versions and `pnpm run sync-drawnui`. With the row restored it passes, and the README describes both checks.

### ✅ Verified - RF3 The fiddle's and the playground's renderers now treat the empty value differently
- **Severity:** Low
- **Evidence:** The fiddle's `nx/src/values.ts:95` (`isEmpty`) drops `null`, `undefined` and `[]` from props, record
  fields and content. The playground's `sites/playground/src/render/values.ts:130` and `:160` still drop only `null`
  and `undefined`. The runtime documents `[]` as the canonical empty value (`runtime/typescript/src/index.ts:701`).
  A probe against the playground's pipeline showed the runtime currently leaves empty props out entirely, for an
  untaken `if`, an empty `string*` and an empty `for`. So nothing visible breaks today. But the two renderers are
  meant to read the same wire form, and only one of them is hardened and tested for it.
- **Recommendation:** Port `isEmpty` and the fiddle's "the empty value, an empty sequence, is dropped" case into the
  playground's `values.ts` and its tests. Alternatively, record in `docs/CATALOG.md` why the playground does not need
  it.
- **Fix:** Ported `isEmpty` into `sites/playground/src/render/values.ts` and used it for record fields,
  `coerceProps` and `childrenOf`. New `src/render/values.test.mjs` mirrors the fiddle's case and adds content; it
  fails on the old `values.ts`. Playground `pnpm test` (97 tests, 20 examples) and typecheck pass.
- **Verification:** `values.ts` now uses the same `isEmpty` as the fiddle, in record fields, `coerceProps` and
  `childrenOf`. The new `values.test.mjs` covers props, a record and content. Playground `pnpm test` passes (97 tests,
  20 examples), and so does `pnpm run typecheck`.

### ✅ Verified - RF4 README-NX's migration hint turns an old optional list into a required one
- **Severity:** Low
- **Evidence:** `README-NX.md:125` says "`T[]` is written `T+`". In 0.3.0, `T[]` admitted an empty list. The
  compiler's own fix-it for a property is `name?: T+`, as the `specs/future.md` entry quotes (`colors?:string+`).
  Outside a property slot it is `T*`. An author who follows the README literally makes a property required that
  used to accept `[]`.
- **Recommendation:** Reword it, for example: "`T[]` becomes `T*`, and on a property `name?: T+` (or `name: T+`
  where it is never empty)."
- **Fix:** `README-NX.md` in the fiddle now says a property `name: T[]` becomes `name?: T+`, or `name: T+` where it
  is never empty, and `T[]` elsewhere becomes `T*`. Uncommitted on `nx-occurrences`; the fiddle's
  `npm run test:nx -- --nx ../nx` still passes (99).
- **Verification:** `README-NX.md` now gives `name?: T+`, or `name: T+` where it is never empty, and `T*` outside
  a property. That matches the compiler's fix-its. The fiddle's `npm run test:nx -- --nx ../nx` passes (99). The
  change is uncommitted on `nx-occurrences`.

### ✅ Verified - RF5 The completion detail restates hover's `name?: T` format rather than sharing it
- **Severity:** Low
- **Evidence:** `crates/nx-language-service/src/lib.rs:2311` builds `format!("{}{}: {}", name, optional_mark(..),
  ty)`, the same string `hover::parameter` and `hover::property` build (`crates/nx-language-service/src/hover.rs:295`
  and `:307`). The comment it replaced justified the old detail as "one fact, spelled once, so the two cannot drift
  apart". That guarantee is gone: the new comment says the detail matches hover, but nothing keeps them in step.
  `lib.rs:534` and `lib.rs:2207` also spell the format out by hand.
- **Recommendation:** Add a small `hover::signature(name, optional, ty)` helper. Use it in `parameter` and `property`
  and in the completion detail, so hover and completion cannot diverge.
- **Fix:** Added `hover::signature(name, optional, ty)`, which `parameter`, `property`, `field_signature` and the
  completion detail now all call. `lib.rs:534` and `lib.rs:2207` are left alone: they spell the compact
  `name?:T` form used inside a declaration listing, a different format from hover's single-entry `name?: T`.
- **Verification:** `hover::signature` is now the only place the `name?: T` form is built. `parameter`,
  `property`, `field_signature` and the completion detail all call it. `lib.rs:534` and `:2207` spell the compact
  `name?:T` form used inside a declaration listing, so leaving them alone is right.
  `cargo test -p nx-language-service` passes (126).

### ✅ Verified - RF6 Two small documentation defects in the playground README
- **Severity:** Low
- **Evidence:**
  - `sites/playground/README.md:174`: the edit left the paragraph's first line unwrapped, at about 200 characters,
    while the rest of the file wraps at 100.
  - `sites/playground/README.md:112`: it says moving the pin "takes three steps", but the block that follows lists
    four commands. The fourth is the `--source` alternative to the third, and nothing marks it as one.
- **Recommendation:** Rewrap the paragraph. Move the `--source` form out of the step list, or give it a comment
  (`# or, from another checkout`).
- **Fix:** Rewrapped the paragraph at 100 columns. The command block now lists only the three steps, and the
  `--source` form follows it as a sentence for a checkout outside `~/src/DrawnUi.React`.
- **Verification:** The paragraph is wrapped at 100 columns. The command block holds exactly the three steps, and
  the `--source` form is a sentence after it.


## New Findings Discovered During 2026-09-25 16:40 Verification

### ✅ Verified - RF7 A value written for a rejected function parameter is not type-checked, though the scenario now says it is
- **Severity:** Low
- **Evidence:** The delta scenario "A property rejected in the type slot is not reported again where it is left out"
  (`specs/optional-properties/spec.md:22`) now lists `let f(a:int, b:int?)`. Its last clause says type checking
  "SHALL still check a value written for it against the type it names". Functions do not do that. `let f(a:int,
  b:int?) = { a }` with `f(2, "x")` reports only the declaration. By comparison:
  - the correct spelling, `b?:int`, reports `Argument 1 expects int?, found string`;
  - the element form, `<Row gap="wide" />` over `gap:float64?`, still reports the mismatch.

  This predates the RF1 fix: HEAD behaves the same. Presumably the rejected parameter's type becomes an error type
  in the function's signature, which accepts anything. The scenario's test covers the value check only for `<Row>`
  (`crates/nx-types/tests/optional_properties.rs`).
- **Recommendation:** Build the function's parameter type from the declared type, as the element path does, so the
  argument is checked against `int?`. Add `f(2, "x")` to the test's value-check case, expecting two errors.
  Alternatively, narrow the scenario's last clause to the element and record forms.
- **Fix:** `infer_function` (`crates/nx-types/src/infer.rs`) now puts a rejected parameter into the function's
  signature as the fix-it writes it: `b:int?` becomes `b?:int`, and `b:T*` becomes `b?:T+`. The body still binds it
  as an error, as an element function's body already does, so the declaration stays the only report inside it.
  `f(2, "x")` now reports the declaration plus `Argument 1 expects int?, found string`, the same message the correct
  spelling gives. The value-check case in
  `a_property_rejected_in_the_type_slot_is_not_reported_again_where_it_is_left_out` covers it and fails without the
  fix. `cargo test --workspace` (2503), `cargo fmt --check` and clippy on `nx-types` pass.
- **Verification:** With the CLI rebuilt from the working tree, `f(2, "x")` now reports the declaration plus
  `Argument 1 expects int?, found string`. Further probes behave correctly:
  - an alias that carries zero (`s:Maybe`) checks its argument against `string?`;
  - a `t:int*` parameter checks against `int*` and can still be left off;
  - a body that uses the rejected parameter (`{ b + 1 }`) adds no second report.

  The test's value-check case now covers the function form. `cargo test --workspace` passes (2503).

## Questions
- 6.11 ran the browser check against the built site. Was `pnpm run dev` also exercised? Vite's dependency
  pre-bundling now handles `drawnui-react`'s `canvaskit.wasm?url` import from `node_modules`. The vendored tree never
  went through that path.

## Summary
- The implementation matches the proposal and design closely in both repositories.
- The fiddle's catalog, presets, tests and schema-4 fixture are in the new spelling, and its suite passes against
  this checkout.
- The playground draws from the published package with a byte-identical catalog, and its `applyProps` replacement
  is faithful to upstream.
- Both NX regressions are fixed with tests. RF1 through RF6 are fixed and verified.
- RF7, found during verification, is also fixed and verified. No findings remain open.
