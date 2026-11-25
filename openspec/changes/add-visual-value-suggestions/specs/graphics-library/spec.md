## ADDED Requirements

### Requirement: Graphics properties name the design value types
The graphics library SHALL type these properties with `@nx/design`'s value types, keeping each
property's constraint, default and optionality: `Stroke.width` as `BorderWidth`, `Text.fontSize` as
`FontSize`, `Text.fontFamily` as `FontFamily`, `Rectangle.rx` and `Rectangle.ry` as `Radius`, and
every `opacity` property and parameter as `Opacity`, which also gives the two internal helpers'
`opacity: float64 = 1` parameters the `[0,1]` constraint their docs state. `@nx/graphics` SHALL
import `@nx/design` under an alias, `import "@nx/design" as design`, writing `design.BorderWidth`
and the rest, so that none of `@nx/design`'s names joins its own, and SHALL stay free of
diagnostics. `Text.fontWeight` and the geometric radii of circles, ellipses,
regular polygons and stars SHALL keep their `float64` types. Since the aliases are transparent, every
program that type checks against the library today SHALL still type check, and evaluate and render
the same.

#### Scenario: A plain number still fits
- **WHEN** a drawing writes a `Stroke` with `width=2` and a `Text` with `fontSize=18` and `opacity=0.5`
- **THEN** analysis SHALL accept it, and the drawing SHALL render as before

#### Scenario: A stroke width takes border-width tokens
- **WHEN** a program that imports `@nx/graphics` and `@nx/design` asks for the source tree with
  `suggestions`
- **THEN** `Stroke`'s `width` property SHALL carry the suggestion key of `BorderWidth`, and the
  suggestions with that key SHALL include `env.borderWidth.thin`
