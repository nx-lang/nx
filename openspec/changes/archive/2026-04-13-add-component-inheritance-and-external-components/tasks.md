## 1. Syntax And Parser

- [x] 1.1 Update `crates/nx-syntax/grammar.js` to support `abstract`/`external` component modifiers, an optional `extends` clause in component signatures, and bodyless abstract or external component declarations, then regenerate the grammar artifacts.
- [x] 1.2 Update `crates/nx-syntax/src/validation.rs`, AST accessors, and parser diagnostics so invalid concrete bodyless components, invalid abstract/external bodies, and invalid multi-base component inheritance report the new canonical component syntax.
- [x] 1.3 Add or update `crates/nx-syntax` parser fixtures and tests for valid abstract, external, abstract external, and derived component declarations plus invalid inheritance/body combinations.

## 2. HIR And Interface Model

- [x] 2.1 Extend `crates/nx-hir` component data structures and lowering to preserve `is_abstract`, `is_external`, `base`, and optional body metadata while keeping inline emitted-action lowering intact for component-scoped actions.
- [x] 2.2 Update prepared/interface metadata in `crates/nx-hir/src/prepared.rs` and `crates/nx-api/src/artifacts.rs` so exported/imported components publish the new modifier/base fields needed for cross-library component inheritance.
- [x] 2.3 Update `crates/nx-hir/src/scope.rs` to seed concrete derived component body scopes with inherited props while keeping local state non-inheritable.

## 3. Component Contract Resolution And Static Analysis

- [x] 3.1 Implement an effective component contract resolver in `crates/nx-hir` that validates abstract-only bases, same-library and imported base resolution, inheritance cycles, duplicate inherited props/content props, duplicate emits, and inherited handler-name collisions.
- [x] 3.2 Update `crates/nx-types/src/infer.rs` and related analysis entry points to validate component invocation bindings against effective inherited contracts, reject abstract component instantiation, and preserve ancestor-qualified inline emitted action types for inherited handlers.

## 4. Runtime And Host Integration

- [x] 4.1 Update `crates/nx-interpreter/src/interpreter.rs` and runtime error handling so component evaluation and lifecycle entry points use effective component contracts, reject abstract component initialization, and treat concrete external components as bodyless stateless typed records.
- [x] 4.2 Update API-facing component/runtime artifacts in `crates/nx-api` and any related serialization paths so evaluated external component values preserve component identity plus normalized props and handlers when returned to hosts or serialized to JSON.

## 5. Verification

- [x] 5.1 Add or update lowering, type-checking, interpreter, API, and artifact tests covering local/imported component inheritance, inherited props/content/emits, external component evaluation, external lifecycle behavior, and host-visible serialization shape.
- [x] 5.2 Run targeted validation for the completed change (`openspec validate add-component-inheritance-and-external-components`) and the affected Rust test suites, then fix any resulting diagnostics or regressions.
