## Context

Today every derived result is evaluated each time something asks for it:

- a top-level value declaration's initializer, at every `reference` (`eval_reference` in
  `crates/nx-ir-runtime/src/eval.rs`);
- an environment value's default and the entry module's provides, at every read that reaches them
  (`provided_or_default` in `environment.rs`);
- a formula, at every read that reaches its layer (`source_value`).

`docs/nx-ir-format.md` charges each of those evaluations node by node, and the conformance corpus
pins the counts. In `design-tokens-theme`, one `env.button.radius` read costs 231,969 operations,
nearly all of it rebuilding `standardPalette` and the `color` and `text` defaults.

The environment is a stack of layers in each runtime's per-call state (`EnvState.layers` in Rust,
and the equivalent in `runtime/typescript/src/index.ts`): `Set` layers for overrides and `Cap`
layers pushed while a source of a value is evaluated. Host roots are fixed for the call. A read walks
the stack from the top. The instance tree already records which environment values a body read
(`record_environment_reads` and `take_environment_reads`) so that its skip check can tell whether a
node must render again.

## Goals / Non-Goals

**Goals:**

- One model for what is computed again when data changes, used by component renders and by derived
  values alike.
- Implement the part that removes the token cost now: each derived result is evaluated at most once
  for each environment it depends on, within one call.
- Keep the operation count a function of the images, the input and the tree's held state, identical
  in the Rust and TypeScript IR runtimes.
- Change no result. NX evaluation is pure, so a reused result equals a fresh one.
- Track the reads that actually happened, rather than a set the compiler predicts.

**Non-Goals:**

- Implementing state-read tracking or reuse across passes in this change (Decision 9 designs them).
- Lazy record fields.

## The model

Two kinds of thing take part.

A **source** is data that can change: an environment value, which an override, a host root value,
an entry-module provide or a default supplies; a component's state field; and a prop.

A **tracked computation** is something whose result depends only on the sources it reads, because
NX evaluation is pure: a component render, a top-level value, an environment value's default, an
entry-module provide and a captured formula.

Each tracked computation records the sources it read, at any depth, and what it found. Reading
another tracked computation, evaluated or reused, adds that computation's reads to its own record.
A computation is reused while every recorded read would find the same thing, and is computed again
otherwise. How "the same thing" is checked depends on how long the result is kept:

| Kept | A read compares by | Why |
| --- | --- | --- |
| Within one call | The identity of what supplied it | Free, and needs no value comparison |
| Across calls, in the instance tree | The value found | The overrides of one call are gone by the next |

The tree already follows the second row for environment reads: a node records what its render read
and renders again on a root change only when a read now differs (`set_environment`), and a child
whose descriptor is equal keeps its render. Decisions 1 to 8 implement the first row. Decision 9
extends the second row to state fields and to derived values.

## Decisions

### 1. Within one call, results live in the call's state

A runtime keeps derived results in its per-call state (`Machine` in Rust, the evaluation context in
TypeScript) and drops them when the call returns. A call's count then depends only on its images and
input. A cache hidden on the linked program would make a call cheaper because of unrelated calls
made before it.

Keeping results across calls is still the goal, and the model says where they belong: in state the
host holds and passes back, which the instance tree already is (Decision 9). There, a count that
depends on earlier calls depends on the tree's held state, as a pass already does.

### 2. Which results are cached, and what identifies them

| Derived result | Identified by |
| --- | --- |
| A top-level value's value, checked against its declared type | The declaration (module and index) |
| An environment value's default, checked against its type | The environment declaration |
| An entry-module provide's value, a bundle provide's bundle included | The provide's index |
| A captured formula's value | The layer that holds it |

None of them depends on local slots: values, defaults and provides are evaluated with an empty
frame, and a formula's captured locals are fixed when it is captured. So the only input that can
differ between two uses is the environment. Parameter defaults and field defaults are not cached:
they can read parameters and are cheap in practice.

What is cached is the result of the source, not of the read. A read of `env.color` with
`env.color.accent=red` on top still applies that field override to the cached default each time.

### 3. Overrides have identities, and a read records the one it found

Each layer pushed on the environment stack, the host's own layers included, gets a sequence number
from a per-call counter. A read of value `V` records the *token* of `V`: the sequence number of the
topmost layer that is a `Set` or a `Cap` for `V`, or none when no layer touches `V`.

Because the stack only grows and shrinks at the top, a layer that is still present has the same
layers beneath it as when it was pushed. So two reads of `V` with the same token find `V` from
exactly the same sources, and with none both times they find it from the same root, provides and
default. Comparing tokens costs nothing and does not depend on the size of any value.

While a derived result `S` is evaluated, each read of an environment value, at any depth (inside
other derived results it evaluates or reuses), is added to `S`'s record with its token. For a read
made inside `S`, the token is computed over the layers that existed when `S` began, ignoring the
layers and caps `S`'s own evaluation pushed. Those are the same every time `S` is evaluated in the
same outer environment. Ignoring caps can only make a token change more often than the value it
guards, so it can cause an extra evaluation but never a wrong reuse. Both runtimes follow the same
rule, so they make the same decisions.

