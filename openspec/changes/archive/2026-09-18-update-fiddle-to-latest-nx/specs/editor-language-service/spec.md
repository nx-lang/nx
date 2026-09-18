## ADDED Requirements

### Requirement: Language service offers and describes handler properties
This requirement refines "Language service exposes conservative completions" and "Language service
exposes hover information" for one kind of property rather than stating a second rule for them: a
component accepts an `on<Emit>` property for each action it emits, its own and those it inherits,
and the compiler checks a handler bound there like any other property. The language service SHALL
treat it as one, so completions and hover SHALL answer at an `on<Emit>` name on the terms those two
requirements already set for a declared property — offered where the opening tag has not supplied
it, looked up through the same import graph and inheritance chain, so an emit declared on a base in
another module is offered on a component that extends it. Hover on a written `on<Emit>` name SHALL
name the action the handler handles, which is what a declared property's type would say.

#### Scenario: Property completions include a handler for each emit
- **WHEN** a component `Toggle` declares `emits { Toggled { value:boolean } }` and extends an abstract
  component that declares `emits { Tapped { } }`
- **AND** a client requests completions at a property-name position inside `<Toggle />`
- **THEN** the completions SHALL include `onToggled` and `onTapped` alongside `Toggle`'s declared
  properties

#### Scenario: A bound handler is not offered again
- **WHEN** the opening tag already binds `onTapped`
- **THEN** property-name completions in that tag SHALL NOT offer `onTapped`
- **AND** they SHALL still offer `onToggled`

#### Scenario: Hover over a handler property names its action
- **WHEN** a client requests hover on `onToggled` written in the opening tag of a `Toggle` element
- **THEN** the hover content SHALL identify `Toggle.onToggled` as a property
- **AND** it SHALL name `Toggle.Toggled` as the action the handler handles
