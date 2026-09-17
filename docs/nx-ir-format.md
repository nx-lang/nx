# NX IR, schema 4

NX IR is a deterministic binary image emitted from a successful `ProgramArtifact`. It is intended
for caching, inspection, and loading by a runtime without re-reading NX source. Schema 4 carries
one module per image, links to other modules by name, and encodes the module as flat tables so
that a name, a type or a literal is written once and referenced by index. The tables are 32-bit
cells over one string blob, laid out so a runtime reads them in place: a JavaScript reader is one
typed-array view per table, and no table is decoded before it is used.

Schema 4 differs from schema 3 by the `text` node, kind `20`, by the `float32` binary operators
`16` to `19`, and by `concat` taking string operands only; every other kind, table and layout is
unchanged. Schema 3 and schema 2 artifacts,
and the JSON encoding schema 3 had before it was released, are not read by any runtime and are not
emitted by the compiler. There is no converter.

## Reading this document

The image is described bottom-up: cells and how an entry's operands become cells, then the tables,
then the sections that hold them and the header that lists the sections. The kind numbers and the
layout of every entry are the same as the emitter's model in `crates/nx-codegen/src/ir.rs`, and
`nxlang ir explain` renders any image as text with every index resolved, so a person reads the
text and a program reads the cells.

Layouts use these operand types:

- `str` — an index into the string table.
- `type` — an index into the type table.
- `const` — an index into the constant table.
- `node` — an index into the node table; `node?` where the operand is optional.
- `decl` — an index into the declaration list.
- `slot` — a small integer naming a local of the enclosing frame (see *Slots*); `slot?` where
  optional.
- `ref` — a reference: a module slot and the referenced declaration's name. Module slot `0` is the
  image's own module.
- `ref?` — an optional reference.
- `[x...]` — a list of the given operand type, possibly empty.
- `flags` — an integer whose bits are named in the layout.

Kind numbers are assigned here and are never reused. A reader that meets a kind number it does not
know must refuse the image.

## Cells

Every number in the image is a little-endian unsigned 32-bit integer, a *cell*, at a four-byte
offset from the start of the image. The value `0xFFFFFFFF`, spelled `none` below, is reserved for
an absent optional operand and never appears as an index or a count.

An entry is a run of cells: its kind, then its operands, written by six rules that are the whole
encoding:

| Operand | Cells |
| --- | --- |
| `str`, `type`, `const`, `node`, `decl`, `slot`, an operator, a `0`/`1` flag or a `flags` word | one |
| `node?`, `slot?`, `str?` | one; `none` when absent |
| `ref` | two: the module slot, then the name as a `str` |
| `ref?` | two; `none` in the module-slot cell when absent, and then the second cell is `none` too |
| `[x...]` | a count, then each element by its own rule, recursively; an element that is itself a list of operands is written as those operands in order, with no count of its own |
| a constant's value | `int` and `float` are two cells, low word first, of the `i64` or the `f64` bits; `bigint` is one `str` |

So the node `[18, ref, [[str, node]...], [node...]]` is the cells `18, slot, name, n, (name, node)
× n, m, node × m`. There is no per-entry length cell: the table's offset array gives every entry's
extent.

## Tables

The **string table** is `count`, then `count + 1` byte offsets into the blob that follows, then the
blob, UTF-8, zero-padded to a multiple of four bytes. String `i` is `blob[offsets[i]..offsets[i +
1]]`. The offsets are non-decreasing, start at `0`, end at the blob's length, and every offset is a
character boundary. Declaration names, parameter, field, property and slot names, element tags,
string literals, primitive type names, intrinsic field orders and the digits of a `bigint` are all
interned here, in the order the emitter first uses them, followed by the strings the module section
names: the runtime ABI, the required features, and each module's identity and version.

The **type table**, the **constant table**, the **node table** and the **declaration list** each
have one shape: `count`, then `count + 1` cell offsets into the pool that follows, then the pool of
cells. Entry `i` is `pool[offsets[i]..offsets[i + 1]]`, and its first cell is its kind. The offsets
are non-decreasing, start at `0` and end at the pool's length.

### Type table

| Kind | Name | Layout | Meaning |
| --- | --- | --- | --- |
| 0 | primitive | `[0, str]` | A built-in type named by the string: `int`, `int32`, `int64`, `float32`, `float64`, `string`, `boolean`, `void`, `never` or `object`. |
| 1 | nominal | `[1, ref]` | A record, union or type alias declared in a module. |
| 2 | array | `[2, type]` | A list of the element type. |
| 3 | nullable | `[3, type]` | The inner type or `null`. |

