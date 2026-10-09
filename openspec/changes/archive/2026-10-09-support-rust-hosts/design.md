## Context

See proposal.md for why. What shapes the approach:

- `nx-ir-runtime` depends on `nx-ir` and `nx-value` by path, all three inherit the workspace's
  `version = "0.1.0"`, none has a `description`, and `docs/deployment.md` says Rust crates are not
  part of the release pipeline. Packaging the three together fails today: `dependency nx-ir does not
  specify a version`.
- The release track is tag → draft GitHub Release with verified artifacts → publishing the release
  publishes the attached files without rebuilding them (`package-release-automation`). npm packages
  are packed by `scripts/pack-packages.mjs <version> <dir>` and published in dependency order by
  `scripts/publish-packages.mjs`.
- The runtime dispatches one instance at a time and composes nothing. The only instance tree is
  TypeScript in the DrawnUI fiddle (`nx/src/instances.ts`, 340 lines, 13 tests); the copy the NX
  playground had went when the playground was rebuilt. It reads three things of an instance that
  the TypeScript runtime exposes and the Rust runtime keeps private: the handlers by token, the
  handlers the parent bound by property, and the component's emits.
- The evaluator recurses on the native stack. Two fixed bounds protect it: 1,000 nested expressions
  and 1 MiB of stack measured from where the evaluation began (`MAX_STACK_BYTES` in `eval.rs`),
  reported as `maxStackBytes`.

What the spike measured (`wasm32-unknown-emscripten`, release, the 1 MB stack DrawnUI for Rust
links with):

| Measured | Result |
| --- | --- |
| Recursion to the 1,000-expression bound | a diagnostic while at least 162 KiB of stack was free where the call began |
| The same call begun with 153 KiB free or less | the browser tab crashed; no diagnostic, no clean abort |
| The same call with the budget set to the free stack less 8 to 64 KiB | a `maxStackBytes` diagnostic from every depth tried, down to 2 KiB free |
| The three crates on Rust 1.94.0 and on `wasm32-wasip1` | `cargo check` passes |
| `cargo package -p nx-ir` run twice | the same SHA-256 both times |

So the guard does measure the WebAssembly stack, and what is missing is only that the host cannot
say how much there is. The first two rows were taken before this change and again after it; the
third with the option this change adds. An earlier reading of these numbers put the need near a
quarter megabyte: the probe counted its own frames as 1 KB each, and they are a little over 1.1.
The probe now reports the stack that is free where it calls the runtime.

## Goals / Non-Goals

**Goals:**

- A Rust project builds `nx-ir-runtime` from crates.io at the release's version, with no checkout.
- A Rust host consumes a program whose components use other components by calling an instance
  tree, with the behavior the fiddle's tree has, whether the host draws the output or keeps some
  other structure derived from it.
- An evaluation on a host with a small stack ends in a diagnostic.

**Non-Goals:**

- Publishing the compiler crates, `nxlang` or `nx-lsp`.
- Moving the TypeScript instance tree into `@nx-lang/ir-runtime`. The capability spec is written so
  that it can follow; until then the fiddle keeps its copy.
- Anything a host makes of the output: turning rendered output into controls or into any other
  structure stays with the host, and so does the walk that finds the descriptors to visit.
- A tree the runtime walks and updates by itself, handing the host a set of changes, and any finer
  unit of change than a component. `specs/future.md` records both, with the direction chosen for
  the second.
- Detecting the free stack. No portable call gives it, and the runtime takes no platform
  dependency for it.
- Changing the bound on nested expressions or the TypeScript runtime's limits.

## Decisions

### The instance tree is a module of `nx-ir-runtime`, not a crate of its own

The tree needs to ask which token of an ancestor holds the same handler as a token of a descendant,
which handler a parent bound for an emit, and which actions a component emits. Inside the crate
those are fields of `InstanceData` and of the prepared module. As a separate crate they would have
to become public API: an iteration over an instance's tokens and an accessor for a component's
emits, added only so that one other crate of ours can call them. One crate also means one fewer
name to publish and to pin.

*Alternative considered:* a crate `nx-instance-tree` over new public accessors. It keeps the
runtime crate's "composes nothing" literally true, at the price of a wider permanent surface. The
tree in a module still composes only through `initialize_component` and
`dispatch_component_actions`; the spec requires that its results are the lifecycle's.

### Nodes are addressed by key; what a node holds is shared, not copied

```rust
let mut tree = InstanceTree::new(program);
tree.begin();
let rendered = tree.visit("0", &descriptor, None, &options)?;        // Arc<NxValue>
let child = tree.visit("0/body/2", &child_descriptor, Some("0"), &options)?;
tree.finish();
let effects = tree.dispatch("0/body/2", "h1-1", action, &options)?;   // Vec<HostEffect>, action by value
```

