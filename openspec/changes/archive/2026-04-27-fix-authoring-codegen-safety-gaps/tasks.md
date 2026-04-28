## 1. Nullable List Binding

- [x] 1.1 Add focused unit tests in `crates/nx-types/src/semantics.rs` for `T[]` satisfying `T[]?`, scalar-to-`T[]?` coercion, `T[]?` not satisfying `T[]`, and `T?[]` not satisfying `T[]?`.
- [x] 1.2 Update `type_satisfies_expected_with_coercion` so array values can satisfy nullable-array targets through normal nullable widening before list-to-scalar rejection runs.
- [x] 1.3 Add type-checker tests for annotated nullable-list `let` bindings and record/component field bindings using multi-item braced literals.
- [x] 1.4 Add regression tests proving incompatible element types still produce `value-type-mismatch` or `property-type-mismatch` diagnostics with full composed type shapes.
- [x] 1.5 Add interpreter coverage proving accepted nullable-list bindings evaluate to the supplied list rather than null or omission.

## 2. TypeScript Cross-Library Imports

- [x] 2.1 Add `typescript_package_prefix: Option<String>` to generation options and expose it as `--typescript-package-prefix` on `nxlang generate`.
- [x] 2.2 Extend the exported type graph or TypeScript render context so dependency-library imports can distinguish local generated names, dependency exported names, and dependency package targets.
- [x] 2.3 Update TypeScript import collection to merge same-library relative imports with cross-library package imports without duplicating symbols.
- [x] 2.4 Emit type-only package imports for dependency-library references, including `import type { Exported as Local }` aliases when the local generated name differs.
- [x] 2.5 Emit one TypeScript warning per assumed dependency package target, including the dependency library name and exact emitted package specifier.
- [x] 2.6 Add generator tests for wildcard dependency imports, selective or qualified dependency imports that need aliases, package-prefix handling, and generated warning text.
- [x] 2.7 Add CLI tests covering `--typescript-package-prefix` parsing and generated output for a `chat-link` library importing a sibling `question-flow` library.

## 3. Strict Record Construction Validation

- [x] 3.1 Add type-checker tests for unknown fields on record literals, element-style plain records, action records, and derived records with inherited fields.
- [x] 3.2 Ensure record literal inference reports an unknown-record-field diagnostic for supplied fields outside the effective record shape before field type checks and defaults are applied.
- [x] 3.3 Ensure element binding checks for record-compatible targets use the effective record shape and report unknown-record-field diagnostics consistently with record literals.
- [x] 3.4 Add a runtime error kind for unknown record fields, including the target record name, field name, and construction operation.
- [x] 3.5 Update runtime record value construction to reject unknown override fields before applying content fields, defaults, or coercions.
- [x] 3.6 Add interpreter/runtime tests proving direct record construction fails on unknown fields and still applies defaults for known fields.

## 4. Documentation And Cleanup

- [x] 4.1 Update language documentation with nullable list authoring examples and strict unknown-field behavior.
- [x] 4.2 Update CLI documentation and help text for `--typescript-package-prefix` and assumed TypeScript dependency package warnings.
- [x] 4.3 Verify `specs/future.md` has no completed entries for nullable-list authoring, TypeScript cross-library imports, or strict record validation remaining to remove.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run focused `nx-types`, `nx-interpreter`, `nx-cli`, and `nx-api` tests that cover the changed paths.
- [x] 5.3 Run full workspace Rust tests if focused tests pass.
- [x] 5.4 Run `openspec validate fix-authoring-codegen-safety-gaps --strict`.
