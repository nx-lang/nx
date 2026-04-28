## ADDED Requirements

### Requirement: Record-compatible construction rejects unknown fields
The system SHALL treat NX record-compatible construction as closed by default. Type checking SHALL
reject any supplied field name that is not part of the target's effective declared field set.
Runtime evaluation SHALL perform the same validation before applying defaults, content fields, or
type coercions so direct evaluation paths cannot silently drop stale fields. This requirement
applies to plain records, action records, inherited record shapes, and element-style construction of
record-compatible targets.

#### Scenario: Record literal unknown field is rejected
- **WHEN** source contains `type ChatLinkConfig = { standaloneAppearance:string } let config = ChatLinkConfig { accentColor: "#3b82f6" standaloneAppearance: "split" }`
- **THEN** type checking SHALL reject `accentColor` because it is not a field of `ChatLinkConfig`
- **AND** the diagnostic code SHALL identify the problem as an unknown record field

#### Scenario: Element-style record unknown field is rejected
- **WHEN** source contains `type ChatLinkConfig = { standaloneAppearance:string } let config = <ChatLinkConfig accentColor={"#3b82f6"} standaloneAppearance={"split"} />`
- **THEN** type checking SHALL reject `accentColor` because it is not a field of `ChatLinkConfig`
- **AND** evaluation SHALL NOT produce a `ChatLinkConfig` value with `accentColor` discarded

#### Scenario: Action record unknown field is rejected
- **WHEN** source contains `action SearchSubmitted = { query:string } let action = <SearchSubmitted query={"docs"} source={"toolbar"} />`
- **THEN** type checking SHALL reject `source` because it is not a field of `SearchSubmitted`

#### Scenario: Inherited record construction accepts inherited fields and rejects unrelated fields
- **WHEN** source contains `abstract type AppearanceBase = { variant:string } type SplitAppearance extends AppearanceBase = { links:string[]? } let appearance = <SplitAppearance variant={"split"} links={ "docs" } accentColor={"#3b82f6"} />`
- **THEN** type checking SHALL accept inherited field `variant`
- **AND** type checking SHALL reject `accentColor` because it is not in the effective field set of
  `SplitAppearance`

#### Scenario: Runtime record construction rejects unknown fields when static analysis is bypassed
- **WHEN** runtime evaluation is asked to construct `ChatLinkConfig` from supplied fields
  `standaloneAppearance` and `accentColor`
- **AND** `ChatLinkConfig` declares only `standaloneAppearance`
- **THEN** evaluation SHALL fail with an unknown-record-field runtime error
- **AND** evaluation SHALL NOT return a value that silently omits `accentColor`

### Requirement: Strict record construction preserves defaults and required-field checks
Strict unknown-field validation SHALL NOT remove existing record construction behavior for known
fields. The system SHALL continue to apply defaults, require non-defaulted non-nullable fields, and
coerce compatible values for declared fields after all supplied field names have been validated.

#### Scenario: Defaulted known field still applies
- **WHEN** source contains `type User = { name:string role:string = "member" } let user = <User name={"Ava"} />`
- **THEN** type checking SHALL accept the construction
- **AND** interpretation SHALL include `role = "member"` on the resulting record value

#### Scenario: Missing required known field remains rejected
- **WHEN** source contains `type User = { name:string role:string } let user = <User name={"Ava"} />`
- **THEN** type checking SHALL reject the construction because required field `role` is missing

#### Scenario: Known field type checking still runs after field-name validation
- **WHEN** source contains `type User = { name:string age:int } let user = <User name={"Ava"} age={"old"} />`
- **THEN** type checking SHALL reject `age` because the supplied value is not compatible with `int`
