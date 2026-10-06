# function-reference-type Specification

## Purpose
Define the function reference type, `<function ... />: R` — a function type whose parameters are
not stated — so that a property can be declared to hold a function a host will call with arguments
it validates against the function's own parameters. The value is a reference to a declaration: NX
code passes, stores and compares it, and does not call it.

## Requirements

### Requirement: A function type may leave its parameters unspecified
Anywhere NX accepts a function type, the parser SHALL accept one whose parameter list is the token
`...`: `<`, the contextual keyword `function`, `...`, `/>`, `:` and a result type. It SHALL denote
the function reference type: the type of a function value of any parameters whose result satisfies
the stated result type. The result type SHALL be required and MAY carry any one occurrence suffix,
which binds to the result, as `function-types` defines for every function type, so an occurrence
over the function reference type itself SHALL be written with parentheses. `...` SHALL stand for
the whole parameter list: a function type that writes `...` and a parameter definition SHALL be
rejected with a diagnostic that says so. `...` SHALL be accepted only there: it SHALL NOT be
accepted in a function or component declaration's parameter list, in an element, or as a type on
its own. The type SHALL have no constructor: no element, record construction or literal produces a
value of it, and the only expression that produces a function value is the name of a visible
function, as `function-values` defines. The language SHALL NOT provide a name for the type: neither
a built-in type name nor a prelude declaration SHALL denote it, and `Function` written as a type
SHALL resolve as any other name does.

#### Scenario: A field is declared at a function reference type
- **WHEN** a file contains `type Args = { q:string } type Tool = { build: <function ... />: Args } let make(q:string, limit?:int): Args = <Args q={q} /> let t = <Tool build={make} />`
- **THEN** parsing, lowering and analysis SHALL accept the file
- **AND** `Tool.build` SHALL have the function reference type with the result `Args`

#### Scenario: A function reference type is aliased
- **WHEN** a file contains `type Args = { q:string } type Builder = <function ... />: Args`
- **THEN** a property declared `build:Builder` SHALL have the same type as one declared `build: <function ... />: Args`

#### Scenario: A suffix after the result binds to the result
- **WHEN** a file contains `type Maybe = <function ... />: string?` and `type Many = (<function ... />: string)+`
- **THEN** analysis SHALL treat `Maybe` as exactly one function whose result is `string?`
- **AND** `Many` as one or more functions each returning exactly one `string`

#### Scenario: The ellipsis cannot be mixed with parameters
- **WHEN** a file contains `type T = <function Item:string ... />: string` or `type U = <function ... Item:string />: string`
- **THEN** each SHALL be rejected
- **AND** the diagnostic SHALL say that `...` stands for the whole parameter list

#### Scenario: The ellipsis is not accepted elsewhere
- **WHEN** a file contains `let <Row ... />: string = "r"`, or `type T = { item:... }`
- **THEN** parsing SHALL reject each

#### Scenario: A result is required
- **WHEN** a file contains `type T = <function ... />`
- **THEN** parsing SHALL reject it as it rejects a function type with no result

#### Scenario: Function is not a type name
- **WHEN** a file contains `type Tool = { fn:Function }` and declares or imports no type named `Function`
- **THEN** analysis SHALL report `unresolved-type` naming `Function`, as it does today

### Requirement: A function of any parameters satisfies a function reference type by its result
A value SHALL satisfy the function reference type `<function ... />: R` when its type is a
function type, of any parameters, whose result satisfies `R`, or is a function reference type whose
result satisfies `R`. This covers the name of a visible function used as a value, in element or
paren style, declared in the same file, a same-library peer or an imported library, and a binding
declared at a function type. No parameter name, parameter type or `content` marking SHALL be
compared. The result SHALL be compared covariantly, by the rule `function-types` gives a function
type's result, and a mismatch SHALL be reported as a function type's result mismatch is, naming
the result the function returns and the one the type expects. Every result type satisfies
`object*`, so every function value SHALL satisfy `<function ... />: object*`; a function whose
result carries an occurrence SHALL NOT satisfy `<function ... />: object`. A value of any other
type SHALL NOT satisfy a function reference type, and an element with no declaration SHALL NOT
satisfy one whatever its tag. The bottom type SHALL satisfy it, as it satisfies every type. The
type of a function name used as a value SHALL stay that function's own function type: binding it
to a function reference site SHALL NOT change the type inferred for the name elsewhere.

