## Context

See proposal.md for motivation. The facts that shape the design:

- `runtime/typescript/src/index.ts` (about 4,000 lines) is the one existing IR runtime, and
  `openspec/specs/typescript-ir-runtime/spec.md` states its behavior. `specs/ir-conformance` holds
  16 programs with committed images and expected results recorded from `nx-interpreter`.
- `crates/nx-codegen/src/ir_image.rs` already validates and reads an image in place
  (`NxIrImage::open`). It depends on the constants, the `kinds` module and the artifact model at the
  top of `ir.rs`, which are plain data; the rest of `ir.rs` is the emitter and depends on
  `nx-api`, `nx-hir`, `nx-interpreter` and `nx-types`.
- `Cells` reads cells from a byte slice with `from_le_bytes`, so a Rust reader has no alignment
  requirement and never copies the image.
- `nx_value::NxValue` distinguishes `Int32` from `Int` and `Float32` from `Float`. The IR erases
  the integer distinction (`docs/nx-ir-format.md`, *Erasure*) and makes `float32` arithmetic
  explicit through its own operator codes.
- The TypeScript instance holds handlers as objects that reference the linked module and the
  prepared declaration. Everything a handler needs is nameable: a module identity, a declaration
  name, a node index and captured values.

## Goals / Non-Goals

**Goals:**

- A Rust host runs any image a JavaScript host can, and gets the same canonical values, handler
  tokens and diagnostic codes.
- The runtime's dependency closure contains no part of the compiler.
- No input — a damaged image, a hostile host value, a deserialized instance — causes a panic or a
  stack overflow.
- An instance can be stored and used by a later process that prepares the same images.

**Non-Goals:**

- Sharing code with `nx-interpreter`. The corpus and the differential tests hold the two together
  until `retire-hir-interpreter` removes one.
- Matching the TypeScript runtime's API names or shape beyond the operations it offers.
- A typed view of rendered output for a renderer. The Skia integration reads `NxValue` and can add
  one when it knows what it needs.
- Beating the TypeScript runtime's speed. The design avoids obvious waste; measurement is later
  work, and the workloads of `add-ir-runtime-performance-harness` are the place to do it.

## Decisions

### 1. Two crates: `nx-ir` for the format, `nx-ir-runtime` for evaluation

`nx-ir` takes `ir_image.rs`, `ir_explain.rs`, and from `ir.rs` the schema, ABI and feature
constants, `kinds`, `IrItem`, `NxIrArtifact`, `NxIrModuleEntry`, `NxIrDebug` and `NxIrDebugSpans`.
It has no dependencies: the explainer's JSON string escaping and its spelling of a function type
and of an occurrence suffix, which came from `serde_json` and `nx-hir`, are small enough to repeat.
`NxIrArtifact::metadata` returns an emitter-side type and stays in `nx-codegen` as a free function.
`ir_bundle.rs` stays in `nx-codegen`: it bundles emitter output.

`nx-codegen` does not re-export the moved items. The project has no compatibility constraint, and
one import path is simpler than two.

*Alternative: put the reader inside `nx-ir-runtime`.* Rejected: the emitter writes images and the
CLI explains them, so both would depend on the runtime.

### 2. A prepared module owns its bytes and decodes declarations on first use

`PreparedModule::prepare` takes anything that becomes an `Arc<[u8]>`, validates the whole image
once with the existing validator, and keeps the validated section layout so that views over the
bytes are rebuilt without validating again. That needs one addition to the reader: `NxIrImageBuf`,
which owns the bytes with the layout validation found and hands out an `NxIrImage` view on request.
The layout itself stays private to `nx-ir`, so it cannot be paired with other bytes.

Validation is eager because the format requires it. The type table and the declaration list are
decoded in `prepare` too: linking, the by-name index and the host boundary all need them, and they
are small. The node table, which is the bulk of an image, is decoded lazily: each node becomes an
owned `Node` enum, holding its children as indices, the first time evaluation reaches it, cached in
a `OnceLock` per node. A snippet linked against a large catalog pays for the controls it uses.

