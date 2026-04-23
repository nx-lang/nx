## ADDED Requirements

### Requirement: Multi-item braced value lists use external component inheritance for common item types

When a `ValuesBracedExpression` has more than one item and type inference computes the most specific
common item type between two named types that both resolve to external component contracts, the
system SHALL consider the declared external component `extends` ancestry when determining whether
one named type subsumes the other or when computing their shared named supertype. This SHALL apply
in addition to the existing record inheritance rules used for record-named types.

#### Scenario: Sibling external components unify to shared abstract base for annotated list

- **WHEN** a file contains `abstract external component <Question label:string /> external component <ShortTextQuestion extends Question /> external component <LongTextQuestion extends Question /> let questions: Question[] = { <ShortTextQuestion label={"Name"} /> <LongTextQuestion label={"Details"} /> }`
- **THEN** type checking SHALL report no errors for `questions`
- **AND** analysis SHALL treat the braced list elements as compatible with `Question`
