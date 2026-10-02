# Review: add-function-reference-type

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, verification.md, specs/function-reference-type/spec.md, specs/nx-ir-format/spec.md, specs/typescript-ir-runtime/spec.md, specs/rust-ir-runtime/spec.md, specs/cli-code-generation/spec.md  
**Reviewed code:** the uncommitted working tree (`git diff` plus the untracked `crates/nx-syntax/tests/fixtures/valid/function-reference-type.nx`, `crates/nx-types/tests/function_reference_type.rs`, `specs/ir-conformance/function-references/`): `crates/nx-syntax` (grammar, validation, highlight query, tests), `crates/nx-hir` (`ast/types.rs`, `lower.rs`, `components.rs`, `lib.rs`), `crates/nx-types` (`ty.rs`, `infer.rs`, `semantics.rs`), `crates/nx-codegen` (`builder.rs`, `ir.rs`, `emit.rs`, `model.rs`, tests), `crates/nx-ir` (`model.rs`, `image.rs`, `explain.rs`), `crates/nx-ir-runtime` (`module.rs`, `tests/prepare_link.rs`), `crates/nx-interpreter`, `crates/nx-api/src/artifacts.rs`, `crates/nx-cli/src/typegen`, `crates/nx-language-service`, `runtime/typescript` (`src/index.ts`, tests, README), `bindings/dotnet` test, docs (`nx-grammar.md`, `nx-grammar-spec.md`, `docs/nx-ir-format.md`, website `types.md` / `functions.md`, conformance README), `src/vscode` grammar, test and changelog.  

**What was run:**
- `cargo test --workspace`: passes (80 test binaries, no failures).
- `pnpm test` in `runtime/typescript`: passes, and the rebuilt `dist/` is identical to the checked-in one.
- `npm test` in `sites/website`: passes (185 nx blocks, 0 failures).
- `dotnet test bindings/dotnet/NxLang.sln --filter NxFunctionValueTests`: passes with `-p:NxSdkNativeLibraryConfiguration=Debug`; fails in the default configuration on this machine (see Questions).
- `nxlang run`, `nxlang codegen --target nx-ir`, `nxlang ir explain`, `nxlang codegen --target typescript` and `nxlang typegen` on probe programs for each spec scenario. `tsc --strict --noEmit` on the generated TypeScript.
- Not run: the VS Code grammar test and the full .NET suite.

**Match-site audit:** every `match`, `matches!` and `if let` over `nx_hir::ast::TypeRef::Function`, `nx_types::Type::Function`, `CodegenTypeRef::Function`, IR type kind `4` and the TypeScript `PreparedType` kind `"function"` was checked. No site that needs the new variant is missing it. The remaining `Type::Function`-only sites (`function_typed_value`, `infer_call`, `type_from_function_binding`, `entry_result_type`, the language-service function-name hover) read a declared function's own type or a callable function type, where excluding the new variant is correct.

## Findings

### ✅ Verified - RF1 Generated TypeScript for a function reference site does not type-check
- **Severity:** Medium
- **Confidence:** High (reproduced with `tsc`)
- **Evidence:** `emit_any_function_type` (`crates/nx-codegen/src/emit.rs:4538`) emits `(args: never) => R`, and design D8 says "every generated function is assignable to it". That is false in two ways.
  - A paren function is emitted with positional parameters, so one with two or more required parameters is not assignable. `type AnyFn = <function ... />: object*`, `let add(a:int, b:int): int`, `pass(add)` gives `error TS2345: Argument of type '(a: number, b: number) => number' is not assignable to parameter of type 'AnyFn'. Target signature provides too few arguments. Expected 2 or more, but got 1.`
  - The result is emitted as the NX result's TypeScript type, and `object*` becomes `readonly object[]`. NX accepts any result at `<function ... />: object*`; TypeScript does not: `type Tool = { fn: <function ... />: object* }`, `let double(n:int): int`, `<Tool fn={double} />` gives `error TS2322: Type '(n: number) => number' is not assignable to type '(args: never) => readonly object[]'. Type 'number' is not assignable to type 'readonly object[]'.`

  The headline use of the feature (`<function ... />: object*` holding an arbitrary function) therefore produces `--target typescript` output that fails `tsc --strict`. The JavaScript target is unaffected, which is why the corpus test passes. A function type with stated parameters has the same arity problem today (`fn: <function a:int b:int />: object*` fails the same way), so the class is pre-existing, but this change adds a new emitted shape and a claim about it that does not hold.
