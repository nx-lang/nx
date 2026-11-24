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
- A dispatch in the tree or a new descriptor renders a body again only when something it read
  changed.
- Changes pushed to exactly the computations that read them, and reported to the host as a change
  set.
- The operation count a function of the images, the input and the tree's held state, identical in
  the Rust and TypeScript IR runtimes.
- Reuse changes no result. NX evaluation is pure, so a reused result equals a fresh one. The one
  difference is in limits: reuse does less work, so a call may stay under a depth or operation
  limit that a fresh evaluation would reach (Decision 4). The narrowed handler capture
  (Decision 10) is a deliberate change of results: two handlers that differ only in a slot they do
  not read are now equal under `==`.

**Non-Goals:**

- A tree that walks rendered output itself; an instance tree in the TypeScript runtime; lazy record
  fields; anything finer than a component body; a dispatch skip in the bare lifecycle APIs.

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
| Across calls, in the instance tree | Whether an override supplies it at the use; then the revision of the root value or kept result behind it, with equality cutoff, or the value an override supplies | Revisions are free to compare, and an equal result stops a change from spreading |

The tree already follows the second row for environment reads: a node records what its render read
and renders again on a root change only when a read now differs (`set_environment`), and a child
whose descriptor is equal keeps its render. Decisions 1 to 7 implement the first row. Decisions 8
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
| A targeted entry-module provide's value | The provide's index |
| A bundle entry-module provide's bundle | The provide's index and the value being read |
| A captured formula's value | The layer that holds it, within the call only |

None of them depends on local slots: values, defaults and provides are evaluated with an empty
frame, and a formula's captured locals are fixed when it is captured. So the only input that can
differ between two uses is the environment. Parameter defaults and field defaults are not cached:
they can read parameters and are cheap in practice.

A bundle provide (`target: None`) is evaluated inside a cap for the value being read
(`provided_or_default`, the bundle arm: `self.capped(key, Below::Provide(index), …)`). So the one
bundle expression is evaluated under `Cap{gap}` for a read of `gap` and under `Cap{scale}` for a read
of `scale`, and when it reads a value it sets, the two give different bundles. Keying the bundle by
the provide's index and the value read makes each entry's own cap part of its identity
(Decision 3). A targeted provide is evaluated only for its target, a default only for its value and
a formula only for its layer's value, so their caps are already fixed by their identity. A formula
among an entry-module bundle's entries is held by no layer, so it is not a derived result of its
own: it is evaluated at each read, from the bundle that is reused. In the instance tree its reads
are direct inputs of the read that evaluated it (Decision 8).

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

