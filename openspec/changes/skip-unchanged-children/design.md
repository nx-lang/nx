## Context

The instance tree (`crates/nx-ir-runtime/src/tree.rs`) keeps one node per use of an authored
component. `visit` compares the descriptor a host hands it with the one the node holds, and skips
initialization only when both hold the same values and the node was initialized through the
instance its parent holds now (`through_same && same(...)`). A parent that renders again holds a
new instance, so every child fails the first test and is initialized again with the state it held.
Its children then fail the same way, and the whole subtree renders.

`specs/future.md` ("Skip a child whose props are unchanged") explains why this wasn't simply
fixed:

- **Captures.** A handler captures the whole frame of the body that bound it. After any change to
  the parent's state, every handler the parent binds is unequal to the one before.
- **Identity.** The tree finds the node a handler belongs to by the handler's allocation, and has
  to. Comparing by value let a component nested under itself patch its ancestor
  (`support-rust-hosts`, RF1). A child that is left alone holds handlers of an instance its parent
  no longer holds, so its events and emits would go nowhere.

The note names two ways out: hand the child the parent's new handler without a render, or have the
parent's new instance keep the old handler where the new one is equal ("remembering").

## Goals / Non-Goals

**Goals:**

- A child whose new descriptor is equal by value to its old one keeps its instance, state and
  rendered output.
- Handlers bound for a child's emits don't block the skip, so the common case works: a list of
  cards each binding `onLogged` to the parent's state.
- Routing stays by identity. RF1 stays fixed, and a kept child's events and emits reach the
  parent's current instance.
- A host's tokens from the kept child's output stay valid, and the rendered output stays the same
  allocation, so `Arc::ptr_eq` and `is_settled` tell the host what didn't change.

**Non-Goals:**

- Narrowing what a handler captures. A handler the child's body can see, such as one inside its
  content, still compares unequal after any change to the parent's state, and the child renders
  again as it does today. Fixing that changes the image format and both runtimes, so it is a later
  change.
- Tracking reads (environment slots or state fields). That arrives with `add-environment-values`.
- The TypeScript runtime, which has no instance tree.
- Changing the component lifecycle API (`initialize_component`, `dispatch_component_actions`).

## Decisions

### Rebind the child rather than remember handlers in the parent

When the skip applies, the tree builds a new instance for the child that equals the old one except
that each handler from the parent's old instance is swapped for the parent's new handler at the
same place. The tree doesn't render the body again.

Remembering (keeping the old handler in the parent's new instance where the new one is equal) was
the cheaper option in `future.md`, but it doesn't reach the common case. A handler the parent binds
for a child's emit captures the parent's whole frame, so it is almost never equal to the old one,
and the child would render again anyway. Rebinding doesn't need the bound handler to be equal,
because the child's body never sees it. Remembering would also make the parent's instance depend on
what its children hold, which the lifecycle keeps separate today.

### What has to match

The tree compares the old descriptor (read through the instance in `through`) with the new one
(read through the parent's current instance) field by field:

- Values other than handlers are compared as `same()` compares them now.
- A handler token the body can see (in a prop, in content, at any depth) is resolved through each
  descriptor's instance, and the two handlers are compared with `handlers_equal`, the comparison
  the language's `==` uses. The body could only tell two handlers apart by comparing them, so
  handlers that `==` finds equal give the same output.
- A handler the parent bound for one of the child's emits (`onLogged` for `emits Logged`) is
  checked only for presence. It is in `handler_props`, never in the body's scope, so it can't
  change the output.

Token text is not compared, since a parent that renders again numbers its tokens from one again
and the same text can name another handler. What matters is the place the token sits at.

The comparison walks both descriptors once and records pairs of old and new handlers as it goes.
If the same old handler is paired with two different new handlers, the pairs don't define a single
replacement, and the child is initialized again as it is today. Two old handlers paired with the
same new one are fine: the rebound output holds the new handler at both places, as a new render
would. A handler bound for an emit must also answer the same emit in both descriptors, which is
the check initialization makes.

### Rebinding the subtree

The pairs give a map from old handler to new handler, keyed by allocation. Starting from the kept
child, the tree:

1. Substitutes the map in the child's `props`, `handler_props`, `state` and `handlers`. Any of the
   child's own handlers whose captures hold a mapped handler is rebuilt with the substitution, and
   the old and new allocations are added to the map, so that what the child handed down follows.
2. Sets the child's `through` to the parent's current instance and its descriptor to the new one.
3. Does the same for every node initialized through the old instance of a rebound node: substitutes
   the extended map in its instance and points its `through` at the rebound one.

A rebound instance keeps its generation and the order of `handlers`, so every token its output
carries names the handler at the same place as before. The rendered output Arc isn't touched.

Rebinding happens during the visit, so it's subject to `atomically` like any other change and
leaves the tree as it was if the pass fails. It runs no NX code, but the comparison and the
substitution run under the visit's operation and stack budgets: comparing handlers costs what `==`
costs, and the substitution one operation for each handler, list and record it walks. It remembers
lists and records by allocation as well as handlers, so a value many handlers capture is walked
once. This is not a lifecycle call, so it leaves the usage and origins reports alone; a visit that
compares and then initializes the node spends a budget on each.

### Settledness and walking

A kept child is marked visited. Its `unwalked` and `unsettled` flags are left as they were: a child
that was dispatched against before its parent re-rendered still holds output no pass has walked,
and the pass still has to walk it. A child that was settled stays settled, so the host stops there.

Nodes under a rebound child aren't visited by the rebinding. They were initialized through it, and
since its rendered output didn't change, what they were read from still stands. The existing rule
(a node under a visited node whose output a pass already walked is kept) keeps them.

### Dispatch before a pass

A dispatch still replaces the instance of the node it runs against, and the nodes under it still
hold handlers of the old instance until a pass visits them. The rebinding happens on that visit,
so the rule that a host runs a pass after every dispatch is unchanged, and so is the refusal of an
event that arrives before the pass.

## Risks / Trade-offs

- **The skip is narrower than it could be.** Content with handlers in it still renders again after
  any state change of its owner. → Narrowing captures is listed as later work in `future.md`.
- **Rebinding walks the subtree's instances.** It is linear in the handlers the kept nodes hold,
  which is far cheaper than rendering them, and it runs only when the parent rendered.
- **A host that relied on every child rendering again** would now see some children keep their
  output. → That is the point of the change. The README and the spec say it, and there are no
  compatibility promises yet.

## Open Questions

- Should rebinding also happen at dispatch time, so an event that arrives between a dispatch and
  its pass reaches a kept child's handlers? It would remove the "run a pass after every dispatch"
  rule for this case, but the rule still holds for children that render again. I'd leave it for
  now.
