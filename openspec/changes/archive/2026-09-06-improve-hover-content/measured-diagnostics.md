# Measured diagnostic delta — task 1.3

The diagnostic delta produced by task 1.2's change to `infer_element_expression`'s unresolved-tag
fallthrough (`crates/nx-types/src/infer.rs`), measured across the repo's NX corpus before and after.

## Method

A temporary harness (`crates/nx-api/examples/measure_diagnostics.rs`, deleted after measuring) ran
the analysis twice over the same corpus, with the change stashed and unstashed, and the sorted
diagnostic lists were diffed. The corpus is analyzed the way each part of it is really compiled:

| Corpus | How it is analyzed | Files |
| --- | --- | --- |
| `examples/nx/` | `NxWorkspace::from_directory` + `analyze_workspace_modules` | 11 |
| DrawnUI catalog | `analyze_str` on `catalog/skia.nx` alone | 1 |
| DrawnUI examples | `analyze_str` on catalog-then-source in one module, as `server/compile.mjs` does | 12 |

## Result

| Corpus | Before | After | New |
| --- | ---: | ---: | ---: |
| `examples/nx/` | 24 | 32 | 8 |
| DrawnUI catalog | 0 | 0 | 0 |
| DrawnUI examples | 0 | 0 | 0 |

Nothing was newly reported anywhere in `sample-apps/`. The DrawnUI corpus nests as deeply as
anything in the repo, but almost every tag in it resolves — the catalog declares them — so the
unresolved fallthrough is barely reached there. The whole delta is in `examples/nx/complex.nx`.

## The eight new diagnostics

All eight are `not-implemented`, all in `examples/nx/complex.nx`, all reported at a member access
written inside an intrinsic element:

| Count | Diagnostic | Written at | Base type |
| ---: | --- | --- | --- |
| 4 | `Member access not yet implemented: .length` | `complex.nx:32,33,34` (×2) | `TodoItem[]`, inside `<div class="stats">` |
| 2 | `Member access not yet implemented: .completed` | `complex.nx:69,70` | `TodoItem`, inside `<li>` |
| 1 | `Member access not yet implemented: .status` | `complex.nx:50` | `TodoItem`, inside `<ul>` |
| 1 | `Member access not yet implemented: .text` | `complex.nx:71` | `TodoItem`, inside `<li>` |

## Classification

**All eight are source defects the checker was hiding. None is a checker defect the recursion
exposed.**

`infer_member_access` (`crates/nx-types/src/infer.rs:1060`) reaches a field on a union, a union
case, or a record, and reports `not-implemented` for every other base. `complex.nx:5` declares
`type TodoItem = object`, an alias with no fields, and then reads `.completed`, `.status` and
`.text` off it; `.length` is read off a list, and NX defines no member on a list — there is no `length` in
`nx-types`, `nx-interpreter`, or `nx-value`, so this is not a member the checker forgot to
implement but one the language does not have.

The same two expressions written outside any element report the same two diagnostics, verbatim:

```
type T = object
let count(items:T[]): int = { items.length }   → Member access not yet implemented: .length
let read(item:T): boolean = { item.completed } → Member access not yet implemented: .completed
```

So inference inside markup is not weaker than inference outside it — these expressions were simply
never visited. **The stop-and-report gate in task 1.4 does not trigger.**

## One more, found by the test suite

The corpus scan covers `.nx` files on disk. `cargo test --workspace` surfaced a ninth newly reported
diagnostic, in an inline test fixture:

- `crates/nx-api/src/artifacts.rs` — `export let <Draw s: Shape = {<Circle r=0 />} /> = <div r={s.r} />`
  now reports `Record 'Shape' has no field 'r'; it has: label`. `s` is declared as the abstract base
  `Shape`, which declares only `label`; `r` belongs to `Circle`. This is the same classification —
  a source defect the checker was hiding — and is fixed in place by reading `s.label`, which is the
  field the declared type actually has. The fixture appears twice, in
  `a_foreign_record_satisfies_a_base_the_consumer_cannot_name` and in
  `a_local_lineage_spelled_like_a_foreign_one_does_not_satisfy_it`; both are corrected, the second
  because a negative test that passes on the wrong diagnostic is not testing what it names.

