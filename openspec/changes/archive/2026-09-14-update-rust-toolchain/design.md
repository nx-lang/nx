## Context

See proposal.md - Why. The toolchain is selected in one file, `rust-toolchain.toml`, and every
build path defers to it: rustup reads it locally, the `setup-rust-node` composite action relies on
it (only adding the wasm target explicitly for a legible failure), and the playground Dockerfile
copies it into a `rust:1-bookworm` image where rustup installs whatever it names. The workspace is
edition 2021 and, before this change, declared `rust-version = "1.85"` with `resolver = "2"`.

The two VS Code extension workflows run `dtolnay/rust-toolchain@stable` before building, but that
action only runs `rustup default stable`; it does not set `RUSTUP_TOOLCHAIN`. A `rust-toolchain.toml`
found above the working directory outranks the default, and `src/vscode/scripts/build-lsp.mjs`
spawns cargo at the repository root, so those jobs build `nx-lsp` with the pin as well. The bump
therefore also changes the compiler behind the released VSIX. Their `Setup Rust` step is redundant
with the pin and is a separate cleanup.

The repository already automates the analogous .NET SDK pin: `.github/renovate.json` groups
`global.json` and Dockerfile updates for `dotnet-sdk`. Renovate has no stock manager for
`rust-toolchain.toml`, so until this change the Rust pin had no automated bump path.

## Goals / Non-Goals

**Goals:**
- Move the pin to current stable with no behavior change to the compiler outputs the repository
  ships (CLI, Node addon, wasm module).
- Keep the pin exact, not a moving channel, so CI, the Docker image, and developer machines agree.
- Leave the repository green under the new compiler and clippy.

**Non-Goals:**
- Moving to edition 2024. That is a code change with its own review surface.
- Removing the redundant `Setup Rust` step from the VS Code extension workflows (see Context).
- Adopting any new lints or language features.

## Decisions

**Pin 1.98.1 exactly, not `stable`.** A moving channel would let CI, the Docker build, and a
developer's machine drift apart within the same week, and the wasm module's bytes depend on the
compiler. The cost of an exact pin is that someone has to bump it; this change is that bump.

**Let Renovate propose the next bump.** The pin went stale because nothing watched it. A Renovate
`customManagers` regex entry matches the `channel = "x.y.z"` line in `rust-toolchain.toml` against
`rust-lang/rust` GitHub releases, so each Rust release arrives as a pull request that runs the
same CI gate this change ran by hand. The alternative, relying on someone noticing the
rust-analyzer mismatch notice, is what produced the ten-month lag. A second manager moves
`rust-version` with it under the same dep name, so the two stay equal in one pull request — with
resolver 3 on, an MSRV that lagged the pin would quietly hold dependencies back.

**Tie the MSRV to the pin and switch to resolver 3.** `rust-version = "1.85"` was inert.
MSRV-aware dependency resolution needs `resolver = "3"` or an `incompatible-rust-versions =
"fallback"` setting, and the workspace had `resolver = "2"` and no `.cargo/config.toml`; the field's
only other effect, a legible error under an older compiler, cannot fire either, because the pin
installs 1.98.1 for whoever builds. It was a claim with no mechanism behind it, which is why it sat
thirteen releases back without anyone noticing — the same failure this change exists to fix, one
field over.

Resolver 3 gives it a mechanism: MSRV-aware resolution makes cargo fall back to the last version
compatible with `rust-version` instead of picking one that needs a newer compiler than the pin
provides. Committing `Cargo.lock` in this same change narrows where that applies — builds now reuse
a resolved graph rather than resolving afresh — but it does not remove the need for it. Resolution
still runs whenever the lockfile is regenerated: a new dependency, a `cargo update`, and above all
Renovate's scheduled `lockFileMaintenance`, which refreshes the lockfile unattended. An unattended
refresh is exactly where you want the guard, because nobody is watching it choose.

That refresh runs `cargo update` under a Rust that Renovate picks, not under the pin: Renovate reads
`config.constraints.rust`, and this repository sets no `constraints`. Leaving it unset is
deliberate. MSRV-aware resolution reads `rust-version` from the manifest rather than from the
running compiler, and cargo preserves an existing lockfile's format version rather than upgrading
it, so the refresh's output does not depend on which rustc Renovate ran; and every refresh arrives
as a pull request that `build.yml` compiles and tests with `--locked` before it can merge, so a bad
one fails visibly rather than landing. Pinning `constraints.rust` would buy determinism that is
already there at the cost of a third copy of the version to keep in step with the other two — and
the only way to keep it in step, a regex manager over `.github/renovate.json` itself, is more likely
to rot than the thing it guards.

The value has to equal the pin: measured on this workspace, leaving it at 1.85 with resolver 3 on
holds eleven crates back — the whole `icu_*` family at 2.1.x instead of 2.3.x, `idna_adapter` at
1.2.1, and the Node binding's `napi-build` at 2.2.4 instead of 2.4.2 — to satisfy a compiler nobody
uses. At 1.98.1 the resolved graph is byte-identical to what resolver 2 produced, so the switch is a
guard against a future break rather than a change to today's build.

Nothing in the workspace is published to crates.io, so the MSRV owes nothing to downstream
consumers. If that changes, lowering it is a deliberate decision to make then, backed by a check
that proves the lower number — not a stale value kept on the chance it might matter.

