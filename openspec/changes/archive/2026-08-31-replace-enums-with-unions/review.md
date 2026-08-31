# Review: replace-enums-with-unions

## Scope
**Reviewed artifacts:** `proposal.md`, `design.md`, `tasks.md`, and all ten delta specs under
`specs/`  
**Reviewed code:** The working-tree and staged changes across the syntax grammar and generated
parser, HIR/types/interpreter/API/FFI, CLI formatting and type generation, executable codegen and
TypeScript runtime, .NET serializers and tests, language service/editor assets, examples, and
documentation  

## Findings

### ✅ Verified - RF1 The .NET suite has eight failures against a freshly rebuilt native runtime
- **Severity:** High
- **Evidence:** The checked-off verification task claims all 96 .NET tests pass, but several test
  sources still use the removed declaration form, including
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxRuntimeBasicTests.cs:74`,
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxRuntimeComponentTests.cs:952`, and
  `bindings/dotnet/tests/NxLang.Sdk.Tests/NxUnionSerializationTests.cs:343`. Running `dotnet test`
  initially passed only because `bindings/dotnet/build/NxLang.Sdk.targets` stages the release
  native library by default and that artifact predated this change. After
  `cargo build --release -p nx-ffi`, `dotnet test bindings/dotnet/NxLang.sln --no-restore` fails 8
  of 96 tests with the new `removed-enum-keyword` diagnostic. This also leaves the new raw-wire
  equivalence test unable to exercise its assertions (`NxUnionSerializationTests.cs:213-218`).
- **Recommendation:** Migrate every embedded NX declaration in the .NET tests to `type`, replace
  the obsolete enum-vs-union comparison with a standing constant-union wire-shape assertion, and
  rerun the suite after rebuilding the release `nx-ffi` artifact. Make the verification command
  rebuild or explicitly stage the current native library so stale binaries cannot produce another
  false green.
- **Fix:** Migrated all eight embedded NX declarations to `type` across `NxRuntimeBasicTests.cs`,
  `NxRuntimeComponentTests.cs`, and `NxUnionSerializationTests.cs`. Replaced the enum-vs-union
  comparison with `RawConstantCaseResults_UseTheBareStringWireShape`, which now asserts the enduring
  property — a constant case is a bare string whether its union is wholly constant or mixed — over
  a `ConstantUnionSource` and the existing mixed-union source. Renamed the three test methods whose
  names asserted something about NX enums; left the `"Unknown NX enum value."` / `"Unknown NX enum
  member."` strings and the CLR `enum` DTOs alone, because those mirror the C# generator verbatim
  (`crates/nx-cli/src/typegen/languages/csharp.rs:627,646`) and are the generated host shape, not an
  NX concept. `cargo build --release -p nx-ffi && dotnet test bindings/dotnet/NxLang.sln` now passes
  96/96. Recorded the required two-step command in task 10.1 with the reason. **Not done:** wiring
  cargo into `NxLang.Sdk.targets` so the managed build cannot run against a stale native library.
  That would make cargo a hard prerequisite of every managed build for every consumer, which is a
  build-contract change beyond this finding — left for the maintainer to decide.
- **Verification:** Rebuilt the release native library with `cargo build --release -p nx-ffi`, then
  ran the managed suite against that artifact; all 96 tests passed. The embedded NX sources no
  longer contain an `enum` declaration, and the replacement raw-wire test exercises both constant
  and mixed unions.

### ✅ Verified - RF2 The shipped Tree-sitter queries still reference grammar symbols that were removed
- **Severity:** Medium
- **Evidence:** `crates/nx-syntax/queries/highlights.scm:16` still queries the anonymous `enum`
  token and `:35` queries `enum_definition`; `crates/nx-syntax/queries/locals.scm:42` also queries
  `enum_definition`. Neither symbol exists in the regenerated grammar or `node-types.json`, so a
  consumer compiling these queries against the new language will receive an invalid-node/token
  query error. The Rust parser tests do not load these query assets, which is why they remain green.