- **Recommendation:** Emit a type every generated function is assignable to, for example `(...args: never[]) => unknown` (or map the result so that an NX-compatible result is TypeScript-assignable, which `readonly object[]` is not). Add a test that type-checks generated TypeScript for the `function-references` corpus program, and correct D8.
- **Fix:** `emit_any_function_type` (`crates/nx-codegen/src/emit.rs`) now emits `(...args: never[]) => unknown`, which every generated function is assignable to whatever its arity or calling shape. The result is `unknown` for every `R`, because NX admits results TypeScript's spelling of `R` does not (`int` at `object*`, `string` at `string*` by the lift). New test `generated_typescript_for_function_reference_sites_type_checks` (`crates/nx-codegen/src/tests.rs`) runs `tsc --strict` over a program binding a one-parameter, a two-parameter and an element function, and a lifted result. Design D8 is corrected.
- **Status:** The test does not use the corpus program: generated TypeScript for any record that has both an optional field and a defaulted field fails `tsc` on the unmodified tree (`type Tool = { name?:string label:string = "x" }`, `<Tool />` gives `readonly never[]` not assignable to `string`), which is unrelated to this change. So is the arity mismatch for a paren function at a function type with stated parameters. Both are left for a separate change and are recorded in `specs/future.md` under the two "Generated TypeScript:" sections.
- **Verification:** Confirmed. `emit_any_function_type` emits `(...args: never[]) => unknown`; regenerating my two failing probes (a two-parameter paren function passed to an `AnyFn` parameter, and `double` bound at `fn: <function ... />: object*`) now passes `tsc --strict`. The new test runs `tsc` and panics rather than skips when `tsc` is absent. Design D8 matches, and no `(args: never)` text remains in code or docs. The part left alone is acceptable: `type Tool = { name?:string label:string = "x" }` with `<Tool />` fails `tsc` with no function type involved, so the corpus program cannot be the test input until that separate defect is fixed.

### ✅ Verified - RF2 `==` is rejected between function reference values whose declared types are unrelated, even when both can hold the same function
- **Severity:** Low
- **Confidence:** Medium (behavior reproduced; whether the spec forbids it is a reading of one sentence)
- **Evidence:** The spec says two such values compare by declaration and "the comparison SHALL NOT depend on the type either side was declared at". The checker admits `==` only when one side's type satisfies the other (`items_comparable`, `crates/nx-types/src/infer.rs:592`, reported at `:2350`). So:
  - `let same(f: <function ... />: int, g: <function n:int />: object): boolean = {f == g}` is rejected with `Cannot compare types <function ... />: int and <function n:int />: object`, although `double(n:int): int` satisfies both parameters and the two values can be equal.
  - `f: <function ... />: int` against `g: <function ... />: string` is rejected the same way.

  The two spec scenarios pass only because one side is `<function ... />: object*`. Function types with stated parameters have the same restriction today.
- **Recommendation:** Treat any two function types (either variant) as comparable for `==`, `!=` and match patterns, or narrow the spec sentence to say comparison requires one type to satisfy the other. Add a test for the unrelated-result case either way.
- **Fix:** `items_comparable` (`crates/nx-types/src/infer.rs`) accepts `==` and `!=` between any two function types, with stated or unspecified parameters. The spec sentence now says so explicitly. New test `function_values_compare_whatever_types_they_were_declared_at` covers unrelated results, two stated types, and that a function still does not compare with a string.
- **Verification:** Confirmed. `items_comparable` accepts any two function types; `<function ... />: int == <function n:int />: object`, two unrelated function reference types and two unrelated stated types now check, while a function against a string or an int is still rejected and `<` between functions is still rejected. The spec sentence states the rule.

