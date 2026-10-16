---
title: 'The visualizer library'
description: 'Reference for @nx/visualizer, the standard library of records that visualizer declarations are written with.'
sidebar:
  label: '@nx/visualizer'
---

`@nx/visualizer` is a [standard library](/reference/syntax/modules#standard-libraries): NX source the
compiler carries. It declares the records a
[visualizer declaration](/reference/concepts/visualizer-declarations) is written with, which tell a viewer
how to present a declaration. Import it by name:

```nx
import "@nx/visualizer"

type Choice = { value:string label:string }

visualizer Choice = <ElementVisualizer title=label id=value idPosition=end />
```

:::caution[Unstable]
`@nx/visualizer` is **unstable**. Its declarations may change incompatibly in any NX release, including
a patch release, with no deprecation period. A change reaches you only when you move to a new
release of the NX packages, and every release that changes the library lists the change under
`@nx/visualizer` in its release notes. Visualizer declarations you have written may need updating when a
record changes; nothing you have compiled does, because visualizer declarations never reach NX IR.
:::

The library holds types only. It has no functions and runs nothing: a viewer reads the resolved
visualizer declarations from the language service and decides how to draw them.

## The `Target` parameter

Every record in the library takes the declaration it describes as its type parameter `Target`, and
names that declaration's properties through `Target.Property`, so `title=label` is checked against
the target's fields. Inside a `visualizer` declaration, `Target` is bound for you: write
`<ElementVisualizer title=label />`, not `<ElementVisualizer Target=Choice title=label />`. For a component,
`Target` is its props record, [`T.Props`](/reference/syntax/functions#component-props-and-state), so
property names are its props.

Outside a visualizer declaration the ordinary rule for
[generic records](/reference/syntax/types#generic-records) holds, and a construction that leaves
`Target` out is rejected:

```nx invalid
import "@nx/visualizer"

let visualizer = <ElementVisualizer />   // error: `Target` was not specified
```

Which record a visualizer declaration takes depends on its target:

| Target | Record |
| --- | --- |
| A record, an action, an inline emitted action, or a component | `ElementVisualizer` |
| An alias of a primitive type | `ValueVisualizer` |

A union cannot have a visualizer yet.

## Elements

| Type | Fields | Notes |
| --- | --- | --- |
| `ElementVisualizer` | `title?`, `id?`, `idPosition: IdPosition = start`, `icon?`, content `properties?: PropertyVisualizer+` | `title` and `id` each name a property of the target. |
| `IdPosition` | `start`, `end` | Where the element's id reads. |
| `PropertyVisualizer` | `property`, `format?: ValueFormat` | `property` names the property it describes. |

- `title` is the property whose value is the element's title. The playground's Visual view shows its
  value alone on the element's line, without the property's name or quotes, and after the header of
  a folded element.
- `id` is the property whose value identifies the element: a key that names it and is kept, such
  as an id stored in a database. Visual shows its value alone, as one quiet token: right after the
  element's name when `idPosition` is `start`, the default, or last on its line, after its other
  properties, when it is `end`. `end` suits an element that reads on one line, such as a choice, so
  its label comes first.
- `icon` names the icon that marks the element where it starts, from the
  [Lucide](https://lucide.dev/icons) set, such as `"list-checks"`. Visual shows it before an element
  that starts a line of its own; an element with none takes one for its kind of declaration. A kind
  that extends another inherits it.
- `properties` says how individual properties read: a `PropertyVisualizer`'s `format` says how the
  property's string value is shown, in place of the format its type gives it.

```nx
import "@nx/visualizer"

export abstract component <Question id:string label:string help?:string required:boolean = true />

visualizer Question = <ElementVisualizer title=label id=id icon="circle-help" />
```

## Values

| Type | Fields | Notes |
| --- | --- | --- |
| `ValueVisualizer` | `format: ValueFormat` | How a string value of the alias is shown. |
| `ValueFormat` | `id`, `color` | A format changes how the value looks, never what it is. |

| Format | Shows a string value |
| --- | --- |
| `id` | As an identifier: a key that names one thing and is kept, such as an id stored in a database. |
| `color` | With a small square of the color it names. |

A `ValueVisualizer` on an alias formats every property typed by that alias. A `PropertyVisualizer`'s
`format` sets it for one property:

```nx
import "@nx/visualizer"

export type Color = string
export type Theme = { name:string accent:Color code:string }

visualizer Color = <ValueVisualizer format=color />
visualizer Theme =
  <ElementVisualizer title=name>
    <PropertyVisualizer property=code format=id />
  </ElementVisualizer>
```

## The library source

```nx
// The NX visualizer library: the records a `visualizer` declaration is written with, which tell a
// viewer how to present a declaration. Import it with `import "@nx/visualizer"`.
//
// UNSTABLE. These declarations may change incompatibly in any NX release, including a patch
// release. See https://nxlang.org/reference/libraries/visualizer.
//
// Each record takes the declaration it describes as its type parameter `Target`. Inside a
// `visualizer` declaration, `Target` is bound for you: write `visualizer Choice =
// <ElementVisualizer title=label />`, not `<ElementVisualizer Target=Choice ... />`. For a
// component, `Target` is its props record, so property names are its props.

/// How a viewer presents an element of a record, action or component.
export type ElementVisualizer = {
  // The declaration this visualizer describes. Bound by the `visualizer` declaration.
  Target: type
  /// The property whose value is the element's title: it reads alone on the element's line, and
  /// says what a folded element holds.
  title?: Target.Property
  /// The property whose value identifies the element: a key that names it and is kept. It reads as
  /// one quiet token, its value alone, where `idPosition` puts it.
  id?: Target.Property
  /// Where the element's id reads.
  idPosition: IdPosition = start
  /// The name of the icon that marks the element where it starts, from the Lucide set
  /// (https://lucide.dev/icons), such as `"list-checks"`. A kind that extends another inherits it.
  icon?: string
  /// How individual properties read.
  content properties?: <PropertyVisualizer Target=Target/>+
}

/// Where an element's id reads.
export type IdPosition =
  /// Right after the element's name.
  | start
  /// Last on the element's line, after its other properties.
  | end

/// How one property of an element reads.
export type PropertyVisualizer = {
  // The declaration whose property this describes. Bound by the `visualizer` declaration.
  Target: type
  /// The property this describes.
  property: Target.Property
  /// How the property's string value is shown.
  format?: ValueFormat
}

/// How a viewer presents values of a scalar type.
export type ValueVisualizer = {
  // The type this visualizer describes. Bound by the `visualizer` declaration.
  Target: type
  /// How a string value of the type is shown.
  format: ValueFormat
}

/// How a string value is shown. A format changes how the value looks, never what it is.
export type ValueFormat =
  /// As an identifier: a key that names one thing and is kept, such as an id stored in a database.
  | id
  /// With a small square of the color it names.
  | color
```

## See also
- Reference: [Visualizer Declarations](/reference/concepts/visualizer-declarations)
- Reference: [Modules](/reference/syntax/modules#standard-libraries)
- Reference: [Generic records](/reference/syntax/types#generic-records)