- **Recommendation:** Remove the `enum`/`enum_definition` query patterns, add the appropriate
  `union_definition` definition/highlight captures, and add a test that compiles every shipped
  Tree-sitter query against `nx_syntax::language()`.
- **Fix:** Dropped the `enum` token from the keyword list and replaced the `enum_definition`
  patterns in both files with `union_definition` captures — `name`/`base` as `@type` and
  `union_case` `name` as `@constant` in `highlights.scm`, `name` as `@local.definition` in
  `locals.scm`. The queries had no union coverage at all before this, so the replacement also closes
  a gap rather than only removing dead patterns. Added
  `crates/nx-syntax/tests/query_tests.rs`, which compiles every `.scm` under `queries/` against
  `nx_syntax::language()` and is directory-driven so a new asset is covered without editing the
  test. Confirmed it catches the regression: re-adding the `enum_definition` pattern fails with
  "Query error at 196:2. Invalid node type enum_definition".
- **Verification:** Both query tests passed as part of `cargo test --workspace`; the directory-driven
  test compiled every shipped `.scm` file against the current grammar, and neither query asset
  references the removed declaration symbols.

### ✅ Verified - RF3 TypeScript type generation does not emit the required constant value object
- **Severity:** Medium
- **Evidence:** The `cli-code-generation` delta requires a constant union to generate an `as const`
  value object plus a type derived from that object. Instead,
  `crates/nx-cli/src/typegen/languages/typescript.rs:361-376` emits only
  `export type Name = "case1" | "case2";` and returns. Consumers therefore cannot write the
  generated-value-object form promised by the spec (for example, `ThemeMode.Dark`). The executable
  TypeScript emitter has the desired object, but `nxlang typegen --language typescript` does not.
- **Recommendation:** Emit the exported `as const` object and derive the type with
  `typeof Name[keyof typeof Name]`, then add a CLI typegen assertion covering both declarations.
  If preserving the old type-only enum output is intentional, amend the delta spec before archive
  rather than marking the current implementation complete.
- **Fix:** Took the second option — the artifacts were wrong, not the code. The root error is
  upstream in task 7.5, which told the implementer to add the object to CLI typegen; the `as const`
  object is executable codegen's form for the same declaration
  (`crates/nx-codegen/src/emit.rs:1163-1178`) and is already correct in the
  `executable-code-generation` delta. Design D4 and `proposal.md` carried the same conflation.
- **Decision:** This was then reconsidered on its merits, explicitly setting backward compatibility
  aside, since byte identity with the enum form is a migration guard rather than a reason to prefer
  one TypeScript shape forever. **The string-literal union is kept**, on three grounds:
  1. *The two forms are the same type.* `typeof ThemeMode[keyof typeof ThemeMode]` evaluates to
     `"light" | "dark"`. Verified with `tsc --strict`: bare-string assignment, narrowing, switch
     exhaustiveness, and even the error text are identical under both — the same `TS2820 ... Did you
     mean '"dark"'?`. The choice is therefore never about the type, only about whether a types-only
     module also exports a value.
  2. *Role.* CLI type generation emits a pure type surface: its TypeScript output contains no
     runtime values whatsoever today, so the module erases completely. The string-literal union is
     TypeScript's idiomatic closed set for that role; the `as const` object is idiomatic for
     emitting runtime values, which is executable codegen's job, not this one. C# getting an `enum`
     from the same command is localization, not asymmetry — each language gets its own closed set.
  3. *Reversibility.* Adding the value object later is a non-breaking addition; removing it once
     consumers import it is not. With no consumer asking for it, the union keeps the option open.
- **Revisit when:** a TypeScript host needs the cases at runtime — enumerating them for a property
  inspector or dropdown, or validating untrusted input — because a hand-maintained case list rots
  silently when NX gains a case. That is the one capability C# hosts have (`Enum.GetValues`) and
  TypeScript hosts do not. The change is roughly fifteen lines in `emit_union`, plus reverting the
  spec edits below. Recorded as the trigger in design D4.