Preparation also checks what the reader's layouts do not say: a `text` node names a type with a
text form, a presence operator or `{}` pattern appears only with `occurrence-v1`, declarations are
unique, local references resolve and entrypoints have the right kind. The references an image
holds come from one more addition to the reader, `NxIrImage::for_each_reference`.

`PreparedModule` and `Program` are `Send + Sync` and shared through `Arc`, so one prepared catalog
serves many programs on many threads. Linking never copies a module.

*Alternative: evaluate straight from cells.* Rejected: every evaluation would re-read operand
layouts, and a `match` over a decoded enum is both faster and easier to read.
*Alternative: decode every node in `prepare`.* Rejected: it makes preparing a catalog cost the
decoding of every body rather than of what is used.

### 3. One internal value type; `NxValue` only at the boundary

Inside the evaluator a value is `Bool`, `Int(i64)`, `Float(f64)`, `Str`, `Seq`, `Record`, `Case`,
`Function` or `Handler`, with sequences and records behind `Arc` so that captures and state are
cheap to clone. The empty value is the empty sequence, as everywhere in NX. A constant union case
and a function stay distinct from a string and a record inside the evaluator and become their
canonical forms only at the boundary; `retire-hir-interpreter` needs that distinction to print a
value as NX text.

At the boundary:

- An integer site accepts `NxValue::Int32` and `NxValue::Int`, and an integral float as the
  integer it is, since JSON cannot spell the difference between `3` and `3.0`; a float site accepts
  `Float32`, `Float` and either integer variant, as the TypeScript runtime accepts any `number`.
  An `int32` site checks its range, as in the TypeScript runtime.
- Results are written as `NxValue::Int` and `NxValue::Float`. A value whose static type is
  `float32` is written as the `f64` it is carried in, which is what the TypeScript runtime returns.
- `NxValue::Null` is read as the empty value where the site admits zero, and written only for a
  cleared field of an update record.

Integer arithmetic is `i64`, wrapping, which is what `primitive-type-names` says integer arithmetic
does until checked arithmetic lands. An `int32` item is therefore not wrapped at 32 bits. That
matches both JavaScript backends and differs from `nx-interpreter` at the top of the `int32` range;
the format document already records this divergence, and `retire-hir-interpreter` resolves it.

Outside JavaScript's safe range the two runtimes cannot agree, and this one does not try to: a
JavaScript number loses precision past 2^53 and the TypeScript runtime refuses an `nx.int` operand
with `nx-ir-number`, where `i64` arithmetic is exact to 2^63 and wraps beyond. Reproducing the
`f64` behavior would mean giving up exact 64-bit integers to match a limitation of the other
host. The spec carves this out, the format document records it, and a test pins it.

The host's `null` becomes the empty value as it is read, before any type is known, because the
runtime has one representation of nothing. At an `object` site, which takes any value, a `null` is
therefore the empty list, where the TypeScript runtime rejects a top-level `null` and keeps a
nested one. The spec and the format document record this too.

A `bigint` constant is parsed into an `i64`; one that does not fit is a malformed image. So an
integer outside JavaScript's safe range is an `NxValue::Int`, where canonical JSON and the
TypeScript runtime spell it `{ "$type": "nx.int", "value": "<digits>" }`. The wrapper is accepted
at an integer site, and the corpus comparison treats the two spellings as one value.

Because the carriers differ, operators choose by operand: `add`, `sub`, `mul` and `mod` are integer
operations on two integers and float operations otherwise, `div` is always float division, and
`idiv` and `imod` take integers or integral floats. Equality and ordering compare an integer and a
float by value, so `3 == 3.0` as in JavaScript.

### 4. Results, not panics, and a depth budget on every recursion

