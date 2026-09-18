## ADDED Requirements

### Requirement: Item templates draw virtualized cells
When a drawn control carries an `ItemTemplate` whose value is a `Function` record and an
`ItemsSource`, the site SHALL hand DrawnUI a cell factory for that control, so that DrawnUI
decides which cells exist, realizes them, recycles them and measures them by its own strategy.
Each cell SHALL be a host control that, whenever DrawnUI binds it to an item, calls the function
with `Item` bound to that item and `Index` to the item's index, and draws the resulting value as
its content by the same translation the output pane applies to the root. The items DrawnUI is
given SHALL be the evaluated items themselves, so that what a cell receives is what the author's
collection holds. A cell that is rebound to another item SHALL redraw with the new item. A
function call that fails SHALL be reported in the diagnostics pane once per failure and SHALL
leave that cell empty rather than abort the drawing. A handler bound inside a cell SHALL be
drawn inert and reported as such, since a cell's content is not an instance of the tree.

#### Scenario: A templated list draws only the cells DrawnUI asks for
- **WHEN** the evaluated tree contains a `SkiaLayout` with `ItemsSource` of ten thousand items,
  `ItemTemplate` bound to a function and `RecyclingTemplate=Enabled`
- **THEN** the site SHALL realize cells for the visible range and reuse them as the visitor scrolls
- **AND** SHALL NOT call the function once per item up front

#### Scenario: A cell is drawn from the function's result
- **WHEN** DrawnUI binds a cell to the item at index 3
- **THEN** the cell SHALL show what the function renders for `Item` = that item and `Index` = 3

#### Scenario: Cells of uneven height are measured by the strategy the author set
- **WHEN** a templated `SkiaLayout` sets `MeasureItemsStrategy=MeasureAll` and its template
  renders cells of differing heights
- **THEN** each cell SHALL be laid out at its own height

#### Scenario: A failing template reports and continues
- **WHEN** the function fails for one item
- **THEN** the diagnostics pane SHALL show the failure with the item's index
- **AND** the rest of the list SHALL still draw

#### Scenario: An ItemTemplate without ItemsSource draws nothing templated
- **WHEN** a control carries `ItemTemplate` and no `ItemsSource`
- **THEN** the site SHALL pass the factory and leave DrawnUI to draw the control's static content,
  as DrawnUI does

## MODIFIED Requirements

### Requirement: Missing capabilities are named from a shared vocabulary
An example that is not complete SHALL attribute its gap to one or more named capabilities drawn
from a fixed vocabulary shared across all examples, rather than to prose written per example, so
that gaps can be counted, compared, and found again when a capability lands. The vocabulary SHALL
distinguish a capability NX does not have from one NX has and the port does not use yet, so that
a landed capability is not presented as missing from NX. List virtualization SHALL NOT be in the
vocabulary of capabilities NX lacks: the two examples built on it, Cells and Uneven Cells, SHALL
be ported with the virtualization the originals demonstrate and SHALL NOT be reduced.

#### Scenario: A gap names a capability
- **WHEN** an example declares itself static or reduced
- **THEN** it SHALL name at least one capability from the shared vocabulary as the reason

#### Scenario: Coverage notes derive from the named capabilities
- **WHEN** an example's coverage is shown to a visitor
- **THEN** the wording SHALL derive from the capabilities it names
- **AND** two examples blocked by the same capability SHALL describe it the same way

#### Scenario: A landed capability reads as a porting gap
- **WHEN** an example names event handlers or component state as its gap
- **THEN** the wording SHALL say the port does not use them yet
- **AND** it SHALL NOT say NX lacks them

#### Scenario: Gaps can be surveyed across the example set
- **WHEN** the example set is inspected
- **THEN** it SHALL be possible to determine which examples are blocked by any given capability

#### Scenario: The list examples are virtualized
- **WHEN** the Cells or Uneven Cells example is opened
- **THEN** its source SHALL bind the original's item count through `ItemsSource` and an element
  function through `ItemTemplate`, with the original's recycling and measuring settings
- **AND** it SHALL NOT declare itself reduced
- **AND** its source SHALL NOT say that NX cannot express a collection or a template
