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