Every API returns `Result<_, NxIrRuntimeError>`, where the error holds a list of diagnostics with
`code`, `message`, `declaration` and an optional span. The codes are the TypeScript runtime's
strings. There is no `try`/throwing pair as in TypeScript; `Result` is the non-throwing form.

A native stack overflow aborts the process and cannot be turned into a diagnostic, so the evaluator
bounds its own recursion and fails with `nx-ir-resource-limit` at a limit. `max_call_depth` defaults
to 100, the TypeScript default, but it is the host's to raise, so it cannot be what protects the
stack. Two fixed bounds do that: a count of nested nodes across every call of one evaluation
(1,000), and a budget on the native stack actually used (1 MiB), measured from the address of a
local where the evaluation began. The count alone is not enough because the size of a frame is not
known: an unoptimized build's frames are several times an optimized one's, and 1,000 levels
overflowed a 2 MiB test thread before the budget was added. The frames every level pays for hold
only the dispatch; each node kind's work is in a function of its own.

More bounds close what an image or a host can otherwise reach: a value crossing the host
boundary nests at most 256 deep, since the walks over a value recurse; component state is held to
the same bound after every patch of a batch, since a batch can nest it one level per entry and
nothing else looks at it until the batch is over; checking a value against a declared type is
under the stack budget, since that walk's frames are large; a type nests at most 64
types, since a type is dropped recursively; and a frame holds at most 65,536 slots, since a slot is
a cell of the image and a frame is allocated to its highest.

The crate denies `unwrap`, `expect`, `panic!`, slice indexing and unchecked integer arithmetic
through clippy lints in non-test code, and the damage tests are the proof.

The runtime does not depend on `nx-diagnostics`: that crate brings Ariadne, and a host that wants
rendered source excerpts can map the span itself.

### 5. An instance is serializable data

`ComponentInstance` holds the component's name, normalized props, handler props, state, the
handlers of the last rendered output in token order, the generation, and the identity and image
hash of every module of the program that produced it. A handler holds module identity,
declaration name, the index of its handler node, the component, emit, action and owner it answers
to, and the captured values. All of it is names, indices and values.

The image hash is FNV-1a over the image's bytes, computed when the module is prepared. The
module's fingerprint would not do: it hashes the source, and a handler names its node and its
captured slots by index, so two images emitted from one source by two compiler releases, or with
and without the debug section, are different programs to an instance.

`ComponentInstance` implements `Serialize`, so the host chooses the format; the runtime depends on
`serde` and on no format crate. The serialized form is not the in-memory one. It is a flat table
of values in which an entry names the values it holds by index, always an earlier entry, and each
shared value is written once. Every handler of a rendered list captures the same list, so writing
values in place would make the form's size the number of handlers times the data they captured;
and a flat table nests a fixed few levels, so a deserializer with a recursion limit reads any
instance, where the nested form cost three levels per level of value. A handler is stored as its
module, declaration, node and capture only; the rest is read from the node again on restore.

An instance is validated once, when it is restored, not each time it is used. Restoring goes
through one function, `Program::restore_component_instance`, that takes a `serde` deserializer and
checks everything: the image hashes must match the program, an entry of the table must name only
earlier entries, nest at most 1,024 levels and hold at most 2^24 values written out (an entry
may name one earlier entry many times, so without the second bound a table of a few dozen entries
names a value of exponential size), every function must resolve, every node index must
name a handler node that the declaration it claims reaches from its body or a default, the props
and the state must normalize against the component's fields with nothing defaulted, and each
handler property must be a handler for an emit of the component. A mismatch is a diagnostic, so an
instance from another program revision, or bytes someone edited by mistake, is refused rather than
run. The check does not authenticate: a rewritten instance that is still a valid one, such as one
with two tokens exchanged, is accepted, and a host that needs more signs what it stores. There is no other way to
build an instance from outside the runtime — the type has no public constructor and no public
`Deserialize` — so every instance a caller can hold either came from the runtime or passed that
check.

