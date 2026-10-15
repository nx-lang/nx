---
title: 'Visualizer Declarations'
description: 'How a visualizer declaration tells a viewer, such as the NX reading view, how to present a record, component, union or alias.'
---

A **visualizer declaration** tells a viewer how to present one declaration. A viewer is a tool that
shows NX source to people, such as the `<nx-viewer>` reading view from `@nx-lang/viewer` that the
Visual view of the [playground](/play) shows. It reads an element written over several lines as a
group with a row to each property, and an element on one line as its name and its properties as
`name=value`, without quotes. Without visualizer declarations it reads every element the same way. A
library author knows better, and a visualizer declaration says so:

```nx
import "@nx/visualizer"

type Choice = { value:string label:string description?:string }

visualizer Choice = <ElementVisualizer title=label id=value idPosition=end icon="circle-dot" />
```

Now each `<Choice value="eng" label="Engineering" />` in a list of choices reads as one line: a
circle-dot icon, "Engineering", and `eng` as a quiet identifier at the end.

A visualizer declaration is applied, not weighed. The compiler resolves every visualizer declaration that
applies to a declaration into one result, the same for every reader, and a viewer presents the
declaration exactly as that result says. It changes how source reads, never what it means: a
visualizer declaration runs nothing, and it does not reach NX IR, generated code or a declaration
schema.

## Writing one

A visualizer declaration is a module member, `visualizer Target = Value`:

- `Target` names the declaration the visualizer describes. It is resolved like any other name in the
  module, so it can be qualified (`Form.Submitted`) or imported.
- `Value` is built from the records of the [`@nx/visualizer`](/reference/libraries/visualizer) standard
  library, so the module must `import "@nx/visualizer"`.
- `visualizer` is a keyword only as the first token of a module member. A prop, field, binding or
  function named `visualizer` keeps working.
- A visualizer declaration declares no name, takes no visibility modifier, and cannot be referred to.
  It applies wherever its module is loaded, without being exported or imported.

Which record the value takes depends on what the target is:

| Target | Record |
| --- | --- |
| A record, an action, an inline emitted action, or a component, concrete, abstract or external | `ElementVisualizer` |
| A union whose cases all carry no payload | `UnionVisualizer` |
| An alias of a primitive type | `ValueVisualizer` |

Anything else cannot have a visualizer: a function, a value, a union with a payload case, an alias of
anything but a primitive type, a generic record, component or union, and a derived declaration such
as `User.Update`, `User.Property` or `Card.Props`.

```nx invalid
import "@nx/visualizer"

type Range2 = { T:type start:T end:T }

visualizer Range2 = <ElementVisualizer />   // error: a generic record cannot have a visualizer yet
```

## A survey library

The visualizer declarations below sit beside the types they describe, in a library of survey
questions:

```nx
import "@nx/visualizer"

/// The address of a picture.
export type ImageUrl = string
export type ChoiceLayout = list | grid | chips
export type Choice = { value:string label:string description?:string imageUrl?:ImageUrl }

export abstract component <Question id:string label:string help?:string required:boolean = true />
export component <SingleChoice extends Question layout:ChoiceLayout = list allowsOther:boolean = false
  content choices:Choice+ /> = { <div /> }

visualizer ImageUrl = <ValueVisualizer format=image />
visualizer ChoiceLayout =
  <UnionVisualizer>
    <CaseVisualizer of=list label="a list" />
    <CaseVisualizer of=chips label="chips" example="A B C" />
  </UnionVisualizer>
visualizer Choice = <ElementVisualizer layout=row title=label subtitle=description badge=value />
visualizer Question =
  <ElementVisualizer title=label subtitle=help>
    <PropertyVisualizer of=required whenTrue="Required" whenFalse="Optional" />
    <PropertyVisualizer of=id label="Key" place=details />
  </ElementVisualizer>

let roleQuestion =
  <SingleChoice id="role" label="Which describes your work best?" layout=chips>
    <Choice value="eng" label="Engineering" description="Building and running software" />
    <Choice value="design" label="Design" imageUrl="https://example.com/design.png" />
  </SingleChoice>
```

