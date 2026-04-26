## 1. Syntax And Parser

- [x] 1.1 Update `nx-grammar.md` and `nx-grammar-spec.md` with discriminated union grammar, required leading-pipe cases, enum guidance, construction forms, and match narrowing rules.
- [x] 1.2 Add `union_definition`, `union_case_list`, and `union_case` rules to `crates/nx-syntax/grammar.js` without changing existing enum parsing.
- [x] 1.3 Regenerate tree-sitter parser artifacts, node types, and syntax kinds for the new union nodes.
- [x] 1.4 Add valid parser fixtures for fieldless cases, payload cases, defaults, inherited shared fields, and mixed enum/union declarations.
- [x] 1.5 Add invalid parser or validation fixtures for missing leading pipe, duplicate cases, empty case list, malformed case payloads, and invalid union `extends` clauses.
- [x] 1.6 Update parser snapshot tests and syntax validation tests for union declarations and scoped case constructors.

## 2. HIR And Prepared Metadata

- [x] 2.1 Add HIR structs for discriminated union definitions, union cases, and case fields, and add `Item::Union`.
- [x] 2.2 Lower union declarations from CST into HIR while preserving visibility, optional abstract record base, case order, case spans, fields, defaults, and content markers.
- [x] 2.3 Add prepared binding and interface metadata for exported, peer, and imported unions without exposing cases as top-level declarations.
- [x] 2.4 Extend library export tables and published interface metadata so imported modules can resolve `Union.case` through the visible union name.
- [x] 2.5 Add semantic validation for duplicate case names, invalid abstract bases, duplicate inherited fields, invalid content markers, and invalid case defaults.
- [x] 2.6 Add HIR and prepared-module tests covering local, same-library, imported, and private union visibility.

## 3. Type Checking And Narrowing

- [x] 3.1 Extend type resolution so union names and union case names can be represented distinctly enough for compatibility, diagnostics, and display.
- [x] 3.2 Validate payload case construction through `<Union.case ... />`, including required fields, defaulted fields, nullable fields, unknown fields, content fields, and field type mismatches.
- [x] 3.3 Support fieldless case member shorthand such as `LoadState.idle` and reject bare member use for payload cases.
- [x] 3.4 Add compatibility rules so each case type satisfies its owning union and unions that extend an abstract record satisfy that abstract base where appropriate.
- [x] 3.5 Update common-supertype inference so sibling cases of the same union infer to the owning union in braced value sequences and typed bindings.
- [x] 3.6 Enforce union field access rules: inherited shared fields are available on the union, case-specific fields require narrowing.
- [x] 3.7 Preserve match-style `if value is { ... }` arm metadata in HIR or an equivalent analysis structure instead of losing it to generic nested `if` lowering.
- [x] 3.8 Implement union case pattern validation, identifier narrowing inside matching arms, and non-exhaustive match diagnostics when a union match lacks `else`.
- [x] 3.9 Add type checker tests for valid construction, invalid construction, compatibility, shared fields, case field access, narrowing, exhaustiveness, and wrong-union patterns.

## 4. Interpreter And Runtime Values

- [x] 4.1 Evaluate payload union case constructors into record-like runtime values whose type name is the fully scoped case discriminator.
- [x] 4.2 Evaluate fieldless case shorthand into record-like runtime values with the fully scoped case discriminator and no case-specific fields.
- [x] 4.3 Apply case defaults and inherited abstract-base defaults during union case construction.
- [x] 4.4 Update runtime expected-type checks so union case values satisfy their owning union and any extended abstract record base.
- [x] 4.5 Update match execution so union case patterns compare against case discriminator identity while preserving existing enum and literal match behavior.
- [x] 4.6 Update canonical `NxValue` conversion, JSON output, and MessagePack output so union cases use `$type` maps and enums remain bare strings.
- [x] 4.7 Add interpreter and runtime output tests for payload cases, fieldless cases, defaults, inherited fields, JSON, MessagePack, and enum separation.

## 5. Code Generation

- [x] 5.1 Extend the exported type graph model with exported union declarations, case metadata, inherited fields, and referenced field types.
- [x] 5.2 Update TypeScript generation so exported unions emit a closed narrowable union surface with literal `$type` case members.
- [x] 5.3 Update TypeScript library generation to emit required type-only imports for union case fields that reference types from other generated modules.
- [x] 5.4 Update C# generation so exported unions emit a root type plus sealed case DTOs using `$type` JSON and MessagePack polymorphism metadata.
- [x] 5.5 Ensure generated C# and TypeScript keep regular enums on the existing enum/literal-string paths and fieldless unions on the `$type` polymorphic path.
- [x] 5.6 Add single-file and library code generation tests for TypeScript and C#, including inherited union fields and cross-module field references.
- [x] 5.7 Update generated code documentation or README examples to show exported discriminated union outputs.

## 6. .NET Binding

- [x] 6.1 Add managed raw JSON and MessagePack tests proving union cases are exposed as `$type` maps through `NxRuntime`.
- [x] 6.2 Add managed typed DTO serialization tests proving generated union cases serialize to the same `$type` map shape as raw runtime output.
- [x] 6.3 Add managed typed DTO deserialization tests proving `$type` maps deserialize to the generated union case DTOs.
- [x] 6.4 Verify enum raw and typed workflows remain bare-string based after union support is added.
- [x] 6.5 Update `bindings/dotnet/README.md` with discriminated union raw and generated typed model guidance.

## 7. Tooling, Examples, And Docs

- [x] 7.1 Update VS Code TextMate grammars to highlight union declarations, case separators, case fields, and scoped case constructors.
- [x] 7.2 Update VS Code grammar tests and snippets for discriminated union declarations and match arms.
- [x] 7.3 Add or update NX examples demonstrating `enum` for scalar choices and discriminated unions for payload-bearing states.
- [x] 7.4 Update language tour and reference docs for types, expressions, and `if` matching with the new union syntax and narrowing behavior.
- [x] 7.5 Document that `type Result = Success | Failure` is intentionally not part of this feature and remains available for a possible future proposal.

## 8. Verification

- [x] 8.1 Run Rust formatting and the relevant Rust test suites for syntax, HIR, type checking, interpreter, runtime values, API, FFI, and CLI code generation.
- [x] 8.2 Run .NET formatting/build/tests for `bindings/dotnet`.
- [x] 8.3 Run VS Code extension grammar tests.
- [x] 8.4 Run documentation or example validation commands used by the repository.
- [x] 8.5 Run `openspec status --change add-discriminated-unions` and confirm the change is apply-ready.
