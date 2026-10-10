## ADDED Requirements

### Requirement: TypeScript runtime reports the origins of the records it returns
The TypeScript runtime's options SHALL accept `origins`. When it is true, the results of
`evaluateFunction`, `initializeComponent`, `evaluateComponent` and `dispatchComponentActions` SHALL
carry an `origins` list with one entry for each record in the value the result returns (the
function's value, or the rendered output), naming the record by its JSON pointer within that value
and giving the identity of the module and the start and end byte offsets of the IR node that
constructed it, read from the image's debug section, as the `value-origin` capability defines an
origin. A record whose constructing node's image has no debug section SHALL have no entry, and that
SHALL NOT be an error. When the option is absent or false, the results SHALL be exactly what they
are without it.

#### Scenario: Rendered output with origins
- **WHEN** a host initializes a component whose body renders `<Screen><Label text="Hi" /></Screen>`
  from an image built with its debug section, with `origins: true`
- **THEN** the result's `origins` SHALL hold an entry for `""` and one for the `Label` record's
  pointer within the rendered output, each with the byte span of its element in the source

#### Scenario: An image without its debug section
- **WHEN** a host makes the same call with an image built without its debug section
- **THEN** the call SHALL succeed and its `origins` SHALL be empty

#### Scenario: The option is off by default
- **WHEN** a host makes the same call without the option
- **THEN** the result SHALL carry no `origins` field