- **Artifacts updated:** the `cli-code-generation` requirement now states the pure-type-surface rule
  and why executable codegen differs, with a new scenario *Generated TypeScript type declarations
  export no runtime values*; design D4 was rewritten to record the decision, the type-equivalence
  fact, and the revisit trigger; `proposal.md` and task 7.5 were corrected the same way.
- **Test:** added `constant_and_mixed_unions_generate_their_respective_host_shapes` to
  `crates/nx-cli/src/typegen.rs`, pinning the C# `enum`, the TypeScript string-literal union, the
  mixed-union shapes, and the absence of *any* runtime export (`export const`/`function`/`class`/
  `enum`) — which is what backs the new scenario. Nothing previously pinned the generated form,
  which is why the divergence went unnoticed.
- **Verification:** The proposal, design D4, task 7.5, and `cli-code-generation` delta now agree that
  CLI TypeScript generation is a pure type surface, while executable codegen owns the `as const`
  value object. The new host-shape test passed in the workspace suite, and strict OpenSpec
  validation passed.

### ✅ Verified - RF4 The removed-keyword path still emits a generic syntax error and does not show the actual replacement
- **Severity:** Medium
- **Evidence:** The `discriminated-unions` delta says an enum declaration reports the replacement
  form *rather than* a generic parse error and its scenario requires the diagnostic to name
  `type Fit = fill | contain | cover`. `validate_reserved_enum_keyword` merely appends a second
  diagnostic (`crates/nx-syntax/src/validation.rs:65-66`) and its note uses
  `type Fit = ...` (`:102-119`). A current .NET failure exposes both `syntax-error` and
  `removed-enum-keyword` for the same declaration. The unit test at `:813-829` checks only that the
  targeted code exists, so it misses both divergences.
- **Recommendation:** Suppress parse diagnostics attributable to the recognized removed
  declaration and construct the replacement from the declaration's remaining source text so the
  message contains the concrete `type` spelling. Strengthen the test to assert the complete
  diagnostic set and replacement text.
- **Fix:** Both halves. Added `enum_replacement_form`, which builds the note from the declaration's
  own text, so `enum Fit = fill | contain | cover` now reports ``Write `type Fit = fill | contain |
  cover` instead`` — the exact spelling the delta scenario requires. When the case list continues
  past the keyword line the list is elided to `...` rather than guessed at. Added
  `suppress_parse_errors_for_removed_declarations`, called from `parse_source` in
  `crates/nx-syntax/src/lib.rs` once both diagnostic sets are merged, which drops a `syntax-error`
  whose primary span contains a `removed-enum-keyword` span. An `enum` declaration now produces
  exactly one diagnostic. Strengthened both tests to `assert_eq!(codes, vec!["removed-enum-keyword"])`
  plus the full replacement text, and added two unit tests for the concrete and elided forms. Also
  replaced the `<para>` tag in that function's doc comment — a C# XML-doc convention that does not
  belong in rustdoc.
- **Verification:** The original scenario now produces only `removed-enum-keyword`, with the exact
  replacement ``type Fit = fill | contain | cover``. The strengthened syntax tests passed. A
  separate false-positive issue discovered in the source-level scanner is recorded as RF6 below.

### ✅ Verified - RF5 The .NET binding documentation still describes the removed model and the wrong mixed-union wire shape
- **Severity:** Low
- **Evidence:** `bindings/dotnet/README.md:299-322` still documents “NX enum values,” enum-typed
  slots, and enum member lists. The following discriminated-union section says union cases are
  encoded as `$type` maps, omitting the newly supported bare-string constant cases in mixed unions.
  This conflicts with the modified `dotnet-binding` requirements that the binding document the
  unified constant-case contract and the polymorphic reader's scalar alternative.
- **Recommendation:** Rewrite the section around constant unions/cases, retain CLR `enum`
  terminology only for the generated host shape, and document both wire forms for mixed unions.
