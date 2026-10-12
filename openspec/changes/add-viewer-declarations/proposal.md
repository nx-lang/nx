## Why

The viewer reads every element the same way: a card headed by its kind's name split into words,
with one row per attribute in declaration order. That is a fair default, but a library author knows
better. A question's `label` is its title, `id` is a detail most readers can skip, a boolean
`required` reads better as "Required" or "Optional", and the case `chips` reads better as "chips"
under "Shown as" than as a pill next to `layout`. Today there is nowhere to say any of this.

Viewer declarations also have to come from outside a library. A host such as ReachMe reads survey
libraries it does not own and wants its own wording, and a team may want friendlier names for a
third-party UI library. Both cases should use one syntax, checked against the declarations it
describes, so a rename in the library breaks the viewer declaration loudly instead of silently.

This change follows the NX Viewer Hints proposal
(https://claude.ai/artifact/9o4DL9vaJFKKV1WxTeethK), with the keyword, the record names and the merge
rule Bret chose on 2026-10-10.

## What Changes

- A new module member, the viewer declaration `viewer Target = <…Viewer …/>`, tells the viewer how
  to present one declaration. It sits beside the declaration or in any module that can see it, with
  the same syntax in both places. `viewer` is a keyword only as the first token of a module member.
- A new standard library, `@nx/viewer`, holds the viewer vocabulary: `ElementViewer`,
  `PropertyViewer`, `UnionViewer`, `CaseViewer` and `ValueViewer`, with their small unions. A viewer
  declaration is ordinary NX markup, type-checked like any value, so a misspelled property or case is
  a type error.
- Inside a viewer declaration, property names are bare and checked: `title=label` resolves `label`
  against the target's property union. To allow that, a generic record's field may be typed
  `P.Property` for a type parameter `P`, which becomes the argument's property union when the record
  is applied.
- **Component props.** Viewer declarations name a component's props, but
  since #57 a component has no `T.Property`. This change adds a derived record `T.Props` for every
  component, with exactly its props, and so the usual `T.Props.Property` and `T.Props.Update`. A
  viewer declaration on a component reads its props through `T.Props`. `Props` joins `Update`,
  `Property` and `State` as a reserved emitted-action name. A props record for functions is left for
  later.
- Viewer declarations are resolved in the compiler: along inheritance (base first), then across
  sources (the declaring module, other modules of the program, then viewer modules a host supplies),
  with each later viewer declaration overriding only what it sets. Two viewer declarations for one
  target in one module, or in two modules at the same level, are a `duplicate-viewer` error.
- The source tree carries the resolved viewers when a client asks for them, on the declaration
  entries it already lists, and a client may name extra host viewer modules.
- The viewer reads with viewer declarations: a kind's name, a card's title, subtitle, badge and image,
  property labels and order, phrases for booleans, properties placed in a details strip, case labels,
  and value formats such as links, images and colour swatches. The reading stays lossless: a viewer
  declaration can move a property into the folded details strip but cannot hide it.
- The playground's Read view asks for viewer declarations.
- Viewer declarations exist for tools only: a `viewer` declaration emits no NX IR, no generated code
  and no declaration-schema entry, and `T.Props` emits no typegen companion in this change.

## Capabilities

### New Capabilities

- `viewer-declarations`: the `viewer` declaration, which targets it accepts, how a viewer
  declaration's `Target` is bound, the constant form of a viewer declaration value, and how viewer
  declarations are resolved and merged.
- `viewer-library`: the standard library `@nx/viewer` and its records.
- `component-props-type`: the derived `T.Props` record of a component.

### Modified Capabilities

- `record-type-parameters`: a generic record's field can be typed by a type parameter's property
  union.
- `property-references`: `T.Props` is record-shaped, so it has `T.Props.Property`; the diagnostic for
  a component's `T.Property` suggests `T.Props.Property` when the component has no state.
- `update-records`: `T.Props` has `T.Props.Update`; the diagnostic for a component's `T.Update`
  suggests `T.Props.Update` when the component has no state.
- `component-syntax`: `Props` is a reserved emitted-action name.
- `source-tree`: a `viewer` declaration is a `declaration` node, and the answer carries resolved
  viewers when asked.
- `viewer`: viewer declarations shape the reading, and a property can fold into a details strip.
- `playground`: the Read view asks for viewer declarations.
- `cli-code-generation`: `viewer` declarations and `T.Props` generate nothing.
- `editor-syntax-highlighting`: the `viewer` keyword is scoped as a declaration keyword.

## Impact

- **Grammar and HIR:** `crates/nx-syntax/grammar.js` and the generated parser gain the `viewer`
  module member; `crates/nx-hir` lowers it and derives `T.Props` beside `T.State`.
- **Type checking:** `crates/nx-types` checks a viewer declaration's value against its target's viewer
  record, binds `Target`, resolves `P.Property` for a type parameter, and reports viewer diagnostics.
- **Standard library:** a new `crates/nx-api/src/std/viewer/viewer.nx`, registered beside
  `@nx/agent`.
- **Language service and protocol:** `crates/nx-language-service/src/source_tree.rs` resolves viewer
  declarations; `@nx-lang/language-protocol` gains the request option and the viewer declaration
  types; the wasm and `@nx-lang/language-core` sides carry them through.
- **Viewer and playground:** `packages/viewer` renders with viewer declarations; `sites/playground`
  requests them.
- **Editor:** the TextMate grammar in `src/vscode/syntaxes`.
- **Docs:** a website page on viewer declarations, the `@nx/viewer` reference, and `nx-grammar.md`.
- **Not changed:** NX IR, both runtimes, the interpreter, typegen output and the .NET binding. The
  change view and the previewer's inputs panel, which the proposal says should also read viewer
  declarations, are later work.
