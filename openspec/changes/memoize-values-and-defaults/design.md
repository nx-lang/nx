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

- Evaluate each derived result at most once for each distinct environment it depends on, within
  one call.
- Keep the operation count a function of the images and the input alone, identical in the Rust
  and TypeScript IR runtimes.
- Change no result. NX evaluation is pure, so a reused result equals a fresh one.
- Track the reads that actually happened, as the instance tree does, rather than a set the compiler
  predicts.

**Non-Goals:**

- Reuse across calls (see Decision 1).
- Lazy record fields, and reads of component state (see the proposal).

## Decisions

### 1. The cache lives for one call

A runtime keeps derived results in its per-call state (`Machine` in Rust, the evaluation context in
TypeScript) and drops them when the call returns. A call's count then still depends only on its
images and input. A cache kept on the linked program would make a call cheaper because of calls
made before it, and two hosts that make the same call would be charged differently.

*Alternative:* keep results on the program and charge each call, at its first use, the operations
the first evaluation cost. That keeps counts deterministic and saves time across calls, but a budget
failure would then fall at a different place than a real evaluation's failure, and a recorded cost
has to be stored with each result. It is the natural next step if per-render palette builds show up
in profiles. It is recorded in `specs/future.md`.

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

## Risks / Trade-offs

- **Every recorded count changes.** Every corpus program that reads a value or a default more than
  once gets smaller counts. The regeneration is mechanical. The PR's corpus diff shows each change,
  and `design-tokens-theme`'s drop is the measurement this change exists for.
- **Reuse is per call.** A host that renders many components still builds the palette once per
  render. Decision 1 records the next step.
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
