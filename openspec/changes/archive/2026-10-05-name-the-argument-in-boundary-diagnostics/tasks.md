## 1. Before anything is changed

- [x] 1.1 Bring this change's `agent-host-package` delta up to the main spec, which `treat-host-values-as-json` changes if it was applied first (on 2026-10-05 the two differed by this change's edits alone): copy the requirement *A function tool runs its function under the evaluation budget* whole from `openspec/specs/agent-host-package/spec.md` and reapply this change's edits (the classification paragraph, the argument named in the scenario *A boundary failure of a kind added later is invalid input*, and the three added scenarios); verify by a line diff of the delta's requirement against the main spec's that those are the only differences, since `openspec validate --strict` passes with the two texts apart and a MODIFIED requirement replaces the whole of what it modifies
- [x] 1.2 List, for `runtime/typescript/src/index.ts` and for `crates/nx-ir-runtime/src`, (a) every site that raises `nx-ir-arguments`, `nx-ir-function-value` or a code beginning `nx-ir-boundary-`, and whether it can be reached while an entry call checks one argument; (b) every place a default is evaluated or its value checked while a value is being checked (a record's fields, a union case's fields, anything else); (c) every place a resource limit can be raised inside that check; and (d) every caller that enters a function at depth 0; record the four lists under this task, verify them against `grep` of both sources, and stop and update `design.md` if a default is evaluated, or a function entered at depth 0, somewhere decision 2 does not cover
  - Found on 2026-10-05, line numbers as of `e63dfcb`. Decision 2 covers all of it.
  - (a) TypeScript, reached while an entry call checks an argument: the missing-required `fail` in
    `invokeFunction` (2821) and everything under `normalizeValue` (`normalizePrimitiveValue`,
    `normalizeNominalValue`, `resolveSubtype`, `normalizeFields`, `normalizePatchFields`,
    `requireObject`, and `resolveFunctionRecord` with its two `nx-ir-function-value` failures). Not
    reached: the count of positional arguments (2804, before the loop), `callFunction`'s own record
    (3362, and `resolveFunctionRecord` called with the path `callFunction`), action records
    (2198, 2223), handler properties (3484, 3490, 3526) and `applyContentBinding` (4148, 4151),
    which serve component entry points and constructions in a body.
  - (a) Rust, reached: the missing-required `fail` in `invoke` (`eval.rs` 944) and every site of
    `normalize.rs` (`require_record`, `resolve_function`, `normalize`, `normalize_nominal`,
    `resolve_subtype`, `normalize_fields`, `normalize_patch_fields`, `normalize_primitive`). Not
    reached: the count (`eval.rs` 900), `bind_content` (`eval.rs` 1464, 1470), `call_function`'s
    own record (`component.rs` 659, and `resolve_function` called with the path `call_function`),
    parent handlers and handler properties (`component.rs` 189, 275, 284) and action records
    (`component.rs` 910, 953).
  - (b) Two places in each runtime, and `grep default` finds no other: a parameter's default in the
    binding loop (`index.ts` 2817, `eval.rs` 940), whose value the loop then checks, and a field's
    default in `normalizeFields` / `normalize_fields` (`index.ts` 4200, `normalize.rs` 439), which
    serves a record's fields, a derived record's, a union case's, a component's props and state
    and an action's. `normalizePatchFields` / `normalize_patch_fields` evaluates no default.
  - (c) Inside the check: the charge of one operation for each value (`index.ts` 4296,
    `normalize.rs` 199) and for each item of a content list (`bindContent`; `eval.rs` 928, 932);
    the Rust stack limit (`normalize.rs` 118); and whatever a default evaluates, which can reach
    `maxOperations`, `maxExpressionNesting`, `maxRangeLength`, `maxCallDepth` and, in Rust,
    `maxStackBytes`. The JavaScript engine's limit is a `RangeError` until `evaluate` reports it,
    outside the binding. Every one has the code `nx-ir-resource-limit`.
  - (d) TypeScript: `evaluateFunction` (2014) and `callFunction` (3365, through
    `invokeFunctionByName`). The two other callers, `evalCall` (3303) and `evalNamedCall` (3316),
    pass `context.depth + 1`. Rust: `evaluate_function` (`component.rs` 636) and `call_function`
    (669, through `invoke_by_name`); `eval_call` (`eval.rs` 819) and `eval_named_call` (835) pass
    `cx.depth + 1`, and a unit test of `eval.rs` enters at 0.
  - Neither runtime recovers from a failure inside one evaluation (the only `catch` on the way is
    in `evaluate`, and the Rust `?` goes to the entry point), so the mark of decision 2 is a flag
    on the evaluation, set where a default fails, and not a field on the error.