**Where an evaluation begins.** A derived result `S` begins at the stack as it stands when `S` is
asked for, before `S`'s own cap is pushed. While `S` is evaluated, each read of an environment value,
at any depth (inside other derived results it evaluates or reuses), is added to `S`'s record with
its token computed over the layers below that position only. The layers above it are the ones `S`'s
own evaluation pushed: first its own cap, which its identity fixes (a default's
`Cap{V, exhausted}`, a targeted provide's `Cap{V, provide i}`, a bundle's `Cap{V, provide i}` for the
value `V` it is keyed by, a formula's `Cap{V, layer}`), then the layers its nested evaluations push
and pop. Those are a function of `S`'s identity and of what the layers below give, so they are the
same at every evaluation of `S` whose tokens below match. A cap below the start, pushed by an
enclosing evaluation, is a layer like any other: it has its own sequence number, and a read it caps
records that number. Both runtimes take tokens at the same position, so they make the same
decisions.

A bundle that reads a value it sets shows the rule at work. With
`provide env with makeTheme(env.scale)` in the entry module, setting both `scale` and `gap`, a read
of `gap` evaluates the bundle keyed by `gap` under `Cap{gap}`. Inside, `env.scale` finds no layer,
so it reaches the provides, which evaluate the bundle keyed by `scale` under `Cap{scale}`. Inside
that, `env.scale` meets its own cap and falls through to the default. The `scale` entry gives
`scale`, and the `gap` bundle is built from it. Both entries record `scale` with token none, because
their own caps are above their start. A later read of `scale` reuses the `scale` entry, which was
built from the default, and a later read of `gap` the `gap` entry, which was built from the
provided `scale`. Each gives what a fresh evaluation gives. Had the bundle been keyed by index alone,
the read of `scale` would reuse the bundle built for `gap`.

### 4. Reuse when every token still matches

At a use of `S`, the runtime looks up `S`'s last result. It reuses that result when, for each
value in the record, the current token equals the recorded one. Otherwise it evaluates `S` again
and replaces the entry. Only the last result is kept for each derived result. Keeping several, one
per environment, would help a program that switches between a light and a dark section many times
in one call, at the cost of memory that grows with the program's output. One entry is simpler and
already removes the repeated reads inside one section.

A reused result's recorded reads are also added to the record of any derived result being
evaluated around the use, and to the instance tree's read list when the call records one. Their
tokens are computed again against the start of each record they are added to, never copied, since a
token depends on where the record's evaluation began. Computing them walks the stack and is free.
So the skip check sees the same reads whether a result was evaluated or reused.

A failure ends the call, so no failure is ever cached.

Reuse means less work, so a call may stay under a limit a fresh evaluation reaches. A default is
evaluated at the depth of the read that first needed it (`depth: cx.depth`), and a deeper read that
reuses it does not evaluate it again, so it can succeed where evaluating there would exceed the
depth or native-stack limit. The same holds for the operation budget. Both runtimes reuse alike, so
they reach the same limits, and the corpus pins counts, not limits of fresh evaluation.

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
read the environment (`const tv = () => …`) cache by the same rule, a bundle provide keyed by its
index and the value read. The generated environment state is the same stack of layers, made by
`nxWithEnvironment` for one call, so each layer gets a sequence number and the cache lives in that
state. A generated function called outside `nxWithEnvironment` runs in the module's own state,
which has no roots and whose layers are numbered for its whole life. Its kept results stay valid
from one such call to the next, since every token they hold still means what it meant, and a
formula's result is held weakly by its layer, so it goes when the layer does. This is reuse across
calls that changes no result, as the constants evaluated at load already are. Component evaluation gets its state the same way. Values that read no environment are already
constants evaluated at load. That is reuse across calls, and it changes no count anyone can see.

Since generated code counts nothing, a test cannot see from a result that a default was evaluated
once. The generated helpers carry a test-only evaluation counter, off unless the test harness turns
it on, that counts each evaluation of a default, provide, formula or top-level value thunk. The
`executable-code-generation` tests read it.

### 8. The tree's records hold direct inputs

A node's record and a kept result's record (Decision 11) have one form, so `set_environment`, the
skip check in `visit` and a dispatch validate them by one rule. A record holds the evaluation's
direct inputs, in the order it first read them:

- **A root read** of value `V`: a read that no layer of the place sets or caps. It holds the
  revision of `V`'s root value, and, when no root value was set and a provide or the default
  supplied `V`, an edge to that kept result.
- **A kept result used**: an edge to the variant of a top-level value, default or entry-module
  provide that the evaluation used, not through a root read.
- **An override-supplied read** of `V`: a read that a layer of the place sets, wholly or at a field
  path. It holds the value found.

The *place* of a node is its environment: the layers above it and its descriptor's `$env`, which
are the host layers of its calls. The place of a kept result is the place where it was used, since
the tree keeps only results whose reads were all found beneath the render's own overrides
(Decision 11).

Reads made inside a kept result the evaluation used are not direct inputs: the edge stands for them
in validation. So when `locale` changes and `direction` evaluates again to `ltr`, a node that read only
`direction` sees an unchanged edge and does not render (the cutoff, Decision 11). With the leaf
reads that `environment_reads_hold` checks by value today, the node would be a `locale` reader and
render. Within one call, a reused result's reads are still added to the call's records with tokens
(Decision 4), since that is how the call decides reuse. The tree validates with the direct inputs
only. Beside them, each kept variant also holds the flattened set of the leaf environment reads it
made at any depth, through its edges included, each with its supplier class (root or override). That
set is used only when a variant is taken into a call (Decision 11): for the check that no layer the
render pushed touches a value it read, and for the tokens it enters the call with. It is bounded by
the program's environment declarations, and building it from the edges' sets is free.

Some evaluations are not kept results, and no edge stands for their reads: a formula, in `$env`
or in an `envProvide`; a formula entry of an entry-module bundle, which is evaluated inside the
read that found it (Decision 2); and a result evaluated under the render's own overrides, which
the tree does not keep (Decision 11). Each gives its direct inputs to what encloses it, as if its
reads had been made there. So a read of `gap` that an entry-module `provide env with
<env.Update gap={env.space * 2} />` supplies is a root read of `gap`, with an edge to the bundle,
followed by a root read of `space`; and a body that overrides `mode` and reads `loud`, whose
default reads `mode` and `locale`, records a root read of `loud` and of `locale`, but not of
`mode`, which its own override supplies. A read inside such an evaluation is classified at the
place like any other: a layer the render pushed changes the value it finds only through what the
render itself recorded, so a root read of `locale` there still holds when `locale`'s root value
is unchanged.

An override-supplied read holds the value found beneath the render's own layers, which a lookup
at the place gives and which stands for the place's own formulas, and is followed by the direct
inputs and edges of the sources of the render's own field overrides on top of it alone, as in
`provide env.colors.accent={env.locale}` under a place that sets `colors.tone`. So under
`<Stack env.gap={if env.space > 0 { 1 } else { 0 }}>`, a change of `space` from `1` to `5` leaves
`gap` at `1` and renders nothing. A
read inside the source of a layer the render pushed itself, as `env.x` in `provide env.x={env.x +
1}`, goes on beneath that layer and is recorded as a read of its own, root or override-supplied.

One case cannot be checked by value: a read that a layer of the place sets, wholly or at a field
path, when a source of it was evaluated under a layer the render pushed itself. If the place sets
`locale` to `if env.mode == "dark" { env.region } else { "fixed" }` and the body overrides `mode`,
the value found under the body's override is not the one a lookup at the place gives, and both can
stay equal while what the body reads changes. Such a read counts as changed at every check, so the
node renders again whenever its record is checked, and the direct inputs of what it evaluated
follow it, so that a change of `region` reaches the node through the indexes.

A node's record also holds the props and state fields its render read (Decisions 9 and 10), and
the node holds its own `verified` revision, the revision at which its record was last found valid.
A record is checked as Decision 11 defines: each input classified at the place, then validated in
order, stopping at the first that changed. A node whose record was verified at the current revision
in the environment it holds is valid without a check.

### 9. State fields are sources, and a dispatch in the tree follows them

A render records, by name, the state fields its body read: a state field's `slot` node evaluated,
and a state field a formula captured when the body wrote an override (Decision 10 lists the reads).
The record is kept on the instance in memory only; the stored form of an instance is unchanged.

The bare lifecycle APIs, `dispatch_component_actions` in Rust and the TypeScript runtime's
dispatch, are unchanged. They render the body after every batch with fresh tokens, as
`rust-ir-runtime` and `typescript-ir-runtime` require, and the corpus's lifecycle results are
unchanged. Their instances (`InstanceData`, `NxComponentInstance`, the stored `Stored`) hold no
rendered output, so there would be nothing to return, and a host of the bare API holds the output
itself. A bare-API host can also run a parent-bound handler for an emit, or a handler handed down,
on what it captured rather than live state (`invoke_handler(handler, action, None)`), which a skip
would leave stale.

The instance tree holds each node's rendered output, so the skip belongs there. The tree runs a
batch through an internal Rust lifecycle entry, a single call with one budget, that applies the
handlers as `dispatch_component_actions` does and returns the new state and the effects without
rendering, unless the render is needed. It is needed when a state field the render read differs
between the start and the end of the batch, or when the node's other records do not hold: its
environment record, checked as Decision 8 says, and its props, which a dispatch does not change.
The comparison covers each state field the handlers patched that the render read, compared once,
its value at the end against its value at the start, charged as `diff` charges a field
(`patch_state` knows the patched fields). When the render is needed, the entry renders the body with
the new state as `dispatch_component_actions` does, with the next generation's tokens, so the count
is today's count plus the comparisons. When it is not, the node keeps its rendered output, its
tokens and its handler generation, and takes only the new state. Nothing is rendered or written for
the output then, and no origins are reported for it: the node's output keeps the origins its render
reported. The effects and the state are written as today.

A dispatch that keeps its render keeps the instance's identity for everything the tree means by
"the same render": the handlers, the generation and the tokens. The tree compares instances with
`ComponentInstance::is` in `through`, `visited`, `read_from_current`, `walked_anew`, `rebinding` and
`creator`. A kept render gives an instance whose render part is shared with the one before, and
`is` compares that part, so the children's `through` and the pass's record of what it visited still
match, and no rebinding follows. Only the node's state changes. The node is not marked unwalked and
stays settled.

In the tree, every handler runs against the node whose body created it (`creator`), and an emit is
routed to the parent's handler by token and run against that handler's creator too. So every
handler the tree runs reads its owner's state live, and the state slots it captured are never read.
A field read only by handlers needs no render, and handlers' state slots are not reads.

### 10. Props are sources, and a new descriptor follows them

A render records the props it read. A prop is read:

- when the body evaluates its `slot` node;
- when the body writes an override formula that captures it. `capture_formula` reads
  `frame.get(slot)` for the formula's locals without evaluating their nodes, so with
  `<Row env.gap={env.space * scale}>`, a prop or state field `scale` is a read of the enclosing
  render;
- when the body binds a handler that reads it. A handler reads its props from what it captured, so
  the prop slots of each handler node the body binds, found as a formula's locals are
  (`slots_read`), are reads. Its state slots are not (Decision 9).

A handler today captures the whole frame of the body (`Handler.captured`), so after any change of
the frame every handler the body binds is unequal to the one before, whether or not it reads what
changed (`specs/future.md`, "What a handler captures"). This change narrows the capture: when a
runtime makes a handler, it captures only the slots its node reads, `slots_read` of the handler
node, computed by the runtime from the image (no image format change), at no charge, and cacheable
per node. Both IR runtimes do this, and both hold and compare the captured slots in ascending slot
index, whatever order `slots_read` finds them in (Rust's walk pops depth first), so a comparison
that stops at the first slot that differs costs the same in each. Two handlers made by one node
then compare equal when only a slot they do not read changed, which is what lets a kept child's
content keep its render. This is a deliberate change of results: `==` on handlers is part of the
language, and today it finds such handlers unequal (`handlers_equal` compares the whole captured
frame), so a program that compares handlers, or keeps the result of a comparison, can give another
result. What a handler does when it runs does not change, since a slot it does not read cannot
change that. The change also moves operation counts, since comparisons compare fewer slots; the
corpus regeneration covers both (task 6.2).

A content parameter counts as one prop. Handler props bound for emits are not reads; they are
compared as the existing descriptor comparison compares them.

Props are compared after defaults resolve. A prop's default can read other props
(`normalize_fields` binds the frame as it goes), so with `title: string = label`, a body that reads
only `title` depends on `label` too, and comparing the normalized `title` sees it. The instance needs
the new normalized props anyway, since a later dispatch builds the body's frame from
`instance.props`.

A prop's default can also read the environment, as with `label: string = { env.mode }` in a
descriptor a host wrote without `label` (a parent's render fills in the defaults of the
descriptors it writes). The render's record starts before the props are normalized, so what the
defaults read is part of it, first. The node also keeps those inputs apart: a dispatch renders
from the props the instance holds without normalizing them, and starts its record with them; and
a prop skip that takes new props joins what their normalization read to the node's record. The
defaults of the state are evaluated only when the node is created, and the state is kept, so
their reads are not the render's.

When a visit hands a node a descriptor that differs from the one it holds, and its environment
record holds at its new place, the tree normalizes the new descriptor's props through an internal
Rust lifecycle entry, charged as initialization charges normalizing them (each check, each
default's nodes). It then compares the recorded props of the old and new normalized props with the
semantics of `Across`, which the rebinding path uses today: a handler the body can see is resolved
through the instance each descriptor was read from, compared as the language's `==` compares it, and
recorded in a `Rebinding`; a handler bound for an emit must be bound in both or in neither; the text
of a token does not count. The comparison is metered as the existing rebinding comparison is
(`Program::metered`), not unmetered as `same_descriptor` is.

When a recorded prop differs, the node is initialized again from the props already normalized for
the comparison, through an internal entry, so normalization is charged once. Otherwise the node
keeps its render. When the parent's instance changed, or the comparison mapped a handler to another, the kept
output, the node's own handlers and the nodes initialized through it are rebound as a kept child is
(`rebinding` and `rebind`), with the rebinding's cost. The instance takes the new normalized props
and handler props without rendering, and keeps its generation and tokens. Props are outside the
render part that `ComponentInstance::is` compares (Decision 9): the identity is the render, its
generation's handlers and tokens. So a prop skip without rebinding leaves the children's `through`
and the pass's record matching, and a rebinding updates them as it does today. The node takes the new
descriptor and environment and stays settled. An equal descriptor still takes today's free path.

### 11. The tree keeps derived results across renders, checked by revision

The tree keeps derived results its renders evaluated and offers them to later renders of any node.
Checking a kept result by looking up and comparing every value it read, at every use, would cost a
lookup and a comparison per read. Instead the tree follows the revision scheme of incremental
engines such as Salsa.

**What is kept.** Top-level values, defaults and entry-module provides (a bundle keyed as in
Decision 2) evaluated in renders the tree makes, when none of the reads they recorded has a token
that names a layer the render pushed. A result evaluated under the render's own `envProvide`
overrides is not kept, matching the "found under the call's own overrides" flag. Formulas, in
`$env` or in an `envProvide`, and bundles a render's `envProvide` holds are kept within the call
only: they are identified by a layer, and layers are numbered per call.

**Revisions.** The tree has one counter, which each `set_environment` advances once. Each root
value it changes, set or cleared, takes the new revision; a root value changes when it is not the
same as before as a host would see it written, by the cutoff's rule below, so `0.0` becoming `-0.0`
is a change, and each kept result whose value changes
when it is evaluated again takes the current revision, as Salsa's derived queries do: a result is
only evaluated again because something changed after its users were last verified, so the current
revision is already later than their `verified`. (The proposal said "the next revision"; with one
advance per `set_environment` the current one is the same rule without a counter that moves during
a pull.) A kept result's variant holds `changed`, the revision at which its value last changed, and
`verified`, the revision at which it was last found valid.

**Records.** A variant's record holds its direct inputs in the order it read them (Decision 8).

**Validity at a use.** Whether a read is a root read or an override-supplied read depends on the
use, since each node's environment is its own stack of `$env` layers. So a variant is valid at a use
only when its inputs, checked in the order they were read, all hold there:

- each read is first classified at the use: whether a layer there sets or caps the value, which
  the walk that gives tokens finds for free. A read recorded as a root read where a layer now sets
  the value, or recorded as override-supplied where none does, does not hold;
- a root read holds when its root value has not changed since `verified`, and, when it has an edge,
  when the edge holds;
- an edge is checked as Salsa's deep verify checks an input query: the variant it names is
  validated first at the use, by this rule, and when it is not valid it is evaluated again there,
  with the equality cutoff. The edge then holds when that variant's `changed` is no later than this
  record's `verified`. An evaluation that gives another variant than the one the edge names, because
  its reads are classified otherwise at the use, counts as changed. Evaluating the used result is
  safe for the reason the order gives: the edge was read after inputs that all still hold, so a
  fresh evaluation would evaluate it too;
- an override-supplied read holds when the value the environment lookup gives at the use equals the
  recorded one. Getting the value evaluates any formula there, and a derived result the lookup needs
  is taken from the tree by this same rule. It cannot need the variant being validated, which would
  have been a cycle when that variant was evaluated. The lookup is charged as the skip check's reads
  and comparisons are.

The check stops at the first input that does not hold, and the result is then evaluated again,
without checking the rest. Each input checked was read by the last evaluation after inputs that all
still hold, so a fresh evaluation would reach that same read, and the check never evaluates a
branch the fresh run would not take, nor raises an error it would not raise. The order also fixes
the operation count. So with a result `K` that reads `flag` and, only when `flag` is true, a result
`K3`, a change of `flag` to false evaluates `K` again without validating `K3`. And with a node that
read only a kept `direction` result, a change of `locale` evaluates `direction` again while checking
the node's edge; when it gives `ltr` again, its `changed` stays, the edge holds and the node does not
render. A variant or node found valid takes the current revision as its `verified`. Comparing revisions costs no operations.

**Equality cutoff.** A result evaluated again is compared with the variant it replaces: the first
variant whose reads are classified at the place as they were recorded and that was found not valid
there, because a root value or an override-supplied value it read changed. Validation stops at the
first input that does not hold, so for a variant found stale there the reads after that input are
classified too, which costs nothing; one of them classified otherwise makes the variant another
place's, not one to replace. It is the same value when a host
could not tell the two apart in written output: the same number kind (`1` and `1.0` differ), `-0.0`
and `0.0` distinct, a NaN never the same as anything, so it never cuts off, and, when the host
collects origins, equal origins. `values_equal` alone is not enough: it equates `Int(1)` with
`Float(1.0)` and `0.0` with `-0.0`, and origins take no part in it. The comparison is charged as a
comparison of values. When the values are the same, the tree keeps the old value and leaves
`changed` alone, so nothing that used it is evaluated or rendered again. When they differ, the new
value takes the next revision.

**Variants.** One result can come out differently in different places. On a page with a normal
section and a section inside `<div env.colorMode=dark>`, `color`'s default gives the light palette
in one and the dark palette in the other. If the tree kept only one `color`, each render would
discard the other section's palette and build it again. So the tree keeps one copy for each
situation, a *variant*.

A situation is how each value the result read is supplied where it is used:

- whether the value comes from no override (the root value, a provide or the default) or from an
  override above that place;
- for a value that comes from an override, the value it sets.

In the example the two variants are "`colorMode` from the root" (light) and "`colorMode` overridden
to `dark`".

When a render reads `env.color`, the tree tries the variants in the order they were kept and checks
each with the validity rule above: inputs in the order they were read, revisions, and
override-supplied values compared. It uses the first valid one. When none is valid, it evaluates the
result at that place and keeps the new value as a variant:

- If a variant for the same situation exists but is out of date, for example because the root
  value of `colorMode` changed, the new value replaces it. The new value is first compared with
  the old one, and when they are equal the variant keeps its old revision, so nothing that used it
  is evaluated or rendered again. This is the equality cutoff.
- If no variant is for this situation, the new value is added as one.

**Lifetime.** A variant is referenced by the node records and kept-result records that hold an edge
to it. When a node renders again, is replaced, removed or dropped, or a variant is replaced or
dropped, its edges are released. A variant that no record references is dropped when the tree
operation ends, releasing its own edges in turn. So a palette the first click used stays while a
node's record uses it, however many passes render nothing in between.

**Into a call.** The tree passes its kept results to each render's `Machine` through an internal
runtime option, not a public API. At a use in the render, the runtime takes a result from the tree
when no layer the render pushed touches a value in the variant's flattened set of leaf reads, which
the tokens show for free. The set is the environment values the variant's direct inputs and, through
its edges, the inputs of the results it used read; it is found from the records when it is needed
rather than held, and holds no supplier class, since validation, not this check, classifies reads. Checking only the direct inputs would not do: `button`'s direct inputs are
an edge to `color`, and a render inside its own `provide env.colorMode=dark` must not take the light
`button`. A result taken from the tree enters the call's own cache as if it had been evaluated at
the call's start, its flattened reads with the tokens they have over the call's host layers, taken at
the use, and Decisions 1 to 4 apply from there, the replay of its reads into enclosing records
included. A result the render evaluates and the tree may keep is kept when
the render succeeds.

**Cost.** A render's count now depends on what the tree holds, which a pass's count already does,
and a host that wants a count independent of history evaluates outside the tree. A kept result
shared by several nodes is paid for by whichever render evaluates it first, so which node's budget
pays depends on the order of the host's calls, as the tree's held state already does.

Checking a node's record can evaluate kept results again, which can be the expensive part, and the
render that follows takes them for free. So the check is charged to the lifecycle call that
follows it at the node: the visit's initialization or prop normalization, the pull's render, or
the dispatch. That call starts with what the check left of the operation budget and reports the
check's operations with its own, so the check and the call share one budget, and the reported
count includes every evaluation the render needed that the tree did not already hold. A check that
holds with no lifecycle call after it, because the node keeps its render, is reported as a call of
its own: the usage report holds what the check spent. A check that fails reports what it spent.
A record found valid at the current revision in the environment the node holds needs no check and
reports nothing.

**All or nothing.** `atomically` snapshots and restores, with the nodes, `visited`, `inert` and
`roots`, the kept results, the revision counter, the reverse indexes and the change set, which one
store holds. A tree operation that evaluates works through a session over the store, which reads
the store in place, holds the revisions at which variants were found valid beside it, and copies
the store only when it keeps a result; what it found is put back only when it succeeds. A `set_environment` or dispatch that fails, and a pass run as one change that
fails, leave all of them as they were.

### 12. Changes are pushed, then pulled

Every source keeps an index of the tracked computations that read it directly: the tree's `readers`
index for root values, extended to kept results; reverse edges from each kept result to the kept
results and nodes that used it; the instance's own record for state fields; and the node's own
record for props. The `readers` index lists, for each environment value, the nodes and kept results
with a root read of it, and those with an override-supplied read whose lookup read its root value,
such as a read of `gap` under `<Row env.gap={env.space * 2}>` for `space`.

`set_environment` gives each changed root value a new revision and follows the indexes to mark the
nodes that could be affected as possibly stale, transitively. That walk visits only what the indexes
name and evaluates nothing. Then, in depth order, the tree pulls: for each possibly stale node it
checks the node's record at its environment by the rule of Decision 11. For each edge in order, it
validates the used kept result first, evaluating it again with the equality cutoff when it is not
valid, and the edge holds when the result's `changed` is then no later than the node's `verified`.
It renders the node again, with the state it holds, at the first direct input that does not hold. A node whose inputs
all hold is left alone, takes the current revision as its `verified`, and stays settled.

A visit's skip check validates the node's record at its new place by the same rule. A dispatch
decides from the instance's own state record (Decision 9). Within one call there are no
notifications: a call's records are checked when a result is asked for, because that check is free.

### 13. The host gets a change set

The tree keeps a change set, which `take_changes` returns and clears. It is a diff by key between
the tree when the host last took the set and the tree now:

- a key with no node then and a node now is *added*;
- a key with a node then and none now is *removed*. Every node dropped with a removed or replaced
  subtree, by `drop_under`, by `finish` or by `remove`, is listed under its own key;
- a key whose node was replaced in between, by a visit with another component or under another
  parent, or removed and then visited again, is listed as removed and added;
- a key whose node is the same node then and now is *rendered* when its rendered output changed in
  any way since, a render or a rebinding that changed what its tokens name included. A node that
  only took a new descriptor or new state with no change to its output is not listed;
- a node added and then removed before the host took the set appears in no list, a node added and
  rendered appears as added only, and a node rendered several times appears once.

A host that derives something from nodes, such as native controls or an index, updates exactly
those and needs no walk. `is_settled` stays for hosts that walk, and the two agree: a node is
unsettled exactly when it or a node under it rendered or was added since a pass last walked it.

### 14. What the implementation settled

Points the decisions above left open, settled while implementing them in
`crates/nx-ir-runtime/src/kept.rs` and `tree.rs`:

- **Marking moves the rest up.** After `set_environment` marks what a changed value reaches, every
  node and variant it did not mark that was verified at the previous revision takes the new one as
  its `verified`, at no cost. Without it, a node verified before the change would find a kept
  result evaluated again during the pull "changed after it was verified" though nothing it read
  changed.
- **Failures during a check.** A lookup or an evaluation that fails while a record is checked makes
  that input not hold, and the evaluation that follows raises the failure where a fresh one would.
  A resource limit is not swallowed: it ends the operation, as it would have ended a fresh render.
- **Reads under the render's own overrides.** A source evaluated under the render's own override
  is not kept, and gives its direct inputs to the read (Decision 8). A body that overrides `mode`
  and reads `loud`, whose default reads `mode` and `locale`, records `loud` and `locale`: it is not
  reached by a change of the root `mode`, which is what the existing cascade scenario asks, and it
  is reached by a change of `locale`.
- **Override-supplied reads compare by written identity.** A kept result is taken at other places
  than the one it was kept at, so an override-supplied read is compared with `same_written`, the
  cutoff's rule, origins included when the call collects them: a variant kept under
  `env.z={0.0}` is not taken under `env.z={-0.0}`.
- **State fields compare by written identity.** A dispatch compares a patched field the render read
  with `same_written`, the equality cutoff's rule, charged as `diff` charges a field, so a field
  patched from `1` to `1.0` renders, as the host would see it written otherwise.
- **Dropping variants.** Each variant knows its users, nodes and other variants; every tree
  operation sweeps the variants with none at its end, releasing their edges in turn. The sweep
  starts from the variants kept or left without a user since the last one, so it costs what the
  operation changed, not what the tree holds.
- **Testing what is not checked.** That an unrelated kept result is not validated has no outcome a
  host can see, since validating it would evaluate nothing; the tests hold the nodes that use it to
  their output and settledness instead.

## Risks / Trade-offs

- **Every recorded count changes.** Every corpus program that reads a value or a default more than
  once gets smaller counts. The regeneration is mechanical. The PR's corpus diff shows each change,
  and `design-tokens-theme`'s drop is the measurement this change exists for. Reuse changes no
  result; only a comparison of handlers that differ in an unread slot does (Decision 10).
- **Counts depend on the tree's history.** A render's count includes the checks of kept results
  and excludes what they save, so the same render costs less in a warm tree, and a shared result is
  paid for by the render that evaluates it first. This is already true of a pass, and is stated in
  the cost rules.
- **Reuse can turn a limit failure into a success.** A read deep in a call that reuses a default
  evaluated shallowly does not reach the depth limit a fresh evaluation would. Both runtimes do the
  same, and the change accepts it.
- **Recording props and state reads touches every render.** The records are small (bounded by the
  component's declared props and state fields) and replace work the dispatch and visit skip.
- **Captured reads must be exact.** A handler or formula whose slots are recorded wrongly could keep
  stale captures, and with the narrowed capture a handler missing a slot would read it unbound. The compiler's slot analysis (`slots_read`) is the source, and the tree's tests
  run the handler programs with the skips on.
- **One entry per derived result within a call.** A call that alternates between environments
  evaluates again at each switch. That is at most what it costs today.
- **The rule must be exact in both runtimes.** A difference in where a token is taken would show as
  a count difference in the corpus and in the cost differential tests, which both run on every PR.
- **Read records cost memory.** A record holds one entry per distinct value read, bounded by the
  number of environment declarations in the program, and a kept result lives only while a record
  uses it.

## Migration Plan

None. Hosts see no API change beyond `take_changes`. A host that sets an operation budget sees
smaller counts. A tree host that relied on a dispatch always giving new tokens sees the old tokens
kept when nothing the render read changed, and they stay valid.

## Open Questions

None.
