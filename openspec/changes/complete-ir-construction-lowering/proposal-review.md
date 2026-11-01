# Proposal review

Reviewed against the existing `source-analysis-pipeline`, `property-list-fragments`,
`content-properties`, `executable-code-generation` and temporary retirement requirements.
This is an implementation-agent proposal review, not an independent implementation review.

Changes made during review:

- Record construction is a deliberate source-language restriction, diagnosed in shared analysis;
  it is no longer an emitter feature or a runtime error. Guidance uses resolved declarations and
  preserves source identity, with a safe fallback when a complete rewrite cannot be suggested.
- Ordinary functions, factories and the existing named interface for first-class function values
  are explicitly outside the restriction. No broad call-style redesign is bundled into this work.
- Conditional lowering retains the established path-sensitive rules: same-name bindings in
  mutually exclusive branches are valid, while possible active duplicates and missing required
  fields are errors. Content and compile-time type-argument rules remain intact.
- Evaluation order, laziness, absence/default distinctions, multiple fragments and emitted-size
  growth have explicit design/test gates. Existing checker path enumeration must be audited;
  compact lowering must not multiply its cost or change the checker rules silently.
- The IR representation is deliberately undecided. Any required extension must be specified and
  versioned, with both runtimes and generated JavaScript covered, before implementation proceeds.
- The old main-spec exception requiring all six tests to keep success assertions is explicitly
  retired. Five conditional/example tests become active successes; the positional test becomes
  active rejection and equivalent markup/default coverage. Archived inventories remain historical.

Artifact consistency pass:

- All four core artifacts (proposal, design, capability deltas and tasks) use the new rejection
  decision; no artifact still asks the compiler to implement positional record constructors.
- Added a six-test migration table so replacement versus re-enabling is explicit.
- Made cross-target conditional coverage, missing required properties and static checking of
  inactive branches explicit in executable-generation scenarios.
- Added source-analysis scenarios for safe fallback guidance, callable-name resolution and valid
  type references; tightening constructors does not ban record type names in other contexts.
- Main specs remain unchanged until implementation and sync; the removal delta explicitly closes
  the previous temporary exception. No implementation task is marked complete.

Validation: `openspec validate complete-ir-construction-lowering --strict` passes. No compiler,
runtime, tests or main specifications were changed during this proposal revision.
