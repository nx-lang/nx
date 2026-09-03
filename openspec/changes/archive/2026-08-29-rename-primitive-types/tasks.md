## 1. Grammar and parsing

- [x] 1.1 Replace the six numeric names in the `primitive_type` rule in `crates/nx-syntax/grammar.js` with `int32`, `int64`, `float32`, `float64`, and replace `bool` with `boolean`, leaving `string`, `void`, `object` unchanged
- [x] 1.2 Regenerate the tree-sitter parser and refresh its committed artifacts
- [x] 1.3 Update primitive name handling in `crates/nx-syntax/src/lib.rs` and `crates/nx-syntax/src/ast.rs`
- [x] 1.4 Update the `PrimitiveType` production in `nx-grammar.md` and the corresponding section of `nx-grammar-spec.md`

## 2. Type system

- [x] 2.1 Rename the `Primitive` variants in `crates/nx-types/src/ty.rs` to `Int32`, `Int64`, `Float32`, `Float64`, and `Bool` to `Boolean`, and update `as_str` to return the new names
- [x] 2.1a Rename the `Type::bool()` constructor to `Type::boolean()` and update all call sites
- [x] 2.2 Delete the `Int` and `Float` variants, then delete `CanonicalPrimitive`, `canonical()`, and the hand-written `PartialEq`, `Eq`, and `Hash` impls, replacing them with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` on `Primitive`
- [x] 2.3 Confirm no compatibility surface was added: no alias variants, no dual-name lookup, no deprecation path
- [x] 2.4 Update `is_integer` and `is_float` to match on the four remaining numeric variants
- [x] 2.5 Replace `Type::int()` and `Type::float()` constructors with `Type::int64()` and `Type::float64()`, updating all call sites
- [x] 2.6 Point `Literal::Int` and `Literal::Float` inference in `crates/nx-types/src/infer.rs` at `int` and `float64`
- [x] 2.7 Update primitive name handling in `crates/nx-types/src/semantics.rs`, including the `"bool" => Some(Type::bool())` lookup at line 108
- [x] 2.8 Confirm `is_compatible_with` still gives width-blind compatibility within the integer and float categories, and still rejects integer against float
- [x] 2.9 Update `crates/nx-hir/src/ast/types.rs` (including the `TypeRef::name("bool")` test at lines 107 and 116), `crates/nx-hir/src/lower.rs` (`"bool" => TypeTag::Boolean` at line 48), and `crates/nx-hir/src/scope.rs`

## 3. Unresolved type diagnostics — dropped from this change

Reporting an undeclared type name at its declaration turned out to require an intrinsic-element
registry NX does not have, because an undeclared name in type position is a valid element type. See
design.md, "Nothing reports a former spelling after the rename". Deferred to its own change, most
likely as a did-you-mean diagnostic that maps `int` to `int64`.

- [x] 3.1 Confirm that no unresolved-type diagnostic ships here, and that the type checker's existing
      behavior for undeclared names is unchanged by the rename
- [x] 3.2 Assert the removed spellings are no longer primitives at the layer where that is
      observable: `builtin_type` returns `None` for `i32`, `i64`, `int`, `f32`, `f64`, `float`,
      and `bool`, and `Some` for each of the eight canonical names
- [x] 3.3 Keep a test that a user-defined `type int = { value:int64 }` shadows the former spelling

## 4. Code generation and runtime

- [x] 4.1 Update the primitive-name `match` arms in `crates/nx-codegen/src/emit.rs`, `builder.rs`, and `ir.rs`, including the full-primitive-set list in `builder.rs:1825`
- [x] 4.1a Update the boolean sites in `crates/nx-codegen/src/emit.rs`: the `"bool" => "nxBooleanSchema"` arm at ~2249, the `"bool" => "boolean"` arm at ~2624, `Primitive::Bool => "boolean"` at ~2657, and the `"bool"` arm at ~3768
- [x] 4.1b Change `Primitive::Bool => "bool"` to `Boolean => "boolean"` in `crates/nx-codegen/src/ir.rs:1831`, resolving its existing disagreement with the `{"kind":"boolean"}` literal tag
- [x] 4.2 Collapse the paired arms in `crates/nx-cli/src/typegen/languages/csharp.rs` so `int32`→`int`, `int64`→`long`, `float32`→`float`, `float64`→`double`, each a single arm
- [x] 4.2a Change the `"bool"` match keys in `csharp.rs` (lines 693, 860, 959) to `"boolean"` while leaving the emitted `text: "bool"` unchanged — NX `boolean` still generates C# `bool`
- [x] 4.3 Update `crates/nx-cli/src/typegen/languages/typescript.rs` — line 702 becomes `"boolean" => "boolean"`, a pass-through — and the primitive-name list in `crates/nx-cli/src/typegen/model.rs:831`
- [x] 4.4 Update `crates/nx-interpreter/src/interpreter.rs` (seven `expected: "bool"` diagnostic strings), `crates/nx-interpreter/src/value.rs` (`Value::Boolean(_) => "bool"` at line 152 and its test at 273), and `crates/nx-interpreter/src/eval/logical.rs` (four `expected: "bool"` strings)
- [x] 4.5 Update type-name sites in `runtime/typescript/src/index.ts`, including the primitive switch near line 852, `case "bool"` at line 867, and the integer predicate near line 1335
- [x] 4.6 Leave IR literal kind tags (`{"kind":"int"|"float"|"boolean"}` on literal values) unchanged, including the sites near line 590 of `runtime/typescript/src/index.ts` and `NxIrLiteral::Boolean`; verify no rewrite touched them
- [x] 4.7 Bump `NX_IR_SCHEMA_VERSION` from `1` to `2` in `crates/nx-codegen/src/ir.rs` and `runtime/typescript/src/index.ts`, writing no upgrade path
- [x] 4.8 Rebuild the TypeScript runtime and refresh `runtime/typescript/dist`

## 5. Tooling

- [x] 5.1 Replace `PRIMITIVE_TYPE_COMPLETIONS` in `crates/nx-language-service/src/lib.rs:40` with the eight canonical names, removing `long`, `double`, and `bool`, and update the completion assertion at line 1735
- [x] 5.2 Check the VS Code extension and editor assets for any hard-coded primitive name list

## 6. First-party NX sources

- [x] 6.1 Keep the 67 type-position occurrences of `int` as `int` across `examples/nx/**` and `crates/nx-syntax/tests/fixtures/**` — `int` is the default integer type and every one of them is an ordinary count, id, age, or coordinate (see 11.6)
- [x] 6.1a Rewrite the 27 occurrences of `bool` to `boolean` across `examples/nx/**`, `src/vscode/samples/**`, and `crates/nx-syntax/tests/fixtures/valid/**`
- [x] 6.2 Verify no rewrite touched `print`, `interface`, `into`, `interpreter`, or `println`
- [x] 6.3a Verify no `bool` rewrite escaped `.nx` files into Rust source, where `bool` is a legitimate type
- [x] 6.3 Re-parse every `.nx` file under `examples/` and `crates/nx-syntax/tests/fixtures/valid/` and confirm each still parses
- [x] 6.4 Confirm `crates/nx-syntax/tests/fixtures/invalid/` fixtures still fail for their original reasons

## 7. Rust tests

- [x] 7.1 Update primitive names in `crates/nx-interpreter/tests/` (`floats.rs`, `edge_cases.rs` including the `TypeRef::name("bool")` at line 357, `error_handling.rs`, `interpreter_direct_hir.rs`, `loops.rs`, `recursion.rs`)
- [x] 7.2 Update `crates/nx-codegen/src/tests.rs`
- [x] 7.3 Update the display assertions in `crates/nx-types/src/ty.rs` tests, replacing the `Type::float()`/`Type::f64()` pair with a single canonical assertion

## 8. Existing spec scenario text

- [x] 8.1 Keep `int` in scenario text across `openspec/specs/record-type-inheritance/spec.md`, `content-properties/spec.md`, `cli-code-generation/spec.md`, `record-construction-validation/spec.md`, `source-analysis-pipeline/spec.md`, `discriminated-unions/spec.md`, `braced-value-sequences/spec.md`, and `executable-code-generation/spec.md` (see 11.6)
- [x] 8.2 Rewrite `bool` to `boolean` in NX source text across `openspec/specs/component-syntax/spec.md`, `discriminated-unions/spec.md`, `cli-code-generation/spec.md`, `external-components/spec.md`, `component-contract-inheritance/spec.md`, and `runtime-output-format/spec.md`
- [x] 8.3 Leave `bool` untouched where it is expected C# output, specifically `public bool Enabled`, `public bool Retryable`, and `public bool Selected` in `cli-code-generation/spec.md` around lines 403, 411, and 416
- [x] 8.4 Confirm no requirement text changed, only illustrative type names inside `WHEN` clauses

## 9. Documentation

- [x] 9.1 Rewrite the NX listings in `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` from `f64` to `float64`, including Appendix A and the inline declarations in sections 4 through 7
- [x] 9.2 Rewrite `docs/drawn-ui-proposal-nx-enhancements.md`, including the NXE1, NXE2, and NXE18 code sketches
- [x] 9.3 Re-verify the NX listings in both documents with `nxlang typegen` so their "verified against `nxlang`" claims stay true, and update the recorded commit
- [x] 9.4 Rewrite the `bool` occurrences in `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md` (9) and `docs/drawn-ui-proposal-nx-enhancements.md` (2)
- [x] 9.5 Check `README.md` and `specs/**` for primitive names in examples

## 10. Verification

- [x] 10.1 `cargo build --workspace` and `cargo test --workspace` pass
- [x] 10.2 `cargo build -p nx-cli` then run `nxlang typegen` over the proposal's catalog libraries and confirm `int32`/`int64`/`float32`/`float64` generate C# `int`/`long`/`float`/`double` and TypeScript `number`, and that `boolean` generates C# `bool` and TypeScript `boolean`
- [x] 10.3 Confirm a mismatch diagnostic reads `expects float64, found int64`
- [x] 10.4 Grep the workspace for surviving `\bi32\b`, `\bi64\b`, `\bf32\b`, `\bf64\b`, `\bint\b`, and `\bfloat\b` in type position and confirm the only remaining hits are in `openspec/changes/archive/` and this change's own artifacts
- [x] 10.4a Grep for surviving `bool` and confirm every remaining hit is either Rust's own `bool`, C# output text, or an IR literal tag — never an NX type name
- [x] 10.5 Confirm IR emitted at schema version 1 is rejected by the TypeScript runtime with an unsupported-version error

## 11. Reintroduce `int` as a distinct primitive type

`int` returns as a type of its own — exact over ±(2^53−1) on every backend, and the default integer
type — rather than as the display-preserving alias for `i64` that this change removed. Bounds-check
enforcement and `int64`-as-`bigint` are specified but deliberately deferred; see design.md.

- [x] 11.1 Add `'int'` to the `primitive_type` rule in `crates/nx-syntax/grammar.js`, regenerate the tree-sitter parser, and confirm `int32`/`int64` still win longest-match against the new `int` alternative
- [x] 11.2 Add a `Primitive::Int` variant in `crates/nx-types/src/ty.rs` with `as_str` returning `"int"`, include it in `is_integer`, and add the `Type::int()` constructor — no alias machinery returns with it, and `Primitive::Int != Primitive::Int64`
- [x] 11.3 Extend `numeric_promotion` to the rank order int32 < int < int64, in both operand orders
- [x] 11.4 Point `Literal::Int` inference at `Type::int()` in `crates/nx-types/src/infer.rs`, along with the loop index binding and index-expression check; add `"int" => Some(Type::int())` to `builtin_type` in `semantics.rs`
- [x] 11.5 Add `TypeTag::Int` in `crates/nx-hir/src/lower.rs`, map `"int"` to it, extend `combine_numeric` to the rank order, and point the three integer-literal `set_expr_type` sites at it
- [x] 11.6 Return the already-rewritten `int64` sites to `int` across `.nx` sources and fixtures, Rust NX-source snippets, `openspec/specs/**` scenario text, `specs/**`, `docs/**`, the .NET and Node binding tests, and the VS Code samples — keeping `int64` only where the site is *about* the 64-bit width
- [x] 11.7 Add `"int"` to the primitive-name lists in `crates/nx-codegen/src/{ir.rs,builder.rs,emit.rs}` (including `emit_primitive_type`), `crates/nx-cli/src/typegen/{model.rs,languages/typescript.rs}`, and `crates/nx-language-service/src/lib.rs`
- [x] 11.8 Map `int` to C# `long` in both mapping tables in `crates/nx-cli/src/typegen/languages/csharp.rs`, and add it to the imported-alias primitive list
- [x] 11.9 Add `case "int"` to the primitive switch and `isIntegerSemanticType` in `runtime/typescript/src/index.ts`, leaving the IR literal *kind* tag at line ~590 untouched, and rebuild `runtime/typescript/dist`
- [x] 11.10 Point `Value::Int` at NX `int` in `crates/nx-interpreter/src/value.rs` (`type_name` → `"int"`) and `interpreter.rs` (`value_type` → `Type::int()`), documenting that `int64` shares the carrier until it gets its bigint representation
- [x] 11.11 Add `int` to the primitive alternations in `src/vscode/syntaxes/nx.tmLanguage.json` (the keyword match and both negative lookaheads), and confirm `\b` anchoring keeps `int32`/`int64` matching
- [x] 11.12 Add `int` to the primitive-set enumerations in `nx-grammar.md`, `nx-grammar-spec.md`, `README.md`, `src/vscode/README.md`, `nx-planning.md`, `nx-rust-plan.md`, `specs/001-nx-core-parsing/spec.md`, and the `nx-types` crate doc block
- [x] 11.13 Rewrite the numeric sections of `specs/future.md` to record the specified range, the deferred bounds checks with their measured cost, the deferred `int64`-as-`bigint` work, and the promotion rank order
- [x] 11.14 Update `docs/drawn-ui-proposal-nx-enhancements.md` and `docs/NX-Drawn-UI-MVP-Object-Model-Proposal.md`: the NXE8 diagnostic now reads *"expects float64, found int"* (verified by running `nxlang`), the primitive collision set gains `int`, and the small bounded values (`gridColumn`, `columnSpan`, `maxLines`) take `int`
- [x] 11.15 Rewrite `test_user_defined_type_may_take_a_former_primitive_name` to use `i64` rather than `int`, and add a test pinning that a user-declared `type int` does not displace the primitive
- [x] 11.16 Add tests: distinctness of `Primitive::Int` from `Primitive::Int64`, promotion rank order, `common_supertype` rank order, `int`/`int32`/`int64` compatibility, `builtin_type("int")`, C# and TypeScript numeric-width mapping over all five types, the `numeric-types.nx` parser fixture, and a VS Code grammar test scoping all five
- [x] 11.17 Remove `int` from the former-spellings assertions in `semantics.rs`, and add `Int`/`INT` to the capitalized-spellings assertion
- [x] 11.18 Confirm no further `NX_IR_SCHEMA_VERSION` bump is needed — `int` joins the same unreleased schema version 2

## 12. Verification of the `int` reintroduction

- [x] 12.1 `cargo build --workspace` and `cargo test --workspace` pass (45 test binaries)
- [x] 12.2 `runtime/typescript` builds and its 12 tests pass; `src/vscode` grammar tests pass (77)
- [x] 12.3 .NET binding tests pass (93) after rebuilding the *release* `libnx_ffi.so`, which the SDK tests bind — a debug-only `cargo build` leaves them running against a stale analyzer
- [x] 12.4 Node binding tests pass (11)
- [x] 12.5 Every `.nx` file under `examples/` and `crates/nx-syntax/tests/fixtures/valid/` re-parses, and the `invalid/` fixtures still fail
- [x] 12.6 `nxlang run` confirms the mismatch diagnostic reads `expects float64, found int`
- [x] 12.7 Grep the workspace and confirm every surviving `int64` is either a deliberate width site, an implementation match key, or the D-language reference in the enhancements document
