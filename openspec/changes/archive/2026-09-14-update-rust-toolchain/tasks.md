## 1. Toolchain pin

- [x] 1.1 Change `channel` in `rust-toolchain.toml` from `1.91.1` to `1.98.1` and verify `rustup show` in the repository reports 1.98.1 as the active toolchain with rustfmt, clippy, and wasm32-wasip1 installed

## 2. Documentation

- [x] 2.1 Replace the "Rust 1.75 or later" prerequisite in `README.md` with rustup and the pinned toolchain, and verify the text no longer names a version
- [x] 2.2 Replace the "Rust 1.75+" prerequisite in `docs/src/content/docs/tutorials/getting-started.md` the same way and verify the text no longer names a version
- [x] 2.3 Drop the stale test count from the `cargo test` comment in `README.md` and verify no line in the Development section names a count or version

## 3. Verification under the new toolchain

- [x] 3.1 Run `cargo fmt --all --check` and verify it passes
- [x] 3.2 Run `cargo clippy --workspace --all-targets --keep-going` under both 1.98.1 and 1.91.1, diff the diagnostics, and fix every one that is new under 1.98.1 (the pre-existing ones are out of scope; see design.md)
- [x] 3.3 Run `cargo test --workspace` and verify all tests pass
- [x] 3.4 Run `pnpm -r build` and verify the WebAssembly module builds with the new compiler
- [x] 3.5 Run `pnpm -r test` and verify the wasm SDK still matches the Node SDK

## 4. Local rustup cleanup

- [x] 4.1 Run `rustup update` and verify the default `stable` toolchain reports 1.98.1
- [x] 4.2 Uninstall the 1.75, 1.75.0, 1.80, and 1.91.1 toolchains and verify `rustup toolchain list` shows only `stable` and `1.98.1`

## 5. Keep the pin current

- [x] 5.1 Add a Renovate `customManagers` regex entry for the `channel` line in `rust-toolchain.toml` and verify the regex matches the current line and `renovate-config-validator` accepts the file

## 6. MSRV and dependency resolution

- [x] 6.1 Set `rust-version` in the workspace `Cargo.toml` to `1.98.1` and `resolver` to `"3"`, and verify `cargo metadata` accepts both
- [x] 6.2 Resolve `Cargo.lock` from scratch under resolver 2 and again under resolver 3, diff the lockfile and `cargo tree`, and verify the graph is identical so the switch changes no dependency version today
- [x] 6.3 Verify MSRV-aware resolution is actually in effect by resolving with `rust-version` temporarily set to 1.85, confirming cargo holds crates back (the `icu_*` family, `idna_adapter`, `napi-build`), then restore 1.98.1 and re-verify the graph
- [x] 6.4 Run `cargo fmt --all --check`, `cargo test --workspace`, `pnpm -r build`, and `pnpm -r test` under the new resolver and verify all pass
- [x] 6.5 Add a second Renovate custom manager for the `rust-version` line in `Cargo.toml`, grouped with the pin by dep name *and* datasource so the group cannot also capture the `rust` Docker base image, and verify both regexes match their file, that the `Cargo.toml` pattern does not match member manifests, and that `renovate-config-validator` accepts the file

## 7. Commit the lockfile

- [x] 7.1 Remove `Cargo.lock` from `.gitignore` and track the resolved lockfile, and verify `git ls-files Cargo.lock` lists it
- [x] 7.2 Copy `Cargo.lock` in `sites/playground/Dockerfile` and rewrite the comment that explained its absence, and verify the image's build stage no longer resolves the graph itself
- [x] 7.3 Add `Cargo.lock` to the `deploy-playground.yml` path triggers, so a dependency change redeploys the module the site ships (`vscode-extension.yml` already lists it)
- [x] 7.4 Add `--locked` to the `cargo test --workspace` step in the CI `rust` job, and verify it rejects a `Cargo.toml` edited without regenerating the lockfile and passes with the committed one
- [x] 7.5 Verify the committed lockfile is already covered by the weekly `lockFileMaintenance` that `config:best-practices` enables via `:maintainLockFilesWeekly`, and confirm no configuration change is needed for it
- [x] 7.6 Run `cargo test --workspace --locked`, `pnpm -r build`, and `pnpm -r test` and verify all pass
- [x] 7.7 Narrow the `sites/playground/Dockerfile` lockfile comment to attribute the guarantee to the `--locked` gate in `build.yml` rather than to the image, and verify no shipped-artifact build script claims a guard it does not pass
