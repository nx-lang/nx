## MODIFIED Requirements

### Requirement: The answer carries resolved visualizer declarations when asked
A `sourceTree` request SHALL accept an optional `visualizers` option, holding an optional list of host
visualizer module identities and an optional includeUnreferencedDeclarations boolean, defaulting to false. When the option is absent the answer SHALL carry no visualizers and SHALL be
unchanged from an answer without this feature. When it is present, the answer SHALL resolve the
program's visualizer declarations and the named host visualizer modules, as the visualizer-declarations capability defines, and
SHALL attach to each declaration entry with a resolved visualizer a `visualizer` value:

- for a record, an action or a component: its `title`, `id`, `icon` and `preview` descriptor when set, its `idPosition`
  when it has an `id`, and a `properties` list holding every property of the entry once, in the
  resolved order (the properties the visualizer lists, then the rest in declaration order), each with
  its name and its resolved `format` when set;
- for an element function (`let <Name … />`): its FunctionVisualizer icon when set and a `properties` list
  holding each of its properties once, in declaration order, each with its name and the `format` its
  type gives it when set;
- for an alias: its `format`.

An entry for a record, action, component or element function that has no visualizer but has a
property with a resolved format SHALL carry a `visualizer` value with the formats. When visualizers
are asked for, the declaration table SHALL also list each alias with a visualizer that types a
property of a listed declaration, so a reader can read the property's format without another query. A host visualizer
declaration module the workspace snapshot does not hold SHALL be reported in the answer as missing
and SHALL NOT fail the query. The `visualizer` values SHALL round-trip through JSON without loss.

#### Scenario: A question with visualizers
- **WHEN** a client asks with `visualizers` for the source tree of a document that constructs
  `<SingleChoice … />`, and the survey library declares a visualizer for `Question` with `title=label`
  and `id=id` and for `SingleChoice` with `icon="list"` and a property visualizer for `allowsOther`
- **THEN** the entry for `SingleChoice` SHALL carry a `visualizer` value with the title `label`, the id
  `id`, the id position `start` and the icon `list`
- **AND** its `properties` SHALL start with `allowsOther` and list every other property of
  `SingleChoice` once

#### Scenario: The alias a property uses comes along
- **WHEN** a client asks with `visualizers` and `Choice`'s `tint` property is typed by `Color`, whose
  visualizer has the format `color`, and no node of the document names `Color`
- **THEN** the declaration table SHALL hold `Color` with a `visualizer` value with the format `color`

#### Scenario: An element function's property takes its type's format
- **WHEN** a client asks with `visualizers` for the source tree of a document that declares
  `let <Glow tint:Color size:int /> = …` and constructs `<Glow tint="#EC4899" size=3 />`, and `Color`'s
  visualizer has the format `color`
- **THEN** the entry for `Glow` SHALL carry a `visualizer` value whose `tint` property has the format
  `color` and whose `size` property has none

#### Scenario: Without the option nothing changes
- **WHEN** a client asks for the source tree of the same document without `visualizers`
- **THEN** no declaration entry SHALL carry a `visualizer` value

#### Scenario: A host module overrides a kind
- **WHEN** a client asks with `visualizers` naming a host module that declares a visualizer for
  `SingleChoice` with `icon="list-todo"`
- **THEN** the entry for `SingleChoice` SHALL carry the icon `list-todo`

#### Scenario: A missing host module
- **WHEN** a client asks with `visualizers` naming a module identity the snapshot does not hold
- **THEN** the answer SHALL list that identity as missing and SHALL carry the program's own visualizers

#### Scenario: A result type is constructed inside an imported helper
- **WHEN** a host asks for visualizers with includeUnreferencedDeclarations=true
- **THEN** the declaration table SHALL also contain loaded visualizer targets not directly referenced
by the document, including derived kinds that only inherit a visualizer, with the same resolved inheritance/override metadata
- **AND** the default query SHALL retain its existing bounded declaration-table behavior
