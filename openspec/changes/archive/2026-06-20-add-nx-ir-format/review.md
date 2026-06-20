# Review: add-nx-ir-format

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md (41/41 complete), specs for `nx-ir-format`,
`typescript-ir-runtime`, `executable-code-generation`, `cli-code-generation`, `dotnet-binding`.

**Reviewed code:**
- `crates/nx-codegen/src/ir.rs` (IR model, lowering, deterministic JSON)
- `crates/nx-codegen/src/emit.rs`, `builder.rs`, `model.rs`, `lib.rs` (relevant diffs)
- `crates/nx-cli/src/main.rs` (`--target nx-ir`)
- `crates/nx-ffi/src/lib.rs`, `cbindgen.toml`, `bindings/c/nx.h`
- `runtime/typescript/src/index.ts` (loader/preparer/evaluator)
- `runtime/typescript/test/runtime.test.ts`, `test/emitted-ir.test.mjs`
- `bindings/dotnet/.../NxProgramArtifact.cs`, `NxRuntime.cs`, `NxGeneratedNxIr.cs`, `NxIrMetadata.cs`, tests
- `docs/nx-ir-format.md`

**Verification performed:**
- `cargo test -p nx-codegen ir` → 5 passed.
- `runtime/typescript` `tsc` build + `node dist/test/runtime.test.js` → 4 suites passed.
- Did not run `emitted-ir.test.mjs` (requires a full `nx-cli` cargo build) or the .NET suite in this pass.

**Fix-pass verification performed:**
- `cargo test -p nx-codegen nx_ir -- --nocapture` → 7 passed.
- `runtime/typescript` `tsc` build + `node dist/test/runtime.test.js` +
  `node test/emitted-ir.test.mjs` → passed.
- `cargo test -p nx-cli nx_ir -- --nocapture` → 4 passed.
- `cargo test -p nx-ffi ffi_codegen_nx_ir -- --nocapture` → 2 passed.
- `dotnet test ... --filter GenerateNxIr` → 3 passed.

**Independent fix-verification pass (2026-06-19 23:55):**
- `cargo test -p nx-codegen nx_ir` → 7 passed (incl. RF1 and RF3 regression tests).
- `cargo test -p nx-cli nx_ir` → 5 passed; `cargo test -p nx-ffi ffi_codegen_nx_ir` → 2 passed.
- `cargo test -p nx-interpreter --test interpreter_direct_hir array_index` → 2 passed (RF5 native).
- `runtime/typescript`: `npm install` + `tsc` build + `node dist/test/runtime.test.js` → 6 suites
  passed (incl. RF3, RF5, RF2 discriminator-only matching).
- `dotnet test --filter NxIr` → 3 passed (after rebuilding `nx-ffi` debug+release; the staged
  native libs were stale at ABI 11 vs the current ABI 12 — environmental, not a defect).
- RF1, RF3, RF4, RF5 verified as correct and complete; RF2 remains Resolved.
- `node test/emitted-ir.test.mjs` → previously failed at CLI invocation (see RF6 below) before this
  review-fix pass.

**Review-fix pass (2026-06-20):**
- `runtime/typescript` `npm test -- --runInBand` → passed (`tsc`, runtime unit tests, and
  `node test/emitted-ir.test.mjs` including RF6/RF7 coverage).

**Independent RF6/RF7 fix-verification pass (2026-06-20):**
- `runtime/typescript`: `npm install` + `npx tsc` (exit 0) + `node dist/test/runtime.test.js` → 7 suites
  passed (incl. "matches native numeric division and modulo semantics").
- `node test/emitted-ir.test.mjs` → 3 cases passed (builds `nx-cli` via cargo): emitted function IR
  (`7 / 2`, `7 % 2`) matches native, division-by-zero IR matches native failure, component IR matches
  generated JS.
- RF6 and RF7 verified correct and complete.

## Findings

