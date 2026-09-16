# Review: binary-nx-ir

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md` (20/20 marked complete), and the six
spec deltas (`nx-ir-format`, `typescript-ir-runtime`, `sdk-wasm`, `sdk-node`, `dotnet-binding`,
`cli-code-generation`).

**Reviewed code:**

- `crates/nx-codegen/src/ir_image.rs` (writer, validating reader, decoder, tests),
  `ir_bundle.rs`, `ir_explain.rs`, `ir_corpus_tests.rs`, `ir.rs`, `lib.rs`
- `crates/nx-cli/src/main.rs` (`ir explain`, the `nx-ir` target, `.nxir` path derivation)
- `crates/nx-ffi/src/lib.rs`, `bindings/wasm/native/src/lib.rs`, `bindings/node/native/src/lib.rs`
- `runtime/typescript/src/index.ts` (image open/validation, `TableReader`, layouts),
  `runtime/typescript/test/corpus.test.mjs`
- `bindings/wasm/src/host.ts` + `sdk-wasm.test.ts` + `parity.test.ts`, `bindings/node/src/*` +
  `sdk-node.test.ts`, `bindings/dotnet/src/NxLang.Sdk/NxProgramArtifact.cs` + `NxEndToEndTests.cs`
- `sites/playground` (`vite.config.ts` catalog plugin, `src/render/catalog.ts`,
  `src/worker/nx.worker.ts`, `src/worker/session.ts`)
- `docs/nx-ir-format.md`, the three READMEs, `specs/ir-conformance/`

**Verification run during this review:** `cargo test --workspace` (green),
`pnpm -r test` across all nine workspace packages (green), `dotnet test bindings/dotnet/NxLang.sln`
(136 passed). Task 4.4's claim holds.

The change is in good shape. The format, the two readers, the bundle framing and the corpus are
carefully built and well documented, and the byte-level pinning across Rust, Node, wasm and .NET is
a genuinely strong contract. One real defect is below, plus smaller cleanups.

## Findings

### ✅ Verified - RF1 The Rust reader accepts a cyclic node or type graph, and `ir explain` then overflows the stack and aborts the process

- **Severity:** High
- **Evidence:** `docs/nx-ir-format.md:105` states the invariant as part of the format — "A node
  refers to its children by index; a child always precedes its parent in the table … so no node
  reaches itself" — and `docs/nx-ir-format.md:86` says the same for an `array`/`nullable` type's
  inner type. The TypeScript runtime enforces it: `runtime/typescript/src/index.ts:1047-1051`
  narrows the bound to the entry's own index (`nodes: table === "nodes" ? index : …`). The Rust
  validator does not: `crates/nx-codegen/src/ir_image.rs:912-920` builds one `Bounds` from the full
  table counts and `validate_op` checks `Op::Node`/`Op::OptNode`/`Op::Type` against it, so a node
  may name itself or any later node.

  `Explainer::node` (`crates/nx-codegen/src/ir_explain.rs:375`) and `Explainer::ty`
  (`:178`, `:191-192`) recurse with no depth cap and no visited set, so the accepted cycle is
  unbounded recursion. Reproduced: taking the committed
  `specs/ir-conformance/snippet/expected/input.nx.stripped.nxir` and rewriting node 13's first
  child cell from `5` to `13` yields an image that `NxIrImage::open` accepts and that kills the CLI:

  ```
  $ nxlang ir explain cyclic.nxir
  thread 'main' has overflowed its stack
  fatal runtime error: stack overflow, aborting
  Aborted (core dumped)   # exit 134
  ```

  The same bytes are correctly refused by the TypeScript runtime
  (`nx-ir-malformed: node 13 node index 13 is out of range`), so the two readers also disagree about
  what a valid image is — the drift this change exists to remove.

  A stack overflow is an abort, not a panic, so `panic::catch_unwind` at
  `crates/nx-ffi/src/lib.rs:978` does not contain it: `NxRuntime.ExplainNxIr` takes the .NET host
  process down with it, `explainNxIr` traps the wasm module, and the Node addon aborts Node. This
  contradicts three requirements of the change: *CLI explains an NX IR artifact as readable text*
  ("SHALL never panic on any input"), *An NX IR reader validates an artifact before it evaluates it*
  ("no reader SHALL throw an exception or panic"), and *The SDK explains an NX IR artifact*
  ("SHALL be reported as an SDK error … rather than as a trap"). It matters most where the proposal
  says it matters — the fiddle plays shares written by strangers, and `explain` is now the only way
  to read one.

  The corpus damage test misses it because its four probe values (`0`, `1`, `NONE`, `0x7FFF_FFF0`)
  almost never coincide with a cell's own entry index.
- **Recommendation:** In `ir_image.rs`, thread the entry index into validation the way the
  TypeScript reader does: when validating `Table::Nodes` entry `i`, bound `Op::Node` and
  `Op::OptNode` by `i`; when validating `Table::Types` entry `i`, bound `Op::Type` by `i`. That
  restores parity with the TypeScript reader, enforces the invariant the document already states,
  and removes the recursion hazard structurally rather than papering over it. Add a regression test
  that pins a self-referencing node cell as `Err`, and extend
  `every_cell_can_be_damaged_without_a_panic` to also probe each cell with its own entry index.
  Consider a depth cap in `Explainer` as defence in depth, since it is the only consumer that
  recurses over untrusted structure.
- **Fix:** `validate_table` in `crates/nx-codegen/src/ir_image.rs` now narrows `Bounds.nodes` to the
  entry's own index when validating the node table and `Bounds.types` when validating the type table,
  matching the TypeScript reader. Three tests were added: `a_self_referencing_node_is_refused` pins
  node 13's first child set to `13` (and to `14`) as `Err`, `every_node_and_type_cell_can_name_its_own_entry_without_a_panic`
  probes every node and type cell of every corpus image with its own entry index, and the corpus
  round-trip tests confirm every emitted image already satisfies the bound. The reproduction now
  answers `Error: NX IR artifact is malformed: node 13: node index 13 is out of range (the table has
  13)` with exit code 1. `docs/nx-ir-format.md`'s *Validation* section states the bound. No depth cap
  was added to `Explainer`, since the validator now guarantees termination structurally.
- **Verification:** Confirmed. `validate_table` (`crates/nx-codegen/src/ir_image.rs:1006-1013`) now
  rebuilds `Bounds` per entry with `nodes: if table == Table::Nodes { index }` and the matching
  clause for types, so every node and type operand is bounded by its own entry index — the same
  rule `runtime/typescript/src/index.ts:1047-1051` applies. Re-ran the original reproduction against
  a rebuilt CLI: the image that previously aborted the process now prints
  `Error: NX IR artifact is malformed: node 13: node index 13 is out of range (the table has 13)`
  and exits `1`. Bounding by the entry's own index makes every forward reference invalid, not just
  self-reference, so a cycle is unreachable by construction and the explainer's two recursive walks
  (`Explainer::node` over children, `Explainer::ty` over inner types) strictly decrease — declining
  the depth cap is the right call. The three new tests are real: `a_self_referencing_node_is_refused`
  pins both the self-reference and a forward reference, and
  `every_node_and_type_cell_can_name_its_own_entry_without_a_panic` probes every node and type cell
  of every corpus image with its own index through `open_damaged`, which also runs the explainer.
  `docs/nx-ir-format.md`'s *Validation* section states the bound. `cargo test --workspace` green
  (0 failures), `pnpm -r test` green, `dotnet test` 136 passed — so the tightening rejects nothing
  the emitter produces.

### 🔴 Open - RF2 Tasks 5.1 and 5.2 are marked complete, but the fiddle work is uncommitted

- **Severity:** Medium
- **Evidence:** `tasks.md` marks 5.1 (commit the catalog as `nx/catalog/drawnui.nxir`, byte
  comparison in the staleness check, base64 in `prepare`) and 5.2 (`moduleFor` emitting
  `const IR = "<base64>"`, `mount` decoding it) as done. In `~/src/DrawnUi.FiddleEngine` on branch
  `nx-linkable-ir`, the newest commit is `02fa142`, which is the `linkable-compact-nx-ir` work; the
  image rework sits entirely in the working tree — `nx/catalog/drawnui.nxir.json` deleted,
  `nx/catalog/drawnui.nxir` untracked, and `dev/build-nx-runtime.mjs`, `nx/catalog-artifact.mjs`,
  `nx/emit-catalog.mjs`, `nx/src/runtime.ts`, `nx/test/runtime.test.ts` all modified but uncommitted.
  Nothing in this review verified that the fiddle's `npm run runtime` or its preset budget test
  passes, because the packages those tasks depend on are not published yet.
- **Recommendation:** Either commit the fiddle branch so the ticked tasks match a durable state, or
  reword 5.1/5.2 to say what is actually done and what waits on the published packages. As it
  stands the work is one `git checkout` away from being lost and the change reads as fully applied
  when a fifth of it is not committed anywhere.
- **Status:** Left open after two fix passes. The recommendation's second branch is applied: a note
  under task 5.2 in `tasks.md` says the work sits uncommitted on the fiddle's `nx-linkable-ir` branch
  and waits on the published packages. The first branch, committing in `~/src/DrawnUi.FiddleEngine`,
  is a change to a different repository and is the user's call, so it was not done unprompted.
  Recommend a WIP commit there before this change is archived, so the ticked tasks match a durable
  state.
- **Verification:** Still open, correctly. The note is present at `tasks.md:35` and reads accurately.
  Re-checked `~/src/DrawnUi.FiddleEngine`: `nx-linkable-ir` is still at `02fa142` with 13 uncommitted
  paths, so the risk the finding describes is unchanged. Nothing further to verify here — this is a
  decision for the user, not a defect.

### ✅ Verified - RF3 Two READMEs still describe NX IR as JSON, which task 4.3 said to check for

- **Severity:** Low
- **Evidence:** Task 4.3's verification is "no README mentions IR JSON text or the compact/pretty
  choice". `bindings/node/README.md:9-10` still reads "Use the pure TypeScript IR runtime when
  JavaScript only needs to execute an already persisted NX IR JSON document." `bindings/wasm/README.md:36`
  summarises the ABI as "JSON in and JSON out over a hand-written loader", which the same file then
  contradicts at `:204-207` where it correctly documents the bundle and the byte arguments.
- **Recommendation:** Change the Node sentence to "an already emitted NX IR image", and qualify the
  wasm summary ("JSON in and JSON out, except the IR calls, which carry bytes — see *SDK boundaries*").
- **Fix:** `bindings/node/README.md` now says "an already emitted NX IR image", and
  `bindings/wasm/README.md`'s build summary now reads "JSON in and JSON out over a hand-written loader,
  except the NX IR calls, which carry an image's bytes (see *ABI* below)".
- **Verification:** Confirmed. `bindings/node/README.md:9-10` now reads "execute an already emitted
  NX IR image", and `bindings/wasm/README.md:36-37` carries the qualification, which no longer
  contradicts the *ABI* section at `:204-207`. Re-ran the leftover sweep across every `.md`, `.ts`,
  `.rs`, `.cs` and `.mjs` file: no README mentions IR JSON text or the compact/pretty choice, so
  task 4.3's stated verification now holds. (The sweep did surface one remaining `nx-ir-json`
  citation outside the READMEs — see RF7.)

### ✅ Verified - RF4 The damage tests leave two gaps: the debug section is never damaged, and no damaged image is ever evaluated

- **Severity:** Low
- **Evidence:** `every_cell_can_be_damaged_without_a_panic`
  (`crates/nx-codegen/src/ir_image.rs:1419`) selects `min_by_key(|(_, _, bytes)| bytes.len())`,
  which is always a stripped artifact, so no cell of a debug section — span offsets in particular —
  is ever overwritten. `line_column` (`ir_explain.rs:666-671`) does clamp and walk back to a
  character boundary, so the property holds today; nothing in the suite pins it.

  On the TypeScript side, `runtime/typescript/test/corpus.test.mjs:100-117` stops at
  `tryPrepareNxIrModule` and never evaluates an image it accepted, even though evaluation of a
  stranger's share is the scenario the *hostile artifact* requirement is written for. I checked this
  property by hand over `expressions/main.nx.stripped.nxir` — 2,166 damaged-but-accepted images,
  ~36k entrypoint evaluations — and every throw was the runtime's own `NxIrRuntimeError`, never a
  raw `TypeError` or `RangeError`. So this is a coverage gap, not a live bug.
- **Recommendation:** Pick the smallest image *with* a debug section as a second damage subject in
  the Rust test, and in `corpus.test.mjs` evaluate each entrypoint of a damaged image that prepared,
  asserting that anything thrown is an `NxIrRuntimeError`. Both are a few lines and they lock in
  behaviour that is currently only incidentally true.
- **Fix:** The Rust test now runs `damage_every_cell` over both the smallest corpus image and the
  smallest image with a debug section. `runtime/typescript/test/corpus.test.mjs` now picks the smallest
  image that owns a function entrypoint (`trailing-element/input.nx` stripped), links each damaged image
  that prepared against the intact sibling modules, evaluates each of its entrypoints, and fails on
  anything thrown that is not an `NxIrRuntimeError`. It reports 152 evaluations succeeding and 78
  failing with a runtime error, none throwing anything else.
- **Verification:** Confirmed on both sides. Running
  `cargo test -p nx-codegen every_cell_can_be_damaged -- --nocapture` prints two subjects, the second
  of which carries a debug section, so span offsets and the source length are now damaged too:

  ```
  snippet/drawnui.nx: 272 damaged images opened, 492 refused
  trailing-element/input.nx +debug: 471 damaged images opened, 669 refused
  ```

  `open_damaged` runs `to_artifact()` and the explainer on every image it accepts, so a damaged span
  reaches `line_column`. On the TypeScript side `runtime/typescript/test/corpus.test.mjs` now picks
  the smallest image owning a function entrypoint, links each damaged image that prepared against
  the intact sibling modules, and evaluates every entrypoint, failing on anything thrown that is not
  an `NxIrRuntimeError`. The suite reports `230 damaged images opened, 538 refused, none threw` and
  `152 entrypoint evaluations of damaged images succeeded, 78 failed with a runtime error, none threw
  anything else` — matching, at a different subject, the hand check in the original finding. The
  property is now pinned rather than incidental.

### ✅ Verified - RF5 The .NET bundle reader assumes a little-endian host and can escape its own error contract

- **Severity:** Low
- **Evidence:** `bindings/dotnet/src/NxLang.Sdk/NxProgramArtifact.cs`, `DeserializeGeneratedNxIr`,
  reads the framing's header length with `BitConverter.ToUInt32(payload, 0)`, which is host-endian.
  The bundle is defined as little-endian (`crates/nx-codegen/src/ir_bundle.rs:1-8`), so on a
  big-endian .NET target the length is read byte-reversed and every bundle is rejected. The same
  call appears in the tests at `NxEndToEndTests.cs:229-230` reading the image header.

  Separately, `checked((int)BitConverter.ToUInt32(payload, 0))` and the two `checked((int)…)` casts
  on `entry.Offset`/`entry.Length` throw `OverflowException`, which the surrounding
  `catch (JsonException e)` does not catch, so a malformed payload can surface as a raw
  `OverflowException` instead of the documented `InvalidOperationException`.
- **Recommendation:** Use `BinaryPrimitives.ReadUInt32LittleEndian(payload.AsSpan(0, 4))` in the
  reader and in the tests, and range-check `headerLength`/`offset`/`length` against `payload.Length`
  before casting rather than relying on `checked`.
- **Fix:** `DeserializeGeneratedNxIr` reads the header length with
  `BinaryPrimitives.ReadUInt32LittleEndian`, range-checks it as a `uint` against `payload.Length - 4`
  before the `int` cast, and drops the `checked` casts on offset/length since the existing end-of-image
  check already bounds both by `payload.Length`. The test's two header reads use the same
  little-endian primitive. 136 .NET tests pass.
- **Verification:** Confirmed. `DeserializeGeneratedNxIr` (`NxProgramArtifact.cs:338-345`) reads the
  header length with `BinaryPrimitives.ReadUInt32LittleEndian`, and `NxEndToEndTests.cs:230-231` uses
  the same primitive, so the reader no longer depends on host endianness. The overflow path is gone
  as well, and the reasoning holds: `headerLength > (uint)(payload.Length - 4)` is checked as a
  `uint` before the `int` cast (and `payload.Length >= 4` is established above it), and the
  `end > payload.Length` guard bounds both `Offset` and `Length` by `payload.Length` before either
  is cast, so neither cast can overflow and every rejection now leaves through `JsonException` into
  the documented `InvalidOperationException`. The comment at the guard states that invariant.
  `dotnet test bindings/dotnet/NxLang.sln`: 136 passed.

### ✅ Verified - RF6 A doc still cites the retired `nx-ir-json` / `.nxir.json` convention as precedent

- **Severity:** Low
- **Evidence:** `docs/drawn-ui-proposal-review.md:63,67` points at `nx-ir-format.md` as the
  established convention — "`format: "nx-ir-json"` plus `schemaVersion`" and "matches the existing
  `.nxir.json` extension convention" — to justify `nx-ui-json` and `.nxui.json`. Neither the key
  nor the extension exists after this change, so a reader following the citation finds nothing.
- **Recommendation:** One sentence noting that NX IR has since moved to a binary image with a magic
  header, and that the `nx-ui-json` naming stands on its own. The NX UI decision itself does not need
  revisiting; only the dangling justification does.
- **Fix:** `docs/drawn-ui-proposal-review.md` RF1 and RF2 now say the convention held at the time and
  that NX IR has since moved to a binary image with a magic header, so `nx-ui-json` and `.nxui.json`
  stand on their own.
- **Verification:** Confirmed for the two lines the finding named. `docs/drawn-ui-proposal-review.md:63`
  now reads "established the convention at the time … NX IR has since moved to a binary image with a
  magic header, so the `nx-ui-json` naming now stands on its own", and `:67` reads "matched the
  `.nxir.json` extension NX IR used before it became a binary `.nxir` image". Both put the citation
  in the past tense without rewriting the NX UI decision, which is the right shape for a historical
  review document. A sweep for the same citation found a third instance the original finding did not
  name, in a different file — tracked as RF7 rather than reopening this one.

## New Findings Discovered During 2026-09-16 01:12 Verification

### ✅ Verified - RF7 A third document still cites `nx-ir-format.md` as using `format` + `schemaVersion` JSON

- **Severity:** Low
- **Evidence:** RF6's fix corrected `docs/drawn-ui-proposal-review.md:63,67`, but a sweep for the
  same citation across every `.md` in the repository turns up
  `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md:138`, which describes the `format` field as
  "following the `format` + `schemaVersion` convention already used by
  [NX IR JSON](nx-ir-format.md)". That link now lands on a document describing a binary image with a
  magic header and no `format` key, so the justification dangles exactly as RF6's did. This is the
  proposal itself rather than its review, so the stale pointer is the more visible of the two.
- **Recommendation:** Apply the same past-tense treatment RF6 used: say the convention was NX IR's
  at the time and that NX IR has since moved to a binary image, leaving `nx-ui-json` standing on its
  own. No change to the NX UI object model is implied.
- **Fix:** The `format` row of §4.1 in `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` now says the
  convention was NX IR's when the proposal was written and that NX IR has since moved to a binary
  image with a magic header, so `nx-ui-json` stands on its own. A sweep of every `.md` in the
  repository outside `openspec/changes/` finds no fourth copy; the only remaining hits are RF6's two
  lines, which already read in the past tense.
- **Verification:** Confirmed. The `format` row of §4.1
  (`docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md:138`) now puts the NX IR convention in the past
  tense and says `nx-ui-json` stands on its own, with no change to the object model. A repository-wide
  sweep of `.md` files outside `openspec/` for `nx-ir-json`, `NX IR JSON` and `.nxir.json` finds only
  RF6's two lines in `docs/drawn-ui-proposal-review.md`, both already past tense.

### ✅ Verified - RF8 `docs/nx-ir-format.md` now understates the damage tests RF4 added

- **Severity:** Low
- **Evidence:** The *Validation* section closes with "Both the Rust reader (`NxIrImage::open`) and
  the TypeScript runtime (`prepareNxIrModule`) are tested by truncating every corpus image at every
  four-byte boundary and by overwriting every cell of the smallest one." After RF4 that is no longer
  what either suite does: the Rust test damages two images — the smallest and the smallest carrying
  a debug section — and the TypeScript test damages the smallest image that owns a function
  entrypoint and then links and evaluates every damaged image it accepted. RF1's fix also added
  a third Rust test that probes every node and type cell with its own entry index. The sentence
  describes less coverage than exists, so a reader trusting it would not know the debug section or
  the evaluation path is exercised.
- **Recommendation:** Rewrite the closing sentence to match: every corpus image truncated at every
  four-byte boundary; every cell of the smallest image and of the smallest image with a debug
  section overwritten; every node and type cell probed with its own entry index; and, in the
  TypeScript runtime, every damaged image that prepared linked and evaluated.
- **Fix:** The *Validation* section's closing sentence is now its own paragraph describing what each
  suite actually does: both truncate every corpus image at every four-byte boundary and overwrite
  every cell of the smallest image with four values; the Rust suite adds the smallest image with a
  debug section and probes every node and type cell of every image with its own entry index; the
  TypeScript suite takes the smallest image owning a function entrypoint and links and evaluates
  every damaged image it accepted. The own-index probe is attributed to Rust alone, since the
  TypeScript suite has no equivalent. The paragraph RF1 amended was also rewrapped, which it had
  left ragged.
- **Verification:** Still inaccurate in one clause, so reopened. `docs/nx-ir-format.md:297-299` says
  both suites overwrite "every cell of the smallest image with four values". The Rust suite does
  (`snippet/drawnui.nx`, stripped). The TypeScript suite does not: `corpus.test.mjs` has one damage
  subject (`corpus.test.mjs:103`), which is the smallest image owning a function entrypoint
  (`trailing-element/input.nx`, stripped), not the smallest image overall. Line 302 then describes
  that subject correctly, so the paragraph contradicts itself. The rest of the rewrite is correct:
  the debug-section subject and own-index probe are Rust-only, and TypeScript links and evaluates
  every damaged image it accepts. The RF1 paragraph was rewrapped cleanly.
  **Still needed:** make the shared sentence cover only what both suites do: truncate every image at
  every four-byte boundary. Move "every cell of the smallest image" into the Rust-only sentence. The
  TypeScript sentence can stay as it is, or say that its subject gets the same four overwrite values.
- **Fix:** The shared sentence in `docs/nx-ir-format.md`'s *Validation* section now says each suite
  overwrites every cell of "a chosen image" with four values. The Rust sentence names its two subjects
  (the smallest image and the smallest with a debug section), and the TypeScript sentence says it
  damages the smallest image that owns a function entrypoint, so the paragraph no longer contradicts
  itself.
- **Verification (second pass):** Confirmed. `docs/nx-ir-format.md`'s *Validation* section now says
  only what both suites do: truncate every image at every four-byte boundary, and overwrite every cell
  of "a chosen image" with four values. Both suites use the same four values: `0`, `1`, `0xFFFFFFFF`
  and `0x7FFFFFF0`. The Rust sentence names its two subjects, the smallest image and the smallest with
  a debug section, plus the own-index probe. The TypeScript sentence names its one subject, the
  smallest image with a function entrypoint, and says it links and evaluates what it accepts. Each
  sentence matches `ir_image.rs` and `corpus.test.mjs`, and the paragraph no longer contradicts
  itself.

## Questions

- RF1's fix changes what `NxIrImage::open` accepts. Is there any producer other than
  `write_nx_ir_image` — the fiddle's tooling, say — that could emit an image whose node or type
  operands are not already in dependency order? The corpus and every binding test suggest not, but
  it is worth confirming before the tightening lands.
  - **Answer:** No. `write_nx_ir_image` is the only writer in the repository, the fiddle emits through
    the SDKs, and every corpus image (with and without debug) still opens under the tightened bound.
- Design D6 says the Node binding does not use the bundle framing because napi has a byte type. That
  leaves three bundle readers (Rust, the wasm loader, .NET) rather than the two the risk list
  anticipated. They agree today, and the corpus pinning would catch a drift in the images; only the
  header shape is unpinned. Is that acceptable, or worth a shared fixture?

## Summary

A well-executed change. The image format is coherently specified and the document is genuinely
sufficient to hand-decode an artifact; the writer, validator and decoder in `ir_image.rs` walk one
shared `Op` table, which is the right structure and keeps the three passes honest with each other;
the corpus pinning bytes plus explained text gives reviewable diffs without giving up the byte
contract; and the byte-for-byte cross-binding checks (wasm↔Node parity, Node and .NET both pinning
the corpus) make the "identical across every SDK" requirement real rather than aspirational. All
three test suites are green as claimed.

RF1 is the one finding that should block: the Rust reader is missing a bound the TypeScript reader
has and the format document already states, and the consequence is a reachable process abort on
attacker-supplied bytes, through the one tool the change promotes to load-bearing. It is a small,
local fix. RF2 is a bookkeeping matter but a real one — a fifth of the ticked work exists only in an
uncommitted working tree in another repository. The rest are cleanups.

## Verification pass, 2026-09-16 01:12

RF1, RF3, RF4, RF5 and RF6 are all verified fixed. RF1's fix is the right one: bounding node and type
operands by the entry's own index enforces the invariant the format document already stated, restores
parity with the TypeScript reader, and makes the explainer's recursion terminate by construction
rather than by a cap — the reproduction that aborted the process now exits `1` with a diagnostic.
RF4's fix is better than what was recommended, since the TypeScript side now evaluates damaged images
rather than only opening them. `cargo test --workspace`, `pnpm -r test` and `dotnet test` (136) are
all green after the changes, so nothing the tightened validator rejects is anything the emitter
produces.

Nothing blocks the change now. RF2 remains open by decision, with an accurate note at `tasks.md:35`;
the fiddle's `nx-linkable-ir` branch is still at `02fa142` with 13 uncommitted paths, so committing
that working tree is still worth doing before the change is archived. RF7 and RF8 are documentation
nits found while verifying — one a third copy of the retired `nx-ir-json` citation, one a sentence in
`nx-ir-format.md` that now understates the tests RF1 and RF4 added.

## Verification pass, 2026-09-16 01:40 (RF7, RF8)

RF7 is verified. RF8 is reopened because one clause in `docs/nx-ir-format.md:297-299` says the
TypeScript suite damages the smallest corpus image, but it damages the smallest image with a
function entrypoint, as the same paragraph says three lines later. The fix is one sentence. RF2
still stands by decision: the fiddle's `nx-linkable-ir` branch is still at `02fa142` with 13
uncommitted paths. No new findings.

## Verification pass, 2026-09-16 01:55 (RF8)

RF8 is verified. Every finding except RF2 is now verified. RF2 stays open by decision until the fiddle
work on `nx-linkable-ir` is committed. No new findings.
