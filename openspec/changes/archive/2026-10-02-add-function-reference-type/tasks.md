## 1. Syntax

- [x] 1.1 Accept `...` as the parameter list of `function_type` in `crates/nx-syntax/grammar.js`, regenerate the parser, and verify a corpus test parses `<function ... />: R`, `<function ... />: string?` and `(<function ... />: string)+` with no error nodes, and that `let function = 1`, `0..n` and `0..=n` still parse as before
- [x] 1.2 Reject `...` beside parameter definitions in `crates/nx-syntax/src/validation.rs` with a diagnostic saying `...` stands for the whole parameter list; verify tests for `<function Item:string ... />: string` and `<function ... Item:string />: string`, and that `let <Row ... />: string = "r"`, `type T = { item:... }` and `type T = <function ... />` are parse errors
- [x] 1.3 Add `crates/nx-syntax/tests/fixtures/valid/function-reference-type.nx` (a field at `<function ... />: R`, an alias of one, the alias under each occurrence and the optional mark, the written-out type under each occurrence in parentheses, one as a function-type parameter and result, a default naming a function, and an alias of one as a type argument) and verify it parses with no error nodes
- [x] 1.4 Scope `...` in the highlight queries under `crates/nx-syntax/queries` and the VS Code grammar, and verify a highlighting test for `<function ... />: R`

## 2. HIR and type checker

- [x] 2.1 Record unspecified parameters on `ast::TypeRef::Function` in `crates/nx-hir`, lower it in `lower_type`, print `...` in `spell_function_type` and `spell_type_ref`, and recurse into the result in `erase_type_parameters`; verify lowering and spelling tests, including the parenthesized form under a suffix
- [x] 2.2 Add the function reference variant to `Type` in `crates/nx-types/src/ty.rs` with a constructor and predicate, resolve the type reference to it in `crates/nx-types/src/infer.rs`, update every exhaustive match over `Type` (substitution, erasure, `mentions_never`, display helpers), and verify `cargo test -p nx-types` compiles and passes and that "Function is not a type name" still reports `unresolved-type`
- [x] 2.3 Add the compatibility arms in `Type::is_compatible_with` and `type_satisfies_expected`: a function type of any parameters, or a function reference type, satisfies `<function ... />: R` when its result satisfies `R`; a function reference type satisfies only a function reference type of a wider result and `object`; verify unit tests for "Functions of unlike signatures satisfy the widest type", "An exactly-one result excludes a result under an occurrence", "The result is checked", "The result is covariant", "A function-typed binding satisfies it", "A non-function is rejected", "An element named Function is not a function value", "A narrower result satisfies a wider one", "A function reference value is an object" and "A function reference parameter is checked contravariantly"
- [x] 2.4 Report a function reference value at a site typed by a function type with stated parameters with a mismatch message saying the value's parameters are not stated, and a result mismatch with the existing function-type result message; verify tests for "A function reference value is not a callable function value" and "The result is checked" assert the message text
- [x] 2.5 Print the type as `<function ... />: R`, parenthesized under a suffix, in `Display`, `qualified_display` and `write_postfix_type`; verify display tests for the three display scenarios
- [x] 2.6 Reject `<f ... />` and `f(...)` on a parameter, prop and top-level `let` of a function reference type (NX has no local `let`), including one under the optional mark or an occurrence, in `infer_element_expression` and `infer_call` with the diagnostic `function-reference-not-callable`, ahead of the undeclared-element fall-through; verify tests for the three "is not invocable" scenarios, including that `Row` is not reported as an unknown element
- [x] 2.7 Return `<function ... />: object*` from `common_item_supertype` in `crates/nx-types/src/semantics.rs` when both sides are function types, of either variant, and neither satisfies the other; verify tests for the three join scenarios, and update any existing snapshot that printed `object` for a list of unlike functions
- [x] 2.8 Verify occurrence, default and type-argument behavior with tests for "Occurrences over a function reference type", "An empty value is rejected where one is required", "A default names a function" and "A function reference type is a type argument"
- [x] 2.9 Verify `==` between two function reference values and between one and a function-typed value type-checks ("Function reference values compare by declaration", "A function reference value compares with a function-typed value")

## 3. IR and code generation

