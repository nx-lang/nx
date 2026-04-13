## ADDED Requirements

### Requirement: Only abstract components may act as base components
The system SHALL accept `extends BaseComponent` only when `BaseComponent` resolves through the
prepared visible element bindings of the analyzing module to an abstract component declaration.
That binding model SHALL include same-library peer declarations and exported imported-library
declarations visible from the caller's build context. Concrete components, missing names, and
non-component symbols MUST NOT be usable as base components.

#### Scenario: Concrete component cannot be extended
- **WHEN** a file contains `component <SearchBase placeholder:string /> = { <button /> } component <SearchBox extends SearchBase /> = { <button /> }`
- **THEN** analysis SHALL reject `SearchBox extends SearchBase` because `SearchBase` is not abstract

#### Scenario: Peer file abstract component can be extended
- **WHEN** `base.nx` in one library contains `abstract component <SearchBase placeholder:string emits { SearchRequested } />`
- **AND** `search.nx` in the same library contains `component <SearchBox extends SearchBase showSearchIcon:bool = true /> = { <button /> }`
- **THEN** analysis SHALL accept `SearchBox extends SearchBase`

#### Scenario: Imported abstract component can be extended
- **WHEN** a host loads `../ui` into a `LibraryRegistry`
- **AND** creates a `ProgramBuildContext` from that registry
- **AND** `../ui` exports `abstract component <SearchBase placeholder:string />`
- **AND** `app/main.nx` contains `import "../ui" as UI` and `component <SearchBox extends UI.SearchBase /> = { <button /> }`
- **THEN** analysis SHALL accept `SearchBox extends UI.SearchBase`

