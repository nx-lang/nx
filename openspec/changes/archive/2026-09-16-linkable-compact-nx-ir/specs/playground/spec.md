## MODIFIED Requirements

### Requirement: Catalog is available to authored source without being shown
The visitor SHALL be able to use catalog controls without declaring or importing them, and the
catalog SHALL NOT appear in the source pane.

The catalog SHALL be a module of its own that every compile and every language query imports
implicitly, so that the visitor's document is analyzed exactly as written. Any source the language
accepts as a whole file SHALL compile in the playground, and SHALL do so with the diagnostics and
positions the visitor would see compiling that same text on its own.

A compile SHALL answer with the visitor's module alone: an NX IR artifact that names the catalog in
its module table and carries none of the catalog's declarations and no debug section. The catalog's
own artifact SHALL be emitted once, at build time, through the same compiler module the browser
loads, and the site SHALL prepare it once per page and link every compile against that preparation.
The catalog source SHALL remain the one committed form; the artifact SHALL NOT be committed.

#### Scenario: Controls are used without an import
- **WHEN** a visitor writes an element naming a catalog control
- **THEN** compilation SHALL resolve it with no import statement present

#### Scenario: Catalog is not editable
- **WHEN** a visitor inspects the source pane
- **THEN** the catalog declarations SHALL NOT be present in it

#### Scenario: A file that is a single trailing element compiles
- **WHEN** a visitor's source declares no `root` and consists of one element expression, such as
  `<SkiaLayer VerticalOptions=Fill></SkiaLayer>`
- **THEN** compilation SHALL accept it and return IR
- **AND** it SHALL NOT report a syntax error caused by the catalog

#### Scenario: A compile carries the visitor's module alone
- **WHEN** a visitor's source compiles
- **THEN** the result SHALL be one NX IR artifact whose module table names the visitor's module
  first and the catalog second
- **AND** it SHALL contain no declaration of the catalog and no debug section

#### Scenario: The catalog artifact is built once and linked every time
- **WHEN** the site is built
- **THEN** the bundle SHALL carry the catalog's NX IR artifact, emitted from the catalog source by the
  same compiler module the worker loads
- **AND** a visitor's compiles SHALL be linked against one preparation of that artifact
- **AND** the example check SHALL link each example against the catalog artifact emitted the same way

#### Scenario: A catalog that lacks a control the visitor names is an application fault
- **WHEN** the bundled catalog artifact does not declare a control a compiled snippet references
- **THEN** linking SHALL fail naming the control
- **AND** the site SHALL report the failure rather than draw a partial tree
