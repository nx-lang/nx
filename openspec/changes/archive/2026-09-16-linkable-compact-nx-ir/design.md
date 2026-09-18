## Context

See proposal.md for the motivation. The facts that shape the design:

- An IR artifact is emitted by `crates/nx-codegen` from a `CodegenProgram`, itself built from a
  `ProgramArtifact` that already carries every module of the program as a `ResolvedModule` with a
  positional `RuntimeModuleId` and `LocalDefinitionId`s. Ids are formatted as `m{module}:d{index}`;
  slots as `{declaration}:prop:{n}`; expression and element ids likewise. Every reference already
  carries the declaration's `name` and `kind` beside the positional id.
- The TypeScript runtime (`runtime/typescript`) prepares one `NxIrProgram` into
  `declarationsById`, entrypoint maps and `nominalShapesByDiscriminator`, and resolves every
  reference through `declarationsById`. There is no second program to link against.
- The wasm module (`bindings/wasm/native`) exposes a single-source build. The Rust API already has
  the in-memory multi-module workspace (`NxWorkspace`, `build_workspace_program_artifact`) and the
  C FFI exposes it; the language snapshot already takes several documents.
- Wildcard imports without an alias (`import "./drawnui"`) bring a module's names into scope
  unqualified. The language service already answers hover and completion across imports.
- Hosts get the catalog into scope by concatenation: `buildProgramWithPrelude` in the SDK and the
  `prelude` option of the language service and the HTTP service, with position shifting in
  `@nx-lang/language-core`. The HTTP service's spec calls this a stop-gap "for hosts whose context
  declarations cannot yet be imported without loss".
- Measurements on the fiddle's four presets, snippet part only, schema 2: 14.5x to 21.6x the
  source. Spans are about 20 percent of the bytes, semantic types about 20 percent, id strings 10
  to 13 percent, the verbatim source text 2 percent. A keyless encoding of the same data without
  spans measured at 8 to 10x; shorter ids and a type table are the remaining levers. MessagePack
  over the same structure saves a further fifth, but the fiddle's share is a JavaScript module
  string whose 256 KB limit counts characters, so binary would have to be base64 and would come
  out larger than JSON.
- The project has no backward-compatibility obligation yet (AGENTS.md): schema 2 can be dropped.

## Goals / Non-Goals

**Goals:**

- A share, or any served program, carries only its own module and links by name to modules the
  runtime already holds.
- One schema, specified independently of the compiler's structs, that a JavaScript runtime reads
  now and a Rust runtime can read later, with a conformance corpus that both are checked against.
- A stripped artifact at most six times its source; the fiddle's presets under 128 KB.
- JSON stays the encoding; the tables are binary-ready.
- Preparation of a large module happens once per host, and linking a snippet to it is cheap.

**Non-Goals:**

- A bytecode or a virtual machine. NX evaluates a value tree once and again on state change; the
  cost is in drawing, not evaluation, and a tree keeps reactivity analysis, tooling and two-runtime
  parity simple. If evaluation ever matters, the runtime can compile nodes to closures at load
  time without a format change.
- A binary encoding now. It is designed for, not built.
- Changing the value model: `$type` discriminators, nominal shapes, canonical values and the
  boundary normalization rules stay as they are.
- The JavaScript and TypeScript source-generation targets, which share `CodegenProgram` but not
  the IR emitter.
- Backward compatibility with schema 2. No runtime reads it, no converter upgrades it, and no
  stored share or cached artifact in the old format is migrated: nothing outside this repository
  and the fiddle's open pull request has ever consumed one, and the project's guidance is to prefer
  simpler new code over compatibility.
- Removing the prelude from the HTTP language service, which serves the VS Code path and is not a
  consumer of artifacts. Its spec stays; only the wasm SDK and `language-core`'s callers change.
- Cross-version linking policy beyond a host opt-in. Whether a share compiled against catalog 9
  should draw on catalog 10 is the host's call; the runtime offers both behaviors.

## Decisions

### D1. One module per artifact, with a module table

An artifact holds one module. Its `modules` table lists the module itself at slot 0 and every
module it references, transitively reachable ones included only when referenced directly, each as
`{ identity, version, fingerprint }`. `version` is whatever string the host passed for that module
at build time, empty otherwise; `fingerprint` is the existing 64-bit fingerprint of the module's
source, as a decimal string.

Why: the fiddle needs the snippet alone and the catalog once; the CLI needs whole programs. One
module per artifact serves both with one shape, and the table is what linking and version checks
read. The alternative, a multi-module artifact with an "omit these modules" option, would make the
snippet case a special mode of the whole-program case, and every consumer would have to handle
both.

### D2. References are `[slot, name]`

A reference is a pair: a module-table slot and a string-table index holding the declaration's
name. Slot 0 is the artifact's own module, so intra-module references use the same shape. The
runtime builds a name-to-declaration map per prepared module; a lookup is one map hit per reference
at prepare or link time, then a direct pointer.