The TypeScript tree hands out node objects and compares them by identity. In Rust the host would
hold borrows of the tree across a pass, so nodes are named by the host's key, and the parent by its
key. A node keeps its descriptor and its rendered output behind `Arc`, and an instance already is
one, so the snapshot that makes a dispatch all-or-nothing clones the map of nodes without copying a
value. `atomically` takes a closure over `&mut InstanceTree` and restores the snapshot when it
returns `Err`.

Whether a visit carries "the same descriptor" is decided by two things together, where TypeScript
compares object identity: the values are equal, and the parent's instance is the very instance the
node was last initialized through. The comparison of values is linear in the descriptor, the same
order as the initialization it avoids. The tree makes it with a walk of its own, which counts
levels and asks the stack budget as the lifecycle's walks do, as it does for the copy a node keeps:
the derived `==` and `clone` of `NxValue` recurse without either.

Values alone are not enough, which the first implementation found out. A dispatch gives an instance
the next generation, so its tokens differ from the last render's, but an initialization starts at
generation one: a parent that is initialized again renders `h1-1` where it rendered `h1-1` before,
for a handler that captured other props. A child's descriptor then reads exactly as the one it
holds and names another handler, and a child left alone would keep one its parent no longer has, so
its emits would stop reaching the parent.

*Alternative considered:* re-initialize a child whenever its parent's instance changed, without
looking at the descriptor. It is cheaper and wrong for a host that visits a different descriptor
under an unchanged parent, which the spec's rule would then not describe.

Which node created a handler is decided by identity: the farthest ancestor whose instance holds
the very handler, the one allocation, as the TypeScript tree compares with `===`. The first
implementation compared handlers by value, the comparison the public `same_handler` makes, and a
review found what that costs: a component nested under itself with equal props and state holds a
handler equal to its ancestor's, and a tap on the inner one patched the outer.

The farthest holder is the creator only while a pass has visited everything that rendered. A
dispatch replaces the
instances it ran against, and until the next pass the nodes below hold handlers nothing above them
holds. The TypeScript tree then runs such a handler against whichever node still holds it, or
returns an emit to the host as unbound, both silently. The Rust tree refuses instead, with
`nx-ir-handler-token`, and it can tell exactly: a node receives a handler only from the instance
it was initialized through, which it keeps, so a farthest holder whose own parent's earlier
instance held the handler was handed it, and a bound handler no ancestor holds was bound by an
instance that is gone. The host runs a pass and sends the event again.

A node names its parent by key, so the tree keeps one invariant the TypeScript tree gets from
holding the parent object: a node's parent is in the tree. A visit that replaces a node drops the
nodes under it, the end of a pass drops the nodes under a dropped one, and a visit under a key
that names no node, or under the node itself, is refused with `nx-ir-instance-key`.

### The tree is not about drawing, and a pass walks only what changed

The tree was ported from a host that draws, and its first wording said so throughout: a "drawing
pass", a node's "place in the drawing", a host that "draws again". Nothing in it draws. What a
host makes of the output is its own business, and a host that keeps a document or an index derived
from the output uses the same three operations. The spec, the README, the doc comments and the two
diagnostics that said "drawn" now say output, pass and visit. "Rendered" stays: it is the
lifecycle's word for a body's evaluated output.

One assumption of a drawing host was in the mechanics, not the words: a pass ended by dropping
every node it had not visited, so every pass had to visit every node, and a host could not follow
changes. A node now records whether it has rendered since a pass last visited it (on being
created, initialized again or dispatched against), and the end of a pass reads that:

- Under a visited node that had rendered, an unvisited node is dropped. The host has just walked
  that node's new output, and the place is not in it. This is the old rule, where it is right.
- Under a visited node that had not rendered, and under a node the pass did not reach, everything
  is kept. The output there is what it was.

`is_settled(key)` says that neither the node nor anything under it has rendered since a pass
visited it, so a host that follows changes stops there. It is one flag a node carries for itself
and one for what is under it, set upward from a node when it renders and recomputed when a pass
ends. A pass with nothing to do is one visit of the root.

An event can reach the tree while a pass is under way, so a visit counts for what the node held
when it was made. A visit records the instance it saw, so a node dispatched against between its
visit and the end of the pass is not taken for walked. And a node's visit counts at the end of the
pass only when the node was initialized through the instance its parent holds then: a child
visited before its parent rendered again in the same pass was read from output the parent no
longer holds. A review found the second half missing, by a pass that walked, took an event that
removed a child's place, walked again and ended with the child still there under a parent called
settled, its state waiting for the place to return.

What a host loses is dropping a node by not visiting it while its parent's output still has the
place, which a host that instantiates list items only while they are in view relied on. `remove`
does that by name.

