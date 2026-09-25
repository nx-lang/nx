## ADDED Requirements

### Requirement: A paren-style function is called by position or as an element; an element-style function only as an element
A function declared paren style, `let name(a:T, b:U)`, SHALL be callable by position,
`name(x, y)`, and as an element, `<name a={x} b={y} />`, which binds arguments by name. A function
declared element style, `let <Name a:T b:U />`, SHALL be callable only as an element: a paren-style
call of it SHALL be rejected with a diagnostic that shows the element form, because its signature
lists attributes and their order is not part of what a caller relies on.

The paren style is meant for small, general-purpose functions a program calls in many places and
that are conventionally called with parentheses, such as `min`, `floor` or `rgb`; the element style
is meant for everything else. Neither the size of a paren-style signature nor how often it is
called is enforced.

#### Scenario: A paren-style function is called both ways
- **WHEN** a file contains `let add(a:int, b:int) = { a + b }`, `let x = { add(1, 2) }` and `let y = <add a=1 b=2 />`
- **THEN** type checking SHALL accept both calls
- **AND** both SHALL evaluate to `3`

#### Scenario: An element-style function cannot be called by position
- **WHEN** a file contains `let <F a:int b:int /> = { a + b }` and `let x = { F(1, 2) }`
- **THEN** type checking SHALL reject the call with a diagnostic that shows `<F a=... b=... />`

### Requirement: A paren-style function lists the parameters a caller may omit last
A parameter a caller may omit is one marked `?` or one with a default. In a paren-style function
every such parameter SHALL follow every parameter a caller must supply; a required parameter after
an omissible one SHALL be rejected, naming both. Among the omissible parameters, optional and
defaulted ones MAY appear in any order. An element-style function SHALL order its parameters
freely.

#### Scenario: A required parameter after an optional one is rejected
- **WHEN** a file contains `let f(a?:int, b:int) = { b }`
- **THEN** validation SHALL reject `b`, naming `a` as the parameter a caller may omit

#### Scenario: A required parameter after a defaulted one is rejected
- **WHEN** a file contains `let f(a:int = 1, b:int) = { b }`
- **THEN** validation SHALL reject `b`, naming `a`

#### Scenario: An element-style function may put an optional parameter first
- **WHEN** a file contains `let <F a?:int b:int /> = { b }` and `let x = <F b=1 />`
- **THEN** validation and type checking SHALL accept the file

### Requirement: A call leaves out a parameter the function fills
A positional call SHALL supply at least every required parameter and MAY stop before any of the
trailing omissible ones; supplying fewer or more arguments than that SHALL be rejected, and the
diagnostic SHALL state the accepted range when it is one. An element call SHALL be able to leave
out any omissible parameter, wherever it stands in the signature. A parameter a call leaves out
SHALL bind the parameter's default when it has one, and the empty value when it is optional. A host
calling a function by position SHALL follow the same rule.

#### Scenario: A positional call leaves out trailing parameters
- **WHEN** a file contains `let add(a:int, b:int = { a * 10 }, c?:int) = { a + b + (c ?? 0) }`
- **THEN** `add(1)` SHALL evaluate to `11`, `add(1, 2)` to `3` and `add(1, 2, 3)` to `6`

#### Scenario: A positional call must reach every required parameter
- **WHEN** a file contains `let f(a:int, b:int = 2, c?:int) = { a }` and `let x = { f() }`
- **THEN** type checking SHALL reject the call, stating that `f` expects 1 to 3 arguments

#### Scenario: An element call leaves out a parameter in the middle
- **WHEN** the file above also contains `let y = <add a=1 c=5 />`
- **THEN** `y` SHALL evaluate to `16`, `b` taking its default

#### Scenario: A host call leaves out trailing parameters
- **WHEN** a host calls `add` with the single argument `2`
- **THEN** the result SHALL be `22`

### Requirement: The function evaluates its own defaults
A parameter default SHALL be evaluated by the function, when a call leaves the parameter out, and
never copied into the calling program: a library that changes a default SHALL change what its
existing callers receive without their source changing. A default SHALL see the parameters
declared before it and the declarations of the function's own module, including private ones,
and SHALL NOT see a parameter declared after it, which SHALL be reported as undefined. A default
SHALL be checked against its parameter's declared type, and its value SHALL be lifted to the
parameter's occurrence as an argument's is. A function type's parameters SHALL NOT take a default,
since a caller supplies every one.

#### Scenario: A default reads an earlier parameter
- **WHEN** a file contains `let <Area w:int h:int = { w } /> = { w * h }` and `let s = <Area w=3 />`
- **THEN** `s` SHALL evaluate to `9`

#### Scenario: A default reads a value its module keeps private
- **WHEN** a library module contains `private let honorific = "Dr."` and `export let title(name:string, prefix:string = { honorific }): string = { prefix + " " + name }`, and another module calls `title("Ada")`
- **THEN** the call SHALL evaluate to `"Dr. Ada"` in every engine

#### Scenario: A default cannot read a later parameter
- **WHEN** a file contains `let f(a:int = { b }, b:int = 1) = { a }`
- **THEN** analysis SHALL report `b` as undefined in `a`'s default

#### Scenario: A default is checked against its parameter
- **WHEN** a file contains `let f(a:int, b:int = "two") = { a }`
- **THEN** type checking SHALL reject the default, naming `int` and `string`

#### Scenario: A default is lifted to its parameter's occurrence
- **WHEN** a file contains `let count(xs:int+ = { 7 }) = { xs }` and `let c = { count() }`
- **THEN** `c` SHALL evaluate to `[7]`
