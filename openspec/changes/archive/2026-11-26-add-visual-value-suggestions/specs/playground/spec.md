## ADDED Requirements

### Requirement: The Visual view offers named values
The playground's Visual view SHALL ask for the source tree with `suggestions`, with the `visualizers`
it already asks for, so that its inspectors offer the named values of each property's type, and SHALL
draw the `named-values` icon. A `nx-edit` that chooses a named value SHALL be applied as any other
edit from Visual. The Visual test cases page SHALL hold an example that imports `@nx/design` and
writes properties with tokens, with literals and unset.

#### Scenario: Choosing a token in the playground
- **WHEN** a visitor on that example opens the named-values popover of a Card's `Padding` in Visual
  and chooses `env.space.lg`
- **THEN** the source SHALL read `padding={env.space.lg}`, and undo SHALL take it back
