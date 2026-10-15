---
title: 'Viewer Declarations'
description: 'How a viewer declaration tells a viewer, such as the NX reading view, how to present a record, component, union or alias.'
---

A **viewer declaration** tells a viewer how to present one declaration. A viewer is a tool that
shows NX source to people, such as the `<nx-viewer>` reading view from `@nx-lang/viewer` that the
Visual view of the [playground](/play) shows. It reads an element written over several lines as a
group with a row to each property, and an element on one line as its name and its properties as
`name=value`, without quotes. Without viewer declarations it reads every element the same way. A
library author knows better, and a viewer declaration says so:

```nx
import "@nx/viewer"

type Choice = { value:string label:string description?:string }

viewer Choice = <ElementViewer title=label id=value idPosition=end icon="circle-dot" />
```

Now each `<Choice value="eng" label="Engineering" />` in a list of choices reads as one line: a
circle-dot icon, "Engineering", and `eng` as a quiet identifier at the end.

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

The compiler resolves these into one viewer for each declaration. `SingleChoice` has no viewer
declaration of its own, so the viewer for `Question` applies to it: its title is `label`, its
subtitle `help`, `required` reads as "Required" or "Optional", and `id` is labelled "Key" and placed
in details. Each choice's title is its `label`, its subtitle its `description` and its badge its
`value`, and every `ImageUrl` is a picture. A viewer presents as much of this as it has room for;
[what a reading shows](#what-a-reading-shows) says what the playground's Visual view uses today.

The playground's *Viewer declarations* example is a smaller library of the same kind. Open it in
the [playground](/play) and switch to Visual to see the reading follow its viewer declarations.

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

The source tree a viewer reads carries each declaration's resolved viewer: its kind's `name`, the
properties bound to `title`, `subtitle`, `badge`, `image` and `id`, where the id reads, its icon and
layout, and for each property its `label`, its boolean phrases (`whenTrue` and `whenFalse`), its
`place` and its format; for a union, each case's `label` and `example`. Properties are listed in the
order of the `PropertyViewer`s, then the rest in declaration order. A viewer declaration changes how
source reads, never what it means, and it hides nothing: a property placed in details is still there
for a viewer to show.

The playground's Visual view uses:

- **Title.** The value of the property bound to `title` reads alone on an element's line, without its
  name: `Engineering` rather than `label=Engineering`.
- **Id.** The value of the property bound to `id` reads as one quiet token: after the element's name,
  or with `idPosition=end`, last on its line, so a list's labels come first.
- **Icon.** `icon` names a [Lucide](https://lucide.dev/icons) icon that an element starts with when
  it starts a line of its own. A kind that extends another inherits it, and an element with none takes
  one for its kind of declaration.
- **Formats.** A string bound to a property with the format `id` reads as a quiet token, and one with
  the format `color` reads with a small square of its color before it. A format changes how a value
  looks, never what it is, and does nothing to a value that is not a string literal.

A property's format comes from its `PropertyViewer` when that sets one, and otherwise from the
`ValueViewer` of the alias its type names, read as written, through `?`, `+` and `*`. In the survey
library every property typed `ImageUrl` shows as a picture, and
`<PropertyViewer of=imageUrl format=url />` would make it a link instead. A viewer should load an
`image` or link a `url` only when it is an `http:` or `https:` URL.

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
viewer for `SingleChoice` lists `layout` then `allowsOther`, a `SingleChoice`'s properties are listed `layout`,
`allowsOther`, `required`, `id`, then the rest in declaration order.

A base's `name` is not inherited, because it names the base itself. If `Question`'s viewer set
`name="Question"`, a `SingleChoice` would still be named "Single choice" unless its own viewer
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