A type is written once: two fields of type `string?` share one `nullable` entry, which itself
refers to one `primitive` entry. An `array` or `nullable` entry's inner type precedes it in the
table, so no type reaches itself.

Types appear on parameters, fields and props. Nodes do not carry types. Where evaluation depends on
a type, the node kind or operator says so: integer division and modulo are their own operators.

### Constant table

| Kind | Name | Layout | Meaning |
| --- | --- | --- | --- |
| 0 | int | `[0, i64]` | An integer within JavaScript's safe range, as two cells, low word first. |
| 1 | bigint | `[1, str]` | An integer outside that range, as its decimal digits. |
| 2 | float | `[2, f64]` | A floating-point literal, as the two cells of its IEEE 754 bits, low word first. |

A JavaScript reader reconstructs an `int` exactly as `(high | 0) * 2^32 + low`, since every value
the emitter writes with that kind is inside the safe range.

### Node table

A node refers to its children by index; a child always precedes its parent in the table, so a
reader that evaluates by index never needs to look ahead and no node reaches itself.

| Kind | Name | Layout | Meaning |
| --- | --- | --- | --- |
| 0 | null | `[0]` | The `null` literal. |
| 1 | bool | `[1, 0 or 1]` | A boolean literal. |
| 2 | string | `[2, str]` | A string literal. |
| 3 | number | `[3, const]` | A numeric literal. |
| 4 | slot | `[4, slot, str]` | Reads a local of the current frame. The string is the local's name, for diagnostics. |
| 5 | reference | `[5, ref]` | A top-level function or value. A function reference evaluates to a callable; a value reference evaluates the value. |
| 6 | binary | `[6, op, node, node]` | A binary operation; see *Binary operators*. |
| 7 | unary | `[7, op, node]` | `0` negation, `1` logical not. |
| 8 | call | `[8, node, [node...]]` | Calls the callee with the arguments. |
| 9 | intrinsic | `[9, op, [node...], [str...]]` | An update intrinsic: `0` apply, `1` merge, `2` diff, `3` changed. The string list is the declared field order for `changed`, empty for the others. |
| 10 | if | `[10, node, node, node?]` | Condition, then-branch, else-branch (absent yields `null`). |
| 11 | ifIs | `[11, node, [[[node...], node]...], node?]` | Scrutinee, arms (each a pattern list and a body), else-branch. |
| 12 | array | `[12, [node...]]` | A list literal. |
| 13 | for | `[13, slot, str, slot?, str?, node, node]` | Item slot and name, index slot and name (both absent when there is no index), iterable, body. Yields the list of body values. |
| 14 | member | `[14, node, str]` | Reads the named member of the base object. |
| 15 | record | `[15, ref, [[str, node]...], [node...]]` | Constructs the record the reference names from named properties and content children. Defaults, required fields, the content field and whether the record is an update record all come from the declaration. |
| 16 | unionCase | `[16, ref, str, [[str, node]...], [node...]]` | Constructs a case of the union the reference names. The string is the case name. A constant case evaluates to its bare name. |
| 17 | element | `[17, id, str, [[str, node]...], [node...]]` | An intrinsic element with a module-local id, a tag, properties and content children. |
| 18 | component | `[18, ref, [[str, node]...], [node...]]` | A component descriptor: props are normalized against the component's declaration. A property named `on<Emit>` for an emit the component declares is a handler property, not a prop: its value is an action handler, and it is carried on the descriptor rather than normalized. |
| 19 | actionHandler | `[19, ref, str, ref, slot, ref?, node]` | An action handler, `onTapped=<Update count={count + 1} />`: the component and the name of the emit it answers, the action record it accepts, the slot of its `action` binding, the owner component whose body bound it (absent at the root), and the body. Evaluating the node captures the frame; the body runs when a host dispatches the action. See *Action handlers*. |
| 20 | text | `[20, node, str]` | The canonical text form of a primitive value: the operand, and the name of its static type, one of `int`, `int32`, `int64`, `float32`, `float64` or `boolean`. See *Text conversion*. |

A `{ expression }` block in NX source is its expression; it has no node of its own. NX has no
syntax for a local `let` binding, an index expression or a function type, so none has a kind.

Property lists are sorted by property name. Content lists are in source order.

#### Binary operators

