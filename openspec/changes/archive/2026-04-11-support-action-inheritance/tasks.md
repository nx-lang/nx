## 1. Parser and Syntax Artifacts

- [x] 1.1 Extend `action_definition` and `emit_definition` grammar, generated parser artifacts, AST helpers, validation hints, and syntax highlighting to support optional `abstract` and `extends` on top-level actions plus `extends` on inline emitted actions.
- [x] 1.2 Add parser fixtures, syntax tests, and grammar/docs examples for valid abstract/derived top-level actions, valid inline emitted derived actions, and invalid multi-base action declarations.

## 2. HIR and Inheritance Validation

- [x] 2.1 Update component predeclaration, lowering, and `nx_hir::records` validation to preserve action inheritance metadata for top-level and inline emitted actions, resolve abstract action bases through aliases/imports, and reject mixed record/action hierarchies.
- [x] 2.2 Add HIR and prepared-binding tests for inherited action ancestry, inline emitted action bases, duplicate inherited fields, invalid base kinds, and inheritance-cycle diagnostics.

## 3. Type System and Runtime Semantics

- [x] 3.1 Update `nx-types` to treat inherited top-level and inline emitted actions like inherited records for field access, construction, handler inputs, and subtype compatibility while still rejecting abstract action instantiation.
- [x] 3.2 Update interpreter/runtime tests to materialize inherited action defaults correctly and keep action-only validation paths, including component handler input checks for inherited inline emitted fields, aligned with action ancestry rules.

## 4. Code Generation Coverage

- [x] 4.1 Add TypeScript code generation coverage for exported abstract/concrete action families in single-file and library output, including inherited fields, concrete `$type` literals, and cross-module imports.
- [x] 4.2 Add C# code generation coverage for abstract action bases and concrete derived actions so inherited members and discriminator defaults remain correct.
