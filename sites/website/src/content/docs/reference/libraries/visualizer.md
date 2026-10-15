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

visualizer Choice = <ElementVisualizer title=label badge=value />
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
| A union whose cases all carry no payload | `UnionVisualizer` |
| An alias of a primitive type | `ValueVisualizer` |

## Elements

| Type | Fields | Notes |
| --- | --- | --- |
| `ElementVisualizer` | `name?`, `title?`, `subtitle?`, `badge?`, `image?`, `id?`, `idPosition: IdPosition = beginning`, `icon?`, `layout: ElementLayout = card`, content `properties?: PropertyVisualizer+` | `title`, `subtitle`, `badge`, `image` and `id` each name a property of the target. |
| `IdPosition` | `beginning`, `end` | Where the element's id reads. |
| `ElementLayout` | `card`, `row`, `inline` | How the element is laid out. |
| `PropertyVisualizer` | `of`, `label?`, `whenTrue?`, `whenFalse?`, `format?: ValueFormat`, `place: Placement = body` | `of` names the property it describes. |
| `Placement` | `body`, `details` | Where the property is shown. |

- `name` is the kind's name as readers see it, in place of the declaration's name split into words.
  A kind that extends another does not inherit it.
- `title` is the property whose value is the element's title, `subtitle` the one that reads under
  it, `badge` the one shown as a badge beside the heading, and `image` the one whose value is the
  URL of the element's picture.
- `id` is the property whose value identifies the element: a key that names it and is kept, such
  as an id stored in a database. A source reading shows its value alone, without the property's
  name or quotes, as one quiet token: right after the element's name when `idPosition` is
  `beginning`, the default, or last in its tag, after the properties that stay, when it is `end`.
  `end` suits an element that reads on one line, such as a choice, so its label comes first and
  the labels of a list line up.