Top-level names are unique per module, which the compiler will enforce with a diagnostic if it does
not already. Derived declarations use the names the language gives them, `T.Update` and
`T.Property`. Slots become integers local to their declaration's frame, assigned in the order the
emitter already walks; expression ids are dropped (nothing read them); element ids become
module-local integers.

Alternative considered: keep positional ids within a module and use names only across modules.
Rejected because a share can be re-emitted for its own module too, and one reference shape is
simpler for both runtimes.

### D3. Flat tables in JSON

Per module: `strings`, `types`, `constants`, `nodes`, `declarations`, plus `entrypoints`. Each
node is an array `[kind, ...operands]` where operands are integers: string, type, constant or node
indices, node ranges as `[start, count]` pairs, or inline small values. Each type is
`[kind, ...operands]` referencing other types by index and nominal declarations as `[slot, name]`.
Declarations are arrays whose layout depends on their kind; fields are `[name, type, default, flags]`
with `default` a node index or `-1`. Kind numbers are assigned in the spec document and never
reused.

Why JSON: `JSON.parse` of a share-sized document is about a millisecond in a browser and faster
than a hand-written binary decoder in JavaScript; the artifact has to live inside a JavaScript
module string in the fiddle; and a readable form is needed for tests and support. Why tables
anyway: they remove the repeated keys, ids and type objects that are the bulk of today's bytes,
and they make a later binary encoding a one-to-one mapping (each array becomes a length-prefixed
sequence of varints).

Readability is recovered by `nxlang ir explain`, which renders an artifact with every index
resolved, so corpus tests and docs read explained text rather than raw tables.

### D4. Debug data is a separate optional section

`debug: { spans: { declarations: [...], nodes: [...] }, source: "..." }`, with spans stored as
`[start, end]` pairs parallel to the declaration list and node table. The CLI writes it; the wasm
SDK and FFI omit it unless asked. Runtime diagnostics use the span when present and fall back to
the declaration name.

Why: spans were a fifth of the bytes and the source text is stored by every host separately (the
fiddle stores the snippet source beside the module). Keeping the section parallel to the tables
means the artifact with and without it differs only in that key, which the corpus checks.

### D5. Implicit imports replace the prelude

A build, and a language snapshot, accept `implicitImports: [identity, ...]`. Lowering synthesizes a
wildcard import of each listed identity into every module that is not itself listed, before scope
resolution. The module's text is untouched, so positions and diagnostics need no shifting.

Why not prepend an `import` line: it would keep the offset arithmetic (one line instead of six
hundred), would break a document that begins with a bare element, and would not help the language
service, which needs the same visibility. Why a build option rather than a language feature: no
NX file should silently depend on a host's context; the option is the host's declaration of what
it provides.

`buildProgramWithPrelude` and the `prelude` option of the wasm SDK's language service are
removed. The playground moves to a two-module workspace: a compile emits the visitor's module
alone, and the catalog's artifact is emitted at build time by a Vite plugin through the same wasm
module, so the renderer prepares it once per page and links each compile against it — the same
shape the fiddle uses, minus a committed artifact, since the site builds from source anyway.
`language-core` keeps its prelude arithmetic for the HTTP service.

### D6. Workspace builds through the wasm SDK

The wasm module gains a workspace build taking `{ modules: [{ identity, source, version? }],
entry, implicitImports? }` and an emit call taking `{ modules?: [identity...], debug?: boolean }`
that returns one artifact per requested module (default: the entry). The single-source build stays
as a wrapper that builds a one-module workspace. The Node SDK and the C FFI take the same options so
parity tests can compare byte-for-byte.

A module's version travels on the module — `NxWorkspaceModule` in Rust and in .NET, the module
input in both JavaScript SDKs, the module descriptor in the C FFI — and is never an emit option.
The version describes the source, the way the fingerprint does, so it is fixed when the module goes
into the build: every artifact emitted from one program then agrees on it, and a catalog artifact
and a snippet's reference to that catalog cannot be emitted with different versions from the same
build and fail to link to each other. The version also enters the program's fingerprint, since two
builds that differ only in a version emit different artifacts.

### D7. Prepare, then link

The runtime API becomes `prepareNxIrModule(input) -> PreparedModule` and
`linkNxIrProgram(entry, { resolve(identity): PreparedModule | undefined, allowVersionMismatch?:
boolean }) -> NxPreparedProgram`. `prepareNxIrProgram(input)` remains for a self-contained artifact
and is `link` with an empty resolver. Every evaluation API keeps taking `NxPreparedProgram`.
Linking builds the program's reference table from the entry's module table and checks versions and
the presence of every referenced declaration eagerly; it never copies a prepared module, so one
prepared catalog serves any number of programs. Nominal shapes are indexed by `prepare`, per module,
and a `$type` is resolved across the linked modules when one is asked about — so linking a snippet
costs the size of the snippet, not the size of the catalog it links against.

