## 1. Derived declarations in HIR

- [x] 1.1 Add a `STATE_RECORD_SUFFIX` (`State`) beside `UPDATE_RECORD_SUFFIX` and `PROPERTY_UNION_SUFFIX` in `crates/nx-hir/src/lib.rs`, with `state_record_name` and `is_state_record_name`, and a record kind or marker that ties `T.State` to its component
- [x] 1.2 In `lower.rs`, synthesize `T.State` for every component with a `state` group (fields in order, types and optional marks kept, defaults dropped), and run the existing update-record and property-union synthesis on it to get `T.State.Update` and `T.State.Property`
- [x] 1.3 Derive a component's `T.Update` and `T.Property` from its effective props (inherited first, type parameter definitions excluded) for every component, stateful or not, external or not
- [x] 1.4 Reject `State` as an inline emit name, a shared emitted action, a prop name and a state field name, beside the existing `Update`/`Property` checks
- [x] 1.5 Reject nested and misplaced derived names: `T.State.State`, `T.Update.State`, `T.Property.State`, and `R.State` for anything that isn't a component with state
- [x] 1.6 Replace bare-name rewriting: remove `resolve_bare_update_tag`, `resolve_bare_property_base`, `resolve_bare_property_type` and the scope suppression for a bare `Property`; add a bare `State` rewrite for type references, element and record-literal tags, and member-access heads inside a component body
- [x] 1.7 Carry derived `State` declarations through imports and `DERIVED_DECLARATION_SUFFIXES` in `crates/nx-api/src/artifacts.rs`

## 2. Type checking

- [x] 2.1 Route handler results in `check_handler_result_item`: a component's `T.State.Update` patches state; its own `T.Update` gets a new diagnostic saying props belong to the parent and suggesting `State.Update`; everything else as before
- [x] 2.2 Replace `bare-update-outside-component` and `bare-property-outside-component` with unknown-name diagnostics that suggest `State.Update`/`State.Property` inside a stateful component and the qualified form elsewhere; add a "declares no state" diagnostic for `State` and `T.State`
- [x] 2.3 Make the update intrinsics accept `T.State` as a record and a component's `T.Update` as an update (`merge`, `changed`), and keep rejecting a component element as the record of `apply` or `diff`
- [x] 2.4 Update `crates/nx-types/tests` (`update_records.rs`, `property_references.rs`, `update_intrinsics.rs`, `component_body_checking.rs`, the type-parameter tests) to the new derivations, and add tests for every scenario in `component-state-type`

## 3. NX IR and codegen

- [x] 3.1 Emit `T.State` whenever its update or property union is emitted, and add the trailing state-record reference to the component layout in `crates/nx-codegen` (`builder.rs`, `model.rs`, `ir.rs`)
- [x] 3.2 Point a component's props patch `T.Update` at the component declaration, and a state patch at `T.State`
- [x] 3.3 Move the IR schema version from 5 to 6 and update `docs/nx-ir-format.md`: the component layout, the dispatch text (`<Owner>.State.Update` patches state), and the examples
- [x] 3.4 Update the JS executable emitter (`emit.rs`) so generated state types and `nxApplyUpdate` use the state record
- [x] 3.5 Update `ir_tests.rs`, `ir_runtime_tests.rs`, `ir_instance_tree_tests.rs`, `tests.rs`, the cost probes, and the prelude image tests

## 4. Runtimes

- [x] 4.1 Rust runtime: read the state-record reference, and make `is_update_record_for` compare an update's target with the owner component's state record (`component.rs`, `module.rs`, `tree.rs`)
- [x] 4.2 TypeScript runtime: the same in `runtime/typescript/src/index.ts`, including `isUpdateRecordFor` and the state-patch operation, which takes `<Component>.State.Update` and rejects `<Component>.Update`; rebuild the committed `dist`
- [x] 4.3 HIR interpreter: change only what its existing tests need to keep passing
- [x] 4.4 Move the 10 IR conformance programs under `specs/ir-conformance/` to the new spellings and expected `$type`s, and run the corpus in both runtimes

## 5. Bindings and typegen

- [x] 5.1 `typegen`: emit `<Name>_update`/`<Name>_property` from props for every exported component, `<Name>_state` for every exported component with state, and `<Name>_state_update`/`<Name>_state_property`; extend the collision warnings to the new names
- [x] 5.2 Regenerate the .NET generated fixtures (`update-records.nx`, `prelude-companions.nx`, `UpdateRecords.g.cs`) and update the .NET tests that use component patches
- [x] 5.3 Update the wasm, node and schema fixtures (`aliases.nx`) and their tests

## 6. Sources, docs and tools

- [x] 6.1 Move the examples (`examples/nx/component.nx`, `generic-records.nx`) and the playground's `counter.nx` to `State.Update`/`State.Property`, and check the playground preview still runs the counter
- [x] 6.2 Update the website pages (reference `functions.md`, `types.md`, `elements.md`; tour `functions.md`, `types.md`; the first-component tutorial; `declaration-schemas.md`), `nx-grammar.md`, `nx-grammar-spec.md`, and the bindings and agent READMEs
- [x] 6.3 Update any viewer, source-tree or language-service tests that spell `<Update …/>`, and note in the PR that the DrawnUI fiddle's snippets, which live in the fiddle repository, need the same spelling change
- [x] 6.4 Run the full Rust, TypeScript and .NET test suites and `cargo fmt`/`clippy`, and validate the change with `openspec validate add-component-state-type --strict`