`cargo test --workspace --no-fail-fast` is otherwise green: 51 test binaries pass, and the two
failures in `nx-codegen` (`generated_component_typescript_type_checks_when_tsc_is_available`,
`generated_record_schema_defaults_can_reference_previous_fields`) reproduce with this change stashed
— they are pre-existing TypeScript typing errors in the generated `nx-runtime.ts`, unrelated to
this work.

## What was fixed, and what was left

| Diagnostic | Count | Outcome |
| --- | ---: | --- |
| `.completed`, `.status`, `.text` on `TodoItem` | 4 | **Fixed.** `type TodoItem = object` became a record declaring `text`, `status`, and `completed`. Making it a record then collided with the element-style function `let <TodoItem item:Todo … />`, because a record resolves as an element and a type alias does not — so the data type was renamed to `Todo`, leaving the component set (`<TodoApp>`, `<TodoStats>`, `<TodoList>`, `<TodoItem>`, `<TodoFilters>`) as the example wrote it. |
| `.length` on `Todo[]` | 4 | **Left in place.** NX has no list length member to write instead, so this is a request for a language feature, not a defect the author can repair. Fixing it would mean restructuring `<TodoStats>` to take a count and changing what the example demonstrates. |
| `Record 'Shape' has no field 'r'` (nx-api fixture) | 1 | **Fixed**, in both copies of the fixture. |

`complex.nx` therefore still reports 4 `not-implemented` diagnostics plus its 6 pre-existing errors.
Task 2.1's "verify the examples check clean" is not achievable and was not achievable before this
change; see the baseline below.

## A consequence beyond diagnostics: the emitted IR

The type environment is not read only by hover. `build_expression`
(`crates/nx-codegen/src/builder.rs:837`) attaches `type_env.get_expr_type(expr_id)` to every
expression it emits, so the recorded types are written into `.nxir.json` as each expression's `ty`.
Filling the hole therefore changes generated IR: every expression inside an element whose tag
resolves to nothing goes from `"ty": null` to its real type.

That is a behavior fix, not only a metadata one. `@nx-lang/ir-runtime` reads `expression.ty` to pick
integer division from float division (`runtime/typescript/src/index.ts:1381`, `isIntegerSemanticType`
at `:1535`), so an untyped expression divided as a float. Measured through the runtime, on the same
IR built with and without the change:

| Source | Before | After |
| --- | --- | --- |
| `let root() = { 7 / 2 }` | `3` | `3` |
| `let root() = <div>{7 / 2}</div>` | **`3.5`** | `3` |

Markup content was silently switching `int` division to float division in the JavaScript runtime.
The Rust interpreter was never affected — it decides integer division from the operand values rather
than from the IR — which is why the two runtimes disagreed and neither test suite caught it.

Pinned by `nx_ir_types_an_expression_written_inside_an_intrinsic_element` in
`crates/nx-codegen/src/tests.rs`, which fails with the inference change stashed.

The `source-analysis-pipeline` delta already requires this: "The recorded type of every such
expression SHALL be available to callers through the analysis result's type environment, on the same
terms as an expression written outside any element." Codegen is such a caller.

## Baseline caveat

The 24 pre-existing diagnostics in `examples/nx/` are unchanged by this work and are not this
change's to fix. They are worth recording because they bear on task 2.1's "verify the examples check
clean":

- `complex.nx` has 5 syntax errors, a tag mismatch, and 2 undefined identifiers, all pre-existing.
  It is not a file that compiles today.
- `utils/formatting.nx` has a syntax error and a contextual-name error, pre-existing.
- The three `Missing workspace module or loaded library` errors come from directory imports
  (`import "./core"`) resolving to a name the workspace has no module for. That may be a harness
  artifact of an empty `ProgramBuildContext` rather than a source defect; either way it predates
  this change.
- `examples/nx/template-candidate.nx` is an untracked working file, not part of the tracked corpus.
  Its 10 diagnostics are listed in the baseline for completeness and are excluded from scope.