Why eager checking: a missing control should fail at link time with the module and name, not
midway through evaluation.

### D8. A conformance corpus, with the size budget in it

`specs/ir-conformance/` (or the equivalent under `crates/nx-codegen/tests`) holds NX sources,
expected artifacts with and without the debug section, and expected canonical results for named
entrypoints. The emitter's tests pin the artifacts; the TypeScript runtime's tests evaluate them;
a Rust runtime will do the same. The size bound of six times the source is a corpus test so a
regression in the encoding is caught where it is introduced.

Why six: keyless JSON without spans measured 8 to 10x on real presets before the string and type
tables and integer slots; the remaining levers are estimated at 5 to 7x, and six leaves the fiddle's
largest preset (8.7 KB of source) near 52 KB, well under its 128 KB budget.

### D9. Schema 3 replaces schema 2 outright

`NX_IR_SCHEMA_VERSION` becomes 3 and the runtime ABI `nx-ir-runtime-v2`. The runtime refuses
schema 2 with a diagnostic naming both versions. The existing required-feature strings survive as
they are, and the eager feature flag remains the baseline.

There is no compatibility layer of any kind: the emitter cannot produce schema 2, the runtime does
not read it, and no converter is written. The project has no backward-compatibility obligation yet
and prefers simpler new code, so the cost of a break is only the tests, docs and fiddle branch that
this change already rewrites.

Why one break instead of two: linkage and compaction each change the schema; landing them under one
version means one runtime release, one docs rewrite and one fiddle migration.

### D10. Fiddle: catalog artifact in the bundle, snippet artifact in the share

`npm run runtime` emits the catalog artifact from `nx/catalog/drawnui.nx` with the version from
`catalog-meta.json` and bundles it as JSON text in nx-runtime.js, failing when the committed source
does not match. `prepare` prepares it once. `compile` builds the two-module workspace and emits the
snippet with `debug: false`; the share module is `const IR = <artifact>; export default () =>
NxDrawn.mount(IR)` as before. `mount` links against the prepared catalog, first strictly, then with
`allowVersionMismatch` when the versions differ, and draws the engine's failure label naming both
versions when that fails. Registration moves to the dev host's Program.cs, where a comment already
marks the spot; the engine keeps the language class, presets and scripts.

### D11. Sequencing across the two repositories

NX first: D1 to D9, docs and the corpus, then a release that publishes `@nx-lang/sdk-wasm`,
`@nx-lang/ir-runtime` and `@nx-lang/language-core`. Then the fiddle branch from its current main,
pinned to those versions, as a new pull request; the open one is closed.

## Risks / Trade-offs

- [Six-times budget is missed on some corpus program] → The corpus test says by how much; the next
  levers are delta-coded node ranges and a per-module property-name table for descriptors. The
  fiddle's own budget has a wide margin, so a modest miss is a tuning task, not a redesign.
- [Top-level names are not unique per module today] → Add the diagnostic in `nx-hir` and cover it
  in the corpus; a language that lets a function and a type share a name would need `[slot, kind,
  name]` references instead, decided before the emitter is written.
- [Cross-version linking draws something subtly different] → Default is strict; the fiddle opts in
  and labels a failure with both versions. Property-level compatibility is out of scope.
- [Two encodings of the same model drift, tables versus explained text] → Explained text is derived
  from the tables by one renderer used by tests and the CLI; there is no second reader.
- [Every IR test in five suites changes shape at once] → Tests move to comparing explained text or
  parsed values, and the corpus becomes the one place expected artifacts live.
- [The playground and the fiddle regress on diagnostics positions] → Implicit imports leave the
  document's text untouched, so positions cannot shift; the trailing-element scenario is in the
  corpus.
- [Rust runtime is hypothetical] → The corpus and the schema document are written so that a Rust
  reader is a serde model plus an evaluator; nothing else in this change assumes it.

## Migration Plan

1. Land the NX change; `cargo test --workspace`, `pnpm -r test` and the .NET tests green.
2. Update `docs/nx-ir-format.md` to the schema 3 document, with the kind tables, and the READMEs.
3. Tag a release; confirm the three packages publish.
4. Fiddle: new branch from main, pin the packages, apply D10, open the new pull request, close the
   old one.

Rollback: the previous package versions remain on npm and the fiddle's old branch pins them; no
share in the new format exists until the fiddle merges.

## Open Questions

- Resolved: the corpus lives at `specs/ir-conformance/`, beside the other specifications, so the
  TypeScript runtime and a future Rust runtime read it from one place; the codegen crate's tests
  reach it by a relative path and regenerate it with `NX_UPDATE_CORPUS=1`.
