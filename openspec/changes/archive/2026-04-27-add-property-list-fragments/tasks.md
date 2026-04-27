## 1. HIR Model And Lowering

- [x] 1.1 Add property-fragment HIR types for direct values, simple conditionals, condition lists, and match fragments.
- [x] 1.2 Update `Element` and related helper APIs so existing direct-property consumers can migrate without losing source-order information.
- [x] 1.3 Lower `property_value` nodes into direct property entries while preserving existing handler-binding behavior.
- [x] 1.4 Lower `property_list_if_simple_expression` into conditional property entries with then and else branches.
- [x] 1.5 Lower `property_list_if_condition_list_expression` into condition-list property entries with arm and else branches.
- [x] 1.6 Lower `property_list_if_match_expression` into match property entries with scrutinee, patterns, arms, and else branch.
- [x] 1.7 Add parser/lowering tests proving conditional, condition-list, and match property fragments are preserved and not silently dropped.

## 2. HIR Traversal And Scope Integration

- [x] 2.1 Update HIR expression visitors and scope construction to walk expressions nested inside property fragments.
- [x] 2.2 Update component handler rewrite collection so handler expressions nested in property fragments are discovered.
- [x] 2.3 Update diagnostics and source-span plumbing so property-fragment errors point at the relevant fragment or property binding.
- [x] 2.4 Add scope tests for identifiers used inside conditional and match property-fragment branches.

## 3. Type Checking And Binding Analysis

- [x] 3.1 Add a property-path analysis helper that summarizes every reachable property set from a property-fragment list.
- [x] 3.2 Reject duplicate properties that can occur on the same reachable path, including static-plus-conditional duplicates.
- [x] 3.3 Allow the same property key in mutually exclusive branches.
- [x] 3.4 Validate required component/function/record/union-case properties across all reachable paths.
- [x] 3.5 Integrate property-path validation with content-property binding and named/body content conflict checks.
- [x] 3.6 Integrate property-path validation with emitted-action handler properties and ordinary component prop checks.
- [x] 3.7 Add type-checker tests for required props supplied on all branches, missing on one branch, same-key mutually exclusive branches, and same-path duplicates.
- [x] 3.8 Add type-checker tests for conditional content-property satisfaction and content body conflicts.

## 4. Match Fragment Narrowing

- [x] 4.1 Factor reusable union match-pattern validation from value `Expr::Match` inference.
- [x] 4.2 Apply union case validation, wrong-union diagnostics, and exhaustiveness checks to property-list match fragments.
- [x] 4.3 Narrow local identifier scrutinees while inferring property values inside each match arm.
- [x] 4.4 Add type-checker tests for property-list match narrowing, non-exhaustive union matches without else, and wrong-union patterns.

## 5. Runtime Evaluation

- [x] 5.1 Evaluate property fragments into an ordered active direct-property list before invoking components, functions, records, or union case constructors.
- [x] 5.2 Evaluate simple conditional and condition-list fragments using existing boolean condition semantics.
- [x] 5.3 Evaluate match property fragments using existing pattern matching and union discriminator semantics.
- [x] 5.4 Fail explicitly if runtime evaluation encounters duplicate active properties that static analysis should have rejected.
- [x] 5.5 Add interpreter/runtime tests for then/else selection, match-arm selection, and active property ordering.

## 6. Tooling And Documentation

- [x] 6.1 Update language-tour and syntax reference docs with conditional and match-style property fragment examples.
- [x] 6.2 Document required-property and duplicate-property branch rules.
- [x] 6.3 Update examples to demonstrate conditional props and a union-narrowing property-list match.
- [x] 6.4 Update VS Code syntax highlighting and grammar tests for property-list fragment syntax.
- [x] 6.5 Remove the completed property-list fragment note from `specs/future.md`.

## 7. Verification

- [x] 7.1 Run Rust formatting and focused parser, HIR, type-checker, interpreter, and API tests for property-list fragments.
- [x] 7.2 Run relevant CLI/codegen tests to ensure element property model changes do not regress generated type surfaces.
- [x] 7.3 Run VS Code grammar tests.
- [x] 7.4 Run documentation or example validation commands used by the repository.
- [x] 7.5 Run `openspec validate add-property-list-fragments --strict` and confirm the change is apply-ready.
