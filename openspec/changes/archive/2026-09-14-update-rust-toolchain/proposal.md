## Why

`rust-toolchain.toml` pins Rust 1.91.1, released November 2025. Current stable is 1.98.1 (September
2026), seven releases later. Nothing in the repository needs the old version; the pin was simply not
kept current. The lag has a visible cost: the VS Code rust-analyzer extension ships a server that
tracks current stable and reports a toolchain version mismatch against the ten-month-old
proc-macro server inside 1.91.1.

## What Changes

- Bump the pinned channel in `rust-toolchain.toml` from 1.91.1 to 1.98.1, keeping the same
  components (rustfmt, clippy) and target (wasm32-wasip1).
- Fix the two places that state a Rust prerequisite (README and the getting-started tutorial)
  so they point at rustup and the pinned toolchain instead of the stale "Rust 1.75+" claim, which
  was already below the minimum the workspace declared even before this change.
- Verify the workspace under the new toolchain: formatting, clippy, native tests, the WebAssembly
  module build, and the pnpm workspace tests that compare the wasm SDK against the Node SDK.
- Add Renovate custom managers for the `channel` line in `rust-toolchain.toml` and the
  `rust-version` line in `Cargo.toml`, grouped into one pull request, so future Rust releases arrive
  as pull requests instead of waiting for someone to notice the pin is stale.
- Raise the workspace MSRV from 1.85 to the pin and switch the workspace to `resolver = "3"`. The
  MSRV was inert under `resolver = "2"`; resolver 3 makes cargo select dependency versions against
  it, so every future resolution — a new dependency, a Renovate lockfile refresh — stays inside the
  pin instead of pulling in a crate that needs a newer compiler.
- Commit `Cargo.lock` instead of ignoring it, so dependency versions stop drifting the way the
  compiler version did, and add `--locked` to the CI test step so a dependency change cannot land
  without the lockfile that goes with it.
- Locally, refresh rustup's default `stable` and remove the toolchains left behind by earlier
  pins (1.75, 1.75.0, 1.80, 1.91.1) once the new pin is verified.

No source code changes are expected, and the resolver switch changes no dependency version today:
resolved from scratch under both resolvers, `Cargo.lock` comes out identical. If the newer compiler
or clippy surfaces new warnings, fix them as part of the bump rather than allowing them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a tooling change with no spec-level behavior change, so `.openspec.yaml` sets
`skip_specs: true`. The `sdk-wasm` spec already refers to "the toolchain the repository documents
and pins" without naming a version, so it needs no delta.

## Impact

- `rust-toolchain.toml`: the single source of the toolchain for local builds, the shared
  `setup-rust-node` CI action, and the playground Dockerfile, which copies the file so rustup
  installs the pin inside the `rust:1-bookworm` image. One edit reaches all three.
- The VS Code extension workflows (`vscode-extension.yml`, `vscode-release.yml`): their
  `dtolnay/rust-toolchain@stable` step only sets the rustup default, which the pin outranks, so
  `nx-lsp` inside the released VSIX is built by the pinned compiler too. The next extension
  release ships an `nx-lsp` compiled by 1.98.1.
- `Cargo.toml`: `rust-version` becomes `1.98.1` and `resolver` becomes `"3"`. Nothing in the
  workspace is published to crates.io, so the MSRV is not a promise to downstream consumers; it is
  now an input to dependency resolution instead of an unverified claim.
- `Cargo.lock`: now tracked. `.gitignore` drops it, `sites/playground/Dockerfile` copies it so the
  deployed image builds the versions the commit names, `deploy-playground.yml` gains it as a path
  trigger, and `vscode-extension.yml` already listed it as one — that trigger has never been able
  to fire.
- `.github/workflows/build.yml`: the Rust test step gains `--locked`.
- `.github/renovate.json`: custom managers for the Rust pin and the MSRV, grouped as one update and
  scoped by datasource so the group does not also capture the `rust` Docker base image. No lockfile
  maintenance setting is added: `config:best-practices` already runs it weekly, so the newly tracked
  `Cargo.lock` joins the pnpm lockfiles it has been refreshing all along.
- `README.md` and `docs/src/content/docs/tutorials/getting-started.md`: prerequisite wording.
- Developer machines: rustup downloads 1.98.1 on the next `cargo` invocation in the repository.
- The `deploy-playground` workflow triggers on `rust-toolchain.toml`, so merging this redeploys
  the playground with a module built by the new compiler.
