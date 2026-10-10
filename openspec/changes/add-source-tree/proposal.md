## Why

NX is written mostly by language models and read mostly by people checking that it says what they
meant. The planned NX viewer shows a document as a typeset reading of its source: each element as a
card headed by its kind, conditions as sentences, references with the target's title pulled in.
To do that it needs to know, for every piece of the source, what it is, what type it has and what
it refers to. Today the language service answers that only one position at a time, through hover,
and `documentSymbols` lists top-level declarations without their contents. A viewer built on hover
would issue a query per token, and one built on its own parse would disagree with the editor
whenever the two analyses differ.

The same knowledge serves more than the viewer: a change view that reports an edit as "added the
choice Product to `role`" rather than as a line diff, an AI summarizing its own edit, structural
edits, outlines and lint rules. So the query is a general, public projection of the source, and the
viewer is its first user.

## What Changes

- The language service gains a query, `sourceTree(uri)`, answered from the same workspace snapshot as
  hover and diagnostics. It returns every piece of the document as a typed node in a flat list in
  source order, each with its role, its range, its parent, a stable key, its resolved type when it
  has one, and the declaration it refers to; and a table of the declarations those nodes refer to,
  from this module or any other, with their properties, defaults, cases, bases and doc comments.
- Every token of a document belongs to exactly one node, so a tool that renders the tree can show
  everything the source says. A conformance test checks this over every `.nx` file in the
  repository.
- A node's key is a path of declaration, attribute and item names that survives edits elsewhere in
  the document, so two trees can be compared by key.
- A document that does not fully parse still has a tree; what did not parse is an `unparsed` node.
- The language protocol defines the query, the wasm language snapshot and
  `@nx-lang/language-core` expose it, and the HTTP service answers it.
- The tree is marked unstable in the packages that expose it until the viewer and the change view
  have used it.

Viewer hints (`viewer` declarations and their resolution) and identity keys from those hints are not
part of this change; they arrive with the change that adds `viewer` declarations, which extends a
declaration entry with its resolved hints.

## Capabilities

### New Capabilities

- `source-tree`: the source tree query, its node roles, keys, declaration table, the token coverage
  invariant, and its exposure through the language service, the protocol and the wasm SDK.

### Modified Capabilities

- `language-protocol`: the protocol defines five queries, adding `sourceTree`.

## Impact

- `crates/nx-language-service`: a `source_tree` query on `WorkspaceSnapshot` and its serializable
  types, built from the parsed tree, the lowered module and the type checker's results the snapshot
  already holds.
- `bindings/wasm`: the language snapshot export and its TypeScript types; the ABI version moves.
- `packages/language-protocol`, `packages/language-core`, `packages/language-http`: the query's request
  and answer shapes, the service method, the HTTP route.
- Tests: Rust tests per role and for keys; a conformance test over every `.nx` file in the
  repository for token coverage; a wasm parity test that the TypeScript and Rust answers agree.
- No language change and no change to evaluation, the IR or the wire format.
