## ADDED Requirements

### Requirement: Graphics visualizers label, group and summarize every element
The graphics library's visualizers SHALL give every property of Drawing, Group, the basic shapes,
paths, text and the element-style shape helpers a label, and SHALL group them as design tools do:
`Position` (`X`, `Y` short labels, for `x`/`y` and centers `cx`/`cy`), `Size` (`W`, `H`), `Shape`
(radii as `R` or icons, points, sides, angles), `Transform` (rotation), `Appearance` (opacity),
`Fill`, `Stroke` and `Text` (font, size). Each SHALL name a `summary` of at most three properties
that tells siblings apart, such as a Star's points and fill or a Rectangle's width, height and fill.
Icons SHALL be Lucide names. These SHALL be presentation only and SHALL NOT change evaluation.

#### Scenario: Star reads like a design tool
- **WHEN** the graphics example is opened in Visual and Star is selected
- **THEN** Star's row SHALL summarize its points and fill, and its inspector SHALL show `Position`
  with `X` and `Y`, and `Shape` with outer radius, inner radius and points

#### Scenario: Every property is labeled
- **WHEN** the graphics library is analyzed with visualizers
- **THEN** every property of every graphics element and element-style helper SHALL resolve a label
  and a group
