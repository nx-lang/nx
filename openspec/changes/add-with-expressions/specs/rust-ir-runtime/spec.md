## MODIFIED Requirements

### Requirement: Rust runtime evaluates environment values
The Rust IR runtime SHALL accept modules that require `environment-v1` or `environment-v2` and SHALL
evaluate `envRead`, `envProvide` and `envUpdate` nodes, entry-module provides and `$env` as the
format defines them, formulas and bundles included, with the same results and operation counts as
the TypeScript runtime. The runtime SHALL export an environment type: an immutable value made from
root values by `Environment::root`, and extended with a component descriptor's `$env` by `enter`.
An environment SHALL hold formulas as captured data, and the runtime SHALL evaluate them with the
prepared program of the evaluation that reads them, so `enter` needs no program. Every evaluation,
the component lifecycle included, SHALL take an environment through its options, and an evaluation
given none SHALL run with no root values. Root values SHALL be named and checked as the
environment-values capability defines, and a malformed `$env` SHALL be refused with `nx-ir-boundary-type` by
`enter`, and a formula the program can't evaluate SHALL be refused with `nx-ir-boundary-type` at
the read that evaluates it.

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

#### Scenario: A host composes an environment with a formula
- **WHEN** `Page`'s output holds a `Card` descriptor whose `$env` holds a formula for `gap`, and a
  host initializes `Card` with the environment `enter` gives for it
- **THEN** `Card`'s reads of `env.gap` SHALL evaluate the formula in `Card`'s environment
