## ADDED Requirements

### Requirement: A resolved visualizer carries its editor hints
Each property in a declaration's `visualizer` `properties` list SHALL carry, when a `sourceTree`
request asks for `visualizers`, besides its name and format, an `editor` value holding its resolved
editor hints that fit its type, as the visualizer-declarations capability resolves them: `kind`
(`field`, `slider`, `dropdown` or `segmented`), `min`, `max`, `step`, `stops`, `unit`, `multiline`
and `placeholder`, each only when set. A hint reported as `editor-hint-mismatch` SHALL be left out,
and a property's own mismatched hint SHALL NOT hide its type's hint of the same name. A `min` above
the `max`, reported as `editor-hint-range`, SHALL leave out both.
An alias's `visualizer` value SHALL carry its own hints the same way. A property with no hints SHALL
carry no `editor` value, and an entry whose only resolved values are hints SHALL carry a `visualizer`
value as one with a format does. The values SHALL round-trip through JSON without loss.

#### Scenario: A slider's hints
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<Card padding=8 />`, where `Card.padding` is typed `Spacing`, hinted `editor=slider min=0 max=64
  stops={ 0 4 8 16 } unit="px"`
- **THEN** the `properties` entry for `padding` in `Card`'s `visualizer` value SHALL carry an `editor`
  value with the kind `slider`, the minimum `0`, the maximum `64`, the stops `0`, `4`, `8` and `16`,
  and the unit `px`

#### Scenario: A mismatched hint is left out
- **WHEN** `Card.title` is a `string` hinted `editor=slider placeholder="Title"`
- **THEN** the entry for `title` SHALL carry an `editor` value with the placeholder `Title` and no kind

#### Scenario: An empty range is left out
- **WHEN** `Card.padding` is typed `Spacing`, hinted `min=10 max=0`
- **THEN** the entry for `padding` SHALL carry neither a minimum nor a maximum

### Requirement: The types of unwritten properties are listed
When a `sourceTree` request asks for `visualizers`, the declaration table SHALL also list the
declaration that types each property of every record, component, element function or union case the document's
nodes refer to, written or not, and the declaration each listed alias names, through its chain, so a
reader can edit a property an element does not write with its type's editor. A declaration listed only
this way SHALL NOT bring the types of its own properties.

#### Scenario: A union no element writes
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<Card title="Hi" />`, where `Card.size` is typed `Size`, a union `small | medium | large` that no
  node of the document refers to
- **THEN** the declaration table SHALL list `Size` with its three cases