- `icon` names the icon that marks the element where it starts, from the
  [Lucide](https://lucide.dev/icons) set, such as `"list-checks"`. A source reading shows it before
  an element that starts a line of its own; an element with none takes one for its kind of
  declaration. A kind that extends another inherits it.
- `card` is a heading and a row per property. `row` is one line holding the heading, title,
  subtitle, badge and picture, with the rest folded. `inline` shows the element within the line that
  holds it, as its heading and title, with the rest folded.
- `properties` lists how individual properties read, in the order they should be listed. Properties
  not named follow in declaration order.
- `label` replaces the property's name. `whenTrue` and `whenFalse` are what a boolean property reads
  as, in place of its row, when it is true or false. `format` says how its string value is shown.
- `body` keeps the property among the element's rows. `details` moves it to a strip at the foot of
  the element, folded until a reader opens it. Nothing is hidden.

```nx
import "@nx/visualizer"

export abstract component <Question id:string label:string help?:string required:boolean = true />

visualizer Question =
  <ElementVisualizer title=label subtitle=help>
    <PropertyVisualizer of=required whenTrue="Required" whenFalse="Optional" />
    <PropertyVisualizer of=id label="Key" place=details />
  </ElementVisualizer>
```

## Unions

| Type | Fields | Notes |
| --- | --- | --- |
| `UnionVisualizer` | content `cases?: CaseVisualizer+` | How individual cases read. |
| `CaseVisualizer` | `of`, `label?`, `example?` | `of` names a case of the target union. |

A case's `label` replaces its name. Its `example` is a short example of what the case looks like,
shown with its name:

```nx
import "@nx/visualizer"

export type ChoiceLayout = list | grid | chips

visualizer ChoiceLayout =
  <UnionVisualizer>
    <CaseVisualizer of=list label="a list" />
    <CaseVisualizer of=chips label="chips" example="A B C" />
  </UnionVisualizer>
```

A union with a payload case cannot have a visualizer, because a payload case cannot be named without
its payload.

## Values

| Type | Fields | Notes |
| --- | --- | --- |
| `ValueVisualizer` | `format: ValueFormat` | How a string value of the alias is shown. |
| `ValueFormat` | `text`, `markdown`, `url`, `image`, `color`, `email`, `phone`, `id` | A format changes how the value looks, never what it is. |

| Format | Shows a string value |
| --- | --- |
| `text` | As plain text. |
| `markdown` | As Markdown prose. |
| `url` | As a link. |
| `image` | As a small picture, when the URL is `https:` or `http:` and the viewer loads images. |
| `color` | As a swatch beside the text. |
| `email` | As a link that writes an email. |
| `phone` | As a link that dials. |
| `id` | As an identifier: a key that names one thing and is kept, such as an id stored in a database. |

A `ValueVisualizer` on an alias formats every property typed by that alias. A `PropertyVisualizer`'s
`format` overrides it for one property:

```nx
import "@nx/visualizer"

export type ImageUrl = string
export type Choice = { label:string imageUrl?:ImageUrl website?:string }

visualizer ImageUrl = <ValueVisualizer format=image />
visualizer Choice =
  <ElementVisualizer title=label image=imageUrl>
    <PropertyVisualizer of=website format=url />
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
  /// The kind's name as readers see it, in place of the declaration's name split into words. A
  /// kind that extends another does not inherit it.
  name?: string
  /// The property whose value is the element's title.
  title?: Target.Property
  /// The property whose value reads under the title.
  subtitle?: Target.Property
  /// The property whose value shows as a badge beside the heading.
  badge?: Target.Property
  /// The property whose value is the URL of the element's picture.
  image?: Target.Property
  /// The property whose value identifies the element: a key that names it and is kept. It reads as
  /// one quiet token, its value alone, where `idPosition` puts it.
  id?: Target.Property
  /// Where the element's id reads.
  idPosition: IdPosition = beginning
  /// The name of the icon that marks the element where it starts, from the Lucide set
  /// (https://lucide.dev/icons), such as `"list-checks"`. A kind that extends another inherits it.
  icon?: string
  /// How the element is laid out.
  layout: ElementLayout = card
  /// How individual properties read, in the order they should be listed. Properties not named
  /// here follow in declaration order.
  content properties?: <PropertyVisualizer Target=Target/>+
}

/// Where an element's id reads.
export type IdPosition =
  /// Right after the element's name.
  | beginning
  /// Last in the element's tag, after its other properties.
  | end

/// How an element is laid out.
export type ElementLayout =
  /// A card with a heading and a row per property.
  | card
  /// One line holding the heading, title, subtitle, badge and picture, with the rest folded.
  | row
  /// Within the line that holds it, as its heading and title, with the rest folded.
  | inline

/// How one property of an element reads.
export type PropertyVisualizer = {
  // The declaration whose property this describes. Bound by the `visualizer` declaration.
  Target: type
  /// The property this describes.
  of: Target.Property
  /// The property's label, in place of its name.
  label?: string
  /// What a boolean property reads as when it is true, in place of its row.
  whenTrue?: string
  /// What a boolean property reads as when it is false, in place of its row.
  whenFalse?: string
  /// How the property's string value is shown.
  format?: ValueFormat
  /// Where the property is shown.
  place: Placement = body
}

/// Where a property is shown on its element.
export type Placement =
  /// Among the element's rows.
  | body
  /// In a strip at the foot of the element, folded until a reader opens it.
  | details

/// How a viewer presents the cases of a union.
export type UnionVisualizer = {
  // The union this visualizer describes. Bound by the `visualizer` declaration.
  Target: type
  /// How individual cases read.
  content cases?: <CaseVisualizer Target=Target/>+
}

/// How one case of a union reads.
export type CaseVisualizer = {
  // The union whose case this describes. Bound by the `visualizer` declaration.
  Target: type
  /// The case this describes.
  of: Target
  /// The case's label, in place of its name.
  label?: string
  /// A short example of what the case looks like, shown with its name.
  example?: string
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
  /// As plain text.
  | text
  /// As Markdown prose.
  | markdown
  /// As a link.
  | url
  /// As a small picture, when the URL is `https:` or `http:` and the viewer loads images.
  | image
  /// As a swatch beside the text.
  | color
  /// As a link that writes an email.
  | email
  /// As a link that dials.
  | phone
  /// As an identifier: a key that names one thing and is kept, such as an id stored in a database.
  | id
```

## See also
- Reference: [Visualizer Declarations](/reference/concepts/visualizer-declarations)
- Reference: [Modules](/reference/syntax/modules#standard-libraries)
- Reference: [Generic records](/reference/syntax/types#generic-records)