## 2. TypeScript runtime

- [x] 2.1 Add `argument?: string` to `NxIrDiagnostic` with a doc comment saying when it is present, and set it by the rule of design decision 2: `evaluateFunction` and `callFunction` say the call is the host's with a parameter that reaches the loop in `invokeFunction` (directly, and through `invokeFunctionByName`); the binding of a parameter the host supplied a value for, and the missing-required failure, name the parameter on a diagnostic whose code is of the boundary family, `nx-ir-arguments` or `nx-ir-function-value` and which no default raised; verify with tests in `runtime/typescript/test` for the first five scenarios of the spec's requirement
- [x] 2.2 Verify with tests that no `argument` is set for: a record field default that divides by zero, one that calls a function that fails, and one whose value does not fit its field (`type Req = { n:int label:string = { n } }`, for as long as the checker accepts it; see `specs/future.md`); a parameter default of the entry call that fails; a field default and a default of a parameter the call leaves out that fail with `nx-ir-arguments` and with `nx-ir-boundary-type`, by calling a function value the host supplied (`type Stepper = { step: <function n:int />: int value:int = { step(1) } }`), which need no checker gap and are what fail when the mark or the exemption is removed; a failure in the arguments of a function the body calls; a failure in the result; input over `maxInputSize`; an operation budget spent while an argument is checked (fifty records for `items:Item+` under `maxOperations: 20`); `maxCallDepth` reached through a default; more positional arguments than parameters; a `Function` record given to `callFunction` as the function to call that names no function; and a failure of a component entry point
- [x] 2.3 Rebuild `runtime/typescript/dist`, run the runtime's tests, and verify the committed `dist` matches the source

## 3. Rust runtime, and the tests that hold the two runtimes together

- [x] 3.1 Add `argument: Option<String>` to `Diagnostic` (`crates/nx-ir-runtime/src/error.rs`) and set it by the same rule: `evaluate_function` and `call_function` (`component.rs`) say the call is the host's through `call` to the binding loop in `eval.rs` (directly, and through `invoke_by_name`), and the place `normalize.rs` evaluates a default marks what it raises; verify with tests in the crate for the scenarios of the spec's requirement, the `Function` record that names no function among them, and for each case of 2.2, the stack limit included
- [x] 3.2 Give the conformance corpus a case that fails, as design decision 4 has it: `"fails": true` on a case in `program.json`; `expected/diagnostics.json` with the `code` and the `argument` of each such case, written by the generator (`crates/nx-codegen/src/ir_corpus_tests.rs`) from the Rust runtime, with no entry for the case in `results.json` or `operations.json`; the generator panicking for a case that fails unmarked or is marked and succeeds; and both runners (`crates/nx-ir-runtime/tests/corpus.rs`, `runtime/typescript/test/corpus.test.mjs`) comparing the code and the argument for both images and making no count, usage or input-size check for the case. Make both runners also require that the diagnostic names no argument where they check a case under a budget below its count and under an input limit below its size. Update `specs/ir-conformance/README.md` (the table of paths, what a case is, and the sentence that a case uses only arguments every runtime accepts); verify that regenerating changes no expected file of an existing program
- [x] 3.3 Add the program `argument-diagnostics` to `specs/ir-conformance` and to `manifest.json`, with `recordFailures` set and a case for each of: a wrong-typed argument, a wrong field three levels down, a missing required argument, a `Function` record that names no function (each recorded with its argument), a record whose field default divides by zero, a division by zero in the body, a record whose field default fails with `nx-ir-arguments` and a parameter left out whose default fails with `nx-ir-arguments` (each recorded with none), and one that succeeds, fifty records for `items:Item+`, whose recorded budgets run out while the argument is checked; verify the corpus tests of both runtimes pass, and that each fails when its runtime's `argument` is removed for one case, added for the field default, or added to a budget failure, and when the default mark or the exemption of a defaulted parameter the call leaves out is removed
- [x] 3.4 In `runtime/typescript/test/cost-runner.mjs` and `crates/nx-codegen/tests/cost/mod.rs`, record the argument wherever a diagnostic is recorded (the refusal with no budget, under ample limits and at the larger scale, and each entry of `failures`), so `crates/nx-codegen/tests/cost_differential.rs` compares it; verify the test passes, and fails when one runtime names an argument on a budget failure
- [x] 3.5 Run `cargo test --workspace --locked`, `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`; verify all pass