| Op | Name | Meaning |
| --- | --- | --- |
| 0 | add | Numeric addition. |
| 1 | sub | Numeric subtraction. |
| 2 | mul | Numeric multiplication. |
| 3 | div | Floating-point division. |
| 4 | idiv | Integer division, truncating toward zero; both operands must be integers. |
| 5 | mod | Floating-point remainder. |
| 6 | imod | Integer remainder; both operands must be integers. |
| 7 | concat | String concatenation. Both operands are strings; see *Text conversion*. |
| 8 | eq | Structural equality. |
| 9 | ne | Structural inequality. |
| 10 | lt | Less than. |
| 11 | le | Less than or equal. |
| 12 | gt | Greater than. |
| 13 | ge | Greater than or equal. |
| 14 | and | Logical and. Non-strict in its right operand. |
| 15 | or | Logical or. Non-strict in its right operand. |
| 16 | fadd32 | `float32` addition: the sum, rounded to the nearest `float32`. |
| 17 | fsub32 | `float32` subtraction: the difference, rounded to the nearest `float32`. |
| 18 | fmul32 | `float32` multiplication: the product, rounded to the nearest `float32`. |
| 19 | fdiv32 | `float32` division: the quotient, rounded to the nearest `float32`. |

`and` and `or` are the only operators that do not evaluate both operands. `and` evaluates its right
operand only when its left is true, and `or` only when its left is false, exactly as
`if left then right else false` and `if left then true else right` would. Every other operator
evaluates both. This matters because NX is not total: it is what lets `d != 0 && n / d > 1` guard
the division rather than perform it.

Division or remainder by zero is a runtime diagnostic.

#### Text conversion

Whether a `+` in NX source adds or concatenates is decided by type analysis, from the operand
types and never from their syntactic form, and the emitter writes `add` or `concat` accordingly.
A `+` concatenates when either operand is a `string`; the other may be a `string` or a primitive
with a text form. The emitter wraps each operand that is not a string in a `text` node, so
`concat` only ever sees strings and a runtime does not coerce its operands: `"Total: " + count`
with `count:int` is `concat` over the string and `text<int>(count)`. A text body that binds to a
`string` content property is emitted the same way, as one chain of `concat` nodes over its runs and
its braced values.

The `text` node names its operand's static type because a runtime need not be able to recover it.
A JavaScript runtime carries every numeric type as a `number`, so a `float32` holding the nearest
value to `0.1` arrives as `0.10000000149011612`, and only the named type says it prints as `0.1`.
The text forms are:

| Type | Text |
| --- | --- |
| `int`, `int32`, `int64` | The decimal digits, with a leading `-` when negative. |
| `float64` | The ECMAScript `Number::toString` form: an integral value without a fraction (`1`), otherwise the shortest digits that round-trip, the exponent form at or above 10^21 and below 10^-6 (`1e+21`, `1e-7`), `0` for negative zero, and `NaN`, `Infinity`, `-Infinity`. |
| `float32` | The same, with the shortest digits that round-trip to the same `float32`. |
| `boolean` | `true` or `false`. |

Numeric widening, by contrast, emits nothing. An `int` written at a `float64` site, or an `int`
operand of a `float64` addition, is recorded at its own type with no conversion node, because every
supported runtime carries the integer and floating-point types in one numeric representation and
the widening is unobservable there. The operator still follows the checked type: `n / x` with
`n:int` and `x:float64` is `div`, not `idiv`.

The same holds for a `float32`, provided every `float32` value a runtime holds is already rounded
to a `float32`. The emitter keeps that true for arithmetic: an `add`, `sub`, `mul` or `div` whose
checked type is `float32` is written as `fadd32`, `fsub32`, `fmul32` or `fdiv32`. A runtime that
computes the operation on `float64` operands and rounds the result to the nearest `float32` (in
JavaScript, `Math.fround`) gets exactly the `float32` result, so `v * 3` with `v:float32` holding
`2.3` is `6.899999618530273` when widened or compared, as it is under the interpreter. A `float32`
remainder is always exact, so it stays `mod`, and negation needs no rounding. Constants are
rounded when emitted, and a runtime rounds a number the host supplies at a `float32` site.

### Declaration list

Each entry is `[kind, str, ...]`, the string being the declaration's name. Top-level names are
unique within a module; the compiler rejects a module that declares one name twice. Derived
declarations use the names the language gives them: `User.Update`, `User.Property`.

