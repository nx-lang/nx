## 1. Compiler and IR format

- [ ] 1.1 Add `NX_IR_REQUIRED_FEATURE_ENVIRONMENT_V3` (`environment-v3`) to `crates/nx-ir/src/model.rs`
  and export it, and give `actionHandler` an optional trailing flag cell in the layouts of
  `crates/nx-ir/src/image.rs` (`Op::Trailing` of an int), refusing a flag cell other than `1`.
- [ ] 1.2 Add an environment flag to `CodegenActionHandler` and set it in `formulas.rs`: a pass
  beside `mark_formulas` that uses `reading_declarations` and `reads_environment` on the handler body
  (its own `provide` blocks and their bodies included).
- [ ] 1.3 Emit the trailing cell for a flagged handler in `crates/nx-codegen/src/ir.rs`, and list
  `environment-v3` exactly when a module holds one. Leave unflagged handlers byte for byte as
  before.
- [ ] 1.4 Mark a flagged handler in the `explain` text (`crates/nx-ir/src/explain.rs`).
- [ ] 1.5 Emitter tests: a handler reading `env.` directly, through a function, through a component
  element whose prop default reads, and through a call of a function-typed value is flagged; a
  handler inside a provide that reads nothing is not; the feature is listed exactly when flagged;
  the explain text.

## 2. Rust IR runtime

- [ ] 2.1 `module.rs`: read the flag into `HandlerNode`. Refuse a flagged node in a module that does
  not list `environment-v3` (beside `check_environment_v2_entries`), and accept the feature.
- [ ] 2.2 `environment.rs`: build the `$env` entries of an `envProvide` before its body, and keep the
  current list of the call's own provide entries in `EnvState` beside the layers, shared per provide
  depth, pushed and truncated with the layers. Stamping uses the same entries.
- [ ] 2.3 `value.rs` and `eval.rs`: give `Handler` its captured overrides (the shared list, empty
  for an unflagged node) and fill it in `eval_handler` at no charge.
- [ ] 2.4 `component.rs` `invoke_handler`: push the captured overrides above `base` as own layers
  (bundle entries one layer each with the bundle flag, formulas through `formula_from_data`, plain
  values unchecked), evaluate the body, then truncate. Keep live state and `action` as they are.
- [ ] 2.5 `handlers_equal`: after the slots, compare the override lists as sequences for flagged
  nodes, with the cost rules of `values_equal`. Update `kept.rs`, which uses it through
  `same_written`, and the rebinding walk in `component.rs`, so that both cover the list.
- [ ] 2.6 `stored.rs`: write the list as one value index (`environment: Option<u32>`), read it back,
  check each entry's shape, value name and formula, and count it in the depth and size bounds.
- [ ] 2.7 Unit tests for every scenario of the `environment-values`, `component-action-handlers` and
  `rust-ir-runtime` deltas through the bare API:
  - the five places from the proposal's probe table;
  - a captured formula under a changed root value;
  - a function the handler calls;
  - an unflagged handler's cost and equality unchanged;
  - equality across renders;
  - stored round trip, shared list and an altered entry.
  Also check that an emit dispatched by the parent's token is exact, and that one dispatched
  through the child doubles only a self-reading override (Decision 8).

## 3. Instance tree

- [ ] 3.1 Confirm that `tree.rs` needs no new logic: handlers run against their creator with its
  current environment, and captured overrides are not reads.
- [ ] 3.2 Tests in `crates/nx-api/tests/codegen/ir_instance_tree_tests.rs`, one per scenario of the
  `component-instance-tree` delta:
  - content handler;
  - emit with `env.` attributes on the child's element;
  - kept node under a changed place;
  - `set_environment` reaching a captured formula without a render;
  - a provide whose value changes;
  - a kept child holding an unflagged handler.
  Also add a stress case in `ir_instance_tree_stress_tests.rs` that runs the handler programs with
  the skips on and compares against fresh renders.

## 4. TypeScript IR runtime

- [ ] 4.1 `runtime/typescript/src/index.ts`, matching Rust exactly:
  - read the flag and accept `environment-v3`;
  - keep the provide entries beside the layers;
  - add `capturedOverrides` to `ActionHandlerValue` in `evalActionHandler`;
  - push the overrides in `invokeHandler`;
  - compare them in `handlersEqual`;
  - walk them in `resolveParentHandlers`.
- [ ] 4.2 Tests in `runtime/typescript/test/runtime.test.ts` for the same bare-API scenarios and
  costs as task 2.7 (no stored form), then rebuild `runtime/typescript/dist`.

## 5. Conformance corpus

- [ ] 5.1 Add `specs/ir-conformance/environment-handlers`: a `Page` lifecycle whose handlers sit
  in five places:
  - inside a `provide` block;
  - on an element with `env.` attributes;
  - under a formula;
  - in a `Box`'s content placed inside another provide;
  - for a `Card` emit, dispatched by the parent's token.
  Add a handler that reads nothing, inside a provide, so the corpus pins its unchanged cost.
  Register the program in `manifest.json`.
- [ ] 5.2 Generate the images, explained text, results and `operations.json`. Check that both
  runtimes' corpus tests pass with equal counts, and that no other corpus image changed.

## 6. Documentation

- [ ] 6.1 `docs/nx-ir-format.md`:
  - the `actionHandler` row (trailing flag);
  - *Required features* (`environment-v3`);
  - *Action handlers*: what a flagged handler captures and how it runs, the lexical rule, and the
    bare-API note for emits dispatched through the child;
  - *Environment values*: a handler's reads;
  - *Evaluation cost*: the comparison rule.
- [ ] 6.2 `sites/website/src/content/docs/reference/syntax/environment.md`: a short "In an action
  handler" section with an example of a handler inside a `provide` and on an element with `env.`
  attributes, and the rule that a handler sees where it is written, not where it is placed.
- [ ] 6.3 `specs/future.md`, in "Environment Values: What `add-environment-values` Left For Later":
  - remove "Handlers lose local provides";
  - add the per-value narrowing of a handler's captured overrides (Decision 3);
  - add the double application for an emit dispatched through the child in the bare API, with the
    fix of an instance that carries its environment (Decision 8).
- [ ] 6.4 Update the README of the corpus (`specs/ir-conformance/README.md`) to describe
  `environment-handlers`.

## 7. Verification

- [ ] 7.1 `cargo test --workspace`, `pnpm -r test` (the TypeScript runtime and the packages that
  embed it), and `DO_NOT_TRACK=1 openspec validate handlers-keep-local-provides --strict`.
- [ ] 7.2 Run the proposal's probe program in both runtimes and in the tree, and record that each
  row gives its expected value.