- **Fix:** Retitled *Enum Encoding* to *Constant Case Encoding* and rewrote it around the D3
  definition — no fields **and** no base — stating explicitly that the CLR `enum` is the generated
  host shape for a constant union and not a separate NX concept. Corrected *Discriminated Union
  Encoding*, which asserted the opposite of current behaviour: a constant case is now shown as the
  bare `"idle"` rather than a `{"$type": "LoadState.idle"}` map, with the note that a fieldless case
  of a **based** union is not constant and keeps the map form. Replaced the "enums and discriminated
  unions intentionally remain separate wire contracts" paragraph with the two-shape mixed-union
  contract and the `[NxConstantCase]` singleton, and updated *Generated Types* to derive the host
  shape from constant-ness. Every remaining `enum` in the file now refers to the CLR type.
- **Verification:** The binding guide now defines constant cases and constant unions, identifies a
  CLR `enum` only as the generated host shape, and documents both bare-string and `$type` map forms
  for a mixed union, including the based-union boundary.

## New Findings Discovered During 2026-08-31 14:14 Verification

### ✅ Verified - RF6 The removed-keyword scanner reports `enum` inside raw text as a declaration
- **Severity:** Medium
- **Evidence:** `validate_reserved_enum_keyword` in
  `crates/nx-syntax/src/validation.rs` scans every source line without consulting the syntax tree.
  Consequently, source containing
  `<code:text raw>\nenum Fit = fill | contain | cover\n</code>` reports
  `removed-enum-keyword` on the raw text line, even though it is element content rather than
  declaration position. The existing negative test covers only the identifier `enumerate`, not an
  exact `enum` token in a non-declaration syntax region.
- **Recommendation:** Restrict the diagnostic to declaration-position syntax or exclude spans owned
  by comments, strings, and text-element content before scanning. Add a regression test using an
  exact `enum` declaration-shaped line inside `<code:text raw>` and assert no
  `removed-enum-keyword` diagnostic is emitted.
- **Fix:** Took the exclusion route, since positive detection is not available — `enum` is out of
  the grammar, so no node names it and the scan has to stay source-level. `validate_reserved_enum_keyword`
  now collects prose regions from the tree first (`prose_spans` / `is_prose`: comments, string
  literals, and every text-content, text-run, and text-chunk kind, raw and embedded included) and
  skips a candidate whose keyword offset falls inside one. Whole regions are collected rather than
  lines, so a multi-line comment or a raw text body is excluded in one piece.
- **Wider than reported:** the scan also fired on plain `<message:>` text content and on block
  comments, neither of which the finding named. It passed on line comments and on the string case
  only by accident — `//` and `let` happen to sit at the start of those lines, so the
  declaration-position prefix test failed before the keyword was reached. All four are now excluded
  on purpose rather than by luck.
- **Test:** `does_not_report_a_declaration_shaped_line_that_is_prose` covers all six positions (raw,
  typed, and plain text content; line and block comments; string literal), and
  `still_reports_a_declaration_that_follows_prose` pins that skipping prose does not cost the report
  for a real declaration later in the same file. Both were confirmed to fail against the original
  scan before the guard was added.
- **Spec:** the requirement already said "in declaration position", so the text was right and the
  implementation was wrong. Added the scenario *A declaration-shaped line in prose is not reported*
  so the negative half is pinned rather than implied, and noted the fix on task 2.3.
- **Verification:** The two regression tests passed, covering all six prose/data positions and a
  real declaration following prose. Re-running the original raw-text reproduction produced the
  rendered `<code>` value with no `removed-enum-keyword` diagnostic, while the existing declaration
  tests still report the targeted replacement. Strict OpenSpec validation also passed.

## Findings Discovered During the 2026-08-31 Fix Pass

These were not in the review. They were found while fixing RF6 and are recorded here so the
reviewing agent can verify or reject them alongside it.

