# fiddle-nx-catalog-visualizers Specification

## Purpose

Defines the visualizers the fiddle gives its DrawnUI catalog, so NX snippets read well in the Visual
view: the host visualizer module and how it is shipped and named, what each control shows, the aliases
the catalog generator writes for colors and text shown to people, and the checks that keep all of it in
step with the catalog.

## Requirements

### Requirement: The catalog's visualizers are a host module the fiddle names
The fiddle SHALL ship one hand-written NX module of visualizer declarations for the DrawnUI catalog,
apart from the generated catalog, and SHALL name it as a host visualizer module in every source-tree
query the Visual view makes, so its declarations apply after the snippet's own. The module SHALL NOT
be part of any snippet's program: compiling, drawing, sharing and playing a snippet SHALL be unaffected
by it, and a share's stored module SHALL be byte for byte what it is without it.

#### Scenario: The module shapes the reading
- **WHEN** a snippet holds `<SkiaLabel Text="Hello" />` alone on its line and Visual shows
- **THEN** the reading SHALL show the label's icon and `Hello` as its title

#### Scenario: A snippet's own visualizer applies too
- **WHEN** a snippet declares `component <Card title:string /> = { <SkiaLabel Text={title} /> }` with
  `visualizer Card = <ElementVisualizer title=title icon="id-card" />`, and uses `<Card title="Plans" />`
- **THEN** the reading SHALL show the `id-card` icon and `Plans` as its title

#### Scenario: A share is unchanged
- **WHEN** an NX preset is compiled for a share with the visualizer module present and without it
- **THEN** the two stored modules SHALL be identical

### Requirement: Every concrete control has an icon, and bases share the rest
Every concrete component of the catalog SHALL resolve to a visualizer with a Lucide icon that tells
its kind of control apart: layout containers, text, shapes, images and animation, inputs, and
scrolling and navigation each by their own icons. What controls share SHALL be declared once on the
abstract base that declares it and be inherited: a control's `Tag` SHALL read as its id, at the end;
the text a label, a text span, a button and an editor show SHALL be their title; and the source an
image, an SVG, a Lottie or a GIF loads SHALL be its title.

#### Scenario: Icons by kind
- **WHEN** a snippet holds a `SkiaStack` with a `SkiaLabel`, a `SkiaImage` and a `SkiaSwitch` in it,
  each on a line of its own
- **THEN** each of the four SHALL start with an icon of its own, and none SHALL start with the
  `component` icon

#### Scenario: A tag reads as an id
- **WHEN** a snippet holds `<SkiaButton Text="Save" Tag="save-button" />` alone on its line
- **THEN** the reading SHALL show `Save` as the title and `save-button` as a quiet id after it

#### Scenario: A folded layout names its count
- **WHEN** the visitor folds a `SkiaStack` that holds three children and sets `Spacing`
- **THEN** the folded group SHALL say how many properties and children it hides, since a stack has no
  title

### Requirement: Colors read with a swatch and text shown to people reads as prose
The catalog generator SHALL declare an alias `Color` of `string` and type with it every string
property whose name ends in `Color`, and SHALL declare an alias `uitext` of `string` and type with it
the string properties that hold text shown to people, from a list the generator keeps. The visualizer
module SHALL give `Color` the `color` format. Type checking, hover and completion SHALL accept every
snippet they accept without the aliases, and the catalog's compiled image SHALL link every share
the catalog without the aliases links.

#### Scenario: A color has a swatch
- **WHEN** a snippet sets `BackgroundColor="#2E7D32"` on a control in a row of a group
- **THEN** the reading SHALL show a square of that color before `#2E7D32`

#### Scenario: A label's text reads as prose
- **WHEN** a snippet holds a `SkiaLabel` written over several lines with `Text="Welcome back"` on a line
  of its own
- **THEN** `Welcome back` SHALL read as text shown to people, in the prose face, without quotes, as
  the label's title on its line or as the value of its `Text` row

#### Scenario: A boolean named like a color is left alone
- **WHEN** the generator meets `ClipBackgroundColor?: boolean`
- **THEN** it SHALL keep the type `boolean`

#### Scenario: Old snippets still compile
- **WHEN** every NX preset and every snippet in the fiddle's NX tests and README is compiled against
  the regenerated catalog
- **THEN** each SHALL compile with the diagnostics it had before the aliases

### Requirement: The visualizer module is checked against the catalog and the icons
The fiddle's NX tests SHALL fail when the visualizer module reports any diagnostic against the
catalog, when a concrete catalog component resolves to no icon, or when an icon the module names is
not in the Lucide set the fiddle bundles. Regenerating the catalog for a new `drawnui-react` version
SHALL therefore name the module's targets that no longer exist and the new controls that have no icon.

#### Scenario: A removed control is named
- **WHEN** a regenerated catalog no longer declares a control the module has a visualizer for
- **THEN** the NX tests SHALL fail naming that visualizer declaration

#### Scenario: A new control without an icon is named
- **WHEN** a regenerated catalog declares a concrete component the module gives no icon, directly or
  through a base
- **THEN** the NX tests SHALL fail naming that component

#### Scenario: A misspelled icon is named
- **WHEN** the module sets `icon="toggle-rigth"` on a control
- **THEN** the NX tests SHALL fail naming the icon and the control

### Requirement: DrawnUI's properties edit with DrawnUI's ranges and choices
The catalog's visualizer module SHALL give editor hints to the properties DrawnUI bounds or whose
values fall on a scale, on the base that declares each so its controls inherit them, or on each
layout container where that base is generic: a range and step for opacity and the ratio properties
(`0` to `1`), whole-number steps from `0` for grid placement, whole-number steps
for `MaxLines` and `ZIndex`, no range where DrawnUI uses values outside one (`-1` sizes, negative
scales and spacing), a range and step of `100` for `FontWeight`, choices on a scale for
`FontSize` and the spacing properties, the font names both of the fiddle's engines register as
choices for `FontFamily`, segmented editors for the short layout and text unions, and multiline text for a
label's and an editor's `Text`. Every hint SHALL fit its property's type.

#### Scenario: Opacity stays in range
- **WHEN** the visitor steps `Opacity=0.95` up twice in Visual
- **THEN** the value SHALL become `1` and step no further

#### Scenario: A font from the fiddle
- **WHEN** the visitor opens the `FontFamily` field of a `SkiaLabel` in Visual
- **THEN** its list SHALL offer `FontText` and `FontTextTitle`, and any other name SHALL still be typeable

#### Scenario: Layout options as buttons
- **WHEN** the visitor clicks `HorizontalOptions=Center` in Visual
- **THEN** the cases `Start`, `Center`, `End` and `Fill` SHALL show as one row of buttons

#### Scenario: Fonts kept in step
- **WHEN** both engines register a font name the module's `FontFamily` choices do not list, or a
  choice names a font one engine does not register
- **THEN** the NX tests SHALL fail naming the font

### Requirement: Common properties lead a control's unset properties
The visualizer module SHALL order each base's properties so the ones people set most read first in a
selected control's unset properties: size and layout on `SkiaControl` (`WidthRequest`,
`HeightRequest`, `HorizontalOptions`, `VerticalOptions`, `Margin`, `BackgroundColor`, `Opacity`), the
layout type, spacing and padding on the layout containers, and the text, size, color and font on
`SkiaLabelBase`, a control's own first and then its bases', then the rest in declaration order.

#### Scenario: A label's first unset rows
- **WHEN** the visitor selects `<SkiaLabel Text="Hi" />` in Visual and opens its unset properties
- **THEN** the first rows SHALL be `FontSize`, `TextColor` and `FontFamily`, in that order
