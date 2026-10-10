## 1. Interpreter and annotated text

- [ ] 1.1 Record the constructing element's module and range on `Value::Record` in `nx-interpreter`, keeping it across binding, props, state, sequences and returns, and setting it to the applying expression for update application, `merge` and `apply`; verify with interpreter tests for each scenario of `A record's origin is the element expression that constructed it`
- [ ] 1.2 Write `origin` on record nodes in `crates/nx-api/src/nx_text.rs` as module identity and `NxTextSpan`; verify with the sdk-wasm scenario `Two records of one type have different origins` and that `format_nx_text` output is unchanged
- [ ] 1.3 Add `origin` to `NxValueNode` in `bindings/wasm/src/types.ts`; verify the parity tests still pass with unchanged text

## 2. TypeScript runtime

- [ ] 2.1 Add `origins?: boolean` to `NxRuntimeOptions`, keep each record's constructing node in a `WeakMap` during evaluation, and collect `{ path, module, start, end }` entries for the returned value when the option is set; verify with the scenarios of `TypeScript runtime reports the origins of the records it returns`
- [ ] 2.2 Check that results without the option are identical to before by running the conformance corpus both ways and comparing serialized results; verify no difference
- [ ] 2.3 Measure the option's cost with the performance harness on the question-flow lifecycle; record it in the README

## 3. Value view

- [ ] 3.1 Pass `origin` through on the `nx-value-navigate` event detail beside `declaration`, and document it; verify with a value-view test

## 4. Specs

- [ ] 4.1 Run `openspec validate add-value-origin --strict`; verify it passes
