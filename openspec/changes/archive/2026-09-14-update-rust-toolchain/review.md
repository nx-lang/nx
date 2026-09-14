# Review: update-rust-toolchain

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, `.openspec.yaml` (specs skipped — justified; no
spec-level behavior changes)

**Reviewed code:** the full working-tree diff — `rust-toolchain.toml`, `README.md`,
`docs/src/content/docs/tutorials/getting-started.md`, `crates/nx-interpreter/src/interpreter.rs`,
`crates/nx-language-service/src/positions.rs` — plus the build paths the change claims to reach:
`.github/actions/setup-rust-node/action.yaml`, `.github/workflows/build.yml`,
`.github/workflows/deploy-playground.yml`, `.github/workflows/vscode-extension.yml`,
`.github/workflows/vscode-release.yml`, `src/vscode/scripts/build-lsp.mjs`, `sites/playground/Dockerfile`,
`Cargo.toml`, `.github/renovate.json`. The RF5 follow-up work added `Cargo.toml`'s `resolver` and
`rust-version` fields and the second Renovate custom manager to the reviewed surface; the lockfile work added
`.gitignore`, `Cargo.lock`, `sites/playground/Dockerfile`, and the `build.yml` and `deploy-playground.yml`
workflows.

**Verification re-run during this review (all green):**

- `rustc --version` → 1.98.1; `rustup show` → active via `rust-toolchain.toml`, `wasm32-wasip1` installed
- `rustup toolchain list` → only `stable` and `1.98.1` (tasks 4.1/4.2 confirmed); `stable` is 1.98.1
- `cargo fmt --all --check` → clean
- `cargo test --workspace` → 0 failures
- `pnpm -r build` → wasm module built (`bindings/wasm/dist/nx.wasm`, hashed into the playground bundle)
- `pnpm -r test` → exit 0, 0 failures, wasm/Node SDK parity and the 20 playground examples pass
- `cargo clippy --workspace --all-targets --keep-going` → the two `useless_conversion` sites and the one
  `while_let_loop` site are gone; the residual set matches what design.md describes as pre-existing

Both code edits are semantically identical to what they replace (`Iterator::zip` has always taken an
`IntoIterator`; the `loop`/`let…else`/`break` and the `while let` are the same control flow), and neither uses
anything newer than the declared MSRV.

## Findings

### ✅ Verified - RF1 design.md's premise that the VS Code extension workflows escape the pin is wrong, so the change's real blast radius — the released VSIX — is undocumented and unverified

