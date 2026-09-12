## Why

No workflow under `.github/workflows` runs `cargo test`. `build.yml` builds the native SDK, runs the
.NET tests, and packages, and the only jobs that install Node do so for the editor and playground
packages, so the Rust workspace and everything it verifies (the checker, interpreter, code
generation, typegen, and the checked-in fixtures) are tested only on developer machines. On top of
that, every test in `crates/nx-codegen/src/tests.rs` that executes generated JavaScript, or checks
generated TypeScript with `tsc`, returns early when the tool is missing, so even a CI run of
`cargo test` on a runner without Node would pass while executing nothing. RF1 and RF8 of the
`add-property-references` review were both bugs in exactly that generated-code path.

## What Changes

- Add a `rust` job to `.github/workflows/build.yml` that installs pnpm and Node 24, restores the
  pnpm workspace so the repository's own `tsc` is present, caches the Cargo build, and runs
  `cargo fmt --all --check` and `cargo test --workspace` on `ubuntu-latest`, under the toolchain
  pinned by `rust-toolchain.toml`. It runs on the same triggers as the existing jobs (pushes to
  `main`, `v*.*`, and `validate/*`, and every pull request). Its setup — pnpm, Node, the Cargo
  cache, and the workspace install — is a `.github/actions/setup-rust-node` composite action that
  `deploy-playground.yml`'s `validate` job, which had the same four steps written out, now calls
  too.
- Make `node` and `tsc` required by the codegen tests: a missing tool is a test failure whose
  message names the tool and how to install it, not a printed "skipping" line and a pass. The
  helpers that return `Option<String>` today return `String`, and the thirty-one
  `let Some(...) else { return; }` call sites become plain bindings. The test
  `javascript_output_executes_as_esm_when_node_is_available` loses its conditional name.
- Update item 5 of `docs/add-update-actions-followsup.md` to record that the job exists.

Out of scope, so this stays small: `cargo clippy` in CI, running the Rust suite on macOS and
Windows, and the `pnpm -r test` run for `runtime/typescript`, which `deploy-playground.yml`
already covers on its own triggers.

## Capabilities

### New Capabilities

None. This change is tooling and test infrastructure; no language, runtime, or generated-code
behavior changes, so `.openspec.yaml` sets `skip_specs: true`.

### Modified Capabilities

None.

## Impact

- `.github/workflows/build.yml`: one new job. It does not gate `package`, which keeps depending on
  `build` only, so a Rust test failure blocks the pull request through the required check rather
  than through the artifact chain.
- `.github/actions/setup-rust-node/action.yaml`: new. The four setup steps the `rust` job and
  `deploy-playground.yml`'s `validate` job share, so the Node version lives in one place.
- `.github/workflows/deploy-playground.yml`: the `validate` job's four setup steps become one call
  to that action, and the action's path joins the workflow's `paths` filter. No behavior change.
- `crates/nx-codegen/src/tests.rs`: the `node_is_available` and `tsc_command` helpers, the five
  execution helpers built on them, and every test that calls those helpers. Developers running
  `cargo test -p nx-codegen` without Node on `PATH`, or without having run `pnpm install`, will
  now see failures that name the missing tool instead of a green run.
- `docs/add-update-actions-followsup.md`: item 5 is closed.
