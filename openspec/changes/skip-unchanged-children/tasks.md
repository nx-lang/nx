## 1. Comparing descriptors across parent instances

- [ ] 1.1 Add a descriptor comparison to the Rust IR runtime that walks the old descriptor (through the node's `through` instance) and the new one (through the parent's current instance) together: values as `same()` compares them, body-visible handler tokens resolved and compared with `handlers_equal`, and handlers bound for the component's emits checked only for presence. Token text is not compared.
- [ ] 1.2 Record old/new handler pairs during the walk. Treat a pairing that isn't one-to-one as a changed descriptor.
- [ ] 1.3 Use the comparison in `InstanceTree::visit` in place of `through_same && same(...)`, keeping today's path for nodes under no parent and for the same instance.

## 2. Rebinding

- [ ] 2.1 Add a crate-private way to build a `ComponentInstance` from another one with handlers substituted by allocation, across `props`, `handler_props`, `state`, `handlers` and the captures of the instance's own handlers. Rebuilt handlers extend the substitution map. Keep the generation and the order of `handlers`.
- [ ] 2.2 In `visit`, when the comparison matches through another parent instance, rebind the node, set its descriptor and `through` to the new ones, keep its rendered output Arc, and leave `unwalked` and `unsettled` as they were.
- [ ] 2.3 Rebind, top-down, every node initialized through an instance that was rebound, pointing each one's `through` at the rebound instance. Make sure it runs inside `atomically` and under the visit's stack budget.

## 3. Tests

- [ ] 3.1 Update the instance-tree tests for the changed scenarios: a render reaches only changed children, and an equal descriptor from a parent initialized again keeps the child.
- [ ] 3.2 Add tests for a visible handler with another capture (re-initializes), a kept child's emit reaching the parent, a handler handed through a kept child, tokens read before rebinding, a node under a kept child, and a component nested under itself after a skip (RF1).
- [ ] 3.3 Add a test that `rendered` hands out the same allocation for a kept child and that `is_settled` is true for it after the pass, and a test that a pass failing after a rebinding leaves the tree as it was.

## 4. Docs and completion

- [ ] 4.1 Update `crates/nx-ir-runtime/README.md`: a pass walks only what changed now also holds under a parent that rendered again, and how handlers are rebound.
- [ ] 4.2 Update the instance-tree section of `specs/future.md`: step 1 is done, narrowing captures to what a handler reads is what would extend it to content handlers, and read tracking arrives with environment values.
- [ ] 4.3 Run the Rust IR runtime and nx-api test suites, `cargo fmt` and `cargo clippy`, then the review loop, then validate strictly, sync and archive.
