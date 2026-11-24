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

- One model for what is computed again when data changes, and how the host learns of it, used by
  component renders and by derived values alike.
- Each derived result evaluated at most once for each environment it depends on, within a call and,
  in the tree, across renders.
- A dispatch or a new descriptor renders a body again only when something it read changed.
- Changes pushed to exactly the computations that read them, and reported to the host as a change
  set.
- The operation count a function of the images, the input and the tree's held state, identical in
  the Rust and TypeScript IR runtimes.
- No result changes. NX evaluation is pure, so a reused result equals a fresh one.

**Non-Goals:**

- A tree that walks rendered output itself; an instance tree in the TypeScript runtime; lazy record
  fields; anything finer than a component body.

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
| Across calls, in the instance tree | Revisions of root values and of other kept results, with equality cutoff; the value found for a read an override supplied | Revisions are free to compare, and an equal result stops a change from spreading |

The tree already follows the second row for environment reads: a node records what its render read
and renders again on a root change only when a read now differs (`set_environment`), and a child
whose descriptor is equal keeps its render. Decisions 1 to 8 implement the first row. Decisions 9
to 13 extend the second row to state fields, props and derived values, with revisions and equality
cutoff, and add notifications and the change set.

## Decisions

### 1. Within one call, results live in the call's state

A runtime keeps derived results in its per-call state (`Machine` in Rust, the evaluation context in
TypeScript) and drops them when the call returns. A call's count then depends only on its images and
input. A cache hidden on the linked program would make a call cheaper because of unrelated calls
made before it.

Keeping results across calls is still the goal, and the model says where they belong: in state the
host holds and passes back, which the instance tree already is (Decision 11). There, a count that
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

### 9. State fields are sources, and a dispatch follows them

A render records the state fields it read, by name, on the instance. A dispatch runs its handlers
as today, with live state, and then compares the state it ends with against the state it began with,
field by field, charged as `diff` compares fields. The body renders again only when a field it read
differs. Otherwise the instance takes the new state and keeps its rendered output and handlers. The
dispatch result says whether it rendered, so the tree leaves a node that did not render settled.

Both IR runtimes have the component lifecycle, so both do this, with equal counts. A stored
instance carries its state read record, and a restored one without a record renders on every
dispatch, which is what happens today.

A handler bound by the body always runs against the instance that made it, and reads that
instance's state live when it runs (`component-action-handlers`), even when a child emits the action
that triggers it. So the state a handler captured is never read, and keeping the old handlers is
sound: a field read only by handlers never needs a render.

### 10. Props are sources, and a new descriptor follows them

A render records the props it read. When the tree visits a node with a descriptor that differs, it
renders again only when a recorded prop, or a prop read by a handler the body bound, differs, by
value. The runtime finds the slots a handler node reads as it finds a formula's locals
(`slots_read`). Otherwise the node takes the new descriptor and keeps its render, as it already does
for a change that touches only `$env`. The comparison is charged as `same_descriptor` is charged today,
over the recorded props only.

A content parameter counts as one prop. Handler props bound for emits are rebound as today and are
not reads.

### 11. The tree keeps derived results across renders, checked by revision

The tree keeps the derived results its renders evaluated and offers them to later renders of any
node. Checking a kept result by looking up and comparing every value it read, at every use, would
cost a lookup and a comparison per read. Instead the tree follows the revision scheme of incremental
engines such as Salsa:

- **A revision counter.** The tree has one counter. Each root value that `set_environment` changes,
  and each kept result whose value changes when it is evaluated again, takes the next revision.
- **Two revisions per kept result.** `changed` is the revision at which its value last changed, and
  `verified` the revision at which it was last found valid.
- **Direct inputs.** A kept result's record holds its direct inputs, not the leaves under them:
  the root values it read (by name), the other kept results it used (by edge), and the values that
  an override supplied (by value, see below).
- **Validity.** A kept result is valid at the current revision when every root value and every kept
  result it used has `changed` no later than its `verified`, and every override-supplied read
  finds an equal value at the use. Checking a used result first validates that result, recursively,
  so a change is found from the inputs up. Comparing revisions costs nothing and is bounded by the
  program's declarations.
- **Equality cutoff.** A kept result that is not valid is evaluated again. Its new value is then
  compared with the old one, charged as a comparison of values. When they are equal, the tree keeps
  the old value and leaves `changed` alone, so nothing that used it is evaluated or rendered again.
  So when `locale` changes and a `direction` result evaluates again to the same `ltr`, the results
  and renders that read only `direction` stay valid.
- **Override-supplied reads compare by value.** A value that a `$env` entry or an `envProvide` above
  the node supplies is not a tree-wide source, since it differs from place to place. Such a read is
  compared by value at the use, charged as the skip check's reads and comparisons are. Those values
  are usually small, such as `dark`.
- **Variants.** A result used at places whose override-supplied reads differ, such as a light and a
  dark section, keeps one variant for each, chosen by those reads. A variant no render used during a
  pass is dropped when the pass finishes, so variants are bounded by the output.

Within a render, a result taken from the tree enters the call's own cache, and Decisions 1 to 4
apply from there. A render's count now depends on what the tree holds, which a pass's count already
does, and a host that wants a count independent of history evaluates outside the tree.

### 12. Changes are pushed, then pulled

Every source keeps an index of the tracked computations that read it directly: the tree's `readers`
index for root values, extended to kept results; reverse edges from each kept result to the kept
results and nodes that used it; the instance's own record for state fields; and the node's own
record for props.

`set_environment` gives each changed root value a new revision and follows the indexes to mark the
nodes that could be affected as possibly stale, transitively. That walk visits only what the indexes
name and evaluates nothing. Then, in depth order, the tree pulls: for each possibly stale node it
validates the kept results the node used, evaluating again only those that are not valid, with
equality cutoff. It renders the node again only when one of its direct inputs changed. A node whose
inputs all came back equal is left alone and stays settled.

A dispatch decides from the instance's own state record (Decision 9). Within one call there are no
notifications: a call's records are checked when a result is asked for, because that check is free.

### 13. The host gets a change set

The tree keeps a change set: the keys of the nodes that rendered again, that were added and that
were removed since the host last took it. `take_changes` returns it and clears it. A node that was
added and then removed before the host took the set appears in neither list. A node that rendered
several times appears once. A host that derives something from nodes, such as native controls or an
index, updates exactly those and needs no walk. `is_settled` stays for hosts that walk, and the two
agree: a node is unsettled exactly when it or a node under it is in the rendered or added lists of
changes not yet walked.

## Risks / Trade-offs

- **Every recorded count changes.** Every corpus program that reads a value or a default more than
  once gets smaller counts. The regeneration is mechanical. The PR's corpus diff shows each change,
  and `design-tokens-theme`'s drop is the measurement this change exists for.
- **Counts depend on the tree's history.** A render's count includes the checks of kept results
  and excludes what they save, so the same render costs less in a warm tree. This is already true of
  a pass, and is stated in the cost rules.
- **Recording props and state reads touches every render.** The records are small (bounded by the
  component's declared props and state fields) and replace work the dispatch and visit skip.
- **Handler reads must be exact.** A handler whose slots are recorded wrongly could keep stale
  captures. The compiler's handler slot analysis is the source, and the corpus's handler programs
  run with the skip on.
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
