## 1. Parser and Syntax Artifacts

- [ ] 1.1 Extend `action_definition` grammar, generated parser artifacts, AST helpers, validation hints, and syntax highlighting to support optional `abstract` and `extends` on top-level actions.
- [ ] 1.2 Add parser fixtures, syntax tests, and grammar/docs examples for valid abstract/derived actions and invalid multi-base action declarations.

## 2. HIR and Inheritance Validation

- [ ] 2.1 Update lowering and `nx_hir::records` validation to preserve action inheritance metadata, resolve abstract action bases through aliases/imports, and reject mixed record/action hierarchies.
- [ ] 2.2 Add HIR and prepared-binding tests for inherited action ancestry, duplicate inherited fields, invalid base kinds, and inheritance-cycle diagnostics.

## 3. Type System and Runtime Semantics

- [ ] 3.1 Update `nx-types` to treat inherited actions like inherited records for field access, construction, and subtype compatibility while still rejecting abstract action instantiation.
- [ ] 3.2 Update interpreter/runtime tests to materialize inherited action defaults correctly and keep action-only validation paths, including handler input checks, aligned with action ancestry rules.

## 4. Code Generation Coverage

- [ ] 4.1 Add TypeScript code generation coverage for exported abstract/concrete action families in single-file and library output, including inherited fields, concrete `$type` literals, and cross-module imports.
- [ ] 4.2 Add C# code generation coverage for abstract action bases and concrete derived actions so inherited members and discriminator defaults remain correct.
