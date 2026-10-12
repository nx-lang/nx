## ADDED Requirements

### Requirement: Generated code leaves out viewer declarations and props records
`typegen` SHALL emit nothing for a `viewer` declaration, and a module's generated output SHALL be
identical with and without its viewer declarations. `typegen` SHALL emit no type for a component's
derived `T.Props`, `T.Props.Update` or `T.Props.Property` in this release; an exported declaration
whose generated type would refer to one SHALL be a `typegen` error naming the declaration and the
props record. A generic record field typed by a type parameter's property union, `P.Property`,
SHALL be generated in TypeScript as `Extract<keyof P, string>` and in C# as `string`.

#### Scenario: Hints change nothing in generated code
- **WHEN** the user runs `nxlang typegen` on a library whose module declares `export type Choice = { label:string }` and `viewer Choice = <ElementViewer title=label />`
- **THEN** the generated files SHALL be identical to those for the same library without the viewer declaration

#### Scenario: A props record in an exported signature
- **WHEN** a library exports `component <Card title:string /> = { <Panel /> }` and `export type Saved = { props:Card.Props }`, and the user runs `nxlang typegen` on it
- **THEN** the command SHALL fail with an error naming `Saved` and `Card.Props`

#### Scenario: The viewer library generates
- **WHEN** the user runs `nxlang typegen @nx/viewer --language typescript --output ./generated`
- **THEN** `viewer.ts` SHALL declare `ElementViewer<Target>` with `title?: Extract<keyof Target, string>`
