## ADDED Requirements

### Requirement: TypeScript runtime evaluates environment values
The TypeScript IR runtime SHALL accept modules that require `environment-v1` and SHALL evaluate
`envRead` and `envProvide` nodes, entry-module provides and `$env` as the format defines them, with
the same results and operation counts as the Rust runtime. The runtime SHALL export an environment:
an immutable value that `environment(values)` makes from root values, and that `enter(descriptor)`
extends with a component descriptor's `$env`. Every evaluation, the component lifecycle included,
SHALL take an environment through its options, and an evaluation given none SHALL run with no root
values. Root values SHALL be named and checked as the environment-values capability defines, in
their JavaScript form, and a malformed `$env` SHALL be refused with `nx-ir-boundary-type`.

#### Scenario: A component reads the environment its host gives it
- **WHEN** a host initializes `Badge`, whose body reads `env.colorMode`, with
  `runtime.environment({ colorMode: "dark" })`
- **THEN** the rendered output SHALL be the body with `colorMode` equal to `dark`

#### Scenario: A host composes a child's environment
- **WHEN** a host renders `Page`, whose output holds a `Card` descriptor with `$env` overriding
  `colorMode`, and initializes `Card` with `pageEnvironment.enter(cardDescriptor)`
- **THEN** `Card`'s body SHALL read the overriding value
