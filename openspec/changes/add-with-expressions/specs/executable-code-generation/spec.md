## MODIFIED Requirements

### Requirement: Generated code evaluates environment values
Generated TypeScript and JavaScript SHALL evaluate `env.` reads, `provide` blocks, elements with
`env.` attributes, bundles, `with` steps and the entry module's provides with the results the native
runtime gives, including the `$env` field of component descriptors. A caller SHALL run a generated
entry function in an environment with the `nxWithEnvironment` helper, and component evaluation SHALL
take an environment through its options, made from root values with the `nxEnvironment` helper the
generated program exports; an evaluation given none SHALL run with no root values. A component
descriptor's `$env` SHALL be what the generated program's `enter` reads, so a host of generated code
composes a child's environment as a host of the TypeScript runtime does. Its entries SHALL have the
runtime's shape, except that a formula SHALL be a function of no arguments that closes over its
captured locals and reads the environment through the generated helpers, in place of the runtime's
`module`, `node`, `fingerprint` and `locals`; such an entry is meaningful only to the generated
program that made it. Because a bundle's formulas are evaluated where a value is read, a top-level
bundle evaluated when the generated module loads SHALL still follow the environment of each read.

#### Scenario: A generated function reads a provided value
- **WHEN** NX source declares `environment locale: string = "en-US"` and
  `let label() = { provide env.locale="fr-FR" { env.locale } }`, and a caller runs the generated
  JavaScript for `label`
- **THEN** the result SHALL be `"fr-FR"`

#### Scenario: A generated component reads a root value
- **WHEN** a caller evaluates a generated component whose body reads `env.colorMode` with an
  environment whose `colorMode` is `dark`
- **THEN** the generated output SHALL equal the native runtime's for the same environment

#### Scenario: A generated main-module theme follows a local mode
- **WHEN** the main module contains `provide env with brand`, `brand`'s `color` matches on
  `env.colorMode`, and generated JavaScript evaluates
  `let darkAccent() = { provide env.colorMode=dark { env.color.accent } }`
- **THEN** the result SHALL be `brand`'s dark accent, as in the IR runtimes

#### Scenario: A generated formula in `$env`
- **WHEN** generated JavaScript renders `<Card env.gap={env.space * 2} />`
- **THEN** the `Card` descriptor's `$env` entry for `gap` SHALL hold a function in `formula`
- **AND** entering the descriptor and reading `env.gap` under `env.space=1` SHALL give `2`
