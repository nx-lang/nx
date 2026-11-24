## ADDED Requirements

### Requirement: Generated code reuses derived results within one call
Generated TypeScript and JavaScript SHALL reuse derived results within one call of a generated
entry function or one component evaluation under the rule the IR runtimes follow: a top-level value
that reads the environment, an environment value's default, an entry-module provide and a captured
formula SHALL be evaluated again only when an environment value the last evaluation read would now
be found from a different override. The generated environment state SHALL give each layer it pushes
an identity of its own, and the kept results SHALL belong to that state, so no result is kept from
one call to the next. A top-level value that reads no environment SHALL remain a constant evaluated
when the module loads. Reuse SHALL change no result.

#### Scenario: A generated default is evaluated once in one environment
- **WHEN** generated JavaScript evaluates
  `let both() = { [env.color.accent, env.color.surface] }`, where `color`'s default reads
  `env.colorMode`
- **THEN** `color`'s default SHALL be evaluated once
- **AND** the result SHALL equal the IR runtimes'

#### Scenario: A generated value follows a new override of what it reads
- **WHEN** NX source declares `environment space: int = 4`, `let tv: int = { env.space * 3 }` and
  `let pair() = { [tv, provide env.space=1 { tv }] }`, and a caller runs the generated JavaScript
  for `pair`
- **THEN** the result SHALL be `[12, 3]`, as the IR runtimes give
