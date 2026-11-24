## MODIFIED Requirements

### Requirement: TypeScript runtime evaluates environment values
The TypeScript IR runtime SHALL accept modules that require `environment-v1` or `environment-v2`
and SHALL evaluate `envRead`, `envProvide` and `envUpdate` nodes, entry-module provides and `$env` as
the format defines them, formulas and bundles included, with the same results and operation counts
as the Rust runtime. The runtime SHALL export an environment: an immutable value that
`environment(values)` makes from root values, and that `enter(descriptor)` extends with a component
descriptor's `$env`. An environment SHALL hold formulas as captured data, and the runtime SHALL
evaluate them with the prepared program of the evaluation that reads them, so `enter` needs no
program. Every evaluation, the component lifecycle included, SHALL take an environment through its
options, and an evaluation given none SHALL run with no root values. Root values SHALL be named and
checked as the environment-values capability defines, in their JavaScript form, and a malformed
`$env` SHALL be refused with `nx-ir-boundary-type` by `enter`, and a formula the program can't
evaluate SHALL be refused with `nx-ir-boundary-type` at the read that evaluates it.

#### Scenario: A component reads the environment its host gives it
- **WHEN** a host initializes `Badge`, whose body reads `env.colorMode`, with
  `runtime.environment({ colorMode: "dark" })`
- **THEN** the rendered output SHALL be the body with `colorMode` equal to `dark`

#### Scenario: A host composes a child's environment
- **WHEN** a host renders `Page`, whose output holds a `Card` descriptor with `$env` overriding
  `colorMode`, and initializes `Card` with `pageEnvironment.enter(cardDescriptor)`
- **THEN** `Card`'s body SHALL read the overriding value

#### Scenario: A formula from another program is refused
- **WHEN** a host enters a descriptor whose `$env` formula names a module fingerprint the program
  doesn't have, and the body reads that value
- **THEN** the evaluation SHALL fail with `nx-ir-boundary-type`
