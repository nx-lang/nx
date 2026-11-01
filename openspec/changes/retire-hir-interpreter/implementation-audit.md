# Implementation audit

## Implementation and accepted exceptions

Started from proposal commit `be7f870`; the planning review is preserved as `proposal-review.md`.
The user approved retirement before the two compiler gaps are fixed. The separate change
`complete-ir-construction-lowering` contains their proposal, design, specs and tasks. Six source
regressions retain their intended successful results and are ignored with that change named:
four conditional-property scenarios, positional record construction (including omitted defaults),
and the component example which contains conditional properties. Nine older ignored array-parser
cases keep their original reasons. No other new semantic regression is ignored.

The formatter owns its value model. Canonical host values and the runtime's read-only rich function
result view feed the same annotated writer. Cases/functions survive at object sites; float32
spelling follows declared sites, and object-site float32 uses its float64 widening. Neither
formatting API executes source. CLI `run` emits all modules, links the selected entry and uses IR
with a fresh 1,000,000-operation budget. Library defaults and playground limits are unchanged.

The Rust/C/.NET/Node/WASM legacy execution adapters are removed, including opaque snapshot binding
transport. Existing IR instance storage is unchanged. C/.NET get no replacement execution binding.
Native ABI is 14; WASM ABI is 12. Complete breaking removals and migration examples are in
`docs/hir-interpreter-retirement.md`.

## Audit gates and consumers

`refactor-tooling-api` is merged (`471da37`); the Rust runtime is present. A fresh source search of
`/home/bret/src/reachme` and `/home/bret/src/DrawnUi.FiddleEngine` found no active callers of removed
methods. A retired FiddleEngine `nx/retired/s5/nx-runtime.js` bundle has historical exports.
The NX playground already emits IR, executes the TypeScript runtime, then formats canonical values.
No new application caller was found. This is a local audit, not a claim about every external user.

Emitter audit: builder rejection paths cover malformed/unchecked HIR, unresolved semantic data,
handlers outside properties, missing/unknown arguments, abstract construction, and conditional
property fragments. The first groups are checked-source/invalid-input guards. IR-specific rejection
of let statements, statement blocks and index expressions concerns syntax NX cannot write. Only
conditional properties and positional record constructors are approved source gaps; their intended
behavior is captured in ignored tests. The former already has an explicit codegen diagnostic; the
latter currently reports `nx-ir-reference` on execution, documented by the follow-up.

## Preserved behavior and reviewed differences

- The original API suite passed before migration, including its temporary HIR/IR differential
  suite. The complete original interpreter suite also passed before deletion. Logs are in
  `target/retire-validation/baseline-api.txt` and `hir-final-baseline.txt`.
- Source tests now compose build/emit/prepare/link only in integration-test support. No production
  source-execution convenience API was introduced. Lower runtime crates stay compiler-independent.
- `source_semantics` has 384 active passing tests and 15 ignored cases (six approved deferred,
  nine pre-existing parser ignores). Existing compiler/runtime/corpus expectations remain active.
- Lifecycle coverage includes state transitions, update validation at nested props/action/state
  sites, tokens, stale invocations, atomic batches, effects and function-valued props. Old tests
  that injected captured handler bodies through interpreter Values are internal to a deleted API;
  IR parent-instance token/effect routing is covered by existing runtime and instance-tree tests.
- Migration found unchecked nested update records under a component-typed prop. Both IR runtimes
  now recursively validate descriptors naming their own nominal component declaration. Rust and
  TypeScript source regression tests cover valid, unknown-field and invalid-empty updates. Existing
  cross-type component compatibility is unchanged. An intrinsic element with both `content=` and
  body content is rejected by the compiler, preserving the HIR diagnostic behavior.
- HIR eager prebinding passes and raw unchecked-HIR calls are implementation details. Checked
  source tests use current annotations and static diagnostics; a referenced failing global still
  fails. Unknown intrinsic records may carry explicit empty fields, while typed optional fields
  preserve their canonical omission rules.
- Error tests now expect `nx-ir-*` diagnostics, source spans in the failing module when present,
  and declaration-only fallback otherwise. No fabricated missing-source coordinates are asserted.
- Arithmetic uses existing IR carriers, with no special int32 wrap. Existing int32 boundary range
  validation still applies: returning an out-of-range result as int32 can fail, while returning it
  as int exposes the wider arithmetic result. Tests cover 2^31, 2^53 and native i64 wrapping.

## Coverage accounting

All original interpreter cases are individually listed in `test-inventory.json`; each is marked
ported, deferred, redundant or interpreter-internal with its coverage/reason. Counts:
- interpreter-internal: 119
- ported: 395
- deferred: 6
- redundant: 33

`api-test-inventory.json` accounts for old API unit tests. `binding-test-inventory.json` records
removed binding overload/envelope tests. DTO, JSON/MessagePack serialization, update helpers,
construction, diagnostics, schema, IR emission and formatting tests remain against retained APIs.
Raw test count parity is not the coverage gate.

## Corpus review

Only two expected-result files changed: `conversions/expected/results.json` replaces four integral
float spellings (`3.0`) with `3`; `occurrence-lifting/expected/results.json` replaces `1.0` with `1`
for `presentWidened`. These reflect existing IR numeric carriers and preserve numeric value.
No image, operation-count, diagnostic, lifecycle, nonnumeric result or origins golden was changed.
Generation now executes directly through the Rust IR runtime; handwritten expected-value tests and
generated-JavaScript/TypeScript comparisons remain independent checks on the emitter.

## Performance and size

Fifteen fresh processes per fixture, median elapsed time, same debug profile; baseline built from
`be7f870` in an isolated checkout. These small local measurements are indicative, not a release
benchmark. All cases exited successfully with identical output.

| Fixture | HIR baseline | IR cutover |
| --- | ---: | ---: |
| small.nx | 2.69 ms | 3.93 ms |
| large.nx | 32.89 ms | 38.59 ms |
| multi.nx | 6.65 ms | 7.40 ms |

The large fixture has 300 function declarations; the multi-module fixture imports standard math.
All ratios are below the design's 2x explanation threshold. Initial wasm-release SDK measurement:
4,582,516 bytes before, 4,333,438 bytes after adapter removal; final deletion/build is remeasured
in the validation report. Scripts/data/logs are in ignored `target/retire-validation/`.

## Specs and validation

All delta requirements have been synced, including the intended scenario-title updates and removal
of the retired component-binding capability. Purpose paragraphs were updated directly. Strict
validation passes all 95 main specs and both changes. Remaining main-spec references to the retired
engine describe its removal or explicitly prohibit the old APIs; no requirement mandates it.
The pulled proposal's scenario-title validator errors are resolved by syncing those reviewed changes.

Pre-deletion checks passed: full API suite, original interpreter suite, eight rich-result/formatting
integration tests, 271 CLI tests, generated code/corpus suite, TypeScript runtime suite, targeted
and workspace clippy with warnings denied. Final workspace, binding, packaging, platform checks
and independent Claude review are recorded in `validation.md` and `review.md` when completed.
