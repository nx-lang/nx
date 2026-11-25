## Context

How a handler works on `origin/main` (2f1f7c0b):

- **Making one.** Evaluating an `actionHandler` node (`eval_handler` in `eval.rs`,
  `evalActionHandler` in `index.ts`) copies the slots that the node and the nodes beneath it read
  (`slots_read`). The slots are held in ascending order, which is the narrowed capture of
  `add-tracked-reactivity`. Nothing about the environment is kept.
- **Running one.** `invoke_handler` (`component.rs`) and `invokeHandler` (`index.ts`) bind the
  captured slots, the live state when the dispatched instance owns the handler, and `action`. They
  then evaluate the body with the machine's environment: the host's root values and `$env` chain
  for the call (`EnvState.base` layers). The `envProvide` layers that were on the stack when the
  handler was made are gone.
- **The tree.** It routes every dispatch to the handler's creator (`creator` in `tree.rs`) and runs
  it with `options_for(&node.environment)`, the creator node's environment. So a handler sees its
  component's place and nothing written inside the body.
- **The bare API.** It runs a handler the dispatched instance did not bind (one handed down, or one
  the parent bound for an emit) in whatever environment the host gives for the call. For an emit
  dispatched against the child with the child's environment, the handler happens to see the
  overrides on the child's descriptor.
- **Stamping.** A provide adds its overrides only to component descriptors (`stamp`). A handler
  value in a descriptor's props or content is never stamped.
- **Equality** (`handlers_equal`, `handlersEqual`): the same module, declaration and node, then the
  captured slots, compared in order up to the first that differs.
- **The stored form** (`stored.rs`): `Handler { module, declaration, node, captured }`. Everything
  else is read back from the node.
- **Generated JavaScript** refuses action handlers, so only the two IR runtimes are involved.

The probes in the proposal confirm all of this. `add-environment-values` recorded the gap in
`specs/future.md`.

## Goals / Non-Goals

**Goals:**

- A handler's body sees the overrides written around the handler, with the same lookup, the same
  formula semantics and the same results in both IR runtimes, with equal operation counts.
- Handlers that can't read the environment are unchanged: they capture nothing, cost the same and
  compare as before. Kept children are not rendered again because of an override their handlers
  don't depend on.
- Equality, the stored form, rebinding and the tree's skip rules all stay well defined.

**Non-Goals:**

- Overrides around the place where another body puts the element. A handler sees where it is
  written, not where it is placed (Decision 1).
- Narrowing the capture to the values a handler reads (Decision 3, `specs/future.md`).
- Making the bare API exact for a parent-bound handler dispatched through the child (Decision 8).
- Generated code, which has no handlers.

## Decisions

### 1. A handler sees the overrides where it is written

A handler's body is code of the component that writes it. It runs against that component, reads its
state live, and reads the locals of its own body from what it captured. So its environment is that
component's environment plus the overrides written around the handler in that body:
`provide { … }` blocks, the `env.` attributes of enclosing elements, and the `env.` attributes of
the element the handler is an attribute of. `<Card env.mode="card" onLogged=…>` means a provide
around `<Card onLogged=…>` (environment-values), and the element's other attributes are evaluated
inside it, so the handler sees `card`. A function the handler calls reads in the handler's
environment, as every function reads in its caller's.

*Alternative: where it is placed.* A component body sees the overrides where its use is placed. A
handler in content that `Box` puts inside `provide env.mode="box"` would then read `box`. This is
rejected for three reasons:

- A handler is not a use. Nothing places it, and the tree runs it against its creator wherever its
  token was read.
- Where it ends up depends on which instance renders it. A handler handed down two levels would see
  different values depending on the node it is dispatched through.
- Equality would have to depend on placement.

The lexical rule is also what the `specs/future.md` entry asks for: "the environment of a `provide`
block written around the element that holds it".

### 2. What a handler captures: the overrides of the provides around it

A flagged handler (Decision 3) captures the overrides of the `envProvide` nodes being evaluated
around its node, outermost first. These are the first source of a lookup in `docs/nx-ir-format.md`,
"Environment values". The capture holds them in `$env` entry form:

- `{value, path, to}` for an override evaluated where written;
- `{value, path, formula}` for a formula, with its captured locals;
- `{bundle: [...]}` for a bundle applied by a block.

`eval_provide` already builds these entries for stamping. It now builds them before its body and
keeps them beside its layers in `EnvState`, so a handler takes the list that is current. The list
for each provide depth is shared, so taking it copies no entries.

It does not capture:

- **The component's `$env` chain** (the `base` layers). At dispatch these come from the instance the
  handler runs against. In the tree that is the creator node's environment as it is now. A node can
  be kept while its place's overrides change (a visit whose recorded reads all hold takes the new
  `$env`), and a handler that captured the old chain would then read stale values.
