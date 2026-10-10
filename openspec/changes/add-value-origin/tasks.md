## 1. TypeScript runtime

- [ ] 1.1 Add `origins` to `NxRuntimeOptions` with its report type, cleared at the start of a call and refused with `nx-ir-options` when it cannot be written; verify with the scenarios of `TypeScript runtime reports origins through its options`
- [ ] 1.2 Keep each record's constructing node in a `WeakMap` while a report is given, carry it through binding, props, state, sequences, returns and update application, and fill the report in the result walk; verify with the scenarios of `A record's origin is the element expression that constructed it` and `An IR runtime reports origins to a host that asks`
- [ ] 1.3 Run the conformance corpus with and without a report and compare serialized results; verify no difference

## 2. Rust runtime

- [ ] 2.1 Add `RuntimeOptions::origins` and the `Origins` report type, exported from the crate; verify with the scenario of `Rust runtime reports origins through its options`
- [ ] 2.2 Hold the constructing module and node on the internal `Record` while a report is given, and fill the report in the conversion to `NxValue`; verify with the same origin scenarios as 1.2
- [ ] 2.3 Compare both runtimes' reports over the conformance corpus; verify with `Corpus parity`

## 3. Cost

- [ ] 3.1 Measure the cost of a report with the performance harness on the question-flow lifecycle in both runtimes, and of no report against the base; record both in the runtime READMEs

## 4. Specs

- [ ] 4.1 Run `openspec validate add-value-origin --strict`; verify it passes
