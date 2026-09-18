# Review: linkable-compact-nx-ir

## Scope

**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and all seven delta specs
(`nx-ir-format`, `typescript-ir-runtime`, `sdk-wasm`, `workspace-programs`, `cli-code-generation`,
`playground`, `fiddle-nx-language`).

**Reviewed code:** the working tree and untracked additions —
`crates/nx-codegen` (`ir.rs`, `ir_explain.rs`, `ir_tests.rs`, `ir_corpus_tests.rs`, `builder.rs`,
`model.rs`), `crates/nx-api/src/artifacts.rs` (implicit imports), `crates/nx-hir/src/declarations.rs`,
`crates/nx-cli/src/main.rs`, `specs/ir-conformance/`, `runtime/typescript/src/index.ts` and its tests,
`bindings/wasm` (`host.ts`, `types.ts`, tests), `bindings/node`, `bindings/dotnet`,
`sites/playground` (compile, render, worker, scripts, `vite.config.ts`), `docs/nx-ir-format.md`, and
the fiddle branch `nx-linkable-ir` in `~/src/DrawnUi.FiddleEngine` at a summary level.

**Test runs (all green):** `cargo test -p nx-codegen` (107 passed), `pnpm test` in
`runtime/typescript`, `pnpm test` + `check-examples` in `sites/playground`, `pnpm test` in
`bindings/wasm` (30) and `bindings/node` (20), `dotnet test bindings/dotnet/NxLang.sln`.

## Findings

### ✅ Verified - RF1 Every literal and identifier node in the debug section carries the span `[0, 0]`