*Alternatives considered:*
- Keep the full sweep and add an explicit "keep what is under this node" call. It leaves the
  default wrong for a host that follows changes and makes the right behavior something to
  remember.
- Have `dispatch` return the keys that rendered. A host would still need the path from the root
  to each, and a pass that re-initializes a node makes more of them as it goes; asking the tree at
  each node covers both.
- Have the tree walk rendered output itself and hand the host a set of changes. That is the fuller
  model, a tree the runtime maintains, and it needs decisions this change does not make: what key
  a list item has, how output refers to the instances inside it, what happens to items a host
  instantiates lazily. It is recorded in `specs/future.md`; `visit` and `is_settled` are what it
  would be built on.

The unit of change stays the component, and a render reaches everything under the node that
rendered: each node there is handed a descriptor read from another instance and is initialized
again, with the state it held. Stopping at a child whose props did not change would need a child
to keep handlers of a parent instance that is gone, which is what the identity of handlers rules
out. The README says so where it lists what the tree does not do.

### The stack budget is `RuntimeOptions::max_stack_bytes`, default unchanged

`MAX_STACK_BYTES` becomes the default of a `usize` field, exported as
`NX_DEFAULT_MAX_STACK_BYTES`. The evaluator, the type-check walk and the host-boundary walks read
the option where they read the constant today; the diagnostic's `Limit` carries the value in force.
The bound of 1,000 nested expressions stays a constant: it is what makes "no image and no option
reaches the end of the stack" true for a host that has the budget free, and the spike shows it
needs a little under 160 KiB in a release WebAssembly build.

