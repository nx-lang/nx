# Review: add-nx-codegen-js-ts-output

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/executable-code-generation/spec.md, .openspec.yaml
**Reviewed code:** crates/nx-cli/src/codegen.rs, crates/nx-cli/src/main.rs, crates/nx-api/src/artifacts.rs, crates/nx-api/src/eval.rs, crates/nx-types/src/check.rs, crates/nx-value/src/lib.rs, crates/nx-interpreter/src/value.rs, crates/nx-interpreter/src/resolved_program.rs, openspec/specs/cli-code-generation/spec.md, README.md, bindings/dotnet/README.md, openspec/changes/support-stateful-component-value-evaluation/proposal.md

This is a design/pre-implementation review; there is no implementation in the working tree yet, so findings target the proposed design and plan.

## Findings

### 🟡 Fixed - RF1 "Modified Capabilities: None" is wrong — the CLI rename modifies the existing `cli-code-generation` capability
- **Severity:** High
- **Evidence:** The change renames `nxlang generate` -> `nxlang typegen`, but a main spec already governs that command: [openspec/specs/cli-code-generation/spec.md:4-17](openspec/specs/cli-code-generation/spec.md#L4) (`The nxlang generate command SHALL ...`). The proposal lists `### Modified Capabilities` as `None` ([proposal.md:30-31](openspec/changes/add-nx-codegen-js-ts-output/proposal.md#L30)) and only expresses the rename as an "AND ... remain separate" scenario inside the new capability ([spec.md:105-120](openspec/changes/add-nx-codegen-js-ts-output/specs/executable-code-generation/spec.md#L105)). After archiving, the main specs would simultaneously mandate `nxlang generate` (old spec) and `nxlang typegen` (new), which is contradictory.
- **Recommendation:** Add a delta spec that MODIFIES/RENAMES the requirements in `cli-code-generation` (generate -> typegen), and list `cli-code-generation` under the proposal's Modified Capabilities.
- **Fix:** Listed `cli-code-generation` under Modified Capabilities and added a `specs/cli-code-generation/spec.md` delta that renames the types-only command from `nxlang generate` to `nxlang typegen` without preserving a compatibility alias.

### 🟡 Fixed - RF2 Breaking CLI rename has no docs/migration tasks or deprecation decision
- **Severity:** High
- **Evidence:** `nxlang generate` is documented in [README.md:87-95](README.md#L87) and [bindings/dotnet/README.md:468-471](bindings/dotnet/README.md#L468). [AGENTS.md](AGENTS.md) requires updating relevant documentation with changes. No task in [tasks.md](openspec/changes/add-nx-codegen-js-ts-output/tasks.md) updates README/bindings docs or the existing spec, and there is no decision on alias-vs-hard-rename for a user-facing command.
- **Recommendation:** Decide and document deprecation strategy (temporary `generate` -> `typegen` alias for one release, or hard cut), and add explicit tasks to update README.md, bindings/dotnet/README.md, and openspec/specs/cli-code-generation/spec.md.
- **Fix:** Chose a hard rename strategy: `nxlang typegen` replaces `nxlang generate`, and tasks now require README.md, bindings/dotnet/README.md, and CLI documentation updates.

### 🟡 Fixed - RF3 "codegen" naming collides with the existing types-generation module
- **Severity:** High
- **Evidence:** nx-cli already has an internal module named `codegen` that produces **types** ([nx-cli/src/codegen.rs:18-23](crates/nx-cli/src/codegen.rs#L18), `GenerateTypesOptions`, `TargetLanguage`). This change makes crate `nx-codegen` and command `nxlang codegen` mean **executable** output, while DTO/type generation moves to `nxlang typegen`. "codegen" would mean executable at the crate/command level but types in the existing nx-cli module — durable confusion.
- **Recommendation:** Rename the existing nx-cli `codegen` module (e.g. `typegen`) as part of this change, or otherwise disambiguate so "codegen" consistently means executable generation.
- **Fix:** Added an implementation task to rename the existing `nx-cli` internal `codegen` module to a type-generation-specific name such as `typegen`, so `codegen` consistently refers to executable generation.

### 🟡 Fixed - RF4 Including components in a "non-reactive" first pass conflicts with their actual semantics
- **Severity:** High
- **Evidence:** Components are the most reactive-adjacent construct in the language. The interpreter's `Value::ActionHandler` captures lexical state and points at a lowered HIR body ([nx-interpreter/src/value.rs:85-99](crates/nx-interpreter/src/value.rs#L85)), and the parallel [support-stateful-component-value-evaluation](openspec/changes/support-stateful-component-value-evaluation/proposal.md) change shows components carry host-owned state plus emit/dispatch. The design defers all reactivity/subscription/dispatch ([design.md:98-105](openspec/changes/add-nx-codegen-js-ts-output/design.md#L98)) yet still emits "executable components" ([spec.md:45-50](openspec/changes/add-nx-codegen-js-ts-output/specs/executable-code-generation/spec.md#L45)), leaving generated action handlers as dead closures with no dispatch model.
- **Recommendation:** Descope components and action handlers from this pass (land them with the reactivity/state work), or explicitly define what a generated component does with handlers and state in this change. At minimum, note the dependency on the stateful-component change.
- **Fix:** Descoped executable components, component lifecycle APIs, state evaluation, dispatch, and action handlers from v1; the design now defers them to a later component/reactivity pass and tasks require negative tests for rejected component lifecycle/action-handler codegen.

### 🟡 Fixed - RF5 Parity boundary is underspecified and not testable for components/elements as written
- **Severity:** High
- **Evidence:** The interpreter's public eval returns `NxValue` ([nx-api/src/eval.rs:175-180](crates/nx-api/src/eval.rs#L175)), and `NxValue` has only Null/Bool/ints/floats/String/Array/Record — no element, component, closure, or function variant ([nx-value/src/lib.rs:23-44](crates/nx-value/src/lib.rs#L23)). The spec repeatedly asserts parity for "equivalent canonical NX runtime values" including components ([spec.md:45-50,122-133](openspec/changes/add-nx-codegen-js-ts-output/specs/executable-code-generation/spec.md#L122)), but a component/action-handler does not serialize to a comparable value.
- **Recommendation:** Pin the parity comparison boundary to serialized `NxValue`/JSON, and specify exactly what a component/element invocation yields for comparison, so the parity scenarios become implementable as tests.
- **Fix:** Pinned parity to serialized `NxValue` payloads, removed component parity from v1, and limited element support to element expressions that evaluate to serializable `NxValue::Record` shapes.

### 🟡 Fixed - RF6 "Source-mapped" framing is not supported by accessible APIs
- **Severity:** Medium
- **Evidence:** The proposal/design describe a "source-mapped semantic program" and require preserving source spans ([proposal.md:12-13](openspec/changes/add-nx-codegen-js-ts-output/proposal.md#L12), [spec.md:3-10](openspec/changes/add-nx-codegen-js-ts-output/specs/executable-code-generation/spec.md#L3)), but `ProgramArtifact.source_map` is `pub(crate)` ([nx-api/src/artifacts.rs:78](crates/nx-api/src/artifacts.rs#L78)), so a separate `nx-codegen` crate cannot read source text. HIR spans are text ranges only; mapping them to files/source needs the text.
- **Recommendation:** Either expose source access from nx-api, or drop the source-map framing for v1 and scope spans to what is reachable.
- **Fix:** Kept source-map support in scope and added a small `nx-api` requirement/task for read-only `ProgramArtifact` source text lookup and source entry iteration, avoiding direct access to artifact internals.

### 🟡 Fixed - RF7 "Open Questions: None" is not credible for this scope
- **Severity:** Medium
- **Evidence:** [design.md:158-160](openspec/changes/add-nx-codegen-js-ts-output/design.md#L158) records no open questions, yet the change introduces a new IR, two emitters, a runtime surface, and a breaking CLI rename. The parity boundary (RF5), component scope (RF4), and runtime-helper version skew are genuinely open. On the last point, decision 4 emits helpers as local generated files ([design.md:83-86](openspec/changes/add-nx-codegen-js-ts-output/design.md#L83)) without addressing regeneration against a stale/edited helper file.
- **Recommendation:** Record the real open questions (parity boundary, component scope, runtime-helper versioning/skew) so they are resolved deliberately rather than implicitly.
- **Fix:** Replaced "None" with the remaining component lifecycle/action-handler open question; source-map support is now addressed by an `nx-api` source access requirement, and `generate` alias removal is resolved by the hard rename. Also added a runtime-helper version/skew mitigation in Risks.

### 🟡 Fixed - RF8 nx-codegen → nx-interpreter coupling is unclarified
- **Severity:** Medium
- **Evidence:** Task 1.1 has `nx-codegen` depend on `nx-interpreter` ([tasks.md:3](openspec/changes/add-nx-codegen-js-ts-output/tasks.md#L3)). The resolved-program types the model needs (`ResolvedProgram`, `RuntimeModuleId`, `ModuleQualifiedItemRef`) live in nx-interpreter ([nx-interpreter/src/resolved_program.rs:105-233](crates/nx-interpreter/src/resolved_program.rs#L105)), so building the model drags in the whole interpreter runtime.
- **Recommendation:** Clarify whether the interpreter is a build dependency (for shared resolved types) or only a dev-dependency for parity tests, and consider relocating the resolved-program types to a lower shared crate.
- **Fix:** Added a design decision clarifying that `nx-interpreter` is initially a build dependency for shared resolved-program model types, not for executing codegen, with future extraction to a lower shared crate left as a cleanup option.

### 🟡 Fixed - RF9 Two hand-written emitters that must stay value-equivalent is a maintenance trap
- **Severity:** Medium
- **Evidence:** Decision 5 hedges between "JS shares most TS rendering logic" and "JS could be transpiled from TS internally" ([design.md:87-95](openspec/changes/add-nx-codegen-js-ts-output/design.md#L87)), while tasks 6.1-6.3 describe an independent JS emitter validated by target-agreement tests (task 6.4) ([tasks.md:37-40](openspec/changes/add-nx-codegen-js-ts-output/tasks.md#L37)). Two parallel emitters plus an equivalence suite doubles maintenance.
- **Recommendation:** Commit to one strategy — preferably a single emitter that treats type annotations as an optional layer — rather than two emitters kept equivalent by tests.
- **Fix:** Updated the design and tasks to use one shared emitter pipeline with TypeScript mode enabling type-only syntax and JavaScript mode disabling it.

### 🟡 Fixed - RF10 Enum/union encoding for parity is left vague
- **Severity:** Low
- **Evidence:** [nx-value/src/lib.rs:30-35](crates/nx-value/src/lib.rs#L30) notes a String may be a plain string or an enum member, indistinguishable without a schema; the interpreter separately has `Value::EnumValue` and typed `Value::Record` ([nx-interpreter/src/value.rs:65-83](crates/nx-interpreter/src/value.rs#L65)). The spec says "unions/enums where supported by the runtime value model" without pinning the encoding ([spec.md:125-127](openspec/changes/add-nx-codegen-js-ts-output/specs/executable-code-generation/spec.md#L125)).
- **Recommendation:** Specify the exact discriminator encoding the runtime helpers use so TS, JS, and interpreter agree on enum/union representation.
- **Fix:** Added a runtime-helper encoding requirement: enums encode as bare authored member strings, while records and union cases encode as deterministic `$type` record/object payloads matching canonical `NxValue` semantics.

### 🟡 Fixed - RF11 Deterministic output ordering should be an explicit requirement
- **Severity:** Low
- **Evidence:** `Value::Record.fields` is an `FxHashMap` (unordered) ([nx-interpreter/src/value.rs:78-83](crates/nx-interpreter/src/value.rs#L78)) while `NxValue::Record.properties` is a `BTreeMap` ([nx-value/src/lib.rs:40-43](crates/nx-value/src/lib.rs#L40)). The design lists deterministic output as a goal but snapshot tests (tasks 5.4/6.3) and generated record property order depend on a stable ordering that HashMap iteration does not provide.
- **Recommendation:** State deterministic generated-name and property/field ordering as an explicit requirement in the spec.
- **Fix:** Added an explicit deterministic output ordering requirement and tasks/tests for module ordering, declaration ordering, import ordering, generated identifiers, and record/property ordering.

## Questions
- RF4/RF5: Should components be in scope for this change at all given the in-flight stateful-component-value-evaluation work, or should this first pass cover only functions/values/records/elements?
- RF2: Is the `nxlang generate` -> `nxlang typegen` rename a hard cut, or should a deprecation alias be kept for a release?

## Summary
- The core architecture is sound: deriving a target-neutral `CodegenProgram` from `ProgramArtifact` (not raw HIR), an eager/non-reactive first cut, and keeping target-specific decisions in the emitters are all the right calls and align with the existing pipeline.
- The most important fixes before implementation are the OpenSpec coherence gap on the CLI rename (RF1/RF2), the "codegen" naming collision (RF3), and reconsidering component scope plus the parity boundary (RF4/RF5), which together are in tension with the deferred-reactivity stance. The remaining findings are medium/low refinements to the plan.