- **Root values.** They come from the call, so a `set_environment` reaches a handler at its next
  dispatch without rendering anything.
- **Caps** (`Layer::Cap`). These belong to the read under way, not to the place.

*Formulas stay formulas.* A captured formula is computed at each read in the handler's body,
in the environment of that read at dispatch, with its captured locals bound. The rule "every
environment value is computed where it is read" then holds for handlers too. A root change, or a
change to the creator's place, reaches `env.gap={env.space * 2}` when the handler runs.

*Alternative: evaluate every override when the handler is made.* That would capture plain values
and keep equality simple. A formula would then be frozen at render time, which contradicts the rule
above, and its value would differ from the one a component under the same provide reads.

*Alternative: capture the whole environment, base included.* It would make the bare API independent
of what the host passes. In the tree it gives stale values for kept nodes. It would also make a
handler unequal whenever any ancestor's override changes, and so render children again that need
nothing.

*Values captured by a plain override come from the render.* An override that reads no environment
value is evaluated where written, with the state as the render saw it. A handler that patches
`label` and then reads `env.mode`, where `mode` is set from `label` around it, sees the mode from the
render. This is the same rule as for props and locals the handler captured, and as for a state field
a formula captures (component-action-handlers, "Captured locals keep their snapshot").

### 3. Only a handler whose body can read the environment captures

The compiler flags an `actionHandler` node when the handler body's static read set is not empty.
It uses the analysis that marks formulas (`formulas.rs`): `env.` reads, the functions and values the
body names, component elements whose prop defaults may read, and calls whose callee is known only at
run time. An unflagged handler captures nothing, so:

- most handlers (`<State.Update count={count + 1} />`) cost and compare exactly as they do today;
- a change of `provide env.colorMode={mode}` around a section does not make the section's handlers
  unequal, so a `Box` that holds them in its content keeps its render.

A flagged handler captures every override around it, not only those for values it reads.

*Alternative: always capture, with no flag.* This needs no IR change. Every handler inside a provide
would become unequal whenever the provide's value changes, and children that see those handlers
would render again for nothing. The comparison cost of existing programs would also change.

*Alternative: capture only the overrides of the values the handler reads.* A captured formula for a
value the handler reads can read other values, and so can a default. The set to keep is therefore
the closure through the formulas and bundles in the capture, and bundles are runtime data. The
compiler can't know it, and a runtime would have to compute it at each capture. It is recorded as
later work.

### 4. IR: a trailing flag cell, behind `environment-v3`

The node becomes `[19, ref, str, ref, slot, ref?, node, 1?]`. A flagged handler has a trailing cell
`1`, and an unflagged one is written as before. A module with a flagged handler lists
`environment-v3`, and a runtime refuses such a module if it doesn't list it. This follows the
formula flag and `environment-v2`: a runtime that predates the flag refuses the module by name
rather than running the handler without its overrides. The schema version doesn't change. `explain`
marks the flag, for example `onTapped reads env`.

*Alternatives:*

- *Find the read set at run time from the image.* Both runtimes would need the call-graph fixpoint
  the compiler already computes.
- *Name the feature `action-handlers-v2`.* `environment-v3` was chosen because the flag only
  matters with environment values, and the environment features already gate formula flags.

### 5. Running a handler

`invoke_handler` and `invokeHandler` do the following:

1. Begin the call's environment as today: the host's roots and `$env` chain as `base` layers.
2. Push the captured overrides above `base`, as an `envProvide` with those entries would push them
   (bundle entries one layer each, with the bundle flag), each with a fresh layer identity.
3. Bind the slots, the live state and `action`, and evaluate the body.
4. Truncate the layers.

The pushed layers are the call's own layers, so every existing rule for own layers applies:

- a read that one of them supplies wholly is not a root read;
- a result evaluated under them is not kept for the place;
- a formula they hold is a derived result keyed by its layer's identity, so within one invocation it
  is reused while what it read is unchanged, and it is evaluated again in the next invocation.

Plain values are not checked against their types again. They were checked when the override was
evaluated, and a handler is runtime data. Pushing costs nothing, the same as entering `$env` costs
nothing beyond its reads.

### 6. Equality, rebinding and the stored form

- **Equality.** Two handlers are equal when they have the same module, declaration and node, their
  captured slots are equal (ascending slot index, stopping at the first that differs, as today),
  and their captured override lists are equal. The lists are compared as two sequences are, with
  the cost rules of `valuesEqual`. Handlers of one node are flagged alike, and an unflagged pair
  has nothing more to compare, so its cost is unchanged. The language's `==` on handlers changes
  only for two handlers of a flagged node whose overrides differ: they are now unequal. That is
  correct, because they now run differently.
- **Rebinding** (`component.rs`, the handler replacement walk). It walks the captured override
  values as it walks captured slots, so a handler held inside an override value is replaced
  consistently. Overrides are almost always data, but the walk keeps the rule uniform.
