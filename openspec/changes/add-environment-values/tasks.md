## 1. Syntax

- [ ] 1.1 Add `environment_definition` (optional visibility, contextual `environment`, name, `:` type, `=` default) and a top-level `provide_definition` (`provide env.path = value` and `provide expression`) to `crates/nx-syntax/grammar.js`, with `environment` and `provide` contextual as `visualizer` is.
- [ ] 1.2 Add `provide_expression` (`provide` overrides `{ body }`) in expression and content positions. Check that `env.name.field=value` attributes already parse through `qualified_markup_name`, and add grammar tests for every form and for `environment`/`provide` used as field names.
- [ ] 1.3 Regenerate the parser and update the tree-sitter highlight queries.

## 2. Lowering and checking

- [ ] 2.1 Lower environment declarations, `env.` reads, `env.` attributes (as a provide around the element, removed from its properties), provide blocks and top-level provides in `crates/nx-hir`. Reject `env` as a binding or declaration name with `reserved-name`.
- [ ] 2.2 Resolve `env.name` among the module's own, its library's internal and its imports' exported environment values, with `unknown-environment-value` and `ambiguous-environment-value`. Add `duplicate-environment`, `environment-default-required` and the reserved `Update` name.
- [ ] 2.3 Type-check defaults, overrides and field paths (`environment-path`), and add the `env.Update` type: fields for the visible values, keyed by declaration, record values taking their update type.
- [ ] 2.4 Compute static read sets for functions, values, defaults and component bodies, and add `environment-cycle` for defaults and main-module provides.
- [ ] 2.5 Reject a top-level provide in a module another module of the program imports, with `provide-outside-main-module`.
- [ ] 2.6 Search the repository's examples, tests and docs for `env` used as a name, and rename what `reserved-name` now rejects.

## 3. NX IR

- [ ] 3.1 Add the `environment` declaration kind, the `envRead` (30) and `envProvide` (31) node kinds, the module `provides` list and the `environment-v1` feature to `crates/nx-ir` (model, emitter, reader validation and `explain`).
- [ ] 3.2 Emit `env.Update` construction with qualified field names, and emit nothing new for programs without environment values (check the existing IR snapshots are unchanged).
- [ ] 3.3 Document the nodes, `$env`, the lookup order and the cost rules in `docs/nx-ir-format.md`. Add conformance corpus cases with results and operation counts.

## 4. Rust IR runtime

- [ ] 4.1 Add the `Environment` type (`root`, `enter`), root-value naming and checking (`nx-ir-environment-name`), and an environment in the runtime options for every evaluation and the lifecycle.
- [ ] 4.2 Evaluate `envRead` with the lookup order, evaluating defaults and entry-module provides at the read (with the "sources after it" rule), and `envProvide` with overrides evaluated outside, the body inside and `$env` added to the component descriptors of the result.
- [ ] 4.3 Record the slots and values each evaluation reads, for the instance tree.
- [ ] 4.4 Add `--env name=value` to `nxlang run` in `crates/nx-cli`, parsing the value as NX value text against the declared type.

## 5. Instance tree

- [ ] 5.1 Give each node its environment, run every initialization, render and dispatch in it, and answer a host's query for a value at a node.
- [ ] 5.2 Keep each node's recorded reads and the index from slot to readers, current across render, replace, remove and drop.
- [ ] 5.3 Add `set_environment` (all or nothing, via the index, re-checking reads top-down, rendering only nodes that see a different value and leaving them unwalked).
- [ ] 5.4 Extend the visit's skip check: compare descriptors without `$env`, then check the recorded reads in the new environment, and take the new descriptor when only overrides changed.

## 6. TypeScript runtime and generated code

- [ ] 6.1 Add the environment (`environment(values)`, `enter`), the options and the evaluation of the new nodes to `runtime/typescript`, matching the Rust runtime on the corpus.
- [ ] 6.2 Emit environment reads, provides and entry-module provides in `crates/nx-codegen`, with the environment as an option of entry functions and component evaluation and the lookup shared with the runtime package.

## 7. Tooling and docs

- [ ] 7.1 Add completion after `env.` (expressions and attributes, then record fields), hover on reads and overrides, the "reads" list on function, value and component hovers, the "sets" list on top-level provides (bundles included), and go to definition, in `crates/nx-language-service`.
- [ ] 7.2 Scope `environment`, `provide` and `env` in the TextMate grammar in `src/vscode`, with highlighting tests.
- [ ] 7.3 Check that the Visual view shows an element with `env.` attributes without errors, and that the source tree reports them.
- [ ] 7.4 Write the language reference page for environment values on the website, add `provide` layout to the style guide, and update `crates/nx-ir-runtime/README.md` (environment, read tracking, `set_environment`).
- [ ] 7.5 Record in `specs/future.md`: lazy local overrides, values only the host sets, standard host inputs, `$env` on host elements, and memoizing defaults.

## 8. Tests and completion

- [ ] 8.1 Add source-level tests for every scenario of the environment-values spec, through both runtimes and generated JavaScript, and instance-tree tests for the new and modified tree scenarios.
- [ ] 8.2 Run the Rust, TypeScript and codegen test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
