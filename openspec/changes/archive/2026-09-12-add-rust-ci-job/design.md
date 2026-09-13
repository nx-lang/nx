## Context

See proposal.md for motivation. The facts that shape the approach:

- `build.yml` has four jobs (`build` on a three-OS matrix, `package`, `smoke-test-package`,
  `editor-assets`) and none of them runs `cargo test`. The `build` job already invokes `cargo build
  -p nx-ffi` through `Stage-NxSdkNativeArtifact.ps1`, so the runners have rustup, and
  `rust-toolchain.toml` pins `1.91.1` with `rustfmt` and `clippy` as components.
- `deploy-playground.yml` is the one workflow that builds Rust with Node present. It relies on
  rustup honoring `rust-toolchain.toml`, caches with `Swatinem/rust-cache@v2`, installs the pnpm
  workspace with `pnpm install --frozen-lockfile`, and uses Node 24. It runs only on pushes to
  `main`, not on pull requests.
- `crates/nx-codegen/src/tests.rs` has two tool probes. `node_is_available` prints "skipping" and
  returns `false`; `tsc_command` looks for `tsc` on `PATH` and then at
  `runtime/typescript/node_modules/.bin/tsc`, and returns `None` when neither exists. Five
  execution helpers return `Option<String>` on top of them, and thirty-one tests unwrap with
  `let Some(..) else { return; }`.
- `Cargo.lock` is not tracked, so every CI run resolves dependencies fresh.
- `cargo fmt --all --check` is clean on the current tree.

## Goals / Non-Goals

**Goals:**

- Every pull request and push to a protected branch runs the whole Rust workspace's tests with
  `node` and `tsc` present, so the generated-JavaScript and generated-TypeScript tests execute.
- A run without those tools fails loudly, on CI and on a developer machine alike.
- The job follows the conventions the neighbouring workflows already use, so it needs no new
  actions or scripts.

**Non-Goals:**

- Clippy in CI. The tree's clippy state is unknown and cleaning it up is separate work.
- Running the Rust suite on macOS or Windows. One OS catches the class of bug this is for.
- Committing `Cargo.lock`. Worth doing, but it is a repository policy decision, not this change.
- Running `pnpm -r test` here. `deploy-playground.yml` covers the TypeScript runtime tests on
  `main`; giving them pull-request coverage is a separate, similar change.

## Decisions

**D1. One `rust` job inside `build.yml`, not a new workflow.** Pull request authors already look at
the `🏭 Build` workflow, and its triggers (`main`, `v*.*`, `validate/*`, every pull request) are
the ones the Rust suite should share. A separate `rust.yml` would duplicate the trigger block and
add a second status to configure as required. The job does not join the `build` → `package` →
`smoke-test-package` chain: it has nothing the packaging steps consume, and keeping it independent
lets it run in parallel with the slow .NET matrix.

**D2. Toolchain from `rust-toolchain.toml` via rustup, cache via `Swatinem/rust-cache@v2`.** This
is what `deploy-playground.yml` does, and it means the CI toolchain is the same one developers
get. `dtolnay/rust-toolchain@stable`, which the extension workflows use, would ignore the pin.
The cache action keys on the toolchain and `Cargo.toml` and keeps the `target` directory between
runs, which matters for a workspace that builds two `cdylib` crates.

**D3. `tsc` comes from the pnpm workspace, not a global install.** `tsc_command` already looks at
`runtime/typescript/node_modules/.bin/tsc`, so `pnpm install --frozen-lockfile` at the repository
root, with `pnpm/action-setup@v4` and `actions/setup-node@v4` at Node 24 with the pnpm cache,
gives the tests the same TypeScript version the runtime package pins. A `npm install -g
typescript` would be a second version to keep in step.

**D4. Missing tools are unconditional test failures, with no environment-variable opt-out.** The
root `package.json` already declares `node >= 22.18` as a development requirement, and the
repository's own TypeScript install is one `pnpm install` away, so there is no supported
configuration in which skipping is the right answer. An opt-out flag would be the vacuous pass
under another name. Concretely: `node_is_available` becomes `node_command() -> Command`, which
probes `node --version` once and panics with a message naming Node 24 and `PATH`; `tsc_command`
returns `Command` and panics naming `pnpm install` at the repository root. The five helpers
return `String`, and the thirty-one `let Some(..) else { return; }` sites become plain `let`
bindings. The doc comments that promise "says so when it cannot, so a run that executed nothing
does not pass silently" are rewritten to say the run fails.

**D5. `cargo fmt --all --check` rides along; clippy does not.** The tree is formatted today,
`rustfmt` is a pinned toolchain component, and the check takes seconds, so it costs nothing to
keep the tree that way. Clippy is excluded for the reason in Non-Goals.

**D6. The job is `ubuntu-latest` only, with a thirty-minute timeout**, matching the playground
validate job. The `build` matrix already proves the native SDK compiles on all three OSes.

**D7. The pnpm, Node, Rust-cache, and install steps live in a `setup-rust-node` composite action.**
Written out, the job's setup was a verbatim copy of `deploy-playground.yml`'s `validate` job: pnpm,
Node 24 with the pnpm cache, `Swatinem/rust-cache@v2`, `pnpm install --frozen-lockfile`. Two copies
means a Node bump has two places to edit and they can drift apart silently, so
`.github/actions/setup-rust-node/action.yaml` holds the four steps once and both jobs call it after
their checkout. The Node version and the pnpm install flags are now stated in exactly one place,
and `deploy-playground.yml` gains the action's path in its `paths` filter so a change to the setup
still validates the site. Checkout stays in the jobs, since a local action cannot run before it.
The six other jobs that pin a Node version are deliberately left alone. The five `src/vscode` ones
(`build.yml`'s `editor-assets`, and the extension and release workflows) set up a different
lockfile under `src/vscode` and need no Rust, and `package-publish.yml` sets up Node for npm
trusted publishing with no pnpm, no lockfile, and no Rust at all; folding any of them in would mean
parameterizing the action past the point where it reads more clearly than the steps it replaces. So
the action owns the Node version for the jobs that build Rust against the workspace, not for the
repository, and its description says so.

## Risks / Trade-offs

- [No `Cargo.lock`, so a new upstream release can break the job on a pull request that did not
  touch Rust] → The failure is visible and attributable through the job log. Committing the
  lockfile is the real fix and is listed as a non-goal so it can be decided on its own.
- [`cargo test --workspace` compiles `bindings/node/native`, a napi `cdylib`, on a runner where
  Node is only on `PATH`] → `napi-build` does not need Node headers on Linux, and
  `deploy-playground.yml` already compiles this crate on `ubuntu-latest`. If it does turn out to
  need an exclusion, `--workspace --exclude nx-node-native` is a one-line change.
- [First run on a cold cache is slow, and the cache invalidates on every dependency change] →
  Accepted. Subsequent runs on the same `Cargo.toml` set reuse the `target` directory.
- [Developers without Node on `PATH` now see thirty-one failing codegen tests instead of a green
  run] → That is the point of D4. The panic message names the fix, and the required version is
  already in `package.json`.
