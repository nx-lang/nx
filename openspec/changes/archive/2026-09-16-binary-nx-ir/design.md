## Context

See proposal.md for motivation. The facts that shape the design:

- `linkable-compact-nx-ir` is implemented on this branch and unreleased. Its emitter builds an
  `NxIrArtifact` whose tables are trees of `IrItem` (int, float, string or list) and serializes it
  through serde, compact for the SDKs and pretty for the CLI and the corpus. `ir_explain.rs` walks
  the same `IrItem` trees. The TypeScript runtime's `TableReader` walks the parsed JSON arrays and
  already validates every declaration and node at prepare time, reporting each malformed entry as a
  diagnostic.
- The layouts in `docs/nx-ir-format.md` use six operand shapes: a single index or small integer, an
  optional node written `-1`, a `[slot, name]` reference, an optional reference written `[]`, a
  nested list `[x...]`, and a constant that is a number or a decimal string.
- The debug section holds spans parallel to the declaration list and node table, and the module's
  source text. The linkable spec requires an artifact with and without it to differ only in that
  section.
- Artifacts cross four boundaries as UTF-8 JSON text today: the wasm ABI's `{ status, ptr, len }`
  result record, the C FFI's `NxBuffer`, napi strings, and .NET `string`. Every boundary already
  carries arbitrary bytes for something else (the FFI's MessagePack payloads, napi `Buffer`).
- The corpus ratios of artifact bytes to source bytes run 1.49x to 2.95x today against a six-times
  budget. The measured image is about 1.4x the compact JSON once the redundant arity cell is
  dropped, so the ratios rise to at most about 4.2x.
- The fiddle's share is a JavaScript module string; its budget counts characters. Its player never
  loads the compiler.

## Goals / Non-Goals

**Goals:**

- One encoding, read by the TypeScript runtime and the Rust tooling from the same document, with
  the corpus pinning bytes and a reviewer reading text.
- A reader in either language opens an artifact without allocating for it, and decodes a string only
  when the string is named.
- Every reader refuses a malformed image with a diagnostic before evaluating anything, and no input
  can make it read outside the image.
- The change is mechanical for the emitter and the runtime: kinds, layouts, linking and the value
  model are untouched.

**Non-Goals:**

- Forward compatibility of entry layouts. A layout change is a schema bump; a reader refuses what it
  does not implement.
- A name index, a metadata section, compression, or a wasm runtime (see proposal.md).
- A textual assembler. Explained text stays a rendering.
- Changing what the debug section carries or when the SDKs emit it.

## Decisions

### D1. A fixed header, a section directory, rigid sections

The image begins with a 16-byte header: the ASCII magic `NXIR`, the schema version `3`, the total
image length, and the number of directory entries. The directory follows, one `(kind, offset,
length)` triple of `u32` per section. Section kinds are `0` strings, `1` module, `2` types, `3`
constants, `4` nodes, `5` declarations, `6` debug. Every section starts on a four-byte boundary, and
every section but `debug` is required, exactly once, in kind order. A reader skips a directory entry
whose kind it does not know.

Why a directory: it is the one place the format can grow without a schema bump. The debug section
is optional today; the precompiled-library metadata section and a name index are sections tomorrow;
none of them requires a reader written before it to change. Within a section the layout is rigid:
moving a field is a schema bump, and a reader refuses a version it does not implement, naming both.
That is the linkable design's D9 policy carried forward, and it is the right one while every host
stores the source beside the artifact and can recompile — the fiddle's share spec requires it, the
playground builds its catalog from source, and the CLI's input is the source. A FlatBuffers-style
per-entry vtable would let a reader skip fields it does not know, but the fields it would skip are
kinds and operands whose meaning it cannot skip; the indirection would buy tolerance of changes the
format does not make.

Why the total length in the header: it is the truncation check, and the first thing a reader
compares against the bytes it was handed.

### D2. Sections

**Strings** (`0`): `count`, then `count + 1` byte offsets into the blob that follows, then the blob,
UTF-8, padded to four bytes. String `i` is `blob[offsets[i]..offsets[i + 1]]`. The offsets are
non-decreasing, and every offset is a character boundary of the blob.

**Module** (`1`): the runtime ABI as a string index; the required features as a count and string
indices; the module table as a count and, per module, identity and version string indices and the
fingerprint as two `u32`, low word first; then the function entrypoints and the component
entrypoints, each a count and declaration indices.

**Types, constants, nodes, declarations** (`2` to `5`): each is `count`, then `count + 1` cell
offsets into the pool that follows, then the pool of `u32` cells. Entry `i` is
`pool[offsets[i]..offsets[i + 1]]`, and its first cell is its kind. The layouts in
`docs/nx-ir-format.md` map onto cells by six rules, which are the whole encoding:

| Operand | Cells |
| --- | --- |
| `str`, `type`, `const`, `node`, `decl`, `slot`, an operator, a `0`/`1` flag or a flags word | one |
| an optional `node`, `slot` or span offset | one; absent is `0xFFFFFFFF`, spelled `none` |
| `ref` | two: module slot, name string |
| `ref?` | two; absent has `none` in the module-slot cell |
| `[x...]` | a count, then each element by its own rule, recursively |
| a constant's value | `int` and `float` are two cells, low word first, of the `i64` or the `f64` bits; `bigint` is one `str` cell |

So the node `[18, ref, [[str, node]...], [node...]]` becomes `18, slot, name, n, (name, node) × n, m,
node × m`, and the declaration list, the type table and the corpus document read the same way they
do today. There is no per-entry arity cell: the offsets give every entry's length.

**Debug** (`6`): the declaration spans as a count and `(start, end)` pairs, the node spans likewise
with `none, none` where the JSON wrote `[-1, -1]`, then the source text as a byte length and UTF-8
bytes padded to four. The source is stored here rather than in the string table so that an image
with and without its debug section differs only in this section and in the directory, which the
corpus checks.

Why `u32` cells everywhere: a JavaScript reader is then one `Uint32Array` view per pool and one
`Uint8Array` view per blob, both O(1) to create, and an entry is a subarray. A varint pool would be
half the size and would put a decode loop back in front of every read; the sizes in the proposal's
appendix say the difference after gzip is a fifth, and transport compression is not this format's
job. Why two cells for an `i64`: a JavaScript reader reconstructs any value in the safe range
exactly from the two words, and a Rust reader reads the `i64` back exactly; a single `f64` cell
would have forced the reader to know which was which.

### D3. Two Rust shapes, one file

`ir_image.rs` in `crates/nx-codegen` gets a writer and a reader. `write_nx_ir_image(&NxIrArtifact)
-> Vec<u8>` flattens the `IrItem` trees by the rules in D2. `NxIrImage::open(&[u8]) -> Result<NxIrImage<'_>, NxIrImageError>`
validates the bytes (D4) and answers with a view: `string(i) -> &str`, `entry(table, i) -> &[u32]`,
the module table, the entrypoints and the debug spans, all borrowed. `NxIrImage::to_artifact()`
rebuilds the owned `NxIrArtifact`, so `ir_explain.rs` and the emitter's tests keep working on the
model and a round trip through the image is one assertion.

`IrItem` stays. The proposal noted that deserializing it through serde's untagged enum cost 4x; that
cost was the JSON reader's, and the JSON reader is gone. As the builder's output the trees are the
shape the layouts are written in, and rewriting the 2,800-line builder to emit cells directly would
change a great deal of code to save an allocation the compiler makes once. The serde derives on the
IR model, `NxIrFormat` and `serialize_nx_ir` are deleted; `GeneratedNxIr.json` becomes
`GeneratedNxIr.bytes`.

Why the view and the owned model both: the view is what a Rust runtime, or the CLI opening a
large library, wants, and it is where validation lives; the owned model is what the existing
explainer and tests consume. The owned decode is written over the view's accessors, so there is one
interpretation of the bytes in Rust.

### D4. One eager validation pass at open, in both readers

A reader validates before it answers any question: the magic, the version, the length against the
bytes given, the directory (every section inside the image, aligned, required kinds present once),
each table's offsets non-decreasing and ending at its pool length, the string offsets likewise with
the blob valid UTF-8 and every offset on a character boundary, and then every entry of every table
by its kind's layout — the kind known, every operand index inside the table it names, every
reference's slot inside the module table, every optional cell either `none` or in range. The
TypeScript runtime does the last part today in `TableReader.validateNode` and `declaration`; the
pass moves onto cells and gains the offset checks in front of it. After the pass, evaluation reads
cells without further checks, in JavaScript because a typed-array read cannot escape its buffer and
in Rust because slicing stays checked and cheap.

Why eager: the runtime already touches every declaration and node at prepare, so the pass adds the
offset checks and nothing else, and it keeps evaluation free of per-read branches. The lazy
alternative, checking each access, spreads the cost over what is touched and would let a wild
string offset surface as a diagnostic in the middle of evaluation; eager makes "prepare succeeded"
mean "every read is in bounds". Lazy string *decoding* is unaffected: validating a string range is a
comparison of two offsets, not a decode, and the blob's UTF-8 check is one linear pass over bytes
that gzip would have read anyway.

A malformed image is a diagnostic from `tryPrepareNxIrModule` and an `Err` from `NxIrImage::open`,
never an exception or a panic. Both readers are tested by truncating each corpus image at every
four-byte boundary and by flipping each cell of the smallest one, asserting a refusal every time.

### D5. The TypeScript runtime reads a `Uint8Array`

`prepareNxIrModule(bytes: Uint8Array | ArrayBuffer)` replaces the `string | NxIrArtifact` input;
`NxIrArtifact` and the `NxIrItem` types leave the public API, and `NxPreparedModule.artifact`
becomes the opened image. When the view's byte offset is not a multiple of four the reader copies
the bytes into a fresh buffer first, since `Uint32Array` requires alignment; a host that hands over
an `ArrayBuffer` or a fresh `Uint8Array` pays nothing. Strings decode through one `TextDecoder` on
first use and are memoized per index, so the catalog's 505 names are decoded as they are named.
Fingerprints are read as `BigInt` and exposed as the decimal strings the API exposes today.

The runtime's tests replace their `ArtifactBuilder.build()` with an image writer of about a hundred
lines that follows D2, so a test still states an artifact by hand. That writer is the test suite's
and is not exported.

### D6. Bytes across the SDK boundaries: one bundle framing

`generateNxIr` answers with `{ identity, bytes, metadata }` per artifact. The wasm ABI and the C
FFI both return one payload per call, so both carry a *bundle*: a `u32` header length, a JSON
header `[{ identity, metadata, offset, length }]`, padding to four bytes, then the images
back to back at the offsets the header gives. The wasm loader and the .NET binding each read the
header and slice the images; the wasm loader copies each image out of the module's memory before
freeing the result. The Node binding returns napi objects whose `bytes` is a `Buffer`, since napi
has the type. The parity tests compare `bytes` for equality across the wasm and Node SDKs.

Why a bundle rather than a call per artifact: the emit options select several modules at once, the
result record already is a pointer and a length, and the header is the JSON the bindings parse
today with two integers added. MessagePack would have been natural for the FFI, which already
speaks it, but the wasm package has no decoder and should not gain a dependency for one call.

### D7. Explain is Rust, reached from every host

`explain_nx_ir_image(&[u8]) -> Result<String, ExplainError>` opens the image (D4) and renders it
through the existing explainer. The CLI's `nxlang ir explain` calls it. The wasm SDK exposes
`NxHost.explainNxIr(bytes)`, the Node SDK `explainNxIr(bytes)`, the FFI `nx_ir_explain` and .NET
`NxRuntime.ExplainNxIr(byte[])`, each returning the same text. A browser devtools session on a page
that has the SDK — the fiddle's editor, the playground — can read any artifact it holds; the
fiddle's player does not load the SDK by design, and a share opened there is read by pasting its
base64 into the editor page.

Why not a renderer in TypeScript: the spec requires the same text from every host, and a second
implementation is a second reading of the tables to keep in step, which is what removing JSON is
for. The wasm module is already on every page that compiles.

### D8. The corpus commits bytes and text

`expected/<identity>.nxir` and `expected/<identity>.stripped.nxir` replace the JSON files, and
`expected/<identity>.nxir.txt` and `.stripped.nxir.txt` hold the explained text of each, all four
written by `NX_UPDATE_CORPUS=1`. The pinning test compares bytes and reports a mismatch as the
explained difference, as it does now; a second test checks that each committed text equals the
explanation of its committed image, so the text cannot go stale. `.gitattributes` marks `*.nxir`
binary. The size budget stays six times the source; the ratios rise by about 1.4x and the largest
today becomes about 4.2x.

### D9. Hosts carry the image as bytes, or as base64 where a string is forced

The CLI writes `<identity>.nxir` per module; a person reads it with `nxlang ir explain`. The
playground's Vite plugin exports the catalog image from its virtual module as a base64 string
decoded on first use, so `catalogModule()` stays synchronous and nothing in the render path learns to
wait; the worker posts the compiled snippet as a `Uint8Array` and transfers its buffer. The fiddle's
share becomes `const IR = "<base64>"` and `mount` decodes it with `Uint8Array.fromBase64` where the
engine has it and `atob` otherwise; the bundled catalog is committed as `drawnui.nxir` and the build's
staleness check compares bytes.

Why base64 in the playground bundle rather than an asset: an asset would save a third of the bytes
on a cached, same-origin file at the price of an asynchronous catalog and a loading state in the
renderer. That trade can be made later without touching the format.

## Risks / Trade-offs

- [The six-times budget is missed by a future corpus program] → The ratios are known: at most 4.2x
  after this change. The budget test names the program and the ratio, and the levers the linkable
  design listed remain.
- [A host hands the runtime an unaligned view and pays a copy every prepare] → The copy is
  documented on `prepareNxIrModule`, and the SDKs and the fiddle hand over fresh buffers.
- [A binding's bundle parser drifts from the wasm loader's] → Both are ten lines over one documented
  framing, and the parity test compares the images they produce byte for byte.
- [Explained text in the corpus goes stale against its image] → A test regenerates it from the
  committed image and compares.
- [A malformed image slips past validation and a read is out of range] → Every table access after
  open is still a bounds-checked typed-array read in JavaScript and a checked slice in Rust, so the
  failure is a wrong diagnostic rather than a wild read. The truncation and flip tests exercise every
  cell of the smallest corpus image.
- [`BigInt` and `Uint8Array.fromBase64` availability] → `BigInt` is in every engine the runtime
  supports. `fromBase64` is used when present with an `atob` fallback, as measured in the proposal.
- [The fiddle branch built for schema 3 JSON needs reworking before its pull request] → It has not
  been opened; the rework is base64 in the share and bytes in the bundle, both listed in tasks.

## Migration Plan

1. Land this change on the `linkable-compact-nx-ir` branch: `cargo test --workspace`,
   `pnpm -r test` and the .NET tests green.
2. Rewrite `docs/nx-ir-format.md` for the image and update the READMEs.
3. Tag the release `linkable-compact-nx-ir` was waiting on, now with the image; archive the linkable
   change and then this one.
4. Fiddle: rework the `nx-linkable-ir` branch for bytes and base64 before its pull request opens.

Rollback: no artifact in either schema 3 encoding has been published, so rolling back is reverting
the branch.