#### Scenario: Functions of unlike signatures satisfy the widest type
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let <Row Item:object Index:int />: string = "r" let none(): string = "n" let many(): string* = {"a" "b"} let maybe(): string? = {} let a = <Tool fn={double} /> let b = <Tool fn={Row} /> let c = <Tool fn={none} /> let d = <Tool fn={many} /> let e = <Tool fn={maybe} />`
- **THEN** analysis SHALL accept `a`, `b`, `c`, `d` and `e`

#### Scenario: An exactly-one result excludes a result under an occurrence
- **WHEN** a file contains `type Tool = { fn: <function ... />: object } let one(): string = "n" let many(): string* = {"a" "b"} let a = <Tool fn={one} /> let b = <Tool fn={many} />`
- **THEN** analysis SHALL accept `a`
- **AND** SHALL reject the `fn` binding of `b`, the diagnostic naming the result `string*` and the expected result `object`

#### Scenario: The result is checked
- **WHEN** a file contains `type Args = { q:string } type Tool = { build: <function ... />: Args } let make(q:string): Args = <Args q={q} /> let wrong(q:string): string = {q} let a = <Tool build={make} /> let b = <Tool build={wrong} />`
- **THEN** analysis SHALL accept `a`
- **AND** SHALL reject the `build` binding of `b`, the diagnostic naming the result `string` and the expected result `Args`

#### Scenario: The result is covariant
- **WHEN** a file contains `abstract type Shape = { id:int } type Circle extends Shape = { r:int } let make(r:int): Circle = <Circle id=1 r={r} /> let ok: <function ... />: Shape = {make} let wide(): Shape = <Circle id=1 r=1 /> let bad: <function ... />: Circle = {wide}`
- **THEN** analysis SHALL accept `ok`
- **AND** SHALL reject `bad`

#### Scenario: A function-typed binding satisfies it
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let wrap(f: <function n:int />: int): Tool = <Tool fn={f} />`
- **THEN** analysis SHALL accept `wrap`

#### Scenario: A non-function is rejected
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } type Plan = { name:string } let a = <Tool fn="double" /> let b = <Tool fn=1 /> let c = <Tool fn=<Plan name="p" /> />`
- **THEN** analysis SHALL reject the `fn` binding of `a`, of `b` and of `c`
- **AND** each diagnostic SHALL show the expected type as `<function ... />: object*`

#### Scenario: An element named Function is not a function value
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let t = <Tool fn=<Function module="main.nx" name="double" /> />` and declares no element named `Function`
- **THEN** analysis SHALL reject the `fn` binding
- **AND** the diagnostic SHALL show the expected type as `<function ... />: object*`

#### Scenario: An undefined name is still undefined
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let t = <Tool fn={noSuchFunction} />`
- **THEN** analysis SHALL report `noSuchFunction` as undefined

#### Scenario: The function keeps its own type
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let double(n:int): int = {n * 2} let t = <Tool fn={double} /> let typed: <function n:int />: int = {double}`
- **THEN** analysis SHALL accept `typed`

### Requirement: A function reference type satisfies only a wider one and `object`
A value of the function reference type `<function ... />: A` SHALL satisfy `<function ... />: R`
when `A` satisfies `R`, and SHALL satisfy `object`. It SHALL NOT satisfy any function type with a
stated parameter list, whatever that type's parameters and result and including one with no
parameters, because the value's parameters are not known where it is checked. The diagnostic for
such a value at a site typed by a function type with stated parameters SHALL say that the value's
parameters are not stated. The language SHALL provide no conversion from a function reference type
to a function type with stated parameters. Inside a function type, the rule composes by the
existing variance: a function that declares a parameter at a function reference type SHALL satisfy
a function type that supplies that parameter at a function type with stated parameters, and a
function that declares the parameter at a function type with stated parameters SHALL NOT satisfy a
function type that supplies it at a function reference type.

#### Scenario: A function reference value is not a callable function value
- **WHEN** a file contains `let narrow(f: <function ... />: object*): <function n:int />: int = {f}` or `let none(f: <function ... />: int): <function />: int = {f}`
- **THEN** analysis SHALL reject each result
- **AND** the first diagnostic SHALL show the expected type `<function n:int />: int`, the found type `<function ... />: object*`, and SHALL say the value's parameters are not stated

