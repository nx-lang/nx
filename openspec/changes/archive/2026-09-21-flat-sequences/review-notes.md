## Corpus sweep

Searched `examples/`, `sites/playground/src/examples/nx`, and `docs/drawnui-proposal` — 39 `.nx`
files — for the three constructs this change alters.

**Nested sequence types (`[][]`).** No `.nx` file spells one. The only hits in the tree are
TypeScript inside the playground's own DrawnUI implementation
(`src/drawnui/controls/SkiaLabel.ts`, `src/drawnui/react/SkiaShell.tsx`), which is not NX and is
unaffected. Documentation hits were rewritten; the three that remain state the rule rather than
offering an example.

**`for` bodies whose yield is list-typed.** None. Every `for` in the corpus yields one element or
one scalar per iteration, so no result changes shape. A list-yielding body would previously have
inferred `T[][]`, which every annotated site in the corpus would have rejected.

**Conditional children with no `else`.** None — no conditional *child* in the corpus is missing an
`else`, so no corpus file produced a null item before this change. That is not the same as every
conditional having an `else`: `examples/nx/types.nx:80` is an `if loadState is { … }` with no
`else` arm. It sits in a value position and covers every case of its union, so it is exhaustive,
`uncovered` never becomes true for it, and its type and value are what they were.

**Type arguments.** `examples/nx/generic-records.nx` was the one file that instantiated a generic
with a sequence argument (`type Ints = int[]`, `<Page T=Ints/>`); it now illustrates a nullable
argument instead, and `docs/src/content/docs/reference/syntax/types.md` carries the matching edit.

The fiddle catalogue lives in a separate repository (`DrawnUi.FiddleEngine`) and was **not** swept
here, although task 10.1 names it: it pins a published NX release rather than this working tree, so
nothing in it runs against these rules until a release ships them. It has to be swept before the
next NX release that carries this change.

## Corpus diff

Two toolchains were built and each run over **its own** sources: `HEAD` in a detached worktree with
`HEAD`'s `.nx` files, and this working tree with its edited ones, both as
`nxlang run <file> --format json`. Comparing source versions as well as toolchains is the point —
an earlier pass ran both toolchains over the same already-edited sources, which cannot detect a
corpus file that an edit broke, only one that the compiler changed.

The captured stdout/stderr is **byte-identical for all 39 files**, so nothing in the corpus lost a
null item, had a sequence flattened, or acquired a diagnostic. `examples/nx/generic-records.nx` is
the one corpus file this change edits, and it evaluates to the same JSON before and after.

14 files evaluate to a value; the other 25 fail identically under both builds, for reasons that
predate this change:

- `sites/playground/src/examples/nx/*.nx` (20 files) need the DrawnUI component library, which a
  bare `nxlang run` does not supply. They are checked properly by the playground's own
  `check-examples`, which compiles and evaluates all 20 with no diagnostics after this change.
- `examples/nx/complex.nx` uses a ternary and an inline-fragment form the parser does not accept.
- `examples/nx/function.nx` imports `./core`, absent from the build context.
- `examples/nx/types.nx` declares `UserCard` twice.
- `examples/nx/utils/formatting.nx` is a helper module with no `root`.
- `docs/drawnui-proposal/{ui/ui.nx,graphics/graphics.nx}` are proposal sketches that import
  `docs/drawnui-proposal/core`, which is not in the build context; `ui.nx` additionally writes a
  union default as a bare name. Neither failure has anything to do with sequences.