- **Stored form.** `StoredValue::Handler` gains `environment: Option<u32>`, the index of the list
  value in the instance's value table, so a list shared by many handlers is written once. Restoring
  reads the list and checks it. Each entry must have the `$env` shape and name an environment value
  of the program, and each formula must be one the program has (`formula_from_data` must not refuse
  it). An altered instance is refused at restore, the same way a node that is not a handler is
  refused. The depth and size bounds count the list as part of the handler's capture.
- **Host form.** The written `ActionHandler` record (`{action, token}`) doesn't change, and neither
  does the host form of a handler held in state.

### 7. The instance tree

- **Where a handler runs.** Unchanged: against its creator, with `options_for(&creator.environment)`.
  With Decision 5, it reads the creator's current place with its captured overrides on top.
- **No new reads.** A plain captured override's value was computed during the render from slots and
  state the render already records. A formula's locals are recorded as render slot reads when it is
  captured (`capture_formula`), and a bundle value comes from slots or declarations. So the captured
  list is a function of the render's recorded inputs. A dispatch skip, prop skip or visit skip that
  keeps the render keeps handlers whose captured overrides are still the ones a fresh render would
  make. A handler's own environment reads at dispatch are not render reads, as today.
- **Kept children.** A child holding a visible handler compares it with Decision 6's equality. A
  parent that renders again with equal override values makes equal handlers, so the child is kept.
- **`set_environment`.** Nothing to do. Roots are never captured.

### 8. The bare API

A dispatch runs each handler in the environment the host gives for the call, with the handler's
captured overrides inside it. For a handler the dispatched instance bound, the host gives that
instance's environment, and the result is exact. For a handler the instance did not bind, the
result is exact when the host gives the creator's environment. The exact route is to dispatch the
handler's token against the instance whose output holds it, as the tree does. For an emit, that is
the parent's `onLogged` token against the parent's instance.

A host that still dispatches an emit against the child, with the child's environment, applies the
overrides on the child's descriptor and the handler's captured overrides one after the other. For an
override that sets a value, or a field of one, that gives the same result, because the second
application sets the same thing again. It differs only for an override that reads the value it
sets, such as `env.space={env.space * 2}`, which then applies twice. This is documented on the
lifecycle API and recorded in `specs/future.md`. The fix is for an instance to carry the environment
it was initialized in, so the runtime can run a parent-bound handler in its parent's environment.

### 9. Cost model

- Making a flagged handler costs nothing more: it takes the list its provides already built. Making
  an unflagged one is unchanged.
- Running a handler costs what its body's nodes cost, plus any default, entry-module provide or
  formula evaluated for its reads, under the reuse rules. Pushing its captured overrides costs
  nothing.
- Comparing two handlers of a flagged node costs, after the slots, what comparing their two override
  lists as sequences costs. The comparison stops at the first slot or entry that differs.
- `docs/nx-ir-format.md` states this under *Action handlers* and *Evaluation cost*. The corpus pins
  it with a new program, `environment-handlers`.

## Risks / Trade-offs

- **The `==` of two handlers changes.** It changes only for flagged handlers whose overrides differ,
  which now run differently. This is deliberate, as the narrowed capture was.
- **A flagged handler under a changing provide is unequal across renders, even for a value it
  doesn't read.** Children that see it render again. Narrowing by value is later work.
- **The double application in the bare API** (Decision 8). It affects only an override that reads
  its own value, in a host that routes an emit through the child. Hosts that route by token, and the
  tree, are exact.
- **Own-layer bookkeeping during dispatch.** Pushing layers in a tracked dispatch has to follow the
  own-layer rules of the render exactly, or a kept result could be stored under the handler's
  overrides. Tests run the handler programs through the tree with its skips on, and through
  `set_environment` after a dispatch.

## Migration Plan

None. Programs without a handler whose body reads the environment emit the same IR. Others gain
the flag and `environment-v3`, and the corpus images are regenerated.

## Open Questions

All resolved, each with the recommended answer:

- **Scoping (Decision 1): lexical.** A handler sees the overrides where it is written, not where it
  is placed. A child that wants its parent to see a value of its own environment passes it in an
  action field, as an ordinary value computed where the child writes the action.
- **The bare-API emit path (Decision 8): documented, made exact later.** An emit is dispatched
  exactly against the parent. The double application through the child is documented on the
  lifecycle API, and `specs/future.md` records the fix of an instance that carries its environment.
- **Narrowing the capture to the values read (Decision 3): later.** It is recorded in
  `specs/future.md`.
- **The feature name (Decision 4): `environment-v3`.**
- **When the overrides are captured: at render.** A handler holds the overrides of the render that
  made it, and a captured formula is evaluated at the read, in the environment of the dispatch.
