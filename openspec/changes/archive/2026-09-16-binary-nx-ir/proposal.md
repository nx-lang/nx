## Why

NX IR is JSON because JSON was readable and because `JSON.parse` was assumed to beat any decoder a
JavaScript host could write. After `linkable-compact-nx-ir`, neither premise holds.

The readability went first. Schema 3 encodes a module as flat tables of integer operands, so a
node reads `[18,1,8,[[6,3],[7,4]],[]]` and a person needs `nxlang ir explain` to know it is a
`SkiaLabel` with two properties. The format is already unreadable; JSON now buys nothing but bytes.

The performance premise turns out to be wrong too. `design.md`'s D3 argues that `JSON.parse` is
"faster than a hand-written binary decoder in JavaScript". That is true of a varint decoder, which
is byte-at-a-time interpreted work. It is not true of a zero-copy image read through typed arrays,
because `new Uint32Array(buf, off, n)` is O(1) and `u32[i]` compiles to a single load. There is
nothing to decode. Measured on a real share and on the DrawnUI catalog, in Node 24 and in Rust
1.98.1, against an identical finish line — every string accounted for, every node kind read, a
name-to-declaration index built, cross-checked by assertion in both languages:

| Getting from bytes to a usable module | `layouts` share | `catalog` |
| --- | --- | --- |
| JavaScript, `JSON.parse` + index | 137.9 µs | 216.2 µs |
| JavaScript, zero-copy image, strings eager | 16.9 µs | 40.8 µs |
| JavaScript, zero-copy image, strings lazy | **2.9 µs** | **8.3 µs** |
| Rust, `serde_json` to `serde_json::Value` | 219 µs | 548 µs |
| Rust, `serde_json` to the typed model | 711 µs | 1,491 µs |
| Rust, varint tables to owned pools | 17.8 µs | 33.9 µs |
| Rust, zero-copy image, read in place | **0.8 µs** | **2.9 µs** |

The lazy row is the format's real advantage rather than the raw speed: the catalog holds 505 strings
and a client that names three controls needs three of them. JSON has to materialize all 505 before
it can look up one.

Even the case JSON was kept for — the fiddle's share, which travels inside a JavaScript module
string and so would have to be base64 — comes out ahead, because base64 decoding is native SIMD code
in every current engine:

| `layouts`, embedded in a JavaScript module | |
| --- | --- |
| `JSON.parse` + index | 116.6 µs |
| base64 decode alone | 2.4 µs |
| base64 decode + zero-copy read | **5.1 µs** |

The portable `atob` plus `charCodeAt` path, for engines without `Uint8Array.fromBase64`, is 36.2 µs
— still three times faster than JSON, at 2.04x the characters.

Parsing is a small share of what a host pays today — on a real share the TypeScript runtime spends
about 126 µs parsing against 1,040 µs evaluating — so the microseconds are not the reason to do
this. The reasons are structural. One encoding read the same way by the TypeScript runtime, the
Rust tooling and a later Rust or wasm runtime is one reader to keep in step, which is the drift the
conformance corpus exists to catch. A zero-copy image is what a persistent precompiled-library
cache needs, and that cache is where the milliseconds are (see *Sequencing*). And schema 3 has not
been released: `linkable-compact-nx-ir` is implemented on this branch but untagged, so making the
image the encoding now means schema 3 ships once, as an image, and no consumer ever holds a schema
3 JSON document.

## What Changes

### The zero-copy image is the only representation of NX IR

A little-endian, four-byte-aligned image replaces the JSON document. Strings become an offset table
plus one blob, so a name is a borrowed slice rather than an allocation. Each table becomes an offset
array plus a flat `u32` operand pool, so entry `k` is reachable without reading entries before it.
The kind numbers, the layouts and the debug section survive unchanged in meaning; only their
spelling moves. `format`, `schemaVersion` and `runtimeAbi` become a fixed header, and a reader that
does not recognize the header refuses the artifact as it does today. The schema version stays `3`,
because no schema 3 artifact has been published in any encoding.

Nothing about linking, module tables, `[slot, name]` references or the value model changes. This is
an encoding change to the format `linkable-compact-nx-ir` defined, not a redesign of it.

### Readability moves into tooling, and tooling becomes load-bearing

`nxlang ir explain` already renders an artifact with every index resolved and reads close to NX
source. Going binary-only promotes it from a convenience to the only way to read an artifact, which
it is not yet built to be:

- It reads JSON today and must read the image.
- It must survive a malformed artifact without panicking. `line_column` panicked on a non-character-
  boundary offset until this week, which is the class of bug that matters much more once `explain`
  is the only window onto the bytes.
