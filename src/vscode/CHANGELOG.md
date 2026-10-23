# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Editing in Visual (unstable)
- New language query, `sourceEdit`: the text edits that set a literal or a bare union case that is
  a whole value to a typed value (a string, a number, a boolean or a case), set a one-line run of
  text, or remove an attribute, by the node's key in the source tree. Quoting, escaping and braces
  are the service's; an edit it cannot make is refused with a reason (`unknownNode`, `notEditable`,
  `invalidValue`), not an error. The language service, the Node and wasm SDKs,
  `@nx-lang/language-protocol`, `-core`, `-http` and `-client` answer it.
- `<nx-viewer>`'s new `editable` property edits values in place, on a click, with the caret where it
  falls: a field for strings, ids and one-line text, a number field that steps with the arrows and
  with the property's name dragged sideways, toggles for booleans, a popover of a union's cases with
  their docs, and a hex field and color picker for a color. Delete resets a property with a default or clears an optional one. Each
  edit fires `nx-edit` with the node's key and a `sourceEdit` edit for the host to make in its
  source; a value the reading cannot edit, such as an expression, fires `nx-open`.
- The playground's Visual edits this way, through the editor, so undo and redo work in both views,
  and the NX title bar has Undo and Redo buttons.
- `sourceEdit` also adds a property an element does not write (`addProperty`, aimed at the element,
  written in the element's layout).
- `@nx/visualizer` has editor hints: `editor` (`field`, `dropdown`, `segmented`), `min`, `max`,
  `step` and `unit`, and `ValueChoice` content (`choices`, each a value and an optional label), on
  `PropertyVisualizer` and `ValueVisualizer`, and `multiline` and `placeholder` on
  `PropertyVisualizer`; `ValueVisualizer.format` is optional. A hint or a choice that does not fit its
  property's type warns `editor-hint-mismatch`, and a `min` above its `max` warns `editor-hint-range`.
  A visualizer declaration's value may hold number literals. With `visualizers`, the source tree
  carries each property's resolved hints, its choices typed as `setValue` values, and lists the types
  of the properties an element does not write.
- `<nx-viewer>`: a selected element lists the properties it does not write under `Show N unset properties`, and sets
  one where it reads; a written row has a menu (Reset to default or Remove, and Show in Source);
  numbers edit with ranges, units and steppers (click, Shift+click, or drag up and down), a number's
  or a string's choices list under its field with the default's row first, unions edit as segmented
  buttons and strings over several lines as their hints say; Shift+Enter breaks a string's line.
- `@nx-lang/sdk-wasm`'s module ABI version is now 10; the loader refuses a module of another version.

### Fixes
- A record written with a property-list `if` or match, as `<Theme accent="c" if flag { muted="d" } />`,
  keeps the properties its active branch sets in the interpreter. It used to drop them when the record
  was declared in the same module. Executable codegen (NX IR, TypeScript, JavaScript), and so the
  playground's preview, now refuses such a record with `codegen-unsupported-construct`, as it does a
  component written that way, instead of emitting it without them.
- A type error in an element with property-list conditions is reported once, not once per path through
  its branches.

## 0.8.0

### Component state and props
- **Breaking.** A component with state has a derived state record, `T.State`, with its own
  `T.State.Update` and `T.State.Property`. Inside a component body a bare `State` names it, so a
  handler patches state with `<State.Update count={count + 1} />` where it wrote
  `<Update count={count + 1} />`; the old spelling is an unknown element whose diagnostic gives the
  new one. A component is not a record and no longer has a `T.Update` or `T.Property` of its own.
- Every component has a derived props record, `T.Props`, with its update record and property
  union, and a generic record's field can be typed by its type parameter's property union.
- `State` and `Props`, like `Update` and `Property`, are reserved as emitted-action names, and a
  prop or state field may not be named `State`, since a bare `State` in the body names the state
  record. A concrete external component with no body has no body to shadow, so it may declare a
  prop named `State` for a host control's property of that name.
- A library module that imports a component selectively (`import { Card } from "../ui"`) can name
  `Card.State`, `Card.Props` and their companions, as a program could.
- **Breaking**, for a host that reads NX IR images: NX IR is schema 6. A component's layout
  carries a reference to its state record, and both IR runtimes match state patches against it. An
  image written by NX 0.7.0 or earlier (schema 5) is refused by this release's runtimes with a
  diagnostic naming both schemas; keep the earlier runtime to play such images.
- typegen emits `<Name>_state`, `<Name>_state_update` and `<Name>_state_property` for a component
  with state, and no `<Name>_update` or `<Name>_property` for a component.

### Visualizer declarations and `@nx/visualizer`
- New. A `visualizer T = <ElementVisualizer ... />` declaration says how a reader shows `T`: which
  property is its `title`, which is its `id` and whether that reads at the `start` or the `end` of
  its line, and the Lucide `icon` that marks it. `<PropertyVisualizer property=... format=... />`
  inside it formats one property's string values, and `visualizer Color = <ValueVisualizer
  format=color />` formats every value of an alias of `string`. The formats are `id` (a quiet
  identifier) and `color` (a swatch).
- The declarations come from `import "@nx/visualizer"`, a standard library. They are type-checked,
  inherited from a base component or record, and kept out of NX IR: compiling, evaluating and
  sharing a program is unchanged by them.
- A host can name visualizer modules of its own in a source-tree request,
  `visualizers: { modules: [...] }`, applied after the program's own: a catalog's presentation can
  live beside it without any program importing it.

### `@nx-lang/viewer` (new)
- `<nx-viewer>`, a custom element with no framework that reads an NX document from its source tree:
  an element whose properties fit on one line in the pane reads as its name, title and properties,
  one whose properties do not, or that holds elements, as a group under its name with a row per
  property (measured again as the pane resizes, whatever the source's line breaks), and the rest as
  written, in the editor's colors (`colors`, from `@nx-lang/value-view`'s `colorize`). Visualizers give elements
  their titles, ids and icons (`icons`, a function the host gives Lucide or its own set), a color a
  swatch, and a `uitext` value reads as text shown to people.
- Groups and declarations fold (`folded`, `collapseAll()`, `expandAll()`, the `nx-fold` event), and
  a click selects a node (`selection`, the `nx-select` event). `keyAtRange` and `rangeOfKey`
  carry a selection between the reading and a host's editor, in the editor's UTF-16 offsets;
  `keyAtTop()`, `topOf()` and `scrollToKey()` carry the scroll position; and `nodeAtSpan` finds the
  node a value origin names. `stale` mutes the reading while a newer tree
  is computed.
- Resting the pointer on a name or a value opens a hover card, in the editor's hover's style, from the
  source tree alone: an element's declaration, a property's type, default and doc, a union case, or
  a reference's type. With the reading focused, `i` shows the selected unit's card. A doc's Markdown
  is rendered as text and elements only, links only to the web, or by the host's `markdown`.
- The playground's NX pane opens in this view, Visual, beside Source.

### `@nx-lang/previewer` (new, unstable)
- Runs one NX component as a headless preview session on `@nx-lang/ir-runtime`: the host draws the
  current output, dispatches actions, steps back through the timeline and keeps several runs. It
  has no UI and no framework. The playground's output pane uses it for Preview.

### Value origins and values
- Both IR runtimes report where the records of a value were constructed, when a call passes an
  `origins` object: each record's path in the value and the source range of the element or range
  expression that built it. A host can select the source of a value it shows.
- The wasm SDK spells a runtime value as annotated NX text (`formatValue`), with every node
  carrying its JSON pointer. `<nx-value>` takes an `actions` function and shows
  each action as a button on its node's first line, firing `nx-value-action`.

### Source tree (unstable)
- New language query, `sourceTree`: every piece of one document as a typed node, with the
  declarations the nodes refer to listed beside them. Each node has a role (`element`,
  `attribute`, `literal`, `case`, `operator`, `condition`, `comment`, `unparsed` and the rest), a
  range, its parent, a key that an edit elsewhere in the document does not move, and the type and
  declaration hover reports inside it; comments and regions that do not parse are nodes too, so
  every token of the source belongs to one. The language service, the Node and wasm SDKs,
  `@nx-lang/language-protocol`, `-core`, `-http` and `-client` answer it. It is
  unstable: its shape may change in any release until a later one commits to it.
- `@nx-lang/sdk-wasm`'s module ABI version is now 8; the loader refuses a module of another version.

## 0.7.0

### NX IR runtimes
- The Rust runtime is on crates.io: `nx-ir-runtime`, with `nx-ir` (the image format) and
  `nx-value` (the host value) that it needs, all three at the release's version. A Rust host adds
  `nx-ir-runtime` and runs compiled NX IR images with no checkout of this repository and no
  compiler. The crates build for `wasm32-unknown-emscripten` and `wasm32-wasip1` as well as for
  native targets. `crates/nx-ir-runtime/README.md` has the API, the limits and the diagnostic
  codes.
- New in `nx-ir-runtime`: `InstanceTree`, which keeps one instance for each use of an authored
  component in a program's output as that output changes. The lifecycle renders one instance at a
  time and left composing them to the host. The tree gives each use a node at a key the host
  chooses, passes props down and keeps state with the node, runs a handler against the node whose
  body bound it, carries an emitted action to the handler the parent bound, and makes a dispatch
  all or nothing. `is_settled` says where nothing has rendered since the host last walked, so a
  pass visits only what changed. It knows nothing of drawing: a host that keeps any structure
  derived from the output uses it the same way. `@nx-lang/ir-runtime` has no instance tree.
- A host sets the native stack a call may use: `RuntimeOptions::max_stack_bytes`, one mebibyte by
  default, which was fixed before. A WebAssembly module's whole stack is often that much or less,
  so a host there states what it has free and gets `nx-ir-resource-limit` naming `maxStackBytes`
  where a deep evaluation would have ended the page. The README (*Small stacks*) has the measured
  need and how to read the free stack under Emscripten.
- A tree's depth is bounded: `RuntimeOptions::max_component_depth`, 100 by default, is how many
  component instances an `InstanceTree` may nest. A component that renders itself now ends with
  `nx-ir-resource-limit` naming `maxComponentDepth`, where it grew until the host ran out of
  stack.
- **Breaking**, for a Rust host that writes `RuntimeOptions` out in full: the struct has two new
  fields. Write `..RuntimeOptions::default()` for the ones you do not set.
- `@nx-lang/ir-runtime` is unchanged in behavior. Its README corrects how arguments by name are
  read (one by one, as positional arguments are) and gives measured costs for reading a host
  value, and the repository gains a performance harness that compares each release with the last
  (`pnpm --filter @nx-lang/ir-runtime bench`).

## 0.6.0

### Doc comments
- New. A `///` line comment documents the declaration or member it is attached to: written on the
  lines above an item, or after an item that starts and ends on its line. Every declaration and
  member can be documented: types, records, actions, unions and their cases and payload fields,
  values, functions and their parameters, components and their properties, `emits` entries and
  `state` fields. A doc comment that documents nothing, a misaligned continuation and an item
  documented twice are errors.
- Documentation is CommonMark; its first paragraph is the summary. `[Name]` and `[Type.member]` are
  doc links, resolved like any name in the module, with an `unresolved-doc-link` warning when one
  names nothing.
- Hover and completion items show documentation, hover on a doc link reports what it names, and
  typing a link suggests the names it can resolve to. Doc comments and their Markdown are
  highlighted.
- `nxlang typegen` writes documentation into the generated C# (XML documentation comments) and
  TypeScript (`/** */` comments).
- `@nx-lang/sdk-wasm`: `NxProgramArtifact.diagnostics()` returns the warnings of a program that
  builds, such as an unresolved doc link.

### Declaration schemas
- New in `@nx-lang/sdk-wasm` and `@nx-lang/sdk-node`: `NxProgramArtifact.functionSchema` answers
  with JSON Schema (draft 2020-12) for a function's arguments and result, with its `///`
  documentation as descriptions and an entry per parameter, and `typeSchema` answers for a declared
  type, written for input or output. Any function of the program can be described, named by its
  module and name as a `Function` record names it, so a host can describe a function to a language
  model or an MCP client as a tool. A schema describes the canonical JSON encoding, and agrees with
  what the runtime accepts and returns.
- A type with no JSON form, such as a function type or an abstract record nothing extends, is
  reported as `schema-inexpressible-type` in the answer rather than approximated. A host can name
  types it supplies itself, such as `@nx/agent`'s `ToolContext`, to leave their parameters out.
  See [declaration schemas](https://nxlang.org/reference/concepts/declaration-schemas).
- A parameter declared with a host-supplied type, with a record that extends one or with a type
  alias of either is left out, and its entry says so as `hostSupplied`. A parameter whose type only
  holds one, under `+` or as a field of a record at any depth, stays in the input schema, and its
  entry lists the types it holds as `hostSuppliedWithin`, so a host can refuse a function that
  would ask a caller for values the host means to supply. An entry's `typeRef` names the record or
  union the parameter is declared with, and through a type alias what the alias denotes.
- `@nx-lang/sdk-wasm`'s module ABI version is now 6; the loader refuses a module of another version.
- The entry module's NX IR image now also lists every module that declares a subtype of an abstract
  record a function takes or returns, so a runtime linked from the entry accepts a host value naming
  any of them, including one declared in a module nothing else uses. Before, such a value was
  refused with `nx-ir-boundary-type`.

### Language
- A function type may leave its parameters unspecified with `...`: `<function ... />: R` is the
  type of a function of any parameters whose result satisfies `R`, and `<function ... />: object*`
  takes any function. `...` is highlighted in that position, hover shows the type as written, and
  calling a value of such a type reports `function-reference-not-callable`.

### Standard libraries
- NX now has libraries of its own: source the compiler carries, imported by a reserved name with
  nothing to install or load. `import "@nx/agent"` works in a single file, a workspace, a library
  and the editor, in every import form. A path under `@nx/` that names no standard library is
  reported as `unknown-standard-library`, listing the ones that exist.
- **Breaking:** the whole `@nx/` root is now reserved, where only `@nx/prelude.nx` was before. A
  workspace module whose identity lies under `@nx/` is refused, a host library whose root lies
  under it is refused with `library-root-reserved`, and an import path beginning `@nx/` is no
  longer resolved relative to the importing file. A directory named `@nx` beside a module has to
  be renamed.
- Hover on a standard library declaration, or on one of its fields, shows its signature and
  documentation under a `(standard library @nx/<name>)` label, and completions offer its names only
  in a file that imports it.
- Completions inside the opening tag of a record written as an element now offer its fields, as
  they already did for a component's properties.

### `@nx/agent` (unstable)
- New. Thirteen product-neutral types for declaring an AI agent, its reference documents and its
  tools: `Agent`, `Document`, `AgentLimits`, `Tool`, `FunctionTool`, `WebSearchTool`, `ToolContext`,
  `Connection`, `HttpConnection`, `HttpMethod`, `HttpParam`, `HttpArguments` and `HttpTool`. The
  library holds types only; a host decides how an agent runs. It is unstable: its declarations may
  change incompatibly in any release, including a patch release. See
  [the agent library](https://nxlang.org/reference/libraries/agent).

### `@nx-lang/agent` (unstable)
- New npm package for JavaScript hosts of `@nx/agent`. `normalizeAgent` turns an evaluated `Agent`
  into a definition a host stores: plain JSON, with each tool's name, description and JSON Schemas
  in the MCP tool shape. `createAgentTools` makes the stored tools executable over a linked NX IR
  program, with no compiler: a function tool runs under an operation budget and two input limits,
  and an HTTP tool's request is built so that nothing a model sends can move it off its
  connection, and handed to the host to send. `@nx-lang/agent/ai-sdk` maps the tools to a Vercel
  AI SDK tool set. The package calls no model and makes no request itself. It is unstable, as the
  library is: its API and the stored definition format may change in any release. See
  [running an agent in a host](https://nxlang.org/reference/libraries/agent-hosts).
- A failed call says whose mistake it was. `@nx-lang/agent` reports `invalid-context` for a context
  record that does not fit the function's context type, as it does for one that was not passed,
  and `invalid-input` only for a failure in an argument the model sent, which is the one failure
  the model is told about in full. A boundary failure that is in no argument, such as a record
  field default whose value does not fit its field, is `evaluation-failed`.
- **Breaking**: the tool context record is read as the IR runtime reads any host value. An
  instance of a class below the top level of the record, which was passed as it was, is refused
  with `invalid-context`, as a `Date` or a typed array anywhere in it is. The record itself can
  still be an instance of a class, and a member that is `undefined` is still left out.

### NX IR runtimes
- New: `nx-ir-runtime`, a Rust crate that runs compiled NX IR images in a Rust host with no
  compiler: prepare, link, evaluate, the component lifecycle and the update helpers, with values
  exchanged as `NxValue`. It reports the `nx-ir-*` codes `@nx-lang/ir-runtime` does, and the
  conformance corpus holds the two runtimes to the same results. The crate is in the repository;
  this release does not publish it to a registry.
- An operation budget for code a host did not write: `maxOperations` in `@nx-lang/ir-runtime` and
  `RuntimeOptions::max_operations` in `nx-ir-runtime`, unset and unlimited by default. An operation
  is a unit of work on a value, such as a node evaluated, an item placed in a list, a value checked
  against a type or written for the host, so the budget bounds work and allocation, which
  `maxCallDepth` and `maxRangeLength` do not. One budget covers one call and everything it does,
  and a call that would exceed it fails with `nx-ir-resource-limit` before the work is done. Both
  runtimes count the same number for the same call. `docs/nx-ir-format.md` (*Evaluation cost*) has
  the rules, with worked counts.
- An input limit: `maxInputSize` and `RuntimeOptions::max_input_size`, unset by default, bound what
  a host hands one call: its arguments, props, content, state, batch and patch. A call given more
  fails with `nx-ir-resource-limit` before the program is looked at. The measure is exported, as
  `measureInputSize` and as `input_size` and `record_input_size`, and both runtimes measure the
  same size for the same input.
- A call reports what it used. Give it a `usage` object (`RuntimeOptions::usage` in Rust) and the
  runtime writes the operations the call cost and its input size, whether it returns or fails, so
  a host can choose a budget by measuring the programs it means to allow.
- Every `nx-ir-resource-limit` diagnostic carries `limit`, which names the limit it met and gives
  the limit's value where it has one. Expressions may nest at most 1,000 deep in one evaluation,
  and `@nx-lang/ir-runtime` reports a `RangeError` the JavaScript engine raises during evaluation
  as `nx-ir-resource-limit` whose limit is named `engine`.
- **Breaking**: `@nx-lang/ir-runtime` refuses an integer literal outside JavaScript's safe range
  where a program reaches it, with `nx-ir-number`. It used to carry the digits without being able
  to compute with them. The Rust runtime, which has 64-bit integers, runs the same program.
- A diagnostic names the argument a failure is in. When `callFunction` or `evaluateFunction`
  refuses a value the host passed for a parameter, or a required parameter was given nothing, the
  diagnostic carries `argument`, the parameter's name (`NxIrDiagnostic.argument` in
  `@nx-lang/ir-runtime`, `Diagnostic::argument` in the `nx-ir-runtime` crate), so a host tells which
  argument to correct without reading the message. A failure a default raises, a resource limit
  and a failure inside the function name none. Both runtimes name the same argument for the same
  call, and the conformance corpus holds them to it with cases that fail.
- What a host value is, is now one rule for every runtime: a canonical value, held in the host's
  own data, with JSON as an encoding of it and not something a host has to produce. A value a
  host computed is passed as one it read from JSON is, with nothing encoded on the way.
  `docs/nx-ir-format.md` (*Host values*) has the forms: plain objects and arrays in JavaScript,
  `NxValue` in Rust. The Rust runtime is unchanged.
- `@nx-lang/ir-runtime` reads its input by that rule, once in a call, after the input limit and
  before anything is checked. A member of a plain object set to `undefined` is now a member left
  out, at any depth, where the call failed: a default applies, an optional field is empty, and
  `maxInputSize` and `measureInputSize` do not count it. The evaluation functions take their input
  as the new types `NxHostValue` and `NxHostRecord`, which admit such a member, and still return
  `NxCanonicalValue`.
- **Breaking**, for a host that passes anything but plain data to `@nx-lang/ir-runtime`: an
  instance of a class, a `Date`, a `Map`, a `Set`, a typed array, a function, a symbol, a big
  integer, an object that holds itself and an `undefined` item of an array are refused where they
  are passed, with `nx-ir-boundary-type` and the path, inside a value typed `object` too, where
  they were let through. Nothing is converted: pass the text or the number a `Date` means, and
  spread an instance of a class. Positional arguments, content and a batch have to be arrays, and
  props, a state, a patch and arguments by name plain objects. A refusal inside an argument of
  `callFunction` or `evaluateFunction` names the argument. An `undefined` item of a list at a
  typed site, which was already refused, keeps its code and has the new message. Values the
  runtime returned are accepted back as they were, and a value read from JSON or built as object
  literals and arrays is unaffected.

## 0.5.0
The extension now ships with the NX packages, from one tag and at one version, so extension 0.5.0
understands the same NX as `@nx-lang/sdk-wasm` and `NxLang.Sdk` 0.5.0. This is why the version
jumps from 0.1.0.

### Diagnostics
- A type name that resolves to no visible type is now an error, `unresolved-type`, reported where
  the name is written with a "did you mean" suggestion: `people: Contatc+` says `Contatc` is not a
  visible type and suggests `Contact`. Files that validated before can show new errors; fix each by
  correcting the name, declaring the type or importing it.
- A union imported from a library no longer reports its defaulted case fields as missing when they
  are left out.

### Hover
- Hover now works on the first character of a name written directly after `<`, as in `<Button`.

## 0.1.0
The first release on the Visual Studio Marketplace and Open VSX.
- TextMate grammar for `.nx` files and for `nx` code blocks in Markdown, and language configuration
- The Rust `nx-lsp` language server, packaged for Windows x64, macOS Apple silicon and Linux x64:
  diagnostics, document symbols, hover and completions
- Snippets

### Syntax highlighting
The grammar follows the occurrence-cardinality change to the language:
- A type carries at most one occurrence suffix, and `?`, `+` and `*` after a type are each scoped
  `keyword.operator.type-modifier.nx`; `[]` is no longer a type suffix and carries no scope
- The `?` between a property name and its colon (`subtitle?: string`) is the optional mark, a token
  of its own scoped `keyword.operator.optional.nx`
- The presence operators are scoped as operators: `x?` as `keyword.operator.presence.nx`, `?.` as
  one token `keyword.operator.optional-access.nx`, and `??` as one token
  `keyword.operator.coalesce.nx`
- The language has no conditional operator and no `null`, so `keyword.operator.conditional.nx`,
  `punctuation.separator.conditional.nx` and `constant.language.null.nx` are gone; `null` is an
  ordinary name
- Hover annotation lines such as `(property) ShapeCommon.shadows?: Shadow+` scope the optional
  mark and the suffix as a declaration does
- Add extension activation code, packaged server path resolution, and `nx.server.path`
- Add package verification for compiled client runtime and native server assets

### Hover
Hover content is now a fenced `nx` code block, spelled the way an author writes the declaration, so
the markdown-fence grammar this extension already ships highlights it. A function reads
`let add(count:int): int` rather than `function add(count: int): int`; a union lists its cases and a
record type lists its fields.

Hover also answers where it used to say nothing:
- Inside an intrinsic element. A type error or a hovered name under `<div>` was invisible because
  type analysis never visited an element whose tag resolves to nothing. It does now, so the checker
  reports those errors and hover reports those types.
- On the member of a member access — `name` in `user.name` reports `(property) User.name: string`.
- On a declaration written inside another: a function or element-style function parameter, a record
  field, a union case, and a union case's payload field.
- On a bare name written in a property value, which reports the same thing that name reports
  anywhere else the type is expected.
- On a value declared without a type annotation, which reports the type analysis inferred.

Some hover fragments are deliberately not NX — a parameter, a field, and a union case have no
standalone spelling in the language, so hover writes them with a parenthesized kind, as
`(property) ShapeCommon.shadows: Shadow[]?`. The grammar scopes that shape now; before, such a line
fell outside every declaration context and its names went unhighlighted, while its annotation colon
and `?` were scoped as a ternary's.

New scopes:
- `meta.annotation.hover.nx` — one line of hover content in that shape
- `meta.annotation.hover.kind.nx` — the parenthesized kind. Left unstyled by design: it is an
  editor convention rather than NX, and reads best in the default foreground, as TypeScript's does

Inside such a line the owner, the name, the annotation colon, the type, and the type suffixes take
the same scopes they take in a declaration (`entity.name.type.nx`, `variable.other.property.nx`,
`punctuation.separator.type.annotation.nx`, `support.type.primitive.nx`,
`keyword.operator.type-modifier.nx`), and `(case) Role.admin` takes the union scopes source uses.

### Grammar: element-shaped declarations
Component and `let` function declarations written in element form now tokenize as declarations
rather than as element references. Themes and Monaco/Shiki consumers of `@nx-lang/language/grammar`
will see the following scopes change.

New scopes:
- `storage.modifier.external.nx` — the `external` modifier, previously unscoped
- `storage.modifier.content.nx` — the `content` property modifier, previously unscoped
- `keyword.declaration.emits.nx` and `keyword.declaration.state.nx` — previously unscoped, or
  scoped `entity.name.qualifier.nx`
- `meta.declaration.signature.nx`, `meta.declaration.emits.nx`, `meta.declaration.state.nx` —
  container scopes for a declaration signature and its groups
- `meta.definition.function.nx` — an element-shaped `let` function definition

Corrected scopes inside a declaration signature:
- The declared name is `entity.name.type.nx`, not `entity.name.tag.nx`
- A property name is `variable.other.property.nx` and its colon
  `punctuation.separator.type.annotation.nx`, rather than both being folded into
  `support.type.text.nx`
- A property type is scoped over its complete name; a greedy-quantifier backtrack previously split
  the last character off (`TextFormat` as `TextForma` + `t`) and left the default value unscoped
- The `?` and `[]` type suffixes are `keyword.operator.type-modifier.nx`
- The signature's closing `/>` is tag punctuation, not `keyword.operator.arithmetic.nx` +
  `keyword.operator.comparison.nx`
- A `//` comment inside a signature is a comment for the whole line, and no longer closes the
  enclosing tag

Renamed scope (breaking for themes keying on the old name, no alias kept):
- Primitive types (`string`, `int`, `float64`, …) are `support.type.primitive.nx`, previously
  `storage.type.primitive.nx`. `storage.type` is for a type that is itself the declarator, as in
  C's `int x = 5`; NX primitives appear only in annotation position, so they belong in the same
  family as user-defined types, which is what TypeScript does. Most themes colour
  `support.type.primitive.nx` and `entity.name.type.nx` alike, so primitives now read as types
  rather than as keywords

Right-hand sides (`= RhsExpression`):
- Every site that admits an `RhsExpression` — a value definition, a function definition, a record
  property default, a signature property default, and an attribute value — now scopes it by the same
  rule. A bare name is a `ContextualName` and is scoped `variable.other.enummember.nx` in all five
- A record property's default previously fell through to `entity.name.qualifier.nx`, the
  module-qualifier catch-all, and a value definition's right-hand side was left unscoped entirely
- A dotted name in a value position is not scoped as a qualified union case, matching the language
  rule that a `ContextualName` is a single identifier

Element references:
- An attribute named after a keyword is an attribute. In `<Question type = "multiple" />`, `type` is
  `entity.other.attribute-name.nx` rather than `keyword.declaration.type.nx`, and the `=` and
  `"multiple"` are scoped rather than left unscoped. `#keywords-core` outranked the attribute rule
  in `attributes`, and once it consumed the name the attribute rule could no longer match

Control forms:
- A `for` loop's binding variables and its iterable are scoped `variable.other.readwrite.nx`, and
  the separating `,` is `punctuation.separator.comma.nx`. The whole header between `for` and the
  body previously had no token scope at all
- A reserved literal used as a condition is a literal: `if true { … }` in a property list scopes
  `true` as `constant.language.boolean.nx` rather than `entity.name.qualifier.nx`
- A loop header keeps those scopes when it spans a line break (`for item,` / `index in items`) or
  is interrupted by a comment, and every name in a compound iterable is scoped, not just the first:
  `for item in left + right` now scopes `right`. The header is a shared `#loop-header` context
  rather than one single-line match
- A control form used as the iterable is scoped as one: in
  `for item in if ready { items } else { fallback } { … }`, `if` and `else` are
  `keyword.control.conditional.nx` rather than names, the branches are scoped, and the loop body no
  longer escapes the loop at the conditional's `}`
- A loop header whose sole binder is separated from its `in` by a line break is scoped: `for item`
  then `in items` scopes the binder, the `in`, and the iterable, where the whole header was
  previously unscoped, and blank lines or comments between the two do not change that. Because a
  header opens only on text shaped like one — `for` also occurs in prose — this layout has its own
  rule, which closes at the first line that does not continue the header. Prose that wraps after
  `for <word>` therefore mis-scopes that one word, and prose whose next line begins with `in` is
  indistinguishable from a header, so that line's names are scoped as well. In both cases the
  misreading ends at the first line that does not continue the header, rather than running to the
  next brace
- A name in a property-list condition arm that merely shares a keyword's spelling keeps its
  positional scope: `state` in `if { state => tone="danger" }` is no longer
  `keyword.declaration.state.nx`
- An attribute inside a condition arm ends at the arm's `}` instead of running to end of line.
  Everything after such an arm previously tokenized one context too deep — in
  `if compact { density="tight" } else { density="normal" }` the second `density` was
  `entity.name.qualifier.nx`, and the element's `/>` was scoped as
  `keyword.operator.arithmetic.nx` and `keyword.operator.comparison.nx`
- New `#literals` rule (`true`, `false`, `null`), split out of `#keywords-core`, which now includes
  it. New `#attribute-spread`, `#attribute-assignment`, and `#attributes-condition` rules, split out
  of `#attributes`. No scope names changed

Comments and type suffixes outside a signature:
- A `//` comment trailing a record property or a value definition is a comment. It was previously
  tokenized as code — the slashes as `keyword.operator.arithmetic.nx`, and words inside it as
  `constant.language.boolean.nx`, `keyword.control.*`, or `entity.name.qualifier.nx`
- `?` in a type position is `keyword.operator.type-modifier.nx` in a record property as well as a
  signature, matching `[]`. It was `keyword.operator.conditional.nx`, which most themes leave
  unstyled. The ternary `?` keeps that scope

Declaration recovery, qualified names, and remaining annotation positions:
- A declaration whose signature or record body is never terminated no longer swallows the rest of
  the file. Every context that can hold a declaration open — the signature, a property definition,
  a record or action body, a union case body, an `emits` or `state` group, a braced or embed
  expression, an `if`/`for` form, a start tag, and a definition's right-hand side — now recovers at
  the next top-level declaration, so the declaration after a typo is scoped as itself. Recovery
  ignores an attribute named after a keyword (`type = "multiple"`) and an indented nested binding
  (`let x = { … }` inside a braced expression), neither of which starts a declaration. Indentation
  is otherwise irrelevant: `type`, `action`, `component`, and a modifier-carrying `let` all recover
  wherever they are written, since none of them can appear inside an expression
- A qualified declaration name (`component <Ns.Widget … />`) is recognized as a declaration.
  Previously the whole signature was left unscoped because only an unqualified name was matched
- A signature terminated `/ >` now closes its component. The signature accepted the whitespace but
  the component did not, leaving the body unscoped until the next declaration
- The `content` modifier is `storage.modifier.content.nx` inside an `emits` or `state` group as
  well as in a signature; it was `entity.name.qualifier.nx` there
- A value definition's type, a parenthesized parameter's type, and a function's return type are all
  scoped by the same rule as a signature or record property. In those three positions `?` was
  `keyword.operator.conditional.nx`
- A parenthesized function parameter is scoped by the property-definition rule, the same production
  it is in the language: its name is `variable.other.property.nx` and its default is scoped by the
  `RhsExpression` rule. Both previously fell through to `entity.name.qualifier.nx`, the
  module-qualifier catch-all
- Type suffixes are scoped in source order, so `string?[]` and `Color[]?[]` scope every suffix.
  Only `[]`-then-`?` was recognized before, which left a trailing `[]` unscoped

A declaration is now scoped independently of what precedes it in the file: `external` was missing
from the lookaheads that terminate a union or record body, so a multi-line union nested every
following declaration inside a stale union-case scope.