### ✅ Verified - RF3 The not-callable diagnostic is skipped for an optional or sequence binding and for a binding that shares a record's name
- **Severity:** Low
- **Confidence:** High for the behavior; Medium that it needs fixing (the stated-parameter function type behaves the same way in each case)
- **Evidence:** The check in `infer_element_expression` (`crates/nx-types/src/infer.rs:3099`) and in the call path (`:739`) tests `is_any_function()` on the binding's exact type.
  - `let opt(f?: <function ... />: int): int = <f n=1 />` reports `Return value for function 'opt' expects int, found f`, which is the fall-through message D5 set out to replace. `(<function ... />: int)+` does the same. The paren form reports `Cannot call non-function type (<function ... />: int)?`.
  - `type Card = { title:string }` with `let b(Card: <function ... />: object*): object = <Card title="x" />`, or the same as a component prop, reports nothing: the element constructs the record. The requirement says an element whose tag is such a binding SHALL be rejected.
- **Recommendation:** Test the binding's item type (`ty.item().is_any_function()`) so an optional or sequence binding gets `function-reference-not-callable`, and decide whether a binding that shadows a record name should be reported; add a test for each.
- **Fix:** Both call paths now test the binding's item type, so an optional or sequence binding reports `function-reference-not-callable` (test `an_optional_or_sequence_function_reference_binding_is_not_invocable`), and the requirement names that case.
- **Status:** The binding that shares a record's name is not changed: `<Card ... />` where `Card` is a declared record lowers to a record construction before the checker sees a tag, and a function-typed binding with stated parameters behaves the same way today. Deciding whether a value binding should shadow a type name at an element is a rule for all bindings, not this type; it is recorded in `specs/future.md` under "A function-typed binding named after a record does not shadow it at a tag".
- **Verification:** Confirmed. `f?: <function ... />: int` and `(<function ... />: int)+` bindings now report `function-reference-not-callable` for both `<f n=1 />` and `f(1)`. The record-name case is unchanged (`<Card title="x" />` still constructs the record); leaving it is acceptable, since a function-typed binding with stated parameters does the same and the precedence of a declared record over a value binding at a tag is a rule for all bindings.

### ✅ Verified - RF4 A diagnostic suggests a function reference type under a suffix without parentheses
- **Severity:** Low
- **Confidence:** High (reproduced)
- **Evidence:** `check_property_slot` (`crates/nx-types/src/infer.rs:6393`) builds its fix as `format!("{name}?:{base}+")` from the item type's plain display. For `type B = { c:(<function ... />: int)* }` the message ends ``write `c?:<function ... />: int+` ``, and for `c:AnyFn*` it ends ``write `c?:<function ... />: object*+` ``. The first means something else (the suffix binds to the result) and the second does not parse. The display requirement says the type SHALL be parenthesized under a suffix wherever it is shown to an author. A function type with stated parameters is misprinted the same way today.
- **Recommendation:** Build the fix through the postfix display used elsewhere (`write_postfix_type`, or the HIR `spell_type_ref_under_suffix`), and add a test for a function type and a function reference type.
- **Fix:** `check_property_slot` builds the suggestion through the type's own display, so a function type under `+` keeps its parentheses: `c?:(<function ... />: int)+`. Test `a_suggested_property_form_keeps_the_parentheses` covers a function reference type written out and through an alias, a function type with stated parameters, and the `?` case, which needs none.
- **Verification:** Confirmed. The suggestions now read `c?:(<function ... />: int)+`, `c?:(<function ... />: object*)+` and `e?:(<function n:int />: int)+`; the `?` case stays `d?:<function ... />: int`, and `g?:string+` is unchanged.

### ✅ Verified - RF5 Two tests do not exercise what their task and scenario describe
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - Task 7.4 and the scenario "A generated contract round-trips a rendered value" call for evaluating into the generated `Tool` type. `FunctionRef_AtAFunctionReferenceMember_RoundTripsThroughBothOutputFormats` (`bindings/dotnet/tests/NxLang.Sdk.Tests/NxFunctionValueTests.cs`) uses a hand-written `ToolRecord`, so a drift in the C# emitter's output would not fail it.
  - The `rust-ir-runtime` scenario "A host-supplied record is validated against the program" supplies a `Tool` whose `fn` is bad and expects `nx-ir-boundary-type` naming `fn`. `a_function_reference_site_takes_any_function_of_the_program` (`crates/nx-ir-runtime/tests/prepare_link.rs`) supplies the record at the parameter `f` of `wrap` and asserts only the text "to be a function value", so the record-field site and the name in the diagnostic are untested in Rust. The TypeScript test does cover the field.
