# Implementation validation

All six tasks are complete. The math API is declared in NX source using `export external let`,
with function and parameter docs attached through the ordinary documentation pipeline. Every
public function and constant has documentation. Trusted module metadata supplies implementations;
source signatures are checked and unregistered external declarations produce analysis diagnostics.

Completed checks:

- `cargo test -q -p nx-syntax -p nx-hir`: 502 passing tests across 10 suites, including five new
  syntax/lowering integration tests. Missing result types, authored external bodies, unregistered
  implementations, signature drift, missing and duplicate declarations are covered.
- `cargo test -q -p nx-api -p nx-codegen -p nx-ir -p nx-ir-runtime -p nx-interpreter -p nx-value -p nx-language-service`:
  1,797 passing tests across 44 suites. The all-public-declarations documentation assertion and final
  aliased-completion test were added afterward and pass separately, along with all three focused
  math editor tests.
- Existing math tests retain numerical results, direct/passed/returned function behavior, generated
  TS/JS behavior, signed zeros, budgets and authored error origins. Declaration/parameter spans now
  point to NX source; runtime body/argument nodes remain synthetic.
- `NX_UPDATE_CORPUS=1 cargo test -q -p nx-codegen ir_corpus_tests`: ten tests pass. Only math-linked
  fixtures need updated source/content-version/debug metadata. Tracked math-free fixture files
  remain unchanged; ordinary conformance passes afterward.
- `pnpm --dir runtime/typescript test`: full suite passes, including 184 math fixtures through TS IR
  and generated TS/JS, emitted IR and persisted corpus checks.
- `pnpm --dir bindings/wasm build` and `pnpm --dir bindings/wasm exec vitest run test/standard-libraries.test.ts`:
  pass; all eight standard-library tests pass.
- `node sites/website/scripts/check-code-blocks.mjs sites/website/src/content/docs/reference/syntax/functions.md sites/website/src/content/docs/reference/libraries/math.md`:
  23 blocks pass. Declaration excerpts use `nx fragment` because standalone application source has
  no trusted implementation binding.
- `pnpm --dir sites/website build`: succeeds with all internal links valid.
- `cargo check -q --workspace`, `cargo fmt --all -- --check`, strict OpenSpec validation and
  `git diff --check` pass.

The completed math change remains archived. This follow-up remains active for review/sync/archive.
No additional runtime capability, callback API or external deployment is introduced.

## Claude review fixes

The independent Claude review found RF1–RF6. Fixes add TextMate declaration coverage, targeted
external-syntax recovery, a standard-libraries delta, unambiguous math links, source-oriented
binding errors and formal grammar documentation. The full Rust and TypeScript suites, 290 grammar
tests, changed documentation examples, website build/links, workspace check, formatting and strict
OpenSpec validation pass after these fixes. Math fixtures were refreshed for the documentation
source/version change; math-free fixture files remain unchanged. Verification is tracked in review.md.

The first Claude verification pass verified RF1–RF5, reopened RF6 (an omitted grammar sum-type
alternative), and added low-severity RF7 (UTF-8 JSON comment churn) and RF8 (external-value recovery).
Those three follow-up fixes are covered by ten focused syntax/lowering tests; affected five-crate
Rust regressions, all 290 TextMate tests, strict validation and formatting/diff checks pass. A second
independent verification pass is recorded in review.md.

The second independent verification accepted RF6–RF8 and added RF9 for misleading diagnostics on
half-typed external declarations. Validation and lowering now skip malformed recovery nodes, while
well-formed invalid shapes keep targeted syntax diagnostics. Parser, HIR and language-service
regressions cover the three reported typing cases, and invalid external values no longer project
as zero-parameter functions. All twelve focused tests, the editor diagnostic regression and the
affected five-crate regression run pass. RF9 verification is recorded in review.md.

Claude independently verified RF9 in the third verification pass. Cumulative result: RF1–RF9 all
Verified, zero reopened/open findings and zero new findings in the final pass. Existing shadowing
observations and a non-blocking malformed-forbidden-body recovery observation remain documented
in review.md; they are not outstanding findings. No resolution semantics changed.

## Bare external syntax revision

The user selected `export external sin(x:float64):float64`, dropping `let`. Live source,
signature diagnostics, compiler tests, TextMate rules/recovery boundaries, NX reference pages,
formal grammar documents and active specs use the shorter spelling. Historical validation and
review entries above describe the preceding syntax. The old spelling is explicitly rejected by
syntax tests. Existing external component declarations retain their spelling.

The focused external tests (13 cases), affected syntax/HIR/API/editor regressions, 290 TextMate
tests, focused math codegen/runtime checks and refreshed math corpus pass. Final runtime/docs
checks and independent Claude review are recorded below/on review.md.

The fresh Claude review of the bare external revision opened RF10–RF12: obsolete-spelling
recovery, TextMate contextual-keyword boundaries, and hover/completion declaration keywords.
Fixes preserve external origin explicitly in HIR, scope recovery to declaration-shaped prefixes,
and diagnose the obsolete let token while keeping later declarations. After fixes, all 3,008
tests in the affected seven Rust crates, all 294 TextMate tests, the full TypeScript suite, updated
WASM build and 23 documentation blocks pass, alongside strict OpenSpec/format/diff checks.
Independent verification is recorded in review.md.

The same Claude CLI session independently verified RF10–RF12. No findings were reopened or added.
All twelve cumulative findings are Verified, with zero open findings. Reviewer checks included
84 passing Rust suites across ten crates, all-target workspace checking, parser regeneration
identity, 294 TextMate tests, 184 numerical cases and documentation/hygiene checks. All eight
implementation tasks are complete. The follow-up remains active for sync/archive when requested.

The incorrect `external let` form never shipped. Its diagnostic is an error, not an obsolescence
or deprecation mechanism. The error-recovery token/variables and active proposal/spec wording
now reflect that, and the diagnostic hint explicitly says to remove `let`. Historical review
entries above and in review.md retain the wording used at the time.
