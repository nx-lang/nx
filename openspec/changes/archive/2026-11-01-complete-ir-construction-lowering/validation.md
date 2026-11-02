# Implementation validation

Validated on November 1, 2026, after recovering from the interrupted read-only filesystem session.

## Automated checks

- `cargo test --workspace --locked`: 3,455 passed, 0 failed, 14 pre-existing ignores.
  This includes checker/language-service, codegen, Rust IR, generated JavaScript/TypeScript,
  conformance corpus, CLI, FFI, examples and documentation tests.
- `pnpm --dir runtime/typescript test`: passed the complete runtime, corpus, emitted-IR,
  standard-library and benchmark-harness correctness suites.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- `pnpm run check:crate-layering`: passed, including its 12 regression tests.
- `cargo check -p nx-ir-runtime --locked --target wasm32-wasip1`: passed.
- `cargo check -p nx-ir-runtime --locked --target wasm32-unknown-emscripten`: passed.
- `openspec validate complete-ir-construction-lowering --strict`: passed.

The workspace test/lint rebuild used `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and
`CARGO_PROFILE_TEST_DEBUG=line-tables-only` to reduce local cache size. No repository build-profile
configuration was changed.

## Compatibility and fixture review

The committed TypeScript runtime at the branch's pre-implementation HEAD was extracted outside the
working tree. Preparing the new conditional-properties image with it rejects the image with
`nx-ir-required-feature`, naming `conditional-properties-v1`, before evaluation.

Existing image/explanation changes preserve authored property order rather than sorting keys.
The binary sizes of those existing images are unchanged; their first-use string/node indices can
change with ordering. Existing expected values, operation counts and origin snapshots are unchanged.
New conditional-property corpus entries include handwritten results and ordered-error expectations.

New coverage exercises omission versus explicit empty, defaults depending on earlier selected fields,
nested/independent fragments, payload and numeric matches, content, updates, sequence-valued component
props, named declared calls and IR function-valued calls. Generated JavaScript instrumentation verifies
once-only authored ordering for all five invocation target kinds. Equal-increment checks at four, eight and twelve fragments cover independent, nested and
condition-list emitted growth without timing thresholds. Existing checker path enumeration is unchanged.

Independent implementation review and fix verification are recorded separately in `review.md`.

## Review fixes

After the independent review, the focused checker suite passed all 106 tests. Six new cases execute
update-record clearing/omission, numeric-pattern matches and intrinsic body content in generated
JavaScript, compare handwritten results with Rust IR, and type-check generated TypeScript.
Ordering instrumentation includes conditional and direct update records. The non-updating corpus
suite passes all 12 tests after the source-spacing fixture refresh, with 108 generated JavaScript
entrypoints compared; `conditional-properties` is not skipped as unsupported.

The proposed defaulted field that admits empty is not valid NX: optional properties cannot also
have defaults, and zero-admitting types belong on the property name. A new regression specifically
asserts `record-field-type-mismatch` for an explicit empty conditional binding on a defaulted required
field. The existing omission/default, optional empty and update-clear scenarios retain their distinct
source rules without expanding the language.

## Final verification

The full Rust workspace and full TypeScript runtime suites passed again after all review fixes and
fixture regeneration. The TypeScript run includes `corpus.test.mjs` against the final goldens.
Claude independently verified RF1–RF3 and RF5–RF10, resolved RF4 as an unreachable source scenario,
and reported no reopened or new findings. All 13 implementation tasks are complete. All five capability deltas were synced to the main specs
and the change was archived on November 1, 2026. Strict validation passed for all 95 main specs.
