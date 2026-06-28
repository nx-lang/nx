## ADDED Requirements

### Requirement: NX IR generation preserves directory-loaded library nominal references

NX IR generation SHALL preserve module-qualified nominal type references for declarations that
originate from libraries loaded through a library registry. Successfully validated and evaluated
program artifacts SHALL NOT fail IR generation with `codegen-missing-semantic-data` solely because a
referenced record, union, enum, or type alias came from a loaded library artifact.

#### Scenario: Library record field references dependency library type

- **WHEN** a program imports a directory-loaded `chat-link` library
- **AND** that library exposes a record or type alias containing `QuestionFlow`
- **AND** `QuestionFlow` is declared in a separate directory-loaded `question-flow` dependency
- **THEN** emitted NX IR SHALL encode the `QuestionFlow` reference as a nominal type reference
- **AND** that reference SHALL identify the owning `question-flow` module and declaration

#### Scenario: Transitive loaded library type reference is preserved

- **WHEN** a directory-loaded library declaration references `FlowStep` from another loaded library
  module
- **AND** validation and JSON evaluation for the resulting program artifact succeed
- **THEN** NX IR generation SHALL preserve the `FlowStep` type as a module-qualified nominal
  reference
- **AND** IR generation SHALL NOT require global bare-name lookup to rediscover `FlowStep`

### Requirement: NX IR program fingerprints are lossless for JavaScript consumers

NX IR JSON and structured IR metadata SHALL expose program fingerprints in a form that can represent
the native fingerprint without JavaScript `number` precision loss.

#### Scenario: Fingerprint exceeds JavaScript safe integer range

- **WHEN** an emitted program fingerprint is greater than JavaScript's maximum safe integer
- **THEN** the NX IR JSON SHALL encode `programFingerprint` as a decimal string
- **AND** structured metadata returned with the generated IR SHALL expose the same fingerprint as a
  string or another explicitly lossless representation
- **AND** JavaScript consumers SHALL NOT need to parse the value as `number` to compare cache
  identity