The walks that convert a value at the host boundary (`NxValue` to the runtime's value and back,
and the resolution of a parent's handlers in props) ask the budget as well as counting levels.
They were bounded only by the 256 levels a value may nest, which is enough under a mebibyte and
not under 64 KiB. So do the comparison of two values (`==`, a match pattern, `diff`) and the check
of how deeply state nests after a patch, which the first implementation left out and a review
found by overrunning a 160 KiB thread from a comparison of two 100-level records. The update
helpers (`apply`, `merge`, `diff`, `changed`), `restore_component_instance` and the comparisons of
an instance's handlers take no options and keep the default budget.

A walk that stays within the budget must also be able to say what it found. A diagnostic names
where a value sits by a path the walk builds on the stack, a segment for each level, and prints
it in the deepest frame. Printing recursed once for each segment, and each record level started a
new root that hid the path above it, so a value wrong at its 200th level overran the margin from
inside the formatter: the review's second pass found it. The path is now one chain from the root,
which a record level extends, and it is printed in a loop.

The budget may be set above the default as well as below. How much stack a thread has is the
host's knowledge, and a host with 8 MiB that wants an unoptimized build's frames to fit is as
right to say so as a host with 1 MB. Nothing new becomes reachable: the nesting bound caps the
recursion either way.

*Alternatives considered:*
- A smaller default on `wasm32`. A default that depends on the target hides the question from the
  one party that knows the answer, and Emscripten's stack size is a link flag of the host's.
- Asking Emscripten for the free stack (`emscripten_stack_get_free`). It is one target's FFI in a
  crate that has no `unsafe` and nothing platform-specific. A host on that target can call it and
  pass the result; the README shows that.
- Only documenting "link 2 MB". It leaves a host that does not read it with a crashed tab, and the
  runtime's contract is that a failure is a diagnostic.

### The three crates are versioned by the pack script, pinned to each other exactly

The root manifest gains `[workspace.dependencies]` entries for `nx-ir` and `nx-value` with both a
`path` and a `version`, and the crates take them with `workspace = true`. A new
`scripts/pack-crates.mjs <version> <dir>` sets `[workspace.package] version` and those two
requirements to `=<version>` in the working tree, runs `cargo package` for the three, and writes
the `.crate` files to `<dir>`; it is the crates' counterpart of `pack-packages.mjs` and takes the
same MinVer version. The committed manifests keep `0.1.0`, as the npm manifests keep theirs.

The pin is exact because the runtime reads the format crate's tables and the value crate's
variants directly, and all three are built and tested together at one commit, as the npm packages'
workspace references resolve to the release version.

Each of the three packages only `src/**`, its README and its manifest (`include`). The runtime's
tests read `specs/ir-conformance` from the repository, so they cannot run from a package and would
only add size; the build-from-package check compiles the library.

Every other member of the workspace, the bindings included, gets `publish = false`.

### Publishing runs `cargo publish` at the tag and proves the bytes are the reviewed ones

Cargo has no command that uploads an existing `.crate` file. The publish job therefore checks out
the release's tag, runs the pack script at the release version, and compares the SHA-256 of each
`.crate` it produced with the one attached to the release. Only when all three match does it run
`cargo publish` for the three packages, which orders them and waits for the index. Afterwards it
reads the checksum crates.io records for each version and compares it with the attached file's
again. A mismatch before publishing publishes nothing; a mismatch after fails the job loudly.

This is the one place the track rebuilds something at publish time. It is acceptable because the
rebuild is of an archive of source files, the toolchain is pinned by `rust-toolchain.toml`, and the
result is compared with the reviewed artifact on both sides of the upload.

It is a job of its own, `publish-crates`, beside `publish-packages` and `publish-extension`, so an
outage of one registry does not stop the others. A version crates.io already has is skipped.

*Alternative considered:* uploading the attached file through the registry's documented web API.
It publishes exactly the reviewed bytes with no comparison needed, but the request also carries
the crate's metadata as JSON, which we would have to derive from the manifest ourselves and keep in
step with what Cargo sends.

### Authentication is trusted publishing, with a token for each crate's first version

crates.io accepts a trusted-publisher policy only for a crate that exists. The job uses the
registry's GitHub OIDC exchange when the policy is there and otherwise reads a token from the
`production` environment, as `Registry authentication prefers trusted publishing` already allows.
The first release that includes the crates is published with the token; the maintainer then adds
the three policies and removes the secret. `docs/deployment-setup.md` records both steps. This is
the npm lesson again: a name must exist before it can be trusted.

### WebAssembly builds are checked, not run

`build.yml` gains `cargo check -p nx-ir-runtime` for `wasm32-unknown-emscripten` and
`wasm32-wasip1`; a check needs no Emscripten SDK, only the target's standard library, which
`rust-toolchain.toml` lists. Running the runtime under WebAssembly in CI would need a harness this
repository does not have for the runtime alone. The stack budget's logic is tested natively, where
it is the same code, and the measurement on WebAssembly is recorded in the README with how it was
taken. Once `retire-hir-interpreter` lands, the WebAssembly binding's own tests evaluate through
this runtime on `wasm32-wasip1`.

### The tree's tests compile NX and live in `nx-codegen`

The runtime crate has no compiler, and its tests run images from the corpus. The tree's scenarios
need small purpose-written programs, so they go where the other source-level runtime tests are
(`crates/nx-codegen/src/ir_runtime_tests.rs` and its neighbours), ported from the fiddle's thirteen
with a few external components declared in the test source in place of the DrawnUI catalog.

## Risks / Trade-offs

- [A crate name and a version on crates.io are permanent] → The names are checked free today
  (`nx-ir`, `nx-value`, `nx-ir-runtime`); a version is published only from a reviewed draft
  release; a bad one is yanked and followed by a higher one, as the runbook says for the other
  immutable registries.
- [Publishing the runtime invites dependence on an API that still changes] → The crates carry the
  release's `0.x` version, and the README says the API may change between minor versions. The
  breaking change in this proposal is the first example.
- [`cargo package` might not produce the same bytes on the publish runner as on the release
  runner] → The comparison runs before anything is published and fails closed. If it proves
  unreliable, the fallback is the web API upload described above; the spec's requirement, that the
  registry serves the attached bytes, holds either way.
- [A host sets a stack budget lower than a legitimate program needs] → It gets a diagnostic naming
  `maxStackBytes` and the value, not a crash. The README gives the measured need of a release
  build and says a debug build needs several times more.
- [The Rust tree and the fiddle's TypeScript tree drift apart] → The capability spec is the
  contract and the Rust tests are ported from the fiddle's. Bringing the TypeScript tree into
  `@nx-lang/ir-runtime` under the same spec is the follow-up that removes the second copy.
- [`retire-hir-interpreter` is in flight on the same crate] → No requirement is modified by both.
  Whichever lands second adds `max_stack_bytes` (or `..RuntimeOptions::default()`) at the call
  sites the other introduced.

## Migration Plan

1. Land the change. `main` and pull request builds now package the three crates and check the
   WebAssembly targets; nothing is published.
2. Before the next release tag, add a crates.io token to the `production` environment.
3. Tag and publish the release as usual. The new job publishes the three crates with the token.
4. On crates.io, add the trusted-publisher policy for each crate (repository, `package-publish.yml`,
   environment `production`), then delete the token secret. Later releases use OIDC.

Rollback: remove the `publish-crates` job; published versions stay, yanked if they are wrong. The
`max_stack_bytes` field and the tree are additive in behavior and need no rollback step.

## Open Questions

- Whether the three crates should declare a `rust-version` lower than the workspace's 1.98.1. They
  compile on 1.94.0 today, which is what DrawnUI for Rust asks of its users. Declaring it means
  checking it in CI and giving up newer standard-library APIs in these crates; not declaring it
  means a host raises its own floor. It can be decided at any later release without changing
  anything here.