| Kind | Name | Layout |
| --- | --- | --- |
| 0 | function | `[0, str, [[str, type, isContent]...], node]` — name, parameters (name, type, `0` or `1`), body. |
| 1 | value | `[1, str, node]` — name and value. |
| 2 | record | `[2, str, [field...], [ref...], isAbstract, ref?]` — name, fields, abstract bases nearest first, `0` or `1`, and the update target when the record is a derived `T.Update`. |
| 3 | component | `[3, str, [field...], [field...], node?, flags, [[str, ref]...]]` — name, props, state, body (absent for an external component), flags: bit 0 abstract, bit 1 external, and the emits: each an emit's local name and a reference to its action record, inherited emits included, in declaration order. An inherited emit's reference names the module that declared it. |
| 4 | union | `[4, str, [[str, [field...], isConstant]...], [ref...], ref?]` — name, cases (name, fields, `0` or `1`), abstract bases, and the property target when the union is a derived `T.Property`. |
| 5 | typeAlias | `[5, str]` — name only. |

A field is `[str, type, node?, flags]`: name, type, default and flags: bit 0 content, bit 1
required. Record and component fields arrive flattened, a derived record listing its inherited
fields before its own, so `bases` answers only what flattening cannot: whether a value stamped with
one name is acceptable where another type was asked for. An abstract record has no values of its
own. An update record's fields are all optional with no defaults, so a value keeps an absent field
absent. A union case is constant when it declares no fields in a union with no base; its wire form
is then the bare case name rather than a `$type` object.

### Slots

A slot is an integer local to the frame that owns it. Each function, value, component, record and
union case is one frame. The frame's leading slots are assigned in declaration order to the
parameters, then (for a component) the props followed by the state fields, or (for a record or
union case) the fields. Every `let`, `letStatement` and `for` binding after that, and every action
handler's `action` binding, takes the next integer in the order the body is walked. Two
declarations are free to use the same integers.

A record's or component's field default can read the fields declared before it through their slots.
A runtime normalizing a construction evaluates each default in a fresh frame for that declaration,
binding each field's slot as it goes.

### Element ids

An element node's id is an integer local to the module, unique among the module's elements.

## Sections

The tables live in *sections*, each starting on a four-byte boundary and listed in the directory
(see *Image*). Section kinds are assigned here and are never reused.

| Kind | Name | Contents |
| --- | --- | --- |
| 0 | strings | The string table. |
| 1 | module | The header data below. |
| 2 | types | The type table. |
| 3 | constants | The constant table. |
| 4 | nodes | The node table. |
| 5 | declarations | The declaration list. |
| 6 | debug | Spans and source text; optional (see *Debug section*). |

Kinds `0` to `5` are required, each exactly once. A reader skips a directory entry whose kind it
does not know, so a section can be added later without a schema change; a section whose layout
changes is a schema change.

### Module section

The module section is one run of cells:

| Cells | Meaning |
| --- | --- |
| `str` | The runtime ABI, `nx-ir-runtime-v2`. |
| `[str...]` | The required features (see *Required features*). |
| count, then per module: `str`, `str`, two cells | The module table: identity, version, and the fingerprint as a 64-bit value, low word first. |
| `[decl...]` | The function entrypoints: the module's top-level functions, in declaration order. |
| `[decl...]` | The component entrypoints: the module's top-level components, in declaration order. |

Entry `0` of the module table is the image's own module. Every other entry is a module the image
references directly — through a call, a value reference, a component descriptor, a record or
union-case construction, a nominal type, a base record, or a derived declaration's target. A module
reachable only transitively is not listed; it appears in the table of the module that references it.
Referenced modules are listed in the order the emitter first meets them, so the order is
deterministic.

- The identity is the module's logical workspace identity, for example `input.nx` or `app/main.nx`.
- The version is the string the host gave the workspace module when it built the program, or `""`
  when it gave none. It is part of the module, not an emit option, so every image emitted from one
  program records the same version for a module. It is recorded, not interpreted: a runtime
  linking two images compares the strings for equality.
- The fingerprint is a 64-bit hash of the module's identity and source text: FNV-1a over the
  identity's UTF-8 bytes, a zero byte, and the source's UTF-8 bytes, so the same module fingerprints
  the same whatever emitted it. A JavaScript reader holds it as a `BigInt`, or as its decimal string.

A host asking for a function or component by name looks it up through the entrypoint lists; the
declaration's own entry gives the name.

### Debug section

