## ADDED Requirements

### Requirement: Type analysis reaches every expression an element encloses
Type analysis SHALL infer a type for every lowered expression reachable from a module's items,
including expressions written as the body content or the property values of an element, and SHALL do
so regardless of whether the element's tag resolves to a declaration in scope.

<para>An element whose tag names no function, component, record, or union case has no binding
contract to check its content and properties against, and the analysis SHALL still infer each of
those expressions on its own terms and report the diagnostics that inference produces. The absent
tag SHALL be the only thing left unchecked about such an element.</para>

<para>The recorded type of every such expression SHALL be available to callers through the analysis
result's type environment, on the same terms as an expression written outside any element.</para>

#### Scenario: A type error inside an unresolved element's content is reported
- **WHEN** a module contains `let root() = <div>{1 + "a"}</div>` and `div` names no declaration in
  scope
- **THEN** the analysis result SHALL include the diagnostic rejecting the addition of `int` and
  `string`
- **AND** it SHALL be the same diagnostic reported for `let root() = { 1 + "a" }`

#### Scenario: A type error in an unresolved element's property value is reported
- **WHEN** a module contains `let root() = <div class={1 + "a"} />` and `div` names no declaration in
  scope
- **THEN** the analysis result SHALL include the diagnostic rejecting the addition of `int` and
  `string`

#### Scenario: An element nested inside an unresolved element is still checked against its declaration
- **WHEN** a module declares `let <Wrap content:string /> = <div />`
- **AND** contains `let root() = <div><Wrap content={42} /></div>`
- **THEN** the analysis result SHALL include the diagnostic reporting that `content` on `Wrap`
  expects `string` and found `int`
- **AND** it SHALL be the same diagnostic reported when the `<Wrap />` element is written outside the
  `<div>`

#### Scenario: A property supplied twice on an unresolved element is reported
- **WHEN** a module contains `let root() = <div class=1 class=2 />` and `div` names no declaration
  in scope
- **THEN** the analysis result SHALL report that `class` is supplied more than once
- **AND** it SHALL report it on the same terms as a duplicate supplied to a resolved element,
  because detecting one reads the element's own property list rather than any contract

#### Scenario: An unresolved tag reports nothing beyond the tag itself
- **WHEN** a module contains `let root() = <div>{"ok"}</div>` and `div` names no declaration in scope
- **THEN** the analysis result SHALL report no diagnostic for the content expression

#### Scenario: The type of an expression inside an unresolved element is recorded
- **WHEN** a module contains `let <Row count:int /> = <div>{count}</div>`
- **THEN** the analysis result's type environment SHALL record `int` for the `count` expression
  written inside the `<div>`
