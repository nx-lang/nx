## MODIFIED Requirements

### Requirement: Snippets compile and draw in the browser
Running an NX snippet SHALL compile the snippet together with the DrawnUI catalog through the NX
WebAssembly compiler in the visitor's tab, evaluate the program's `root` function with the NX IR
runtime, and draw the resulting value tree with DrawnUi.React on the engine's React canvas. No
server SHALL take part in compilation or drawing.

#### Scenario: A valid snippet draws
- **WHEN** the visitor runs a snippet whose `root` function returns DrawnUI controls
- **THEN** the canvas SHALL show those controls with the properties the snippet set, and
  `fiddle.getState()` SHALL report success with no errors

#### Scenario: Union cases and records reach DrawnUI in its own shapes
- **WHEN** a snippet sets a property to a union case such as `Center`, or to a record such as
  `<Thickness Left=4.0 />`
- **THEN** the control SHALL receive the bare case name, and the record SHALL arrive as the DrawnUI
  value type of that name, so that DrawnUI's own type checks accept it

#### Scenario: Unset properties keep DrawnUI's defaults
- **WHEN** a snippet leaves a property unset, or binds it to a value that evaluates to the empty
  value `{}` — an `if` with no `else` that took no branch, an optional prop that was not written
- **THEN** the control SHALL be created without that property, so the DrawnUI default applies
- **AND** the renderer SHALL NOT pass `null`, `undefined` or an empty array in its place

#### Scenario: A compile error is reported in the engine's form
- **WHEN** the snippet does not compile
- **THEN** the run SHALL fail, nothing new SHALL be drawn, and each error SHALL be listed under the
  editor as `L{line}: {message}` in the snippet's own line numbers, with the catalog's lines never
  blamed

#### Scenario: An unknown control is drawn as a placeholder
- **WHEN** an evaluated value carries a type the catalog does not declare and the snippet does not
  define
- **THEN** the rest of the tree SHALL still draw, and a visible placeholder naming the type SHALL
  stand in its place

#### Scenario: An authored component draws in its place
- **WHEN** the snippet declares its own component and uses it in `root`
- **THEN** what the component renders SHALL be drawn where the component was used

#### Scenario: A compiler crash costs one run
- **WHEN** the compiler traps during a run
- **THEN** that run SHALL fail with a readable message, and the next run SHALL compile normally
  without reloading the page

### Requirement: The fiddle's catalog declares templated controls
The catalog the fiddle generates SHALL declare, for each class that carries an item collection and
a cell factory, a type parameter `TItem`, the property `ItemsSource?: TItem+` and the property
`ItemTemplate?: <function Item:TItem Index:int />: DrawnNode` — an optional function-typed property
marked on the name, which needs no parentheses — on the component standing for the class that
declares them and inherited by every control below. Every other author-settable property SHALL
carry its optionality on the name the same way, `x?: T` for a scalar and `x?: T+` for a DrawnUI
array, never `?` or `*` in the type slot. The generated metadata SHALL record, for each renderable
control carrying `ItemTemplate`, the parameter names the template is called with, in order, so the
renderer can bind the item and its index. Neither property SHALL appear in the omitted list.

#### Scenario: A layout binds a collection and a template
- **WHEN** the catalog is regenerated from the pinned `drawnui-react` package
- **THEN** `SkiaLayout` SHALL declare `TItem:type`, `ItemsSource?:TItem+` and
  `ItemTemplate?:<function Item:TItem Index:int />: DrawnNode` on the class that declares them
- **AND** a snippet binding `TItem=Contact ItemsSource={contacts} ItemTemplate={ContactCell}
  RecyclingTemplate=Enabled` SHALL compile when `ContactCell` is
  `let <ContactCell Item:Contact Index:int />: DrawnNode = …` and `contacts` is a `Contact+`

#### Scenario: A template whose item type disagrees is rejected
- **WHEN** a snippet binds `TItem=Contact` and an `ItemTemplate` whose `Item` parameter is another
  record type
- **THEN** the run SHALL fail with an error in the snippet's own line numbers

#### Scenario: Children are one or more controls, optional on the name
- **WHEN** the catalog is regenerated
- **THEN** every container's content property SHALL be spelled `content Children?: DrawnNode+`
- **AND** a snippet that nests one control in a container SHALL compile and draw that child

#### Scenario: The template's parameter names are recorded
- **WHEN** the generated metadata is inspected
- **THEN** each renderable control carrying `ItemTemplate` SHALL record the template's parameter
  names in order, and the property that carries the collection
