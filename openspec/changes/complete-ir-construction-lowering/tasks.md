## 1. Shared rejection of positional record construction

- [ ] 1.1 Add `record-construction-requires-element` in shared source analysis with the offending call span, caller file identity and a short named-element example without field mapping or generated rewrites; reject zero-argument calls as well as supplied positional arguments.
- [ ] 1.2 Cover semantic recognition of imported/qualified/type-aliased record targets; verify the same simple diagnostic for empty and nonempty calls without secondary missing-function errors. No per-field, default-aware or invalid-arity rewrite tests are needed.
- [ ] 1.3 Replace the ignored positional success test with active rejection and equivalent markup/default tests; verify ordinary factory functions and existing first-class-function/shadowing rules remain unchanged.
- [ ] 1.4 Verify the diagnostic reaches checker-backed language services, CLI and codegen consumers before emission/execution; preserve positive imported, inherited and defaulted markup cases.

## 2. Conditional property lowering

- [ ] 2.1 Document the representation before coding: absence versus empty, once-only evaluation and ordering, defaults, target kinds, content, mixed entries and multiple/nested fragments; audit existing checker path enumeration.
- [ ] 2.2 If needed, specify/version any IR feature and update affected capability deltas before implementation; update preparation/evaluation in both runtimes and verify old-runtime refusal by feature name.
- [ ] 2.3 Implement simple, condition-list and match fragments across the shared builder/backends, preserving first-match selection, lexical narrowing and lazy inactive expressions; activate the five retained conditional/example success tests.
- [ ] 2.4 Verify required fields on every possible path, duplicate active properties, valid mutually exclusive duplicates, omission/default versus explicit empty, content binding, and existing restrictions on conditional type arguments.
- [ ] 2.5 Add explicit expected-result/error tests across Rust IR, TypeScript IR and generated JavaScript for multiple fragments, inactive failing expressions, ordering and once-only evaluation across each existing target kind; verify inactive branches still receive static diagnostics; assert linear emitted image/code growth without flaky timing thresholds.

## 3. Documentation and completion

- [ ] 3.1 Update syntax/style documentation, migration guidance and future notes with named record construction and explicit factory examples; remove promises of future positional constructor support without rewriting archived history.
- [ ] 3.2 Remove the temporary retirement exception through the runtime delta and confirm no ignores naming this change remain; keep unrelated pre-existing ignores intact.
- [ ] 3.3 Run affected checker/language-service, compiler, Rust/TypeScript runtime, generated JavaScript, corpus, CLI and example suites plus formatting/lint checks; review any fixture/image changes explicitly.
- [ ] 3.4 Validate the change strictly, reconcile any new IR capability deltas, and verify the implementation against the revised requirements before syncing/archiving.
