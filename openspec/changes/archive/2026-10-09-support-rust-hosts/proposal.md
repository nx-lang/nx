## Why

A Rust program can run compiled NX today only from a checkout of this repository: `nx-ir-runtime`
and the two crates under it are not on crates.io, the code that composes the instances of nested
components exists only as TypeScript in a host's repository, and the runtime's stack budget assumes a
megabyte of free stack that a WebAssembly host does not have. A spike that drew NX snippets with a
Rust UI engine, natively and in a browser, met all three: it worked through path dependencies, left
out every component below the root, and crashed the page when a deep evaluation began with less
than about 160 KiB of stack free. Hosts that build from crates.io alone cannot adopt the runtime until
these are closed.

## What Changes

- `nx-ir`, `nx-value` and `nx-ir-runtime` are published to crates.io on the package release track:
  packed at the release tag's version, attached to the draft GitHub Release, and published when the
  release is. Every other crate of the workspace is marked unpublishable.
- The Rust runtime gains an instance tree: one instance per use of an authored component, keyed by
  its place in the program's output, with props flowing down, state kept as the output changes, a
  handler dispatched against the instance whose body bound it, and an emitted action routed to the
  handler the parent bound, all or nothing. The tree says which nodes have rendered since the host
  last walked them, so a host walks only what changed. It is indifferent to what the host makes of
  the output: a host that draws nested components and one that keeps another structure derived
  from them both use it without writing the composition themselves.
- The native stack an evaluation may use becomes a runtime option, `max_stack_bytes`, defaulting to
  the megabyte that is fixed today. A host with a small stack states what it has free, and an
  evaluation that would overrun it fails with `nx-ir-resource-limit` instead of ending the process.
- The runtime's build for `wasm32-unknown-emscripten` and `wasm32-wasip1` is checked in CI, and its
  README says what a host on a small stack sets.
- **BREAKING**: `RuntimeOptions` gains a field, so a Rust caller that writes the struct out in full,
  without `..RuntimeOptions::default()`, no longer compiles.

## Capabilities

### New Capabilities

- `component-instance-tree`: how a host-facing tree composes instances of authored components
  (identity by position, re-initialization with kept state, dispatch against the binding instance,
  effect routing to parents, atomic failure, inert handlers), and its implementation in the Rust
  runtime.

### Modified Capabilities

- `rust-ir-runtime`: the native stack budget is an option the host sets rather than a fixed
  megabyte, and the runtime is required to build for the WebAssembly targets hosts use.
- `package-release-automation`: the release track covers crates.io for the three runtime crates;
  the Rust tools stay out of it.

## Impact

- `crates/nx-ir-runtime`: a new `tree` module and its exports, `RuntimeOptions::max_stack_bytes`,
  README sections for the tree and for small stacks.
- `crates/nx-ir`, `crates/nx-value`, `crates/nx-ir-runtime`: crates.io metadata (description,
  keywords, README), versioned dependencies on each other.
- Root `Cargo.toml` and every other crate manifest: `publish = false`.
- `scripts/`: packing and publishing of the crates beside the npm scripts. `.github/workflows/`:
  `build.yml`, `release.yml`, `package-publish.yml`. `docs/deployment.md`,
  `docs/deployment-setup.md`.
- crates.io: three new crate names owned by the project, with a trusted-publisher policy each. The
  first version of each has to be published by a maintainer with a token before a policy can exist.
- Callers inside the repository that build `RuntimeOptions` field by field (`nx-codegen` tests, the
  bindings once `retire-hir-interpreter` lands) add the field or the default.
- Not affected: the TypeScript runtime, the image format, the conformance corpus's existing
  programs, the .NET and npm packages.