#### Scenario: A narrower result satisfies a wider one
- **WHEN** a file contains `let widen(f: <function ... />: int): <function ... />: object* = {f}` and `let narrow(f: <function ... />: object*): <function ... />: int = {f}`
- **THEN** analysis SHALL accept `widen`
- **AND** SHALL reject `narrow` as a result mismatch

#### Scenario: A function reference value is an object
- **WHEN** a file contains `let widen(f: <function ... />: object*): object = {f}`
- **THEN** analysis SHALL accept `widen`

#### Scenario: A function reference parameter is checked contravariantly
- **WHEN** a file contains `type AnyFn = <function ... />: object*`, `let <Keep Fn:AnyFn />: string = "k" let ok: <function Fn:<function n:int />: int />: string = {Keep}` and `let <Use Fn:<function n:int />: int />: string = "u" let bad: <function Fn:AnyFn />: string = {Use}`
- **THEN** analysis SHALL accept `ok`
- **AND** SHALL reject `bad`

### Requirement: A function reference value is passed, stored and compared
NX code SHALL be able to bind a value of a function reference type to a record field, a component
prop, a function parameter, a `let` and a function result declared at that type, and to place it in
a sequence. Two such values SHALL compare by the declaration they name, in every runtime and under
every comparison `function-values` lists, and the comparison SHALL NOT depend on the type either
side was declared at: the checker SHALL accept `==` and `!=` between any two function types, with
stated or unspecified parameters, whether or not either satisfies the other. Evaluation SHALL NOT call the function such a value names when the value is
bound, stored, compared or rendered.

#### Scenario: A function reference value is forwarded and returned
- **WHEN** a file contains `type AnyFn = <function ... />: object* type Tool = { fn:AnyFn } let make(f:AnyFn): Tool = <Tool fn={f} /> let double(n:int): int = {n * 2} let root() = {make(double)}`
- **THEN** analysis SHALL accept the file
- **AND** evaluating `root` SHALL produce a `Tool` record whose `fn` field is the function value for `double`

#### Scenario: Function reference values compare by declaration
- **WHEN** a file contains `type AnyFn = <function ... />: object* let double(n:int): int = {n * 2} let greet(name:string): string = {name} let same(f:AnyFn, g:AnyFn): boolean = {f == g} let a() = {same(double, double)} let b() = {same(double, greet)}`
- **THEN** evaluating `a` SHALL produce `true` and `b` SHALL produce `false` in every runtime

#### Scenario: A function reference value compares with a function-typed value
- **WHEN** a file contains `let double(n:int): int = {n * 2} let same(f: <function ... />: object*, g: <function n:int />: int): boolean = {f == g} let a() = {same(double, double)}`
- **THEN** analysis SHALL accept `same`
- **AND** evaluating `a` SHALL produce `true`

### Requirement: A function reference value is not invocable
A binding whose type is a function reference type SHALL NOT be invocable, whatever the type's
result. An element whose tag is such a binding, `<f ... />`, and a paren-style call on one,
`f(...)`, SHALL each be rejected with a diagnostic that says the value cannot be called because
its parameters are not stated, and that the binding must be declared at a function type with its
parameters to be called. The rule SHALL apply to a parameter, a component prop and a top-level
`let`, and to a binding declared at the type under the optional mark or an occurrence. A tag that names such a binding SHALL NOT fall through to a lookup of a declared
element of the same name being reported as an unknown element.

#### Scenario: An element call on a function reference binding is rejected
- **WHEN** a file contains `let invoke(f: <function ... />: int, n:int): int = <f n={n} />`
- **THEN** analysis SHALL reject the element
- **AND** the diagnostic SHALL say `f` cannot be called because its parameters are not stated, and SHALL name a function type with parameters as the way to declare a callable parameter

#### Scenario: A paren call on a function reference binding is rejected
- **WHEN** a file contains `let invoke(f: <function ... />: int, n:int): int = {f(n)}`
- **THEN** analysis SHALL reject the call with the same diagnostic

#### Scenario: A function reference prop is not a tag
- **WHEN** a file contains `component <Section Row: <function ... />: object* /> = { <Row Item="a" /> }` and declares no element named `Row`
- **THEN** analysis SHALL reject the element with the diagnostic for calling a function reference value
- **AND** SHALL NOT report `Row` as an unknown element

