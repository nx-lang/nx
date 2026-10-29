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

`ElementVisualizer`, `PropertyVisualizer` and `ValueVisualizer` take the declaration they describe
as their type parameter `Target`, and
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
| An element-style function | `FunctionVisualizer` |

A union cannot have a visualizer yet.

## Elements

| Type | Fields | Notes |
| --- | --- | --- |
| `ElementVisualizer` | `title?`, `id?`, `idPosition: IdPosition = start`, `icon?`, `preview?: PreviewVisualizer`, content `properties?: PropertyVisualizer+` | `title` and `id` each name a property of the target. |
| `PreviewVisualizer` | `renderer: string` | Nonempty literal host renderer ID; see [Preview descriptors](#preview-descriptors). |
| `FunctionVisualizer` | `icon?: string` | Presentation-only icon for element-style function invocations. |
| `IdPosition` | `start`, `end` | Where the element's id reads. |
| `PropertyVisualizer` | `property`, `format?: ValueFormat`, the [editor hints](#editor-hints), `multiline: boolean = false`, `placeholder?: string` | `property` names the property it describes. |

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
- `properties` says how individual properties read and edit: a `PropertyVisualizer`'s `format` says
  how the property's string value is shown, in place of the format its type gives it, and its
  [editor hints](#editor-hints) how the value is edited. The order of the `PropertyVisualizer`s is the
  order in which Visual lists the properties an element does not write.

```nx
import "@nx/visualizer"

export abstract component <Question id:string label:string help?:string required:boolean = true />

visualizer Question = <ElementVisualizer title=label id=id icon="circle-help" />
```

## Values

| Type | Fields | Notes |
| --- | --- | --- |
| `ValueVisualizer` | `format?: ValueFormat`, the [editor hints](#editor-hints), content `choices?: ValueChoice+` | How a value of the alias is shown and edited. |
| `ValueFormat` | `id`, `color` | A format changes how the value looks, never what it is. |
| `ValueEditor` | `field`, `dropdown`, `segmented` | The editor a viewer offers for a value. |
| `ValueChoice` | `value: object`, `label?: string` | A value a viewer offers to pick, such as a size on a scale. |

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

## Editor hints

A `PropertyVisualizer` and a `ValueVisualizer` can also say how a value is edited. Each hint changes
how a viewer edits a value, never what the value is: `min` and `max` are not constraints, and a
document may still write a number outside them.

| Hint | Applies to | Says |
| --- | --- | --- |
| `editor: ValueEditor` | A number: `field`. A union of cases with no properties: `dropdown`, `segmented` | Which editor to offer. |
| `min`, `max: float64` | A number | The range the editor's arrows, steppers and dragging keep to. |
| `step: float64` | A number | How far one step moves the number, as an arrow key or a stepper does. |
| `multiline: boolean` | A string, on a `PropertyVisualizer` only | The field takes line breaks: Enter starts a new line. |
| `placeholder: string` | A string, on a `PropertyVisualizer` only | Text shown in the empty field; never written. |
| content `choices: ValueChoice+` | A number or a string | The values offered to pick, in order: a number literal for a number, a string literal for a string. They suggest and never restrict. |

A property's hints come from its `PropertyVisualizer`, hint by hint, and otherwise from the
`ValueVisualizer` of the alias its type names, so a design system can say once how its spacing and
its text sizes edit. Choices are one hint: a `PropertyVisualizer` that gives any replaces all of its
type's, so two scales never mix. Picking a choice writes its value, never its label:

```nx
import "@nx/visualizer"

export type Spacing = int
export type FontSize = int
export type Align = start | center | end
export type Card = {
  title:string
  padding:Spacing = 8
  gap:Spacing = 4
  size:FontSize = 16
  align:Align = start
  body?:string
}

visualizer Spacing = <ValueVisualizer min=0 max=64 />
visualizer FontSize = <ValueVisualizer min=8 max=96>
  <ValueChoice value=12 label="sm" />
  <ValueChoice value=16 label="base" />
  <ValueChoice value=20 label="lg" />
</ValueVisualizer>
visualizer Card =
  <ElementVisualizer title=title>
    <PropertyVisualizer property=gap max=16 />
    <PropertyVisualizer property=align editor=segmented />
    <PropertyVisualizer property=body multiline=true placeholder="A line or two" />
  </ElementVisualizer>
```

A hint that does not fit its property's type, such as `field` on a string or a string choice for a
number, is reported as the warning `editor-hint-mismatch` and ignored, so the hint it would have
replaced (its base visualizer's, or its type's) stays in force. A `min` above its `max` is reported
as `editor-hint-range`, and so is a range that only merging empties: with `Spacing` at `max=64`
above, `<PropertyVisualizer property=gap min=100 />` warns at `min=100`.
Warnings, because a hint never changes what a program means.

## Preview descriptors

A nominal element type can request a rendered preview alongside its Visual presentation:

```nx
import "@nx/visualizer"

external component <Survey title:string/>

visualizer Survey = <ElementVisualizer
  title=title
  icon="list"
  preview=<PreviewVisualizer renderer="app.survey"/>
/>
```

`PreviewVisualizer` has one required property, `renderer`, a nonempty string literal naming a host
implementation. The host decides which IDs it supports. An unsupported ID leaves the ordinary
structured Output available. The descriptor runs no code and does not install or load plugins.
Preview descriptors merge through inheritance and program/host overrides like other visualizer hints.
This first profile attaches previews to nominal element types; primitive aliases retain their
existing ValueVisualizer formats and editors.

## Element-function icons

An element-style function may use a `FunctionVisualizer` with an optional Lucide `icon`:

```nx
import "@nx/visualizer"

let <Badge text:string/> = <Label text={text}/>
visualizer Badge = <FunctionVisualizer icon="badge"/>
```

This marks the authored invocation. The returned value keeps its own type and determines its
preview. FunctionVisualizer has no Target parameter, property hints or preview descriptor.
Paren-style functions cannot be visualizer targets. This profile supports source-bearing element
functions; function targets carried only by an interface without source are not supported yet.

## The library source

```nx
// The NX visualizer library: the records a `visualizer` declaration is written with, which tell a
// viewer how to present a declaration. Import it with `import "@nx/visualizer"`.
//
// UNSTABLE. These declarations may change incompatibly in any NX release, including a patch
// release. See https://nxlang.org/reference/libraries/visualizer.
//
// ElementVisualizer, PropertyVisualizer and ValueVisualizer take the declaration they describe
// as their type parameter `Target`. Inside a
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
  preview?: PreviewVisualizer     /// Host-rendered view, inherited and overridden like other hints.
                                  /// Unsupported renderer IDs leave structured Output available.

  /// How individual properties read.
  content properties?: <PropertyVisualizer Target=Target/>+
}

/// A rendered presentation requested by a type; the host supplies the implementation.
/// Unknown renderer IDs leave the ordinary structured output available. This descriptor loads no code.
export type PreviewVisualizer = {
  renderer: string             /// Logical renderer ID registered by the host, such as "nx.graphics".
}

/// How a viewer marks an authored element-style function invocation.
/// Previews are selected from the function's result type, independently of this invocation icon.
export type FunctionVisualizer = {
  icon?: string                /// Lucide icon name for the invocation, such as "star".
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
  /// The editor a viewer offers for the value. A viewer that cannot offer it uses its usual one.
  editor?: ValueEditor
  /// The smallest number the editor offers. Not a constraint: a document may write a smaller one.
  min?: float64
  /// The largest number the editor offers. Not a constraint: a document may write a larger one.
  max?: float64
  /// How far one step of the editor moves a number, as an arrow key or a stepper does.
  step?: float64
  /// Whether the property's string field takes line breaks: Enter starts a new line, and the
  /// value is written over several lines.
  multiline: boolean = false
  /// Text shown in the property's empty field, quieter than a value, and never written.
  placeholder?: string
  /// The values a viewer offers to pick for the property, in order. They suggest and never
  /// restrict: any value the property's type takes may still be written.
  content choices?: ValueChoice+
}

/// How a viewer presents values of a scalar type.
export type ValueVisualizer = {
  // The type this visualizer describes. Bound by the `visualizer` declaration.
  Target: type
  /// How a string value of the type is shown.
  format?: ValueFormat
  /// The editor a viewer offers for the value. A viewer that cannot offer it uses its usual one.
  editor?: ValueEditor
  /// The smallest number the editor offers. Not a constraint: a document may write a smaller one.
  min?: float64
  /// The largest number the editor offers. Not a constraint: a document may write a larger one.
  max?: float64
  /// How far one step of the editor moves a number, as an arrow key or a stepper does.
  step?: float64
  /// The values a viewer offers to pick for a value of the type, in order. They suggest and never
  /// restrict: any value of the type may still be written.
  content choices?: ValueChoice+
}

/// A value a viewer offers to pick, such as a size on a type's scale. Picking it writes the value,
/// never the label.
export type ValueChoice = {
  /// The value written when the choice is picked: a number literal for a number, or a string
  /// literal for a string.
  value: object
  /// The name the choice reads by, beside its value, such as `"lg"`.
  label?: string
}

/// How a string value is shown. A format changes how the value looks, never what it is.
export type ValueFormat =
  /// As an identifier: a key that names one thing and is kept, such as an id stored in a database.
  | id
  /// With a small square of the color it names.
  | color

/// The editor a viewer offers for a value. A hint changes how a value is edited, never what it is.
export type ValueEditor =
  /// A number field, with arrow-key steps, even when the number has a range.
  | field
  /// A list of a union's cases.
  | dropdown
  /// A union's cases as one row of buttons.
  | segmented
```

## See also
- Reference: [Visualizer Declarations](/reference/concepts/visualizer-declarations)
- Reference: [Modules](/reference/syntax/modules#standard-libraries)
- Reference: [Generic records](/reference/syntax/types#generic-records)
