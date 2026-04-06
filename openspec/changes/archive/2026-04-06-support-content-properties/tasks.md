## 1. Syntax And Lowering

- [x] 1.1 Add content-aware property-definition grammar for all content-capable declaration surfaces: plain records, actions, paren-style `let`, element-style `let`, component props, emitted-action payloads, and component state; then regenerate parser artifacts and update syntax metadata/highlighting.
- [x] 1.2 Extend lowered function/component/record metadata to preserve `is_content` on params and fields, including record-shape resolution and effective-shape validation for inherited content properties.
- [x] 1.3 Preserve ordinary text body runs during element lowering so text content can participate in content-property binding.

## 2. Binding And Runtime Semantics

- [x] 2.1 Replace implicit body-binding special cases in `nx-types` with explicit content-property binding for NX-defined record, function, and component invocations.
- [x] 2.2 Update `nx-interpreter` evaluation and runtime representation to inject normalized body content into the declared content property and to use generic `content` terminology for intrinsic/native element body handling.
- [x] 2.3 Propagate content-property metadata through `nx-api` library interfaces so imported records, functions, and components behave the same as local ones.

## 3. Diagnostics, Docs, And Tests

- [x] 3.1 Add parser, lowering, and record-inheritance tests covering content-marked properties across all supported declaration forms and contextual-keyword behavior.
- [x] 3.2 Add type-checker and interpreter tests for text content binding, sequence content binding, missing content-property errors, double-supply conflicts, and component/paren-function markup invocation.
- [x] 3.3 Update grammar/reference docs and tutorials to use inline `content` property markers and remove stale `children` terminology or standalone-slot examples.
