## 1. Record construction

- [ ] 1.1 Lower positional record calls to construction and enable the retained positional regression test; verify explicit values and defaults.
- [ ] 1.2 Cover inherited, imported and aliased fields and invalid arguments; verify Rust/TypeScript IR and generated JavaScript against handwritten results.

## 2. Conditional properties

- [ ] 2.1 Settle and document a linear lowering representation; verify inactive branches remain lazy and independent fragments do not cause exponential image growth.
- [ ] 2.2 Implement simple, condition-list and match fragments and enable their retained ignored tests; verify lexical narrowing, defaults and duplicate diagnostics.
- [ ] 2.3 If the representation adds an IR feature, version it and update preparation/evaluation in both runtimes; verify older runtimes refuse it by feature name.

## 3. Integration

- [ ] 3.1 Run compiler, Rust/TypeScript runtime, generated JavaScript, corpus and CLI suites; verify explicit expected results across engines.
- [ ] 3.2 Remove the temporary limitation documentation and all ignores naming this change; verify no pending regressions remain.
