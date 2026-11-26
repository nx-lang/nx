## ADDED Requirements

### Requirement: The source tree answers value suggestions when asked
A `sourceTree` request SHALL accept an optional `suggestions` boolean, defaulting to false. When it is
false the answer SHALL carry no suggestions and no suggestion keys, and SHALL be unchanged from an
answer without this feature. When it is true, the answer SHALL carry a `suggestions` list, computed
from the same analysis as the rest of the answer, as the value-suggestions capability defines it.
These SHALL carry a `suggestionKey` when their type has a suggestion key: each entry of a
declaration entry's `properties` and `state` lists and of each of its cases' `properties`, and each
value declaration entry for a parameterless top-level `let` that this document declares. A
declaration listed only because a suggestion starts from it SHALL NOT bring keys of its own, nor
the types of its properties. A suggestion key SHALL be an opaque string, and two keys SHALL be equal
exactly when they identify the same declaration. The list SHALL hold only entries whose key some
`suggestionKey` of the answer carries. Each entry SHALL carry:

- `name`: the value as the document would write it, such as `env.space.md`, `brandRed` or
  `d.standardPalette.blue600`;
- `key`: its suggestion key;
- `group`: which group it belongs to and the group's label, as the value-suggestions capability
  says;
- `scope`: for a parameter or state field, the declaration-table index of the top-level component or
  function whose body may use it; absent for every other entry;
- `type`: its type spelled in NX;
- `declaration`: the index in the declaration table of the declaration it starts from, which the
  table SHALL then list;
- `doc`: the doc comment of the field or declaration it names, when it has one;
- `preview`: its value at the defaults, when the value-suggestions capability finds one;
- `self`: the declaration-table indices of the top-level declarations inside which it SHALL NOT be
  offered, as the value-suggestions capability says; absent when there are none.

The entries SHALL be listed in the order the value-suggestions capability gives. The answer SHALL round-trip through JSON without loss, and the wasm snapshot and
`@nx-lang/language-core`'s `sourceTree` SHALL give the same answer the Rust language service gives.

#### Scenario: Asking for suggestions
- **WHEN** a client asks with `suggestions` for the source tree of a document that imports
  `@nx/design` and constructs `<Card padding=8 />`, where `Card.padding` is typed `Spacing`
- **THEN** `Card`'s `padding` property SHALL carry a suggestion key
- **AND** the suggestions with that key SHALL include `env.space.md` with the type `Spacing`

#### Scenario: Without the option nothing changes
- **WHEN** a client asks for the source tree of the same document without `suggestions`
- **THEN** the answer SHALL carry no `suggestions` and no `suggestionKey`

#### Scenario: Parity between Rust and TypeScript
- **WHEN** the wasm SDK's language-service tests compute the source tree of each document in their
  corpus with `suggestions` through the TypeScript service and through the Rust service
- **THEN** the two answers SHALL be equal after JSON round-trip