- **Recommendation:** Add a Rust runtime case that passes a `Tool` record through a `Tool`-typed parameter and asserts the diagnostic names `fn` (the corpus program needs a function taking a `Tool`, or the test can build its own image). For .NET, either generate the contract in the test or note in the task that a hand-written mirror is the accepted substitute.
- **Fix:** The corpus program gained `passTool(tool:Tool): Tool`, and `a_host_supplied_record_with_a_function_reference_field_is_validated` (`crates/nx-ir-runtime/tests/prepare_link.rs`) supplies a whole `Tool` and asserts `nx-ir-function-value` naming `Nope` and `nx-ir-boundary-type` naming `fn`.
- **Status:** The .NET test keeps its hand-written `ToolRecord`. That is how the existing function-value tests in the same file mirror a generated contract, the SDK test project has no typegen step, and the emitter's output for the member is pinned by the `nx-cli` typegen tests.
- **Verification:** Confirmed. The corpus program has `passTool`, its expected files are regenerated, no other corpus image changed, and the new Rust test supplies a whole `Tool` and asserts both codes and the names; `cargo test --workspace` and the TypeScript corpus tests pass on the changed program. Keeping the hand-written .NET `ToolRecord` is acceptable given the emitter output is pinned by the typegen tests.

## Questions
- `dotnet test bindings/dotnet/NxLang.sln` in the default configuration stages `target/release/libnx_ffi.so`, which on this machine is dated 2026-09-23 and predates the change, so the new .NET test fails with `NX evaluation failed`. It passes with `-p:NxSdkNativeLibraryConfiguration=Debug`. Was task 10.1 verified against a rebuilt release library, or against the debug one?
- The requirement "A function reference value is not invocable" and task 2.6 name a local `let`. The grammar has no local `let` (`let` exists only as a top-level value or function definition), so that case cannot be written or tested. Should the wording be dropped from the spec and the task?
- RF2: is rejecting `==` between function types neither of which satisfies the other the intended rule, with the spec sentence meaning only the runtime result?

## Answers to the questions
- **.NET configuration:** task 10.1 was first verified against the debug native library. `cargo build --release -p nx-ffi` has now been run and `dotnet test bindings/dotnet/NxLang.sln` passes in the default configuration (170 tests).
- **Local `let`:** the wording is dropped from the requirement and from task 2.6; NX has no local `let`.
- **RF2 intent:** comparison should not depend on the declared types; see the RF2 fix.

## Summary
- The implementation matches the delta specs closely. Every scenario I probed in the checker, IR emission, `ir explain`, both IR runtimes, the HIR interpreter and `typegen` behaves as specified, the feature gating is correct (a declaration-only module lists `function-reference-type-v1` alone, a module without `...` lists nothing new, and no other corpus image changed), and no match site for the new variants is missing.
- Five findings are open: one Medium (RF1, generated TypeScript from `codegen --target typescript` fails `tsc` at function reference sites, contradicting D8) and four Low (RF2 to RF5). RF2, RF3 and RF4 are behaviors the new type inherits from function types with stated parameters.
- The test suites listed under Scope pass, with the .NET caveat in Questions.

## Verification 2026-10-02
- All five findings are verified; none reopened and no new findings.
- Re-run after the fixes: `cargo test --workspace`, `pnpm test` in `runtime/typescript` (rebuilt `dist/` identical to the checked-in one), `npm test` in `sites/website`, `openspec validate add-function-reference-type --strict`, and `dotnet test bindings/dotnet/NxLang.sln` in the default configuration (170 passed, against the release native library rebuilt at 13:36). All pass.
- The answers to the questions are accepted: the local-`let` wording is gone from the requirement and task 2.6.
- Still open by decision, not by defect in this change: a value binding that shares a declared record's name at an element tag (RF3), and two `--target typescript` type-check failures that predate the change (RF1 status note).
