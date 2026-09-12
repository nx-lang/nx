## ADDED Requirements

### Requirement: TypeScript runtime normalizes property union values as constant cases
When the TypeScript runtime normalizes a value whose declared type is a property union — from an
IR union-case expression or from host input at a boundary — it SHALL accept the bare string of any
case listed in the property union's declaration, SHALL reject any other value with a diagnostic
naming the property union and the offending value, and SHALL keep the value as that bare string.
A prepared program that lists the property-union feature in its required features SHALL be
accepted by a runtime that supports this change, and the feature name SHALL appear in the
diagnostic a runtime without support reports.

#### Scenario: Evaluated property union case is the bare field name
- **WHEN** a prepared IR program evaluates `User.Property.email` for `type User = { name:string email:string? }`
- **THEN** the result SHALL be the string `"email"`

#### Scenario: Host input for a property union is validated against the cases
- **WHEN** a caller passes `"nickname"` where a `User.Property` prop is expected
- **THEN** normalization SHALL fail with a diagnostic naming `User.Property` and `nickname`
- **AND** passing `"name"` SHALL be accepted unchanged

### Requirement: TypeScript runtime evaluates the update intrinsics and exports them as helpers
The TypeScript runtime SHALL evaluate the intrinsic call construct for `apply`, `merge`, `diff`,
and `changed` with the semantics the `update-records` capability specifies: present fields
replace, absent fields keep, a present `null` is carried, `merge` lets the second argument win,
`diff` lists only differing fields comparing records and lists structurally, and `changed` lists
present fields in declaration order. The result of `apply` SHALL be stamped with the target
record's `$type`, the results of `merge` and `diff` with the update record's `$type`, and the
result of `changed` SHALL be an array of bare strings. The runtime SHALL also export the same four
operations as functions a host can call on values it holds, typed so that the record and update
arguments share one record type parameter. A prepared program that lists the update-intrinsic
feature SHALL be accepted by a runtime that supports this change.

#### Scenario: Evaluated apply replaces present fields only
- **WHEN** a prepared IR program evaluates `apply(<User name="Ada" email="x@y" />, <User.Update email={null} />)` for `type User = { name:string email:string? }`
- **THEN** the result SHALL be `{ $type: "User", name: "Ada", email: null }`

#### Scenario: Evaluated diff and changed agree
- **WHEN** a prepared IR program evaluates `changed(diff(<User name="Ada" email="x@y" />, <User name="Bo" email="x@y" />))` for the same `User`
- **THEN** the result SHALL be `["name"]`

#### Scenario: Exported helper applies a host-held update
- **WHEN** a host calls the exported apply helper with `{ $type: "User", name: "Ada", email: "x@y" }` and `{ $type: "User.Update", email: null }`
- **THEN** the helper SHALL return `{ $type: "User", name: "Ada", email: null }`
- **AND** the exported merge helper called with `{ $type: "User.Update", name: "Ada" }` and `{ $type: "User.Update", name: "Bo" }` SHALL return `{ $type: "User.Update", name: "Bo" }`

#### Scenario: Exported helpers reject mismatched targets
- **WHEN** a host calls the exported apply helper with a `User` record and an update whose `$type` is `Team.Update`
- **THEN** the helper SHALL fail with a diagnostic naming both types
