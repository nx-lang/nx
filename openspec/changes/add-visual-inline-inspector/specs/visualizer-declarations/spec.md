## ADDED Requirements

### Requirement: Presentation hints that cannot be shown are warnings
Analysis SHALL report a `visualizer-hint` warning, and a viewer SHALL ignore the hint, for a `short`
that is empty or longer than two characters, and for a property named more than once in a
`summary`, where the viewer keeps its first place. A property named in a `summary` SHALL be shown
on the row even when the visualizer gives it no `PropertyVisualizer`.

#### Scenario: A long short label
- **WHEN** a property visualizer has `short="Width"`
- **THEN** analysis SHALL report `visualizer-hint` at the value, and a viewer SHALL show the property's
  label as a caption instead

#### Scenario: A repeated summary property
- **WHEN** an element visualizer has `summary={width width fill}`
- **THEN** analysis SHALL report `visualizer-hint` at the second `width`, and the row SHALL show
  `width` then `fill`

### Requirement: Labels, icons, short labels and summaries resolve like other hints
A property's `label`, `icon` and `short` SHALL each resolve on its own, as the editor hints
do: to what its resolved `PropertyVisualizer` sets, the derived or later visualizer winning, and
otherwise to none. An element's `summary` SHALL resolve whole to the derived or later visualizer's
that sets one. A viewer SHALL derive a label for a property without one from its name.

#### Scenario: A derived visualizer relabels one property
- **WHEN** a base element's visualizer gives `x` the label `"X"` and the short `"X"`, and a derived element's visualizer gives `x` only the label `"Left"`
- **THEN** the derived element's `x` SHALL resolve to the label `Left` and the short `X`

## MODIFIED Requirements

### Requirement: A visualizer declaration binds its visualizer records' `Target`
The system SHALL bind `Target` for every construction of an `@nx/visualizer` record in a visualizer
declaration's value that does not bind it, as if written `Target=X`. `X` SHALL be the target itself
when the target is a record, an action or an alias, and the target's derived props record
`T.Props` when the target is a component or an element-style function, so that its visualizer names
its props through `T.Props.Property`. A construction that binds `Target` explicitly SHALL bind it to `X`, and any other
binding SHALL be a diagnostic. This binding SHALL apply only inside a visualizer declaration's value;
everywhere else the ordinary rule that a generic record's construction binds every type parameter
SHALL hold.

#### Scenario: Children take the target too
- **WHEN** a module declares `type Choice = { value:string label:string }` and `visualizer Choice = <ElementVisualizer title=label><PropertyVisualizer property=value format=id /></ElementVisualizer>`
- **THEN** analysis SHALL accept the visualizer, checking `property=value` against `Choice.Property`

#### Scenario: A component's visualizer names its props
- **WHEN** a module declares `component <Question label:string help?:string /> = { <Panel /> }` and `visualizer Question = <ElementVisualizer title=label id=help />`
- **THEN** analysis SHALL accept the visualizer, checking `title` and `id` against `Question.Props.Property`

#### Scenario: A misspelled property is a type error
- **WHEN** a module declares `type Choice = { label:string }` and `visualizer Choice = <ElementVisualizer title=lable />`
- **THEN** analysis SHALL reject `lable` because it is not a case of `Choice.Property`

#### Scenario: A wrong explicit target
- **WHEN** a module declares `type Choice = { label:string }` and `type Other = { name:string }` and `visualizer Choice = <ElementVisualizer Target=Other title=name />`
- **THEN** analysis SHALL reject the binding `Target=Other` because the visualizer declaration's target is `Choice`

#### Scenario: Outside a visualizer declaration the rule is unchanged
- **WHEN** a module imports `@nx/visualizer` and contains `let h = <ElementVisualizer />`
- **THEN** analysis SHALL reject the element because `Target` was not specified

#### Scenario: An element function's visualizer names its properties
- **WHEN** a module declares `let <Badge label:string tone:string = "neutral" /> = <Label text={label} />` and `visualizer Badge = <FunctionVisualizer summary={tone}><PropertyVisualizer property=tone label="Tone" /></FunctionVisualizer>`
- **THEN** analysis SHALL accept the visualizer, checking `tone` against `Badge.Props.Property`
