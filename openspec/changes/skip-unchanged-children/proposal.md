## Why

When a component renders again, the instance tree renders every component under it, even when a
child is handed exactly what it was handed before. The reason is handlers: each render of the
parent makes new handler objects and numbers its tokens again, so a child's descriptor never counts
as the one it was initialized from. A host that follows changes is then told that everything under
a changed node changed, and has to compare output itself to find that most of it didn't.

This is the first step of the reactivity plan in the design-system tokens design
(https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG). Environment values come next. Without this
step, overriding `env.colorMode` around one subtree, or changing it at the root, would render every
component in the app, not just the ones that read it. NX bodies are pure, so a child whose inputs
are equal renders the same output, and the runtime can skip it without annotations, rules of
hooks or signals.

## What Changes

- A child visited under a parent that rendered again keeps its instance, state and rendered output
  when its new descriptor is equal to the one it holds. Values are compared by value. Handlers the
  body can see are compared the way the language's `==` compares them: same handler node, equal
  captures. A handler the parent bound for an emit is never in the body's scope, so only its
  presence has to match.
- The kept child is rebound to the parent's new handlers instead of rendering again. Each handler it
  held from the parent's old instance is replaced by the new handler at the same place in the new
  descriptor. The same goes for every handler under it that holds one of those (its own handlers
  capture its props), and for the nodes under it that were initialized through it. The tokens of
  its output don't change, so a host's tokens stay valid and its rendered output stays the same
  allocation.
- A handler keeps running against the instance whose body bound it, and an emit still reaches the
  parent's current instance. Identity stays what routing uses.
- `is_settled` becomes useful under a re-rendered parent: a pass that visits a kept child gets its
  old rendered output back and can stop there.
- The Rust IR runtime README, the component-instance-tree spec and `specs/future.md` describe the
  new behavior.

This is a behavior change for hosts, which no longer see a kept child render again. Nothing needs
to stay compatible yet (AGENTS.md).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `component-instance-tree`: a parent that renders again no longer renders every node under it. A
  child whose descriptor is unchanged by value keeps its instance and output, and is rebound to the
  parent's new handlers.

## Impact

- `crates/nx-ir-runtime/src/tree.rs` (visit, rebinding the subtree) and `component.rs` (an instance
  rebound to new handlers). `value.rs` gains a descriptor comparison that resolves handler tokens
  through the instances they were read from.
- `crates/nx-api/tests/codegen/ir_instance_tree_tests.rs`: two scenarios change meaning, and new
  ones cover skipping, rebinding, emits and nesting after a skip.
- The TypeScript runtime has no instance tree, so it is unaffected. The image format, codegen and
  the component lifecycle API are unchanged.
- Docs: `crates/nx-ir-runtime/README.md` and the instance-tree notes in `specs/future.md`.
