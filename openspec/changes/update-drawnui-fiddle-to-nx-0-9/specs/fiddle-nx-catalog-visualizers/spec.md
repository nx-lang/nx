## ADDED Requirements

### Requirement: DrawnUI's properties edit with DrawnUI's ranges and choices
The catalog's visualizer module SHALL give editor hints to the properties DrawnUI bounds or whose
values fall on a scale, on the base that declares each so its controls inherit them, or on each
layout container where that base is generic: a range and step for opacity and the ratio properties
(`0` to `1`), whole-number steps from `0` for grid placement and `MaxLines` and whole-number steps
for `ZIndex`, a range and step of `100` for `FontWeight`, choices on a scale for
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
