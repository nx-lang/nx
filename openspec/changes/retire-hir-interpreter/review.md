# Review: retire-hir-interpreter

Reviewed 2026-11-01 against main at 471da37 (PR #68 merged). This is a planning review of the
proposal, design, tasks and deltas; no implementation exists yet. The earlier review of the
superseded "keep every execution API" plan is in git history. All findings below were fixed in the
change itself.

## Verdict

Retiring the HIR interpreter is the right direction. Every real host (playground, ReachMe,
FiddleEngine, graphics preview) already runs IR, so the interpreter mostly runs the CLI, legacy SDK
wrappers and tests. It carries its own copy of the language semantics (`nx-interpreter` has 21
test files and 553 tests) and it already disagrees with the IR runtimes, e.g. the aliased-import
failures in `specs/future.md` that the IR runtime does not share. One evaluator means every
language feature is built once, and the CLI exercises the emitter on ordinary use.

What is lost: an oracle independent of the emitter (two IR runtimes running the same wrong IR
agree), a slightly faster cold CLI run, and .NET/C execution. The plan answers the first with
handwritten expectations, reviewed corpus goldens and the generated-JavaScript comparison; the
second is measured; the third is a stated non-goal. Removing the APIs rather than re-implementing
them on IR is also right under the repository's no-backward-compatibility rule.

## Findings (all fixed)

- **RF1 `rust-crate-layering` still names the interpreter.** The spec PR #68 added requires
  `nx-api` to depend on `nx-interpreter` and has scenarios about it; the change had no delta, so
  archiving would leave a requirement the deletion breaks. Added a MODIFIED delta, and task 6.1 now
  removes `nx-interpreter` from `scripts/check-crate-layering.mjs`.
- **RF2 About 45 requirements in 23 other specs name the interpreter.** Most say "the interpreter,
  the TypeScript IR runtime and every code generation target SHALL agree"; some make it the actor
  (`external-components`, `component-action-handlers`, `artifact-model`, `value-constraints`).
  Task 6.2 only promised a later "sweep". Added MODIFIED deltas naming the Rust IR runtime, removed
  the interpreter-only `value-constraints` requirement, and reflowed four requirements so SHALL is
  on the first line. Also fixed `occurrence-types` and the Node lifecycle requirement, which name
  removed SDK methods.
- **RF3 Direct IR input to `nxlang run` is new scope.** `--input-format ir` and `--ir-module`
  catalogs are a feature with their own design questions, not part of retiring an engine, and no
  caller needs them. Removed from the change and recorded in `specs/future.md`.
- **RF4 Two formatting paths, and a float32 kind only for the CLI.** The design proposed a separate
  rich-result formatter and float32 provenance in the runtime. Today one annotator already serves
  both, fed by an interpreter `Value`. The design now keeps one formatter with a formatter-owned
  model, fed from canonical JSON or a read-only runtime view that keeps constant cases and function
  references (the runtime already has both). A `float32` is spelled by its declared type; at an
  `object` site it prints as its `float64` widening, a declared difference.
- **RF5 Stale references.** `support-stateful-component-value-evaluation` exists in neither
  `openspec/changes` nor the archive; references removed. The `sdk-wasm` deltas carried scenarios
  from earlier changes ("no new export is needed", ABI "8 and 7") that contradict this change's
  ABI bump; replaced with a 12-versus-11 scenario.
- **RF6 Open questions left without answers.** Performance criteria, CLI flags and the result
  interface were open. Settled: flags dropped (RF3), interface defined (RF4), and a cold run more
  than 2x slower or a WASM SDK more than 5% larger needs an explanation before deletion.

## Validation

`openspec validate retire-hir-interpreter --strict` passes.