When present, the debug section is: the declaration spans as a count and `(start, end)` pairs, one
per declaration; the node spans likewise, one per node; then the source text as a byte length and
UTF-8 bytes, zero-padded to a multiple of four. A span's offsets are byte offsets into the source.
A node emitted from another module's text — a default inherited from a base component declared
elsewhere — has `none, none`. The source is stored here rather than in the string table so that an
image with and without its debug section differs only in this section and in the directory.

The CLI writes the section; the SDKs omit it unless asked. An image without it prepares and
evaluates exactly as one with it. A runtime diagnostic cites the span when the section is present
and the declaration's name when it is not.

## Image

An image begins with a 16-byte header of four cells:

| Offset | Cell | Value |
| --- | --- | --- |
| 0 | magic | The ASCII bytes `NXIR`. |
| 4 | schema version | `4`. |
| 8 | total length | The length of the whole image in bytes. |
| 12 | directory count | The number of directory entries that follow. |

The directory follows at offset 16: one entry of three cells per section, `kind`, `offset` and
`length`, each offset and length a multiple of four and each section inside the image. The emitter
writes the sections in kind order, contiguously after the directory. A reader refuses an image
whose magic or schema version it does not implement, naming the version found and the version it
supports, and does not interpret the bytes that follow the header; it refuses an image whose total
length is not the length of the bytes it was given, which is the truncation check.

## Validation

A reader establishes, before it answers any question about an image, that the header and directory
are well formed, that every offset array is non-decreasing and ends at its pool, that the string
blob is UTF-8 and every string offset is a character boundary, and that every entry of every table
follows its kind's layout with every index inside the table it names: strings, types, constants,
nodes, declarations and module slots, with `none` allowed only where the layout says so. A node's
child indices and an `array` or `nullable` type's inner index are bounded by the entry's own index
rather than the table's count, so an entry that names itself or a later entry is refused and every
walk over children terminates. After that pass every read is in bounds, so evaluation reads cells
without further checks. A malformed or truncated image is refused with a diagnostic rather than an
exception, and no input can make a reader read outside the image.

Both the Rust reader (`NxIrImage::open`) and the TypeScript runtime (`prepareNxIrModule`) are
tested against the corpus: each truncates every image at every four-byte boundary, and each
overwrites every cell of a chosen image with four values, refusing the result or reading it as valid
but never failing another way. The Rust suite damages the smallest image and the smallest image
carrying a debug section, so span offsets and the source length are damaged too, and probes every
node and type cell of every image with its own entry index, which is the bound the paragraph above
describes. The TypeScript suite damages the smallest image that owns a function entrypoint and links
and evaluates every damaged image it accepted, so a hostile artifact is exercised through evaluation
and not only through opening.

## Required features

A module lists a feature only when it uses the construct that needs one, so most modules list none.
A module that declares a derived update record lists `update-records-v1`; one that declares a
derived property union lists `property-unions-v1`; one that calls an update intrinsic lists
`update-intrinsics-v1`; one that binds an action handler lists `action-handlers-v1`. A runtime that
does not know a listed feature refuses the image rather than guessing, which is what makes the list
safe to grow. An image does not record its evaluation
semantics here. The schema version and the runtime ABI carry that, and an intentional change to how
a runtime evaluates an image bumps the ABI.

## Values

The value model is unchanged from schema 2. A record value is an object with a `$type` naming the
record and one key per field. A union case is `{ "$type": "Union.case", ... }`, or the bare case
name when the case is constant. A component descriptor is an object whose `$type` is the component
name. An intrinsic element is an object whose `$type` is the tag, with its children under `content`: one
child as itself, several as a list.
An integer outside JavaScript's safe range is `{ "$type": "nx.int", "value": "<decimal>" }`, and
arithmetic on one is a runtime diagnostic in JavaScript. An action handler is
`{ "$type": "ActionHandler", "action": "<name>" }`, the name being the declaration name of the
action record it accepts (`Button.Tapped` for an inline emit, `SearchSubmitted` for a shared one),
plus a `token` when the output came from a lifecycle render; the record names the handler and is
not the handler, so a runtime accepts it as input only where *Action handlers* says.

## Action handlers

A handler node captures the frame when it is evaluated: the body reads every local it closes over
through the slot it already had, so the image carries no environment, and a handler for a component
declared in another module resolves its component and action by reference like any other node.
Whether the owner reference is present decides nothing by itself; what matters is which instance
dispatches the handler. A handler whose owner is the component an instance was initialized from
reads that instance's state live when it runs, through the owner's state slots, and every
`<Owner>.Update` record it returns patches the instance's state. Any other handler in the output,
one bound at the root or one written in a parent's body and rendered by a child through content, sees
only what it captured, and everything it returns is an effect for the host.