- It must be reachable from JavaScript, because a browser devtools session is exactly where someone
  will need to read an artifact and will not have `nxlang`. The renderer stays one Rust
  implementation, exposed through the wasm SDK and the other bindings; a second renderer written in
  TypeScript would be the drift this change exists to remove.

There is no text-to-image assembler. The explained text is a rendering with no grammar, and giving
it one would mean a second textual format with its own parser and documentation. The needs an
assembler would have served are met more cheaply: a test that states an artifact by hand uses a
small image writer in the test suite, as the runtime's tests already use a builder; a malformed
artifact is easier to make from bytes than from text; and a damaged artifact is repaired by
recompiling its source, which every host stores beside it.

### The corpus keeps its reviewability

The corpus pins artifacts byte for byte, and today a pull request diff names the table entry that
moved. A binary corpus diffs to nothing. Each corpus program therefore commits both: the image as
the pinned bytes, and its explained text as a sibling, both written by `NX_UPDATE_CORPUS=1`, so the
bytes are the contract and the text is what a reviewer reads. `explained_difference` in
`ir_corpus_tests.rs` already renders that text on failure.

### Untrusted artifacts are validated before they are read

JSON gave structural checking away for free: a malformed document failed in `serde` or in
`TableReader` with a diagnostic. An image does not. The fiddle plays shares written by strangers, so
an attacker-controlled offset is a wild read — bounds-checked and merely wrong in JavaScript,
genuinely unsafe in Rust or wasm if indices are trusted for speed. A reader therefore validates in
one linear pass that every offset, string range and table index is in bounds before it evaluates
anything. The TypeScript runtime already walks every declaration and node at prepare time, so this
pass is a change of what it walks, not a new cost; the measurements above validate nothing and are a
floor, not a promise.

## Capabilities

### Modified Capabilities

- `nx-ir-format`: the artifact is a binary image rather than a JSON document; the tables, kinds,
  module table, reference shape and debug section keep their meaning; artifacts are validated
  structurally before use; the conformance corpus commits explained text beside each image.
- `typescript-ir-runtime`: preparation reads an image through typed arrays and decodes a string
  only when it is named, rather than parsing a document.
- `sdk-wasm`: emitted IR is bytes; the compact-JSON requirement is removed; the SDK explains an
  artifact as the CLI would.
- `sdk-node`, `dotnet-binding`: emitted IR is bytes, byte-identical across the bindings.
- `cli-code-generation`: the `nx-ir` target writes images; `ir explain` reads the image and is
  specified to report rather than crash on a malformed artifact.

## Impact

**`crates/nx-codegen`** — the emitter writes an image instead of serializing `NxIrArtifact` through
serde. The `IrItem` trees stay as the builder's output; their serde derives and the compact/pretty
`NxIrFormat` go. A new module writes an image from the model and opens one as a validated view over
the bytes, from which `ir_explain.rs` reads. `ir_corpus_tests.rs` pins images and writes explained
text beside them.

**`runtime/typescript`** — `TableReader` becomes a typed-array reader over an `ArrayBuffer`, with
`prepareNxIrModule` taking bytes rather than text or a parsed object. Lazy string decoding is the
reason to do it, so the API should not force every name to be materialized.

**`bindings/*`** — `generateNxIr` returns bytes rather than a JSON string, through one bundle
framing shared by the wasm ABI and the C FFI. The parity test compares byte for byte, more simply
than it does now. Each binding exposes `explainNxIr`.

**`sites/playground`** — the Vite plugin bundles the catalog image, and the worker transfers an
`ArrayBuffer` rather than structured-cloning an object.

**Fiddle repository** — the share carries base64 and pays 2.04x the characters. The corpus ratios
today run 1.5x to 3x the source against the six-times budget, so the largest preset's share stays
far under 128 KB. Moving the artifact to a file of its own would remove base64 altogether; that is
the fiddle's call.

## Decisions taken in `design.md`

The proposal's open questions are answered in the design rather than left for implementation:
the compatibility policy is rigid entry layouts behind a versioned header, with a section
directory as the one extension point (D1); validation is one eager pass at open in every reader
(D4); the image carries no name index yet, because a section can add one later without a redesign
(D1, D7).

## Sequencing

This change lands on the `linkable-compact-nx-ir` branch before that change's release is tagged, so
the packages publish once with the image. Its OpenSpec deltas are written against the requirements
`linkable-compact-nx-ir` introduces, and the two changes archive in that order.

Two siblings depend on the container or are freed by it, and neither belongs here.

