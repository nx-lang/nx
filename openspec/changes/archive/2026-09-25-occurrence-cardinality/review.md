# Review: occurrence-cardinality

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, review-notes.md, release-notes-v0.4.0.md, every
delta under specs/ (with close reading of occurrence-types, optional-properties, presence-operators,
value-equality, conditional-result-types, sequence-model, update-records, external-components,
nx-ir-format, typescript-ir-runtime, executable-code-generation, cli-code-generation)  
**Reviewed code:** the uncommitted working tree against `HEAD` (c197f4e): crates/nx-syntax, nx-hir,
nx-types, nx-interpreter, nx-value, nx-api, nx-codegen, nx-cli (typegen, value formatter), nx-ffi,
nx-language-service; runtime/typescript; bindings/dotnet, bindings/node, bindings/wasm;
src/vscode grammar; specs/ir-conformance; docs pages. Spec scenarios were exercised with
`target/debug/nxlang run`, and the three engines (interpreter, IR runtime, generated JS) were compared
on scratch programs. Baseline: `cargo test --workspace` passes (77 suites), `openspec validate --strict`
passes.

## Findings

### ✅ Verified - RF1 Narrowing is keyed by name, so a shadowing binding inherits a narrowed type
- **Severity:** High
- **Evidence:** `crates/nx-types/src/infer.rs` narrowing map (~386-392, `narrowed_type` ~499,
  `narrowed_or` ~509) is applied after `env.lookup` whatever scope the name resolves in.
  `let f(o?:string, xs:int+): string+ = { if o? { for o in xs { o } } else { "a" } }` type-checks and
  then fails at runtime with "expected string, got int". The union-case analogue
  (`if sh is { circle => for sh in xs { sh } … }`) is wrongly typed `object+`. HEAD's case narrowing
  rebound in a pushed scope and shadowed correctly.
- **Recommendation:** Record the scope depth of the root binding with each narrowing and apply it only
  when the lookup resolves the root at that depth (or drop narrowings whose root is rebound by `for`,
  `let` or a parameter). Add a shadowing test for both the presence and the case narrowing.
- **Fix:** Narrowings now record the scope depth their root identifier resolved at (`Narrowing.root_depth`, `push_narrowings`/`pop_narrowings` in `crates/nx-types/src/infer.rs`) and apply only while the root still resolves there, so a `for`, `let` or parameter of the same name shadows them. Tests: `a_shadowing_binding_does_not_inherit_a_narrowing`, `a_shadowing_binding_does_not_inherit_a_case_narrowing` (`presence_operators.rs`).
- **Verification:** Verified by the original reviewer: the shadowing repros are rejected (found `object+`) with no runtime crash; the `int+` and union-case variants are clean; scopes pushed by `for`, `let` and match arms make the depth comparison sound.

### ✅ Verified - RF2 Relational operators accept `?`, `+` and `*` operands and crash at runtime
- **Severity:** High
- **Evidence:** `infer.rs` ~2117-2150 accepts when either operand satisfies the other; with the scalar
  lift inside that relation, `int` satisfies `int+`. `let e(n:int+): boolean = { n < 1 }` passes and
  fails at runtime ("got array and int"); `n?:int > 1` likewise ("got empty and int").
- **Recommendation:** Require both relational operands to be exactly one; suggest `??` when an operand
  admits zero and `for` when it admits many.
- **Fix:** `<`, `<=`, `>`, `>=` now require exactly one value on each side and name `??` (may be empty) or `for` (may be many). Test: `ordering_takes_exactly_one_value_on_each_side` (`occurrence_types.rs`).
- **Verification:** Verified: `n?:int > 1`, `xs:int+ < 1` and `string+ >= "a"` are rejected with the `??`/`for` hints.

### ✅ Verified - RF3 Equality does not treat an item as a sequence of one
- **Severity:** High
- **Evidence:** value-equality delta, "An item compares as a sequence of one". `values_equal`
  (`crates/nx-interpreter/src/eval/logical.rs` ~126-177) compares `Array` only with `Array`:
  `let one:int? = 1 let ones:int* = { 1 }` → `one == ones` is `false`. The same function backs `diff`
  and match patterns. The checker also rejects `int? == int+` ("Cannot compare types"), which the
  scenario's `other` case needs.
- **Recommendation:** In `values_equal`, compare a non-array against an array as a one-item slice; let
  the checker accept `==`/`!=` between types with a common item type regardless of occurrence; add
  the spec scenarios as tests (and make the IR runtime/JS helpers agree).
- **Fix:** Interpreter `values_equal` compares an item with a one-element array as equal items (`eval/logical.rs`); the checker accepts `==`/`!=` across occurrences when the item types compare (`items_comparable` in `infer.rs`); the IR runtime's `valuesEqual` and the generated-JS `nxValuesEqual` follow the same rule. Tests: `an_item_compares_as_a_sequence_of_one` (interpreter), `equality_compares_across_occurrences` (checker), `emitted_runtime_equality_treats_an_item_as_a_sequence_of_one` (codegen).
- **Verification:** Verified in the checker, interpreter, IR runtime and generated JS (`one == ones` true, `{} == {1}` false, mixed-occurrence comparisons accepted, `int? == string+` rejected). The fix exposed a literal-typing gap, tracked as RF41.

### ✅ Verified - RF4 `{}` cannot be written as an item or operand inside another expression
- **Severity:** High
- **Evidence:** conditional-result-types delta scenario `let xs:string* = { "a" {} }` and value-equality
  scenario `{none == {}}` both fail to parse ("Unclosed brace" / "Syntax error"); so do
  `if c { {} } else { 1 }` and `{ {} {} }`. `{}` only parses as a whole braced value.
- **Recommendation:** Admit an empty braced value as a `value_expression` (and after binary operators)
  in `grammar.js`, regenerate, and add parser tests for each form.
- **Fix:** `grammar.js` admits the empty brace as a list item and operand (hidden `_empty_braced_expression` aliased to `values_braced_expression`, so lowering is unchanged); parser regenerated with no new conflicts; non-empty nested braces stay errors. This resolved a contradiction inside the change: the braced-value-sequences delta scenario "A braced value is still not an item of a braced value" required `items={{}}` to be a syntax error, so its body now tests `{{"a" "b"}}`/`{{"a"} "b"}` and states `{{}}` reads as the empty value; unbraced-literal-forms' `{{}}` rationale and the `format.rs` comments are reworded to match. Tests: `test_parse_empty_braced_value_as_an_item_and_operand`, `..._prefers_the_full_brace_where_both_fit`.
- **Verification:** Verified: every listed form parses and evaluates; no parse regressions across `for … {}`, `if … {} else`, condition lists, the `{}` pattern, element bodies and `type R = {}`; non-empty nested braces stay rejected; the spec rewording is consistent. The value-equality scenario's `{one == { 1 2 }}` still does not parse, tracked as RF46.

### ✅ Verified - RF5 A `for` over an optional yields a one-element array rather than the item
- **Severity:** High
- **Evidence:** `eval_for` (`crates/nx-interpreter/src/interpreter.rs` ~4273) always collects into an
  array. `let p:Person? = … let v = { for x in p { x.name } }` outputs `["A"]` (canonical encoding
  requires `"A"`), `v == "A"` is `false`, and `if v? { v.name }` over a record body type-checks then
  fails ("expected record, got array"). `a_for_over_an_optional_yields_its_item` passes only because of
  a `: string?` return annotation.
- **Recommendation:** When the iterated occurrence and the body occurrence are both at most one
  (product is `?`), return the single body result (or empty) instead of wrapping; add a test without a
  return annotation. The IR runtime and emitted JS need the same rule (see RF6).
- **Fix:** A `for` whose iterable is not an array (an optional holding its item) now yields the body's value unchanged in the interpreter (`eval_for`), the IR runtime (`nodeKinds.for`) and generated JS (new `nxForOne` helper, chosen when the iterable's static type holds at most one item). Tests: `a_for_over_an_optional_yields_its_item_without_an_annotation` (interpreter), updated `test_for_loop_over_a_single_item_iterates_once` and the runtime.test.ts case, `generated_javascript_for_over_an_optional_yields_its_item` (codegen).
- **Verification:** Verified in all three engines (`h(1)` is `1`, `for`-then-`==` is `true`, a two-item body stays a list). A runtime-versus-static decision difference is tracked as RF44.

### ✅ Verified - RF6 The IR runtime and generated JS ignore a function's declared return occurrence
- **Severity:** High
- **Evidence:** the IR function declaration carries no return type, so `invokeFunction` cannot lift or
  un-lift. `let g(): int+ = { 5 }` gives interpreter `[5]`, IR/JS `5`; `let h(o?:int): int? = { for x in o { x } }`,
  `h(1)` gives interpreter `1`, IR/JS `[1]`; `g(xs?:int+): int+ = { xs ?? 5 }` with `{}` gives `[5]` vs `5`.
  typescript-ir-runtime / executable-code-generation require the three engines to agree.
- **Recommendation:** Carry the result type on IR function declarations (schema 5 is being introduced
  anyway) and normalize results with `normalizeValue`; wrap emitted returns in an occurrence-lift helper.
  Needs a small IR-format spec addition.
- **Status:** Not fixed: needs a design decision. Carrying a result type on IR function declarations (and values) changes the nx-ir-format layout and every reader; alternatives are a builder-inserted normalization node, or accepting the divergence for unannotated-but-declared results. The same gap affects `let one: Ints = { 1 }` values. Recommendation: add the result type to the declaration layout in schema 5 before release, since the schema is being bumped anyway.
- **Fix:** Schema 5 layout: a function declaration is `[0, str, params, node, type?, flags]` and a value `[1, str, node, type?]`. The type is the declared result type, and bit 0 of the flags marks a standalone `T?` result, declared or inferred (for RF21). The IR runtime normalizes a result or value to its declared type with `normalizeValue`, as the interpreter coerces only a declared type. Generated JS lifts a declared result with `lift_to_declared_occurrence`, and its TS signature now uses the checker's declared-or-inferred type instead of the body's type. `ir explain` prints `: T` and `(optional result)`. The corpus program `occurrence-lifting` gains `declaredMany`, `declaredOne`, `declaredFallback` and `declaredValue`, which agree across all three engines (`[5]`, `1`, `[5]`, `[5]`). Covered by the IR test `a_declaration_carries_its_declared_result_and_whether_it_is_optional` and a TS runtime test; docs/nx-ir-format.md and the nx-ir-format, typescript-ir-runtime and executable-code-generation deltas are updated. RF44 (the unannotated `xs ?? 5` result) is unchanged.
- **Verification:** Verified: `g()` `[5]`, `h(1)` `1`, `g2({})` `[5]` and `one` `[1]` agree in the interpreter, IR runtime and generated JS, as do declared `float64+`, alias, `Person?`/`Person*` and empty `int*` results; `ir explain` prints `: T` and `(optional result)` for declared and inferred `T?` results, and docs/nx-ir-format.md matches the new layout.

### ✅ Verified - RF7 Reading an unstored optional field fails for union-case and imported record types
- **Severity:** High
- **Evidence:** `record_field_reads_as_empty` (`interpreter.rs` ~4171) resolves the shape through
  `effective_record_shape` → `resolve_item(module, name)` (~814-841), which neither resolves
  `Shape.circle` nor looks outside the evaluating module. `type Shape = | circle { r:int label?:string } | square { s:int }`,
  `s.label ?? "nolabel"` on `<Shape.circle r={1} />` → "Record 's' has no field named 'label'". A record
  from another module that is not imported by name hits the same path (from code).
