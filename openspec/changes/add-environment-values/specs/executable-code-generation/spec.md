## ADDED Requirements

### Requirement: Generated code evaluates environment values
Generated TypeScript and JavaScript SHALL evaluate `env.` reads, `provide` blocks, elements with
`env.` attributes and the entry module's provides with the results the native runtime gives,
including the `$env` field of component descriptors. A caller SHALL run a generated entry function
in an environment with the `nxWithEnvironment` helper, and component evaluation SHALL take an
environment through its options, made from root values with the `nxEnvironment` helper the
generated program exports; an evaluation given none SHALL run with no root values. A component
descriptor's `$env` SHALL be what the runtime's `enter` reads, so a host of generated code
composes a child's environment as a host of the TypeScript runtime does.

#### Scenario: A generated function reads a provided value
- **WHEN** NX source declares `environment locale: string = "en-US"` and
  `let label() = { provide env.locale="fr-FR" { env.locale } }`, and a caller runs the generated
  JavaScript for `label`
- **THEN** the result SHALL be `"fr-FR"`

#### Scenario: A generated component reads a root value
- **WHEN** a caller evaluates a generated component whose body reads `env.colorMode` with an
  environment whose `colorMode` is `dark`
- **THEN** the generated output SHALL equal the native runtime's for the same environment
