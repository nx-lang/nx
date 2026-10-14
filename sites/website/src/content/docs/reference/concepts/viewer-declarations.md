---
title: 'Viewer Declarations'
description: 'How a viewer declaration tells a viewer, such as the NX reading view, how to present a record, component, union or alias.'
---

A **viewer declaration** tells a viewer how to present one declaration. A viewer is a tool that
shows NX source to people, such as the `<nx-viewer>` reading view from `@nx-lang/viewer`, which
draws elements as cards and values without their quotes, or the Read view of the
[playground](/play). Without viewer declarations it reads every element the same way: a card headed
by its kind's name split into words, with one row per attribute in declaration order. A library
author knows better, and a viewer declaration says so:

```nx
import "@nx/viewer"

type Choice = { value:string label:string description?:string }

viewer Choice = <ElementViewer layout=row title=label subtitle=description badge=value />
```

Now a `<Choice value="eng" label="Engineering" description="Building and running software" />`
reads as one line: "Engineering", with "Building and running software" under it and `eng` as a
badge.

A viewer declaration is applied, not weighed. The compiler resolves every viewer declaration that
applies to a declaration into one result, the same for every reader, and a viewer presents the
declaration exactly as that result says. It changes how source reads, never what it means: a
viewer declaration runs nothing, and it does not reach NX IR, generated code or a declaration
schema.

## Writing one

A viewer declaration is a module member, `viewer Target = Value`:

- `Target` names the declaration the viewer describes. It is resolved like any other name in the
  module, so it can be qualified (`Form.Submitted`) or imported.
- `Value` is built from the records of the [`@nx/viewer`](/reference/libraries/viewer) standard
  library, so the module must `import "@nx/viewer"`.
- `viewer` is a keyword only as the first token of a module member. A prop, field, binding or
  function named `viewer` keeps working.
- A viewer declaration declares no name, takes no visibility modifier, and cannot be referred to.
  It applies wherever its module is loaded, without being exported or imported.

Which record the value takes depends on what the target is:

| Target | Record |
| --- | --- |
| A record, an action, an inline emitted action, or a component, concrete, abstract or external | `ElementViewer` |
| A union whose cases all carry no payload | `UnionViewer` |
| An alias of a primitive type | `ValueViewer` |

Anything else cannot have a viewer: a function, a value, a union with a payload case, an alias of
anything but a primitive type, a generic record, component or union, and a derived declaration such
as `User.Update`, `User.Property` or `Card.Props`.

```nx invalid
import "@nx/viewer"

type Range2 = { T:type start:T end:T }

viewer Range2 = <ElementViewer />   // error: a generic record cannot have a viewer yet
```

## A survey library

The viewer declarations below sit beside the types they describe, in a library of survey
questions:

```nx
import "@nx/viewer"

/// The address of a picture.
export type ImageUrl = string
export type ChoiceLayout = list | grid | chips
export type Choice = { value:string label:string description?:string imageUrl?:ImageUrl }

export abstract component <Question id:string label:string help?:string required:boolean = true />
export component <SingleChoice extends Question layout:ChoiceLayout = list allowsOther:boolean = false
  content choices:Choice+ /> = { <div /> }

viewer ImageUrl = <ValueViewer format=image />
viewer ChoiceLayout =
  <UnionViewer>
    <CaseViewer of=list label="a list" />
    <CaseViewer of=chips label="chips" example="A B C" />
  </UnionViewer>
viewer Choice = <ElementViewer layout=row title=label subtitle=description badge=value />
viewer Question =
  <ElementViewer title=label subtitle=help>
    <PropertyViewer of=required whenTrue="Required" whenFalse="Optional" />
    <PropertyViewer of=id label="Key" place=details />
  </ElementViewer>

let roleQuestion =
  <SingleChoice id="role" label="Which describes your work best?" layout=chips>
    <Choice value="eng" label="Engineering" description="Building and running software" />
    <Choice value="design" label="Design" imageUrl="https://example.com/design.png" />
  </SingleChoice>
```

In a reading of `roleQuestion`:

- The card is headed "Single choice", the declaration's name split into words, with
  "Which describes your work best?" as its title. `SingleChoice` has no viewer declaration of its
  own, so the viewer for `Question` applies to it.
