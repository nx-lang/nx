## ADDED Requirements

### Requirement: TypeScript runtime supplies the prelude module itself
The TypeScript runtime package SHALL carry the compiled image of the prelude that its compiler
release emits, and linking SHALL supply it for the prelude's reserved identity whenever the host's
resolver returns no module for that identity — at any depth of the link, including for a module the
resolver did supply — so that no host has to ship, prepare or resolve the prelude. A module the
host's resolver does return for that identity SHALL be used instead. The built-in prelude SHALL be
prepared at most once and reused across linked programs, and preparing a self-contained program
SHALL succeed for an image whose only other module is the prelude. A declaration the image
references that the runtime's prelude lacks SHALL fail linking with the missing-declaration
diagnostic naming the prelude and the declaration. An image whose module table records a prelude
version other than the one the resolved prelude carries SHALL fail linking with the version
diagnostic, whether the prelude came from the built-in image or from the host, unless the host opts
into linking across versions. The checked-in image SHALL be regenerated from the prelude's source by
a documented command, and a test SHALL fail when it is stale.

#### Scenario: A self-contained program that builds a range
- **WHEN** a host prepares a program from one image whose module table lists only the prelude
- **THEN** preparation SHALL succeed with no resolver and the program SHALL evaluate

#### Scenario: A host resolver that knows nothing of the prelude
- **WHEN** a snippet and a catalog each reference `Range` and the host's resolver supplies the catalog alone
- **THEN** linking SHALL succeed, with the runtime's prelude serving both modules

#### Scenario: A host overrides the prelude
- **WHEN** the host's resolver returns a prepared module for the prelude's identity
- **THEN** linking SHALL use that module and SHALL NOT use the built-in one

#### Scenario: A prelude declaration the runtime lacks
- **WHEN** an image references a prelude declaration that the runtime's prelude does not hold
- **THEN** linking SHALL fail naming the prelude and that declaration

#### Scenario: An image compiled against another prelude contract
- **WHEN** an image records a prelude version other than the one this runtime's prelude carries, and references only declarations that prelude holds
- **THEN** linking SHALL fail with the version diagnostic naming the prelude
- **AND** linking SHALL succeed when the host opts into linking across versions

#### Scenario: The checked-in image is current
- **WHEN** the prelude's source changes and the runtime's image is not regenerated
- **THEN** a test SHALL fail naming the command that regenerates it

### Requirement: TypeScript runtime iterates a range without building it
The TypeScript runtime SHALL accept the required feature `ranges-v1` and SHALL evaluate a
`forRange` node by evaluating its iterable to a `Range` record and running the body once for each
integer from `start` upward — stopping before `end`, or after it when `endInclusive` is true — with
the item bound to that integer and the index, when present, bound to its position from zero. It
SHALL yield the list of body values, SHALL yield the empty list when the range holds no integers,
and SHALL NOT allocate the list of integers. A `forRange` iterable that does not evaluate to a
`Range` record with integer bounds SHALL fail evaluation with a diagnostic, as a `for` over a
non-list does.

The runtime SHALL refuse to iterate a range holding more integers than a limit the host may set in
its runtime options, with a default of one million, failing with the resource-limit diagnostic that
names the limit before running the body at all.

#### Scenario: A half-open and a closed range
- **WHEN** an image evaluates `for i in 0..4 { i * i }` and `for p in 1..=3 { p }`
- **THEN** the runtime SHALL return the lists `0 1 4 9` and `1 2 3`

#### Scenario: An empty range runs no body
- **WHEN** an image evaluates `for i in 5..2 { i }`
- **THEN** the runtime SHALL return the empty list

#### Scenario: A host-supplied range iterates
- **WHEN** a host passes `{ "$type": "Range", "start": 2, "end": 4, "endInclusive": true }` for a parameter typed `<Range T=int/>` and the function iterates it
- **THEN** the runtime SHALL run the body for `2`, `3` and `4`

#### Scenario: The names the runtime recognizes are the prelude's
- **WHEN** a program links against the built-in prelude
- **THEN** the prelude's `Range` declaration SHALL carry exactly the field names the runtime's range check requires, typed `object`, `object` and `boolean`

#### Scenario: An oversized range is refused
- **WHEN** an image evaluates `for i in 0..2000000 { i }` under the default options
- **THEN** evaluation SHALL fail with the resource-limit diagnostic naming the limit of one million
- **AND** the same image SHALL evaluate under options that raise the limit above two million

#### Scenario: An older runtime refuses the module by name
- **WHEN** a runtime that does not know `ranges-v1` prepares a module that lists it
- **THEN** preparation SHALL fail naming the feature `ranges-v1`