## 4. The agent package

- [x] 4.1 Give `classifyRuntimeFailure` (`packages/agent/src/classify.ts`) the tool's context parameter names and the rule of design decision 3; verify with tests in `test/classify.test.ts`: an argument the model sent (`invalid-input`), a context parameter (`invalid-context`), one of each (`invalid-context`), a boundary code with no `argument` (`evaluation-failed`), an unknown boundary code with an `argument` (`invalid-input`), a resource limit beside any of them (`resource-limit`), and no diagnostics (`evaluation-failed`)
- [x] 4.2 Pass the names from `prepareFunctionCalls` (`packages/agent/src/execute.ts`); verify with tests against a compiled program, for a `function` tool and an `http` tool: a context record missing a required field and one with another `$type` are `invalid-context` with `usage`; a wrong-typed model argument beside a good context is `invalid-input`; and the existing execution tests pass; and a tool whose function takes `req:Req` with `type Req = { n:int label:string = { n } }`, called with a `req` that fits, is `evaluation-failed` (for as long as the checker accepts that default)
- [x] 4.3 Verify through `toAiSdkTools` that a context that does not fit is thrown with the fixed sentence for `invalid-context` and that the thrown error holds no field name of the context record, and that `onResult` is given the diagnostics
- [x] 4.4 Update `packages/agent/README.md` (the result codes table, *Tool context*, and the paragraph of *The AI SDK* that says a context record that does not fit is reported as `invalid-input` for now, which is removed), `runtime/typescript/README.md` and `crates/nx-ir-runtime/README.md` (*Diagnostics*: the diagnostic's members), `docs/nx-ir-format.md` (where it describes `limit` for each runtime) and the host page `sites/website/src/content/docs/reference/libraries/agent-hosts.md`; verify the package's README check and the website's test pass, and that `grep -rn "limit" docs runtime/typescript/README.md crates/nx-ir-runtime/README.md` shows no description of a diagnostic's members that leaves `argument` out

## 5. Records

- [x] 5.1 Add the condition of design decision 5 to `specs/future.md`, under *What a constrained-types change has to hold for the agent package*: a constraint checked on a value the host passed is reported with the argument named; verify the entry names this change and the condition
- [x] 5.2 Add the member and the changed codes to the next release's section of `src/vscode/CHANGELOG.md`; verify it says `@nx-lang/agent` reports `invalid-context` for a context record that does not fit
- [x] 5.3 Run `openspec validate name-the-argument-in-boundary-diagnostics --strict`, `pnpm -r build`, `pnpm -r test` and `pnpm run verify:packages`; verify each exits 0
