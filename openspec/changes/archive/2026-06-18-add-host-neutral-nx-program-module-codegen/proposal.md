## Why

NX executable JavaScript output is currently generated as a small ESM file graph intended to be
written to disk. ReachMe needs a cacheable codegen artifact that can be stored independently from
host policy, then loaded later into a V8 isolate with a separately supplied NX runtime and
host-specific wrapper.

## What Changes

- Add a host-neutral NX program module output style for JavaScript executable codegen.
- Emit the generated NX program as a single ESM module whose public exports expose NX entrypoints,
  component descriptor/schema values, and cache metadata, while importing shared runtime helpers
  from a stable virtual runtime specifier.
- Keep the generated NX program module free of isolate host concerns such as HTTP routing,
  Cloudflare Worker exports, Rivet actor setup, bindings, auth, limits, and logging.
- Add public Rust API support for emitting this single module directly from a `ProgramArtifact`.
- Add an explicit `nxlang codegen` output-format option so callers can inspect or write either the
  existing file graph or the host-neutral program module without changing the default.
- Add .NET runtime binding support so managed hosts can invoke program-module codegen from an
  `NxProgramArtifact` and receive structured metadata plus source text.
- Preserve the existing multi-file JavaScript/TypeScript output style for readable file-based
  generation.
- Defer generation of isolate manifests, Cloudflare Dynamic Worker `WorkerCode` objects, Rivet
  adapters, and other host wrappers to the isolate host or later adapter-specific changes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `executable-code-generation`: Add a cacheable, single-module, host-neutral JavaScript program
  output style emitted from `ProgramArtifact` inputs, with the NX runtime supplied as a separate
  versioned dependency.

## Impact

- Updates `crates/nx-codegen` options, emitter plumbing, generated import handling, and tests.
- Extends `nxlang codegen` with an explicit output format option while keeping the existing file
  layout as the default.
- Updates `crates/nx-ffi` and `bindings/dotnet` so .NET hosts can generate program modules from
  reusable program artifacts.
- Adds generated-output and runtime execution tests that load the cached program module together
  with a separately supplied runtime module.
- Does not add or require a Cloudflare, Rivet, database, bundler, or isolate-host dependency.
