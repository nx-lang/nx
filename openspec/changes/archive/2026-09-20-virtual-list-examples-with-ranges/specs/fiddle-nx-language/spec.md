## ADDED Requirements

### Requirement: The fiddle's catalog declares templated controls
The catalog the fiddle generates SHALL declare, for each class that carries an item collection and
a cell factory, a type parameter `TItem`, the property `ItemsSource: TItem[]?` and the property
`ItemTemplate: (<function Item:TItem Index:int />: DrawnNode)?`, on the component standing for the
class that declares them and inherited by every control below. The generated metadata SHALL record,
for each renderable control carrying `ItemTemplate`, the parameter names the template is called
with, in order, so the renderer can bind the item and its index. Neither property SHALL appear in
the omitted list.

#### Scenario: A layout binds a collection and a template
- **WHEN** the catalog is regenerated from the pinned `drawnui-react` package
- **THEN** `SkiaLayout` SHALL declare `TItem:type`, `ItemsSource:TItem[]?` and
  `ItemTemplate:(<function Item:TItem Index:int />: DrawnNode)?` on the class that declares them
- **AND** a snippet binding `TItem=Contact ItemsSource={contacts} ItemTemplate={ContactCell}
  RecyclingTemplate=Enabled` SHALL compile when `ContactCell` is
  `let <ContactCell Item:Contact Index:int />: DrawnNode = …`

#### Scenario: A template whose item type disagrees is rejected
- **WHEN** a snippet binds `TItem=Contact` and an `ItemTemplate` whose `Item` parameter is another
  record type
- **THEN** the run SHALL fail with an error in the snippet's own line numbers

#### Scenario: The template's parameter names are recorded
- **WHEN** the generated metadata is inspected
- **THEN** each renderable control carrying `ItemTemplate` SHALL record the template's parameter
  names in order, and the property that carries the collection

### Requirement: A templated control draws its cells
A control bound to an item collection and a template SHALL draw one cell per item, the template
being called with the item and its index at the moment DrawnUI binds a cell, so that only the cells
DrawnUI realizes are ever built and recycling and measuring behave as they do for a control the
engine's own templates drive. A template that fails for one item SHALL leave the rest of the
drawing standing and SHALL be reported in the host's console naming the template and the index. A
templated control drawn inside a cell SHALL template its own cells the same way.

#### Scenario: Only realized cells are built
- **WHEN** a snippet binds a collection far larger than the visible area through `ItemsSource` with
  an `ItemTemplate`
- **THEN** the drawing SHALL show the cells for the visible items
- **AND** the template SHALL be called once per cell DrawnUI binds, not once per item in the
  collection

#### Scenario: A cell is bound to its item and index
- **WHEN** DrawnUI binds a cell to an item
- **THEN** the template SHALL be called with that item and its index, and the cell's content SHALL
  be what the template returned

#### Scenario: A failing template is reported, not fatal
- **WHEN** a template call fails for one item
- **THEN** the rest of the drawing SHALL stand
- **AND** the console SHALL carry one line naming the template and the index that failed

#### Scenario: A nested templated control still templates
- **WHEN** a cell's own content binds `ItemsSource` and `ItemTemplate`
- **THEN** that inner control SHALL draw its cells through its template rather than losing it

### Requirement: A preset draws a virtualized list
The NX presets SHALL include at least one that binds a collection of the size the DrawnUI original
demonstrates through `ItemsSource`, with a cell as an element function through `ItemTemplate` and
the original's recycling and measuring settings. Its collection SHALL be built from a range rather
than from a written-out list or a helper that concatenates shifted copies of one, and neither its
source nor its description SHALL say NX cannot express a collection, a template or a count.

#### Scenario: The list preset draws and scrolls
- **WHEN** the virtualized-list preset is drawn
- **THEN** it SHALL show the cells for the visible items, and scrolling SHALL show later ones

#### Scenario: The collection is one loop over a range
- **WHEN** the preset's source is read
- **THEN** its collection SHALL be one `for` over a range whose bound is the item count the
  original demonstrates

#### Scenario: The list preset fits the share budget
- **WHEN** the runtime's tests compile the presets
- **THEN** the virtualized-list preset's share module SHALL be under 128 KB, as every preset's is

## MODIFIED Requirements

### Requirement: The fiddle's catalog declares DrawnUI events as emits
The catalog the fiddle generates SHALL declare every DrawnUI event, an optional function-typed member
whose return type is `void` or includes it, as an emit of the component that stands for the class
declaring it, inherited by every control below. A callback parameter with a primitive NX type SHALL
become a payload field under its own name; a parameter typed as an engine object SHALL be dropped
and recorded with the catalog's other divergences. The generated metadata SHALL record each event's
parameter names in order, so the renderer can build an action from a callback's arguments.
Function-typed members that are neither events nor a templated control's cell factory SHALL stay
omitted.

#### Scenario: A tap is bindable on every control
- **WHEN** a snippet binds `onTapped` on a `SkiaShape` inside a component
- **THEN** it SHALL compile, because `SkiaControl` declares `Tapped` and `SkiaShape` inherits it

#### Scenario: A payload carries the callback's primitive arguments
- **WHEN** a snippet binds `onToggled=<Update on={action.value} />` on a `SkiaSwitch`
- **THEN** it SHALL compile, with `action.value` typed `boolean`

#### Scenario: Events leave the omitted list
- **WHEN** the catalog is regenerated
- **THEN** the omitted list SHALL contain no event callback and no templated control's
  `ItemsSource` or `ItemTemplate`
- **AND** it SHALL still list function-typed members that are neither, such as `ProcessJson`, with
  their TypeScript types

#### Scenario: An unknown event is a compile error in the snippet's lines
- **WHEN** a snippet binds `onNoSuchEvent` on a control
- **THEN** the run SHALL fail with an error listed as `L{line}: {message}` in the snippet's own
  line numbers