- `required` is left to its default, `true`, so the card shows the phrase "Required" in place of a
  `required` row.
- The `id` row is labelled "Key" and folded into the details strip at the foot of the card.
- The `layout` row shows a pill reading "chips", with the case name and the example "A B C" in its
  hover.
- Each choice reads as one line: its label, its description under it and its value as a badge.
  Everything else, here the second choice's `imageUrl`, is folded into the line's details strip,
  where it shows as a small picture beside its URL.

The playground's *Viewer declarations* example is a smaller library of the same kind. Open it in
the [playground](/play) and switch to Read to see the reading follow its viewer declarations.

## The target is bound for you

Every `@nx/viewer` record takes the declaration it describes as its type parameter `Target`, and
names that declaration's properties with fields typed `Target.Property`. Inside a viewer
declaration, `Target` is bound for every `@nx/viewer` construction that leaves it unset, children
included, so `title=label` and `<PropertyViewer of=required … />` need no `Target=`. A property
name is checked against the target, so a misspelling is a type error, and a rename in a library
breaks its viewer declarations loudly:

```nx invalid
import "@nx/viewer"

type Choice = { label:string }

viewer Choice = <ElementViewer title=lable />   // error: `lable` is not a case of `Choice.Property`
```

A construction that binds `Target` itself must bind it to the viewer declaration's target. Outside
a viewer declaration nothing is bound, and `<ElementViewer />` is rejected for leaving `Target`
unspecified, as any [generic record](/reference/syntax/types#generic-records) would be.

### Components name their props

A component is not a record, so it has no `T.Property`. A viewer declaration on a component binds
`Target` to the component's props record,
[`T.Props`](/reference/syntax/functions#component-props-and-state), and names its props through
`T.Props.Property`. In the survey library, `title=label` on `Question` is checked against
`Question.Props.Property`, which lists `id`, `label`, `help` and `required`. State fields are not
props, so a viewer declaration cannot name them.

## Values are written out

A viewer declaration's value may hold only `@nx/viewer` constructions, string and boolean literals,
and bare names of properties or cases. A braced expression, a reference to a value, a conditional, a
loop, an interpolated string and a call are each rejected. That lets the compiler read every viewer
declaration without evaluating it:

```nx invalid
import "@nx/viewer"

type Choice = { label:string }
let prefix = "Pick"

viewer Choice = <ElementViewer name={prefix} />   // error: a viewer's values must be written out
```

## What a reading shows

A viewer such as `<nx-viewer>` uses the resolved viewer declarations this way:

- **Kind name.** A card is headed by the viewer's `name` when it sets one, and otherwise by the
  declaration's name split into words, so `SingleChoice` reads "Single choice".
- **Title, subtitle, badge and image.** The attributes bound to these properties move into those
  parts of the card, and are not shown again as rows. A reader can still open each one's source.
- **Labels.** A row is labelled with its property's `label`, and otherwise with the property's
  name.
- **Order.** Rows follow the order of the `PropertyViewer`s, then the remaining properties in
  declaration order.
- **Boolean phrases.** A boolean property with `whenTrue` or `whenFalse` reads as that phrase in
  place of its row, for a value written out or left to its default. When the phrase for the value
  is not set, the row reads as usual, so nothing turns blank.
- **Details.** `place=details` moves a property into a strip at the foot of the card, folded until
  a reader opens it.
- **Layouts.** `card` is the default. `row` shows the element as one line holding its heading,
  title, subtitle, badge and picture, and `inline` shows it within the line that holds it, as its
  heading and title. Either way every other row goes into the details strip.
- **Cases.** A case's `label` shows in its pill, with the case name and its `example` in the hover.
- **Formats.** A string literal bound to a property with a format is shown by it: `url` and `email`
  as links, `phone` as a link that dials, `image` as a small picture with its URL, `color` with a
  swatch, `markdown` as prose, `text` as a plain value and `id` as an identifier, quiet and set
  apart as one token. A format changes how a value looks, never
  what it is, and does nothing to a value that is not a string literal.

The reading stays lossless: a viewer declaration can fold a property into the details strip but
cannot hide it.

A property's format comes from its `PropertyViewer` when that sets one, and otherwise from the
`ValueViewer` of the alias its type names, read as written, through `?`, `+` and `*`. In the survey
library every property typed `ImageUrl` shows as a picture, and
`<PropertyViewer of=imageUrl format=url />` would show one as a link instead. An `image` or `url`
value is loaded or linked only when it is an `http:` or `https:` URL, and a host can turn image
loading off.

## Viewer declarations in another module

A viewer declaration does not have to sit beside the declaration it describes. The same syntax works
in any module that can see the target, so a team can write its own wording for a library it does
not own:

```nx fragment
// viewers.nx, in the team's own program
import "@nx/viewer"
import "../survey"

viewer SingleChoice = <ElementViewer name="Pick one" />
viewer ChoiceLayout =
  <UnionViewer>
    <CaseViewer of=chips label="tags" />
  </UnionViewer>
```

The viewer declarations that apply to a program are those in all of its modules: the entry module,
every module it imports, directly or through others, and the standard libraries. A viewer
declaration in a module the program does not load does not apply.

A host can also supply viewer modules the program never imports, such as its own wording for
libraries its users write against. It names them by module identity in the `viewers` option of a
`sourceTree` request, and they are analyzed like any module, so a host viewer declaration against a
renamed property fails to type-check:

```ts
viewer.tree = snapshot.sourceTree("nx:///survey/form.nx", {
  viewers: { modules: ["tenant/viewers.nx"] }
});
```

Without the `viewers` option the answer carries no viewers, exactly as before. A module the request
names but the workspace does not hold is reported as missing, and the program's own viewer
declarations still apply.

## How viewer declarations combine

A declaration's resolved viewer is built by applying viewer declarations in order, each later one
overriding only what it sets. The order is:

1. Along inheritance, from the furthest base to the declaration itself.
2. For each of those, first the viewer declaration in the module that declares it, then the one in
   another module of the program, then those in the host's viewer modules, in the order the host
   names them.

Overriding works attribute by attribute: an attribute a later viewer declaration sets replaces the
earlier value, and one it leaves unset keeps it. `PropertyViewer`s merge by their `of` property and
`CaseViewer`s by their `of` case, attribute by attribute too. If the survey library declares
`<PropertyViewer of=required whenTrue="Required" whenFalse="Optional" />` for `Question`, and a host
module declares

```nx fragment
import "@nx/viewer"
import "../survey"

viewer Question =
  <ElementViewer>
    <PropertyViewer of=required whenTrue="Must answer" />
  </ElementViewer>
```

then `required` reads "Must answer" when true and still "Optional" when false. The other module
above relabels only `chips`, so `list` keeps "a list".

Property order follows the later viewer declaration: its `PropertyViewer`s in its order, then the
earlier ones it does not name, in theirs. If `Question`'s viewer lists `required` then `id`, and a
viewer for `SingleChoice` lists `layout` then `allowsOther`, a `SingleChoice` card lists `layout`,
`allowsOther`, `required`, `id`, then the rest in declaration order.

A base's `name` is not inherited, because it names the base itself. If `Question`'s viewer set
`name="Question"`, a `SingleChoice` card would still read "Single choice" unless its own viewer
declaration named it. Every other attribute is inherited.

## Duplicate viewer declarations

A module declares at most one viewer for a declaration. A second one is a `duplicate-viewer` error,
naming the first:

```nx invalid
import "@nx/viewer"

type Choice = { label:string }

viewer Choice = <ElementViewer title=label />
viewer Choice = <ElementViewer name="Option" />   // error: duplicate-viewer
```

Two modules of the program that do not declare `Choice` may not both declare a viewer for it
either, since neither has a better claim to apply later. That is a `duplicate-viewer` error naming
both, reported on each document whose program loads both: at the viewer declaration when the
document holds one, and otherwise at the import that brings in the other. Neither of the two
applies. The declaring module, another program module and the host's modules never conflict,
because they apply in order, and two host modules may both declare a viewer for one target: the one
named later applies later.

## See also
- Reference: [`@nx/viewer`](/reference/libraries/viewer)
- Reference: [Modules](/reference/syntax/modules#viewer-declarations)
- Reference: [Component props and state](/reference/syntax/functions#component-props-and-state)
- Grammar: [nx-grammar.md – Viewer Declarations](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#viewer-declarations)
