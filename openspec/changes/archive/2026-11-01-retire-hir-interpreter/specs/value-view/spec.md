## MODIFIED Requirements

### Requirement: One element shows an NX value in any host
The repository SHALL publish `@nx-lang/value-view`, which defines an `<nx-value>` custom element
that shows an `NxValueText`, annotated NX text produced by value formatting independently of execution. The element SHALL depend
on no UI framework and SHALL keep its markup and styles in a shadow root, so a host needs only to
import the package, create the element and set its `value`.

#### Scenario: A plain page
- **WHEN** a page with no framework imports the package, adds `<nx-value>` and sets its `value`
- **THEN** the element SHALL show the value's text

#### Scenario: A React page
- **WHEN** a React 19 component renders `<nx-value>` and passes the value as a property
- **THEN** the element SHALL show the value's text, with no wrapper component needed

#### Scenario: Host styles don't reach in
- **WHEN** the host page styles `pre`, `span` or `button` elements globally
- **THEN** the element's rendering SHALL not change