The lifecycle a runtime implements is the interpreter's. Initialization normalizes the props, sets
the handler properties aside (a body never sees `onTapped`), materializes the state, renders the
body, and assigns every handler in the output a token `h<generation>-<n>`, generation `1`, numbered
by a walk that visits lists in order and object keys in sorted order. Dispatch takes an ordered batch
of entries, each an action record the component emits or `{ "$type": "ActionHandlerInvocation",
"token", "action" }` naming a handler of the most recent output; an emitted action runs the handler
the parent bound under `on<Emit>`, and is a no-op when none was, once the action has been
constructed against its record. After the batch the body renders once against the state the batch
produced, with the generation incremented, so a token from an earlier output never names a handler
again. A batch is atomic: a failure returns a diagnostic and the instance the host holds is
unchanged. A host initializing a child from a parent's rendered descriptor hands the descriptor's
fields over as props together with the parent's instance, and every `ActionHandler` record among
them, at any depth, is replaced by the handler the parent holds under its token; what is
substituted is the parent's handler value itself, so a host holding both instances can recognize it
in the child's table. Initialization may also be given a complete state to use in place of the
initial one, which is how a host re-renders an instance with new props and the state it holds;
the runtime has no other "update props" operation and needs none.

## Determinism

Equivalent `ProgramArtifact` inputs emitted with the same options produce byte-identical images.
Declarations are in source order; entrypoints in declaration order; a node's children precede it
and sibling lists keep source order; properties are sorted by name; strings, types and constants
are in first-use order, the module section's strings last; referenced modules are in first-use
order; sections are in kind order. The same bytes come out of the wasm SDK, the Node SDK, the .NET
SDK and the CLI, which share one emitter, and the parity tests compare them.

## Linking

A reference is a module slot and a declaration name, never a position, so a module regenerated with
a new declaration in the middle still satisfies every image compiled against the old one, as long
as the names it referenced remain. A runtime prepares each module once — validating its image,
indexing its declarations by name, building its entrypoint tables — and links an entry module
against prepared modules a host resolver supplies by identity. Linking checks that each resolved
module's version equals the version recorded in the entry's table, unless the host opts into
linking across versions, and that every declaration the entry references is present. A module whose
table holds only itself is a program on its own.

## Worked example

The corpus program `snippet` compiles this `input.nx` against a `drawnui.nx` module (version `9`)
that declares external components `SkiaLayout`, `SkiaLabel` and `SkiaButton`:

```nx
let title = "Conformance"
let root() =
  <SkiaLayout Type="Column" Spacing=8>
    <SkiaLabel Text={title} FontSize=24 />
    <SkiaLabel Text="A snippet links by name to a catalog the runtime already holds." />
    <SkiaButton Text="Run" />
    <SkiaButton Text="Share" Enabled={false} />
  </SkiaLayout>
```

Its image without the debug section, `specs/ir-conformance/snippet/expected/input.nx.stripped.nxir`,
is 836 bytes. Cells are shown as little-endian values, sixteen bytes per line, offsets in hex.

```
0000  "NXIR"  4         344       6           header: magic, schema 4, 836 bytes, 6 sections
0010  0       58        118       1           directory: strings at 0x58, 280 bytes; module …
0020  170     38        2         1a8         … at 0x170, 56 bytes; types at 0x1a8 …
0030  8       3         1b0       28          … 8 bytes; constants at 0x1b0, 40 bytes
0040  4       1d8       140       5           nodes at 0x1d8, 320 bytes; declarations …
0050  318     2c                              … at 0x318, 44 bytes
```

The string section holds 20 strings: a count, 21 offsets and the blob.

```
0058  14      0         5         10          count 20; offsets 0, 5, 16, …
0068  14      1b        21        25
0078  2d      31        3a        79
0088  7c      86        8d        92
0098  9c      a4        a4        ae          strings 15 and 16: "input.nx", then ""
00a8  af      bf                              … and the blob ends at byte 191
00b0  "titleConformancerootSpacingColumnTypeFontSizeTextSkiaLabelA snippet links by name
       to a catalog the runtime already holds.RunSkiaButtonEnabledShareSkiaLayoutinput.nx
       drawnui.nx9nx-ir-runtime-v2" + one zero byte of padding
```

