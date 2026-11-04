## ADDED Requirements

### Requirement: Graphics visualizers label every element's properties
The graphics library's visualizers SHALL give every property of Drawing, Group, the basic shapes,
paths, text and the element-style shape helpers a label, and SHALL list them in the order design
tools do: position (`X` and `Y` for `x`/`y`, `Center X` and `Center Y` for `cx`/`cy`), size (width,
height) and shape (radii, points, sides, angles, rotation), then fill and stroke, then transform,
opacity, clipping, accessibility and a container's content where an element has them. Text SHALL
list its text first, then its position, its font and its fill.
Element icons SHALL be Lucide names; properties SHALL have no icons. These SHALL be presentation only and SHALL NOT change evaluation.

#### Scenario: Star reads like a design tool
- **WHEN** the graphics example is opened in Visual and Star is selected
- **THEN** Star's inspector SHALL show `Center X` and `Center Y` first, then outer radius, inner radius and points

#### Scenario: Every property is labeled
- **WHEN** the graphics library is analyzed with visualizers
- **THEN** every property of every graphics element and element-style helper SHALL resolve a label
