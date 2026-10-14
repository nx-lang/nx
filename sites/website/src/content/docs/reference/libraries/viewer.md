---
title: 'The viewer library'
description: 'Reference for @nx/viewer, the standard library of records that viewer declarations are written with.'
sidebar:
  label: '@nx/viewer'
---

`@nx/viewer` is a [standard library](/reference/syntax/modules#standard-libraries): NX source the
compiler carries. It declares the records a
[viewer declaration](/reference/concepts/viewer-declarations) is written with, which tell a viewer
how to present a declaration. Import it by name:

```nx
import "@nx/viewer"

type Choice = { value:string label:string }

viewer Choice = <ElementViewer title=label badge=value />
```

:::caution[Unstable]
`@nx/viewer` is **unstable**. Its declarations may change incompatibly in any NX release, including
a patch release, with no deprecation period. A change reaches you only when you move to a new
release of the NX packages, and every release that changes the library lists the change under
`@nx/viewer` in its release notes. Viewer declarations you have written may need updating when a
record changes; nothing you have compiled does, because viewer declarations never reach NX IR.
:::

The library holds types only. It has no functions and runs nothing: a viewer reads the resolved
viewer declarations from the language service and decides how to draw them.

## The `Target` parameter

Every record in the library takes the declaration it describes as its type parameter `Target`, and
names that declaration's properties through `Target.Property`, so `title=label` is checked against
the target's fields. Inside a `viewer` declaration, `Target` is bound for you: write
`<ElementViewer title=label />`, not `<ElementViewer Target=Choice title=label />`. For a component,
`Target` is its props record, [`T.Props`](/reference/syntax/functions#component-props-and-state), so
property names are its props.

Outside a viewer declaration the ordinary rule for
[generic records](/reference/syntax/types#generic-records) holds, and a construction that leaves
`Target` out is rejected:

```nx invalid
import "@nx/viewer"

let viewer = <ElementViewer />   // error: `Target` was not specified
```

Which record a viewer declaration takes depends on its target:

| Target | Record |
| --- | --- |
| A record, an action, an inline emitted action, or a component | `ElementViewer` |
| A union whose cases all carry no payload | `UnionViewer` |
| An alias of a primitive type | `ValueViewer` |

## Elements

| Type | Fields | Notes |
| --- | --- | --- |
| `ElementViewer` | `name?`, `title?`, `subtitle?`, `badge?`, `image?`, `layout: ElementLayout = card`, content `properties?: PropertyViewer+` | `title`, `subtitle`, `badge` and `image` each name a property of the target. |
| `ElementLayout` | `card`, `row`, `inline` | How the element is laid out. |
| `PropertyViewer` | `of`, `label?`, `whenTrue?`, `whenFalse?`, `format?: ValueFormat`, `place: Placement = body` | `of` names the property it describes. |
| `Placement` | `body`, `details` | Where the property is shown. |

- `name` is the kind's name as readers see it, in place of the declaration's name split into words.
  A kind that extends another does not inherit it.
- `title` is the property whose value is the element's title, `subtitle` the one that reads under
  it, `badge` the one shown as a badge beside the heading, and `image` the one whose value is the
  URL of the element's picture.
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
import "@nx/viewer"

export abstract component <Question id:string label:string help?:string required:boolean = true />

viewer Question =
  <ElementViewer title=label subtitle=help>
    <PropertyViewer of=required whenTrue="Required" whenFalse="Optional" />
    <PropertyViewer of=id label="Key" place=details />
  </ElementViewer>
```

## Unions

| Type | Fields | Notes |
| --- | --- | --- |
| `UnionViewer` | content `cases?: CaseViewer+` | How individual cases read. |
| `CaseViewer` | `of`, `label?`, `example?` | `of` names a case of the target union. |

A case's `label` replaces its name. Its `example` is a short example of what the case looks like,
shown with its name:

```nx
import "@nx/viewer"

export type ChoiceLayout = list | grid | chips

viewer ChoiceLayout =
  <UnionViewer>
    <CaseViewer of=list label="a list" />
    <CaseViewer of=chips label="chips" example="A B C" />
  </UnionViewer>
```

A union with a payload case cannot have a viewer, because a payload case cannot be named without
its payload.

## Values

| Type | Fields | Notes |
| --- | --- | --- |
| `ValueViewer` | `format: ValueFormat` | How a string value of the alias is shown. |
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

A `ValueViewer` on an alias formats every property typed by that alias. A `PropertyViewer`'s
`format` overrides it for one property:

```nx
import "@nx/viewer"

export type ImageUrl = string
export type Choice = { label:string imageUrl?:ImageUrl website?:string }

viewer ImageUrl = <ValueViewer format=image />
viewer Choice =
  <ElementViewer title=label image=imageUrl>
    <PropertyViewer of=website format=url />
  </ElementViewer>
```

## The library source

```nx
// The NX viewer library: the records a `viewer` declaration is written with, which tell a viewer
// how to present a declaration. Import it with `import "@nx/viewer"`.
//
// UNSTABLE. These declarations may change incompatibly in any NX release, including a patch
// release. See https://nxlang.org/reference/libraries/viewer.
//
// Each record takes the declaration it describes as its type parameter `Target`. Inside a
// `viewer` declaration, `Target` is bound for you: write `viewer Choice = <ElementViewer
// title=label />`, not `<ElementViewer Target=Choice ... />`. For a component, `Target` is its
// props record, so property names are its props.

/// How a viewer presents an element of a record, action or component.
export type ElementViewer = {
  // The declaration this viewer describes. Bound by the `viewer` declaration.
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
  /// How the element is laid out.
  layout: ElementLayout = card
  /// How individual properties read, in the order they should be listed. Properties not named
  /// here follow in declaration order.
  content properties?: <PropertyViewer Target=Target/>+
}

/// How an element is laid out.
export type ElementLayout =
  /// A card with a heading and a row per property.
  | card
  /// One line holding the heading, title, subtitle, badge and picture, with the rest folded.
  | row
  /// Within the line that holds it, as its heading and title, with the rest folded.
  | inline

/// How one property of an element reads.
export type PropertyViewer = {
  // The declaration whose property this describes. Bound by the `viewer` declaration.
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
export type UnionViewer = {
  // The union this viewer describes. Bound by the `viewer` declaration.
  Target: type
  /// How individual cases read.
  content cases?: <CaseViewer Target=Target/>+
}

/// How one case of a union reads.
export type CaseViewer = {
  // The union whose case this describes. Bound by the `viewer` declaration.
  Target: type
  /// The case this describes.
  of: Target
  /// The case's label, in place of its name.
  label?: string
  /// A short example of what the case looks like, shown with its name.
  example?: string
}

/// How a viewer presents values of a scalar type.
export type ValueViewer = {
  // The type this viewer describes. Bound by the `viewer` declaration.
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
- Reference: [Viewer Declarations](/reference/concepts/viewer-declarations)
- Reference: [Modules](/reference/syntax/modules#standard-libraries)
- Reference: [Generic records](/reference/syntax/types#generic-records)