**Commit `Cargo.lock`.** The repository pinned the compiler exactly for determinism while leaving
the dependency graph to resolve fresh on every build, so two builds a week apart could differ in
everything but rustc. Cargo's own guidance no longer splits on binary-versus-library — `cargo new`
tracks the lockfile by default — and the one argument against it, that a lockfile does not affect
consumers of a published package, does not apply: nothing here is published. What the repository
does ship is a wasm module baked into a deployed site and an `nx-lsp` binary inside a released
VSIX, both of which should be reproducible from a commit. The `add-rust-ci-job` change already
flagged this as "worth doing, but a repository policy decision"; this is that decision.

Two consequences are handled here rather than left to be discovered. `cargo test --workspace` in CI
gains `--locked`, so a pull request that edits `Cargo.toml` without regenerating the lockfile fails
instead of silently re-resolving. That is the only `--locked` in the repository: the cargo runs that
produce the shipped artifacts — `build-wasm.mjs` for the playground module, `build-lsp.mjs` for the
VSIX — are the same scripts developers run locally, where `--locked` would make a build fail after
every `Cargo.toml` edit until cargo was run by hand. The gate does not need to be on those paths,
because `build.yml` has no path filters and runs on every push and pull request, so a lockfile out
of step with `Cargo.toml` cannot reach `main` for them to build from; the provenance of a shipped
artifact comes from that, not from the build script. The Dockerfile comment says so rather than
implying the image enforces it itself.

The second consequence is staleness in the other direction: Renovate only opens pull requests for
versions outside a declared range, so without a refresh the in-range and transitive dependencies
would freeze at whatever this commit resolved. No configuration is needed for it —
`config:best-practices` already extends `:maintainLockFilesWeekly`, which sets
`lockFileMaintenance` enabled on a weekly schedule, and has been refreshing the four tracked pnpm
lockfiles all along. Tracking `Cargo.lock` simply brings it under the same job.

**Do not skip 1.98.0 for 1.98.1 or wait for 1.99.** 1.98.1 is the current point release; 1.99
lands October 1. Taking the current point release is the normal choice and there is no reason to
wait.

**Fix the prerequisite docs in the same change.** Both say "Rust 1.75+", which was below the MSRV
the workspace declared even before this change and is irrelevant under an exact pin: rustup installs
the pinned toolchain on the first `cargo` command regardless of what else is installed. Saying
"rustup; the pinned toolchain installs itself" is accurate and never goes stale. The alternative,
naming 1.98.1 in prose, would repeat the mistake that produced the stale 1.75 text.

**Fix any new warnings rather than allow them.** Seven releases of rustc and clippy may add lints.
If `cargo clippy` or the build reports something new, the fix goes into this change so the bump
lands green. If a fix turns out to be non-trivial, it is split out and the lint is allowed with a
comment naming the follow-up; that is a judgment made during implementation, not decided here.

**Remove the stale local toolchains after verification.** rustup keeps every toolchain a pin has
ever named. 1.75, 1.75.0, and 1.80 are dead weight now; 1.91.1 joins them once the new pin passes
the full verification. They are re-downloadable, so removal is low risk.

## Risks / Trade-offs

- [New rustc or clippy lints fail the build] → Run fmt, clippy, and tests before committing; fix
  or narrowly allow with a follow-up note.
- [wasm32-wasip1 output changes size or behavior] → The pnpm workspace tests compare the wasm SDK's
  answers with the Node SDK's, so a behavioral divergence fails locally and in CI. A size change
  is expected and harmless.
- [rustup cannot download 1.98.1 from the sandboxed shell] → Retry the download outside the
  sandbox; the toolchain install is the only step needing network.
- [Playground redeploys on merge] → Intended; the `deploy-playground` workflow validates before
  deploying.
- [Resolver 3 changes which dependency versions are selected] → Resolved the workspace from scratch
  under both resolvers and diffed: `Cargo.lock` and `cargo tree` are identical, because the MSRV
  equals current stable. The full test, build and wasm suites were re-run on top of it.
- [A committed lockfile becomes a merge-conflict surface] → Real, and the usual cost of tracking
  one. Conflicts in `Cargo.lock` are resolved by regenerating it, not by hand-merging, and the
  repository has one maintainer today.
- [The lockfile freezes dependencies instead of drifting them] → The weekly `lockFileMaintenance`
  that `config:best-practices` already enables refreshes it, and `--locked` in CI proves each
  refresh still builds and tests before it merges.

## Findings during implementation

- Clippy was never clean: `cargo clippy --workspace --all-targets` reports about 140 diagnostics
  under 1.91.1 as well, including deny-level errors (`approx_constant` in `nx-hir` and `nx-value`,
  and a `criterion` bench in `nx-syntax` whose dependency is not declared). Clippy is not a CI
  gate, so this never surfaced. The bump fixed the three warnings that were new under 1.98.1
  (two `useless_conversion`, one `while_let_loop`) and left the rest for the
  `fix-clippy-diagnostics` change, which also adds clippy to the CI gate. The 1.91.1 baseline was
  produced before that toolchain was uninstalled; `rustup toolchain install 1.91.1` recreates it.
- Downloads through rustup failed with `InvalidCertificate(UnknownIssuer)` because `~/.bashrc`
  exports `SSL_CERT_DIR` pointing at the ASP.NET dev-cert trust directory alone, which hides the
  system CA bundle from rustup. `env -u SSL_CERT_DIR rustup ...` works. That is a machine issue,
  not a repository one.