- **Recommendation:** Since the checker guarantees the member exists and required fields are always
  stored, have member projection yield the empty value for any declared-but-unstored field (or resolve
  the shape by the record's declaring origin, including union cases). Add union-case and cross-module
  tests.
- **Fix:** `record_field_reads_as_empty` treats a record whose shape cannot be resolved from the evaluating module (a union case, or a type not nameable there) as reading an unstored member as empty, since the checker proved the member is declared and required fields are always stored. Test: `an_unstored_optional_field_of_a_union_case_reads_as_empty`.
- **Verification:** Verified for union cases and types not nameable in the evaluating module. Residual: a same-named local type without the field still answers the lookup (tracked in RF48).

### ✅ Verified - RF8 Generated JS `==` is reference equality, so two empty optionals compare unequal
- **Severity:** High
- **Evidence:** `crates/nx-codegen/src/emit.rs` ~4957 maps `BinOp::Eq` to `===`. `a.nick == b.nick` with
  both omitted: interpreter `true`, IR `true`, JS `false` (was `null === null`, a regression). Records
  compare by reference too (pre-existing).
- **Recommendation:** Emit the runtime's structural `nxValuesEqual` for `==`/`!=` unless both operands
  are exactly-one primitives.
- **Fix:** Generated `==`/`!=` call a structural `nxValuesEqual` unless both operands are exactly-one primitives (`emit.rs`, `runtime.rs`).
- **Verification:** Verified across engines: empty optionals, equal records, differing sequences and item-versus-one-element comparisons agree.

### ✅ Verified - RF9 Generated TypeScript no longer type-checks under `--strict` for list loops and `??`
- **Severity:** High
- **Evidence:** `runtime.rs` ~141 `nxItems(value: unknown): readonly NxValue[]` (now emitted for every list
  `for`), ~136 `nxCoalesce<T>(value: T, fallback: () => T): T`, ~127 `nxStep` returns `NxValue`.
  `let inc(xs:int+): int+ = { for x in xs { x + 1 } }` and `p.nick ?? "none"` produce TS18047, TS2365 and
  TS2322 under tsc. No type-check test covers a list `for` or an operator.
- **Recommendation:** Make the helpers generic (`nxItems<T>(v: T | readonly T[]): readonly T[]`,
  `nxCoalesce<T, U>(v: T | NxEmpty, f: () => U): T | U`, a typed `nxStep`) and add the strict
  type-check to loop and operator codegen tests.
- **Fix:** `nxItems<T>`, `nxCoalesce<T, U>`, `nxStep` and `nxEmpty` are generic/typed in the TS runtime (`runtime.rs`); a strict type-check test now covers a list `for`, `??`, `?.` and `==`.
- **Verification:** Reopened (partial): the originally reported shapes type-check, but `tsc --strict` still fails for `?.` into a `*` member (`nxStep` result `| NxEmpty` includes `null`), `nxForOne` over a member read written `?? []` (item inferred `Person | never[]`), conditional object literals at `?`/`*` sites (`never[] | Item`, `$type` widened), and a `??` result typed `int+` (RF6 surfacing). Emit a typed `nxEmpty` instead of bare `[]`, exclude `never[]`/`null` from helper item types, and add these shapes to the strict test.
- **Fix (cycle 2):** Generated code writes a typed `nxEmpty` instead of bare `[]` for optional member reads, untaken `if` branches and optional state reads; `nxStep`/`nxForOne` return `readonly never[]` when empty; TypeScript output writes `$type: "X" as const` so record literals stay literal inside conditionals and `.flat()`; the strict type-check test covers `?.` into a `*` member, `for` over an optional member, chained `??`, and conditional records at `?`/`*` sites. The remaining `??`-at-`int+` return error is RF6.
- **Verification:** Verified (cycle 2): t17, t2b f1/f2/f8, t4b h4/h5, t5, t14, t3 and t19 pass `tsc --strict`; the remaining strict errors belong to RF6, RF28, RF49 and RF51.

### ✅ Verified - RF10 The IR drops the optional flag on function parameters
- **Severity:** Medium
- **Evidence:** `crates/nx-codegen/src/ir.rs` ~1348 writes only `is_content`; `CodegenParam`
  (`model.rs` ~150) has no `optional`. `<T Item="x" />` for a `<function Item:string Note?:string />`
  argument works in the interpreter (`"x-none"`) but the IR runtime reports "Function 'Row' requires
  argument 'Note'".
- **Recommendation:** Add `optional` to `CodegenParam`, set it in `build_params`, emit
  `FUNCTION_PARAM_OPTIONAL`, and add an emitted-IR test.
- **Fix:** `CodegenParam.optional` is set by both param builders and written as the optional flag bit (`model.rs`, `builder.rs`, `ir.rs`); a direct element call omitting an optional parameter binds the empty value. `ir_explain.rs` now reads the declaration flags as bits (it had shown an optional parameter as `content`) and prints `name?: T`; `docs/nx-ir-format.md` documents the flags word; corpus explained text regenerated.
- **Verification:** Verified: an element call through a function value omitting `Note?` gives `"x-none"` in the interpreter and IR runtime, `callFunction` agrees, and explain prints `Note?: string`.

### ✅ Verified - RF11 Function-type spelling drops the `?` mark on parameters
- **Severity:** Medium
- **Evidence:** `SpelledParam` (`crates/nx-hir/src/ast/types.rs` ~296-305) has no `optional`;
  `spell_function_type`, `nx-types/src/ty.rs` ~443 and `nx-codegen/src/ir_explain.rs` ~247 lose it.
  Diagnostics read "expects <function A:int B:int />: string, found <function A:int B:int />: string;
  parameter 'B' is supplied as int, which is not int". `ir explain` instead doubles the mark
  (`Note?:string?`).
- **Recommendation:** Add `optional` to `SpelledParam`, print `name?:T` (the declared type), report read
  types in `FunctionMismatch::ParameterType`, and add a spelling test for `<function Index?:int />`.
- **Fix:** `SpelledParam` gained `optional` and prints `name?:T` with the declared type; the checker, HIR and `ir explain` pass it (ir explain now recovers the declared type from the read type instead of doubling the mark); `FunctionMismatch::ParameterType` reports read types. The mismatch now reads "expects <function Item:Person Index?:int />: string … parameter 'Index' is supplied as int?, which is not int".
- **Verification:** Verified in diagnostics, hovers and `ir explain` (`(<function Item:string Note?:string />: string)?`, no doubled mark).

### ✅ Verified - RF12 Host `null`/`[]` at a required-with-default field silently takes the default
- **Severity:** Medium
- **Evidence:** `from_nx_value_at_path` (`crates/nx-api/src/value.rs` ~146-160) drops every empty field of
  a non-`.Update` record before construction, so construction sees "not written" and applies the
  default; props/state maps pass through the same function (`component.rs` ~123, 191, 195).
  occurrence-types: "Decoding at an exactly-one site SHALL reject `null`"; optional-properties: a
  defaulted property rejects `{}` and host construction applies the same rules.
- **Recommendation:** Stop dropping empties in the decoder; construction already omits an empty
  optional field after coercion and would then reject `{}` at the exactly-one site naming the field.
- **Status:** Not fixed: coupled with RF13. Dropping the decoder's empty-field removal is correct for host records that are rebuilt (`Host` origin), but entry-argument records are passed through unrebuilt by design (`ValueOrigin::EntryArgument`), so they would then keep a stored `[]` for an optional field and compare unequal to an NX-built record. Decide RF13 first, then remove the drop.
- **Fix:** Decided with RF13. `from_nx_value_at_path` (`crates/nx-api/src/value.rs`) keeps a present `null`/`[]` as written, and the `.Update` special case is gone: construction decides what it means (empty at `?`/`*`, cleared in an update record, rejected at `+` and at an exactly-one site even with a default; only a missing key takes the default). A host record with no declared shape (`object`) drops its empty fields in `construct_host_record_value` (`without_empty_fields`), as the decoder did. Test `a_host_empty_value_is_rejected_where_exactly_one_is_required_even_with_a_default` (`crates/nx-api/tests/occurrence_boundary.rs`); release notes say so.
- **Verification:** Verified in the interpreter and the IR runtime: a missing key takes a default, while `null`/`[]` at a defaulted `int` field or parameter is rejected naming the field; `[]` at `tags?:string+` is dropped, `null` at a `+` field is rejected, and an update record keeps `email: null` and rejects `name: null`. The release notes say so.

### ✅ Verified - RF13 Host records passed as entry-call arguments bypass occurrence normalization
- **Severity:** Medium
- **Evidence:** `ValueOrigin::EntryArgument` passes records through unchanged (`interpreter.rs` ~3843), so
  `"author":[{…}]` stays a one-element array, `"author":["X","Y"]` at a `?` site is accepted and
  `"items":[]` for `items:string+` is not rejected at the boundary.
  `a_host_empty_value_is_rejected_where_at_least_one_is_required` passes only because the function later
  reads the field and fails lazily.
- **Recommendation:** Rebuild entry-argument records as the `Host` origin does; add tests whose function
  does not read the offending field, plus one- and two-element arrays at a `?` site.
- **Status:** Not fixed: `EntryArgument` records are deliberately passed through as the host built them (documented on `ValueOrigin`). Rebuilding them adds boundary validation cost and may reject payloads hosts send today; the alternative is a lighter per-field occurrence normalization. Needs a decision on the entry-call boundary contract.
- **Fix:** Decided: entry-call arguments are host values and are rebuilt. The pass-through was inherited, not chosen (before 124f9a5 entry arguments were coerced as `Internal`; `EntryArgument` split off only for number widths), and the TypeScript runtime already validates function arguments at the boundary. `ValueOrigin::EntryArgument` is removed and `execute_function` coerces with `ValueOrigin::Host`. Tests: `a_host_empty_value_is_rejected_where_at_least_one_is_required` now calls a function that does not read `items`; new `a_host_array_at_an_optional_field_is_normalized_at_the_boundary` (one- and two-element arrays at `author?`) and `a_host_record_with_an_unknown_field_is_rejected_at_an_entry_call`. Four `simple_functions.rs` tests that pinned the pass-through now assert the constructed result (defaults applied, a missing required field rejected before the body). Release notes gained an entry for the stricter entry-call boundary.
- **Verification:** Verified: entry-call records are rebuilt, so a two-element array at `author?` and an unknown field are rejected at the boundary, a one-element array is un-lifted, nested and union-case records drop empty optionals, and a nested record missing a required field is reported. The IR runtime gives the same results for the same arguments.

### ✅ Verified - RF14 Presence narrowing is not applied to property-list `if`
- **Severity:** Medium
- **Evidence:** `infer.rs` ~4648-4671 (`PropertyEntry::If`/`ConditionList`) checks the condition but never
  pushes `condition_narrowings`. `<Label if p.nick? { text={p.nick} } else { text="x" } />` reports
  "expects string, found string?". The spec narrows "the condition of an `if`".
- **Recommendation:** Wrap the then/else entries and each arm in the same push/truncate as
  `infer_under_narrowings`.
- **Fix:** Property-list `if` and condition-list arms push the condition's narrowings for their entries (`property_paths_for_entry`). Test: `a_property_list_if_narrows_its_branches`.
- **Verification:** Verified for the property-list `if` and the arm-level form. Later arms and `else` of a condition list are not narrowed by an earlier negated test, tracked as RF42.

### ✅ Verified - RF15 A `for` whose body yields nothing is typed `never*`
- **Severity:** Medium
- **Evidence:** `infer.rs` ~1007 builds `Type::seq(never, product)`. `let h(xs?:int+): int? = { for x in xs { } }`
  → "expects int?, found never*"; occurrence-types says this is the empty value, rendered `{}`.
- **Recommendation:** Normalize a `Seq` whose item is `never` to `Type::empty()` in `Type::seq`; fix the
  stale `never[]` doc at `ty.rs` ~44-45.
- **Fix:** `Type::seq` normalizes `never` under an occurrence that admits zero to the empty type, so a body that yields nothing is `{}`; the stale `never[]` doc is fixed. Tests: `a_for_whose_body_yields_nothing_is_the_empty_type`, updated rendering tests.
- **Verification:** Verified: `for x in xs { }` is accepted at `int?` and renders `{}`.

### ✅ Verified - RF16 Several checker diagnostics miss wording the specs require
- **Severity:** Medium
- **Evidence:**
  - Plain `.` on a `?` receiver says "write ?.name" rather than showing `b.author?.name`
    (presence-operators "A plain dot … SHALL show `b.author?.name`"); the test only checks `?.name`.
  - An alias in a property slot (`type Maybe = string?`, `subtitle:Maybe`) reports "admits no value
    through its type string?" and does not name `Maybe` (optional-properties scenario); "admits no
    value" also reads as "admits nothing". A malformed chain (`x:string??`) additionally reports this
    slot error.
  - `subtitle?:string = "none"` offers `subtitle:string = ...` rather than `subtitle:string = "none"`.
  - `"value: " + s` with `s?:string` says only a string, number or boolean has a text form, and arithmetic
    on `int?` gives a bare operator error; neither suggests `??`.
- **Recommendation:** Render the receiver path with the existing path helper; pass the written alias
  name through as `reject_occurrence_type_argument` already does and say "admits zero"; skip the slot
  check when suffix validation already failed; echo the default's source text; add `??`/`for` hints
  when an operand admits zero/many.
- **Fix:** The plain-dot diagnostic shows the whole path (`write b.author?.name`); the slot diagnostic names the alias ("has type Maybe (an alias to string?), which admits zero"); `+` concatenation and arithmetic on an operand that is not exactly one name `??` or `for`. The default echo is split out as RF37. Tests tightened in `presence_operators.rs`, `optional_properties.rs`; new `an_operand_that_is_not_exactly_one_is_pointed_at_the_fix`.
- **Verification:** Verified for the claimed items at every slot kind; the default echo is RF37. Residuals (a malformed chain still reported twice; unary operators give no `??` hint) are tracked in RF48.

### ✅ Verified - RF17 An element type argument `T=int?` parses as a ternary or fails to parse
- **Severity:** Medium
- **Evidence:** occurrence-types scenario "A type argument that carries an occurrence is rejected" uses
  `<Box T=int? value=1 />`; it reports a syntax error (or the ternary fix-it with a garbled `if`), and
  `<Box T=int+ …/>` is a syntax error too. Only the alias form reaches the "exactly one value" check.
- **Recommendation:** Needs a decision: either accept a suffixed type in an unbraced type-argument
  position in the grammar so the checker can reject it with the intended wording, or amend the
  scenario to the braced/alias form and accept a syntax error.
- **Status:** Not fixed: needs a decision between admitting a suffixed type in an unbraced type-argument position (grammar change with tree-sitter conflict risk around postfix `?` in property lists) and amending the scenario to the alias form.
- **Status (cycle 3):** Decided, not yet implemented: admit the suffix in the grammar and keep the scenario as written. An unbraced value is never an operator expression, so there is no conflict (checked: a scratch grammar generates with the same warnings and identical trees for every valid fixture). `contextual_name` takes glued suffixes only (`token.immediate('?' | '+' | '*')`), so `a=n + 1` and `a=c ? 1 : 2` are not read as suffixes; `Expr::ContextualName` carries them; `resolve_type_argument` applies them and reports through `reject_occurrence_type_argument`; a suffix on a name that is not a type argument gets its own diagnostic; a rejected argument counts as bound, so no "was not specified" follows (the alias form shows that cascade today). Later, optionally: disallow a space before a suffix in type positions too, for one consistent spelling.
- **Fix:** As decided. `contextual_name` admits glued suffixes (`grammar.js`); lowering keeps the first as `Expr::ContextualName.occurrence`, only inside a property value. `resolve_type_argument` reports `T=int?`, `T=int+`, `T=int*` as "A type argument must be exactly one value; 'int?' carries an occurrence" on the argument; a suffixed non-type-argument property value gets `occurrence-suffix-on-value` in `check_typed_binding_for`; post-parse validation rejects a suffix on a bare name anywhere else (`let x = foo?`, a default) and a second suffix. A rejected argument binds its parameter as the error type, so `T=Maybe` no longer adds "was not specified". `T=int ?` and `a=n + 1` remain syntax errors. Tests in `validation.rs`, `record_type_parameters.rs`, `component_type_parameters.rs`, and `occurrence_types.rs` (the scenario test now asserts the `int?` diagnostic itself). Not covered: a bare name on an element with no checked contract (`<Div x=a? />`) is not checked at all, suffix or not, as before.
- **Verification:** Verified: `T=int?`, `T=int+`, `T=int*` and `T=Maybe` each report only "A type argument must be exactly one value" on the argument, with no "was not specified" cascade; `a=n?` on a checked field gets the value-suffix diagnostic; `T=int ?` and `a=n + 1` stay syntax errors; `T=int??` adds only the suffix-chain error.

### ✅ Verified - RF18 The ternary diagnostic brings cascading errors and wrong fix-its
- **Severity:** Medium
- **Evidence:** `validate_removed_conditional_operator` (`crates/nx-syntax/src/validation.rs` ~272)
  suppresses parse errors only; `{ n > 0 ? n * 2 : -1 }` also yields "always present" and "Cannot
  compare types int and boolean". Fix-its: `g(c ? 1 : 2, 3)` → `if c { 1 } else { 2, 3 }`; a trailing
  `// comment` lands inside the braces; nested `c ? 1 : d ? 2 : 3` → `if c ? 1 : d { 2 } else { 3 }`;
  a multi-line ternary gets no fix-it.
- **Recommendation:** Stop the alternative at a depth-0 `,`, exclude comment tokens, omit the fix-it
  when the alternative contains another bare `?`, and suppress semantic diagnostics inside the salvaged
  region (or lower it to an error expression). Add these cases to the parser test.
- **Fix:** The fix-it stops the alternative at a depth-0 `,`, an unmatched closer, a comment or a line break, strips comments, gives a generic note for a nested chain, and looks back a line for a multi-line ternary (`validation.rs`). New parser test cases. Suppressing the downstream semantic diagnostics is split out as RF38.
- **Verification:** Reopened (Low leftovers): an alternative continuing on the next line is cut at the break (`c ? 1 : n +` / `2` suggests `if c { 1 } else { n + }`); `c ?` followed by `1 : 2` on the next line and braced operands get no fix-it. Skip the fix-it when the alternative ends in a binary operator, at minimum. A new false positive is tracked as RF43.
- **Fix (cycle 2):** When the consequent or alternative ends in a binary operator the note falls back to the generic `if condition { a } else { b }` form instead of a broken fix-it (`ends_in_binary_operator`, test `test_conditional_operator_alternative_cut_short_gets_no_filled_in_fix_it`). Joining a next-line continuation into a full fix-it, `c ?` at a line end, and braced operands still get only the generic message; left as accepted limitations.
- **Verification:** Verified (cycle 2): the cut-short alternative gets the generic note; every earlier positive case still gets a filled-in fix-it; the remaining shapes give a generic message rather than a wrong one, which is an acceptable limitation. A related malformed-input case is RF50.

### ✅ Verified - RF19 The `[]` diagnostic suggests invalid spellings after an existing suffix
- **Severity:** Low
- **Evidence:** `validation.rs` ~531-560 uses everything before `[` as the base: `x:string?[]` suggests
  `string?*` and `string?+`, both of which are rejected.
- **Recommendation:** Strip trailing `?`/`+`/`*`/`[]` from the base text; add `string?[]` and
  `string[][]` tests.
- **Fix:** `strip_occurrence_suffixes` removes trailing `?`/`+`/`*`/`[]` from the base before suggesting spellings. Test: `test_validate_list_suffix_replacement_drops_an_existing_suffix`.
- **Verification:** Reopened (Low gap): `(string+)[]` still suggests `(string+)*`/`(string+)+`, both rejected; `strip_occurrence_suffixes` does not look inside parentheses.
- **Fix (cycle 2):** `list_suffix_base` suggests from the enclosed type of a parenthesized base that carries a suffix (`(string+)[]` → `string*`/`string+`), leaving function types alone; tests added.
- **Verification:** Verified (cycle 2): `(string+)[]` and `((string?))[]` suggest `string*`/`string+`; function types in parentheses are left alone.

### ✅ Verified - RF20 Hover and navigation do not treat the member of `x?.m` as a member access
- **Severity:** Medium
- **Evidence:** `member_access_context` (`crates/nx-language-service/src/positions.rs` ~452-466) accepts only
  `MEMBER_ACCESS_EXPRESSION`; `is_expression_container` (~767-780) omits `OPTIONAL_MEMBER_EXPRESSION` and
  `EXISTS_EXPRESSION`; `member_access_hover` (`lib.rs` ~575) handles only `Expr::Member`. A cursor on `name`
  in `b.author?.name` falls back to a plain name reference (from code).
- **Recommendation:** Accept the new kinds in both helpers, handle `Expr::OptionalMember`, add a hover
  test on `b.author?.na|me`.
- **Fix:** `member_access_context` and `is_expression_container` accept the new node kinds and `member_access_hover` handles `Expr::OptionalMember`; hover and position tests added.
- **Verification:** Verified by code and the new hover/position tests.

### ✅ Verified - RF21 Typegen maps a standalone `T?` to a nullable type, but such a value encodes as `[]`
- **Severity:** Medium
- **Evidence:** `typescript.rs` ~965 and `csharp.rs` ~1747 map type-position `T?` to `T | null` / `T?`;
  `let root(): string? = { if false {"a"} }` encodes `[]` (asserted in `sdk-node.test.ts` ~403). The .NET
  `NxRuntime.Evaluate<string?>` (`NxRuntime.cs` ~301-351) deserializes with MessagePack and will fail on
  an array. design.md calls the asymmetry cosmetic, but on .NET it is a failure.
- **Recommendation:** Needs a decision: map standalone `T?` to `T | readonly never[]` and teach the .NET
  SDK to read a top-level `[]` as `null`, or encode an empty top-level `?` as `null`. Add the missing
  .NET test either way.
- **Status:** Not fixed: needs a decision. Options: (a) typegen maps a standalone `T?` to `T | readonly never[]`/an array-tolerant C# shape and the .NET SDK reads a top-level `[]` as `null`; (b) the encoder writes an empty top-level `?` as `null`, which is type-directed encoding the design rejected. design.md calls the asymmetry cosmetic, but .NET `Evaluate<string?>` fails on it.
- **Fix:** Decided that an entry call returns an empty result whose type, declared or inferred, is a standalone `T?` as `null`, and every other empty result as `[]`. Encoders stay value-directed and the runtimes keep `[]` internally. New host-only `NxValue::Null` (JSON `null`, MessagePack nil): a host `null` decodes to it and NX reads it as empty. A cleared update-record field now converts to it in `to_nx_value`, so the serializer's `.Update` special case is gone. `nx_api::entry_result_to_nx_value` applies the rule for `eval_source`, `eval_program_artifact` and `eval_program_artifact_function`, using the checker's bound result type, and so does the CLI's `run --format json`. The IR runtime's `evaluateFunction` and `callFunction` use the RF6 optional-result flag. A generated JS function stays typed NX code returning `nxEmpty` (its signature says `T | NxEmpty`), and the JS comparisons accept `[]` for a recorded `null`. Tests: .NET `Evaluate_OptionalResult_ReadsAsNullableType` (`Evaluate<string?>`, `Evaluate<int?>`, `Evaluate<int[]>`); nx-api `an_entry_calls_empty_optional_result_is_null`; a CLI JSON test; `sdk-node.test.ts` now expects `null`; corpus `noString`/`noneInferred`/`noInts`, where the `occurrences` corpus's `noName`/`noNick` now record `null`. design.md records the decision, replacing the "cosmetic" risk, and the occurrence-types delta, Node README and release notes are updated. Typegen is unchanged.
- **Verification:** Verified: the empty results of `aliasOpt(): M` (with `M = int?`), an inferred `p.nick` result and an optional member read are `null` in the interpreter (CLI JSON) and the IR runtime, `callFunction` agrees, `int*` stays `[]`, and generated JS returns `nxEmpty` as designed. .NET `Evaluate_OptionalResult_ReadsAsNullableType` passes against a rebuilt `libnx_ffi` (the release library in `target/` was stale and had to be rebuilt first).

### ✅ Verified - RF22 Typegen accepts field shapes the language rejects
- **Severity:** Medium
- **Evidence:** typegen reads a lowered module without type checking; `model.rs` ~27-29 documents that a
  field type never carries `?`/`*` but nothing enforces it. `export type P = { all:string* sub:Maybe }`
  exits 0 and emits `NxProperty<P, string?> Sub … clearable: false`. The unit test at `typegen.rs` ~2449
  uses `all:string*` and locks the output in.
- **Recommendation:** Reject (or type-check before typegen) a field whose type admits zero, and replace
  the `all:string*` fixture.
- **Fix:** Typegen fails on the checker's `optional-in-type-slot` diagnostic (single-file and library paths), reusing its message; the `all:string*` fixture is removed; new test `a_field_whose_type_admits_zero_is_rejected`.
- **Verification:** Verified: single-file and library typegen fail naming the field and fix for direct, alias, state and function-type-parameter slots. The single-file error lacks a source location (RF47).

### ✅ Verified - RF23 The external-components delta and .NET README say an empty optional `+` prop is an array
- **Severity:** Medium
- **Evidence:** `specs/external-components/spec.md` ~52-55 and `bindings/dotnet/README.md` ~569-571 ("or an
  empty array for a `name?:T+` field") contradict nx-ir-format (~257-260), `nx-api/src/value.rs` and the
  Node test, which omit the key.
- **Recommendation:** Reword the delta to say the key is omitted; drop the README parenthetical.
- **Fix:** The external-components delta says an empty optional prop is an omitted key (an array when present); the .NET README parenthetical is removed.
- **Verification:** Verified against nx-ir-format and the codegen test for `<Name>Element`. A line-wrap glitch in the delta is tracked in RF47.

### ✅ Verified - RF24 Generated-JS record schemas validate optional fields at the declared type
- **Severity:** Medium
- **Evidence:** `emit.rs` ~2759 and ~2829 pass `&field.ty` rather than the read type. Host props
  `{book:{$type:"Book",title:"x",tags:[]}}` for `tags?:string+` throw "expected at least one value" in JS
  (`sub:null` throws "invalid value for string") while the IR runtime accepts both.
  `nxNormalizeRecordInput` (`runtime.rs` ~487, ~1052) would then still keep `tags: []`.
- **Recommendation:** Validate against `field.ty.read_type(field.optional)` and drop empty values of
  optional fields in `nxNormalizeRecordInput`.
- **Fix:** JS record, component and boundary schemas use the alias-resolved read type, record schema fields carry `optional: true`, and the normalizer drops an empty optional field (`emit.rs`, `runtime.rs`).
- **Verification:** Verified: host props with `tags:[]`/`sub:null` are accepted by generated JS and render identically to the IR runtime.

### ✅ Verified - RF25 Emitted-JS occurrence decisions read the declared `TypeRef` and miss aliases
- **Severity:** Medium
- **Evidence:** `lift_to_declared_occurrence`, content `binds_many` (`emit.rs` ~3461, ~3501, ~3545) and the
  `nxOptional` many flag (~1906). With `type Ints = int+`: `<R xs={3}/>` gives JS `3` vs `[3]`; `xs?:Ints`
  holding one item is un-lifted to `1` in JS.
- **Recommendation:** Decide from the alias-resolved type (`CodegenTypeRef::Seq { occ }`).
- **Fix:** Every emitted occurrence decision (sequence lift, content `binds_many`, `nxOptional` many flag, empty value) reads the alias-resolved type (`emit.rs`).
- **Verification:** Verified: alias lifts, alias content and one-element `Ints` at `xs?:Ints` agree in all three engines; only the RF6 value case differs.

### ✅ Verified - RF26 Generated JS does not lift call arguments or a `??` right operand to a `+`/`*` site
- **Severity:** Medium
- **Evidence:** `let f(xs?:int+) = { xs ?? 5 }`, `f(3)` → JS `3`, interpreter/IR `[3]`; `p(5)` for `xs:int+` →
  JS `5` vs `[5]`; `<R xs={xs ?? 5}/>` with `xs` empty → JS `5`.
- **Recommendation:** Lift arguments to the parameter's occurrence and lift the right operand of `??` when
  the result admits many and the operand does not.
- **Fix:** Call arguments to a known declaration are lifted to the parameter's occurrence, and a `+`/`*` site wraps a join that may hold a bare item as `[v].flat()`; the bare `??` right operand is deliberately not lifted, since the interpreter and IR return the item there too.
- **Verification:** Verified: `f(3)`, `<R xs={xs ?? 5}/>`, `p(5)`, element-call and alias-typed parameters all give `[5]` where expected in all three engines.

### ✅ Verified - RF27 Three-engine parity is not tested for generated JS, and one scenario cannot pass
- **Severity:** Medium
- **Evidence:** `corpus.test.mjs` compares only the IR runtime to recorded interpreter output; generated JS
  never runs over the corpus (task 5.4 claims all three agree), which is why RF8, RF25 and RF26 were
  not caught. executable-code-generation "Generated JavaScript matches the empty pattern" cannot pass
  because executable codegen rejects every match (acknowledged only in a test comment).
- **Recommendation:** Add a generated-JS pass over the corpus entry points; amend the scenario (or
  implement match) so the spec does not claim unsupported behavior.
- **Fix:** The executable-code-generation delta now says JS codegen refuses a match on `{}` (renamed scenario, `{}` dropped from the parity scenario). Added `generated_javascript_agrees_with_the_recorded_results` (JS corpus pass, skipping only constructs refused as unsupported) and a match-free corpus program `specs/ir-conformance/occurrence-lifting/`.
- **Verification:** Verified as written (spec amended, JS corpus pass, `occurrence-lifting` program). Coverage gap: skipping is per program, so the operator entry points in `occurrences` never run as JS; tracked as RF45.

### ✅ Verified - RF28 Reading a field of an update record is typed exactly-one but may be empty
- **Severity:** Medium
- **Evidence:** `type User = { name:string email?:string }`, `let g(u:User.Update): string = { u.name }` type
  checks, then `g(<User.Update email={} />)` fails "expected string, got {}"; JS returns `undefined`.
- **Recommendation:** Needs a decision: type every `T.Update` field read as `T?` (`T*` for `+`), or
  disallow member reads on update records; the update-records delta does not say.
- **Status:** Not fixed: the update-records delta does not say what reading a `T.Update` field yields. Options: type every update field read as `T?` (`T*` for `+`), matching the runtime, or reject member reads on update records in favour of the intrinsics.
- **Fix:** Every field of an update record now reads as an optional field does: `T?`, or `T*` for a `+` field (`infer_item_member_access` in `infer.rs`), so the example's `g` is rejected with "expects string, found string?". Absent and cleared both read as empty in every engine; generated JS already emits `?? nxEmpty` for a read typed as admitting zero, which also covers a cleared field stored as `null`. New update-records requirement "Reading a field of an update record admits zero", with scenarios. Tests: `a_field_of_an_update_record_reads_as_optional` (`update_records.rs`) and `occurrence-lifting` corpus entrypoints `updatedName`, `absentName`, `clearedNick`, `updatedNick`, `absentRead` and `clearedRead`, held in the interpreter, the IR runtime and generated JS.
- **Verification:** Verified: `u.name` at `string` is rejected naming `string?`; `u.tags` reads as `string*`; `u.name?` narrows; absent and cleared fields read as empty, and `changed` tells them apart, identically in all three engines.

### ✅ Verified - RF29 Optional parameters cannot be omitted in a positional call
- **Severity:** Medium
- **Evidence:** optional-properties lists "function call" among the sites where an unwritten optional
  binds empty; `let f(a:int, b?:int)`, `f(1)` → "Function expects 2 arguments, got 1" (checker
  `infer.rs` ~2321; runtime `interpreter.rs` ~885, ~3168). Element-call syntax omits correctly.
- **Recommendation:** Needs a decision: allow omitting trailing optional parameters (checker, interpreter,
  IR, JS), or say in the spec that positional calls must pass `{}`.
- **Status:** Not fixed: the spec text ("function call") is ambiguous. Options: allow omitting trailing optional parameters in positional calls (checker, interpreter, IR runtime, JS), or state that a positional call passes `{}` explicitly.
- **Fix:** Decided with the author and specified as the new `function-parameters` capability. A paren-style signature lists its optional and defaulted parameters last (`required-parameter-after-omissible`). A positional call may stop before them; the checker reports "Function expects 1 to 3 arguments". An element call may skip any of them. A paren call of an element-style function is rejected, showing the element form. The function fills each parameter left out: with its default, or with `{}` when it is optional. Parameter defaults were parsed and silently dropped in both styles; they now reach HIR, are checked against their parameter with only earlier parameters in scope, and are evaluated in the callee:
  - interpreter: one `bind_function_params` for positional, element, function-value and host calls;
  - IR: a default node per parameter, and an absent call argument `-1`;
  - TS runtime: `invokeFunction`;
  - generated JS: JavaScript default parameters, with `undefined` for a skipped argument.

  The interpreter also ran a function from another module in the caller's context, so neither its body nor its defaults could read that module's top-level values ("Undefined variable"). It now runs in a context with that module's values. New tests: `crates/nx-types/tests/function_parameters.rs`, `crates/nx-interpreter/tests/function_parameters.rs`, an IR test and a TS runtime test. The corpus program `parameter-defaults` includes a default that reads a value private to another module, and all three engines agree. Docs, docs/nx-ir-format.md, the nx-ir-format delta and `specs/future.md` ("Function parameters") are updated.
- **Verification:** Verified: every function-parameters scenario (`add(1)` 11, `add(1, 2)` 3, `<add a=1 c=5 />` 16, `count()` `[7]`, `<Area w=3 />` 9, `<F b=1 />`, a record default, a host call `add(2)` 22) agrees in the interpreter, the IR runtime and generated JS. Order, range, element-style and default-type diagnostics fire as specified, and a function from a separately loaded library fills its defaults, including one that reads a private value. A related function-type compatibility question is RF52.

### ✅ Verified - RF30 An empty attribute on an undeclared (host) element is emitted as `[]`
- **Severity:** Low
- **Evidence:** `<Label text="x" note={if c { "n" }} />` with `c=false` outputs `"note": []` and NX text
  `note={}`; occurrence-types says an empty optional attribute is omitted.
- **Recommendation:** Drop empty-valued attributes when building a record for an undeclared element.
- **Fix:** An undeclared element drops empty-valued attributes (`eval_element_expr`); two interpreter tests updated to assert the key is omitted.
- **Verification:** Verified (`<Label text="x" />`). Side effect worth a release note: an empty body on an undeclared element no longer emits a `content` key.

### ✅ Verified - RF31 An empty optional state field is stored inconsistently
- **Severity:** Low
- **Evidence:** `apply_component_update` (`interpreter.rs` ~1320) inserts an empty value while
  `apply_update_record` (~3064) removes it and initialization never stores it; generated JS
  `initial<C>State` stores `{sel: []}` (`emit.rs` ~2447, ~2940) where the IR runtime stores `{}`.
- **Recommendation:** Remove the key when the value is empty in all three paths.
- **Fix:** `apply_component_update` removes the entry for an empty value; generated JS initial state omits an empty optional field, reads it as empty when absent, and types it optional.
- **Verification:** Verified in both paths; the interpreter change introduced RF40.

### ✅ Verified - RF32 TypeScript typegen drops the `?` on function-type parameters
- **Severity:** Low
- **Evidence:** `typescript.rs` ~985 ignores `FunctionParam.optional`; `<function Item:string Index?:int />`
  emits `{ Item: string; Index: number }`.
- **Recommendation:** Emit `Index?: number` and add a test.
- **Fix:** The TS typegen emits `Index?: number`; test `an_optional_function_type_parameter_is_an_optional_typescript_member`.
- **Verification:** Verified: `(args: { Item: string; Index?: number }) => string`.

### ✅ Verified - RF33 .NET update-record API gaps
- **Severity:** Low
- **Evidence:** `NxUpdate.Diff` (`NxUpdate.cs` ~120-127) calls `SetFieldValue` directly, bypassing the
  clearable check `Set` enforces, and carries `[]` rather than cleared for an emptied array; the
  clearable message is written three times (`NxField.cs` ~64-71, `NxUpdateRecordJsonConverter.cs` ~73-78,
  `NxUpdateRecordMessagePackFormatter.cs` ~109-114); `NxUpdateRecordTests.cs` ~937-939 comment contradicts
  its assertions.
- **Recommendation:** Call `CheckClearable` in `Diff`, share one message helper, fix the comment.
- **Fix:** `Diff` calls `CheckClearable` and carries an emptied field as cleared (matching the TS runtime's `nxDiffRecords`); `NxField.CannotClearMessage` is shared by the three sites; the stale test comment is rewritten; test `Diff_EmptyingANonClearableField_ThrowsNamingTheField`.
- **Verification:** Verified: `Diff` normalizes an emptied field to cleared and checks clearability; shared message; comment fixed; tests pass. `Diff` now throws for an invalid `after` where the runtime used to reject the patch, which is documented.

### ✅ Verified - RF34 Spec and doc inaccuracies
- **Severity:** Low
- **Evidence:**
  - presence-operators and `nx-grammar.md` ~403-405 say `??` binds tighter than every arithmetic and
    logical operator; prefix `-`/`!` bind tighter (`-x ?? 1` is `(-x) ?? 1`).
  - presence-operators "Narrowing does not escape its branch" expects `string?`, but the trailing `o` is
    collected with the `if` into a `string*` body, which the implementation (correctly) reports.
  - proposal lists `sdk-node` as modified but there is no delta.
  - cli-code-generation "Non-emptiness is validated at the boundary" names a generated `Payload`; the
    .NET test uses a hand-written DTO.
  - task 5.3 and design say `T?` emits as `T | null`; the emitter writes `T | NxEmpty`.
  - task 7.1 describes source-formatter work, but `format.rs` is a value printer.
  - `nx-grammar-spec.md` ~842/916 lists no empty pattern kind.
  - `docs/.../reference/syntax/functions.md` `formatName` snippet does not parse.
- **Recommendation:** Fix each wording; drop `sdk-node` from the proposal or add a delta.
- **Fix:** presence-operators and `nx-grammar.md` say `??` binds below prefix `-`/`!`; the no-escape scenario names `string*`; `sdk-node` is dropped from the proposal's modified list (the Node contract is the canonical encoding); the cli-code-generation scenario matches the tested host payload; tasks 5.3 and 7.1 describe what was built; `nx-grammar-spec.md` has the `empty` pattern kind; the `functions.md` snippet parses.
- **Verification:** Verified: `??` precedence wording, the no-escape scenario, the proposal's sdk-node bullet, the Payload scenario, tasks 5.3/7.1, `nx-grammar-spec.md` and the `functions.md` snippet.

### ✅ Verified - RF35 Minor diagnostics and cleanup
- **Severity:** Low
- **Evidence:**
  - `let x:string? = null` suggests writing `"null"` (misleading); `x? .m` reports "Member access not
    yet implemented".
  - Cascades: a rejected record type argument also reports "Type parameter 'T' … was not specified";
    a non-exhaustive match also reports a return mismatch.
  - `type_satisfies_expected_with_coercion` is a pass-through in `semantics.rs` and `infer.rs` ~6885;
    array wording remains in messages (`infer.rs` ~785/797, ~1432).
  - `crates/nx-cli/src/main.rs` ~2863-2870 split a doc comment between two tests.
  - Stale `[]` in `crates/nx-hir/src/components.rs` tests (~2223, ~2401) and comments (~1220,
    `lib.rs` ~468).
  - `Value::type_name()` returns `"empty"` where coercion errors say `{}`.
  - The `.Update` name test is duplicated (`nx-value/src/lib.rs` ~263, `nx-api/src/value.rs` ~149,
    `index.ts` ~2922, `emit.rs` ~1210).
  - `ir_image.rs` ~175 does not validate the `seq` occurrence cell or a nested `seq`.
  - `emitted-ir.test.mjs` ~659-662 has a stale comment.
- **Recommendation:** Special-case `null` in the unknown-name message; remove the pass-through; fix the
  comments and stale spellings; share the update-name helper; validate `seq` in the Rust reader.
- **Fix:** Done: `null` at a typed site now says NX has no `null` and names `{}` (test `null_at_a_typed_site_names_the_empty_value_not_a_string`); `main.rs` doc comment restored; stale `[]` in nx-hir tests/comments replaced; `Value::type_name()` returns `{}`; `ir_image.rs` validates the `seq` occurrence cell and nested `seq` (test added); stale `emitted-ir.test.mjs` comment removed. The remaining cleanup items are split out as RF39.
- **Verification:** Verified: `null` message, `main.rs` doc, nx-hir `[]`, `type_name`, `ir_image.rs` `seq` validation, emitted-ir comment.

### ✅ Verified - RF36 Test coverage gaps on risky paths
- **Severity:** Low
- **Evidence:** No parser assertions for `??` vs `*`, `!a?`, `a? && b?`, `x? .m` or unary vs `??`;
  `empty_value_span` (`positions.rs` ~668) misses `{ /* c */ }` and the `{}` pattern; the removed
  `test_format_empty_list_at_a_field_with_a_non_empty_default_still_renders` has no replacement
  asserting that `{}` at a defaulted `+` field is rejected.
- **Recommendation:** Add the listed tests.
- **Fix:** Parser shape tests (`test_presence_operator_precedence_shapes`), `empty_value_span` comment/pattern handling with tests, and `the_empty_value_at_a_defaulted_one_or_more_field_is_rejected` (nx-types) added.
- **Verification:** Verified: parser precedence-shape test, `empty_value_span` for comments and the pattern, the nx-types negative test.

### ✅ Verified - RF37 The optional-with-default diagnostic does not echo the default
- **Severity:** Low
- **Evidence:** Split from RF16. `subtitle?:string = "none"` offers `subtitle:string = ...`; the
  optional-properties scenario asks for `subtitle:string = "none"`. The checker works on HIR and has
  no source text for the default expression.
- **Recommendation:** Render a literal default from HIR, or carry the default's source text into the
  diagnostic.
- **Status:** Not fixed: no clean way to recover the default's source text in the checker without a new
  HIR/source plumbing decision.
- **Fix:** The rule moved from the checker to post-parse validation (`validate_optional_property_defaults` in `validation.rs`), where every property slot is a `property_definition` with `optional`, `type` and `default` fields. The diagnostic quotes the type and default as written: `write \`subtitle:string = "none"\` or \`subtitle?:string\``. An alias keeps its name, and a default over 40 characters or on several lines shows as `...`. A function type's parameter and a type parameter are left to their own no-default rules, so neither is reported twice. The checker's check and the `optional`/`has_default` arguments of `property_slot_type` are removed. Tests: `a_default_on_an_optional_property_is_rejected` now asserts the quoted forms; new `the_default_is_quoted_as_written_in_every_property_slot` (alias, element-style parameter, prop, long default), `test_validate_optional_property_with_default_quotes_both_forms`; the paren-parameter case in `function_parameters.rs` asserts `write \`b:int = 2\` or \`b?:int\``.
- **Verification:** Verified: the diagnostic quotes the type and default as written for a record field, an alias, a `+` default, an element-style parameter, a component prop, a function-typed field and a negative literal; a long default becomes `...`; function-type parameters and type parameters report only their own rules.

### ✅ Verified - RF38 The ternary diagnostic still brings downstream semantic errors
- **Severity:** Low
- **Evidence:** Split from RF18. `{ n > 0 ? n * 2 : -1 }` still also reports "always present" and
  "Cannot compare types int and boolean", because the salvaged `x?` reaches the checker.
- **Recommendation:** Lower a presence test whose span overlaps a `removed-conditional-operator`
  diagnostic to `Expr::Error`, or filter overlapping semantic diagnostics.
- **Status:** Not fixed: needs a choice between lowering-level and diagnostic-level suppression.
- **Fix:** Lowering-level. In a sequence, an item that ends in a presence test and is followed by a parse error, directly or after one more item (the consequent, which flat sequences keep as its own item in `{ c ? 1 : 2 }`), lowers to `Expr::Error`, and so does that consequent (`salvaged_conditional_items` in `lower.rs`). The checker is silent on an error expression, so `{ n > 0 ? n * 2 : -1 }` now reports only `removed-conditional-operator`. Earlier items in the sequence are still checked. Considered and set aside: parsing the ternary as a grammar node, which a scratch grammar showed commits to the flat-sequence reading (`c?` then `"a"`) before the `:`. Test `a_conditional_operator_brings_no_diagnostics_about_its_salvaged_test` (`presence_operators.rs`) covers braced, flat, list, unbraced and property-value shapes, and that an undefined name before the ternary is still reported.
- **Verification:** Verified: `{ n > 0 ? n * 2 : -1 }` reports only `removed-conditional-operator`, and an undefined name before the ternary is still reported. A related wrong fix-it is RF53.

### ✅ Verified - RF39 Remaining low-priority cleanup
- **Severity:** Low
- **Evidence:** Split from RF35: `x? .m` reports "Member access not yet implemented"; a rejected
  record type argument also reports "Type parameter 'T' … was not specified"; a non-exhaustive match
  also reports a return mismatch; `type_satisfies_expected_with_coercion` is a pass-through; array
  wording remains in some messages (`infer.rs` index errors, `empty-list-element-type-unknown`); the
  `.Update` name test is duplicated across `nx-value`, `nx-api`, `index.ts` and `emit.rs`; alias
  diagnostics inside a declaration span the whole declaration.
- **Recommendation:** Address as a follow-up cleanup pass.
- **Status:** Not fixed: low value relative to risk in this pass; none affects behavior.
- **Fix:** Six of the seven are fixed; the seventh is recorded in `specs/future.md`.
  - `x? .m` now reports `member-access-on-presence-test`: a presence test is a boolean with no members, so write `x?.m` with no space (the fix shows the receiver's path when it has one). Any other value with no members reports `unknown-member`, "A value of type string has no member 'length'", in place of "Member access not yet implemented".
  - An applied type whose argument was rejected (`<Box U=int/>`) no longer also reports the parameter it left unbound. `applied_type` returns before the missing-parameter pass. A record literal `<Box U=int value=1 />` still reports both "T was not specified" and "no field 'U'", since there `U` reads as a field.
  - A non-exhaustive union match is no longer read as an implicit `else { }`, so its missing-cases error comes without a return mismatch.
  - `type_satisfies_expected_with_coercion` is removed from `semantics.rs`, `infer.rs` and the crate exports, and its callers use `type_satisfies_expected`.
  - Array wording is gone from the messages. The checker says "Index must be an integer" and "Cannot index into T, which is not a sequence". The diagnostic code is now `empty-value-type-unknown`, and the interpreter's runtime says "Index … is out of bounds" and names a sequence value `sequence`.
  - `emit.rs` uses `nx_hir::is_update_record_name`. In Rust it was the only copy of the check; nx-value has none, and the TypeScript runtime's copy cannot share a Rust helper.
  - Deferred: alias and type-argument diagnostics span the whole declaration because HIR's `TypeRef` has no spans. The fix touches every crate that builds or matches a `TypeRef`. See "HIR type references carry no span and no error form" in `specs/future.md`.
  - Tests:
    - new: `a_member_of_a_presence_test_names_the_step_spelling` and `a_member_of_a_value_with_no_members_names_its_type` (`presence_operators.rs`);
    - changed: the two unknown-argument tests in `record_type_parameters.rs` now require the missing parameter to go unreported, and `test_non_exhaustive_union_match_without_else_is_rejected` requires exactly one error.
- **Verification:** Verified: `o? .name` names `o?.name`; `s.length` says a `string` has no member; `<Box U=int/>` reports only the unknown argument; a non-exhaustive union match reports only its missing cases; `type_satisfies_expected_with_coercion` is gone; `empty-value-type-unknown` replaces the array wording; `emit.rs` uses `is_update_record_name`; the deferral is in `specs/future.md`.

## New Findings Discovered During 2026-09-22 02:40 Verification

### ✅ Verified - RF40 A handler later in a dispatch batch reads a stale value for a state field an earlier handler cleared
- **Severity:** Medium
- **Evidence:** Introduced by RF31. `invoke_action_handler_in` (`crates/nx-interpreter/src/interpreter.rs`
  ~980-988) binds the render-time captured variables, then overrides only `owner_state` names present in
  `live_state`. Clearing now removes the entry, so the override is skipped and the render-time value shows
  through (state `note?:string` rendered `"x"`, batch `[<Update note={} />, handler reading note]` sees `"x"`).
- **Recommendation:** Bind `Value::empty()` for an `owner_state` name absent from `live_state`, and add a
  clear-then-read dispatch test.
- **Fix (cycle 2):** `invoke_action_handler_in` binds `Value::empty()` for a live state name absent from the working state. Test `a_cleared_optional_state_field_reads_as_empty_later_in_the_batch` (fails without the fix).
- **Verification:** Verified (cycle 2): an absent owner-state name binds empty; the new test exercises the stale-read path; required state fields are always stored, so only a cleared optional can be absent.

### ✅ Verified - RF41 A numeric literal compared with a `?`/`+`/`*` value is not typed by its item type
- **Severity:** Medium
- **Evidence:** Introduced by RF3. With `h:float32? = {0.1}`, `h == 0.1` is `false` (`g:float32` gives
  `true`), and `int32? == 9999999999` is accepted without the range error. `convert_literal_operand`
  (`infer.rs`) converts only against a bare numeric primitive.
- **Recommendation:** For `==`/`!=`, convert literals against `other.item()`.
- **Fix (cycle 2):** `convert_literal_operand` types a literal operand of `==`/`!=` by the other side's item type. Tests: `a_literal_compared_with_an_occurrence_is_typed_by_the_item_type` (checker, range error) and `a_literal_compared_with_an_optional_float32_takes_its_item_type` (interpreter).
- **Verification:** Verified (cycle 2): `float32`, `float32?`, `float32+` compare equal to `0.1` with the literal on either side; `int32? == 9999999999` reports the range error; `string? == 1` is still rejected.

### ✅ Verified - RF42 A property-list condition list does not narrow later arms by an earlier negated test
- **Severity:** Low
- **Evidence:** `<Label if { !p.nick? => text="x" else => text={p.nick} } />` reports "expects string,
  found string?"; the expression form is accepted because it lowers to nested `if`/`else`.
- **Recommendation:** Push each earlier arm's `condition_narrowings(cond, false)` for later arms and
  `else_entries`.
- **Fix (cycle 2):** Each condition-list arm and the `else` see the earlier arms' false-branch narrowings (`property_paths_for_entry`). Test `a_property_list_condition_list_narrows_later_arms_by_earlier_refuted_tests`.
- **Verification:** Verified (cycle 2): the negated-arm condition list is accepted; the expression form and property-list `if … else` still pass.

### ✅ Verified - RF43 The ternary detector fires on errors that are not ternaries and hides the real error
- **Severity:** Medium
- **Evidence:** `let f(a?:int): int = { a ) : 3 }` reports only the ternary message, suggesting
  `if a { :int): int = { a ) } else { 3 }`; the `?` of the parameter's optional mark is taken as the
  ternary's `?` (`bare_question_before`/`is_bare_question` in `validation.rs`).
- **Recommendation:** Only accept a bare `?` inside the same expression container as the error, never one
  inside a property definition; add the case as a negative test.
- **Fix (cycle 2):** A `?` before the error is taken as the ternary's only when it is the operator of a well-formed presence test (`is_presence_test_operator`); `{ a ) : 3 }` after `a?:int` now reports the real parse error. Test `test_optional_mark_is_not_taken_for_a_conditional_operator`.
- **Verification:** Verified (cycle 2): the reported case and six further non-ternary errors near optional marks and suffixes report their real parse errors.

### ✅ Verified - RF44 The single-item `for` rule is decided at runtime in two engines and statically in JS
- **Severity:** Medium
- **Evidence:** `let f(xs?:int+) = { for x in xs ?? 5 { x + 1 } }`, `f({})`: interpreter `6`, IR `6`,
  JS `[6]`, because a `+`-typed `??` result may hold a bare item (RF26 leaves the right operand unlifted).
- **Recommendation:** Tied to RF6: lift a `??` right operand to the result's occurrence when the result
  admits many in every engine, or decide the single-item case statically everywhere (mark it on the IR
  `for` node).
- **Status (cycle 2):** Not fixed: needs the RF6 decision (lift a `??` right operand to the result's occurrence in every engine, or mark the single-item case statically on the IR `for` node).
- **Status (cycle 3):** Still open. RF6 normalizes only a declared result, matching the interpreter, so an unannotated `xs ?? 5` result (`orFive({})` in the `occurrence-lifting` corpus) is still a bare `5` in every engine. Deciding the single-item case statically, or lifting the `??` right operand, remains to be done.
- **Fix:** `??` is now lifted like the other joins. The checker already recorded a `??` operand that the result lifts from an item to a sequence (`record_join_lifts`), but `apply_join_lifts` and `apply_join_widenings` rewrote only `if` and match branches; both now treat `??` as a join through a shared `join_branches_mut`, so either operand is wrapped as a one-item sequence (an empty left operand stays empty, so the fallback test holds) and widened to the join's numeric type. A value whose type admits many is therefore a sequence in every engine, so the interpreter's runtime `for` decision agrees with generated JS's static one without marking the `for` node: `f({})` is `[6]`, `orFive({})` `[5]`, `o ?? ys` with `o = 1` `[1]`, and `o ?? 1.5` with `o = 1` is `1.0` (it was the int `1`). Generated JS's `may_hold_a_bare_item` site wrapper (RF26) is removed as dead. The `occurrence-lifting` corpus gains `fallbackLoop`, `presentLifted` and `presentWidened` and `fallbackItem` becomes `[5]`, agreeing in the interpreter, the IR runtime and generated JS. The presence-operators `??` requirement and the sequence-model lift requirement state the rule, each with a scenario; the `??` reference doc says so too.
- **Verification:** Verified: `f({})` `[6]`, `orFive({})` `[5]`, `itemOr({2 3}, 1)` `[1]`, `halfOr(1)` `1.0`, chained `??` with `+` operands, `lenOf(o ?? 9)` and `nickOr` agree in all three engines, and the presence-operators and sequence-model deltas state the rule with scenarios.

### ✅ Verified - RF45 The generated-JS corpus pass skips whole programs
- **Severity:** Low
- **Evidence:** `generated_javascript_agrees_with_the_recorded_results` skips a program when any construct
  is refused; `occurrences` is skipped only because of one match, so its `x?`, `x?.m` and `??` entry
  points never run as JS.
- **Recommendation:** Skip per entry point, or move the match entry points of `occurrences` into a
  separate program.
- **Fix (cycle 2):** The `{}`-match entry points moved from `specs/ir-conformance/occurrences` to a new `occurrence-patterns` program, so `occurrences` runs as generated JS; 66 entry points now agree across the three engines (was 47).
- **Verification:** Verified (cycle 2): `occurrences` runs as generated JS; 66 entry points agree.

### ✅ Verified - RF46 The value-equality scenario still does not parse as written
- **Severity:** Low
- **Evidence:** "An item compares as a sequence of one" writes `{one == { 1 2 }}`; a non-empty brace is
  not an operand (RF4 kept that), so it is a syntax error.
- **Recommendation:** Reword to `let two = { 1 2 } let other = {one == two}`.
- **Fix (cycle 2):** The value-equality scenario binds `let two:int+ = { 1 2 }` and compares `one == two`.
- **Verification:** Verified (cycle 2): the reworded scenario parses and gives `same=true other=false`.

### ✅ Verified - RF47 Typegen error location and spec/README formatting
- **Severity:** Low
- **Evidence:** The single-file typegen error joins messages with `; ` and has no file/line/column
  (`reject_fields_admitting_zero`, `model.rs`); `specs/external-components/spec.md` ~55-56 has a broken
  line wrap; the edited `bindings/dotnet/README.md` ~569 line is 137 characters.
- **Recommendation:** Render the diagnostics as the library path does; reflow both paragraphs.
- **Fix (cycle 2):** Single-file typegen renders the rejected diagnostics with file, line, column and source excerpt (reading the source back; bare messages if it cannot); the external-components paragraph and the README sentence are reflowed.
- **Verification:** Cycle 2 reopened the README part (the overflow had moved to the next line); the paragraph is now reflowed to 120 columns (checked by line length). The typegen locations and the external-components reflow were verified by the original reviewer. Nit: locations are rendered only when the source file can be read back, so an in-memory source gets bare messages.

### ✅ Resolved - RF48 Residual low-priority checker/runtime gaps
- **Severity:** Low
- **Evidence:** `x:string??` still reports both the suffix and the slot error; unary `-`/`!` on `int?`/
  `boolean?` give no `??` hint; `record_field_reads_as_empty` still consults a same-named local type
  before falling back, so a foreign record shadowed by a local type of the same name can still fail.
- **Recommendation:** Skip the slot check when suffix validation failed; add the hint to unary operators;
  have `record_field_reads_as_empty` return `true` (the checker guarantees the member).
- **Fix (cycle 2):** Partial: unary `-`/`!` on an operand that is not exactly one now name `??`/`for` (test `a_unary_operand_that_is_not_exactly_one_is_pointed_at_the_fix`).
- **Status (cycle 2):** Still open: the double report for `x:string??` needs the checker to know suffix validation failed; making `record_field_reads_as_empty` return `true` unconditionally would also silence the lazy missing-field error for unrebuilt entry-argument records (RF13), so it waits on that decision.
- **Fix (cycle 3):** Partial: with RF13 fixed every record is constructed against its declaration, so `project_member` reads an unstored field as the empty value without consulting a shape; `record_field_reads_as_empty` and `RuntimeErrorKind::RecordFieldNotFound` are removed (an undeclared member is the checker's error, `unknown_member_suggests_a_near_match`). `test_undefined_record_field` became `test_unstored_optional_record_field_reads_as_empty`, and `test_record_missing_field_errors` was removed. The `x:string??` double report remains open.
- **Resolution:** Deferred to `specs/future.md` ("HIR type references carry no span and no error form"). Lowering keeps the first of two suffixes, so the checker sees `x:string?` and applies the slot rule. Suppressing that needs an error form on HIR's `TypeRef`, which reaches about a dozen files across the checker, codegen, typegen and the language service. The same rework gives `TypeRef` the spans RF39 deferred, so the two are recorded together.

### ✅ Verified - RF49 Generated TypeScript for `changed(...)` names an unemitted `Person_Property` type
- **Severity:** Low
- **Evidence:** Found while fixing RF9: a function returning `changed(...)` (type `Person.Property*`) emits a
  return type naming `Person_Property`, which is never emitted into the module, so `tsc --strict` fails
  ("Cannot find name 'Person_Property'"). Likely pre-existing (derived-declaration emission), not an
  occurrence rule. PLAUSIBLE as pre-existing; not checked against `HEAD`.
- **Recommendation:** Emit derived `T.Property` unions referenced by a signature, or spell the type inline.
- **Status:** Not fixed: outside the occurrence change's scope; needs a decision on derived-declaration emission.
- **Fix:** Codegen emits only the derived declarations a program references, and it found references in syntax alone: constructions, member paths and written type annotations. An unannotated function or value is emitted at its inferred type, so `let edited(p:Person.Update) = { changed(p) }` printed `Person_Property` without emitting it. The same happened to `diff(a, b)` with `Person_Update`. `referenced_derived_declarations` (`builder.rs`) now also collects the names in each function's and value's checker type (`type_names`). The declaration-emission rule is unchanged. Test `an_inferred_type_naming_a_derived_declaration_emits_it` (`nx-codegen` tests) runs `tsc` on the output and fails without the fix.
- **Verification:** Verified: `edited(p) = { changed(p) }` and `delta(a, b) = { diff(a, b) }` with inferred types emit `Person_Property` and `Person_Update`, and the module passes `tsc --strict`.

## New Findings Discovered During 2026-09-22 03:40 Verification

### ✅ Verified - RF50 A real presence test followed by a malformed consequent is still reported as a ternary
- **Severity:** Low
- **Evidence:** `let f(a?:int): boolean = { a? ) : 3 }` suggests ``if a { ) } else { 3 }``, and
  `{ a? + ) : 3 }` suggests `if a { + ) } else { 3 }`, hiding the real parse error. The input must
  already be malformed.
- **Recommendation:** Skip the filled-in fix-it when an operand contains an unmatched closer or starts
  with a binary operator; fall back to the real parse error.
- **Status:** Not fixed: this is the third round on the same fix-it heuristic (RF18, RF43); rather than
  keep adding special cases, decide whether the heuristic should instead require a well-formed
  consequent node from the parser.
- **Fix:** The fix-it is now checked instead of guessed: `rewrite_parses` (`validation.rs`) splices the suggested `if` into the source where the ternary stood, reparses with the raw parser, and offers the filled-in form only when no error or missing node touches the spliced range; otherwise the note names the generic `if condition { a } else { b }` form. Both RF50 inputs, and the RF18 cut-short `c ? 1 : n +` case, now get the generic note, and `ends_in_binary_operator` is removed. The nested-conditional checks stay, since they catch rewrites that parse but do not mean what the author wrote. The reparse also exposed that the unbraced `let ratio = ready ? 1 : 2` suggested `let ratio = if ready { 1 } else { 2 }`, which does not parse; when the bare `if` does not parse but `{ if … }` does, the braced form is suggested. The parser test `test_conditional_operator_rewrite_that_does_not_parse_gets_no_filled_in_fix_it` covers the three inputs, and the presence-operators delta states the rule, updates the `let ratio` scenario and adds "A rewrite that would not parse is not suggested". The ternary diagnostic still covers the malformed operand's own parse error, as for any ternary; that error surfaces once the author writes the `if`.
- **Verification:** Verified: both RF50 inputs and the cut-short `c ? 1 : n +` case get the generic note, and `let ratio = ready ? 1 : 2` suggests the braced form. The reparse check does not catch a rewrite that parses but changes meaning; that case is RF53.

### ✅ Verified - RF51 Generated TypeScript gaps outside the occurrence rules
- **Severity:** Low
- **Evidence:** Seen while verifying RF9: a sequence alias referenced in a signature (`type Ints = int+`)
  is never emitted, so `tsc` reports "Cannot find name 'Ints'"; a derived record passed to a parameter
  typed by its abstract base fails (`"Derived"` is not assignable to `"Base"`), since the abstract
  record's `$type` is the literal `"Base"`. Both look pre-existing (PLAUSIBLE; the second may be more
  visible now that TypeScript output writes `$type: "X" as const`).
- **Recommendation:** Emit sequence aliases referenced by signatures; type an abstract record's `$type`
  as the union of its concrete descendants.
- **Status:** Not fixed: outside this change's scope; needs confirmation against `HEAD`.
- **Fix (aliases):** Aliases. Every alias was affected, not only sequence aliases: `type Name = string` and `type P = Person` used in a signature or field also failed `tsc`. The TypeScript target printed an alias by name but never declared it. `CodegenDeclarationKind::TypeAlias` now carries its target, and TypeScript output declares `export type Ints = readonly number[];` and so on. The target's references are collected for imports and its optionality for `NxEmpty`, so an alias imported from another module works. JavaScript and the IR are unchanged. Test `a_type_alias_named_by_a_signature_or_field_is_declared` covers same-module and imported aliases, runs `tsc`, and checks the declaration.
- **Fix (abstract base):** Record inheritance is modelled with interfaces, after discussion. A union of the descendants (`type Shape = Circle | Square`) was considered and set aside: it closes a base that NX leaves open, makes the base's module import its descendants, and is only as complete as the program. Instead:
  - An abstract record is `interface Shape { readonly $type: string; ... }`.
  - A record or union case extending it is `interface Circle extends Shape { readonly $type: "Circle"; ... }`, importing the base as a type when it lives in another module.
  - A plain literal of such a record is pinned to its own type, `({ ... } satisfies Circle as Circle)`, so TypeScript's excess-property check on a fresh literal does not refuse it at a `Shape` site. A wrong field is still caught against `Circle`.
  - A case of another module's union is pinned through the imported union, `Extract<m1_Badge, { readonly $type: "Badge.icon" }>`.
  - Literals built by the default-filling IIFE are not fresh and need no pin. JavaScript output is unchanged, and records outside a hierarchy keep their `type` form.
  - Tests `a_record_extending_an_abstract_base_is_accepted_where_the_base_is_expected` and `a_record_extending_a_base_from_another_module_is_accepted_there` run `tsc --strict`.
  - The `executable-code-generation` delta adds "Generated TypeScript declares every type its signatures name" and "Generated TypeScript models record inheritance with interfaces".
- **Fix (typegen alignment):** `nxlang typegen --language typescript` now uses the same model, so an abstract type is open in every generated surface, as in NX and in generated C#. A closed set of records is an NX union, which every target already renders closed.
  - Before, typegen emitted an open `interface ShapeBase` and a closed `type Shape = Circle | Square` over the library's descendants. Now `interface Shape extends NxRecord` declares the fields with `$type: string`, and each record, action or union case extending it is `interface Circle extends Shape { $type: "Circle"; ... }`. No descendant union is generated, so a base's module no longer imports its descendants.
  - A base from a dependency library, previously dropped from the `extends` clause, is now extended and imported.
  - Tests: nine typegen tests are updated to the open shape, and the abstract-family tests are renamed `generates_typescript_abstract_{records,actions}_as_open_contracts`. New `generated_typescript_abstract_family_type_checks_in_host_code` runs `tsc --strict` on the output with host code that passes a pinned literal, a `Circle` and a `Badge` where a `Shape` is expected, and narrows `Badge` by `$type`.
  - The `cli-code-generation` delta rewrites "TypeScript generated records preserve concrete runtime discriminators" and its four abstract-family scenario bodies. The types reference (`types.md`, Record Types) now states that abstract records are open and unions closed, in NX and in generated host types.
- **Verification:** Verified: a workspace with same-module and imported aliases (`Name`, `Ints`, `P`, `Tags`), records extending an imported abstract base, and a literal of another module's union case passes `tsc --strict`. Typegen emits `interface Shape extends NxRecord` with open descendants (including an abstract middle level). Host code passing a `Circle`, a `Cube` and a `Badge` as `Shape` type-checks, and a wrong `$type` literal is still rejected.

## New Findings Discovered During 2026-09-23 21:14 Verification

### ✅ Resolved - RF52 A function whose extra parameters are all omissible does not satisfy a function type that leaves them out
- **Severity:** Low
- **Evidence:** Surfaced by RF29. `let f(a:int, b:int = 2) = { a + b }` and
  `let use(h:<function a:int />: int) = <h a=5 />`: `use({f})` is rejected ("the function declares
  parameter 'b', which the function type does not supply"), and so is `f(a:int, b?:int)`. The
  function-types delta requires this ("A function that declares a parameter the type does not SHALL
  NOT be compatible"). That rule's reason, that a caller could not supply the parameter, no longer
  holds for an omissible one: under function-parameters the function fills it, and every engine's
  call path already binds a left-out parameter (`bind_function_params`, IR default nodes, JS default
  parameters). A template such as `<Row Item:T showIcon:boolean = true />` therefore cannot be passed
  as `<function Item:T />`.
- **Recommendation:** Needs a decision. Either let a function satisfy a type that omits only its
  optional or defaulted parameters, and amend the function-types requirement with a scenario for each,
  or keep the rule and state in function-types why an omissible parameter still counts.
- **Status:** Resolved by keeping the rule, after discussion. Parameters match by name and a
  function may ignore a parameter the type supplies, so accepting an omissible parameter the type
  does not supply would let a misspelled name on either side compile silently (`Idx:int = 0`
  against a type supplying `Index` always reads 0). Staying strict can be relaxed later without
  breaking anyone; the reverse cannot. The `function-types` delta now states the reason and adds
  "A function with an extra omissible parameter is rejected", pinned by
  `a_function_with_an_extra_omissible_parameter_is_rejected`. `specs/future.md` (Function
  parameters) records when to relax it and how: accept the parameter, with a near-miss check that
  reports "did you mean `Index`?".

### ✅ Verified - RF53 The ternary fix-it pulls later items on the same line into the `else` branch
- **Severity:** Low
- **Evidence:** `let f(c:boolean): string+ = { c ? "a" : "b" "z" }` suggests
  ``if c { "a" } else { "b" "z" }``. That rewrite parses, so RF50's reparse check accepts it, but it
  changes the meaning: `"z"` becomes part of the `else` branch instead of following the conditional
  (`{ if c { "a" } else { "b" } "z" }`). Likewise `{ c ? 1 : 2 undefinedAfter }` suggests
  `else { 2 undefinedAfter }`. The alternative still runs to the closing brace when nothing on the
  line stops it.
- **Recommendation:** End the alternative at its first complete sequence item and keep the rest after
  the `if` in the suggested text, or fall back to the generic note when the alternative spans more
  than one item. Add the `"b" "z"` case to `test_conditional_operator_*`.
- **Fix:** `validate_removed_conditional_operator` (`nx-syntax/src/validation.rs`) now ends the
  alternative at its first sequence item: new `first_item_len` parses the alternative inside braces
  and, when it is a list, keeps only the first `value_list_item_expression`. The rest stays after the
  `if`, and the label stops where the rewrite does. `{ c ? "a" : "b" "z" }` now suggests
  `if c { "a" } else { "b" }` and underlines `? "a" : "b"`. An alternative that does not parse on its
  own (a nested conditional, a cut-off `n +`) is left whole, so the existing fallbacks still apply.
  `test_conditional_operator_is_rejected_with_the_if_form` gains the `"b" "z"` and
  `2 undefinedAfter` cases.
- **Verification:** Verified: `{ c ? "a" : "b" "z" }` and `{ c ? 1 : 2 undefinedAfter }` suggest
  `else { "b" }` / `else { 2 }` with the label ending at the alternative, as do `2 3 4`, `g(2) 3` and
  `2 - 3`; a single-item alternative is unchanged. Alternatives whose items are not a valid sequence on
  their own (`n + 1 7`, `-1 2`) and ones followed by `for`, a braced value or an `if` fall back to the
  generic note, never to a meaning-changing rewrite. `nx-syntax` tests and the `presence_operators`
  checker tests pass. The label span itself is not asserted by a test (nit).

## New Findings Discovered During 2026-09-25 Documentation Review

### ✅ Verified - RF54 A qualified pattern naming a fieldless union case never matches in the interpreter
- **Severity:** High
- **Evidence:** Found while checking the language-tour examples; it predates this change (`HEAD`
  c197f4e behaves the same). `eval_match_pattern` (`crates/nx-interpreter/src/interpreter.rs`)
  turned every qualified pattern that names a union case into a `Value::Record` of that case, but a
  fieldless case evaluates to `Value::UnionCase`, so the two never compared equal.
  `if s is { DealStage.draft => … DealStage.pending_review => … else => … }` took `else` for every
  stage; without an `else` the match was empty, and `LoadState.idle =>` in a mixed union never
  matched either. Bare patterns (`draft =>`) and payload cases (`LoadState.failed =>`) were fine,
  and `test_union_match_compares_case_discriminator` covered only the payload arm. The tour's
  `StageBadge` and `label` examples and `types.md`'s `badgeTone`/`loadLabel` all hit it.
- **Recommendation:** Build the record pattern only for a case with fields, and let a fieldless case
  match by equality like any other value pattern.
- **Fix:** `eval_match_pattern` builds the record only when the resolved case is not fieldless, so
  a fieldless case falls through to `eval_expr` and matches its `Value::UnionCase` by equality. The
  helper also serves property-list matches. Tests:
  `test_union_match_selects_a_qualified_constant_case` (`simple_functions.rs`), and the
  `expressions` IR corpus program gains `stageLabel`/`loadLabel` with the `reviewStage`, `idleLoad`
  and `failedLoad` entrypoints.
- **Verification:** Verified in the interpreter: qualified fieldless patterns select their arm in an
  all-constant union and in a mixed union, with or without `else`; a multi-pattern arm
  (`Load.idle, Load.failed =>`) matches either case; and a property-list match
  (`<span if l is { Load.idle => … Load.failed => … else => … } />`) picks the right entries. The IR
  runtime agrees on every shape it can emit, and the `nx-interpreter` suites and the TS runtime
  corpus pass. Only single-module programs were exercised; `nxlang run` cannot load an import.
  A bare pattern naming a payload case is a separate failure, tracked as RF56.

### ✅ Verified - RF55 The IR runtime fails on a pattern naming a union case with fields
- **Severity:** High
- **Evidence:** Surfaced by RF54's corpus entrypoints. The TypeScript IR runtime's `takenBranch`
  evaluated each pattern with `evalNode`, so `Load.failed =>` built a `Load.failed` record with no
  properties and failed with "Missing required Load.failed field 'message'". The interpreter never
  had this problem because it special-cases the pattern. Generated JS refuses match expressions, so
  it is not affected.
- **Recommendation:** Evaluate a payload-case pattern as its `$type` alone, which `patternMatches`
  already compares by.
- **Fix:** New `evalPattern` (`runtime/typescript/src/index.ts`) returns `{ $type }` for a
  `unionCase` node whose case has fields and evaluates every other pattern as before. The
  `expressions` corpus `failedLoad` entrypoint covers it (`npm test` in `runtime/typescript`).
- **Verification:** Verified: `evalPattern` builds the same `${union}.${case}` path that
  `evalUnionCase` stamps on a value, so the two compare. It handles a qualified payload pattern
  (`Load.failed =>`), a bare one (`failed =>`, which the builder also emits as a `unionCase` node),
  and a multi-pattern arm. A constant case still evaluates to its name and matches by equality. The
  `npm test` suite passes (461 ok), including the `expressions` corpus in both variants.

## New Findings Discovered During 2026-09-25 12:07 Verification

### ✅ Verified - RF56 A bare pattern naming a payload case fails at runtime in the interpreter
- **Severity:** High
- **Evidence:** `type Load = | idle | loading | failed { message:string }` with
  `let f(l:Load) = { if l is { idle => "idle"  failed => "f"  else => "z" } }`: `f(<Load.failed
  message="q" />)` type-checks, then the interpreter fails with "Type mismatch in union case
  construction: expected union case field 'Load.failed.message', got missing". The IR runtime
  returns `"f"`. Type checking rewrites a bare case name to `Expr::ResolvedUnionCase`
  (`apply_contextual_name_resolutions`, `nx-hir/src/components.rs`). `eval_match_pattern`
  (`crates/nx-interpreter/src/interpreter.rs` ~2815) recognizes a case only through
  `flattened_expr_name`, which handles `Ident`/`Member`, so the resolved case goes to `eval_expr` and
  is constructed with no fields. A bare fieldless case (`idle =>`) works because it evaluates to the
  constant. The RF54 fix does not touch this path, so the failure is probably pre-existing (not
  checked against `HEAD`). RF54's evidence says bare patterns were fine, but it tested only
  fieldless ones.
- **Recommendation:** In `eval_match_pattern`, treat an `Expr::ResolvedUnionCase` whose case has fields
  as a record pattern of that case's type, as the qualified form now is. Add a bare payload arm to
  `test_union_match_selects_a_qualified_constant_case` or a sibling test, and a bare-pattern
  entrypoint to the `expressions` corpus so the IR runtime stays covered.
- **Fix:** `eval_match_pattern` now checks for an `Expr::ResolvedUnionCase` first: when the resolved
  case has fields (new `resolved_union_case_has_fields`, which looks the case up by the module
  identity and definition id the checker recorded), the pattern is a record of `Union.case` and
  matches by type; a fieldless resolved case still evaluates to its constant. The qualified path is
  unchanged.
  Tests: `a_bare_pattern_naming_a_payload_case_matches_its_records` (`occurrence_runtime.rs`, which
  runs the checker so the name is resolved; it fails without the fix), and the `expressions` corpus
  gains `bareLabel` with the `bareFailed` entrypoint, recorded as `"Offline"` and matched by the TS
  runtime. A property-list match (`<span if l is { failed => a="f" … } />`) also picks the payload
  arm. `cargo test --workspace` (79 suites) and `npm test` in `runtime/typescript` pass.
- **Verification:** Verified. In the interpreter, bare payload arms select correctly:
  - alone, and beside `else` or other bare arms;
  - several in one arm (`failed, retry =>`);
  - with a narrowed field read after them (`failed => l.message`);
  - in a property-list match.

  The IR runtime gives the same results. `resolved_union_case_has_fields` finds the case the same way
  `eval_resolved_union_case` does, and the record pattern gets the same `Union.case` name that
  construction stamps. A two-module workspace (`nx-api`, scratch test since removed) also passes: a
  bare `failed =>` over a value built in the importing module or returned from the declaring one
  works with a plain import and with `import … as S`. A `HEAD` build fails every one of those bare
  payload cases, so the bug predates this change and is now fixed. The same check found a separate
  aliased-import failure in the interpreter, tracked as RF57.

## New Findings Discovered During 2026-09-25 12:25 Verification

### ✅ Resolved - RF57 The interpreter compares union types by spelling, so an aliased import breaks calls and qualified patterns
- **Severity:** Medium
- **Evidence:** `shapes.nx` declares `export type Load = | idle | loading | failed { message:string }` and
  `export let make(): Load = { <Load.failed message="lib" /> }`. `main.nx` has `import "./shapes.nx" as S`
  and `let f(l:S.Load) = { if l is { idle => "idle" failed => l.message else => "z" } }`. The program
  type-checks. In the interpreter:
  - `f(S.make())` fails: "Type mismatch in function call parameter 'l': expected S.Load, got
    Load.failed".
  - `f(S.Load.idle)` fails the same way ("got Load.idle"). The case value carries the declaring name
    `Load`, while the parameter type is spelled `S.Load`.
  - `if S.make() is { S.Load.failed => "qual" else => "z" }` gives `"z"`. The qualified pattern
    flattens to the record name `S.Load.failed`, and the value is stamped `Load.failed`.

  The IR runtime gives `"lib"`, `"idle"` and `"qual"`. A `HEAD` build (c197f4e) fails in exactly the
  same ways, so this predates the change and is not an occurrence rule. It is recorded because it
  breaks the three-engine agreement this change relies on, and an aliased import is a documented
  form (`import "./core" as CoreHtml`).
- **Recommendation:** Compare union (and record) identity by the declaration rather than by the name
  as spelled at the use site. For example, resolve a parameter's type and a qualified pattern to the
  declaring module and local name before comparing, as `Expr::ResolvedUnionCase` already does for a
  bare case name. Add an aliased-import case to a workspace test and to a multi-module IR corpus
  program so both engines stay pinned. If this is out of scope here, defer it to `specs/future.md`
  and mark it resolved.
- **Status:** Deferred to `specs/future.md`, as the new subsection "Aliased imports: the interpreter
  looks a value's type up in the wrong module" under "Nominal Value Identity". A scratch `nx-api`
  workspace test (since removed) showed that the failure is wider than unions. Under
  `import "../lib" as S`, `n(S.ada())` at a `u:S.User` parameter fails ("expected S.User, got
  User"), and `n(<S.User name="Bo" />)`, built in the importing module, fails with "Record type not
  found: User". A bare payload pattern under the alias and every plain-import case work. Fixing it
  means resolving type names to their declarations at every place the interpreter turns a name back
  into a declaration: coercion, record and union type matching, qualified patterns and record
  construction. That is a change of its own, not a fix to the occurrence rules this change is about.

## Questions
- None.

## Summary
- Three review-fix-verify cycles ran. 54 findings are verified fixed: the 2026-09-23 verification
  confirmed RF6, RF12, RF13, RF17, RF21, RF28, RF29, RF37, RF38, RF39, RF44, RF49, RF50 and RF51
  across the interpreter, the IR runtime and generated JavaScript, and with `cargo test
  --workspace`, the TypeScript runtime, Node, wasm and .NET suites, and `openspec validate --strict`
  all passing. RF48 is resolved by deferral to `specs/future.md`. Two new Low findings came from
  that verification: RF53 (the ternary fix-it pulls later same-line items into the `else` branch)
  is fixed and verified on 2026-09-25, and RF52 (whether a function whose extra parameters are all
  omissible satisfies a function type that leaves them out) is resolved by keeping the stricter
  rule on purpose, with relaxing it recorded in `specs/future.md`.
- A 2026-09-25 documentation review found two High bugs in union-case match patterns, both
  older than this change: RF54 (a qualified fieldless case never matched in the interpreter) and
  RF55 (the IR runtime could not evaluate a payload-case pattern). Both are fixed and were
  verified on 2026-09-25. That verification found RF56: a bare pattern naming a payload case
  (`failed =>`) failed in the interpreter, while the IR runtime handled it. RF56 is fixed and
  was verified on 2026-09-25, including across modules. That verification found RF57, which
  predates this change: under `import … as S`, the interpreter rejects a call whose parameter is
  typed `S.Load` or `S.User`, and never matches a qualified `S.Load.failed` pattern, while the IR
  runtime handles them. RF57 is resolved by deferral to `specs/future.md`.
