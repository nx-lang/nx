## 1. Syntax

- [ ] 1.1 Add `with_expression` to `crates/nx-syntax/grammar.js`: left-associative at precedence 127 (above `??` at 125, below prefix unary at 130), with a right side that is a `with_field_list` (`{` plain attribute-form entries `}`) or an operand (name, member access, call, element, braced expression). Make `with` a keyword after an operand only.
- [ ] 1.2 Add `env with bundle` (the same operand, then optional `with` steps) as an override of `provide_expression`, and replace `provide_definition`'s bundle form with `provide env with bundle`, with an error node or message that suggests the new form for `provide brand.theme`. A block holds attribute-form overrides or one `env with` chain, never both; make a mix a syntax error that suggests a `with { … }` step. Grammar tests for every form: the rejected mix, an inline element, a `with` chain then the body, an empty field list before the body, `with` beside `??`, `with` as a record field, attribute and field-list entry name, and the removed `provide bundle`.
- [ ] 1.3 Regenerate the parser and update the tree-sitter highlight queries.
- [ ] 1.4 Update the VS Code TextMate grammar (`src/vscode/syntaxes/nx.tmLanguage.json`): `with` after an operand and after `provide env`, `provide-definition`'s new form, and `with` kept as a name and in markup text. Extend `src/vscode/test/grammar/environment.test.ts` and add a `with` grammar test.

## 2. Lowering and checking

- [ ] 2.1 Lower record and update-record `with` in `crates/nx-hir` to `apply` and `merge`, building the update from a field list and expanding paths with one evaluation of the left side.
- [ ] 2.2 Type-check `with`: update types (`env.Update` included) before records; field lists as the left type's `T.Update` (contextual typing of entries, duplicates, unknown fields); operands as `T.Update`; `with-path` for a step that is not a required record field (and for any path on a non-bundle update record); `with-target` for other left types, with an unwrap hint for optional and sequence types.
- [ ] 2.3 Type-check bundle `with` (field lists read as `<env.Update … />` fields, paths allowed) and bundles in `provide` blocks. Report `env with` outside `provide` as `reserved-name` with the "only after `provide`" message.
- [ ] 2.4 Mark each override and bundle entry whose static read set is not empty as a formula. Extend `environment-cycle` to an override, bundle entry or main-module provide of `V` that reads a `W` whose default reads `V`.
- [ ] 2.5 Reject `with` as a declaration, parameter, `for` item or binding name with `reserved-name`. Search the repository's examples, tests and docs for `with` used as a name and rename them.

## 3. NX IR

- [ ] 3.1 Add the formula flag on `envProvide` and `envUpdate` overrides, bundle overrides in `envProvide`, the optional `base` of `envUpdate`, `formula` and `bundle` entries in `$env` and `$entries`, the fingerprint check, `nx-ir-environment-cycle`, and the `environment-v2` feature to `crates/nx-ir` (model, emitter, reader validation, `explain`). Write the flag as an optional fourth override cell, present only on a formula, so programs that use none of these keep declaring only `environment-v1` and emit unchanged IR. Write `fingerprint` in `$env` in the lossless program-fingerprint form.
- [ ] 3.2 Document the additions and their cost in `docs/nx-ir-format.md`. Check that a program written with `with` emits the same images as its `apply`/`merge` form.

## 4. Runtimes

- [ ] 4.1 Rust IR runtime: accept `environment-v2`; capture formulas in `envProvide` and `envUpdate` and evaluate them at the read with the prepared program, with the "sources after it" rule at any depth and the runtime cycle error; evaluate only a bundle's entries for the value read; carry formulas and bundles in `$env`, check only their shape in `enter`, and refuse a stale fingerprint at the read that evaluates the formula; record the reads they make for the instance tree, so `set_environment` reaches readers through them.
- [ ] 4.2 TypeScript runtime: the same, matching the Rust runtime's results and operation counts on the corpus.
- [ ] 4.3 Generated code: emit record `with` through the existing `apply`/`merge` helpers, bundle `with`, bundles in blocks, and formulas as functions of no arguments that close over their locals and also hold `module` and `locals`, in the generated environment helpers and `enter`; make `JSON.stringify` of a generated formula throw.

## 5. Editor

- [ ] 5.1 In `crates/nx-language-service`: completion of fields and paths inside `with { … }` (environment values for bundles), hover on `with` showing the patched type, and the bundle hover on `provide env with …` following `with` steps.

## 6. Corpus and tests

- [ ] 6.1 Add conformance programs: `with-expressions` (field lists, paths, chains, `with` beside `??`, update records, the `apply` equivalence) and `environment-formulas` (a bundle in a block, inline, named and passed as a prop; patching the app theme; a mode set inside the section; a `with` step after a bundle; an override following a value overridden nearer the reader; captured locals in a formula; one bundle entry reading another value it sets; descriptors with formulas; the runtime cycle error; operation counts). Move the `environment` and `design-tokens-theme` programs to `provide env with`, and check whether any recorded result changes under the new rule.
- [ ] 6.2 Unit tests for each diagnostic: `with-target`, `with-path`, `environment-cycle` on an override, `reserved-name` (for `with` and for `env with` outside `provide`), the syntax errors for `provide bundle` and for a block that mixes a bundle with attribute-form overrides, and a stale formula fingerprint.
- [ ] 6.3 Migrate the tests that use `provide bundle`: `crates/nx-language-service/src/environment_tests.rs`, `crates/nx-syntax/tests/parser_tests.rs`, `crates/nx-types/tests/environment_values.rs`, and the TextMate test; and the comments in `runtime/typescript/src/index.ts`.
- [ ] 6.4 Check generated JavaScript agrees on the new programs, including a theme that varies by mode in a section, compare formula entries by `value`, `path`, `module`, `locals` and the values reads give, and remove `design-tokens-theme`'s `darkAccent` from `KNOWN_GENERATED_DIVERGENCES`, updating its doc comment (or removing the constant if empty) and `specs/ir-conformance/README.md`.

## 7. Docs

- [ ] 7.1 Add a `with` section to `reference/syntax/expressions.md`, and point from `apply` in `reference/syntax/functions.md` to it.
- [ ] 7.2 Update `reference/syntax/environment.md`: `provide env with` for bundles, bundles in blocks, and the single rule "computed where it is read" with the `gap`/`space` example; remove the "evaluated where it is written" text.
- [ ] 7.3 Update the `@nx/design` reference, the style guide and the design-tokens tutorial (step 4 can apply a theme to one section; `provide env with brand`), remove the "a bundle for one section" entry and the generated-code divergence note from `specs/future.md`, and record the open questions (an element form, `with` on union cases, conditional entries in field lists) there.
