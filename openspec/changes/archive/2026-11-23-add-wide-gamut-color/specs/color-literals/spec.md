## MODIFIED Requirements

### Requirement: Hex colors are typed expressions
NX SHALL accept #RGB, #RGBA, #RRGGBB, and #RRGGBBAA expressions with case-insensitive hex digits and
alpha last, each giving a `Color.srgb` value of the canonical @nx/color Color. Shorthand SHALL
duplicate nibbles and omitted alpha SHALL be 1. Each channel SHALL equal float32(byte/255).
Literals SHALL infer the canonical Color type independently of contextual expectations, local names,
aliases, and explicit imports. Strings SHALL remain strings without implicit coercion. Literal syntax
SHALL be permitted in ordinary expressions, defaults, sequences, and unbraced element properties.
Wide-gamut colors SHALL have no literal syntax; they come from the library's constructors.

#### Scenario: Import-free color authoring
- **WHEN** source declares `let accent = #5b6ef5` and uses a hex literal as an element property without importing color or graphics
- **THEN** both literals SHALL resolve to canonical `Color.srgb` values with the correct float32 channels

#### Scenario: Equivalent spellings and alpha
- **WHEN** #f008, #FF000088, and the corresponding parsed color are evaluated
- **THEN** they SHALL compare equal with alpha float32(136/255)

### Requirement: Literal compilation uses standard record contracts
The compiler SHALL discover the canonical color module through standard-library identity/version resolution, record the implicit dependency, and lower literals to existing typed union-case record IR constants of `Color.srgb`. A literal-only program SHALL require neither a new generic runtime value kind nor the standard-color-v2 external-function feature. Missing canonical modules SHALL fail explicitly. Source-local declarations named Color SHALL NOT change literal meaning.

#### Scenario: Literal-only runtime
- **WHEN** a compiled literal-only program runs on supported IR without color external-function implementations but with its required library images
- **THEN** it SHALL return the canonical `Color.srgb` record successfully

## ADDED Requirements

### Requirement: Printed values use hex for byte-exact sRGB colors
NX value text SHALL write a canonical `Color.srgb` record whose fields are only `r`, `g`, `b` and
`alpha`, each exactly float32(byte/255), as the hex literal that reads back as it: `#rrggbb`, or
`#rrggbbaa` when alpha is not 1. Every other color SHALL be written as its case record. The written
node SHALL keep the record's type, pointer and swatch, and a hex color SHALL NOT force its parent
onto one property per line.

#### Scenario: Hex in printed output
- **WHEN** a value holding `#f4e9e7` and `c.lighten(#f4e9e7, 0.1)` is printed as NX text
- **THEN** the first SHALL print as `#f4e9e7` and the second, whose channels fall between bytes, as a `<Color.srgb … />` record
