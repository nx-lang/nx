## ADDED Requirements

### Requirement: Rust runtime evaluates environment values
The Rust IR runtime SHALL accept modules that require `environment-v1` and SHALL evaluate
`envRead` and `envProvide` nodes, entry-module provides and `$env` as the format defines them,
with the same results and operation counts as the TypeScript runtime. The runtime SHALL export an
environment type: an immutable value made from root values by `Environment::root`, and extended
with a component descriptor's `$env` by `enter`. Every evaluation, the component lifecycle
included, SHALL take an environment through its options, and an evaluation given none SHALL run
with no root values. Root values SHALL be named and checked as the environment-values capability
defines, and a malformed `$env` SHALL be refused with `nx-ir-boundary-type`.

#### Scenario: A function reads a root value
- **WHEN** a host evaluates `let mode() = { env.colorMode }` with the root value `colorMode` equal to
  `dark`
- **THEN** the result SHALL be `dark`

#### Scenario: A host composes a child's environment
- **WHEN** a host initializes `Page`, whose output holds a `Card` descriptor with `$env` overriding
  `colorMode` to `dark`, and initializes `Card` from that descriptor with the environment `enter`
  gives for it from `Page`'s environment
- **THEN** `Card`'s body SHALL read `dark` for `env.colorMode`

#### Scenario: Operation counts match the TypeScript runtime
- **WHEN** both runtimes evaluate the environment cases of the conformance corpus
- **THEN** their results and operation counts SHALL be the same

### Requirement: The CLI takes root environment values
`nxlang run` SHALL accept `--env name=value`, any number of times. `name` SHALL be named as a host
names a root value, and `value` SHALL be NX value text parsed and checked against the value's
declared type. A malformed or mistyped value, and an unknown or ambiguous name, SHALL be reported
as a diagnostic before the program runs, and the command SHALL exit with failure.

#### Scenario: A root value from the command line
- **WHEN** a file declares `environment colorMode: ColorMode = light` and its root element reads it,
  and the user runs `nxlang run app.nx --env colorMode=dark`
- **THEN** the output SHALL be the root rendered with `colorMode` equal to `dark`

#### Scenario: A value of the wrong type from the command line
- **WHEN** the user runs `nxlang run app.nx --env colorMode=dim`
- **THEN** the command SHALL fail with a diagnostic naming `colorMode` and its type
