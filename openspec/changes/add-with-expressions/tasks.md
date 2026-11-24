## 1. Syntax

- [ ] 1.1 Add `with_expression` to `crates/nx-syntax/grammar.js`: left-associative at precedence 125, with a right side that is a `with_field_list` (`{` attribute-form entries `}`) or an operand (name, member access, call, element, braced expression). Make `with` a keyword after an operand.
- [ ] 1.2 Add `env with bundle` as an override of `provide_expression`, and replace `provide_definition`'s bundle form with `provide env with bundle`. Grammar tests for every form, for `with` as a record field name, and for the removed `provide bundle`.
- [ ] 1.3 Regenerate the parser and update the tree-sitter highlight queries.

## 2. Lowering and checking

- [ ] 2.1 Lower record and update-record `with` in `crates/nx-hir` to `apply` and `merge`, building the update from a field list and expanding paths with one evaluation of the left side.
- [ ] 2.2 Type-check `with`: field lists as the left type's `T.Update` (contextual typing of entries, duplicates, unknown fields), operands as `T.Update`, `with-path` for a step that is not a required record field (and for any path on an update record), and `with-target` for other left types.
- [ ] 2.3 Type-check bundle `with` (field lists read as `<env.Update … />` fields, paths allowed) and bundles in `provide` blocks (any `env.Update` expression). Count them in the static read sets and `environment-cycle`, and mark each override that reads the environment, from the read sets, so it is lowered as a formula.
- [ ] 2.4 Reject `with` as a declaration, parameter, `for` item or binding name with `reserved-name`. Search the repository's examples, tests and docs for `with` used as a name and rename them.

## 3. NX IR

- [ ] 3.1 Add the optional `base` operand of `envUpdate`, bundle overrides and formulas in `envProvide`, `formula` and `bundle` entries in `$env`, and the `environment-v2` feature to `crates/nx-ir` (model, emitter, reader validation, `explain`). Programs whose overrides read no environment value, and that use no bundle in a block, keep declaring only `environment-v1`.
- [ ] 3.2 Document the additions in `docs/nx-ir-format.md`. Check that a program written with `with` emits the same images as its `apply`/`merge` form.

## 4. Runtimes

- [ ] 4.1 Rust IR runtime: evaluate bundle `with`; capture formulas and bundles in `envProvide` and evaluate them at the read (with the "sources after it" rule) and their cost; carry them in `$env`; record the reads for the instance tree, including those a formula makes.
- [ ] 4.2 TypeScript runtime: the same, matching the Rust runtime on the corpus.
- [ ] 4.3 Generated code: emit record `with` through the existing `apply`/`merge` helpers, bundle `with`, bundles in blocks and formulas (as closures), and emit a top-level value of type `env.Update` as a function evaluated at each read.

## 5. Editor

- [ ] 5.1 Completion of fields and paths inside `with { … }`, and hover on `with` showing the patched type, in `crates/nx-language-service`.

## 6. Corpus and tests

- [ ] 6.1 Add conformance programs: `with-expressions` (field lists, paths, chains, update records, the `apply` equivalence) and `environment-formulas` (a bundle in a block, inline and named; patching the app theme; a mode set inside the section; a single override after a bundle; an override following a value overridden nearer the reader; captured locals; descriptors with formulas; operation counts). Move the `environment` programs to `provide env with`, and check whether any recorded result changes under the new rule.
- [ ] 6.2 Unit tests for each diagnostic: `with-target`, `with-path`, `reserved-name`, and the syntax errors for `env with` outside `provide` and for `provide bundle`.
- [ ] 6.3 Check generated JavaScript agrees on the new programs, including a theme that varies by mode in a section. If #75 has merged, remove `design-tokens-theme`'s `darkAccent` from `KNOWN_GENERATED_DIVERGENCES`.

## 7. Docs

- [ ] 7.1 Add a `with` section to `reference/syntax/expressions.md`, and point from `apply` in `reference/syntax/functions.md` to it.
- [ ] 7.2 Update `reference/syntax/environment.md`: `provide env with` for bundles, bundles in blocks, and the single rule "computed where it is read" with the `gap`/`space` example; remove the "evaluated where it is written" text.
- [ ] 7.3 If #75 has merged, update the `@nx/design` reference, the style guide and the design-tokens tutorial (step 4 can apply a theme to one section), and remove the "a bundle for one section" entry from `specs/future.md`. Record the open questions (an element form, `with` on union cases) in `specs/future.md`.