- **Severity:** Medium
- **Evidence:** `design.md:9` states "The two VS Code extension workflows use `dtolnay/rust-toolchain@stable`
  rather than the pin. They only build the extension and are outside this change," and `design.md:24` makes
  touching them a non-goal. The action does not do that. Its only selector step is
  `rustup default stable` (verified against the action's own `action.yml`); it never exports
  `RUSTUP_TOOLCHAIN`. `rustup default` is the lowest-precedence selector — a `rust-toolchain.toml` found by
  walking up from the working directory wins over it. `src/vscode/scripts/build-lsp.mjs:26-27` spawns cargo
  with `cwd: repoRoot`, and `./rust-toolchain.toml` is the only toolchain file in the repository. So
  `.github/workflows/vscode-extension.yml:49` and `.github/workflows/vscode-release.yml:86` install stable and
  then build `nx-lsp` with the pin anyway. Consequences: (a) the next released VSIX ships an `nx-lsp` compiled
  by 1.98.1 as a direct result of this change, which `proposal.md`'s Impact section does not list; (b) those
  two jobs now download two toolchains per run, and the `Setup Rust` step is dead weight the pin supersedes;
  (c) a reader who trusts the design will mis-reason about which compiler produces a shipped binary. The
  release matrix sets no `CARGO_BUILD_TARGET`, so each runner builds natively and no extra rustup targets are
  needed — the bump is safe there, just unacknowledged.
- **Recommendation:** Correct the Context paragraph in `design.md` and add the extension build/release path to
  `proposal.md`'s Impact. Keep "do not touch those workflows" as a non-goal if you like, but on the real
  reason (the redundant `Setup Rust` step is a separate cleanup), not on a false premise.
- **Fix:** Confirmed against `dtolnay/rust-toolchain`'s `action.yml` (its only selector is `rustup default`).
  Rewrote the `design.md` Context paragraph to say the pin outranks the default and that the released VSIX's
  `nx-lsp` is built by the pinned compiler; the non-goal now reads "removing the redundant `Setup Rust` step".
  Added the two extension workflows to `proposal.md`'s Impact list.
- **Verification:** Verified. `design.md`'s Context now states that `dtolnay/rust-toolchain@stable` only runs
  `rustup default stable`, that a `rust-toolchain.toml` above the working directory outranks the default, and
  that the released VSIX's `nx-lsp` is therefore built by the pinned compiler; the non-goal is restated as
  "Removing the redundant `Setup Rust` step". `proposal.md`'s Impact gains the two extension workflows and the
  consequence that the next extension release ships an `nx-lsp` built by 1.98.1. Re-confirmed the underlying
  mechanism independently: the action's `action.yml` has only `rustup toolchain install` and `rustup default`
  and never exports `RUSTUP_TOOLCHAIN`, and `src/vscode/scripts/build-lsp.mjs` still spawns cargo with
  `cwd: repoRoot`.

### ✅ Verified - RF2 The change fixes the stale pin but not the reason it went stale, even though the repository already automates the exactly analogous .NET pin

- **Severity:** Medium
- **Evidence:** `proposal.md`'s Why is that the pin "was simply not kept current" for seven releases and ten
  months. `design.md:29-33` accepts that "someone has to bump it" and offers one trigger: a human noticing
  rust-analyzer's version-mismatch toast. Meanwhile `.github/renovate.json:17-21` already carries a
  `packageRule` grouping `dotnet-sdk` / `mcr.microsoft.com/dotnet/sdk` updates for `global.json` and the
  Dockerfile — the same shape of problem (an exact toolchain pin in a checked-in file), already solved by a
  bot. `rust-toolchain.toml` has no equivalent, and Renovate's stock managers do not cover it, so the file has
  no automated bump path at all. The design never weighs this option.
- **Recommendation:** Add a Renovate `customManagers` regex entry for the `channel = "x.y.z"` line in
  `rust-toolchain.toml` (datasource `github-releases`, `rust-lang/rust`), grouped like the dotnet rule. If
  that is out of scope here, record it as a follow-up change rather than leaving the root cause to the next
  person who notices a toast.
- **Fix:** Added a `customManagers` regex entry to `.github/renovate.json` (`managerFilePatterns` on
  `rust-toolchain.toml`, `channel = "x.y.z"` capture, datasource `github-releases` for `rust-lang/rust`,
  semver versioning). Verified the two regexes match the current file under JavaScript regex semantics and
  that `renovate-config-validator` accepts the config. Recorded the decision in `design.md` and the change in
  `proposal.md` and `tasks.md` (task 5.1). Not grouped with the dotnet rule, since a Rust bump and a .NET bump
  should be reviewed separately.
- **Verification:** Verified functionally, not just structurally. Ran the committed config's own patterns
  under JavaScript regex semantics: `/^rust-toolchain\.toml$/` matches the path, and the `matchStrings` entry
  captures `currentValue` = `1.98.1` from the live file. Confirmed the datasource actually resolves —
  `rust-lang/rust` publishes GitHub Releases tagged bare (`1.98.1`, `1.98.0`, `1.97.1`, …), so
  `github-releases` + `semver` will find successors rather than silently matching nothing, which is the usual
  way a custom manager fails. `npx --package renovate renovate-config-validator .github/renovate.json` reports
  "Config validated successfully". The decision is recorded in `design.md` ("Let Renovate propose the next
  bump"), in `proposal.md`'s What Changes and Impact, and as task 5.1. Not grouping it with the dotnet rule is
  the right call — the two need separate CI runs.

### ✅ Verified - RF3 The deferred clippy cleanup has no follow-up change, and nothing gates clippy in CI, so the "leave the repository green" goal is neither met nor protected

- **Severity:** Medium
- **Evidence:** `design.md:87-92` states the bump "left the rest for a separate change." No such change exists
  — `openspec list --json` shows only `update-rust-toolchain`, `infer-unannotated-return-types`, and
  `type-conditionals-without-else`. The deferred set is not cosmetic and is still present under 1.98.1:
  `crates/nx-value/src/lib.rs:309` and `:519` are deny-level `approx_constant` **errors**, `crates/nx-ffi/src/lib.rs`
  has 16 deny-level `not_unsafe_ptr_arg_deref` errors, and `crates/nx-syntax/benches/parse_benchmark.rs:1`
  fails to compile outright (`error[E0432]: unresolved import criterion` — the dev-dependency is not
  declared), which means `cargo clippy --all-targets` and `cargo bench` cannot succeed on this workspace.
  Separately, the CI `rust` job (`.github/workflows/build.yml:216-235`) runs only `cargo fmt --all --check`,
  `cargo test --workspace`, `pnpm -r build` and `pnpm -r test` — no clippy — which is precisely why 140
  diagnostics accumulated unnoticed. Task 3.2's method (diff clippy output between 1.98.1 and 1.91.1) is also
  no longer repeatable, since task 4.2 uninstalled 1.91.1; the two-toolchain baseline exists only in prose.
- **Recommendation:** Open the follow-up change now (fix `approx_constant`, audit the `nx-ffi` raw-pointer
  signatures, declare the `criterion` dev-dependency) and add `cargo clippy --workspace --all-targets` to the
  CI `rust` job once it is clean, so this bump's cleanup is the last one done by hand.
- **Fix:** Created the follow-up change `fix-clippy-diagnostics` with proposal, design, and tasks
  (`skip_specs: true`; `openspec validate` passes). Its tasks cover the five `approx_constant` sites, the
  sixteen `nx-ffi` entry points (design decision: mark them `unsafe` with `# Safety` sections rather than allow
  the lint), the undeclared `criterion` dev-dependency (design decision: declare it rather than delete the
  bench), the warn-level lints crate by crate, and a final `cargo clippy --workspace --all-targets -D warnings`
  step in the CI `rust` job. Nothing from it is implemented yet. `design.md` in this change now names the
  follow-up and notes that `rustup toolchain install 1.91.1` recreates the lost baseline.
- **Verification:** Verified. `openspec/changes/fix-clippy-diagnostics/` exists with proposal, design and
  tasks and `skip_specs: true`; `openspec validate fix-clippy-diagnostics` passes. Its scope was re-checked
  against live clippy output rather than taken on trust: the 16 `not_unsafe_ptr_arg_deref` sites in
  `crates/nx-ffi/src/lib.rs` and the `criterion` E0432 in `crates/nx-syntax/benches/parse_benchmark.rs` are
  confirmed, and the five `approx_constant` literals sit in exactly the three crates task 1.1 names
  (`crates/nx-cli/src/format.rs:314`, `crates/nx-hir/src/ast/expr.rs:387-388`,
  `crates/nx-value/src/lib.rs:309` and `:519`). Task 3.1 carries the CI gate this finding asked for. Nothing
  is implemented yet, which matches the recommendation — the point was that the deferral needed a home, and
  now it has one that `design.md` names, along with the note that `rustup toolchain install 1.91.1` recreates
  the lost baseline.

### ✅ Verified - RF4 The new prerequisite sentence is a 130-character line duplicated verbatim in two files, breaking the wrap both files otherwise keep

- **Severity:** Low
- **Evidence:** [README.md:217](README.md#L217) and
  [docs/src/content/docs/tutorials/getting-started.md:9](docs/src/content/docs/tutorials/getting-started.md#L9)
  are byte-identical at 130 characters. Before this change `README.md` had exactly one line over 110
  characters (line 103, at 113); this adds the longest line in the file. The change's own artifacts
  (`proposal.md`, `design.md`) wrap at 100.
- **Recommendation:** Wrap both to ~100 columns. The wording itself is right — it names no version, so it
  cannot go stale the way "Rust 1.75+" did.
- **Fix:** Wrapped the `README.md` bullet at 100 columns as a two-line list item. Left
  `getting-started.md` on one line: that file does not wrap at all (its prose lines run to 196 and 208
  characters), so a wrapped bullet there would be the exception, not the norm.
- **Verification:** Verified, and the pushback on the second file is accepted. `README.md:217-218` is now a
  wrapped two-line list item (valid markdown lazy continuation; it renders as one bullet), and the file has no
  line over 110 characters again — only the pre-existing 113 at line 103. The finding was wrong about
  `getting-started.md`: that file wraps nothing, with prose lines at 196, 208 and 159 characters, so leaving
  its bullet at 130 is the consistent choice there, not an exception.

### ✅ Verified - RF5 `rust-version = "1.85"` is now thirteen releases below the pin and nothing verifies it — the same silent-drift failure this change exists to fix, one field over

- **Severity:** Low
- **Evidence:** `Cargo.toml:24` declares the MSRV; `design.md:20-22` correctly makes raising it a non-goal,
  but no job, task or tool ever builds the workspace with 1.85, and task 4.2 removed every toolchain below
  1.98.1 from the machine, so the claim is now unfalsifiable locally. The `crates/*` members do not set
  `publish = false` (only the two `bindings/*/native` crates do), so the declaration is nominally a promise to
  consumers. The two edits in this change happen to be MSRV-safe, but nothing establishes that the rest of the
  workspace still is.
- **Recommendation:** Decide which it is and say so: either add a `cargo +1.85 check --workspace` CI job (or
  `cargo-msrv`), or drop `rust-version` if nothing consumes it. Leaving an unverified version claim in place
  is how the README ended up advertising 1.75.
- **Fix:** Neither of the two options this finding offered. Investigation showed the field was not merely
  unverified but inert: MSRV-aware resolution needs `resolver = "3"` or an `incompatible-rust-versions`
  setting, and the workspace had `resolver = "2"` and no `.cargo/config.toml`, so cargo never read
  `rust-version` at all — and its fallback effect, a legible error under an older compiler, cannot fire under
  an exact pin. So rather than verify the claim or delete it, `Cargo.toml` now sets `rust-version = "1.98.1"`
  and `resolver = "3"`, which makes the MSRV an input to dependency resolution. That matters here specifically
  because `Cargo.lock` is gitignored: cargo will now hold a dependency back rather than break inside one that
  raised its own minimum. A second Renovate custom manager tracks the `rust-version` line, grouped with the
  pin under the `rust` dep name, so the two cannot drift apart — with resolver 3 on, an MSRV lagging the pin
  would silently hold dependencies back. Recorded in `proposal.md` (What Changes, Impact), `design.md` (a new
  decision, a new risk entry, and the removed non-goal), and `tasks.md` section 6.
- **Verification:** Verified, with one caveat stated plainly: unlike RF1-RF4, this finding's fix and its
  verification were both done by the reviewing agent, so it does not have the independence the rest of this
  report has. Evidence — resolved `Cargo.lock` from scratch under `resolver = "2"` and again under
  `resolver = "3"`; the lockfile and `cargo tree --workspace` output are byte-identical, so the switch changes
  no dependency version today. Confirmed the guard is live rather than silently ignored by resolving with
  `rust-version` temporarily set to 1.85: cargo then reports eleven "requires Rust" holdbacks and pulls the
  `icu_*` family back to 2.1.x, `idna_adapter` to 1.2.1, and the Node binding's `napi-build` to 2.2.4. That
  measurement also corrected a wrong figure in the first draft of the design, which had cited `clap` from a
  run at a different MSRV. Restored 1.98.1 and re-confirmed the graph matches the resolver-3 baseline.
  `cargo fmt --all --check`, `cargo test --workspace`, `pnpm -r build` and `pnpm -r test` all pass on top of
  the change. Both Renovate regexes match their own file, the `Cargo.toml` pattern correctly does not match
  member manifests such as `crates/nx-cli/Cargo.toml`, and `renovate-config-validator` accepts the file.
  `openspec validate update-rust-toolchain` passes.

### ✅ Verified - RF6 README's test-count comment is stale in the section this change edits

- **Severity:** Low
- **Evidence:** [README.md:227](README.md#L227) reads `# Run all tests (197 tests)`, ten lines below the
  prerequisite this change rewrote and inside the same `## Development` block. The workspace run performed for
  this review passes well over a thousand tests across the crates.
- **Recommendation:** Drop the parenthetical rather than update it — a count in prose goes stale the same way
  a version number does, which is the lesson the prerequisite rewrite already applied.
- **Fix:** The comment now reads `# Run all tests`. Recorded as task 2.3 in `tasks.md`.
- **Verification:** Verified. `README.md:228` now reads `# Run all tests`, and task 2.3 records it.

## Questions

- None outstanding. The lockfile question raised in the first pass was answered by committing it (see the
  2026-09-14 06:40 verification below).

## Verification (2026-09-14 05:12)

Five findings were marked fixed (RF1-RF4, RF6); all five verify. The Rust diff was byte-identical to what the
original review ran against (`git diff HEAD -- crates/` unchanged), so the earlier fmt/test/build/clippy
results still stood and were not re-run; everything the fixes touched — `.github/renovate.json`, `README.md`,
the two artifact files, and the new `fix-clippy-diagnostics` change — was re-checked directly. No new
findings. RF5 was left open at this point, pending a maintainer decision.

## Verification (2026-09-14 06:05)

RF5 closed. The maintainer decided to resolve it inside this change rather than defer it, and the
investigation changed the answer: the finding had framed the choice as "verify the 1.85 claim or delete the
field", but the field turned out to be inert under `resolver = "2"`, so a third option — make it load-bearing
by switching to resolver 3 and setting it to the pin — was both cheaper and more useful. See RF5's Fix and
Verification notes for the evidence, including the caveat that this one finding was fixed and verified by the
same agent. The full suite (`cargo fmt --all --check`, `cargo test --workspace`, `pnpm -r build`,
`pnpm -r test`) was re-run against the resolver change and passes. No new findings.

## Verification (2026-09-14 06:40)

The remaining question was closed rather than deferred: `Cargo.lock` is now tracked. Cargo's current guidance
no longer splits on binary-versus-library — `cargo new` tracks the lockfile by default — and the one argument
against tracking, that a lockfile does not reach consumers of a published package, does not apply to a
workspace that publishes nothing and ships a deployed wasm module and a released `nx-lsp` binary.

Checked: `.gitignore` no longer ignores it and `git ls-files Cargo.lock` lists it; the playground Dockerfile
copies it and the comment explaining its absence is gone; `deploy-playground.yml` gained it as a path trigger,
and `vscode-extension.yml` turns out to have listed it as one all along, a trigger that could never fire.
`cargo test --workspace --locked` passes, and the guard was shown to bite: editing a dependency version in
`Cargo.toml` without regenerating the lockfile makes it fail with "cannot update the lock file ... because
--locked was passed". Renovate `lockFileMaintenance` is enabled so the lockfile does not simply freeze, and
`renovate-config-validator` still accepts the config. `cargo fmt --all --check`, `pnpm -r build` and
`pnpm -r test` pass.

This also corrected part of RF5's own reasoning, which is recorded in `design.md` rather than quietly
dropped: the resolver-3 argument had leaned on "every build resolves the graph afresh", which committing the
lockfile makes false. Resolver 3 still earns its place, but on narrower grounds — it now guards the moments
when resolution actually runs, above all Renovate's unattended lockfile refresh.

## Summary

The implementation was correct and complete against its own tasks from the first pass: the pin, the two
documentation fixes, and the three new-lint fixes are all present and verified, every task's stated check
re-runs green, and both code edits are behavior-preserving. The gaps were in the surrounding reasoning rather
than the code, and all of them are now closed. `design.md` no longer rests on a false premise about the VS
Code extension workflows, and the proposal records that the bump reaches the released VSIX (RF1). The root
cause — a pin nothing watched — is now automated rather than merely re-fixed, and the Renovate manager was
checked against live release data, not just a schema (RF2). The clippy deferral has a validated follow-up
change whose scope matches the actual diagnostics, including the CI gate that stops the backlog from
re-forming (RF3).

All six findings are now closed. RF5 ended up widening the change slightly — the workspace moves to
`resolver = "3"` with `rust-version` tied to the pin — but the verification shows that costs nothing today:
resolved from scratch, the dependency graph is identical under both resolvers, and the full test, build and
wasm suites pass on top of it. What it buys is that the two settings this change exists to keep honest, the
pin and the MSRV, are now both watched by Renovate and both do real work.

The lockfile question from the first pass was answered by tracking `Cargo.lock`, with `--locked` in CI to keep
it honest and the weekly lockfile maintenance `config:best-practices` already runs to keep it fresh — so "same
input, same output" now holds for the whole build rather than just the compiler.

The later round (RF7-RF10) found no build break but did find that three of the claims this report and the
artifacts made about the Renovate configuration were wrong: the group rule was unscoped, the
`lockFileMaintenance` block was redundant with the preset, and the Dockerfile comment credited the image with
a guarantee that lives in CI. All four are fixed and await verification by the agent that raised them.

## New Findings Discovered During 2026-09-14 01:44 Review

**Pass scope:** the currently unstaged working-tree diff only — `.github/renovate.json`,
`.github/workflows/build.yml`, `.github/workflows/deploy-playground.yml`, `.gitignore`, `Cargo.toml`,
`sites/playground/Dockerfile` — read against the staged `Cargo.lock`, the full `sites/playground/Dockerfile`,
`.dockerignore`, the workflows that build Rust, and `openspec/changes/update-rust-toolchain/{proposal,design,tasks}.md`.
The Rust sources are untouched in this pass, so the earlier fmt/test/clippy results were not re-run in full.

**Re-verified in this pass (all green):** `cargo metadata --locked` and `cargo check --workspace --locked` both
succeed, so the staged lockfile is in sync with the resolver-3 manifest; `resolver = "3"` under `edition = "2021"`
produces no cargo warning; all fourteen workspace members carry `rust-version.workspace = true`, so the new MSRV
actually reaches them; both Renovate regexes still capture `1.98.1` from their own file and the `Cargo.toml`
pattern still does not match member manifests; `renovate-config-validator` accepts the file; `.dockerignore`
does not exclude `Cargo.lock`, so the new `COPY` line resolves; `Cargo.lock` is the only lockfile the removed
`.gitignore` entry was hiding; and bumping `rust-version` alone does **not** invalidate the lockfile, so
Renovate's grouped MSRV bump will not spuriously fail the new `--locked` gate.

**Status:** the earlier "nothing is left open, ready to archive" conclusion is superseded — four findings below.
Nothing found here is a correctness break in the build; all four are in the Renovate configuration and in the
accuracy of what the artifacts claim.

### ✅ Verified - RF7 The new `matchDepNames: ["rust"]` group rule is unscoped, so it also captures the `rust` Docker base image and will file its digest bumps under "Rust toolchain pin and MSRV"

- **Severity:** Medium
- **Evidence:** [.github/renovate.json:34-37](.github/renovate.json#L34-L37) adds
  `{"matchDepNames": ["rust"], "groupName": "Rust toolchain pin and MSRV"}` with no datasource or manager
  constraint. Renovate's Dockerfile manager names a dep by the image without its tag — in renovate 44.83.2,
  `getDep` splits on `:` and sets `depName = packageName = depTagSplit.join(":")`
  (`modules/manager/dockerfile/extract.js:56-67`) — so
  [sites/playground/Dockerfile:12](sites/playground/Dockerfile#L12), `FROM rust:1-bookworm`, extracts as
  `depName: "rust"`, `currentValue: "1-bookworm"`, datasource `docker`. It matches the new rule exactly.
  This is not dormant: `config:best-practices` extends `docker:pinDigests`
  (`config/presets/internal/config.preset.js:3-14`, `docker.preset.js:25-30`), which sets `pinDigests: true`
  for every `docker` dep, so Renovate will pin that image to a digest and then propose a digest bump on every
  rebuild of the base image — each one titled and grouped as a Rust toolchain pin update. The two are
  unrelated: the image only supplies rustup, which then honors `rust-toolchain.toml`, so the base image's
  digest says nothing about which compiler builds the code. The file's own established pattern avoids this —
  the dotnet rule at [.github/renovate.json:47-51](.github/renovate.json#L47-L51) pairs `matchDepNames` with
  `matchDatasources` — and the new rule is the only `matchDepNames` rule in the file without that scoping.
  Note that `renovate-config-validator` cannot catch this (the config is schema-valid), and it reported
  "Validating `.github/renovate.json` as global config", so task 6.5's validation evidence is weaker than it
  reads.
- **Recommendation:** Scope the rule to the two custom managers, e.g. add `"matchManagers": ["custom.regex"]`
  (or `"matchDatasources": ["github-releases"]`) alongside `matchDepNames`, matching how the dotnet rule is
  written.
- **Fix:** Confirmed against the renovate 44.83.2 package before changing anything: `dockerfile/extract.js`
  does set `depName = depTagSplit.join(":")`, so `FROM rust:1-bookworm` at
  `sites/playground/Dockerfile:12` extracts as `depName: "rust"`, and `docker:pinDigests` — reached from
  `config:best-practices` — does set `pinDigests: true` for `matchDatasources: ["docker"]`. The finding is
  correct and the rule would have captured it. Added `"matchDatasources": ["github-releases"]` alongside
  `matchDepNames`, which is how the dotnet rule in the same file is written; the two custom managers both
  declare that datasource, and the Docker dep declares `docker`. Verified by evaluating the rule against both
  shapes: it still captures the pin and the MSRV, and no longer captures the base image.

### ✅ Verified - RF8 `lockFileMaintenance` was already enabled by `config:best-practices`, so the new block is dead config and task 7.5 and design.md record a capability the repository already had

- **Severity:** Low
- **Evidence:** [.github/renovate.json:5-7](.github/renovate.json#L5-L7) adds
  `"lockFileMaintenance": {"enabled": true}`. But `extends: ["config:best-practices"]` already resolves to
  `:maintainLockFilesWeekly` (`config/presets/internal/config.preset.js:3-14`), which is
  `lockFileMaintenance: {enabled: true, extends: ["schedule:weekly"]}`
  (`config/presets/internal/default.preset.js:282-288`). `lockFileMaintenance` is declared `mergeable: true`
  (`config/options/index.js:2519-2535`) and `mergeChildConfig` recurses into mergeable objects
  (`config/utils.js:19-27`), so the repo block merges in, the weekly schedule survives, and the net effect is
  nil. Consequences: a config line that looks load-bearing but is not; task 7.5 ("Enable Renovate
  `lockFileMaintenance`") describes work that was already done by the preset; and design.md:85 and the risk
  entry at design.md:125-126 read as if this change supplied the refresh mechanism. The mechanism was in fact
  already refreshing the four tracked pnpm lockfiles weekly — `pnpm-lock.yaml`, `docs/pnpm-lock.yaml`,
  `src/vscode/pnpm-lock.yaml`, `crates/nx-syntax/pnpm-lock.yaml` — which is worth stating, since the artifacts
  discuss `lockFileMaintenance` as if it were Cargo-only.
- **Recommendation:** Delete the block. Restate task 7.5 and the design decision as what is actually true —
  `config:best-practices` already runs weekly lock file maintenance, so the newly tracked `Cargo.lock` is
  covered with no configuration change — and verify that claim, since the change now rests on it rather than
  on a line in the file.
- **Fix:** Verified the claim in the shipped package rather than taking it on trust —
  `config.preset.js` shows `best-practices` extending `:maintainLockFilesWeekly`, and `default.preset.js`
  shows that preset setting `lockFileMaintenance: {enabled: true, extends: ["schedule:weekly"]}`. The block
  was redundant, so it is deleted. Task 7.5 now reads as verifying that the committed lockfile is already
  covered rather than as enabling anything, `proposal.md` says no maintenance setting is added and notes the
  newly tracked `Cargo.lock` joins the pnpm lockfiles the preset has been refreshing all along, and the
  design's decision and risk entry are reworded the same way.

### ✅ Verified - RF9 Renovate regenerates `Cargo.lock` under a Rust toolchain nothing pins, in a repository whose whole premise is that the toolchain is pinned

- **Severity:** Low
- **Evidence:** design.md:59-60 names Renovate's unattended `lockFileMaintenance` as the main remaining
  justification for resolver 3 — "above all Renovate's scheduled `lockFileMaintenance`, which refreshes the
  lockfile unattended". Renovate does that by running `cargo update --workspace` under a toolchain it selects
  from `config.constraints.rust` (`modules/manager/cargo/artifacts.js:12-21` and `:61`), falling back to the
  newest Rust in its image when the key is absent. `.github/renovate.json` sets no `constraints`. So the one
  scheduled, unattended job that resolves this workspace's dependency graph is the one place the pin does not
  reach. It works today — resolver 3 only needs cargo 1.84+, and MSRV-aware resolution reads `rust-version`
  from the manifest rather than the running rustc, so the result is deterministic regardless of Renovate's
  compiler — which is why this is Low and not a live break. It is still unverified: no `lockFileMaintenance`
  run has yet produced a `Cargo.lock` for this repository, and `constraints.rust` is the one setting that
  would make the claim hold by construction rather than by luck.
- **Recommendation:** Either add `"constraints": {"rust": "1.98.1"}` and keep it equal to the pin the same way
  the MSRV is kept equal (a third `matchStrings` entry on a custom manager over `.github/renovate.json`
  itself), or record in design.md why running Renovate's resolution on an unpinned compiler is acceptable
  given that `rust-version` makes the outcome independent of it. Do not leave it unstated — it is the same
  "a version nothing watches" shape this change exists to remove.
- **Fix:** Taken as the second of the two options, and now stated rather than implied. Confirmed the
  mechanism first: `cargo/artifacts.js` does pass `config.constraints?.rust` when running `cargo update`, and
  this config sets no `constraints`. `design.md` now records why that is acceptable here rather than leaving
  it unsaid — MSRV-aware resolution reads `rust-version` from the manifest rather than from the running
  compiler, cargo preserves an existing lockfile's format version rather than upgrading it, and every refresh
  arrives as a pull request that `build.yml` compiles and tests with `--locked` before it can merge, so a bad
  one fails visibly. Pinning `constraints.rust` was rejected on the ground the finding itself raises: it would
  add a third copy of the version, and the only way to keep it in step is a regex manager over
  `.github/renovate.json` itself, which is more likely to rot than what it guards.

### ✅ Verified - RF10 The Dockerfile's new comment promises determinism that is enforced in exactly one job, and not in either place that produces a shipped artifact

- **Severity:** Low
- **Evidence:** [sites/playground/Dockerfile:36-38](sites/playground/Dockerfile#L36-L38) now reads "`Cargo.lock`
  is tracked, so this image builds the dependency versions the validate job that precedes a deploy resolved,
  rather than whatever is newest when the image happens to be built." That is true only while `Cargo.lock` and
  `Cargo.toml` agree; if they ever diverge, cargo silently re-resolves and the image gets exactly "whatever is
  newest", with no signal. Nothing on the path the comment describes checks that: the image's cargo run comes
  from `bindings/wasm/scripts/build-wasm.mjs:25-43` with no `--locked`, and neither does
  [deploy-playground.yml:63-66](.github/workflows/deploy-playground.yml#L63-L66) (`pnpm -r build` / `pnpm -r
  test`) or the extension release build (`src/vscode/scripts/build-lsp.mjs:13`, `cargo build -p nx-lsp`). The
  only gate is [build.yml:233](.github/workflows/build.yml#L233), in a different workflow. In practice the
  exposure is small — `build.yml` has no path filters and runs on every push and pull request, so a stale
  lockfile cannot reach `main` — but the two artifacts that actually ship, the wasm module inside the image
  and the released `nx-lsp`, are precisely the ones built without the flag, and the comment in the Dockerfile
  is where a reader will look for that guarantee.
- **Recommendation:** Pick one and make the comment match it. Either pass `--locked` where the shipped
  artifacts are built (`build-wasm.mjs` and `build-lsp.mjs`) — noting the cost, that a local `pnpm build` then
  fails after a `Cargo.toml` edit until cargo is run — or narrow the Dockerfile comment to say the guarantee
  comes from the `--locked` gate in CI, not from the image, and add `Cargo.lock` reasoning to design.md's
  "that one gate is enough" sentence, which currently covers staleness detection but not artifact provenance.
- **Fix:** Narrowed the comment rather than adding `--locked` to the shipped-artifact builds, because
  those scripts are the ones developers run locally and the flag would make a build fail after every
  `Cargo.toml` edit until cargo was run by hand. `sites/playground/Dockerfile` now says the lockfile fixes the
  versions the commit names, that the cargo runs there do not pass `--locked` and why, and that what keeps the
  lockfile honest is the gate in `build.yml`, which has no path filters and so runs on every push and pull
  request. `design.md`'s "that one gate is enough" sentence is expanded to cover artifact provenance, not just
  staleness detection, and names `build-wasm.mjs` and `build-lsp.mjs` as the deliberate exceptions. Recorded
  as task 7.7.

- **Verification:** Verified. [sites/playground/Dockerfile:36-41](sites/playground/Dockerfile#L36-L41) no
  longer attributes the guarantee to the image: it says the lockfile fixes the versions the commit names, that
  the cargo runs there deliberately omit `--locked` because the same scripts serve local development, and that
  what keeps the lockfile honest is the `--locked` gate in `build.yml`, which has no path filter and so runs
  on every push and pull request. `design.md:93-101` expands the "that one gate is enough" sentence to cover
  artifact provenance rather than only staleness detection, and names `build-wasm.mjs` and `build-lsp.mjs` as
  deliberate exceptions with the local-development reason. Task 7.7 records the work. The new comment wraps at
  100 columns, matching the longest existing comment line in the file. The alternative the finding offered —
  `--locked` in the two build scripts — was weighed and declined on a stated cost, which is a legitimate
  resolution of a finding that offered both.

## Questions

- RF9's choice has been made: `constraints.rust` stays unset and `design.md` now records why. See RF9's Fix
  note. Nothing else outstanding.

## Summary (2026-09-14 01:44 pass)

The build-side implementation holds up under re-examination. The lockfile is genuinely in sync with the
resolver-3 manifest (`cargo check --workspace --locked` passes), every workspace member inherits the new MSRV
so the field is not decorative, `.dockerignore` does not defeat the new `COPY`, and an MSRV-only Renovate bump
provably does not trip the new `--locked` gate — a failure mode that would have made the grouped Renovate PR
red every month.

All four new findings are in the Renovate configuration and in the accuracy of the change's own record. RF7 is
the one with real operational cost: an unscoped `matchDepNames: ["rust"]` silently swallows the `rust`
Docker base image, and because `config:best-practices` turns on `docker:pinDigests`, that misfiling is
recurring rather than hypothetical. RF8 and RF9 are the same shape as each other — the change asserts things
about Renovate that were checked against the schema rather than against Renovate's behavior, and one of them
turns out to have been already true before the change while the other is not yet true at all. RF10 is a
comment that promises more than the pipeline delivers.

## New Findings Discovered During 2026-09-14 02:35 Verification

### ✅ Verified - RF11 `proposal.md` still credits the validate job with resolving the versions the image builds, the same misattribution RF10 just removed from the Dockerfile

- **Severity:** Low
- **Evidence:** `proposal.md:59-61` reads "`sites/playground/Dockerfile` copies it so the deployed image builds
  the versions the validate job resolved". Under a tracked lockfile the validate job resolves nothing: it
  builds against the versions the commit names, which is exactly what the Dockerfile comment now says after
  RF10's fix, and the validate job's own cargo runs
  ([deploy-playground.yml:63-66](.github/workflows/deploy-playground.yml#L63-L66)) do not pass `--locked`
  either, so they establish nothing the image could inherit. The sentence is a leftover of the pre-RF10
  wording — the proposal and the file it describes now tell different stories about where the image's
  dependency versions come from, and the proposal is the one a reader reaches first.
- **Recommendation:** Reword to match the Dockerfile: the image builds the versions the commit names. Drop the
  reference to the validate job resolving them.
- **Fix:** `proposal.md:59-62` now reads "`sites/playground/Dockerfile` copies it so the deployed image builds
  the versions the commit names", matching the wording RF10 put in the Dockerfile; the reference to the
  validate job resolving anything is gone. Confirmed by grep that this was the last occurrence in the change —
  the only other hits are this report's own quotation of the pre-RF10 Dockerfile comment and the finding text
  itself. The bullet is rewrapped at 100 columns and `openspec validate update-rust-toolchain` passes.

- **Verification:** Verified. `proposal.md:59-62` now reads "`sites/playground/Dockerfile` copies it so the
  deployed image builds the versions the commit names", which is the same attribution the Dockerfile comment
  makes, and the wrapped bullet runs 97-98 columns, inside the file's 100. Checked independently that nothing
  else in the change still describes the lockfile-less mechanism: `grep "validate job"` over `proposal.md`,
  `design.md`, `tasks.md` and `sites/playground/Dockerfile` returns nothing, and the only surviving
  "resolving afresh" phrase, `design.md:58`, is the deliberate contrast with the old behavior rather than a
  claim about the new one. `openspec validate update-rust-toolchain` passes.

## Verification (2026-09-14 02:35)

All four findings from the 01:44 pass verify. Each fix was checked against the mechanism it claims rather than
against its own description: RF7 against Renovate's `DatasourcesMatcher` and the Dockerfile manager's
datasource assignment, RF8 against the shipped `config:best-practices` and `:maintainLockFilesWeekly` presets,
RF9 against `build.yml`'s absent path filter and the committed lockfile's format version, RF10 against the
rewritten comment and the design sentence it points to. `renovate-config-validator` still accepts the config
and the Renovate group now resolves to exactly the two custom-manager deps.

Two of the four were resolved by argument rather than by code — RF9 keeps `constraints.rust` unset and records
why, RF10 keeps `--locked` off the two build scripts and records why. Both are legitimate: each finding
offered the alternative and the artifacts now carry the reasoning instead of leaving it implicit. Nothing
functional changed in this round beyond the Renovate `matchDatasources` scoping, so the 01:44 pass's
`cargo check --workspace --locked` result still stands.

One new finding, RF11, at Low: `proposal.md` kept the pre-RF10 sentence attributing the image's dependency
versions to the validate job. It is the last place in the change that describes the old, lockfile-less
mechanism.
