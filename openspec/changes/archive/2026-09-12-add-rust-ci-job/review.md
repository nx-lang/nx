# Review: add-rust-ci-job

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `.openspec.yaml` (`skip_specs: true`, so no
spec deltas to review)

**Reviewed code:**

- `.github/workflows/build.yml` (new `rust` job, lines 217-239), compared against the `build`,
  `package`, and `editor-assets` jobs and against `deploy-playground.yml`'s `validate` job
- `crates/nx-codegen/src/tests.rs` (staged + unstaged; `node_command`, `tsc_command`, the five
  execution helpers, and every call site)
- `docs/add-update-actions-followsup.md` (working tree, unstaged)
- Supporting context: `package.json`, `pnpm-workspace.yaml`, `runtime/typescript/package.json`,
  `rust-toolchain.toml`, `Cargo.toml`, the two `build.rs` files, `.gitignore`

**Verified locally:**

- `cargo fmt --all --check` — clean.
- `cargo test --workspace` — all green (89 nx-codegen tests, no failures anywhere; the only
  `#[ignore]`d tests are the pre-existing array-literal ones in `nx-interpreter`/`nx-types`).
- Failure mode of task 1.4 reproduced: with `node` off `PATH`,
  `cargo test -p nx-codegen javascript_output_executes_as_esm` fails at `tests.rs:209` with
  ``  `node` is not available: install Node 24 and put it on `PATH` ``.
- No `let Some(..) else { return; }` / bare `return;` sites remain (`grep -B1 -E '^\s*return;\s*$'`
  is empty), and no `node_is_available` or "skipping" behaviour survives anywhere in the Rust tree.
- `actionlint .github/workflows/build.yml` — clean. `rust` has no `needs`; `package` still needs
  only `build`.
- `openspec validate add-rust-ci-job --strict` — valid.
- CI preconditions hold: root `package.json` declares `packageManager: pnpm@10.28.1`, root
  `pnpm-lock.yaml` is tracked, `runtime/typescript` is a workspace member with `typescript` as a
  devDependency, no workspace package has an install hook that would shell out to cargo, and
  `crates/nx-syntax/src/parser.c` is checked in so the grammar needs no Node at build time.

## Findings