#### Scenario: Abstract external component can act as a base component
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string /> external component <SearchBox extends SearchBase showSearchIcon:bool = true />`
- **THEN** analysis SHALL accept `SearchBox extends SearchBase`

#### Scenario: Non-component base is rejected
- **WHEN** a file contains `abstract type SearchBase = { placeholder:string } component <SearchBox extends SearchBase /> = { <button /> }`
- **THEN** analysis SHALL reject `SearchBox extends SearchBase` because `SearchBase` does not resolve to an abstract component declaration

### Requirement: Derived components inherit props, defaults, content props, and body scope
The system SHALL treat a derived component as having the effective prop set of its entire abstract
base chain plus its own declared props. Inherited props SHALL participate in invocation binding,
default application, content binding, and component-body name resolution. Duplicate prop names and
more than one effective content prop across the base chain and derived declaration MUST be
rejected.

#### Scenario: Derived component accepts inherited and local props at call site
- **WHEN** a file contains `abstract component <SearchBase placeholder:string emits { SearchRequested } /> component <NxSearchUi extends SearchBase showSpinner:bool = false /> = { <button /> } let render() = <NxSearchUi placeholder="Recipe ingredient" showSpinner=true />`
- **THEN** analysis SHALL accept the invocation of `NxSearchUi` using both inherited prop `placeholder` and local prop `showSpinner`

#### Scenario: Base prop default applies to derived component initialization
- **WHEN** a module contains `abstract component <SearchBase placeholder:string = "Enter search query here" /> component <NxSearchUi extends SearchBase /> = { <TextInput value={placeholder} placeholder={placeholder} /> }` and `NxSearchUi` is initialized without an explicit `placeholder`
- **THEN** initialization SHALL bind `placeholder` as `"Enter search query here"`
- **AND** SHALL return a rendered `TextInput` element whose `value` and `placeholder` are both `"Enter search query here"`

#### Scenario: Derived component body can reference inherited props
- **WHEN** a file contains `abstract component <SearchBase placeholder:string /> component <NxSearchUi extends SearchBase /> = { <TextInput placeholder={placeholder} /> }`
- **THEN** lowering and analysis SHALL accept the reference to inherited prop `placeholder` inside the `NxSearchUi` body

#### Scenario: Inherited content prop participates in markup content binding
- **WHEN** a file contains `abstract component <PanelBase title:string content body:Element /> component <Panel extends PanelBase /> = { <section><h1>{title}</h1>{body}</section> } let root() = <Panel title="Docs"><Badge /></Panel>`
- **THEN** analysis SHALL bind the markup body content to inherited content prop `body`

#### Scenario: Duplicate inherited prop name is rejected
- **WHEN** a file contains `abstract component <SearchBase placeholder:string /> component <NxSearchUi extends SearchBase placeholder:string showSpinner:bool = false /> = { <button /> }`
- **THEN** analysis SHALL reject `NxSearchUi` because `placeholder` duplicates an inherited component prop

#### Scenario: Duplicate effective content prop is rejected
- **WHEN** a file contains `abstract component <PanelBase content body:Element /> component <Panel extends PanelBase content child:Element /> = { <section>{body}</section> }`
- **THEN** analysis SHALL reject `Panel` because it declares a content prop while already inheriting content prop `body`

### Requirement: Derived components inherit emitted actions and handler binding surface
The system SHALL include inherited emitted actions in the derived component's effective emits set.
Shared emitted actions SHALL keep their referenced action type. Inherited inline emitted actions
SHALL retain the public action type declared by the ancestor component that introduced them.
Duplicate emitted-action names across the base chain MUST be rejected, and generated handler prop
names SHALL be validated against the effective emits set.

#### Scenario: Derived component inherits a shared emitted action
- **WHEN** a file contains `action SearchSubmitted = { searchString:string } abstract component <SearchBase emits { SearchSubmitted } /> component <NxSearchUi extends SearchBase /> = { <button /> } let render() = <NxSearchUi onSearchSubmitted=<DoSearch search={action.searchString} /> />`
- **THEN** lowering and analysis SHALL accept `onSearchSubmitted` as a handler binding on `NxSearchUi`

#### Scenario: Derived component inherits an inline emitted action with ancestor public name
- **WHEN** a file contains `action TrackSearch = { value:string } abstract component <SearchBase emits { ValueChanged { value:string } } /> component <NxSearchUi extends SearchBase /> = { <button /> } let render() = <NxSearchUi onValueChanged=<TrackSearch value={action.value} /> />`
- **THEN** lowering SHALL bind `onValueChanged` as a handler for inherited emitted action `ValueChanged`
- **AND** SHALL bind `action` inside the handler body as a `SearchBase.ValueChanged` action value

#### Scenario: Duplicate inherited emitted action name is rejected
- **WHEN** a file contains `action SearchRequested = { query:string } abstract component <SearchBase emits { SearchRequested } /> component <NxSearchUi extends SearchBase emits { SearchRequested } /> = { <button /> }`
- **THEN** analysis SHALL reject `NxSearchUi` because `SearchRequested` duplicates an inherited emitted action

#### Scenario: Inherited emitted action handler name collision is rejected
- **WHEN** a file contains `action SearchSubmitted = { searchString:string } abstract component <SearchBase emits { SearchSubmitted } /> component <NxSearchUi extends SearchBase onSearchSubmitted:string /> = { <button /> }`
- **THEN** lowering SHALL reject `NxSearchUi` because prop `onSearchSubmitted` collides with the inherited emitted action handler name

### Requirement: Abstract components are contract-only and not instantiable
Abstract components SHALL be valid as reusable public contracts and inheritance bases, but they
MUST NOT be instantiated directly through markup or component lifecycle entry points.

#### Scenario: Markup instantiation of an abstract component is rejected
- **WHEN** a file contains `abstract component <SearchBase placeholder:string /> let render() = <SearchBase placeholder="docs" />`
- **THEN** type checking SHALL reject the construction of `SearchBase` because it is abstract

#### Scenario: Markup instantiation of an abstract external component is rejected
- **WHEN** a file contains `abstract external component <SearchBase placeholder:string /> let render() = <SearchBase placeholder="docs" />`
- **THEN** type checking SHALL reject the construction of `SearchBase` because it is abstract

#### Scenario: Lifecycle initialization of an abstract component is rejected
- **WHEN** a host initializes `SearchBase` from a `ProgramArtifact` containing `abstract component <SearchBase placeholder:string />`
- **THEN** the public NX component lifecycle bindings SHALL reject initialization of `SearchBase`