So string `0` is `title`, `1` `Conformance`, `2` `root`, `3` `Spacing`, `4` `Column`, `5` `Type`,
`6` `FontSize`, `7` `Text`, `8` `SkiaLabel`, `9` the long literal, `10` `Run`, `11` `SkiaButton`,
`12` `Enabled`, `13` `Share`, `14` `SkiaLayout`, `15` `input.nx`, `16` `""`, `17` `drawnui.nx`,
`18` `9` and `19` `nx-ir-runtime-v2`.

The module section:

```
0170  13      0         2         f           ABI string 19; no features; 2 modules; module 0 …
0180  10      a3ea564e  1cde592b  11          … "input.nx", "", fingerprint 2080198121860257358 …
0190  12      9a4d6f33  90ed48c1  1           … module 1: "drawnui.nx", "9", fingerprint 10443083107296702259
01a0  1       0                               function entrypoints [1]; component entrypoints []
```

The type table is empty (`0` and one offset, `0`), and the constant table holds `8` and `24` as
`int` constants, two cells each:

```
01b0  2       0         3         6           count 2; offsets 0, 3, 6
01c0  0       8         0                     constant 0: int 8
01cc  0       18        0                     constant 1: int 24
```

The node table holds fourteen nodes in 64 cells:

```
01d8  e       0         2         4           count 14; offsets 0, 2, 4, 6, 8, 11, 20, 22, 29, 31, …
01e8  6       8         b         14
01f8  16      1d        1f        26
0208  28      2a        33        40          … 38, 40, 42, 51, 64
0218  2       1                               node 0:  string "Conformance"
0220  3       0                               node 1:  number 8
0228  2       4                               node 2:  string "Column"
0230  3       1                               node 3:  number 24
0238  5       0         0                     node 4:  reference slot 0 "title"
0244  12      1         8         2           node 5:  component drawnui.nx:SkiaLabel, 2 props:
0254  6       3         7         4           …        FontSize = node 3, Text = node 4,
0264  0                                       …        no content
0268  2       9                               node 6:  string "A snippet links …"
0270  12      1         8         1           node 7:  component SkiaLabel, 1 prop:
0280  7       6         0                     …        Text = node 6; no content
028c  2       a                               node 8:  string "Run"
0294  12      1         b         1           node 9:  component SkiaButton, 1 prop:
02a4  7       8         0                     …        Text = node 8; no content
02b0  1       0                               node 10: bool false
02b8  2       d                               node 11: string "Share"
02c0  12      1         b         2           node 12: component SkiaButton, 2 props:
02d0  c       a         7         b           …        Enabled = node 10, Text = node 11
02e0  0                                       …        no content
02e4  12      1         e         2           node 13: component SkiaLayout, 2 props:
02f4  3       1         5         2           …        Spacing = node 1, Type = node 2,
0304  4       5         7         9           …        4 children: nodes 5, 7,
0314  c                                       …        9, 12
```

The declaration list:

```
0318  2       0         3         7           count 2; offsets 0, 3, 7
0328  1       0         0                     declaration 0: value "title" = node 0
0334  0       2         0         d           declaration 1: function "root", no params, body node 13
```

`nxlang ir explain` prints the same thing in words:

```
module input.nx fingerprint 2080198121860257358
links drawnui.nx version "9" fingerprint 10443083107296702259
entrypoints functions [root] components []

value title =
  "Conformance"

function root() =
  <drawnui.nx:SkiaLayout Spacing=8 Type="Column">
    <drawnui.nx:SkiaLabel FontSize=24 Text=title />
    <drawnui.nx:SkiaLabel Text="A snippet links by name to a catalog the runtime already holds." />
    <drawnui.nx:SkiaButton Text="Run" />
    <drawnui.nx:SkiaButton Enabled=false Text="Share" />
  </drawnui.nx:SkiaLayout>
```

## CLI usage

```bash
nxlang codegen ./app/main.nx --target nx-ir --output ./generated
```

Workspace inputs use the same selected entry identity as executable source generation:

```bash
nxlang codegen ./workspace --target nx-ir --entry app/main.nx --output ./generated
```

IR generation writes one `<identity>.nxir` image per module of the program, each with its debug
section, and does not emit JavaScript or TypeScript runtime helper files.

```bash
nxlang ir explain ./generated/main.nxir
```

