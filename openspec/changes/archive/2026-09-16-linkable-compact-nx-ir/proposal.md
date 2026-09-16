## Why

An NX IR artifact today is one self-contained program: whatever a snippet was compiled with travels
inside it, and every reference is a position (`m0:d44`) that changes whenever anything before it
changes. For the DrawnUI fiddle that means a one-line share weighs about 878 KB, because the
description of the whole DrawnUI API rides along with each snippet, and a share compiled against one
regeneration of that description would silently mean something else against the next. The fiddle
now refuses any share over 256 KB and asks, reasonably, that a share carry only the snippet, refer
to controls by name, and name the version of the description it was built against. The same
properties are what any host wants that serves NX programs from a server to a JavaScript runtime
today and to a Rust runtime later: an artifact that is small, links by name to things the runtime
already holds, and is defined by a schema rather than by the compiler's data structures.

The snippet's own part of the IR is also fourteen to twenty-two times its source, because the JSON
repeats every key name, every id string and every nested type object on every node. Measured on the
fiddle's presets, spans are a fifth of the bytes, types another fifth, id strings an eighth, and the
source text is included verbatim.

## What Changes

- **BREAKING** NX IR schema version 3, runtime ABI `nx-ir-runtime-v2`. Schema 2 artifacts are not
  read by the new runtime and are not emitted by the new compiler; there is no compatibility shim.
- **One module per artifact, with a module table.** An artifact carries exactly one module and a
  table of the modules it links against, each with an identity, a host-supplied version string and a
  content fingerprint. A program that spans several modules is several artifacts.
- **References by identity and name.** A reference is a module-table slot and a declaration name,
  never a position. Slots, expression ids and element ids become integers local to their declaration
  or module. Derived declarations keep the names the language already gives them (`User.Update`,
  `User.Property`).
- **Flat tables.** Each module is encoded as a string table, a type table, a constant table, a node
  table and a declaration list, with small integer kinds and integer operands, so a name, a type or a
  literal is written once and referenced by index. JSON remains the encoding; the shape is designed
  so a binary encoding is a later mechanical mapping of the same tables.
- **Debug data is a separate, optional section.** Spans and source text move into a section the CLI
  writes and the wasm SDK omits unless asked. Only the source fingerprint is always present.
- **Linking in the runtime.** The TypeScript runtime prepares a module once and links an entry
  module against prepared modules supplied by the host, checking the recorded versions. Evaluation
  APIs are unchanged once a program is linked.
- **Workspace builds through the wasm SDK, with implicit imports.** The SDK builds from a set of
  in-memory modules with an entry, lets the host name modules every document sees without an import
  line, and emits artifacts for the modules the host asks for. The prelude-concatenating build is
  removed; the playground and the fiddle move to implicit imports.
- **A conformance corpus.** NX sources with their expected artifacts and expected evaluation results,
  checked by the emitter and the TypeScript runtime, and later by a Rust runtime. It also holds the
  size budget: a stripped artifact is at most six times its source.
- **CLI.** The `nx-ir` target writes one artifact per module with the debug section, and a new
  `nxlang ir explain` renders an artifact as readable text so tests and docs never read raw tables.
- **Fiddle.** A share carries the snippet's artifact and the catalog version it names; the catalog's
  artifact ships inside the NX runtime bundle and is prepared once; NX is registered by the dev host
  rather than by the engine. (The fiddle work is tracked here for the spec and sequenced after the
  NX packages publish; its code lives in the fiddle repository.)

## Capabilities

### New Capabilities

None. Every change is to a capability that already exists.

### Modified Capabilities

- `nx-ir-format`: artifacts carry one module and a module table; references are identity plus name;
  modules are flat tables; provenance is an optional debug section; a conformance corpus with a size
  budget defines the format.
- `typescript-ir-runtime`: preparation is per module and linking is a separate step with version
  checks; the runtime passes the conformance corpus.
- `sdk-wasm`: program builds take a workspace of modules with an entry, implicit imports, emit
  selection and versions; the prelude-aware build is removed; the in-process language service takes
  implicit imports instead of a prelude; emitted IR omits the debug section unless requested.
- `workspace-programs`: a build can name modules that every other module imports implicitly.
- `cli-code-generation`: the `nx-ir` target writes one artifact per module with debug data, and an
  `ir explain` command renders an artifact readably.
- `fiddle-nx-language`: a share carries the snippet artifact and the catalog version; the catalog
  artifact ships with the runtime bundle and is prepared once; the host registers NX.

## Impact

- `crates/nx-codegen`: the IR model, emitter and its tests are rewritten around the new schema;
  the JavaScript and TypeScript targets are untouched.
- `crates/nx-api`, `crates/nx-hir`: implicit imports as a build and snapshot option, synthesized at
  lowering; a check that top-level declaration names are unique per module.
- `bindings/wasm/native` and `bindings/wasm`: workspace build, emit selection, the removal of
  `buildProgramWithPrelude`, language service options; `bindings/node`, `crates/nx-ffi` and
  `bindings/dotnet` follow for parity, since their IR emission changes shape.
- `runtime/typescript`: module preparation, linking, evaluation over the flat tables, and the
  conformance corpus runner.
- `crates/nx-cli`: per-module artifacts and the explain command.
- `sites/playground`: compiles the catalog as its own module with an implicit import, emits the
  catalog's artifact at build time, and links each compile's snippet-only artifact against it.
- `docs/nx-ir-format.md` and the READMEs of `bindings/wasm` and `runtime/typescript`. The fiddle
  consumes `@nx-lang/sdk-wasm` and `@nx-lang/ir-runtime` from npm and pins the versions this change
  is released under; no manifest is edited for that, since the release stamps every publishable
  package at the tag's version.
- Fiddle repository: the runtime bundle, `compile` and `mount`, the share module, registration in
  the dev host, tests and docs; a new pull request against the fiddle's current main.