- [x] 3.1 Add type kind `ANY_FUNCTION = 6` and its name to `crates/nx-ir/src/model.rs`, and the feature constant `NX_IR_REQUIRED_FEATURE_FUNCTION_REFERENCE_TYPE_V1 = "function-reference-type-v1"`; validate the entry (the kind and one type operand) in `crates/nx-ir/src/image.rs` and print it as `<function ... />: R` in `crates/nx-ir/src/explain.rs`; verify `ir_image_tests.rs` tests refuse a kind-6 entry with no operand, with two, and with an operand past the type table
- [x] 3.2 Add the function reference case to `CodegenTypeRef` in `crates/nx-codegen/src/model.rs`, produce it in `builder.rs`, emit and intern it in `ir.rs`, and add the feature when the type table holds an entry of the kind; verify `ir_tests.rs` tests over explained text for "A function reference field is typed by the function reference kind", "A stated result is the entry's operand" and "A module that uses the type lists its feature"
- [x] 3.3 Map the new `CodegenTypeRef` case to `nxAnySchema` in `crates/nx-codegen/src/emit.rs`, as a function type is, and verify a generated-JavaScript test evaluates a record with a function reference field
- [x] 3.4 Add `specs/ir-conformance/function-references/` (`main.nx`, `program.json`) covering a field typed `<function ... />: object*`, an optional one, one under `+`, a field typed `<function ... />: R` for a record `R`, functions of unlike signatures, a function reference parameter and result, `==` on two such values and a field default; run `NX_UPDATE_CORPUS=1 cargo test -p nx-codegen --lib ir_corpus`, review the explained text and `results.json`, and verify `manifest.json` lists the new type kind as covered
- [x] 3.5 Verify every other corpus image is byte-for-byte unchanged (`git status specs/ir-conformance` shows only the new directory and `manifest.json`), which is the "lists nothing new" scenario

## 4. HIR interpreter and nx-api (skip if `retire-hir-interpreter` has landed)

- [x] 4.1 Accept a `Value::Function` at a site typed by a function reference type wherever `crates/nx-interpreter/src/interpreter.rs` tests `Type::Function` when coercing a value, and verify an interpreter test evaluates `<Tool fn={double} />` and the default in `type Tool = { fn: <function ... />: object* = {identity} }`
- [x] 4.2 Verify `nx-api` renders the field as `{ $type: "Function", module, name }` through `nxlang run` and that the new corpus program's recorded results match the interpreter's

## 5. Rust IR runtime

- [x] 5.1 Read type kind `6` in `crates/nx-ir-runtime/src/module.rs`, validating its operand, into the existing signature-less function type, and add the feature to the supported set; verify `tests/prepare_link.rs` prepares a module that lists `function-reference-type-v1` and refuses one listing an unknown feature
- [x] 5.2 Verify boundary behavior in a runtime test for the `rust-ir-runtime` scenarios: a rendered function reference field, a host record naming a function of another signature (accepted), one naming `Nope` (`nx-ir-function-value`), and a string (`nx-ir-boundary-type`)
- [x] 5.3 Run `cargo test -p nx-ir-runtime` and verify `tests/corpus.rs` passes the new program and `tests/damage.rs` covers its images without a panic

## 6. TypeScript IR runtime

- [x] 6.1 Add kind `6` to the type reader and validators in `runtime/typescript/src/index.ts` as `{ kind: "anyFunction", result }`, export `NX_IR_REQUIRED_FEATURE_FUNCTION_REFERENCE_TYPE_V1` and add it to the supported features; verify a `runtime.test.ts` test that a module listing the feature is refused with `nx-ir-required-feature` when the feature is removed from the supported set, and prepared otherwise
- [x] 6.2 Handle `anyFunction` in `normalizeValue` with the function branch (program function value, else host `Function` record resolved against the linked program, else `nx-ir-boundary-type`), without comparing the result; verify tests for each `typescript-ir-runtime` scenario: rendered record, a field with a stated result, host function of any signature, record naming no function, record naming an unlinked module, record naming a non-function declaration, non-function values, and occurrences
- [x] 6.3 Export the type `NxFunctionRecord` and verify a type-level test assigns the result field of `evaluateFunction` narrowed by `$type === "Function"` to it and passes it to `callFunction`
- [x] 6.4 Verify "A record from a function reference field is callable by its own parameters": `callFunction(program, record, { n: 4 })` returns `8` and `{ n: "four" }` fails naming `n`
- [x] 6.5 Run `npm test` in `runtime/typescript` and verify `test/corpus.test.mjs` and `test/emitted-ir.test.mjs` pass the new program with results identical to the recorded ones, and that the damage test still refuses or reads every overwritten cell