Dispatch and child initialization therefore trust the instance's contents and compare only its
image hashes with the program's, a few integer comparisons, which catches an in-memory instance
handed to the wrong program. A host that keeps instances in memory, as a renderer does, never pays
for a walk over the handler table.

*Alternative: validate on every dispatch.* Rejected: it is a walk over every handler on every
interaction, to re-check data the runtime itself produced a moment earlier.

The TypeScript runtime promises that a handler a child receives from its parent is the same object
the parent holds. In Rust the handler is an `Arc`, so `Arc::ptr_eq` answers that in memory, and
handlers also compare equal structurally, which is what survives serialization.

*Alternative: keep instances in-memory only for now.* Rejected: `retire-hir-interpreter` needs a
snapshot the bindings can hand to hosts, and adding serialization later would reshape the handler
type.

### 6. The prelude image is generated beside the TypeScript one

`prelude_image_tests.rs` writes `crates/nx-ir-runtime/src/prelude.nxir` in the same run that
writes `runtime/typescript/src/prelude-image.ts`, and its staleness test compares both. The runtime
embeds the file with `include_bytes!` and prepares it at most once per process.

### 7. Where the tests live

- `crates/nx-ir-runtime/tests/corpus.rs` reads `specs/ir-conformance` from disk and needs nothing
  from the compiler: entrypoints, lifecycles, both variants, every four-byte truncation of every
  image, and the cell-overwrite run of every image, which runs the entrypoints and lifecycles of
  whatever prepares and round-trips each instance through its serialized form. A further test
  reads the manifests to check that the runtime's dependency closure names no compiler crate.
- `crates/nx-ir-runtime/tests/prepare_link.rs` covers preparation and linking over corpus images
  and images altered through `nx-ir`: an unknown feature, another ABI or schema, a missing feature,
  each link failure, the prelude, sharing across threads and the nesting bound.
- The boundary and lifecycle scenarios of the spec are tested from NX source, which reads better
  than a hand-built table and is what `retire-hir-interpreter` needs anyway. Those tests need the
  compiler, so they live in `nx-codegen` (`ir_runtime_tests.rs`), with `nx-ir-runtime` as a
  dev-dependency. `runtime/typescript/test/runtime.test.ts` builds its artifacts by hand; its cases
  were the checklist, not the code.
- The differential test lives beside them. It runs every source of
  `runtime/typescript/test/emitted-ir.test.mjs`, extracted into `ir_runtime_sources.rs`, and a
  source exercising every arithmetic and comparison operator, through both engines: each function
  entrypoint, and each component initialized and then evaluated from explicit state. Dispatch is
  compared through the corpus, whose lifecycle results the interpreter recorded.

Comparison with recorded results is numeric-aware — `3` and `3.0` are one canonical value — and
ignores key order, as the existing corpus comparison in `ir_corpus_tests.rs` does.

## Risks / Trade-offs

- [The TypeScript runtime's behavior is larger than its spec: 4,000 lines against 23 requirements]
  → Port from the source, treat the corpus and the ported unit tests as the contract, and add a
  corpus program for any behavior found only in the TypeScript code.
- [Three engines must now agree until the interpreter is retired] → The corpus runs in all three on
  every build; the retirement change follows directly.
- [A diagnostic's message text may drift between runtimes] → The spec fixes the code and what the
  message names, not its wording; tests assert codes and named items.
- [Lazy decoding hides a malformed declaration until it is used] → It cannot: validation is eager
  and complete, and decoding a validated entry has no failure a caller can cause.
- [Moving the format changes import paths in `nx-cli`, `nx-ffi` and both native bindings] → Mechanical, done in one commit before
  any runtime code, with the existing test suites as the check.

## Open Questions

- Whether to publish `nx-ir` and `nx-ir-runtime` to crates.io, and under which names. Nothing in NX
  is published there today, and the first consumer is in-tree or path-dependent.
- Whether the built-in prelude should be behind a default-on Cargo feature for hosts that always
  supply their own.