- **Severity:** High
- **Evidence:** `build_expression` reads the span from the AST enum rather than the module's span
  map — [builder.rs:1018](crates/nx-codegen/src/builder.rs#L1018) does `let span = expr.span();`.
  `ast::Expr::Literal` and `ast::Expr::Ident` carry no span of their own (see the note at
  [lower.rs:271-279](crates/nx-hir/src/lower.rs#L271-L279)); their spans live in the span map that
  [`LoweredModule::expr_span`](crates/nx-hir/src/lib.rs#L1110) consults. The result is visible in the
  committed corpus: in `specs/ir-conformance/snippet/expected/input.nx.nxir.json`, node spans are
  `[[0,0],[0,0],[0,0],[0,0],[0,0],[206,244],[0,0],[249,333],[0,0],[338,363],[0,0],[0,0],[368,411],[165,427]]`
  — only the five component-descriptor nodes have real spans; every string, number, bool and
  reference node points at offset 0. `nx-ir-format`'s requirement *NX IR preserves source provenance
  for diagnostics and source maps* says the section holds "source spans for declarations and nodes"
  and that a runtime diagnostic "SHALL identify the originating source identity and span"; for a
  literal or a reference it identifies line 1, column 1. Task 2.4's verification passed because the
  runtime test that checks a span uses a binary node. The same loss reaches
  `nxlang ir explain`, whose span rendering is fed from this array.
- **Recommendation:** Use `lowered_module.expr_span(expr_id)` at
  [builder.rs:1018](crates/nx-codegen/src/builder.rs#L1018), regenerate the corpus with
  `NX_UPDATE_CORPUS=1`, and add a runtime test that reports a diagnostic on a *literal* operand
  (for example a boundary-type failure on a string property) and asserts a non-zero span. Note this
  predates the change; it becomes a defect here because the debug section is newly specified
  behavior and the corpus now pins the zeros as expected output.
- **Fix:** `build_expression` reads `lowered_module.expr_span(expr_id)`, which consults the span map
  before falling back to the node ([builder.rs](crates/nx-codegen/src/builder.rs)). The corpus was
  regenerated with `NX_UPDATE_CORPUS=1`: `snippet`'s node spans are now
  `[[136,149],[199,200],[182,190],[239,241],[223,228],[206,244],…]` with no `[0,0]` left.
  `explain_annotates_literal_and_identifier_nodes_with_where_they_were_written` in `ir_tests.rs`
  asserts no node span is `[0, 0]` and that the literal `41` sits at its own offsets. One correction
  to the evidence: `ir explain` does *not* render node spans — `span_suffix` is applied only to
  declaration spans — so the explain output was never affected.
- **Verification:** Confirmed. `build_expression` reads `lowered_module.expr_span(expr_id)`
  ([builder.rs:1018-1021](crates/nx-codegen/src/builder.rs#L1018-L1021)). I re-derived the spans
  from the regenerated corpus rather than trusting the note: `snippet`, `expressions`, `records` and
  `two-module` now have zero `[0, 0]` node spans between them (198 nodes), and slicing
  `debug.source` by each span in `two-module` yields `answer`, `answer()`, `1`, `answer() + 1`,
  `makeUser` — each node's own text. `explain_annotates_literal_and_identifier_nodes_...` asserts the
  literal `41` at `[17, 19]`. The correction about `ir explain` is right: `span_suffix` is called
  only from `declaration`.

### ✅ Verified - RF2 A record field inherited from another module records that module's offsets against this module's source

- **Severity:** Medium
- **Evidence:** The emitter tracks span provenance per component field —
  [ir.rs:1251](crates/nx-codegen/src/ir.rs#L1251) sets `self.span_module = field.owner_module_id`
  before emitting a component field's default, and [`span`](crates/nx-codegen/src/ir.rs#L1119)
  returns `[-1, -1]` for a foreign module. `record_fields`
  ([ir.rs:1269](crates/nx-codegen/src/ir.rs#L1269)) has no equivalent, because
  `CodegenRecordField` ([model.rs:172-181](crates/nx-codegen/src/model.rs#L172-L181)) has no
  `owner_module_id` — even though `build_effective_record_fields`
  ([builder.rs:964](crates/nx-codegen/src/builder.rs#L964)) already resolves `field.module_identity`
  to an owner module and then drops it. A record extending an abstract base declared in another
  module, where the base's field has a default, therefore writes the *base module's* byte offsets
  into an artifact whose `debug.source` is the deriving module's text. RF1 currently masks this for
  literal defaults; fixing RF1 makes it live. The corpus does not cover it: `two-module`'s
  `shared/model.nx` declares `abstract type Base = { id:int }` with no default.
- **Recommendation:** Add `owner_module_id` to `CodegenRecordField` the way `CodegenComponentField`
  has it, set `span_module` in `record_fields`, and extend the `two-module` corpus program with a
  base field that has a default so the artifact pins the `[-1, -1]`.
- **Fix:** `CodegenRecordField` gained `owner_module_id`
  ([model.rs](crates/nx-codegen/src/model.rs)), set from the owner `build_effective_record_fields`
  already resolves; `record_fields` sets `span_module` from it and restores the module's own id
  after the fields ([ir.rs](crates/nx-codegen/src/ir.rs)). The `two-module` corpus program now
  declares `abstract type Base = { id:int = 7 }` and `<Local label="x" />` leaves `id` to that
  default, so the inherited default is both evaluated and pinned: the first node span of
  `app__main.nx.nxir.json` is `[-1, -1]`.
- **Verification:** Confirmed. `CodegenRecordField.owner_module_id` exists
  ([model.rs](crates/nx-codegen/src/model.rs)) and `record_fields` sets `span_module` per field and
  restores the module's own id after the list ([ir.rs](crates/nx-codegen/src/ir.rs)) — the restore
  matters, since the fields are emitted before the declaration's own span is taken. In the
  regenerated `app__main.nx.nxir.json`, node 0 is the inherited `id = 7` default and its span is
  `[-1, -1]`, while every other node has a span that slices to its own text, so the fix is pinned in
  both directions.

### ✅ Verified - RF3 `line_column` panics when an offset is not a UTF-8 character boundary

- **Severity:** Medium
- **Evidence:** [ir_explain.rs:661-671](crates/nx-codegen/src/ir_explain.rs#L661-L671) clamps the
  offset to `source.len()` and then slices `&source[..offset]`. Clamping to the length does not make
  an offset a char boundary, so any span that does not line up with one panics the CLI with
  `byte index N is not a char boundary`. The reachable path today is RF2 — a foreign module's offset
  landing mid-character in this module's text — but the function should be total for any artifact,
  including a hand-edited or third-party one, since `ir explain` is the support tool for exactly
  those.
- **Recommendation:** Walk back to the nearest boundary (`while !source.is_char_boundary(offset)
  { offset -= 1; }`) or use `source.get(..offset)` with a fallback, and add a test that explains an
  artifact whose span falls inside a multi-byte character.
- **Fix:** `line_column` walks back to the nearest character boundary after clamping
  ([ir_explain.rs](crates/nx-codegen/src/ir_explain.rs)).
  `explain_reads_a_span_that_falls_inside_a_character` explains an artifact whose declaration span
  starts inside a multi-byte character and ends past the end of the source, and asserts it reads as
  `@1:18-1:25`.
- **Verification:** Confirmed. `line_column` walks back to the nearest boundary after clamping
  ([ir_explain.rs](crates/nx-codegen/src/ir_explain.rs)). The new test is a genuine regression test:
  it sets the span to `[18, 1_000_000]` where offset 18 is the second byte of the `é` at 17, which
  the previous `&source[..offset]` would have panicked on.

### ✅ Verified - RF4 A single-case union is generated without `export`, so the implicit import cannot see it

- **Severity:** Medium
- **Evidence:** `nxUnionDeclaration` in
  [generate-catalog.mjs:316-324](sites/playground/scripts/generate-catalog.mjs#L316-L324) gained
  `export` on the one-line and multi-line branches but not on the single-case branch at
  [line 317](sites/playground/scripts/generate-catalog.mjs#L317), which still returns
  `` `type ${union.name} = | ${union.cases[0]}\n` ``. The catalog is now its own module reached
  through an implicit import, which sees only exports, so the next regeneration that yields a
  one-case enum produces a type no control prop can name. The current `catalog/skia.nx` happens to
  contain no single-case union, so the bug is latent and nothing fails today.
- **Recommendation:** Add `export` to the single-case branch. Consider asserting in
  `check-examples` or a catalog test that every top-level declaration in `catalog/skia.nx` starts
  with `export`, so the next generator change cannot reintroduce this silently.
- **Fix:** the single-case branch emits `export`
  ([generate-catalog.mjs](sites/playground/scripts/generate-catalog.mjs)), and `catalog.test.mjs`
  gained "exports every declaration the catalog holds", which fails on any top-level line of
  `catalog/skia.nx` that starts a declaration without `export`.
- **Verification:** Confirmed. The single-case branch emits `export`
  ([generate-catalog.mjs:317-318](sites/playground/scripts/generate-catalog.mjs#L317-L318)), and the
  new test's regex covers every form the generator produces (`abstract `, `external `, `type `,
  `let `, `component `). Note the guard is on the committed catalog, so it would not have caught the
  original bug while no single-case union existed — it catches it at the moment a regeneration
  introduces one, which is when it matters. The direct fix is what closes this.

### ✅ Verified - RF5 Any explicit import of an implicitly imported identity, including a named or aliased one, cancels the implicit wildcard

- **Severity:** Medium
- **Evidence:** `synthesize_implicit_imports` collects `explicit_targets` from every import in the
  module regardless of `ImportKind`
  ([artifacts.rs:1162-1176](crates/nx-api/src/artifacts.rs#L1162-L1176)) and skips the synthesized
  wildcard for any identity already there. A document that writes
  `import { SkiaLabel } from "./skia.nx"` or `import * as d from "./skia.nx"` therefore loses every
  other catalog name, with no diagnostic explaining why. The `workspace-programs` spec says "An
  implicit import SHALL behave exactly as the written wildcard import would: the same names in
  scope, the same ambiguity diagnostics" — with a written wildcard alongside a named import, the
  other names stay in scope. The playground compiles arbitrary visitor source, so this is reachable.
  The existing test
  [`an_explicit_import_of_the_implicit_module_is_not_a_repeated_import`](crates/nx-api/src/artifacts.rs#L5851)
  covers only the alias-less wildcard.
- **Recommendation:** Suppress the synthesized import only when the module already has an
  alias-less wildcard import of that identity; let the normal ambiguity rules handle the rest. Add
  tests for a named import and an aliased wildcard of the implicit module.
- **Fix:** the finding is real, though narrower than described: a *component* of the implicit module
  still resolves — element names reach a workspace peer through the graph — while a function or
  value does not, which is what the two new tests exercise. The recommendation alone does not work:
  NX allows one import per module identity, so synthesizing a second one makes the module report
  "'drawnui.nx' is imported more than once". Two changes together: `synthesize_implicit_imports`
  now skips only an alias-less wildcard of the identity, and `apply_graph_imports` exempts a
  synthesized import — recognizable by its empty span, which no written import has — from the
  duplicate-import check, binding its names instead of reporting it. Added
  `a_named_import_of_the_implicit_module_leaves_its_other_names_in_scope` and
  `an_aliased_wildcard_import_of_the_implicit_module_leaves_its_other_names_in_scope`, which build
  and *evaluate* `<SkiaButton Text={hint()} />` against a catalog whose other names the explicit
  import does not list. All nine implicit-import tests pass, as do the wasm SDK's and the
  playground's.
- **Verification:** Confirmed, and I checked the two things that could have gone wrong. First, that
  the new tests regress: reverting only the
  `matches!(import.kind, ImportKind::Wildcard { alias: None })` filter to accept every import makes
  `a_named_import_of_the_implicit_module_leaves_its_other_names_in_scope` and
  `an_aliased_wildcard_import_of_the_implicit_module_leaves_its_other_names_in_scope` fail while the
  other seven implicit-import tests still pass, so they pin exactly this fix and nothing else.
  Second, that the duplicate-import exemption did not swallow the real diagnostic: a workspace whose
  `main.nx` writes both `import { answer } from "./lib.nx"` and `import "./lib.nx"` still reports
  "Module or library 'lib.nx' is imported more than once in this file; first imported at line 1,
  column 1". The empty-span heuristic is a little implicit — nothing stops a future recovery path
  from producing a zero-length import span — but no written import can have one today, and the
  synthesized import is always appended last, so a written import is always the entry that seeds the
  map. On the reconstructed `artifacts.rs`: its diff against HEAD is now 348 lines against the 283 I
  read before the fix pass, which is the original feature plus this fix and two tests; the six
  `workspace-programs` scenarios each still have a passing named test, and the wasm, Node and
  playground suites that drive the feature end to end all pass. The suggestion to read that file's
  diff anyway is worth taking.

### ✅ Resolved - RF6 The npm package versions are still `0.1.0` (not a defect: the release stamps them)

- **Severity:** Medium
- **Evidence:** `@nx-lang/ir-runtime`, `@nx-lang/sdk-wasm` and `@nx-lang/language-core` are all at
  `0.1.0`, and `git tag` shows `v0.1.0` is already cut and published — with schema 2 and
  `buildProgramWithPrelude`. `package-publish.yml`
  ([line 168-169](.github/workflows/package-publish.yml#L168-L169)) throws when a package's
  `version` does not equal the release version rather than stamping it, so tagging `v0.2.0` fails
  until the manifests are bumped. The fiddle branch `nx-linkable-ir` pins `"@nx-lang/ir-runtime":
  "0.1.0"` and `"@nx-lang/sdk-wasm": "0.1.0"` in `nx/package.json`, which today resolve to the
  schema-2 packages. The proposal's Impact section lists these package versions as part of the
  change; tasks 6.5 and 7.1 are correctly still open, but the bump itself is a repo edit that has
  not been made.
- **Recommendation:** Bump the three manifests (and any workspace package that depends on them) to
  the next minor before tagging, and update the fiddle's pins in the same pass as task 7.1.
- **Status:** Resolved, not a defect — no manifest bump is needed, and making one would change
  nothing. `scripts/pack-packages.mjs` writes the release version into *every* publishable
  manifest before running `pnpm pack`, then restores the files, so the committed `0.1.0` is never
  what ships. `package-publish.yml`'s check reads the version out of the packed tarball, which that
  stamp already set; it verifies the stamp rather than rejecting the source. The version comes from
  the git tag, so `v0.2.0` will publish `0.2.0` with the manifests untouched. The real remaining
  work is the fiddle's pins, which is task 7.1 and correctly still open. The proposal's Impact
  section claimed the change edits these package versions; it now says what actually happens.
- **Verification:** Confirmed, and my original finding was wrong.
  [`scripts/pack-packages.mjs:46`](scripts/pack-packages.mjs#L46) writes `{...manifest, version}`
  into every publishable `package.json` before `pnpm pack`, and
  [package-publish.yml:167-170](.github/workflows/package-publish.yml#L167-L170) reads the version
  back out of the packed tarball (`tar -xOf … package/package.json`), so it verifies that stamp
  rather than the committed manifest. `release.yml:224` passes the tag's version to the pack script,
  so `v0.2.0` publishes `0.2.0` with the manifests untouched. The proposal's Impact bullet now says
  this. The fiddle's `0.1.0` pins remain, correctly, as task 7.1.

### ✅ Verified - RF7 Linking walks every declaration of every linked module on every link

- **Severity:** Low
- **Evidence:** `tryLinkNxIrProgram` rebuilds the nominal-shape index from scratch for each linked
  program — [index.ts:938-946](runtime/typescript/src/index.ts#L938-L946) iterates
  `modulesByIdentity.values()` and calls `nominalShapesOf(linked)`, which allocates one
  `NominalShape` per record and per non-constant union case of *every* module, including the
  catalog. Design D7 states "it never copies a prepared module, so one prepared catalog serves any
  number of programs" and the goal is "linking a snippet to it is cheap"; in practice each link is
  O(catalog declarations). The `typescript-ir-runtime` scenario "a host links a hundred snippet
  artifacts against the same prepared `drawnui`" is satisfied only in the weak sense that
  *preparation* happens once. The DrawnUI catalog is the largest module in play and the fiddle links
  on every mount.
- **Recommendation:** Precompute the per-module shape skeletons on `NxPreparedModule` at prepare
  time, storing `bases` as `(slot, name)` pairs, and resolve them to `identity::name` keys lazily
  through the `LinkedModule` at use. Add a test that links the same prepared module into two
  programs and asserts the shapes are not rebuilt (for example by identity on a memoized array).
- **Fix:** taken as recommended. `prepare` indexes each module's own shapes once into
  `NxPreparedModule.nominalShapeSkeletons`, whose `bases` stay as the artifact's `(slot, name)`
  references. `NxPreparedProgram.nominalShapesByDiscriminator` is replaced by
  `nominalShapesFor(discriminator)`, which reads one map per linked module — the modules of a
  program are few, however many declarations they hold — and resolves each base through the module
  that wrote it. Linking now allocates nothing per linked declaration, and the dead
  `nominalShapesOf` walker is gone. `linking reads the prepared module's shape index rather than
  rebuilding it` links ten programs against one prepared catalog, asserts the module's
  `declarations` array is never read, and asserts each linked shape reuses the very field list
  preparation built. Design D7 was reworded to describe this. 115 runtime tests pass, along with
  the playground, both SDKs and `check-examples`.
- **Verification:** Confirmed, and it is the recommended shape. `indexNominalShapes` runs once per
  module at preparation, keyed by discriminator with `bases` left as `(slot, name)`;
  `nominalShapesFor` does one map lookup per linked module and materializes only the matching
  shapes, resolving each base through the module that wrote it. Linking allocates nothing per
  declaration, and `nominalShapesOf` is gone. The new test is a real guard, not a smoke test: it
  wraps the prepared module in a getter that counts reads of `declarations`, links ten programs, and
  asserts the count is zero and that each linked shape's `fields` is the identical array preparation
  built. One consequence worth knowing: shapes are now allocated per `nominalShapesFor` call rather
  than once per link, but the callers are cold paths (`resolveSubtype`, only when a derived value
  meets a base-typed site, and `changedFields`), and each call allocates per matching shape rather
  than per declaration.

### ✅ Verified - RF8 The runtime rewrite dropped the two tests that guard the behaviors the multi-module design makes more reachable

- **Severity:** Low
- **Evidence:** `runtime/typescript/test/runtime.test.ts` went from 27 tests to 18. Two of the
  removed ones have no replacement anywhere: *"reports a subtype whose name two declarations share
  rather than guessing"*, which covered the ambiguity branch of
  [`resolveSubtype`](runtime/typescript/src/index.ts#L1638), and *"the exported helpers reject
  mismatched targets"*, which covered the `$type` checks in `applyRecordUpdate`,
  `mergeUpdateRecords` and `diffRecordValues`
  ([index.ts:1591-1612](runtime/typescript/src/index.ts#L1591-L1612)). Both behaviors are still
  implemented and both matter more under schema 3 than before: `nominalShapesByDiscriminator` is
  explicitly documented as keying a bare name to several shapes ("two modules may each declare a
  record of one name"), and the helpers compare `$type` strings that are bare names. No corpus
  program has two modules declaring one name, so nothing exercises the collision at all.
- **Recommendation:** Restore both tests against schema-3 fixtures, and add a corpus program in
  which the entry and a linked module each declare a record of the same name, with an entrypoint
  that passes one where the other's base is expected.
- **Fix:** both behaviors are covered again, built with the test file's `ArtifactBuilder` rather
  than a corpus program — the collision is a linking property, and the builder states it in one
  test instead of a four-file fixture. `reports a subtype whose name two modules share rather than
  guessing` links `left.nx` and `right.nx`, each declaring `Card extends base.nx::Base` with a
  different field, and asserts the boundary reports "2 declarations named 'Card' extend Base"
  rather than picking one. The helper test gained the two missing assertions, so `mergeUpdates` and
  `diffRecords` are checked for mismatched targets alongside `applyUpdate`.
- **Verification:** Confirmed, and the builder was the right call over a corpus program. The
  ambiguity test constructs the collision properly — `left.nx` and `right.nx` each declaring `Card`
  with a distinct field, both extending `base.nx::Base`, all three linked into one program — and
  asserts the boundary reports "2 declarations named 'Card' extend Base" rather than picking one.
  The helper test now asserts all three mismatches: `applyUpdate` ("only 'User.Update' patches it"),
  `mergeUpdates` and `diffRecords` ("the records have different types").

### ✅ Verified - RF9 The CLI writes an artifact for every workspace module, not only the ones the entry references

- **Severity:** Low
- **Evidence:** `generate_executable_nx_ir` uses `NxIrEmitOptions::every_module_with_debug()`
  ([main.rs:474](crates/nx-cli/src/main.rs#L474)), which emits every module of the program. The test
  at [main.rs:1608-1611](crates/nx-cli/src/main.rs#L1608-L1611) documents the consequence with the
  comment "the other module is still part of the workspace and gets its own file". The
  `cli-code-generation` scenario *Workspace codegen writes NX IR for selected entry* says the CLI
  writes an artifact "for `app/main.nx` and one for each module it references, directly or
  transitively"; the requirement body says "one per module of the program", so the requirement and
  its scenario disagree and the implementation follows the body.
- **Recommendation:** Pick one. Either restrict the emit set to the entry's reachable modules, or
  reword the scenario to say every module of the workspace program. Leaving both texts as they are
  will re-open this on the next reading.
- **Fix:** the scenario was reworded to match the requirement body and the implementation — the CLI
  writes one artifact per module of the workspace program, and it is the entry's *module table*
  that names only what it references, directly or transitively. Keeping the implementation was the
  call because a workspace codegen whose output depends on which entry was selected would surprise
  anyone building all of a workspace's modules, and the requirement body already said so.
- **Verification:** Reopened. The artifact-count half is fixed and keeping the implementation was
  the right call, but the reword traded one contradiction for a sharper one. The scenario's last
  bullet now reads "`app/main.nx`'s artifact SHALL name each module it references, **directly or
  transitively**, in its module table", and that is the opposite of what the format requires:
  `nx-ir-format`'s *An artifact carries one module and names the modules it links against* and
  `docs/nx-ir-format.md`'s module-table section both say "A module reachable only transitively is
  not listed; it appears in the table of the module that references it." Before the reword the
  clause said "each of those modules", which was merely ambiguous; it is now explicitly wrong, and
  it is the text that would be copied into the main spec on archive. Verified against a three-module
  chain — `app/main.nx` imports `shared/value.nx`, which imports `deep.nx` — where the CLI writes
  three artifacts and `app/main.nx`'s table lists `['app/main.nx', 'shared/value.nx']`, with
  `deep.nx` appearing only in `shared/value.nx`'s table.
- **Recommendation:** Drop "or transitively" so the bullet reads "each module it references
  directly", or drop the bullet entirely — `nx-ir-format` already governs the module table, and this
  scenario only needs to say which files get written.
- **Fix (2):** the bullet is gone, which is the second option. Rewording it to "directly" would have
  left the CLI spec restating a rule `nx-ir-format` owns — and that duplication is exactly what
  drifted here, twice. `nx-ir-format`'s *An artifact carries one module and names the modules it
  links against* states the table's contents and carries its own scenarios for them, including *A
  module nothing references is not in the table*. The CLI scenario now says only what is the CLI's
  to say: which entry is selected, and that one artifact is written per module of the workspace
  program.
- **Verification (2):** Confirmed. The scenario now selects the entry and requires one artifact per
  module of the workspace program, `app/main.nx` included, and says nothing about the module table.
  `nx-ir-format` covers the table on its own, including *A module nothing references is not in the
  table*, which `a_module_nothing_references_is_not_in_the_table` tests. The two specs no longer
  disagree, and `openspec validate --strict` passes.

### ✅ Verified - RF10 Module fingerprints come from `DefaultHasher`, whose output the standard library does not promise across Rust releases

- **Severity:** Low
- **Evidence:** [`module_fingerprint`](crates/nx-codegen/src/ir.rs#L625) hashes identity and source
  through `std::collections::hash_map::DefaultHasher`. Its documentation states the algorithm is
  unspecified and hashes should not be relied on across releases. The corpus now pins those values
  byte-for-byte (`"fingerprint":"4933815242345845422"` in every expected artifact), so a toolchain
  bump would fail `corpus_artifacts_are_pinned` for every program with no semantic change. The repo
  pins Rust 1.98.1, which masks it until the pin moves.
- **Recommendation:** Either switch to a stable hash the repo controls (FxHash, or a small
  explicitly-specified FNV/xxHash) and say so in `docs/nx-ir-format.md`, or have the corpus
  comparison normalize fingerprints the way `emit-example-ir.mjs` already does and assert them in a
  separate, narrower test.
- **Fix:** the first option. `module_fingerprint` is now FNV-1a over the identity's bytes, a zero
  byte and the source's bytes, spelled out in `ir.rs` with the reason
  ([ir.rs](crates/nx-codegen/src/ir.rs)); `DefaultHasher` is gone from that file.
  `docs/nx-ir-format.md` states the algorithm as part of the format, which makes the promise it
  already implied — the same module fingerprints the same whatever emitted it — actually true. A
  stable hash, rather than normalizing the corpus, because the fingerprint travels inside every
  artifact and a reader comparing two artifacts from different toolchains should not see a
  difference that is not there. The corpus was regenerated.
- **Verification:** Confirmed. `module_fingerprint` is FNV-1a with the standard 64-bit offset basis
  and prime, XOR-then-multiply, over identity + `0x00` + source
  ([ir.rs](crates/nx-codegen/src/ir.rs)); the only remaining mention of `DefaultHasher` in that file
  is the comment saying why it is not used. `docs/nx-ir-format.md` states the algorithm as part of
  the format, which is what makes the fingerprint comparable across toolchains rather than merely
  stable within one.

### ✅ Verified - RF11 `NxUtf8SliceScope.cs` declares a second public-surface type

- **Severity:** Low
- **Evidence:** [NxUtf8SliceScope.cs:82](bindings/dotnet/src/NxLang.Sdk/Interop/NxUtf8SliceScope.cs#L82)
  defines `internal struct NxUtf8Slice` below the class. AGENTS.md's C# conventions say "One primary
  type per file", and the surrounding folder follows that: `NxWorkspaceModuleDescriptor.cs` and
  `NxBuffer.cs` each hold a single interop struct.
- **Recommendation:** Move `NxUtf8Slice` to `Interop/NxUtf8Slice.cs`.
- **Fix:** moved to [NxUtf8Slice.cs](bindings/dotnet/src/NxLang.Sdk/Interop/NxUtf8Slice.cs), with a
  doc comment and the same header as its siblings. The solution builds with no warnings and the 134
  .NET tests pass.
- **Verification:** Confirmed. `Interop/NxUtf8Slice.cs` holds the struct alone with the standard
  header and a doc comment, `NxUtf8SliceScope.cs` ends at its class, and the 134 .NET tests pass.

### ✅ Verified - RF12 The playground bundles the catalog artifact as a JavaScript object literal rather than JSON text

- **Severity:** Low
- **Evidence:** The Vite plugin emits `` `export default ${JSON.stringify(artifact)};` ``
  ([vite.config.ts:36](sites/playground/vite.config.ts#L36)). Design D10 has the fiddle bundle its
  catalog "as JSON text in nx-runtime.js", and `dev/build-nx-runtime.mjs` on the fiddle branch does
  exactly that. A large object literal is parsed by the JavaScript parser on every page load, which
  is materially slower than `JSON.parse` of an equivalent string literal — the cost this change
  exists to reduce. It also avoids the one case where `JSON.stringify` output is not valid ES5
  source (a raw U+2028/U+2029 inside a string), which `build.target: "esnext"` happens to tolerate.
- **Recommendation:** Emit `export default JSON.parse(${JSON.stringify(JSON.stringify(artifact))});`
  and relax `vite.config.test.mjs`'s `^export default (.*);$` assertion accordingly.
- **Fix:** taken as recommended, with the reason in a comment at the emit site
  ([vite.config.ts](sites/playground/vite.config.ts)). The config test now matches
  `^export default JSON\.parse\((".*")\);$` and double-parses, so it still pins the artifact's
  contents as well as its shape.
- **Verification:** Confirmed. The plugin emits
  `export default JSON.parse(${JSON.stringify(JSON.stringify(artifact))});` with the reasoning in a
  comment at the site, and `vite.config.test.mjs` matches the new shape and double-parses, so it
  still asserts `format`, the module table and the absence of a debug section.

### ✅ Verified - RF13 Module versions are a wasm-SDK-only concept in the build API and an emit-time option everywhere else

- **Severity:** Low
- **Evidence:** The wasm SDK takes `modules[].version` on `buildWorkspaceArtifact`, stashes the map
  on the JS side, and merges it into the emit options
  ([host.ts, `WasmProgramArtifact`](bindings/wasm/src/host.ts)); the Rust side never sees a
  build-time version. The Node SDK's `NxWorkspaceModuleInput`
  ([types.ts:30-42](bindings/node/src/types.ts#L30-L42)) has no `version` at all, so Node, the C FFI
  and .NET callers must pass `versions` through `NxIrEmitOptions` themselves. Only the `sdk-wasm`
  spec requires the build-time form, so nothing is violated, but the same concept now has two shapes
  across a family of SDKs whose parity is otherwise tested byte-for-byte.
- **Recommendation:** Either add the optional `version` to `NxWorkspaceModuleInput` in the other
  SDKs, or drop the wasm-side stash and document `versions` as an emit option uniformly in
  `bindings/wasm/README.md` and `docs/nx-ir-format.md`.
- **Status:** Left open deliberately — neither branch is clearly right, and the finding itself says
  nothing is violated. The second branch is not available: the `sdk-wasm` delta spec *requires* the
  build-time form ("its source text and an optional version string"), so dropping the stash would
  break a requirement this change adds. The first branch is a design decision rather than a repair:
  the Node SDK's shape differs — `NxWorkspace` is a reusable object built from the modules and
  `NxProgramArtifact.buildWorkspace(workspace, options)` never sees them — so the versions would
  have to live on the workspace and follow it into every artifact built from it, which is a
  different contract from the wasm SDK's per-build stash, and the C FFI and .NET would need the
  matching decision to make the parity real. The documentation half is already done:
  `bindings/wasm/README.md` says the module table records "the versions the build was given, plus
  any passed as `options.versions`". Recommend deciding this as its own change, across all four
  SDKs at once, rather than growing a third shape here.
- **Verification:** Left open, and the reasoning holds. The `sdk-wasm` delta really does require the
  build-time form ("its source text and an optional version string"), so dropping the stash would
  break a requirement this change adds, and `bindings/wasm/README.md` documents the merge. Agreed
  that aligning the other three SDKs is its own change.
- **Fix:** taken after all, in this change, before schema 3 ships and the emit-time form becomes
  something four SDKs would have to retire. A version is part of the module, not of an emit:
  - `NxWorkspaceModule` in Rust gained `with_version`/`version` (an empty string is none), carried
    through the logical module graph onto the `ProgramArtifact` and into `CodegenSourceEntry`; the
    emitter reads it from there. The version also enters the program's fingerprint, since two
    builds differing only in a version emit different artifacts.
  - `NxIrEmitOptions::versions` is gone in Rust, Node, wasm and .NET, and the options now use
    `deny_unknown_fields`, so a caller still passing `versions` is refused rather than silently
    losing them. That exposed the Node SDK forwarding its caller's whole options object (including
    `generateNxIrFromSource`'s `fileName`); it now sends only `modules` and `debug`, as wasm does.
  - The C FFI's `NxWorkspaceModule` gained `version_ptr`/`version_len` (ABI 13 is already this
    branch's unreleased bump); `bindings/c/nx.h` was regenerated. .NET's descriptor and
    `NxWorkspaceModule.Version` match. Node's module input gained `version`. The wasm SDK passes
    `modules[].version` to Rust, and its per-build stash is deleted.
  - Tests: `every_artifact_of_a_build_records_the_version_the_module_was_given`,
    `a_program_built_under_another_version_is_another_program` and
    `emit_options_refuse_a_key_they_do_not_have` (codegen); the corpus harness gives `program.json`'s
    versions to the modules; `ffi_workspace_module_version_reaches_the_module_table`; the Node
    corpus test now also pins `snippet` (versioned, implicitly imported); a wasm↔Node parity test
    emits a versioned workspace byte-for-byte alike; and
    `GenerateNxIr_RecordsTheVersionTheWorkspaceGaveEachModule` in .NET.
  - Specs and docs: `workspace-programs` gained *Workspace modules carry an optional version*;
    design D6, tasks 2.3 and 5.3, `docs/nx-ir-format.md`, both SDK READMEs and the corpus README
    say where the version lives.

  `cargo test --workspace`, `pnpm -r test`, the playground's typecheck and `check-examples`, and
  `dotnet test bindings/dotnet/NxLang.sln` (137) pass; `openspec validate --strict` passes.
- **Verification (2):** Confirmed on every path. `NxWorkspaceModule::with_version` treats an empty
  string as no version. The version goes through `LogicalSourceModule`, `ProgramArtifact`'s
  `version_map` and `ProgramSourceEntry` into `CodegenSourceEntry`, and `module_version` in `ir.rs`
  reads it for both the module's own table entry and each referenced module's entry. It is part of
  the program fingerprint, and `a_program_built_under_another_version_is_another_program` tests
  that. `NxIrEmitOptions` has no `versions` field and uses `deny_unknown_fields` in Rust. The FFI
  test checks that passing `versions` fails with `InvalidArgument` rather than being ignored.
  `NxWorkspaceModule` in the FFI reads `version_ptr`/`version_len`, and a zero length is accepted
  even with a null pointer, so a caller that zero-fills the struct gets no version.
  `bindings/c/nx.h` is current: a fresh `cbindgen` run produces the committed file byte for byte.
  .NET pins the version bytes only when there are some and frees that handle on dispose. The Node
  SDK now sends only `modules` and `debug`. The wasm SDK's per-build stash is gone and
  `modules[].version` reaches Rust. The wasm↔Node parity test builds a versioned workspace and
  compares every module's bytes and metadata. The fiddle branch already gives the catalog its
  version on the module (`nx/src/runtime.ts:156`), so nothing downstream still uses the removed
  option. One small usability gap came up in the .NET API; it is recorded as RF15 and does not
  affect this fix.

## New Findings Discovered During 2026-09-15 18:52 Verification

### ✅ Verified - RF14 `pnpm run typecheck` in `sites/playground` fails, so task 6.1's stated verification does not hold

- **Severity:** Low
- **Evidence:** `pnpm run typecheck` exits 2 with `NxEditor.tsx(24,41): error TS2345` — the `monaco`
  the playground passes to `registerNxLanguage` is `monaco-editor@0.56.0`, while
  `@nx-lang/monaco`'s `MonacoNamespace` is typed against the `monaco-editor@^0.52.2` in its own
  devDependencies, and `editor.create`'s `ITextModel` has diverged between the two. Both versions
  resolve under `node_modules/.pnpm`. This is **not** caused by this change:
  `sites/playground/package.json` (`^0.56.0`), `packages/monaco/package.json` (peer `>=0.52.0`, dev
  `^0.52.2`) and `pnpm-lock.yaml` are unmodified in the working tree and carry the same ranges on
  `HEAD`, and the change's only edit to `NxEditor.tsx` is one word in a comment. It is nonetheless a
  currently-failing command that task 6.1 names as its verification ("verify `pnpm run typecheck`,
  `pnpm test` and `pnpm run check-examples` pass"), so the task is not verifiable as written. It also
  corrects this report's own Scope section, which listed the playground typecheck among the green
  runs: the original run piped `typecheck` and `test` into one tail and only the tail was read, so
  the failure was not visible.
- **Recommendation:** Out of scope to fix here. Either align `packages/monaco`'s devDependency with
  the peer range the playground actually uses, or note in task 6.1 that the playground's typecheck
  has a pre-existing Monaco failure unrelated to this change, so the next reader does not chase it.
  Everything else in task 6.1 passes: `pnpm test` is 61 passing, `check-examples` is 20 examples.
- **Fix:** aligned rather than documented — a command that fails is worse to carry than a one-line
  range to change. `packages/monaco`'s `monaco-editor` devDependency went from `^0.52.2` to
  `^0.56.0`, the version the only consumer in the workspace already depends on and well inside the
  package's own `>=0.52.0` peer range; `pnpm-lock.yaml` follows, and the workspace now resolves one
  copy instead of two. `sites/playground`'s `pnpm run typecheck` passes, and `packages/monaco` still
  builds and passes its 15 tests against the newer types, so nothing it uses changed between the
  two. This is a pre-existing failure, as the finding says, not one this change introduced — fixing
  it here is what makes task 6.1's stated verification hold. One incidental line in the lockfile
  came with the re-resolve: `@napi-rs/cli`'s optional `@emnapi` peers moved from `1.9.2` to
  `1.11.2`. Both were already in the lockfile, nothing was downloaded, and the Node binding builds
  and passes its 20 tests; it is left as pnpm wrote it rather than hand-reverted into a state the
  next install would undo.
- **Verification:** Confirmed. `packages/monaco` now asks for `monaco-editor` `^0.56.0`, and
  `pnpm-lock.yaml` no longer lists `monaco-editor@0.52.2` in its packages or snapshots.
  `packages/monaco/node_modules/monaco-editor` points to the 0.56.0 copy; a 0.52.2 folder is still
  in `node_modules/.pnpm`, but nothing links to it. `pnpm run typecheck` in `sites/playground`
  exits 0, and `packages/monaco` still passes its 15 tests. The lockfile also gains the
  `@nx-lang/ir-runtime` workspace links that the Node SDK's new devDependency needs; that comes
  from the `dist/` question, not from this fix.

## New Findings Discovered During 2026-09-16 10:30 Verification

### ✅ Verified - RF15 In .NET, a module built with `NxWorkspaceModule.FromSourceText` cannot be given a version

- **Severity:** Low
- **Evidence:** `NxWorkspaceModule.Version` is an `init` property
  ([NxWorkspaceModule.cs](bindings/dotnet/src/NxLang.Sdk/NxWorkspaceModule.cs)). C# only lets an
  object initializer follow a `new` expression. `NxWorkspaceModule` is a class, not a record, so
  `with` is unavailable too. As a result, `FromSourceText(identity, source)`, the text factory used
  by most callers and by nearly every test, has no way to set a version. The only way to version a
  module is `new NxWorkspaceModule(identity, Encoding.UTF8.GetBytes(source)) { Version = "9" }`,
  which is what `GenerateNxIr_RecordsTheVersionTheWorkspaceGaveEachModule` has to do. Rust
  (`with_version`), Node and wasm (`{ identity, source, version }`) all let any module carry a
  version.
- **Recommendation:** Add an optional `string? version = null` parameter to `FromSourceText`, or add
  a `WithVersion(string?)` method that returns a copy, matching Rust's `with_version`. Then let the
  .NET test use the text factory.
- **Fix:** `FromSourceText` takes an optional `string? version = null` and sets `Version` from it
  ([NxWorkspaceModule.cs](bindings/dotnet/src/NxLang.Sdk/NxWorkspaceModule.cs)); the `init` property
  stays for the byte constructor. `GenerateNxIr_RecordsTheVersionTheWorkspaceGaveEachModule` now
  builds the versioned module with `FromSourceText(..., version: "9")`, and `docs/nx-ir-format.md`
  mentions the parameter. `dotnet test bindings/dotnet/NxLang.sln` passes (137).
- **Verification:** Confirmed. `FromSourceText(identity, source, string? version = null)` sets
  `Version` in an object initializer on the byte constructor, and its doc comment points to
  `Version`, which already defines an empty string as no version. The .NET test now builds the
  versioned module with `version: "9"` and still checks that `drawnui.nx` is recorded with version
  `"9"`. `docs/nx-ir-format.md` names both ways to set the version. The build has no warnings and
  `dotnet test bindings/dotnet/NxLang.sln` passes 137 tests. Adding the parameter breaks binary
  compatibility but not source compatibility, which is fine: the project promises no compatibility
  yet, and ABI 13 has not been released.

## Verification run 2026-09-16 (RF15)

**Suites:** `dotnet test bindings/dotnet/NxLang.sln` (137, all green).

**Result:** 1 fix verified (RF15), 0 reopened, 0 new findings. Every finding in this report is now
verified or resolved.

## Verification run 2026-09-16 10:30

**Suites (all green):** `cargo test --workspace` (1,886 passed); `pnpm -r test`, including
`bindings/wasm` (33), `bindings/node` (23), `sites/playground` (61, and its 20 examples),
`packages/monaco` (15) and the `runtime/typescript` corpus; the playground's `pnpm run typecheck`
and `pnpm run check-examples`; `dotnet test bindings/dotnet/NxLang.sln` (137);
`openspec validate linkable-compact-nx-ir --strict`. The working tree also holds the unarchived
`binary-nx-ir` work, so these runs cover both changes together.

**Result:** 3 fixes verified (RF9, RF13, RF14), 0 reopened, 1 new finding (RF15, Low).

## Verification run 2026-09-15 18:52

**Suites (all green except RF14):** `cargo test --workspace`; `runtime/typescript` `pnpm test`
including the corpus runner; `sites/playground` `pnpm test` (61) and `check-examples` (20 examples),
with `pnpm run typecheck` failing per RF14; `bindings/wasm` (30); `bindings/node` (20);
`dotnet test bindings/dotnet/NxLang.sln` (134).

**Result:** 10 fixes verified, 1 resolution verified, 1 finding reopened (RF9), 1 left open by
agreement (RF13), 1 new finding (RF14).

## Fix pass 2 (RF9 reopened, RF14)

- **RF9:** the offending bullet was removed rather than reworded, so the CLI spec no longer restates
  a module-table rule `nx-ir-format` owns. See its **Fix (2)** note.
- **RF14:** fixed rather than documented — `packages/monaco`'s `monaco-editor` devDependency now
  matches the `^0.56.0` its only workspace consumer uses, so one copy resolves and the playground's
  typecheck passes. See its **Fix** note.
- **RF13** remains open by agreement: a cross-SDK API decision, not a repair.

**Test runs after this pass (all green):** `cargo test --workspace`; `pnpm -r test` across all ten
workspace projects (`language-protocol` 4, `language-core` 13, `monaco` 15, `language-http` 18,
`language-client` 15, `playground` 61, `sdk-wasm` 30, `sdk-node` 20); `runtime/typescript` 115;
`sites/playground` `pnpm run typecheck` (now passing) and `check-examples` (20 examples);
`dotnet test bindings/dotnet/NxLang.sln` (134).

## Questions

- The proposal's Impact lists `@nx-lang/language-core` among the packages this change touches, but
  `packages/` is entirely unmodified and design D5 says language-core keeps its prelude arithmetic
  for the HTTP service. Does it still need a republish, or should the Impact list drop it?
  - **Answered.** `packages/` is untouched, and no manifest is edited by this change at all (see
    RF6): a release stamps every publishable package at the tag's version, so language-core is
    republished at the new version with unchanged code, like every other package. The Impact bullet
    was rewritten to say that rather than claiming the change edits package versions.
- RF9 needs a decision on which text is authoritative before the change archives, since archiving
  copies the requirement and its scenario into the main spec as they stand.
  - **Answered.** The requirement body and the implementation win; the scenario was reworded. See
    RF9.
- `runtime/typescript/dist/` is tracked in git and rebuilt by `pnpm test`. That predates this
  change, but the change rewrites 4,300 lines of it; is keeping the build output committed still
  intended?
  - **Answered: no, it was an accident.** Every other package ignores its `dist/`, and every
    consumer builds before reading it: CI and the release run `pnpm -r build`, `prepack` builds, the
    playground's Dockerfile builds from source, and the fiddle's `--nx` mode requires a built
    checkout. `runtime/typescript/.gitignore` now ignores `dist/`, to be untracked with
    `git rm -r --cached runtime/typescript/dist`. The one hidden reader was the Node SDK's test,
    which imported the runtime by a relative path into `dist/`; `bindings/node` now declares
    `@nx-lang/ir-runtime` as a workspace devDependency and imports it by name, so pnpm builds the
    runtime first.

## Summary

The change is substantially complete and well built. The schema document is good enough to
hand-decode a corpus artifact from (I checked `snippet` operand by operand and it matches), the
conformance corpus is a genuine kind-coverage gate rather than a set of golden files, the linking
API covers every scenario in the `typescript-ir-runtime` delta with a named test each, and every
suite I ran is green: `cargo test -p nx-codegen`, the TypeScript runtime including the corpus, the
playground with `check-examples`, both binding SDKs, and the .NET tests.

The one finding I would fix before the release is RF1: the debug section's node spans are `[0, 0]`
for every literal and identifier, which leaves the provenance requirement materially unmet and is
now pinned as expected output in the corpus. The fix is a one-line change in `builder.rs` plus a
corpus regeneration, and it makes RF2 and RF3 live, so those three are best handled together.
RF4 (a latent catalog-generation bug), RF5 (implicit imports cancelled by a named import) and RF6
(package versions not bumped, which blocks tasks 6.5 and 7.1) are the rest of the pre-release set.
The remainder are cleanups and coverage gaps that can follow.

13 findings opened: 1 High, 5 Medium, 7 Low.

## Fix pass

Eleven findings fixed, one resolved as not a defect, one left open.

- **Fixed (await verification):** RF1, RF2, RF3, RF4, RF5, RF7, RF8, RF9, RF10, RF11, RF12.
- **Resolved:** RF6 — the release stamps package versions at pack time, so no manifest bump exists
  to make.
- **Open:** RF13 — a cross-SDK API decision, not a repair; see its Status note.

Two findings turned out to be narrower or different than described, and the notes say so: `ir
explain` never rendered node spans (RF1), and RF5's recommended fix on its own makes the compiler
report a repeated import, because NX allows one import per module identity — the fix needed a
second change in `apply_graph_imports` to exempt the synthesized import from that check.

**Test runs after the fixes (all green):** `cargo test --workspace`, `pnpm test` in
`runtime/typescript` (115), `sites/playground` (61) with `check-examples` (20 examples),
`bindings/wasm` (30) and `bindings/node` (20), and `dotnet test bindings/dotnet/NxLang.sln` (134).

### A note on the working tree

While fixing RF5 I ran `git checkout` on `crates/nx-api/src/artifacts.rs` to undo an experiment,
which discarded that file's uncommitted work — the whole implicit-import feature, which lives only
in the working tree. It was reconstructed from the `workspace-programs` spec, the FFI and SDK call
sites, and the failing tests: `ProgramBuildContext`'s `implicit_imports` field with
`with_implicit_imports`/`implicit_imports`, `synthesize_implicit_imports` and
`workspace_import_path`, the `implicit-import-not-found` program diagnostic, the fingerprint
contribution, and all seven original tests, every one of which now passes by name. The wasm module
was rebuilt from the reconstructed source and its 30 tests pass, as do the playground's and the
Node SDK's, which exercise the feature end to end. Worth a read of that file's diff all the same.