## 7. Generated types

- [x] 7.1 Carry a function type with unspecified parameters through `crates/nx-cli/src/typegen/model.rs`; verify a model test for a member typed `<function ... />: R` and for one typed by an alias of it
- [x] 7.2 Map it to `global::NxLang.Nx.NxFunctionRef` in `crates/nx-cli/src/typegen/languages/csharp.rs`, including under `?`, `+`, `*` and in the update and property companions; verify the typegen tests for "C# types a function reference member as the function reference" and "A stated result maps to the same record type"
- [x] 7.3 Map it to `NxFunctionRef` in `crates/nx-cli/src/typegen/languages/typescript.rs`, emitting the interface once into the helper module for library output and inline for single-file output only when referenced; verify typegen tests for the two TypeScript scenarios and for "Output without a function reference member is unchanged"
- [x] 7.4 Add a case to `bindings/dotnet/tests/NxLang.Sdk.Tests/NxFunctionValueTests.cs` that evaluates a record with a function reference field into a generated contract and round-trips it through MessagePack and JSON; verify with `dotnet test bindings/dotnet/NxLang.sln`
- [x] 7.5 If `add-declaration-schema-export` has landed, add the test for its scenario "A function reference field is not described as a record": `artifact.typeSchema` for a record with a field typed `<function ... />: object*` and one typed `<function ... />: R`, and `artifact.functionSchema` for a function with a parameter of a function reference type, each report `schema-inexpressible-type` naming the member and emit no schema for it; verify the test passes. If it has not landed, skip with a note in the PR, since that change's task 3.6 then adds the case and the test — **Skipped:** `add-declaration-schema-export` has not landed (no `schema-inexpressible-type` in the tree), so its task 3.6 adds the case and the test

## 8. Language service

- [x] 8.1 Verify the hover test for "A hover shows the type as written", and that a hover on a function name bound at a function reference site shows that function's own declaration

## 9. Documentation

- [x] 9.1 Update `nx-grammar.md` and `nx-grammar-spec.md`: add `...` to the function type production, and state that it stands for the whole parameter list and is accepted nowhere else; verify the production against `grammar.js` by review
- [x] 9.2 Update `sites/website/src/content/docs/reference/syntax/types.md`: add an "Any parameters" subsection to "Function Types" covering `<function ... />: R`, what satisfies it, that it cannot be called, occurrences and the join, introducing `...` as a token and not as elision, and stating that `<function ... />: object*` is the type every function satisfies and that `<function ... />: object` is not; verify `npm test` in `sites/website` passes its code-block check
- [x] 9.3 Update `sites/website/src/content/docs/reference/syntax/functions.md` ("Functions as values") with a host-called function example (a field typed `<function ... />: object*` and one with a stated result) and the not-callable diagnostic; verify the code-block check passes
- [x] 9.4 Update `docs/nx-ir-format.md` (type kind `6` and its operand, the feature in "Required features", the boundary rule in "Function values"), `specs/ir-conformance/README.md` (the new program) and `runtime/typescript/README.md` (`NxFunctionRecord`, reading function records only from members declared at a function type); verify by review against the spec deltas
- [x] 9.5 Add an entry to `src/vscode/CHANGELOG.md` under the next version for the editor-visible part (`...` in a function type), and note the new type and the new required feature in the release PR description; verify the changelog entry by review

## 10. Whole-repo verification

- [x] 10.1 Run `cargo test --workspace`, `npm test` in `runtime/typescript`, and `dotnet test bindings/dotnet/NxLang.sln`, and verify all pass
- [x] 10.2 Run `openspec validate add-function-reference-type --strict` and verify it passes
- [x] 10.3 Compile a sample that mirrors the agent library's use (`type FunctionTool = { function: <function ... />: object* }` and `type HttpTool = { arguments: <function ... />: HttpArguments }`, with documented paren functions bound to them, and one function of the wrong result that is rejected), emit IR with `nxlang codegen`, evaluate it with `@nx-lang/ir-runtime`, read the record and call it with `callFunction`; verify the result and record the commands in the PR so `add-agent-library` and `add-agent-host-package` can reuse them