**Persistent precompiled libraries** is the reason the container matters most, and it needs content
this format does not carry. Analyzing the DrawnUI catalog from source costs 6.37 ms; compiling a
one-line snippet against it costs 6.72 ms, of which the snippet is 0.007 ms. Library re-analysis is
99.9% of that build, and nothing survives a process exit today — `LibraryRegistry` memoizes within a
process and `PreparedModuleCache` within a build. A persistent cache would remove nearly all of it.
But NX IR cannot be that cache as it stands, because it is an execution format and is lossy in four
places a front end needs: a type alias is `[5, str]` and does not record what it aliases; type
parameters are erased by `erase_type_parameters` before a prop's type reaches the type table;
function types collapse to the `object` primitive; and no declaration carries an export bit
anywhere, so the image cannot answer the first question import resolution asks. That work is a
metadata section beside the IR — unerased `nx_types::Type`, visibility, alias targets, declaration
spans — sharing this container's section directory and keyed by the fingerprint the module table
already carries.

**Runtime evaluation performance** is independent of the format and is the larger number. Profiling
the TypeScript runtime on a real share shows evaluation at 1,040 µs against 126 µs to parse, 49 µs
to prepare and 0.7 µs to link. A structural clone of the same result tree — 243 objects, 6,711
properties, 609 strings — costs 150 µs, so roughly 890 µs is dispatch, normalization and
intermediate-object churn rather than allocation anyone is forced to pay. `evalReference`
re-evaluates a top-level `let` on every reference with no memoization, so a value used inside a
`for` body is recomputed per iteration, and `linkable-compact-nx-ir`'s design already names closure
compilation at load time, in the non-goal that rules out a bytecode, as available without a format
change. That is worth doing, and it needs none of this.

## Not in this change

Not in this change is a **wasm NX runtime**. It is worth revisiting once the JavaScript evaluator has
been optimized, and the case for it splits by host rather than being general. Where the value tree
must become JavaScript objects for a renderer, wasm still pays the boundary — the 150 µs clone floor
is a lower bound on materializing 243 objects, and crossing from wasm is worse than a JavaScript-side
clone — so a plausible outcome is 1,180 µs becoming 400 to 600 µs, bought with a few hundred
kilobytes of module on every page. Where the host serializes instead of rendering, such as a
Cloudflare Worker answering with bytes, the tree never becomes JavaScript objects at all: that
deletes the boundary and the 427 µs `JSON.stringify` with it, and the win is nearer tenfold. The
correctness argument is the stronger one either way — a runtime built from `nx-interpreter` collapses
the two implementations the corpus exists to keep in step — but it moves the seam to wasm-to-host
marshalling rather than removing it, and a browser would still want the JavaScript runtime for size.
Note only that going wasm forces this change: a wasm runtime loses the host's native `JSON.parse` and
would be running `serde_json` compiled to wasm, slower than the JavaScript path it replaced.

Not in this change is the **metadata section** for precompiled libraries, described under Sequencing.

Not in this change is a **name index** in the image. About 1.1 µs of the catalog's 2.9 µs Rust open
is the `HashMap` build, so a declaration list sorted by name would let a reader binary search the
bytes and build nothing. It is a section to add when a reader that opens many modules per request
exists; the TypeScript runtime builds prepared-declaration objects at open regardless.

Not in this change is **compression**. Gzip closes most of the size gap in both directions — 7,691
bytes for the share as JSON against 10,082 for the image, 5,617 against 6,794 for the catalog — so
transport encoding is a deployment choice rather than a format one.

## Appendix: measurements

Node 24.15.0 and Rust 1.98.1, one WSL2 machine, warm cache, means over 200 to 2,000 repetitions.
Artifacts: `layouts`, the largest playground example, 25,490 B compact JSON, 1,047 nodes; `catalog`,
the DrawnUI catalog emitted by `emitCatalogArtifact`, 55,896 B compact JSON, 505 strings, 82
declarations. Every decode path returns `{string_bytes, kind_sum, named}` and the harness asserts
all paths agree, in both languages; the assertion caught two real bugs in the harness while these
numbers were being taken. Ratios are the signal, not the absolute microseconds.

| Encoding | `layouts` raw | gzip | `catalog` raw | gzip |
| --- | --- | --- | --- | --- |
| Compact JSON | 25,490 | 7,691 | 55,896 | 5,617 |
| Varint tables, keyless | 14,661 | 7,279 | 28,197 | 5,524 |
| Zero-copy image | 38,956 | 10,082 | 79,728 | 6,794 |

Encoding, from the in-memory model to bytes, in Rust: compact JSON 26.8 µs and 65.2 µs, varint
44.9 µs and 54.9 µs, image 44.6 µs and 72.5 µs. Encoding happens once in a compiler that has already
spent milliseconds on analysis, so the asymmetry favours the reader, which is the right way round.

The image measured here is a sketch. It carries a redundant per-entry arity cell, about 4 KB of
`layouts`' 38,956, and performs no structural validation. Both make the real numbers worse than
these and the sizes better; the layout in `design.md` drops the arity cell, since a table's offset
array already gives every entry's length.
