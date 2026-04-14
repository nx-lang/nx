## 1. Syntax And Validation

- [x] 1.1 Update `crates/nx-syntax/grammar.js` and regenerate parser artifacts so component bodies can parse without a rendered expression when used by a state-only external component body.
- [x] 1.2 Update `crates/nx-syntax/src/validation.rs` and related diagnostics to enforce the new body matrix: concrete components still require a render body, abstract components remain bodyless, and concrete external components may be bodyless or state-only but reject empty or rendered bodies.
- [x] 1.3 Add or update `crates/nx-syntax` fixtures and parser tests covering valid state-only external bodies plus invalid empty external bodies and invalid abstract/external rendered-body combinations.

## 2. External Component Semantics

- [x] 2.1 Update `crates/nx-hir` lowering and prepared/interface tests so external components can preserve declared state with `body == None` while keeping state out of effective prop/emits contracts and inheritance.
- [x] 2.2 Update `crates/nx-interpreter/src/interpreter.rs` so external components with declared state skip NX state materialization and continue to initialize and dispatch with empty NX-managed snapshots.
- [x] 2.3 Add or update `crates/nx-hir`, `crates/nx-types`, `crates/nx-interpreter`, and `crates/nx-api` tests proving external state does not become an invocable prop and does not change external evaluation or lifecycle record shape.

## 3. Code Generation

- [x] 3.1 Extend `crates/nx-cli/src/codegen/model.rs` to collect companion state declarations for exported external components with state, assign stable `<ComponentName>_state` names, and warn/skip generated-name collisions.
- [x] 3.2 Update `crates/nx-cli/src/codegen/languages/typescript.rs` and `crates/nx-cli/src/codegen/languages/csharp.rs` to emit plain companion state contracts without `$type` discriminators while preserving cross-module type references.
- [x] 3.3 Add or update `crates/nx-cli` codegen and CLI tests for single-file and library generation of external component state, cross-module state-type imports, omitted non-export externals, and collision failures.

## 4. Verification

- [x] 4.1 Run `openspec validate allow-external-component-state` and targeted Rust test suites for `nx-syntax`, `nx-hir`, `nx-types`, `nx-interpreter`, `nx-api`, and `nx-cli`, then fix any resulting regressions.