The compiler resolves these into one visualizer for each declaration. `SingleChoice` has no visualizer
declaration of its own, so the visualizer for `Question` applies to it: its title is `label`, its
subtitle `help`, `required` reads as "Required" or "Optional", and `id` is labelled "Key" and placed
in details. Each choice's title is its `label`, its subtitle its `description` and its badge its
`value`, and every `ImageUrl` is a picture. A viewer presents as much of this as it has room for;
[what a reading shows](#what-a-reading-shows) says what the playground's Visual view uses today.

The playground's *Visualizer declarations* example is a smaller library of the same kind. Open it in
the [playground](/play) and switch to Visual to see the reading follow its visualizer declarations.

## The target is bound for you

Every `@nx/visualizer` record takes the declaration it describes as its type parameter `Target`, and
names that declaration's properties with fields typed `Target.Property`. Inside a visualizer
declaration, `Target` is bound for every `@nx/visualizer` construction that leaves it unset, children
included, so `title=label` and `<PropertyVisualizer of=required … />` need no `Target=`. A property
name is checked against the target, so a misspelling is a type error, and a rename in a library
breaks its visualizer declarations loudly:

```nx invalid
import "@nx/visualizer"

type Choice = { label:string }

visualizer Choice = <ElementVisualizer title=lable />   // error: `lable` is not a case of `Choice.Property`
```

A construction that binds `Target` itself must bind it to the visualizer declaration's target. Outside
a visualizer declaration nothing is bound, and `<ElementVisualizer />` is rejected for leaving `Target`
unspecified, as any [generic record](/reference/syntax/types#generic-records) would be.

### Components name their props

A component is not a record, so it has no `T.Property`. A visualizer declaration on a component binds
`Target` to the component's props record,
[`T.Props`](/reference/syntax/functions#component-props-and-state), and names its props through
`T.Props.Property`. In the survey library, `title=label` on `Question` is checked against
`Question.Props.Property`, which lists `id`, `label`, `help` and `required`. State fields are not
props, so a visualizer declaration cannot name them.

## Values are written out

A visualizer declaration's value may hold only `@nx/visualizer` constructions, string and boolean literals,
and bare names of properties or cases. A braced expression, a reference to a value, a conditional, a
loop, an interpolated string and a call are each rejected. That lets the compiler read every visualizer
declaration without evaluating it:

```nx invalid
import "@nx/visualizer"

type Choice = { label:string }
let prefix = "Pick"

visualizer Choice = <ElementVisualizer name={prefix} />   // error: a visualizer's values must be written out
```

## What a reading shows

The source tree a viewer reads carries each declaration's resolved visualizer: its kind's `name`, the
properties bound to `title`, `subtitle`, `badge`, `image` and `id`, where the id reads, its icon and
layout, and for each property its `label`, its boolean phrases (`whenTrue` and `whenFalse`), its
`place` and its format; for a union, each case's `label` and `example`. Properties are listed in the
order of the `PropertyVisualizer`s, then the rest in declaration order. A visualizer declaration changes how
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

A property's format comes from its `PropertyVisualizer` when that sets one, and otherwise from the
`ValueVisualizer` of the alias its type names, read as written, through `?`, `+` and `*`. In the survey
library every property typed `ImageUrl` shows as a picture, and
`<PropertyVisualizer of=imageUrl format=url />` would make it a link instead. A viewer should load an
`image` or link a `url` only when it is an `http:` or `https:` URL.

## Visualizer declarations in another module

A visualizer declaration does not have to sit beside the declaration it describes. The same syntax works
in any module that can see the target, so a team can write its own wording for a library it does
not own:

```nx fragment
// visualizers.nx, in the team's own program
import "@nx/visualizer"
import "../survey"

visualizer SingleChoice = <ElementVisualizer name="Pick one" />
visualizer ChoiceLayout =
  <UnionVisualizer>
    <CaseVisualizer of=chips label="tags" />
  </UnionVisualizer>
```

The visualizer declarations that apply to a program are those in all of its modules: the entry module,
every module it imports, directly or through others, and the standard libraries. A visualizer
declaration in a module the program does not load does not apply.

A host can also supply visualizer modules the program never imports, such as its own wording for
libraries its users write against. It names them by module identity in the `visualizers` option of a
`sourceTree` request, and they are analyzed like any module, so a host visualizer declaration against a
renamed property fails to type-check:

```ts
viewer.tree = snapshot.sourceTree("nx:///survey/form.nx", {
  visualizers: { modules: ["tenant/visualizers.nx"] }
});
```

Without the `visualizers` option the answer carries no visualizers, exactly as before. A module the request
names but the workspace does not hold is reported as missing, and the program's own visualizer
declarations still apply.

## How visualizer declarations combine

A declaration's resolved visualizer is built by applying visualizer declarations in order, each later one
overriding only what it sets. The order is:

1. Along inheritance, from the furthest base to the declaration itself.
2. For each of those, first the visualizer declaration in the module that declares it, then the one in
   another module of the program, then those in the host's visualizer modules, in the order the host
   names them.

Overriding works attribute by attribute: an attribute a later visualizer declaration sets replaces the
earlier value, and one it leaves unset keeps it. `PropertyVisualizer`s merge by their `of` property and
`CaseVisualizer`s by their `of` case, attribute by attribute too. If the survey library declares
`<PropertyVisualizer of=required whenTrue="Required" whenFalse="Optional" />` for `Question`, and a host
module declares

```nx fragment
import "@nx/visualizer"
import "../survey"

visualizer Question =
  <ElementVisualizer>
    <PropertyVisualizer of=required whenTrue="Must answer" />
  </ElementVisualizer>
```

then `required` reads "Must answer" when true and still "Optional" when false. The other module
above relabels only `chips`, so `list` keeps "a list".

Property order follows the later visualizer declaration: its `PropertyVisualizer`s in its order, then the
earlier ones it does not name, in theirs. If `Question`'s visualizer lists `required` then `id`, and a
visualizer for `SingleChoice` lists `layout` then `allowsOther`, a `SingleChoice`'s properties are listed `layout`,
`allowsOther`, `required`, `id`, then the rest in declaration order.

A base's `name` is not inherited, because it names the base itself. If `Question`'s visualizer set
`name="Question"`, a `SingleChoice` would still be named "Single choice" unless its own visualizer
declaration named it. Every other attribute is inherited.

## Duplicate visualizer declarations

A module declares at most one visualizer for a declaration. A second one is a `duplicate-visualizer` error,
naming the first:

```nx invalid
import "@nx/visualizer"

type Choice = { label:string }

visualizer Choice = <ElementVisualizer title=label />
visualizer Choice = <ElementVisualizer name="Option" />   // error: duplicate-visualizer
```

Two modules of the program that do not declare `Choice` may not both declare a visualizer for it
either, since neither has a better claim to apply later. That is a `duplicate-visualizer` error naming
both, reported on each document whose program loads both: at the visualizer declaration when the
document holds one, and otherwise at the import that brings in the other. Neither of the two
applies. The declaring module, another program module and the host's modules never conflict,
because they apply in order, and two host modules may both declare a visualizer for one target: the one
named later applies later.

## See also
- Reference: [`@nx/visualizer`](/reference/libraries/visualizer)
- Reference: [Modules](/reference/syntax/modules#visualizer-declarations)
- Reference: [Component props and state](/reference/syntax/functions#component-props-and-state)
- Grammar: [nx-grammar.md – Visualizer Declarations](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#visualizer-declarations)
