## Context

The shared codegen builder rejects conditional property fragments and currently treats a positional
record constructor as a function call. These are separate issues: conditional fragments keep their
existing language semantics; implicit positional record construction is intentionally removed.
The runtime already supports conditional expressions and named record construction, but that alone
does not establish a lazy, compact representation for conditional property presence.

## Goals / Non-Goals

**Goals:** Diagnose record calls during shared analysis; preserve named record construction; execute
conditional fragments consistently; replace every temporary ignore with an active test.

**Non-goals:** Restore source-evaluation APIs or the interpreter; introduce positional or tuple
records; change function declaration/call styles, first-class function types, factory functions,
component invocation rules, or record defaults/inheritance semantics.

## Decisions

### Reject record calls at the shared semantic boundary

A call such as `User("Bob", 42)` receives `record-construction-requires-element` during shared
source analysis, before emission or execution. This includes zero-argument calls and calls through
imported, qualified or type-aliased names resolving to a named record. Resolve the actual callee;
do not infer constructor status from capitalization or the result type. Existing function and
lexical-shadowing rules remain authoritative.

Use the callee/call span and retain the caller's file identity. A short message is sufficient:
"Records require element syntax with named properties, such as `<User ... />`."
The example is illustrative syntax, not a complete replacement expression. Use the resolved record
name when readily available; the message must not require enumerating fields, mapping positional
arguments, extracting argument source text, or synthesizing a rewrite/code action. The diagnostic
is the same whether arguments are supplied, omitted or invalid. Do not cascade into misleading
missing-function or runtime diagnostics.

Checker-backed tooling, CLI and all codegen paths must share this rejection. Named construction,
including imported, inherited, generic and defaulted records, must continue to work. An explicit
factory such as `let user(name:string, age:int): User = <User name={name} age={age} />` remains valid.
The record declaration must not be emitted as a function reference to work around the rejection.

### Preserve conditional property presence, laziness and order

Follow `property-list-fragments` and `content-properties`: only the selected branch contributes
bindings; condition lists choose the first matching arm; match bindings keep lexical narrowing.
Conditions, active property expressions and body/content expressions retain their authored order
and are evaluated once where source semantics require it. Inactive guards/arms must not perform
unneeded evaluation. Laziness applies to execution, not static checking: inactive branches must
still be analyzed for invalid names and types under the existing rules. Defaults are applied after active bindings have been selected. Omission and
an explicitly supplied empty value remain distinct.

Keep path-sensitive static checking: reject a required property missing on a possible path and
reject duplicate bindings that can coexist. Reusing a property in mutually exclusive branches is
valid. Preserve existing rules for content properties and compile-time type-argument bindings;
this change must not make conditional type arguments newly legal.

Before coding, document how the representation handles absence, mixed direct/conditional entries,
multiple fragments, targets (intrinsics, records, unions, components and functions) and content.
Do not lower an omitted binding as an empty value or evaluate both branches. Avoid a Cartesian
product of constructions: emitted image/code growth must be linear in fragments and bindings.
Audit the checker's existing property-path enumeration separately so lowering does not compound
analysis growth or silently change validation rules.

Use existing operations if they express these semantics. If an IR feature is needed, update the
proposal's affected capabilities and specify/version the feature first, including preparation and
execution in both runtimes, old-runtime refusal and generated-JavaScript behavior. A schema change
is neither assumed necessary nor ruled out in advance.

### Replace temporary tests according to the language decision

There are six ignores naming this change. Activate the four conditional-fragment tests and the
component example test with their successful expectations. Replace the one positional constructor
test with active static-diagnostic coverage plus markup success/default coverage. Preserve the
historical success expectation in the archived retirement inventory; do not rewrite that history.
Rust/TypeScript parity supplements handwritten values and diagnostic expectations.

| Existing regression | Planned outcome |
| --- | --- |
| `simple_functions::test_paren_call_constructs_record_type_positionally` | Replace with diagnostic assertions and equivalent named construction/default tests |
| `conditionals::test_property_fragment_on_a_declared_record_is_kept` | Enable unchanged success expectation |
| `conditionals::test_property_fragment_simple_conditional_selects_then_and_else` | Enable unchanged success expectation |
| `conditionals::test_property_fragment_condition_list_selects_first_matching_arm_and_else` | Enable unchanged success expectation |
| `conditionals::test_property_fragment_match_selects_matching_arm` | Enable unchanged success expectation |
| `update_records::the_component_example_counts_and_resets` | Enable unchanged lifecycle expectations |

These modules live under `crates/nx-api/tests/semantics/`. New diagnostic tests should also cover
source-analysis and language-service entry points, rather than relying only on execution helpers.

Remove the temporary runtime-spec exception through this change's delta on completion. Update the
migration guide and future-work notes to state the permanent record syntax rule, rather than
removing useful migration guidance or promising future positional support.

## Risks / Trade-offs

- Positional convenience is lost; explicit factory functions retain opt-in positional interfaces.
- Callee classification must respect semantic resolution; keep the message simple and avoid
  introducing field-mapping or rewrite machinery for an uncommon mistake.
- Conditional presence/content may need an IR extension; settle/version it before runtime edits.
- Eager branches, duplicated evaluation, premature defaults or exponential expansion can change
  behavior or cost; cover failing inactive branches, ordering, nesting and image/code growth.

## Migration Plan

Implement shared rejection and conditional lowering, activate/replace the six tests as described,
run checker/tooling and cross-backend suites, then sync specs and remove temporary-limit wording.

## Selected representation

Use `conditional-properties-v1`, a versioned extension within schema 6. A property list entry
`[string-index("$spread"), node]` spreads the fields of the intrinsic element value produced by `node`; ordinary entries
remain `[string, node]`. Spread nodes are ordinary lazy `if`/`match` expressions whose branches
construct compiler-owned intrinsic binding bags. Nested fragments recursively spread bags. Empty
bags mean absence; a present field whose value is empty remains present until target normalization.
Evaluate entries in authored order once, then body content, then existing target defaults and
normalization. Records, union payloads, components and named calls use the same property reader.
Declared functions with fragments use named calls, preserving their declaration's defaults.
Generated JavaScript materializes the same ordered bags before target construction. Binding bags
never escape as user values. No Cartesian construction expansion is needed.

Checker audit: `property_paths_for_entries` currently forms a Cartesian product for path-sensitive
validation. This implementation does not change that analysis or enumerate those paths again during
lowering. Size tests isolate emitted growth; they do not claim linear checker complexity.