### 4. Reuse when every token still matches

At a use of `S`, the runtime looks up `S`'s last result. It reuses that result when, for each
value in the record, the current token equals the recorded one. Otherwise it evaluates `S` again
and replaces the entry. Only the last result is kept for each derived result. Keeping several, one
per environment, would help a program that switches between a light and a dark section many times
in one call, at the cost of memory that grows with the program's output. One entry is simpler and
already removes the repeated reads inside one section.

A reused result's recorded reads are also added to the record of any derived result being
evaluated around the use, and to the instance tree's read list when the call records one. So the
skip check sees the same reads whether a result was evaluated or reused.

A failure ends the call, so no failure is ever cached.

### 5. An override counts as a change whatever its value

A new `envProvide`, `env.` attribute, `$env` entry or bundle entry for `V` gives `V` a new token
even when it sets an equal value. Comparing values would cost operations, and would make the cost
of a read depend on the size of the overriding value. The corpus pins this case.

### 6. The cost of reuse

A reused result costs nothing beyond the node that asked for it: one operation for the `reference`
or `envRead` node. Its nodes are not evaluated, its value is not checked against its type again,
and the comparison of tokens is free. A formula is still charged one operation where it is
captured. The cost table in `docs/nx-ir-format.md` changes where it says a value's initializer is
charged "each time a `reference` evaluates it", and the environment section changes where it says a
default, provide or formula is charged "each time it is evaluated".

### 7. Generated code

Generated JavaScript and TypeScript have no operation counts. Their environment helpers
(`nxEnvRead`, `nxProvidedOrDefault`, `nxFormula`) and the emitted thunks for top-level values that
read the environment (`const tv = () => …`) cache by the same rule. The generated environment
state is the same stack of layers, made by `nxWithEnvironment` for one call, so each layer gets a
sequence number and the cache lives in that state. Component evaluation gets its state the same
way. Values that read no environment are already constants evaluated at load. That is reuse across
calls, and it changes no count anyone can see.

### 8. The tree's records are the model's records

A node's read list, which `set_environment` and the skip check use, is the record of its render.
Reads made inside a derived result that the render reused are added to it from the reused result's
record, with the values found and with the same "found under the call's own overrides" flag that
`take_environment_reads` uses today. So the tree's behavior does not change when a derived result is
reused instead of evaluated.

### 9. Next: state reads and reuse across passes

These steps follow from the model, and are the next change.

- **State fields are sources.** A render records the state fields it read. A dispatch patches some
  fields. The dispatched node renders again only when its render read a patched field. Its children
  are then skipped by descriptor equality, as today. A handler reads state live when it runs, so a
  field read only by handlers never causes a render. A node's render that read no patched field
  keeps its output, and the host's pass finds it settled.
- **Derived results are kept across passes.** The tree keeps the derived results its renders
  evaluated, each with its record of values found, and offers them to the next render of any node.
  A result is reused when every read in its record, looked up at the asking read, finds an equal
  value. That lookup and comparison is charged as the skip check is charged today. So a render after
  a dispatch reuses the palette and the `color` default instead of rebuilding them. The count stays a
  function of the images, the input and the tree's held state, which is already true of a pass.

- **Changes are pushed, not polled.** Every source keeps an index of the tracked computations whose
  records hold it, as the tree's `readers` index already does for root values. A change to a source,
  whether a root value from `set_environment` or a state field a dispatch patched, notifies exactly
  those computations, which become stale. Nothing else is checked. Within one call, records are
  checked when a result is asked for instead, because the check is free there and a call's
  overrides last only for the call.
- **The host gets a change set.** After a dispatch or a root change, the tree tells the host which
  nodes rendered again, which were added and which were removed, so a host updates what it derived
  from those nodes without walking the tree or comparing output. This is the "tree the runtime
  maintains, with a change set" in `specs/future.md`, and `is_settled` remains for hosts that walk.

The TypeScript runtime has no instance tree yet, so these steps are Rust first. When the TypeScript
tree arrives, it follows the same model.

## Risks / Trade-offs

- **Every recorded count changes.** Every corpus program that reads a value or a default more than
  once gets smaller counts. The regeneration is mechanical. The PR's corpus diff shows each change,
  and `design-tokens-theme`'s drop is the measurement this change exists for.
- **Reuse is per call.** A host that renders many components still builds the palette once per
  render. Decision 9 is the next step.
- **One entry per derived result.** A call that alternates between environments evaluates again at
  each switch. That is at most what it costs today.
- **The rule must be exact in both runtimes.** A difference in when a token is taken would show as a
  count difference in the corpus and in the cost differential tests, which both run on every PR.
- **Read records cost memory.** A record holds one entry per distinct value read, bounded by the
  number of environment declarations in the program.

## Migration Plan

None. Hosts see no API change. A host that sets an operation budget sees smaller counts.

## Open Questions

None.