### ✅ Verified - FP1 The change leaves the tree less `cargo fmt`-clean than it found it
- **Severity:** Low
- **Evidence:** `cargo fmt --all --check` reported drift in 18 files. Checking each file's `HEAD`
  revision with `rustfmt --check` separated the two causes: 7 were already drifted at `HEAD`, and
  10 were clean at `HEAD` and drifted only under this change's edits (plus the new
  `crates/nx-syntax/tests/query_tests.rs`). Line-by-line, 14 further hunks in the already-drifted
  files also fell inside lines this change edited.
- **Fix:** Formatted the 10 files that were clean at `HEAD`, and in the 7 that were not, applied
  only the hunks overlapping this change's own edited lines — reformatting those files whole would
  have swept unrelated pre-existing churn into this change's diff. `cargo fmt --all --check` now
  reports drift only on lines this change never touched.
- **Verification:** `cargo fmt --all --check` now reports only the seven files already drifted at
  `HEAD`. Comparing its remaining hunks with `HEAD` and the zero-context working diff confirms that
  those lines are pre-existing and do not belong to this change's edits.

### ✅ Verified - FP2 `SyntaxKind::ENUM` survived the kind removal that task 2.4 claims
- **Severity:** Low
- **Evidence:** The grammar no longer produces an `enum` node, but `crates/nx-syntax/src/syntax_kind.rs`
  still declared the `ENUM` variant, mapped `"enum"` to it in `syntax_kind_from_str`, and listed it
  in both `is_token` and `is_keyword`. `is_keyword(SyntaxKind::ENUM)` therefore answered `true` for
  a keyword the language does not have. Task 2.4 states the removed kinds were dropped from this
  file.
- **Fix:** Removed the variant, its `"enum"` mapping, and both classification entries; `"enum"` now
  falls through to `SyntaxKind::ERROR` like any other non-keyword word. Workspace builds clean and
  the suite is unchanged. Annotated task 2.4, which had claimed this was already done.
- **Verification:** `SyntaxKind::ENUM`, the removed enum node kinds, and their string mappings are
  absent from `syntax_kind.rs`; `"enum"` reaches the default `SyntaxKind::ERROR` mapping. The full
  workspace suite passed.

### ✅ Verified (code owned by archived `contextual-literal-binding`) - FP3 Three union helpers in `nx-types` are now dead code
- **Severity:** Low
- **Evidence:** `cargo build --workspace` warns that `union_type_of`, `nominal_is_nameable_here`,
  and `union_def_for_contextual` in `crates/nx-types/src/infer.rs` are never used.
- **First read was wrong.** This was initially left open on the theory that `union_type_of`, being
  newly added and never called, might be unfinished wiring rather than a leftover. Checking each
  helper against `HEAD` showed otherwise — none is pending work:
  - `union_type_of` is a duplicate, not new functionality. `union_type_from_def` already exists at
    `HEAD`, has three live callers, and computes the identical value: `Type::union_type(name, cases,
    base)` expands to `Type::Union(UnionType::new(name, cases, base))`, which is exactly
    `Type::Union(union_type_shape(def))`.
  - `nominal_is_nameable_here` is superseded. Its body is the `nameable_here` computation now
    inlined at the single call site, unchanged.
  - `union_def_for_contextual` is superseded *and* is the pre-fix version. The inline replacement
    adds `.filter(same_shape)` to both the local and the foreign branch; the helper does the bare
    `union_defs.get(name).or_else(|| foreign_union_defs.get(name))` with no shape check — the exact
    lookup the comment above the call site warns about, where "a same-named local declaration used
    to stand in for a foreign one."
- **Fix:** Removed all three. The third was removed on purpose rather than for tidiness: leaving it
  parks the shape-unchecked lookup under the most inviting name in the file, so the next caller to
  reach for it silently reinstates the bug the surrounding fix removed. Workspace builds with no
  warnings and the suite is unchanged at 1268 passed / 0 failed, as expected for dead code.