### ✅ Verified - RF1 Inherited component field spans are tagged with the wrong source identity
- **Severity:** Medium
- **Evidence:** In `ir_component_fields` ([ir.rs:971-1009](crates/nx-codegen/src/ir.rs#L971-L1009)) the field
  `span` is emitted via `ir_span_for_module(module_id_value, source.clone(), field.span)`, where `source`
  is always the *declaring component's* module source. But `CodegenComponentField` carries its own
  `owner_module_id` ([model.rs:150-158](crates/nx-codegen/src/model.rs#L150-L158)) and `field.span` is in
  the owner module's coordinate space — the same code already uses
  `context.source_for(field.owner_module_id)` for the field's `default` expression source, proving fields
  can originate in another module. For an inherited prop/state field (e.g. `external component
  <ShortTextQuestion extends Question />`), the emitted `span.source` will name the deriving component's
  source while `start`/`end` index into the base module's text. A runtime diagnostic such as
  "Missing required prop" (`normalizeFields` reports `field.span`,
  [index.ts:784](runtime/typescript/src/index.ts#L784)) would then point at the wrong source file/offset,
  contradicting the `nx-ir-format` "preserves source provenance" requirement.
- **Recommendation:** Use `context.source_for(field.owner_module_id)` (the same value computed for
  `default_source`) when building the field span, instead of the component module's `source`.
- **Fix:** Updated `ir_component_fields` to emit each component field span with `field.owner_module_id`
  and added a regression test covering an inherited prop from a cross-module base component.
- **Verification:** Confirmed at [ir.rs:1003-1021](crates/nx-codegen/src/ir.rs#L1003-L1021): the field
  span now uses `default_source` (= `context.source_for(field.owner_module_id)`) and
  `field.owner_module_id`, so the span source matches the owner module's coordinate space.
  Regression test `nx_ir_component_inherited_field_spans_use_owner_source` passes
  (`cargo test -p nx-codegen nx_ir` → 7 passed). Fix is correct and complete.

### ✅ Resolved - RF2 `if is` union-case patterns match on the discriminator only, ignoring field constraints
- **Severity:** Low
- **Evidence:** `patternMatches` ([index.ts:1168-1173](runtime/typescript/src/index.ts#L1168-L1173))
  returns `value.$type === pattern.$type` whenever both operands are objects and the pattern has a string
  `$type`; it never compares the remaining fields. The IR match arm
  (`NxIrMatchArm`, [ir.rs:296-301](crates/nx-codegen/src/ir.rs#L296-L301)) carries pattern expressions
  built from arbitrary `build_expression` output ([builder.rs:737-748](crates/nx-codegen/src/builder.rs#L737-L748)),
  so a pattern like `LoadState.failed { message: "offline" }` evaluates to a full payload but is matched
  purely by case. If the native interpreter treats such patterns structurally (matching the field values),
  the TS runtime will select an arm the interpreter would reject — a parity divergence not covered by the
  current tests (which only exercise case/enum/literal patterns). Enum-member and literal patterns are
  unaffected (they fall through to `deepEqual`).
- **Recommendation:** Confirm native `if is` semantics for union-case patterns with field constraints. If
  fields participate in matching, compare the pattern's non-`$type` fields too (or add a parity test that
  pins the intended behavior).
- **Status:** Confirmed native interpreter semantics in `values_match`: record/union patterns match by
  type name only. Added a TypeScript runtime test pinning discriminator-only matching for a fielded union
  case pattern.

### ✅ Verified - RF3 Name-keyed declaration/entrypoint lookup collides across modules
- **Severity:** Low
- **Evidence:** `declarationsByName` is keyed by the bare `declaration.reference.name` with last-write-wins
  ([index.ts:305](runtime/typescript/src/index.ts#L305)). It backs the entrypoint fallback in
  `evaluateFunction`/`componentDeclaration`
  ([index.ts:354](runtime/typescript/src/index.ts#L354), [index.ts:912](runtime/typescript/src/index.ts#L912))
  and nominal type resolution in `normalizeNominalValue`
  ([index.ts:860](runtime/typescript/src/index.ts#L860)). In a multi-module program with two
  same-named declarations (e.g. a library and root both declaring `User` or `root`), the map resolves to
  whichever module was iterated last, so boundary normalization or a name-based entrypoint call can bind to
  the wrong declaration. Entry calls that go through the `functionEntrypoints`/`componentEntrypoints`
  module-qualified maps are safe; only the name fallback and nominal lookup are exposed.
- **Recommendation:** Resolve nominal types via the module-qualified reference (`declarationsById` using the
  field/type reference) rather than bare name, and prefer module-qualified entrypoint resolution.
- **Fix:** Added resolved `primitive` versus `nominal` IR type references, with nominal references
  carrying module-qualified declaration identities. Updated the TypeScript runtime to normalize
  nominal boundary values via `declarationsById`, validate nominal type references during
  preparation, and remove bare-name fallback lookup from public function/component APIs. Added Rust
  IR coverage for imported nominal type references and TypeScript runtime coverage for same-named
  nominal declaration collisions plus entrypoint-only lookup.
- **Verification:** Confirmed `NxIrTypeRef` now has distinct `Primitive`/`Nominal` variants
  ([ir.rs:357-364](crates/nx-codegen/src/ir.rs#L357-L364)) with nominal carrying a module-qualified
  `reference`, lowered by `ir_type_ref` ([ir.rs:1668-1689](crates/nx-codegen/src/ir.rs#L1668-L1689)).
  The TS runtime resolves nominal boundary values via `declarationsById`
  ([index.ts:864-909](runtime/typescript/src/index.ts#L864-L909)), validates nominal references during
  preparation ([index.ts:1006-1023](runtime/typescript/src/index.ts#L1006-L1023)), and the public
  function/component entrypoints use only the module-qualified `functionEntrypoints`/`componentEntrypoints`
  maps with no bare-name `declarationsByName` fallback (that map no longer exists). Tests
  `nx_ir_nominal_type_refs_are_module_qualified` (Rust) and "uses module-qualified nominal type
  references and entrypoint-only lookup" (TS) pass. Fix is correct and complete.

### ✅ Verified - RF4 `programFingerprint` can lose precision when parsed in JavaScript
- **Severity:** Low
- **Evidence:** The fingerprint is a Rust `u64` ([ir.rs:53](crates/nx-codegen/src/ir.rs#L53)) emitted as a
  bare JSON number, but the TS `NxIrProgram.programFingerprint` is typed `number`
  ([index.ts:31](runtime/typescript/src/index.ts#L31)) and obtained through `JSON.parse`
  ([index.ts:1135](runtime/typescript/src/index.ts#L1135)). Fingerprints above 2^53 silently lose
  precision on the JS side. The runtime never uses the parsed value semantically and cache identity is on
  the JSON bytes, so this is latent — but any consumer comparing the parsed fingerprint would get wrong
  results. (`NxIrMetadata.ProgramFingerprint` on the .NET side is correctly `ulong`.)
- **Recommendation:** Document that cache identity is byte-level on the JSON, and/or surface the fingerprint
  as a string in the TS surface if consumers are expected to compare it.
- **Fix:** Changed the IR document `programFingerprint` to a decimal string, updated the TypeScript type
  and tests, and documented that byte-level JSON identity is authoritative while structured native/.NET
  metadata may still expose integer fingerprint values.
- **Verification:** Confirmed the IR document field is serialized as a decimal string via the
  `u64_decimal_string` serde adapter ([ir.rs:23-40](crates/nx-codegen/src/ir.rs#L23-L40),
  [ir.rs:72-73](crates/nx-codegen/src/ir.rs#L72-L73)) and the TS type is `string`
  ([index.ts:31](runtime/typescript/src/index.ts#L31)). Real CLI output shows
  `"programFingerprint": "6829579808617076204"`. The structured `NxIrMetadata.program_fingerprint`
  remains a numeric `u64` (no string adapter, [ir.rs:50-51](crates/nx-codegen/src/ir.rs#L50-L51)), which
  the .NET `NxIrMetadata.ProgramFingerprint` (`ulong`) deserializes correctly — the .NET IR tests pass.
  Encodings are internally consistent. Fix is correct and complete.

### ✅ Verified - RF5 Array index out-of-bounds returns null rather than matching interpreter behavior
- **Severity:** Low
- **Evidence:** `evalIndex` returns `base[index] ?? null` ([index.ts:1118-1123](runtime/typescript/src/index.ts#L1118-L1123)),
  so out-of-bounds and negative indices yield `null`. Whether this matches the native interpreter (which
  may raise) is unverified, and no parity test covers out-of-range indexing.
- **Recommendation:** Confirm native indexing semantics and add a parity test; reject or match as
  appropriate.
- **Fix:** Implemented native interpreter `Index` evaluation with integer and bounds checks, changed the
  TypeScript IR runtime to raise diagnostics for negative or out-of-bounds indexes instead of returning
  `null`, and added direct-HIR/native plus TypeScript IR regression tests. Updated the OpenSpec specs and
  reference docs to pin the bounds behavior.
- **Verification:** Confirmed `evalIndex` ([index.ts:1200-1219](runtime/typescript/src/index.ts#L1200-L1219))
  rejects non-array bases, non-integer indexes, and negative/out-of-bounds indexes with diagnostics; the
  native `eval_index` ([interpreter.rs:2860-2904](crates/nx-interpreter/src/interpreter.rs#L2860-L2904))
  raises `ArrayIndexOutOfBounds`/`TypeMismatch` for the same cases, so behavior matches. Tests
  `test_array_index_out_of_bounds_direct_hir` (native) and "rejects out-of-bounds array indexes" (TS)
  pass. Fix is correct and complete.

## New Findings Discovered During 2026-06-19 23:55 Verification

### ✅ Verified - RF6 Cross-runtime parity test invokes a CLI interface that no longer exists
- **Severity:** Medium
- **Evidence:** `emitToIr` in the parity suite runs the CLI with
  `["codegen", sourcePath, "--target", "javascript", "--format", "nx-ir", "--output", outputPath]`
  ([emitted-ir.test.mjs:68](runtime/typescript/test/emitted-ir.test.mjs#L68)). The CLI's `--format`
  value enum now only accepts `files` and `program-module`
  ([main.rs:135-140](crates/nx-cli/src/main.rs#L135-L140)); IR is requested via `--target nx-ir`
  ([main.rs:127-133](crates/nx-cli/src/main.rs#L127-L133), [main.rs:358-364](crates/nx-cli/src/main.rs#L358-L364)).
  Running the suite fails immediately with `invalid value 'nx-ir' for '--format <FORMAT>'`. The CLI
  change is deliberate and pinned by `test_cli_codegen_nx_ir_is_not_source_output_format`
  ([main.rs:1634-1654](crates/nx-cli/src/main.rs#L1634-L1654)), which asserts exactly this invocation is
  rejected — but the `.mjs` parity test was not updated to match. As a result the cross-runtime parity
  coverage that backs tasks 6.1/6.2 (IR-vs-interpreter and IR-vs-generated-JS) is effectively disabled,
  and the earlier "Fix-pass verification" claim that `emitted-ir.test.mjs` passed is no longer true
  against the current CLI.
- **Recommendation:** Update [emitted-ir.test.mjs:68](runtime/typescript/test/emitted-ir.test.mjs#L68)
  to `["codegen", sourcePath, "--target", "nx-ir", "--output", outputPath]` (drop `--format` and the
  `--target javascript`). Manually confirmed that `nxlang codegen <src> --target nx-ir --output <dir>`
  writes `test.nxir.json` as expected, so this single-line change should restore the suite.
- **Fix:** Updated the emitted-IR parity helper to invoke `nxlang codegen` through `--target nx-ir`
  and added end-to-end parity coverage for integer division/modulo plus division-by-zero failure.
- **Verification:** Confirmed `emitIr` now runs `["codegen", sourcePath, "--target", "nx-ir",
  "--output", outputPath]` ([emitted-ir.test.mjs:66-70](runtime/typescript/test/emitted-ir.test.mjs#L66-L70))
  with no `--format`/`--target javascript`, matching the live CLI surface. Ran
  `node test/emitted-ir.test.mjs` end-to-end (builds `nx-cli` via cargo) → all 3 cases passed,
  including "emitted function IR matches native interpreter output". The suite is no longer disabled.
  Fix is correct and complete.

### ✅ Verified - RF7 Integer division and division-by-zero diverge from the native interpreter
- **Severity:** Medium
- **Evidence:** `evalBinary` evaluates `div` as `checkedNumber(lhs) / checkedNumber(rhs)` and `mod` as
  `%` ([index.ts:1147-1180](runtime/typescript/src/index.ts#L1147-L1180)) using JavaScript floating
  semantics with no zero check. The native interpreter performs integer-truncating division on integer
  operands (`a / b` on `i64`/`i32`) and raises `DivisionByZero`
  ([arithmetic.rs:95-151](crates/nx-interpreter/src/eval/arithmetic.rs#L95-L151), and the analogous
  `eval_mod` at [arithmetic.rs:153-208](crates/nx-interpreter/src/eval/arithmetic.rs#L153-L208)). So for
  an integer program `7 / 2` the interpreter yields `3` while the IR runtime yields `3.5`, and `x / 0`
  raises in the interpreter but silently produces `Infinity`/`NaN` in the IR runtime. This is the same
  class of interpreter-parity gap that RF5 fixed for indexing, and the design explicitly treats
  interpreter behavior as the semantic oracle. No TS unit test or parity test currently exercises
  division/modulo, so the divergence is uncovered.
- **Recommendation:** Make integer `div`/`mod` truncate toward zero when both operands are integers
  (the expression carries `ty`/semantic-type metadata that can distinguish int from float), and raise an
  `nx-ir`-coded diagnostic on division/modulo by zero to match `DivisionByZero`. Add a parity test
  covering integer division, integer modulo, and division-by-zero. Confirm the intended float-division
  semantics for float operands and pin them with a test as well.
- **Fix:** Updated the TypeScript IR runtime to raise `Division by zero` for `div`/`mod`, truncate
  integer division toward zero when the IR semantic type is integer, preserve fractional float
  division, and added runtime plus emitted-IR parity coverage for the affected cases.
- **Verification:** Confirmed `evalBinary` routes `div`/`mod` through `evalDivision`/`evalModulo`
  ([index.ts:1161-1164](runtime/typescript/src/index.ts#L1161-L1164)), which `fail` with
  `nx-ir-division-by-zero` on a zero divisor and, when `isIntegerSemanticType(expression.ty)` holds,
  validate integer operands and `Math.trunc` the quotient (`mod` uses JS `%`, which truncates toward
  zero like Rust's `%`) ([index.ts:1188-1223](runtime/typescript/src/index.ts#L1188-L1223)). The
  integer-type check keys on `shape.kind === "primitive"` with name `int`/`i32`/`i64`, matching the
  Rust `primitive_name` lowering ([ir.rs:1796-1808](crates/nx-codegen/src/ir.rs#L1796-L1808)) and the
  `expression.ty` carried on each IR expression; float types fall through to JS division. Ran the TS
  unit suite ("matches native numeric division and modulo semantics" passes) and
  `node test/emitted-ir.test.mjs`, which emits `7 / 2`/`7 % 2` and `1 / 0` IR through the CLI and
  confirms IR-runtime output equals the native interpreter (`3`+`1`) and that both raise on
  division-by-zero. Fix is correct and complete.

## Questions
- RF5 has been answered by this fix: array indexing requires an integer index, and negative or
  out-of-bounds indexes fail with a runtime diagnostic.
- The cross-runtime parity test (`emitted-ir.test.mjs`) and .NET IR binding suite are listed in
  fix-pass verification above.

## Summary
Solid, well-structured implementation that satisfies the spec scenarios. The five original findings are
resolved: RF1, RF3, RF4, and RF5 are verified correct and complete against the current code and tests,
and RF2 remains Resolved (confirmed discriminator-only union-case matching parity). Action-handler
rejection (task 1.6) is correctly wired through `build_expression` → `Unsupported` →
`validate_ir_program`. The CLI short-circuits source emission for `--target nx-ir` and is host-neutral as
required, and the FFI/.NET surfaces line up on field names and types (the only .NET test failure observed
was stale native libraries at ABI 11 vs the current ABI 12 — resolved by rebuilding `nx-ffi`).

Two new findings were discovered during an earlier verification pass: RF6 (the cross-runtime parity test
`emitted-ir.test.mjs` invoked a removed `--format nx-ir` CLI interface) and RF7 (integer `div`/`mod`
and division-by-zero diverged from the native interpreter's integer-truncating, fault-raising
semantics). Both have now been independently verified correct and complete (2026-06-20): the parity
suite invokes `--target nx-ir`, the TS runtime truncates integer division and raises on division-by-zero
matching the native interpreter, and the emitted-IR parity tests pass end-to-end. All findings are
resolved — the change is ready to archive.