renders an image as text with every index resolved, and fails with a diagnostic on a file that is
not an image, is truncated, or is of a schema version the build does not read. The same text comes
from `explainNxIr` in the wasm and Node SDKs and `NxRuntime.ExplainNxIr` in the .NET SDK, so a
page that has the SDK can read any image it holds.

## SDK boundaries

Every SDK emits the image as bytes: `Uint8Array` from the wasm SDK, `Buffer` from the Node SDK,
`byte[]` from the .NET SDK. The wasm ABI and the C FFI answer a call with one buffer, so they carry
several artifacts as a *bundle*: a little-endian `u32` header length, a JSON header
`[{ identity, metadata, offset, length }]`, zero padding to a four-byte boundary, then the images
at the offsets the header gives, measured from the start of the bundle. The loaders slice the images
out and hand each over as its own buffer.

Every SDK takes a module's version on the workspace module: `{ identity, source, version }` in the
wasm and Node SDKs, `NxWorkspaceModule.Version` (or `FromSourceText`'s `version`) in the .NET SDK,
and `version_ptr`/`version_len` on the C FFI's `NxWorkspaceModule`. The emit options are the modules to emit and whether to keep the
debug section, and nothing else; an unknown key is refused.

## TypeScript runtime

The TypeScript runtime lives in `runtime/typescript` and exposes:

- `prepareNxIrModule` / `tryPrepareNxIrModule`, over a `Uint8Array` or an `ArrayBuffer`
- `linkNxIrProgram` / `tryLinkNxIrProgram`
- `prepareNxIrProgram` / `tryPrepareNxIrProgram`, for a self-contained image
- `evaluateFunction`
- `constructComponentDescriptor`
- `initializeComponent`, which returns the rendered output, the initial state and an instance, and
  takes a `parent` instance to resolve `ActionHandler` records in the props and a `state` to use
  in place of the initial one
- `evaluateComponent`
- `dispatchComponentActions`, over an instance and a batch, returning the rendered output, the
  effects, the next state and the next instance
- `normalizeComponentState`
- `applyComponentStatePatch`

Preparation validates the image as described under *Validation*, then the runtime ABI, the required
features and the kind numbers, before returning a prepared module whose tables are views over the
bytes given; a string is decoded the first time it is named. Public host APIs resolve names through
the entrypoint tables. A module that names another module in its table must be linked before it is
evaluated. An instance is an immutable value the host holds between calls; the runtime keeps no
state of its own, and a dispatch that fails leaves the instance it was given usable.

## Conformance corpus

`specs/ir-conformance/` holds NX programs with their expected images, with and without the debug
section, the explained text of each image beside it, the expected canonical value of every named
entrypoint, and, for every lifecycle a program names, the rendered output of initialization and
the rendered output and effects of each dispatched batch, tokens included. The emitter's tests pin
the images byte for byte and check that each committed text is the explanation of its committed
image; the TypeScript runtime's tests evaluate the images, drive the lifecycles, and refuse every
truncation and cell overwrite of them; and the corpus is where a second runtime starts. It covers
every node, type and declaration kind, a program spanning two images, derived declarations, a
document that is a single trailing element, and components that bind action handlers. It also holds
the size budget: an image emitted without its debug section is at most six times the UTF-8 length
of its module's source.

## Non-goals

NX IR is eager. Every operand is evaluated before its parent and exactly once, except in the
branches of `if` and `ifIs` and in the right operand of `and` and `or`, which its left operand may
skip. Nothing in the encoding mutates a value or reassigns a binding, and a node's children precede
it in the table, so a runtime is free to memoize by node index: only a changed input can invalidate
a cached result.

Dependency tracking, subscriptions, invalidation and hidden mutable component instances are
therefore runtime policy rather than properties of this format, which neither provides them nor
prevents them. The runtimes here implement none of them. They evaluate a declaration when asked,
and a component's state belongs to the host, which validates and patches it through pure runtime
APIs. A runtime that wants to track dependencies derives what it needs by walking the tables at
prepare time, so no image carries that metadata and none needs to.

The one deferred evaluation the format has is the action handler, and it is deferred the way the
language defines it: the body closes over the frame by value, runs when a host dispatches the
action, and returns records rather than performing anything. Which instance a handler runs against,
and what becomes of what it returns, is the lifecycle a runtime implements (see *Action handlers*),
not a property of the node. Reducers sit in between. Functions, derived update records and the
update intrinsics express what a reducer does, while no declaration says which function reduces
which component, so a runtime that wants NX-owned reducers supplies that association itself.