### Requirement: A function reference type takes occurrences and defaults like any exactly-one type
A function reference type SHALL be an exactly-one type. Written out it SHALL take an occurrence
suffix in parentheses, `(<function ... />: R)+`, and under an alias it SHALL take one directly,
`AnyFn?`, `AnyFn+` and `AnyFn*`. It SHALL take the optional mark on a name, `fn?: <function ... />: R`.
Each SHALL have the meaning `occurrence-types` and `optional-properties` give any exactly-one type.
A field, prop or parameter declared at the type MAY carry a default value, which SHALL be an
ordinary expression checked against the type. The type MAY be a type argument of a generic record
or component, and MAY be a parameter type or the result type of a function type.

#### Scenario: Occurrences over a function reference type
- **WHEN** a file contains `type AnyFn = <function ... />: object* type Kit = { all:AnyFn+ extra?:AnyFn+ one?:AnyFn listed: (<function ... />: string)+ } let double(n:int): int = {n * 2} let greet(name:string): string = {name} let k = <Kit all={double greet} listed={greet} />`
- **THEN** analysis SHALL accept the file
- **AND** `Kit.all` SHALL read as one or more, `Kit.extra` as zero or more and `Kit.one` as zero or one of `<function ... />: object*`

#### Scenario: An empty value is rejected where one is required
- **WHEN** a file contains `type Tool = { fn: <function ... />: object* } let t = <Tool fn={} />`
- **THEN** analysis SHALL reject the binding, because `{}` does not satisfy an exactly-one type

#### Scenario: A default names a function
- **WHEN** a file contains `let identity(value:string): string = {value} type Tool = { fn: <function ... />: object* = {identity} } let t = <Tool />`
- **THEN** analysis SHALL accept the file
- **AND** evaluating `t` SHALL produce a `Tool` whose `fn` field is the function value for `identity`

#### Scenario: A function reference type is a type argument
- **WHEN** a file contains `type AnyFn = <function ... />: object* type Holder = { T:type item:T } let double(n:int): int = {n * 2} let h:<Holder T=AnyFn/> = <Holder T=AnyFn item={double} />`
- **THEN** analysis SHALL accept `h`

### Requirement: Two unlike function types join to the widest function reference type
Where inference joins two types and each is a function type, with stated or unspecified
parameters, the result SHALL be the one both satisfy when one satisfies the other, and SHALL
otherwise be `<function ... />: object*` rather than `object`. The results of the two SHALL NOT be
joined. The join of a function reference type with a type that is not a function type SHALL be
`object`, as before.

#### Scenario: A sequence of unlike functions is a sequence of the widest type
- **WHEN** a file contains `let double(n:int): int = {n * 2} let greet(name:string): string = {name} let both = {double greet} let tools: (<function ... />: object*)+ = {both} let anything:object+ = {both}`
- **THEN** analysis SHALL infer `both` as `(<function ... />: object*)+`
- **AND** SHALL accept `tools` and `anything`

#### Scenario: A function that satisfies the other joins to the other
- **WHEN** a file contains `let none(): string = "n" let one(n:int): string = "o" let both = {none one}`
- **THEN** analysis SHALL infer `both` as `(<function n:int />: string)+`, because `none` satisfies that type

#### Scenario: A function and a non-function still join to object
- **WHEN** a file contains `let double(n:int): int = {n * 2} let mixed = {double "x"}`
- **THEN** analysis SHALL infer `mixed` as `object+`

### Requirement: A function reference type is displayed in NX spelling
Wherever the system shows a function reference type to an author — a diagnostic, a hover, an
explained artifact — it SHALL spell it as it is written in source, `<function ... />: Result`, and
SHALL parenthesize it under a `?`, `+` or `*` suffix, as `function-types` requires of every
function type. It SHALL NOT spell it as a function type with no parameters, and SHALL NOT use an
arrow form.

#### Scenario: A mismatch diagnostic shows the type
- **WHEN** analysis reports `let t = <Tool fn="double" />` against `type Tool = { fn: <function ... />: object* }`
- **THEN** the diagnostic SHALL show the expected type as `<function ... />: object*` and the found type as `string`

#### Scenario: A function reference type under a suffix is parenthesized
- **WHEN** the system displays the read type of a field declared `fn?: <function ... />: object*`, or of one declared `all: (<function ... />: string)+`
- **THEN** it SHALL show `(<function ... />: object*)?` and `(<function ... />: string)+` respectively

#### Scenario: A hover shows the type as written
- **WHEN** a client requests hover on the field `build` of `type Tool = { build: <function ... />: Args }`
- **THEN** the fenced content SHALL read `(property) Tool.build: <function ... />: Args`
