## MODIFIED Requirements

### Requirement: A bare name is a contextual literal resolved against the expected type
An unquoted single identifier SHALL be accepted as a property value, and SHALL resolve against the
declared type of the binding site rather than against lexical scope. The system SHALL resolve it
only against a closed nominal set determined by the site: at a site typed by a discriminated union,
the constant cases of that union; at a type-parameter site of a component, the type names visible
at the use site. Resolution SHALL NOT consult variables, parameters, imports of values, or any
other lexical value binding, and a bare name SHALL NOT be affected by whether an identically named
value binding is in scope.

#### Scenario: Bare name resolves to an enum member at an enum-typed property
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` as a value of type `Fit`
- **AND** interpretation SHALL produce the same value as `fit={Fit.cover}`

#### Scenario: Bare name resolves to a payloadless union case
- **WHEN** a file contains `type LoadState = idle | failed { message:string } component <View state:LoadState /> = { <div /> } let v = <View state=idle />`
- **THEN** type checking SHALL accept `idle` as a value of type `LoadState`
- **AND** interpretation SHALL produce the constant case value of `LoadState.idle`

#### Scenario: A lexical binding of the same name does not shadow the member
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit /> = { <img /> } let cover = "something else" let v = <Img fit=cover />`
- **THEN** type checking SHALL resolve `cover` to the constant case `Fit.cover`
- **AND** it SHALL NOT resolve `cover` to the `let` binding

#### Scenario: Nullable expected type accepts a bare name
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fit:Fit? /> = { <img /> } let v = <Img fit=cover />`
- **THEN** type checking SHALL accept `cover` by resolving against the underlying union type `Fit`

#### Scenario: List-typed site accepts a bare name through scalar-to-list coercion
- **WHEN** a file contains `type Fit = fill | contain | cover component <Img fits:Fit[] /> = { <img /> } let v = <Img fits=cover />`
- **THEN** type checking SHALL resolve `cover` against the list's element type `Fit`
- **AND** the existing scalar-to-list coercion at typed binding sites SHALL apply to the resolved
  value

#### Scenario: Bare name resolves to a visible type at a type-parameter site
- **WHEN** a file contains `type Contact = { name:string } external component <List TItem:type items:TItem[]? /> let Contact = "shadow" let v = <List TItem=Contact />`
- **THEN** type checking SHALL resolve `TItem=Contact` to the record type `Contact`
- **AND** it SHALL NOT consult the `let` binding named `Contact`

### Requirement: An unresolvable bare name is a diagnostic that names the candidates
When a bare name does not resolve, the system SHALL report an error rather than silently accepting
the name as a string or as an unresolved identifier. The diagnostic SHALL name the expected type,
and SHALL list or suggest the constant cases of that type when it has any. At a type-parameter
site the diagnostic SHALL say that a type name is expected and SHALL suggest a near-matching
visible type when there is one. When the expected type is neither a discriminated union nor a type
parameter, the diagnostic SHALL say so and SHALL indicate the form the site does accept.

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
