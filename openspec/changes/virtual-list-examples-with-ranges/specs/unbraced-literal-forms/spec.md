## MODIFIED Requirements

### Requirement: An unresolvable bare name is a diagnostic that names the candidates
When a bare name does not resolve, the system SHALL report an error rather than silently accepting
the name as a string or as an unresolved identifier. The diagnostic SHALL name the expected type,
and SHALL list or suggest the constant cases of that type when it has any. At a type-parameter
site the diagnostic SHALL say that a type name is expected and SHALL suggest a near-matching
visible type when there is one. When the expected type is neither a discriminated union nor a type
parameter, the diagnostic SHALL say so and SHALL indicate the form the site does accept: where a
value of that name is lexically visible, it SHALL direct the author to the braced form `{name}`,
which references the binding; the quoted form SHALL be offered only where a string satisfies the
site. The diagnostic SHALL NOT propose a form the site would reject.

#### Scenario: Unknown member suggests a near match
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=containt />`
- **THEN** type checking SHALL reject `containt`
- **AND** the diagnostic SHALL report that `containt` is not a case of `Fit` and SHALL suggest
  `contain`

#### Scenario: Payload union case cannot be used as a bare name
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=failed />`
- **THEN** type checking SHALL reject `failed` because that case requires payload construction
- **AND** the diagnostic SHALL direct the author to the element-style form `<LoadState.failed ... />`

#### Scenario: Unknown type name at a type-parameter site suggests a near match
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type /> let v = <List TItem=Contatc />`
- **THEN** type checking SHALL reject `Contatc`
- **AND** the diagnostic SHALL report that `TItem` expects a type name and SHALL suggest `Contact`

#### Scenario: A visible binding is offered in its braced form
- **WHEN** a file contains `let r = 5` and `let x:int = r`
- **THEN** type checking SHALL reject the bare `r`
- **AND** the diagnostic SHALL direct the author to `{r}`
- **AND** it SHALL NOT propose writing `"r"`, which an `int` site does not accept

#### Scenario: The quoted form is offered only where a string fits
- **WHEN** a bare name that resolves to nothing is written at a site whose expected type is
  `string`
- **THEN** the diagnostic SHALL offer the quoted form
- **AND** where the expected type is a record, a range or another non-string type, it SHALL NOT