### ✅ Verified - RF1 Task 2.2 is unfinished: the `🦀 Rust` job has never actually run, and nothing is committed
- **Severity:** Medium
- **Evidence:** `tasks.md` item 2.2 ("Push the branch and open or update the pull request. Verify the
  `🦀 Rust` job runs... and that it finishes green") is the one unchecked task, and `git status` shows
  `.github/workflows/build.yml` staged-but-uncommitted with `crates/nx-codegen/src/tests.rs` split
  across index and working tree. Everything I could check locally passes, but the change's entire
  point is a green CI job, and no runner has executed it. The parts only CI can prove are the ones
  most likely to bite: `pnpm install --frozen-lockfile` resolving against the root lockfile on a
  clean checkout, `runtime/typescript/node_modules/.bin/tsc` existing after that install, and
  `cargo test --workspace` compiling `bindings/node/native` (a napi `cdylib`) on `ubuntu-latest`
  with a fresh dependency resolution and no `Cargo.lock`.
- **Recommendation:** Commit, push, and confirm the job green; check the log shows `node --version`
  at 24 and that `generated_typescript_type_checks`, `generated_component_typescript_type_checks`,
  and the `execute_script_against_emitted_runtime` tests ran rather than skipped. Then mark 2.2 done.
- **Status:** Not addressed here — this needs a push and a CI run, and the working branch
  `add-update-actions` carries unrelated uncommitted work (`docs/scratch-highlighting.nx`,
  `drawnup-startup-fixes-review.md`, `examples/nx/template-candidate.nx`, and two other change
  directories), so how to commit is the user's call. Everything locally verifiable still passes
  after this fix pass: `cargo fmt --all --check` clean, `cargo test --workspace` green,
  `actionlint` clean, `openspec validate --strict` valid. Task 2.2 stays unchecked.
- **Verification:** Verified against a real CI run. The branch is committed and pushed (`934cfe6`
  for this change, `aed9a9b` for an unrelated .NET doc-comment fix) and PR
  [#23](https://github.com/nx-lang/nx/pull/23) is open, so task 2.2 is now marked done. In run
  [34715125679](https://github.com/nx-lang/nx/actions/runs/34715125679) the `🦀 Rust` job
  **succeeded**, and its log satisfies each of 2.2's checks:
  `node: v24.20.0` from the setup step; `test result: ok. 89 passed; 0 failed` for nx-codegen; and
  `javascript_output_executes_as_esm`, `generated_typescript_type_checks`, and
  `generated_component_typescript_type_checks` each reported `... ok`, so the tests that used to
  skip executed. `grep -ci skipping` over the whole job log returns 0. The preconditions I could
  only infer locally all held on the runner: `pnpm install --frozen-lockfile` resolved against the
  root lockfile on a clean checkout, `runtime/typescript`'s `tsc` was present for the TypeScript
  tests, and `cargo test --workspace` compiled `bindings/node/native` with no `Cargo.lock` and no
  exclusion needed. The one failure in that run was `🏭 Build (ubuntu-latest)`'s
  `dotnet format --verify-no-changes` step on a pre-existing DOC104 warning in
  `NxOptionalSerialization.cs`, unrelated to this change and fixed in `aed9a9b`; every other job
  (all three `🏭 Build` OSes, `📦 Editor assets`, and the VS Code Extension workflow) was green.
  The branch-protection question under Questions is still open — the job is green but gates nothing
  until it is a required check.

### ✅ Verified - RF2 Two `tsc` tests inline ~30 lines each that are byte-identical to `assert_generated_typescript_artifact_type_checks`
- **Severity:** Medium
- **Evidence:** `crates/nx-codegen/src/tests.rs:1405-1435` (`generated_typescript_type_checks`) and
  `crates/nx-codegen/src/tests.rs:1455-1485` (`generated_component_typescript_type_checks`) are
  character-for-character the body of the helper at
  [tests.rs:213-244](crates/nx-codegen/src/tests.rs#L213-L244) — same `emit_program`, same
  `tsc_command()`, same temp dir and `package.json`, same nine `tsc` args, same assertion. This
  change edited exactly those blocks (removing the `let Some(mut tsc) = tsc_command() else` guard),
  so the duplication was directly in hand. Both test bodies reduce to one call:
  `assert_generated_typescript_artifact_type_checks(&artifact);`.
- **Recommendation:** Replace both inlined blocks with the helper call, dropping ~60 lines and the
  two now-unneeded `emit_program`/`tsc_command` lines. The helper already takes `&ProgramArtifact`
  and is behaviourally identical, so this is a pure substitution.
- **Fix:** Both test bodies are now the single call
  `assert_generated_typescript_artifact_type_checks(&artifact);`
  ([tests.rs:1435](crates/nx-codegen/src/tests.rs#L1435),
  [tests.rs:1455](crates/nx-codegen/src/tests.rs#L1455)), dropping 60 duplicated lines. Both tests
  still pass.
- **Verification:** Verified. Both tests are now a single
  `assert_generated_typescript_artifact_type_checks(&artifact);`
  ([tests.rs:1435](crates/nx-codegen/src/tests.rs#L1435),
  [tests.rs:1455](crates/nx-codegen/src/tests.rs#L1455)); the helper at
  [tests.rs:244-275](crates/nx-codegen/src/tests.rs#L244-L275) is unchanged, so the substitution is
  behaviour-preserving, and the `tsc` args and assertion the tests exercise are the same nine flags
  as before. `cargo test -p nx-codegen typescript_type_checks` passes both.

### ✅ Verified - RF3 `node_command` never checks the Node version it probes, so the named error only covers "missing", not "too old"
- **Severity:** Low
- **Evidence:** [tests.rs:207-212](crates/nx-codegen/src/tests.rs#L207-L212) runs
  `node --version` and discards the output, treating only a spawn failure as an error. A `node` that
  is present but older than what the generated ESM needs still passes the probe, and the test then
  fails with the generic `assert!(output.status.success(), "stderr: {}", ...)` in the caller — the
  opposite of the design's goal that a missing-or-wrong tool "fails loudly" with a message naming
  the fix. Relatedly, the panic text says "install Node 24" while the repository's own declared
  requirement is `engines.node: >=22.18.0` in `package.json`, so the message names a stricter
  version than the project asks for.
- **Recommendation:** Parse the `vNN.` prefix from the probe's stdout and panic with the same style
  of message when it is below the supported floor; quote that floor (`>= 22.18`, per
  `package.json`) rather than the CI pin, or state both ("Node >= 22.18; CI uses 24").
- **Fix:** `node_command` now keeps the probe's output: it requires
  `status.success()`, parses the `vNN.NN` prefix through a new `parse_node_version`, and asserts it
  is at least `MINIMUM_NODE_VERSION`, a named `(22, 18)` sourced from `engines.node`. All three
  failure paths quote one shared `NODE_REQUIREMENT` string, "these tests need Node >= 22.18 on
  `PATH`, as CI uses Node 24", so the message names both the project floor and the CI pin. Verified
  with a shim reporting `v20.0.0`: the run fails with ``` `node` is v20.0.0: these tests need Node
  >= 22.18 on `PATH`, as CI uses Node 24 ```, and with `node` off `PATH` entirely it still fails
  with the not-available form.
- **Verification:** Verified independently with two shims.
  (1) A `node` shim reporting `v20.0.0`: `javascript_output_executes_as_esm` fails at
  [tests.rs:228](crates/nx-codegen/src/tests.rs#L228) with ``` `node` is v20.0.0: these tests need
  Node >= 22.18 on `PATH`, as CI uses Node 24 ```, so the too-old case now gets the named message
  instead of a `tsc`/`node` diagnostic dump.
  (2) With `node` off `PATH` entirely, all 11 `generated_javascript_*` tests fail with the
  ``` `node` is not available: <requirement> ``` form.
  `MINIMUM_NODE_VERSION` is `(22, 18)`, matching `engines.node: >=22.18.0`, and tuple comparison
  orders correctly across a major bump (`(24, 15) >= (22, 18)`). `parse_node_version`
  ([tests.rs:237-242](crates/nx-codegen/src/tests.rs#L237-L242)) falls through to the
  ``` `node --version` printed ... ``` panic on anything it cannot parse, so no probe outcome is
  silently accepted. One residual nit is carried forward as RF10.

### ✅ Verified - RF4 The `node` probe re-spawns a process on every helper call, though the task specifies probing once
- **Severity:** Low
- **Evidence:** Task 1.1 asks for a `node_command()` "that probes `node --version` **once**".
  [tests.rs:208](crates/nx-codegen/src/tests.rs#L208) probes on each of the three call sites'
  invocations (`tests.rs:92`, `130`, `192`), so every executing codegen test pays two process spawns
  instead of one. This matches the old `node_is_available()` behaviour, so it is not a regression —
  but the task wording reads as a memoized probe, and nothing memoizes it.
- **Recommendation:** Cache the probe result in a `static OnceLock<()>` (or a `LazyLock` holding the
  resolved version), or reword task 1.1 so the intent matches the implementation.
- **Fix:** The probe is memoized in a `static PROBE: OnceLock<()>`, so task 1.1's "probes
  `node --version` once" is now literal. Verified with a counting shim: a run of the 14 tests
  matching `javascript` logs exactly one `node --version` call, where it previously logged one per
  executing test.
- **Verification:** Verified by counting. A `node` shim that logs every `--version`
  invocation and delegates to the real binary records **exactly 1** probe across a run of the 14
  tests matching `javascript` (all 14 pass), where the old code would have logged one per executing
  test. Also checked the failure interaction the memoization could have broken: because
  `OnceLock::get_or_init` leaves the cell uninitialized when its closure panics, a bad `node` still
  fails *every* dependent test rather than only the first — confirmed by the 11-of-11 failures in
  RF3's second shim run. Task 1.1's "probes `node --version` once" is now literal
  ([tests.rs:217](crates/nx-codegen/src/tests.rs#L217)). Note for context, not a defect:
  `tsc_command` is deliberately left unmemoized, and its four call sites re-probe; task 1.2 never
  asked for a single probe and the cost is two spawns per `tsc` test.

### 🔴 Open - RF5 The working tree's `docs/add-update-actions-followsup.md` diff rewrites item 1, which is outside this change's stated impact
- **Severity:** Low
- **Evidence:** Task 1.7 says to rewrite item 5 "leaving items 1 through 4 and 6 untouched", and
  `proposal.md`'s Impact lists only "item 5 is closed". Item 5 is correctly closed, and items 2, 3,
  4, and 6 are byte-identical to `HEAD` (verified by per-section hashing). But the intro paragraph
  and the whole of item 1 are rewritten — retitled from "Proposal: first-class properties, and
  update records as property maps" to "Map-backed update records and typed property keys in the .NET
  SDK", and now crediting `add-property-references` as landed. That rewrite is `add-property-references`
  bookkeeping, not this change's, and it is unstaged alongside this change's work, so the two will
  land in one indistinguishable commit.
- **Recommendation:** Commit the item-1 rewrite separately (attributed to `add-property-references`),
  or add it to this change's Impact so the diff matches the artifacts.
- **Status:** Left to the commit, not the code — the item-1 rewrite is pre-existing uncommitted
  work in the tree and reverting it here would discard it. Recommendation stands: stage
  `docs/add-update-actions-followsup.md`'s item 5 and intro-paragraph-plus-item-1 rewrite as two
  commits, attributing the latter to `add-property-references`.

### ✅ Verified - RF6 The new job duplicates the first five steps of `deploy-playground.yml`'s `validate` job verbatim
- **Severity:** Low
- **Evidence:** [build.yml:222-235](.github/workflows/build.yml#L222-L235) and
  `deploy-playground.yml:47-66` are the same five steps in the same order with the same inputs —
  pinned `actions/checkout`, `pnpm/action-setup@v4`, `actions/setup-node@v4` at Node 24 with
  `cache: 'pnpm'`, `Swatinem/rust-cache@v2`, `pnpm install --frozen-lockfile`. Design D2/D3 chose
  this deliberately as "the conventions the neighbouring workflows already use", but the result is a
  second copy that will drift (the Node version and the pnpm setup are now stated twice). The repo
  already uses composite actions for exactly this (`./.github/actions/publish-artifacts`).
- **Recommendation:** Either extract a `./.github/actions/setup-rust-node` composite action and call
  it from both jobs, or record the accepted duplication in `design.md` so the next Node bump knows
  there are two places to edit.
- **Fix:** Extracted, not just recorded. `.github/actions/setup-rust-node/action.yaml` is a new
  composite action holding the four shared steps (pnpm, Node 24 with the pnpm cache,
  `Swatinem/rust-cache@v2`, `pnpm install --frozen-lockfile`), and both the `rust` job and
  `deploy-playground.yml`'s `validate` job now call it after their checkout — checkout has to stay
  in the jobs, since a local action cannot run before the repository is there. The Node version and
  the install flags are stated in exactly one place, and `deploy-playground.yml` gained
  `.github/actions/setup-rust-node/action.yaml` in its `paths` filter so a change to the setup
  still validates the site. Recorded as design decision D7, with the new files in `proposal.md`'s
  Impact and the work as task 2.3. The `src/vscode` jobs that repeat a similar block
  (`editor-assets` plus four extension and release workflows) are left alone on purpose: they set
  up a different lockfile under `src/vscode` and need no Rust, so folding them in would mean
  parameterizing the action past the point where it reads better than the steps it replaces — see
  the note under Questions. Verified: `actionlint` clean on both workflows and across the whole
  `.github/workflows` tree, all three files parse, and the steps left in each job are unchanged
  (`rust`: checkout, setup, `cargo fmt --all --check`, `cargo test --workspace`; `validate`:
  checkout, setup, build, test, typecheck).
- **Verification:** Verified as resolved by documentation, which the finding offered as
  one of two acceptable outcomes. `design.md`'s Risks / Trade-offs now carries a bullet naming the
  five duplicated steps, both files that hold them, the reason for accepting the copy, and the
  trigger for extracting a composite action ("Extract one if a third appears"). The duplication
  itself is unchanged and intentionally so; a future Node or pnpm bump still has two places to edit,
  but that is now recorded where the next reader will find it.
- **Note (after that verification):** This verification read the tree as it stood at 15:21, when the
  fix was the `design.md` bullet it describes. That bullet no longer exists: on the user's
  preference for abstracted CI YAML the duplication was then actually removed, as the **Fix** note
  above records, and `design.md` carries decision D7 instead of a Risks / Trade-offs entry. The
  duplication is gone rather than documented, so "a future Node or pnpm bump still has two places
  to edit" no longer holds — there is one.
- **Verification (re-verified after the extraction, 2026-09-12 15:47):** Verified in the new form.
  `.github/actions/setup-rust-node/action.yaml` is a well-formed composite action — `using:
  composite`, `shell: bash` on its only `run` step, `action.yaml` matching the existing
  `publish-artifacts` action's filename and layout — and holds the four shared steps exactly once.
  Both callers reduce to `uses: ./.github/actions/setup-rust-node` after their own checkout, and
  each job's remaining steps are unchanged (`rust`: checkout, setup, `cargo fmt --all --check`,
  `cargo test --workspace`; `validate`: checkout, setup, `pnpm -r build`, `pnpm -r test`, the
  playground typecheck). The `paths` entry added to `deploy-playground.yml` matches the action's
  real path character-for-character, so a setup change still triggers the site validation. Nothing
  about the extraction changes runtime behaviour: `pnpm/action-setup@v4` still resolves
  `packageManager` from the root `package.json`, and `Swatinem/rust-cache@v2` still keys per `job`,
  so `rust` and `validate` keep separate caches. `actionlint` over the whole tree is clean (exit 0),
  and `openspec validate add-rust-ci-job --strict` is valid. The bookkeeping is complete too:
  decision D7, the two new/changed files in `proposal.md`'s Impact, and task 2.3 marked done. One
  wording nit in the action's own `description` is raised as RF11.

### ✅ Verified - RF7 The step comment sits above the wrong step, so it reads as documenting the Rust cache
- **Severity:** Low
- **Evidence:** [build.yml:230-233](.github/workflows/build.yml#L230-L233) places "The codegen tests
  execute generated JavaScript with `node` and check generated TypeScript with the `tsc` from
  `runtime/typescript`; rustup picks the toolchain from rust-toolchain.toml" immediately above
  `- name: Cache the Rust build`. The first clause explains the *Node/pnpm* steps above it and the
  second explains the *cargo* steps below it, and neither describes the cache step it is attached to.
- **Recommendation:** Split it: put the node/tsc sentence above `Setup pnpm` (or above
  `Install dependencies`, which is what actually provides `tsc`) and keep the one-line rustup comment
  where `deploy-playground.yml` has it, directly above `Cache the Rust build`.
- **Fix:** Split in two. The node/tsc sentence moved above `Setup pnpm` and now says the `tsc`
  comes from the pnpm install below; the one-line "rustup picks the toolchain from
  rust-toolchain.toml" stays directly above `Cache the Rust build`, matching
  `deploy-playground.yml`. `actionlint` still clean.
- **Verification:** Verified. The node/tsc sentence now sits above `Setup pnpm`
  ([build.yml:223-224](.github/workflows/build.yml#L223-L224)) and is reworded to say the `tsc`
  comes from "the pnpm install below", and the one-line "rustup picks the toolchain from
  rust-toolchain.toml" sits directly above `Cache the Rust build`
  ([build.yml:232](.github/workflows/build.yml#L232)), matching `deploy-playground.yml`'s placement.
  Each comment now describes the step it precedes. `actionlint .github/workflows/build.yml` is
  clean and the job still has no `needs`.

### ✅ Verified - RF8 `tsc_command`'s probe accepts any `tsc` that merely spawns, including a broken one
- **Severity:** Low
- **Evidence:** [tests.rs:152](crates/nx-codegen/src/tests.rs#L152) uses
  `Command::new(&program).arg("--version").output().is_ok()`, which is true whenever the process
  *started*, regardless of exit status. A `tsc` shim on `PATH` that exits non-zero is selected and
  the workspace `tsc` is never tried, and all four `tsc` tests then fail with a `tsc` diagnostic
  dump instead of the "run `pnpm install`" message. Pre-existing, but this change is the one that
  makes the message load-bearing (it is now the only guidance a failing run gets).
- **Recommendation:** Require `output.status.success()` in the probe so a non-working `tsc` on
  `PATH` falls through to `runtime/typescript/node_modules/.bin/tsc`.
- **Fix:** The probe is now `probe.is_ok_and(|output| output.status.success())`, so a `tsc` that
  starts but exits non-zero falls through to `runtime/typescript/node_modules/.bin/tsc`. Verified
  with a shim `tsc` that exits 1 first on `PATH`: `generated_typescript_type_checks` passes on the
  workspace compiler instead of dumping diagnostics.
- **Verification:** Verified with a broken-`tsc` shim first on `PATH` that logs its
  arguments, writes to stderr, and exits 1. The log shows the shim being probed
  (`broken tsc invoked: --version`, once per test), and both
  `generated_typescript_type_checks` and `generated_component_typescript_type_checks` **pass** on
  the workspace compiler — the probe rejected the non-zero exit and fell through to
  `runtime/typescript/node_modules/.bin/tsc`, which is exactly the behaviour the old
  `.output().is_ok()` could not give. Implementation is
  `probe.is_ok_and(|output| output.status.success())` at
  [tests.rs:153-154](crates/nx-codegen/src/tests.rs#L153-L154).

### 🔴 Open - RF9 The job installs the entire root pnpm workspace to obtain one `tsc`
- **Severity:** Low
- **Evidence:** [build.yml:235](.github/workflows/build.yml#L235) runs a bare
  `pnpm install --frozen-lockfile`, which per `pnpm-workspace.yaml` pulls `packages/*`,
  `bindings/node`, `runtime/typescript`, and `sites/*` — including the playground's `vite` and
  `monaco` trees. The only thing `crates/nx-codegen/src/tests.rs` needs from the install is
  `runtime/typescript/node_modules/.bin/tsc`.
- **Recommendation:** Optional, for job time on a cold pnpm cache:
  `pnpm install --frozen-lockfile --filter @nx-lang/ir-runtime...`. Worth measuring against the full
  install once RF1 gives a real timing to compare.
- **Status:** Not addressed, as recommended — a `--filter` is a speed optimization worth
  measuring against a real cold-cache timing, which only RF1's first CI run can provide. Revisit
  with that number in hand. One thing changed since: RF6's extraction moved
  `pnpm install --frozen-lockfile` out of the job and into
  `.github/actions/setup-rust-node/action.yaml`, which `deploy-playground.yml`'s `validate` job also
  calls — and that job needs the whole workspace for `pnpm -r build`. So a `--filter` can no longer
  just be appended to the step: it would have to be an input on the action, defaulting to the full
  install. That raises the cost of the optimization and is another reason to wait for a measurement
  that shows it is worth an input.

## Questions

- Does the `🦀 Rust` job need to be added to the branch-protection required-check list?
  `proposal.md` states "a Rust test failure blocks the pull request through the required check",
  but that is a repository setting no file here controls and no task covers it. If it is not made
  required, a red `🦀 Rust` job is advisory only.
- `cargo test --workspace` runs without `RUSTFLAGS: -D warnings`, so new rustc warnings stay
  invisible in CI. Clippy is an explicit non-goal, but `-D warnings` is a cheaper, narrower version
  of the same idea — deliberate omission, or worth folding in here?

- The eight jobs across seven workflows that set up pnpm and Node 24 for `src/vscode`
  (`build.yml`'s `editor-assets`, `vscode-extension.yml`, `vscode-extension-publish.yml`,
  `vscode-release.yml`, `release.yml`, and `package-publish.yml`) still each write the block out,
  and they are not even consistent — some pin `actions/setup-node` by SHA, some use `@v4`. A second
  composite action with `package-json-file` and `cache-dependency-path` inputs would cover them,
  but it touches the publish and release paths, so it belongs in its own change rather than this
  one.

## Summary

The implementation matches the design closely and the parts I could verify locally all hold: the
suite is green, `cargo fmt` is clean, every skip path is gone (no `node_is_available`, no bare
`return;`, no "skipping" behaviour anywhere in the Rust tree), the hidden-`node` run fails with the
intended panic, the workflow parses cleanly under `actionlint`, the job correctly carries no `needs`,
and the CI preconditions (`packageManager`, tracked root lockfile, `runtime/typescript` in the
workspace, checked-in `parser.c`, no cargo-invoking install hooks) are all satisfied.

Nine findings, none blocking the approach. RF1 (the job has never run, and nothing is committed) is
the one that matters for confidence — every local check passes, but the deliverable is a green CI job
and that remains unproven. RF2 is the clearest code improvement: ~60 lines in two tests this change
already edited are an exact copy of an existing helper. The rest are message accuracy (RF3), a
memoization mismatch with task 1.1 (RF4), diff hygiene (RF5), workflow duplication and comment
placement (RF6, RF7), and two small robustness/efficiency items (RF8, RF9).

## Fix pass

Six findings fixed (RF2, RF3, RF4, RF6, RF7, RF8), three left open (RF1, RF5, RF9). The code fixes
are all in `crates/nx-codegen/src/tests.rs` and `.github/workflows/build.yml`, plus one
new composite action in `.github/actions/setup-rust-node/`, with decision D7 in `design.md` and
matching updates to `proposal.md` and `tasks.md`. Re-verified afterwards: `cargo fmt --all --check` clean,
`cargo test --workspace` green (89 nx-codegen tests, no failures anywhere), `actionlint
.github/workflows/build.yml` clean, `openspec validate add-rust-ci-job --strict` valid, and the
three panic paths (`node` missing, `node` too old, `tsc` broken on `PATH`) each exercised with a
shim. The three open findings all need something outside the code: a CI run (RF1, RF9) or a
decision about how to split the commit (RF5).

### Second pass, after the 15:21 verification

RF10, the one new finding, is fixed: the floor in the panic advice is formatted from
`MINIMUM_NODE_VERSION` instead of retyped, verified by temporarily raising the constant and
watching the message follow. RF6's verification note is annotated, because it verified the
documentation-only fix that the later extraction replaced — the duplication is now removed, not
recorded. RF9's status note picks up a consequence of that extraction: the shared install step
means a `--filter` would need an action input rather than a step edit. RF1 and RF5 are unchanged
and still need a push and a commit-splitting decision respectively. Re-verified after this pass:
`cargo fmt --all --check` clean, `cargo test --workspace` green (89 nx-codegen tests), all three
panic paths reproduced with shims, `actionlint` clean across all ten workflows, and
`openspec validate add-rust-ci-job --strict` valid.

### Third pass, after the 15:47 verification

RF11, the one new finding, is fixed: the composite action's `description` no longer promises a
repository-wide single edit, and D7 now accounts for all six other Node pins including
`package-publish.yml`. No code changed in this pass — it is YAML prose and design text only — so
the Rust verification from the second pass still stands; `actionlint` over the whole tree and
`openspec validate --strict` were re-run and are clean. RF1, RF5, and RF9 are unchanged: a push,
a commit-splitting decision, and a measurement respectively, none of which can be settled from
here.

## New Findings Discovered During 2026-09-12 15:21 Verification

### ✅ Verified - RF10 The Node requirement message restates both version numbers as literal text, so a bump silently makes it wrong
- **Severity:** Low
- **Evidence:** `NODE_REQUIREMENT` at
  [tests.rs:209](crates/nx-codegen/src/tests.rs#L209) is the literal
  `"these tests need Node >= 22.18 on `PATH`, as CI uses Node 24"`. Both numbers in it already
  exist elsewhere: `22.18` is `MINIMUM_NODE_VERSION` two lines above
  ([tests.rs:206](crates/nx-codegen/src/tests.rs#L206), itself sourced from `engines.node`), and
  `24` is `node-version: '24'` in [build.yml:228](.github/workflows/build.yml#L228). Raising
  `MINIMUM_NODE_VERSION` to, say, `(24, 0)` leaves the panic advising `>= 22.18` while the assertion
  rejects 22 — the exact drift RF3 was opened about, one level up. This is a nit, not a live bug:
  the three numbers agree today, and I verified the message the `v20.0.0` shim produces is accurate.
- **Recommendation:** Derive the floor from the constant rather than repeating it, e.g. a
  `fn node_requirement() -> String` that formats `MINIMUM_NODE_VERSION.0`/`.1`, or `concat!` over a
  pair of `const` digit strings that `MINIMUM_NODE_VERSION` is also built from. The CI pin is
  cross-file, so a comment on `NODE_REQUIREMENT` pointing at `build.yml`'s `node-version` is enough
  for that half.
- **Fix:** Took the first option. `NODE_REQUIREMENT` is gone; `node_requirement() -> String`
  formats the floor from `MINIMUM_NODE_VERSION.0`/`.1`, so the constant is the only place the floor
  is written, and all three failure paths call it. The CI half stays literal with a doc comment
  pointing at where the pin now lives — `.github/actions/setup-rust-node/action.yaml`, not
  `build.yml`, since RF6's extraction moved it. Verified by raising `MINIMUM_NODE_VERSION` to
  `(99, 7)` in a scratch edit: the panic became ``` `node` is v24.15.0: these tests need Node >=
  99.7 on `PATH`, as CI uses Node 24 ```, then restored. All three panic paths re-checked against
  the restored file (`node` absent from `PATH`, a `v20.0.0` shim, a `tsc` shim exiting 1), plus
  `cargo fmt --all --check` clean and `cargo test --workspace` green.
- **Verification:** Verified, including the claim the fix rests on. `NODE_REQUIREMENT` is gone and
  `node_requirement()` ([tests.rs:213-218](crates/nx-codegen/src/tests.rs#L213-L218)) formats the
  floor from `MINIMUM_NODE_VERSION`; all three panic paths call it. Checked the derivation
  independently rather than trusting it: with the constant temporarily raised to `(99, 7)` the panic
  read ``` `node` is v24.15.0: these tests need Node >= 99.7 on `PATH`, as CI uses Node 24 ```, then
  I restored the file and confirmed it byte-identical to the pre-edit copy. The three panic paths
  still behave on the restored tree — `node` absent → ``` `node` is not available: these tests need
  Node >= 22.18 ... ```, a `v20.0.0` shim → ``` `node` is v20.0.0: ... ```, a `tsc` shim exiting 1 →
  both `tsc` tests pass on the workspace compiler — the memoized probe still fires exactly once
  across 14 executing tests, `cargo fmt --all --check` is clean, and `cargo test --workspace` is
  green (89 nx-codegen tests). The CI half stays a literal `24`, correctly, with the doc comment now
  pointing at `.github/actions/setup-rust-node/action.yaml` where RF6's extraction moved the pin.

## New Findings Discovered During 2026-09-12 15:47 Verification

### ✅ Verified - RF11 The composite action's description claims the Node version lives in one place, but six other CI pins still set it
- **Severity:** Low
- **Evidence:** `.github/actions/setup-rust-node/action.yaml`'s `description` says "The Node version
  lives here, so a bump is one edit." Repo-wide that is not so: `node-version: '24'` also appears in
  `build.yml:199` (`editor-assets`), `vscode-extension.yml:60`, `vscode-extension-publish.yml:91`,
  `vscode-release.yml:95`, `release.yml:184`, and `package-publish.yml:98`. Design D7 scopes the
  claim correctly — it names the five `src/vscode` jobs and explains why folding them in would mean
  over-parameterizing the action (`package-publish.yml` is a sixth pin D7 doesn't mention, and is
  fairly out of scope: it sets up Node for npm trusted publishing, with no pnpm, no lockfile, and no
  Rust). The problem is only that the action's own description, read by someone bumping Node without
  opening `design.md`, promises a single edit that would leave six pins behind. Same class as RF10 —
  a doc string asserting a single source of truth that isn't one — just in YAML rather than Rust.
- **Recommendation:** Scope the sentence to what the action actually owns, e.g. "The Node version
  for the Rust jobs lives here"; optionally add "the `src/vscode` jobs pin their own — see D7".
- **Fix:** The sentence is scoped and the exception is named in the action itself, so it holds
  without `design.md` in hand: "The Node version for the jobs that do that lives here, so bumping it
  is one edit; the `src/vscode` jobs and `package-publish.yml` set up Node on their own terms and
  pin it themselves." D7 was amended to match — it now counts all six other pins and says why
  `package-publish.yml` is the furthest out of scope (Node for npm trusted publishing: no pnpm, no
  lockfile, no Rust), and states that the action owns the Node version for the jobs that build Rust
  against the workspace, not for the repository. Verified by counting `node-version: '24'` across
  `.github`: exactly seven, one per file — the action plus the six the description now excludes.
  `actionlint` clean over the whole tree, the action's `description` still parses as one scalar, and
  `openspec validate add-rust-ci-job --strict` valid.
- **Verification:** Verified. The `description` now reads "The Node version for the jobs that do
  that lives here, so bumping it is one edit; the `src/vscode` jobs and `package-publish.yml` set up
  Node on their own terms and pin it themselves" — scoped, and it names the exception without
  needing `design.md`. I recounted rather than trusting the note: `node-version: '24'` appears
  exactly 7 times under `.github/`, one per file — the action plus the six the sentence now excludes
  (`build.yml`'s `editor-assets`, `vscode-extension.yml`, `vscode-extension-publish.yml`,
  `vscode-release.yml`, `release.yml`, `package-publish.yml`); `deploy-docs.yml:31` is a
  commented-out sample line, correctly not counted. The six are characterized accurately — the five
  `src/vscode` ones each pair `package_json_file: src/vscode/package.json` with
  `cache-dependency-path: src/vscode/pnpm-lock.yaml`, and `package-publish.yml` sets up Node with
  `registry-url` and `cache: ''` for npm trusted publishing, with no pnpm, lockfile, or Rust. D7 was
  amended to match and now counts all six. Mechanically sound: the `description` still parses as a
  single scalar, `using: composite` with every `run` step carrying `shell: bash`, `actionlint` over
  the whole tree clean (exit 0), and `openspec validate add-rust-ci-job --strict` valid. The Rust
  side is byte-identical to the tree I verified for RF10 (`cargo fmt --all --check` clean,
  89 nx-codegen tests green), so nothing re-verified there changed.