- **Ownership:** the code belongs to `contextual-literal-binding`, which was archived on 2026-08-30,
  so there is no live change to file it under; it is recorded here the way that change's own review
  records its cross-change items. It was this change's enum→union rewrite that stranded the three
  helpers, and the archived review's closing claim — "`cargo test` passes with 0 failures and no
  warnings" — had stopped holding in this working tree until this removal. The archive itself is
  left untouched.
- **Verification:** All three helper definitions are absent, their live replacements remain in
  place, and `cargo test --workspace` completed without the prior dead-code warning or any test
  failure.

## Questions
- ~~Is the type-only output in TypeScript CLI typegen intentionally being retained for byte identity
  with the old enum generator? If so, the `as const` requirement in `cli-code-generation` conflicts
  with that decision and needs to be corrected before archive.~~ **Answered: it is retained, but not
  for byte identity — that was only what first exposed the conflict.** Raised with the maintainer
  with backward compatibility explicitly set aside; the union was kept on the merits, because it is
  the same type as the `as const` form, because CLI type generation emits a pure type surface, and
  because adding the value object later is non-breaking while removing it is not. The requirement,
  design D4, `proposal.md`, and task 7.5 have been corrected, and the trigger to revisit is recorded
  in D4. See RF3.

## Summary
- All nine findings are verified: RF1–RF6 and FP1–FP3. No finding was reopened, and no new finding
  was added during this verification pass. FP3's code belongs to the archived
  `contextual-literal-binding`.
- Verification performed: Rust workspace passed; .NET passed 96/96 after rebuilding the release
  native runtime; TypeScript runtime passed 12/12; VS Code grammar passed 82/82; strict OpenSpec
  validation passed.

## Fix pass

All five original findings, RF6, and the three fix-pass findings have now been verified by the
reviewing agent.

Four were code or asset defects (RF1, RF2, RF4, RF5). RF3 was an artifact defect: the
implementation was right and the requirement, the design, the proposal, and task 7.5 were wrong,
each in the same way — conflating CLI type generation with executable codegen, which have emitted
different TypeScript for a constant set since before this change. Because that conflation had gone
unnoticed in four places, the underlying choice was put to the maintainer rather than settled by the
byte-identity guard alone, and was decided on its merits with backward compatibility set aside: the
string-literal union stays, and D4 now records why and when to revisit.

Two of the five (RF2, RF3) were invisible to the suite because nothing exercised the artifact in
question — no test compiled a shipped Tree-sitter query, and no test pinned CLI typegen's generated
form. Both gaps now have tests, and RF2's was confirmed to fail on the original defect.

RF6 was the same kind of gap as RF2 and RF3, one layer down: the requirement said "in declaration
position" and the implementation only checked the *shape* of a line, never its position. The one
negative test it had — the identifier `enumerate` — passed for a reason that had nothing to do with
position, so it could not have caught this. Two of the four false positives the fix removes were
passing only by accident of where `//` and `let` sit on a line.

FP1, FP2, and FP3 were found while fixing RF6 rather than reported: the change had drifted from
`cargo fmt` in ten files that were clean at `HEAD`, `SyntaxKind::ENUM` had outlived the kind removal
task 2.4 claims to have completed, and three union helpers in `nx-types` had been stranded by the
enum→union rewrite. FP3 was first left open on a misread — that one of the three might be unfinished
wiring — and was resolved once each helper was checked against `HEAD` and found to be either a
duplicate of a function that already had callers or the superseded half of a correctness fix.

**Verification after fixes:** the two TypeScript forms were compared under `tsc --strict` to
establish that they are the same type before choosing between them. Rust workspace 1268 passed /
0 failed (up from 1261: two query tests, two `enum`-keyword tests, one typegen shape test, and two
prose-exclusion tests for RF6); .NET 96 passed / 0 failed after `cargo build --release -p nx-ffi`;
VS Code grammar 82 passing; TypeScript runtime 12 passing; strict OpenSpec validation 41 passed /
1 failed, the failure being `spec/contenttype-directive`, which pre-exists at `HEAD` and is
unrelated to this change.
