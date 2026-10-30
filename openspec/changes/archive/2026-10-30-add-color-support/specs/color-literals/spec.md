## Purpose

Allow authors to express portable typed colors directly in NX source using familiar hexadecimal literals without requiring graphics or explicit color-library imports.

## ADDED Requirements

### Requirement: Hex colors are typed expressions
NX SHALL accept #RGB, #RGBA, #RRGGBB, and #RRGGBBAA expressions with case-insensitive hex digits and alpha last. Shorthand SHALL duplicate nibbles and omitted alpha SHALL be 1. Each channel SHALL equal float32(byte/255). Literals SHALL infer the canonical @nx/color Color type independently of contextual expectations, local names, aliases, and explicit imports. Strings SHALL remain strings without implicit coercion. Literal syntax SHALL be permitted in ordinary expressions, defaults, sequences, and unbraced element properties.

#### Scenario: Import-free color authoring
- **WHEN** source declares `let accent = #5b6ef5` and uses a hex literal as an element property without importing color or graphics
- **THEN** both literals SHALL resolve to canonical Color values with the correct float32 components

#### Scenario: Equivalent spellings and alpha
- **WHEN** #f008, #FF000088, and the corresponding parsed color are evaluated
- **THEN** they SHALL compare equal with alpha float32(136/255)

### Requirement: Literal compilation uses standard record contracts
The compiler SHALL discover the canonical color module through standard-library identity/version resolution, record the implicit dependency, and lower literals to existing typed-record IR constants. A literal-only program SHALL require neither a new generic runtime value kind nor the standard-color-v1 external-function feature. Missing canonical modules SHALL fail explicitly. Source-local declarations named Color SHALL NOT change literal meaning.

#### Scenario: Literal-only runtime
- **WHEN** a compiled literal-only program runs on supported IR without color external-function implementations but with its required library images
- **THEN** it SHALL return the canonical Color record successfully

### Requirement: Literal diagnostics and source round trips are precise
Malformed hash-prefixed tokens, including wrong lengths, nonhex digits, and adjacent identifier characters, SHALL receive a color-literal diagnostic spanning the offending token without accepting a valid prefix and leaving stray suffix tokens. Strings/comments containing hashes SHALL retain their existing meaning. Formatting and syntax-tree/source round trips SHALL preserve valid literal spelling unless explicitly edited. Source highlighters and printers SHALL recognize colors as literals. Evaluated value printing SHALL preserve float32 values and SHALL NOT replace non-byte-representable colors with lossy hex.

#### Scenario: Invalid token recovery
- **WHEN** an author writes #12, #gg0000, or #ff0000oops followed by another declaration
- **THEN** the invalid color SHALL be diagnosed as a whole and parser recovery SHALL retain the following declaration

#### Scenario: Delimiters and strings
- **WHEN** a color precedes an operator, comment, closing brace, or element delimiter, or a string contains "#ff0000"
- **THEN** the literal SHALL end at that delimiter and the quoted text SHALL remain a string
