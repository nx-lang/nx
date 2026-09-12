## 1. Require `node` and `tsc` in the codegen tests

- [x] 1.1 In `crates/nx-codegen/src/tests.rs`, replace `node_is_available` with `node_command() -> Command` that probes `node --version` once and panics with a message naming Node 24 and `PATH` when it is missing; route the three `Command::new("node")` call sites through it. Verify `cargo test -p nx-codegen` passes with Node installed.
- [x] 1.2 Make `tsc_command` return `Command`, panicking with a message that names `pnpm install` at the repository root when neither `PATH` nor `runtime/typescript/node_modules/.bin/tsc` has it; update its three `let Some(..) else { return; }` callers. Verify `cargo test -p nx-codegen` passes with the pnpm workspace installed.
- [x] 1.3 Change the five execution helpers (`execute_generated_javascript_root` and the four beside it) to return `String`, remove every `let Some(..) else { return; }` from their callers, rename `javascript_output_executes_as_esm_when_node_is_available` to `javascript_output_executes_as_esm`, and rewrite the two helper doc comments that describe skipping. Verify `grep -c 'else {$' crates/nx-codegen/src/tests.rs` followed by a bare `return;` finds no remaining sites (a `grep -B1 -E '^\s*return;\s*$'` over the file is empty) and `cargo test -p nx-codegen` passes.
- [x] 1.4 Verify the failure mode: run one JavaScript test with Node hidden, for example `PATH=$(dirname "$(command -v cargo)") cargo test -p nx-codegen javascript_output_executes_as_esm`, and confirm it fails with the panic message from 1.1 rather than passing.

## 2. Add the Rust job to the build workflow

- [x] 2.1 Add a `rust` job (`name: 🦀 Rust`) to `.github/workflows/build.yml` on `ubuntu-latest` with `timeout-minutes: 30`: checkout (same pinned `actions/checkout` SHA as the other jobs), `pnpm/action-setup@v4`, `actions/setup-node@v4` with `node-version: '24'` and `cache: 'pnpm'`, `Swatinem/rust-cache@v2`, `pnpm install --frozen-lockfile`, `cargo fmt --all --check`, and `cargo test --workspace`; include the same "rustup picks the toolchain from rust-toolchain.toml" comment `deploy-playground.yml` carries. Verify `actionlint` (if installed) or a YAML parse reports no errors, and that the job has no `needs`.
- [ ] 2.2 Push the branch and open or update the pull request. Verify the `🦀 Rust` job runs, that its log shows `node --version` reporting 24 and the codegen JavaScript and TypeScript tests executing rather than skipping, and that it finishes green.
- [x] 2.3 Extract the job's pnpm, Node, Rust-cache, and `pnpm install` steps into
  `.github/actions/setup-rust-node/action.yaml`, call it from both the `rust` job and
  `deploy-playground.yml`'s `validate` job, and add the action's path to that workflow's `paths`
  filter. Verify `actionlint` reports no errors for either workflow and that neither job's
  remaining steps changed.

## 3. Documentation

- [x] 3.1 Rewrite item 5 of `docs/add-update-actions-followsup.md` to record that the `rust` job exists and the tests now fail without `node` and `tsc`, leaving items 1 through 4 and 6 untouched. Verify the section reads as closed and the file's other headings are unchanged.

## 4. Integration verification

- [x] 4.1 Run `cargo test --workspace` and `cargo fmt --all --check` locally with Node and the pnpm workspace present. Verify both pass.
- [x] 4.2 Run `openspec validate add-rust-ci-job --strict`. Verify it reports no issues.
