## Context

See proposal.md for why. What shapes the approach:

- `nx-language-service` holds a `WorkspaceSnapshot` per set of documents and answers
  `diagnostics`, `document_symbols`, `hover` and `completions` from one analysis: the tree-sitter
  tree from `nx-syntax`, the lowered module from `nx-hir`, and the type checker's results from
  `nx-types`. Ranges leave the service as `EditorRange`: zero-based line and UTF-16 character
  positions with UTF-8 byte offsets.
- `@nx-lang/value-view` already shows an annotated value as a flat list of nodes in text order,
  each parent before its children, with a `parent` index and a role (`NxValueText`). Its consumers
  are used to that shape.
- Declarations are named across modules by `{ module, name }` (`NxDeclarationRef` in the wasm
  SDK), where `module` is the identity of the declaring module.
- Doc comments are attached to declarations and members by a post-parse pass (`doc-comments`);
  ordinary comments are trivia the parser keeps in the tree but the lowered module drops.
- The first consumer is the NX viewer, which renders the tree as a typeset reading of the source
  and must be able to show everything the source says. The second is a change view that diffs two
  versions of a document by structure.

## Goals / Non-Goals

**Goals:**

- One query that gives a tool everything it needs to render or compare a document, with no
  per-position follow-up queries.
- Answers that agree with hover and diagnostics for the same snapshot.
- A coverage guarantee that can be tested, so a renderer built on the tree is lossless.
- Keys that let two versions of a document be compared structurally.

**Non-Goals:**

- Presentation. The tree says `!=`; how a viewer renders it is the viewer's business.
- Viewer hints and identity keys from them. They come with `viewer` declarations, a later change,
  which adds resolved hints to declaration entries.
- The structural diff itself. It is a function over two trees and belongs with its first user.
- Incremental or ranged answers. A few thousand nodes for a large document is acceptable in a
  worker; a range parameter can be added without changing the node shape.
- Stability. The shape is marked unstable until the viewer and the change view have used it.

## Decisions

### A flat list, like `NxValueText`, rather than nested objects

Nodes are a list in source order with parent indices. JSON of a deep nested tree is awkward to
diff and to index, and the value view's consumers already walk this shape. A tool that wants
nesting rebuilds it in one pass.

### Built from the syntax tree, annotated from the lowered module and the checker

Coverage is a property of the source text, so the walk follows the tree-sitter tree, which keeps
every token, comments included. Each syntax node is mapped to the HIR item or expression lowered
from it, through the ranges lowering already records, to read its type and the declaration it
resolves to. A syntax node with no lowered counterpart (an import, a comment, an unparsed region)
still becomes a node; it simply carries no type.

Alternative considered: walking the lowered module. It drops comments, folds some syntax (braced
literals, signed literals) and has no node for text a parse error skipped, so coverage could not be
guaranteed.

### Roles name constructs, not grammar productions

The roles are the constructs a reader recognizes (`element`, `attribute`, `condition`), not the
grammar's nonterminals. The grammar can be refactored without changing the tree, and a renderer
handles a small closed set. Variations that matter to a renderer and not to the role are flags:
`braced`, `parenthesized`, `content`, `handler`, `stateUpdate`, `inherited`, `optional`, `raw`,
and the declaration modifiers `abstract`, `external`, `export` and `private`.

### Facts, not wording

An operator node carries its token as written. A case node carries the case name and the union.
Nothing in the tree is phrased for a reader, so the viewer's wording can change without a contract
change, and other tools are not burdened with it.

### Keys are name paths, positions only where nothing names an item

`roleQuestion.value.choices[2].label` survives an edit to another declaration or another
attribute, which a byte offset does not. A construct in a fixed slot of its parent is keyed by the
slot's name (`value`, `body`, `default`, `test`, `left`, `callee`), so the key reads as the path a
reader would describe. Items of sequences and content are keyed by position because nothing in the
language names them; identity hints on types (a later change) let `choices[2]` become
`choices[engineer]`. A comment or an unparsed region names nothing either, so it is keyed by the
sibling it precedes (`total:comment[0]`), which an edit elsewhere does not move; the rare key two
nodes would still share, as two declarations of one name do mid-edit, takes a `#2` suffix.

### Coverage is tested over the whole repository

The rule "every token belongs to exactly one node, and the only tokens a node owns directly are its
role's punctuation and keywords" is checked by a test that walks the tree-sitter tokens of every
`.nx` file in the repository, examples and specs included, against the answer. A construct the
builder forgets fails the build with the file, the line and the token.

### Exposure follows `documentSymbols`

The query is added wherever `documentSymbols` is: the Rust service, the wasm language snapshot, the
protocol's query list, `@nx-lang/language-core` and the HTTP service. The ABI version of the wasm
module moves with the new export.

## Risks / Trade-offs

- [The tree becomes a de facto public API before its shape settles] → It is marked unstable in each
  package that exposes it, and the change that commits to it is separate.
- [Mapping syntax nodes to lowered ones is incomplete for some constructs] → Such nodes still appear
  with their role and range and without a type; the coverage test does not depend on types, and a
  per-role test lists which roles must carry one.
- [Answer size for large documents] → Measured by the tasks on the question-flow conformance
  program; a range parameter is the fallback.

## Open Questions

- Whether a declaration's entry should list the declarations that refer to it (a "used by" list), or
  whether a tool computes that from the nodes. Computing it is cheap within one document; across a
  program it needs the other documents' trees.
