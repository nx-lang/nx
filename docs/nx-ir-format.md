# NX IR, schema 5

NX IR is a deterministic binary image emitted from a successful `ProgramArtifact`. It is intended
for caching, inspection, and loading by a runtime without re-reading NX source. Schema 5 carries
one module per image, links to other modules by name, and encodes the module as flat tables so
that a name, a type or a literal is written once and referenced by index. The tables are 32-bit
cells over one string blob, laid out so a runtime reads them in place: a JavaScript reader is one
typed-array view per table, and no table is decoded before it is used.

Schema 5 differs from schema 4 by the `seq` type kind, `5`, which carries an occurrence and
replaces the `array` and `nullable` type kinds; by the `exists`, `optionalMember` and `coalesce`
node kinds, `23` to `25`, and the `{}` match pattern; and by the `null` node kind no longer being
emitted. Every other kind, table and layout is unchanged. Schema 4 differed from schema 3 by the
`text` node, kind `20`, by the `float32` binary operators `16` to `19`, and by `concat` taking
string operands only. Schema 4 and earlier artifacts, and the JSON encoding schema 3 had before it
was released, are not read by any runtime and are not emitted by the compiler. There is no
converter.

## Reading this document

The image is described bottom-up: cells and how an entry's operands become cells, then the tables,
then the sections that hold them and the header that lists the sections. The kind numbers and the
layout of every entry are the same as the model in the format crate, `crates/nx-ir`, and
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
know must refuse the image. A retired kind keeps its number, so no later kind takes it; the emitter
never writes it, and a reader that meets one reports the image as malformed.

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
| 2 | array | — | Retired with schema 5, replaced by `seq`. Never emitted; an entry of this kind is malformed. |
| 3 | nullable | — | Retired with schema 5, replaced by `seq`; as `array`. |
| 4 | function | `[4, type, [[str, type, flags]...]]` | A function type, `<function Item:Contact Index:int />: DrawnNode`: the result type, then each parameter's name, type and flags (bit 0: the parameter takes body content; bit 1: the parameter is optional, `p?:T`), in declared order. A function satisfies it by parameter name, so a function may declare fewer parameters than the type. |
| 5 | seq | `[5, type, occurrence]` | An item type under an occurrence, `string?`, `Person+`, `object*`: the item type, then an occurrence cell whose bit 0 says the type may be empty and bit 1 that it may hold many, so `?` is `1`, `+` is `2` and `*` is `3`. `0` is not written: a type that is exactly one has no wrapper. The item type is never itself a `seq`. |
| 6 | anyFunction | `[6, type]` | A function reference type, `<function ... />: HttpArguments`: a function type whose parameters are not stated, with the result type as its one operand. A function of any parameters whose result satisfies the result type satisfies it; `<function ... />: object*` is the type every function satisfies. A module whose type table holds an entry of this kind lists `function-reference-type-v1`. |

A type is written once: two fields of type `string?` share one `seq` entry, which itself refers to
one `primitive` entry. A `seq` entry's item type precedes it in the table, so no type reaches
itself. Every occurrence in source is a `seq`, and a field, prop, state field or parameter declared
`p?:T` is typed by its read type — `T?`, or `T*` for `p?:T+` — so a runtime normalizes an omitted
field to the empty value from the type alone, with no second flag. `nxlang ir explain` prints a
`seq` by its NX spelling and parenthesizes a function type under a suffix,
`(<function Item:object Index:int />: string)?`, because a suffix written after a function type's
result would bind to the result. A function reference type is written once per result type like
any other, prints as `<function ... />: R`, and is parenthesized the same way,
`(<function ... />: object*)+`. The top type never stands in for it, and neither does a function
type with no parameters, which only a function declaring none satisfies.

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
| 0 | null | — | Retired with schema 5: the language has no null value. Never emitted; a node of this kind is malformed. The empty value is an empty `array` node, `[12, []]`. |
| 1 | bool | `[1, 0 or 1]` | A boolean literal. |
| 2 | string | `[2, str]` | A string literal. |
| 3 | number | `[3, const]` | A numeric literal. |
| 4 | slot | `[4, slot, str]` | Reads a local of the current frame. The string is the local's name, for diagnostics. |
| 5 | reference | `[5, ref]` | A top-level function or value. A function reference evaluates to a function value, in any expression position — a call's callee, a property value, a list element, a result; a value reference evaluates the value. See *Function values*. |
| 6 | binary | `[6, op, node, node]` | A binary operation; see *Binary operators*. |
| 7 | unary | `[7, op, node]` | `0` negation, `1` logical not. |
| 8 | call | `[8, node, [node?...]]` | Calls the callee with its arguments by position. An absent argument is a parameter the call left out, and the list may stop before the trailing parameters it leaves out; the callee fills each with its default or, when the parameter is optional, the empty value. See *Parameters a call leaves out*. |
| 9 | intrinsic | `[9, op, [node...], [str...]]` | An update intrinsic: `0` apply, `1` merge, `2` diff, `3` changed. The string list is the declared field order for `changed`, empty for the others. |
| 10 | if | `[10, node, node, node?]` | Condition, then-branch, else-branch. An `if` with no else yields the empty value, `[]`, when the condition is false. |
| 11 | ifIs | `[11, node, [[[node...], node]...], node?]` | Scrutinee, arms (each a pattern list and a body), else-branch. The `{}` pattern, which matches the empty value, is an empty `array` node, `[12, []]`, in an arm's pattern list; a module with one requires `occurrence-v1`. |
| 12 | array | `[12, [node...]]` | A list literal. With no elements it is the empty value, `{}` in source, and `ir explain` prints it as `{}`. |
| 13 | for | `[13, slot, str, slot?, str?, node, node]` | Item slot and name, index slot and name (both absent when there is no index), iterable, body. Yields the list of body values. |
| 14 | member | `[14, node, str]` | Reads the named member of the base object. |
| 15 | record | `[15, ref, [[str, node]...], [node...]]` | Constructs the record the reference names from named properties and content children. Defaults, required fields, the content field and whether the record is an update record all come from the declaration. |
| 16 | unionCase | `[16, ref, str, [[str, node]...], [node...]]` | Constructs a case of the union the reference names. The string is the case name. A constant case evaluates to its bare name. |
| 17 | element | `[17, id, str, [[str, node]...], [node...]]` | An intrinsic element with a module-local id, a tag, properties and content children. |
| 18 | component | `[18, ref, [[str, node]...], [node...]]` | A component descriptor: props are normalized against the component's declaration. A property named `on<Emit>` for an emit the component declares is a handler property, not a prop: its value is an action handler, and it is carried on the descriptor rather than normalized. |
| 19 | actionHandler | `[19, ref, str, ref, slot, ref?, node]` | An action handler, `onTapped=<Update count={count + 1} />`: the component and the name of the emit it answers, the action record it accepts, the slot of its `action` binding, the owner component whose body bound it (absent at the root), and the body. Evaluating the node captures the frame; the body runs when a host dispatches the action. See *Action handlers*. |
| 20 | text | `[20, node, str]` | The canonical text form of a primitive value: the operand, and the name of its static type, one of `int`, `int32`, `int64`, `float32`, `float64` or `boolean`. See *Text conversion*. |
| 21 | namedCall | `[21, node, [[str, node]...]]` | A call of a function-typed value by name, `<Row Item={c} Index={i} />` where `Row` is a parameter, a prop or a `let`: the callee, which evaluates to a function value, then each argument as a name and a node, sorted by name like any property list. The runtime binds the arguments to the callee's own parameters by name — a name the declaration lacks is dropped, a parameter it declares must be present. See *Function values*. |
| 22 | forRange | `[22, slot, str, slot?, str?, node, node]` | A `for` whose iterable evaluates to a `Range` record, with the layout of a `for`. The body runs once per integer from the range's `start` upward, stopping before `end` or after it when `endInclusive` is true, with the item bound to that integer and the index, when present, to its position from zero. An empty or reversed range runs the body no times. Yields the list of body values. Requires `ranges-v1`. |
| 23 | exists | `[23, node]` | `x?`: `true` when the operand holds at least one item, `false` when it is the empty value. Requires `occurrence-v1`. |
| 24 | optionalMember | `[24, node, str]` | `x?.m`: the named member of the base when the base holds a value, and the empty value when the base is empty. Requires `occurrence-v1`. |
| 25 | coalesce | `[25, node, node]` | `x ?? y`: the left operand when it holds at least one item, otherwise the right, which is evaluated only when the left is empty. Requires `occurrence-v1`. |

A `{ expression }` block in NX source is its expression; it has no node of its own. NX has no
syntax for a local `let` binding or an index expression, so neither has a node kind. A function
type has a *type* kind but no node kind: a function reaches an expression by name, as a
`reference`. There is no node for a `null` value and none for a conditional operator: absence is
the empty value, an empty `array`, and the presence operators are the three nodes above.

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

`and` and `or` are the only binary operators that do not evaluate both operands. `and` evaluates
its right operand only when its left is true, and `or` only when its left is false, exactly as
`if left then right else false` and `if left then true else right` would. Every other operator
evaluates both; among the nodes, only `coalesce` is non-strict the same way. This matters because
NX is not total: it is what lets `d != 0 && n / d > 1` guard the division rather than perform it.

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
| 0 | function | `[0, str, [[str, type, node?, flags]...], node, type?, flags]` — name, parameters (name, type, default, flags: bit 0 content, bit 1 optional), body, declared result type and result flags. A parameter declared `p?:T` is typed by its read type, `T?` (`T*` for `p?:T+`), and sets the optional bit, so a call that leaves it out binds the empty value. A default is a node of the function's frame. See *Parameters a call leaves out* and *Results*. |
| 1 | value | `[1, str, node, type?]` — name, value and declared type. See *Results*. |
| 2 | record | `[2, str, [field...], [ref...], isAbstract, ref?]` — name, fields, abstract bases nearest first, `0` or `1`, and the update target when the record is a derived `T.Update`. |
| 3 | component | `[3, str, [field...], [field...], node?, flags, [[str, ref]...]]` — name, props, state, body (absent for an external component), flags: bit 0 abstract, bit 1 external, and the emits: each an emit's local name and a reference to its action record, inherited emits included, in declaration order. An inherited emit's reference names the module that declared it. |
| 4 | union | `[4, str, [[str, [field...], isConstant]...], [ref...], ref?]` — name, cases (name, fields, `0` or `1`), abstract bases, and the property target when the union is a derived `T.Property`. |
| 5 | typeAlias | `[5, str]` — name only. |

A field is `[str, type, node?, flags]`: name, type, default and flags: bit 0 content, bit 1
required. Record and component fields arrive flattened, a derived record listing its inherited
fields before its own, so `bases` answers only what flattening cannot: whether a value stamped with
one name is acceptable where another type was asked for. An abstract record has no values of its
own. An update record's fields are all optional with no defaults, so a value keeps an absent field
absent; a field whose type is a `?` `seq` is clearable, and a runtime accepts a present empty value
only there. A union case is constant when it declares no fields in a union with no base; its wire
form is then the bare case name rather than a `$type` object.

### Results

A function's result and a value's initializer are typed sites. The declared type is present only
when the source declares one, and a runtime normalizes the result to it as it does an argument:
`5` returned where `int+` is declared is `[5]`, and a one-item sequence where `int?` is declared is
its item. Without a declared type the result is the body's value as it is.

Bit 0 of a function's result flags is set when its result type, declared or inferred, is a
standalone `T?`. An entry call — a host evaluating the function — then returns an empty result to the host
as `null`, the host's spelling of an absent single value, rather than `[]`. Inside the program, and
for any other type, the empty value stays `[]`.

### Slots

A slot is an integer local to the frame that owns it. Each function, value, component, record and
union case is one frame. The frame's leading slots are assigned in declaration order to the
parameters, then (for a component) the props followed by the state fields, or (for a record or
union case) the fields. Every `let`, `letStatement` and `for` binding after that, and every action
handler's `action` binding, takes the next integer in the order the body is walked. Two
declarations are free to use the same integers.

A record's or component's field default, and a function's parameter default, can read the fields or
parameters declared before it through their slots. A runtime normalizing a construction evaluates
each default in a fresh frame for that declaration, binding each field's slot as it goes, and a call
binds the callee's parameter slots the same way.

### Parameters a call leaves out

A call may leave a parameter out: a `call` node's argument is absent, or the list stops before it.
The callee fills the parameter, not the caller: a runtime evaluates the parameter's default in the
callee's frame, after binding the parameters before it, so the default a program gets is the one the
function's module declares at link time, and it may read values that module keeps private. A
parameter with no default is the empty value when it is optional; otherwise the call is refused,
naming the parameter. A default is normalized to its parameter's type, as an argument is. A host
calling a function positionally may likewise pass fewer arguments than it has parameters, and a
`namedCall` or a host call by name leaves out a parameter by not naming it.

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

The entry module's image also lists, after the modules it references and in the program's module
order, every module that declares a concrete record or union extending an abstract record that a
function takes or returns, at any depth, counting the functions of the modules the entry reaches
through its imports. A host may name such a shape by `$type` in a
value it passes to the function, and a runtime resolves a shape only from a module it linked, so the
entry links them even when nothing references them. No other image lists a module it does not
reference, which keeps a library module's image the same whichever program emitted it.

- The identity is the module's logical workspace identity, for example `input.nx` or `app/main.nx`.
  A module of a library a host loaded from memory is named by the library's logical root and its
  identity within the library, for example `libraries/question-flow/QuestionFlow.nx`; a module of a
  library loaded from a directory is named by its canonical path.
- The version is the string the host gave the workspace module when it built the program, or the
  one it gave a library module's library when it loaded it, or `""` when it gave none. It is part of
  the module, not an emit option, so every image emitted from one program records the same version
  for a module. It is recorded, not interpreted: a runtime linking two images compares the strings
  for equality.
- The fingerprint is a 64-bit hash of the module's identity and source text: FNV-1a over the
  identity's UTF-8 bytes, a zero byte, and the source's UTF-8 bytes, so the same module fingerprints
  the same whatever emitted it. A JavaScript reader holds it as a `BigInt`, or as its decimal string.

The whole root `@nx/` is reserved for modules the compiler carries: a workspace that supplies a
module under it is refused, and so is a host library whose root lies under it. Two kinds of module
live there, the prelude and the standard libraries.

`@nx/prelude.nx` is the NX prelude. It holds the declarations every NX module sees without an
import, starting with `Range`. It is otherwise an ordinary module: an image that constructs a `Range`, names it as a type,
or derives from it lists the prelude in its table and reaches the declaration through that slot, and
no prelude declaration is ever copied into another module's image. Unlike a library module, whose
table entry carries the version its host gave it, the prelude's entry carries the compiler's prelude version —
`1` today. No host supplies the prelude, so that version is the only thing a runtime can compare its
own built-in copy against. It names the prelude's contract rather than its text: it is bumped when a
declaration's shape changes and left alone for an edit that changes no declaration, so a reworded
comment does not reject images already in the wild. Linking is then decided by that version, through
the same check every library goes through — a mismatch is `nx-ir-link-version` — and by which
declarations the resolved prelude holds, where a gap is `nx-ir-link-missing-declaration`, naming the
prelude and the declaration.

An emit request produces the prelude's image when it names the prelude's identity, and a request for
every module includes it exactly when some module of the program references it — so a program that
uses no prelude declaration emits what it emitted before the prelude existed, byte for byte. The
image is written under a file name derived from the identity like any module's, which is a legal path
on every supported platform.

A standard library is NX source the compiler carries and a module imports by name, as
`import "@nx/agent"`. Its modules are named by the library's root and the module's identity within
the library, `@nx/agent/agent.nx`, and each is an ordinary linked module: an image that references
one of its declarations lists the module in its table and reaches the declaration through that
slot, and nothing of the library is copied into another module's image. An emit request produces a
standard library module's image when it names the module's identity, and a request for every module
includes it exactly when the program links the library, so a program that imports no standard
library emits what it emitted before they existed. The image is the same whichever program it was
emitted from.

A standard library module's version is derived from the library's source rather than given by a
host or bumped by hand: the 16 lowercase hexadecimal digits of a 64-bit FNV-1a hash over, for each
module of the library in identity order, the module's identity within the library, a zero byte, its
source text and a zero byte. Any edit to the library, a comment included, changes it. That is the
opposite of the prelude's rule, and for a reason: no runtime carries a standard library's image, so
there is no built-in copy for a contract number to be compared against. The entry image and the
library image of one build come from the same compiler and always agree; an image from another
build is caught by the ordinary version check.

A host asking for a function or component by name looks it up through the entrypoint lists; the
declaration's own entry gives the name.

### Recognizing a range

A `forRange` node's iterable evaluates to a value, and a consumer recognizes a range by that value's
shape: the `$type` `Range` and the fields `start`, `end` and `endInclusive`. It has nothing else to
go on. A canonical value carries `$type` as a bare declaration name with no module — two modules that
each declare a `Card` produce values a consumer cannot tell apart, which is why a program answers
`nominalShapesFor` with every shape of a name rather than a guess — and a record's type arguments are
erased: a nominal type is a slot and a name with no arguments, so the prelude's `Range` has `start`
and `end` typed `object`, and `<Range T=int/>` and `<Range T=float64/>` are one declaration with one
shape in the image.

Provenance is decided where the image does carry it. A `forRange` node is emitted only for an
iterable whose declaration is the prelude's, so a `Range` a module declares for itself never reaches
one; a value a host supplies is normalized against the site's nominal type, which names its module by
slot. What neither catches is a host handing a record of the right name and shape to a range-typed
site, and that is the exposure every record of a shared name has rather than one of ranges.

Erasure also settles what an item's carrier is. NX's `int`, `int32` and `int64` are a distinction the
checker draws; below it, a backend binds items with whatever numeric types it has. An engine with a
separate 32-bit integer and a separate float, such as the NX interpreter, can bind a narrow item and
will refuse a range whose bounds are floats. The Rust IR runtime has a 64-bit integer and a float,
and computes every integer width as the one integer. JavaScript has one number type, so an `int32` item is an
`int` — as every `int32` is in that backend, not only a range's — and a range of `float64` whose
bounds happen to be integral iterates, because `4.0` and `4` are one value.

Of those two, only the float range is out of reach from checked NX, because `range-not-iterable`
refuses a non-integer range iterable outright; it takes a host value or a hand-built image. The
carrier is reachable, and it is observable, because the interpreter wraps `int32` arithmetic where a
JavaScript backend widens: `for i in lo..hi { i * 2 }` over an `int32` range at the top of the range
yields `-294967296` under the interpreter and `4000000000` under both JavaScript backends and the
Rust IR runtime. That
divergence is not the range's — `let a:int32 = 2000000000` with `a + a` divides the backends the
same way — but a range is one of the places a program meets it.

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
| 4 | schema version | `5`. |
| 8 | total length | The length of the whole image in bytes. |
| 12 | directory count | The number of directory entries that follow. |

The directory follows at offset 16: one entry of three cells per section, `kind`, `offset` and
`length`, each offset and length a multiple of four and each section inside the image. The emitter
writes the sections in kind order, contiguously after the directory. A reader refuses an image
whose magic or schema version it does not implement, naming the version found and the version it
supports — a schema 4 image is refused as found `4`, supported `5` — and does not interpret the
bytes that follow the header; it refuses an image whose total length is not the length of the bytes
it was given, which is the truncation check.

## Validation

A reader establishes, before it answers any question about an image, that the header and directory
are well formed, that every offset array is non-decreasing and ends at its pool, that the string
blob is UTF-8 and every string offset is a character boundary, and that every entry of every table
follows its kind's layout with every index inside the table it names: strings, types, constants,
nodes, declarations and module slots, with `none` allowed only where the layout says so; an entry
of a retired kind, `array`, `nullable` or `null`, is malformed. A node's child indices and a `seq`
type's item index are bounded by the entry's own index rather than the table's count, so an entry
that names itself or a later entry is refused and every walk over children terminates. After that
pass every read is in bounds, so evaluation reads cells without further checks. A malformed or
truncated image is refused with a diagnostic rather than an exception, and no input can make a
reader read outside the image.

The Rust reader (`NxIrImage::open` in `nx-ir`), the Rust runtime (`PreparedModule::prepare` in
`nx-ir-runtime`, which reads through that reader) and the TypeScript runtime (`prepareNxIrModule`)
are tested against the corpus: each truncates every image at every four-byte boundary, and each
overwrites every cell of a chosen image with four values, refusing the result or reading it as valid
but never failing another way. The Rust suite damages the smallest image and the smallest image
carrying a debug section, so span offsets and the source length are damaged too, and probes every
node and type cell of every image with its own entry index, which is the bound the paragraph above
describes. The TypeScript suite damages the smallest image that owns a function entrypoint and links
and evaluates every damaged image it accepted, so a hostile artifact is exercised through evaluation
and not only through opening. The Rust runtime's suite does the same over every cell of every
image of every corpus program whose images are all within 16,384 bytes, running each damaged
image's function entrypoints and component lifecycles. That is every program but the two large
ones: overwriting a cell runs the whole program again, so they are cut at every boundary and not
swept cell by cell.

## Required features

A module lists a feature only when it uses the construct that needs one, so most modules list none.
A module that declares a derived update record lists `update-records-v1`; one that declares a
derived property union lists `property-unions-v1`; one that calls an update intrinsic lists
`update-intrinsics-v1`; one that binds an action handler lists `action-handlers-v1`; one whose type
table holds a function type, whose node table references a function anywhere but as a `call`'s
callee, or which contains a `namedCall` lists `function-values-v1`; one whose type table holds a
function reference type (kind `6`) lists `function-reference-type-v1`, so that a runtime that
predates the kind refuses the module by name rather than as malformed, and lists
`function-values-v1` too only on that feature's own terms; one that contains a `forRange`
lists `ranges-v1`; one that contains an `exists`, `optionalMember` or `coalesce` node, or an `ifIs`
arm with the `{}` pattern, lists `occurrence-v1`. Constructing a range needs no feature: that is an
ordinary record construction, and only iterating one is a node a runtime may not know. A `seq` type
needs none either: it is a type kind, not a node, so a module that declares `string?` fields and
never tests, steps through or falls back from one lists nothing. A runtime that does not know a
listed feature refuses the image rather than guessing, which is what makes the list safe to grow.
An image does not record its evaluation semantics here. The schema version and the runtime ABI
carry that, and an intentional change to how a runtime evaluates an image bumps the ABI.

## Values

A record value is an object with a `$type` naming the record and one key per present field.
Absence is the empty value, and the empty value is the empty array: a `+` or `*` value is an array,
the empty `*` value being `[]`; a `?` value holding an item is that item itself, unwrapped; and a
record stores no key for a field that holds the empty value, whatever its occurrence, so an omitted
`author?:Person` and an omitted `tags?:string+` are both missing keys. No value is `null`. The one
exception is an update record, which must tell a cleared field from an untouched one: it carries a
key only for each present field and writes a cleared field as `null` on the wire, and a host
supplying `null` or `[]` under a key of an update record clears that field. A union case is
`{ "$type": "Union.case", ... }`, or the bare case name when the case is constant. A component
descriptor is an object whose `$type` is the component name. An intrinsic element is an object
whose `$type` is the tag, with its children under `content`: one child as itself, several as a
list. An integer outside JavaScript's safe range is `{ "$type": "nx.int", "value": "<decimal>" }`
in canonical JSON. The TypeScript runtime cannot hold such an integer and refuses one where it
reads it from an image, with `nx-ir-number`; the Rust runtime holds it as a 64-bit integer. A host
that passes that record at `object` passes a record, in every runtime (see *Rust runtime*). An action handler is
`{ "$type": "ActionHandler", "action": "<name>" }`, the name being the declaration name of the
action record it accepts (`Button.Tapped` for an inline emit, `SearchSubmitted` for a shared one),
plus a `token` when the output came from a lifecycle render; the record names the handler and is
not the handler, so a runtime accepts it as input only where *Action handlers* says.

### Host values

What a host passes to an evaluation API, and what it gets back, are *canonical values*: the empty
value, a boolean, a number, a string, a sequence, a record, which may carry a type name, and the
function values and action handlers a runtime makes. That is the whole model, and it is the same
for every host.

A host holds a canonical value in its own language's data, in the *form* its runtime defines, and
passes it as it holds it. Nothing is encoded on the way in or on the way out, so a value a host
computed a moment ago is passed exactly as one it read from JSON is. Canonical JSON, described
above, is the encoding of the same values for a wire or a store. A host does not have to produce
it, and what a host reads from it is already in the form:

| A canonical value | Canonical JSON | JavaScript | Rust |
| --- | --- | --- | --- |
| The empty value | `[]`, or the key left out | `[]`, or the member left out or set to `undefined` | `NxValue::Array` of no items, or the property left out |
| A boolean | `true`, `false` | A boolean | `NxValue::Bool` |
| An integer | A number. Outside JavaScript's safe range, the `nx.int` record | A number. The runtime cannot hold one outside the safe range | `NxValue::Int`, or `NxValue::Int32` |
| A float | A number | A number | `NxValue::Float`, or `NxValue::Float32` |
| A string, a constant union case | A string | A string | `NxValue::String` |
| A sequence | An array | An array | `NxValue::Array` |
| A record | An object, with `$type` where the site needs it to tell the type | A plain object, with a `$type` member likewise | `NxValue::Record`, with its `type_name` likewise |
| A function value, an action handler | The record that names it | That record, or, where a value is held, the value the runtime returned in a state | That record |

A number takes the width its site declares, so no form says `int32` or `float32` apart from Rust's,
which can. A host's `null`, `NxValue::Null` in Rust, is read as the empty value where a site
admits none; *Rust runtime* says where the two runtimes differ on one at `object`.

A form may spell a value in a way JSON cannot, where the language makes it natural. JavaScript has
one: a member of a plain object set to `undefined` is a member left out, at any depth, which is
what an optional property of a TypeScript type produces. Rust has one: `NxValue::Int` holds any
64-bit integer, with no record around it. Each such spelling has one meaning and one size (see
*Input size*).

A plain object is the record of JavaScript: an object whose prototype is `Object.prototype`, of
any realm, or that has none. JavaScript can hold much that is not a canonical value: an instance
of a class, a `Date`, a `Map`, a `Set`, a typed array, a function, a symbol, a big integer, an
item of an array that is `undefined`. The TypeScript runtime refuses each where the host passes
it, at any depth, inside a value typed `object` included, with `nx-ir-boundary-type` and the path
to it. It converts none of them: a `Date` could be read as text or as milliseconds and an instance
with or without what its class keeps for itself, and whichever a runtime chose, the program would
be given something the host did not write. A host converts what it has, which for an instance of a
class is to spread it. A host's type joins a form when NX has the kind of value it would spell,
and not before, so what is refused today can be accepted later without changing what any working
host gets. Rust needs no such rule: an `NxValue` can hold nothing else.

What holds the values a host passes is read by the same rule. Positional arguments, content and a
batch are each an array, and props, a state, a patch and arguments by name are each a plain
object; a `Set`, an object with a `length` or an instance of a class in their place is refused.
The same object held in two places is two values, read and measured once for each place.

## Function values

A function is a value: a `reference` node naming a function declaration evaluates to one wherever
it stands, not only as a `call`'s callee. Functions are module-level and a body has no nested
declarations, so a function value captures nothing: the declaration is the whole value. Canonical
output renders one as the record `{ "$type": "Function", "module": "<identity>", "name": "<name>" }`,
the same record in both runtimes, and two function values are equal exactly when they name one
declaration. A host that supplies such a record where a function type is expected — a descriptor's
prop, a component's initialization — has it resolved to the declaration it names, and one naming
no function of the linked program is refused with a diagnostic naming it.

A site typed by a function reference type, `<function ... />: R`, follows the same boundary rule.
A runtime accepts a function value of the program there, and a host-supplied `Function` record
that names a function declaration of the linked program, whatever that function's parameters. A
record naming a module the program does not link, or a name that module does not declare as a
function, is refused with `nx-ir-function-value`, and any other value with `nx-ir-boundary-type`.
The result operand is not compared with the named function's result, as no part of a signature is
compared at a function type with stated parameters: the compiler checked the value where the
program bound it. A host that takes such records from elsewhere checks what the function returns.
NX code never calls a value of the type, so no node kind is involved; a host calls the record by
the function's own parameter names. The `function-references` corpus program covers the type.

A function-typed value is called by name, never by position: `namedCall` carries each argument's
name because the callee's declaration is known only at run time, and its parameters may be fewer,
or ordered differently, than the type the caller holds. The TypeScript runtime exposes the same
binding to hosts as `callFunction(program, record, args)`, which drops an argument the function
does not declare and fails naming the first parameter it declares and the arguments lack. The
`function-values` corpus program covers a function type, a function bound to a prop and to a
parameter, calls by element of a parameter with the exact and a smaller parameter set, and a
function value in a result.

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

## Evaluation cost

The cost of an evaluation is counted in *operations*, and the count is a property of the images and
the input alone: two runtimes that evaluate the same images with the same values and compute the
same result count the same operations, in the same order. That is what lets a host give one
evaluation a budget of operations and mean the same thing in every runtime. Six things cost
operations:

| Charge | Cost | When |
| --- | --- | --- |
| A node is evaluated | 1 | Before any of its children. Every evaluation counts: a loop body once per iteration, a parameter's default each time a call leaves the parameter out, a field's or a state field's default each time it is evaluated, a value's initializer each time a `reference` evaluates it, and a handler's body each time a dispatch runs it. |
| An item is placed in a sequence a node builds | 1 per item | As the item is placed, after the child that produced it. The sequences are the value of an `array`, `for` or `forRange` node and the content list of a `record`, `unionCase`, `element` or `component` node. A child whose value is a sequence places each of its items; an empty one places none. A `for` over one item rather than a sequence yields its body's value and places nothing. A call that binds a sequence to a content parameter builds the list anew: each item of the argument costs 1, empty or not, or 1 for each item it contributes when it is itself a sequence of several. |
| A `concat` produces a string | 1 per 64 UTF-16 code units, rounded down | After both operands, before the string exists. |
| A value is checked against a declared type | 1 per value that is not a sequence | Before the value is checked. A sequence type costs nothing itself: each item is checked, and a one-item sequence at a type that is not a sequence type is read as its item. A record, or a union case with fields, is one and then each of its field values, defaults included; a field left out with no default is not checked. A value at the type `object` is one value whatever it holds. |
| Two values are compared | 1 per pair, and 1 more per 64 UTF-16 code units of text read | Before the pair is compared. Two sequences of one length compare their items in order, and two handlers of one node their captured values slot by slot, each up to and including the first pair that differs. Two records of one type are lined up by field name: a name both hold compares the pair of its values, and a name only one holds costs 1, all of them compared and counted whether or not an earlier one differed. The text read is the shorter of two strings, the shorter of two records' type names, and every field name of two records of one type, for each record that holds it. |
| A value is written for the host | 1 per value, and 1 more per 64 UTF-16 code units of text written | Before the value is written. Every value is one, because every value takes a place in what is written: a sequence, the empty value included, and then its items; a record and then its field values. The text written is a string, a record's type name, and each of its field names. A function value and an action handler are one each, and only a value the runtime made is one of them: a record a host passes with the type name or the internal marking of one is a record, entered and paid for like any other, and so is the `nx.int` record canonical JSON spells a large integer with. |

The first three are what a program does. The last three are the walks a runtime makes over a value,
and they are charged because a value is held by reference: a record that names one value in two
fields unfolds, for anything that walks it as a tree, to twice the value. Forty levels of that cost
a few hundred operations to build and are 2^40 values to a walk, so each walk pays for every value
it visits. A value nobody walks costs only what building it cost.

A value is checked against a type wherever evaluation gives it one: an argument or a default
against its parameter, a function's result against its declared result type, a value declaration's
value against its declared type, each field of a `record`, `unionCase` or `component` construction,
and every value the host supplies to a typed site. Two values are compared by `eq` and `ne`, by a
pattern of an `ifIs` arm that matches by equality, and by `diff` for each field, a field one record
does not hold comparing as the empty value. A pattern matches by equality unless the pattern or the
scrutinee is the empty value, or both are records and the pattern names a type: those are decided
by emptiness or by the type name and compare nothing. A record holds a field when it stores a
value for it, so an optional field that was left out or is empty is not compared, and a field an
update record clears is. A handler captures the slots of its frame as they stood when it was made.
A value is written when a call returns it: a function's result, a descriptor, rendered output, each
effect, and component state. A value reached from several places is written once for each, and a
`null` inside a value the host passed at `object` is the empty value: one value where it is
written, and one item where a list that holds it is bound to a content parameter.

Names are text too. A declared type or field name is short and costs nothing, but a host may pass
an object at `object` whose key is as long as it likes, so a name is charged by its length where it
is compared and where it is written, like any string. A comparison of two lists stops at the first
pair of items that differs, since a list's items are in one order in every runtime and the count
is the same wherever it stops; a comparison of two records goes on to the end, since the order of
a record's fields is not, and a count that stopped at the first differing field would depend on
it. The result of a comparison never depends on whether a budget is set or on how the two values
are held: a record that holds a NaN is not equal to itself.

Two nodes cost one operation whether or not a runtime evaluates them. A pattern of an `ifIs` arm
costs one when it is tested, including a pattern naming a union case with fields, which a runtime
reads without building the record; a pattern after the one that matched is not tested. The callee
of a `call` or `namedCall` costs one, including a `reference` to a function a runtime resolves in
place.

Everything else is free. A node that is not evaluated costs nothing: the branch a conditional does
not take, and the right operand of `and`, `or` or `coalesce` when the left decides. So does the
rest of what a runtime does: preparing and linking, reading the host's input before it is checked,
an update intrinsic's own work apart from the comparisons of `diff`, collecting a dispatch's
effects, and assigning handler tokens. None of that may grow with the size of a value, as it is
held or as it unfolds when walked as a tree: a runtime that walks a value for a purpose of its own
visits a value reached from several places once, and does not walk again what an evaluation did
not change.

A runtime that offers a budget fails an evaluation at the first charge that would take the count
above it, before the operation charged is performed: a node is not evaluated, an item is not placed,
a string is not built, a value is not checked, compared or written. The failure is
`nx-ir-resource-limit` and names a declaration. For a node, an item placed, a `concat` or a
comparison it is the declaration the node belongs to, with the node's span when the image carries
its debug section: the node evaluated, the node building the sequence, the `binary` node, the
pattern, or the `intrinsic` node. For a check against a type it is the declaration whose parameter,
result, field or value is being checked for, which for a field of an update record is the update
record; for an item bound to a content parameter it is the function called; and for a value written
for the host it is the function or component the call evaluated. None of the three has a span. A
budget equal to an evaluation's cost succeeds.

The rules are written so that every step that touches a value in proportion to its size is charged
in proportion, and the time and the memory of an evaluation are proportional to its count: what is
left uncharged in a step is meant to be bounded by the program's declarations, the fields of a
record or the slots of a frame, and not by its data. It is a claim about every step of two
runtimes, and review has several times found a step that broke it, each time through a large or
unusual value a host passed at `object`: a wide object, a long key, a list of empty values. Four
things back the claim now. The input limit of [*Input size*](#input-size) bounds what a step
nobody has found can cost. A differential test runs generated host values through both runtimes
and requires the same count, input size and failures of each. The Rust runtime's allocations are
held to a multiple of the count for every generated case. And a time report, which does not block
a merge, names a case whose time grows faster than its count. They are described under
[*Validation against generated host values*](#validation-against-generated-host-values). What is
still open is what those tests have found and not yet had fixed, the known findings listed in
`crates/nx-codegen/tests/cost/mod.rs`, and whatever they cannot see: a step driven by a value a
program builds, which only the fixed probes exercise. So a host that runs code it did not write
should treat the budget as its first limit, set the input limit beside it, and keep a limit of its
own on time and memory as well, a CPU limit or a separate isolate, where it has one. Typed data is checked each time it meets
a type, so a list of `n` items returned through `k` calls with a declared result costs about `k ×
n`. A count belongs to an image, not to source; an emitter change that adds or removes nodes changes
what a program costs, and the corpus's recorded counts show every such change.

Worked counts, which both runtimes' tests and the corpus pin:

| Source | Evaluated with | Operations |
| --- | --- | --- |
| `let add(a:int, b:int) = { a + b }` | `1`, `2` | 6: two arguments checked, the `binary` node and its two `slot` operands, and the number written |
| `let root() = { for i in 0..4 { i * i } }` | | 29: the `forRange` node, 7 for its iterable (the `Range` record, its three literals and its three fields checked), 3 for each of the 4 evaluations of the body, 4 for the items placed, and 5 for the list and its numbers written |
| `let pick(flag:boolean) = { if flag { 1 } else { for i in 0..1000 { i } } }` | `true` | 8: the argument checked, the `if`, its `slot`, the `array` the branch `{ 1 }` is emitted as, its literal, the item placed, and the one-item list and its number written; the loop costs nothing |
| `let one() = { 1 }` and `let root() = { one() }` | | 4: the `call`, its `reference` callee, the literal body of `one`, and the number written |
| `let twice(xs:int+) = { xs xs }` | 500 integers | 2,504: 500 items checked, the `array`, two `slot` nodes and 1,000 items placed, and the list and its 1,000 items written |
| `let join(a:string, b:string) = { a + b }` | two strings of 1,000 code units | 68: two arguments checked, the `binary` and its two `slot` nodes, 31 for the 2,000 code units built, and 32 to write the string |
| `let same(xs:int+) = { xs }` | 10,000 integers | 20,002: 10,000 items checked, the `slot`, and the list and its 10,000 items written |
| `let sameText(a:string, b:string) = { a == b }` | two strings of 1,000 code units | 22: two arguments checked, the `binary` and its two `slot` nodes, 16 for the comparison (the pair and 15 for its 1,000 code units), and the boolean written |
| `<Point x={1} y={2} /> == <Point x={3} y={2} />` | | 3 for the comparison, besides the nodes: the two records and each pair of fields, the second although the first differs |
| `a == b` for two host lists at `object` | two lists of 100,000 integers that differ in their first item | 2 for the comparison: the two lists and the first pair of items |
| `let ignore(content c:object): int = { 1 }` | a list of 100 integers, held as `xs:object` and passed as `ignore(xs)` | 106 for the call: the `call`, its callee and its `slot` argument, 100 items bound, the list checked as one value, the literal, and the result checked |
| `let passObject(o:object): object = { o }` | `{ "a": null, "b": [null, null], "c": [] }` | 9: the argument and the result checked, the `slot`, and six values written, the record, the empty value under `a`, the list under `b` and its two empty items, and the empty list under `c` |
| `a == b` for two host objects at `object` | `{ "a": 1, "b": 2 }` and `{ "a": 1, "c": 2 }` | 4 for the comparison: the two records, the pair under `a`, and one for each of `b` and `c`, which only one holds |

### Input size

The *input* of one call of an evaluation API is what the host passes to it, and it has a size that
depends on those values alone, so every runtime measures the same input alike. The size is the sum
of the sizes of the values the host supplies, each measured as a value written for the host is
charged: one for each value, a sequence, the empty value and a `null` included, and one more for
every 64 UTF-16 code units, rounded down, of a string, of a record's type name and of each field
name. A value costs the same to hand in as to get back.

The values a host supplies are:

| What the host passes | Measured as |
| --- | --- |
| The positional arguments of a function | Each argument. The list of them is not a value and is not counted. |
| The arguments of a call by name | One record |
| The function record such a call names | One value |
| The props of a component | One record |
| Content | Each item |
| A state the host passes in | One record |
| A dispatched batch | Each entry |
| A state patch | One record |

A map measured as one record costs one for the record and then, for each entry, the length of its
name and the size of its value, so the names a host chooses count like any field names. A component
instance is not input, and neither is the parent instance a host names when it initializes a child:
a runtime produced them under earlier calls, where what they hold was charged as it was written.
The helpers that take no runtime options and evaluate no program code, the update helpers and the
restoring of a stored instance, have no input in this sense. The size is measured before any value
is checked against a type and does not depend on the types the values reach.

Hosts do not all spell one input the same way, and the two runtimes are not given the same forms
of it. Where they differ, the size is fixed so that it does not:

- An integer is one value, a 64-bit integer the Rust runtime is given as `NxValue::Int` included.
  The record canonical JSON spells an integer outside the safe range with,
  `{ "$type": "nx.int", "value": "<digits>" }`, is a record wherever a host passes it and is
  measured as one: the record, its string, and the string's length. No measure recognizes it.
- Props, and the arguments of a call by name, that the host leaves out are an empty record, whose
  size is one. A state the host does not pass is not input.
- A member named `$type` that holds a string is the type name of the record or the map it is in,
  and counts its length alone. Holding anything else, it is a field like any other.
- An element of the positional arguments that is undefined, or a hole in them, is the empty value,
  whose size is one. A parameter the arguments do not reach is not input, so the size does not
  depend on the declaration.
- A member of a JavaScript object that is `undefined` is a member left out (see *Host values*):
  neither its name nor a value is counted, so `{ a: 1, b: undefined }` measures what `{ a: 1 }`
  does.
- A value that is not a canonical value is one value and is not read further. In JavaScript that
  is anything but `null`, a boolean, a number, a string, an array or a plain object: a function, a
  big integer, a `Date`, a `Map`, an instance of a class, and the handler and function objects a
  runtime keeps for its own use, which a measure must not follow into the linked program. A
  runtime knows its own by having made them, not by a marking: an object read from JSON that
  carries the marking of one is a plain object and is measured as one. The size is taken before
  any value is read, so the measure counts one for such a value; a call then refuses it when it
  reads its input, unless it is one of the runtime's own, so a value that counted one is never one
  the runtime goes on to walk.

A runtime may offer a limit on the input size: `maxInputSize` in the TypeScript runtime and
`RuntimeOptions::max_input_size` in the Rust runtime, each unset by default, which is unlimited. A
call whose input is larger than the limit fails with `nx-ir-resource-limit` whose limit is named
`maxInputSize`, before the program is looked at, before any value is checked against a type and
before any node is evaluated; the failure names no declaration and has no span. So input over the
limit is reported before any other fault of the call: an entrypoint the program lacks, props of the
wrong type, an exhausted budget. A batch is measured whole before its first entry runs. Measuring
stops as soon as the size passes the limit, and it does not recurse, so input that is too large,
that nests deeply or, in JavaScript, that holds itself is refused by the limit and by nothing else.
Input within the limit is then read as *Host values* describes, and a value that is not canonical
is refused there: the limit comes first.
Text far longer than the limit allows is refused from its length without being read: the Rust
runtime, whose strings are UTF-8, refuses a string from its byte length when that alone passes what
is left of the limit, and scans at most 192 bytes for each unit of the limit otherwise.

The limit is separate from the operation budget. Measuring charges no operation, an evaluation
costs the same number of operations under any limit its input fits, and a host that sets no limit
has nothing measured. One difference between the runtimes remains. The Rust runtime bounds the
nesting of a host value at 256, and it applies the input limit first: input over the limit is
reported for its size, and input within the limit that nests more deeply is refused for its
nesting, naming `maxValueNesting`, which the TypeScript runtime does not bound.

Worked sizes, which both runtimes' tests pin, with these declarations:

```nx
let add(a:int, b:int): int = { a + b }
component <Card title:string = "none" count:int = 0 content body?:object+ /> = { <Text value={title} /> }
component <Counter /> = {
  state { count:int = 0 note?:string data?:object }
  <Button onTapped=<Update count={count + 1} /> />
}
```

| Call | Input | Size |
| --- | --- | --- |
| A function of one argument | A list of 1,000 integers | 1,001: the list and its items |
| A function of one argument | A string of 6,400 UTF-16 code units | 101: the value and 100 for its length |
| A function of one argument | An object whose one field name is 1,048,576 code units long and holds a number | 16,386: the object, 16,384 for the name, and the number |
| A function of one argument | `{ "$type": "nx.int", "value": "9007199254740993" }` | 2: the record and its string. A list of two of them is 5 |
| A function of one argument, in the Rust runtime | `NxValue::Int(9007199254740993)` | 1 |
| A function of one argument | `{ "$type": "nx.int", "value": <a string of 1,048,576 code units> }` | 16,386 |
| A function of one argument, in JavaScript | An object read from JSON, `{ "$nxKind": "actionHandler", "text": <a string of 1,048,576 code units> }` | 16,387: the object, the marking's string, and the text with its 16,384. It is a plain object |
| `add` called by name | The function record `{ "$type": "Function", "module": "main.nx", "name": "add" }` and the arguments `{ "a": 1, "b": 2 }` | 6: the function record and its two strings, and the arguments as one record with its two numbers |
| A descriptor of `Card` | The props `{ "title": "Home", "count": 3 }` and the content `1`, `2`, `3` | 6: the props as one record with its two values, and three items |
| `Card` initialized | The props `{ "title": "Home", "count": 3 }` | 3: the record and its two values |
| `Card` initialized | No props, or the props `{}` | 1 |
| `Card` initialized | The props `{ "$type": "Card", "title": "Home" }` | 2: the record and the title. The call within the limit is then refused for a field `Card` does not declare |
| `Counter` initialized or evaluated | No props and the state `{ "count": 3, "note": "x" }` | 4: the empty props, and the state as one record with its two values |
| A dispatch against a `Counter` | Two entries, each `{ "$type": "ActionHandlerInvocation", "token": "h1-1", "action": { "$type": "Button.Tapped" } }` | 6: for each, the entry, its token and its action. The instance is not input, whatever its state holds |
| The state of `Counter` normalized | `{ "count": 3, "note": "x" }` | 3 |
| A patch applied to the state of `Counter` | The state `{ "count": 3, "note": "x" }` and the patch `{ "count": 4 }` | 5: the state, and the patch as one record with its number |

#### What the limit bounds

The budget bounds what a program does by its count, and the count is proportional to the work only
if every step is charged. The input limit is there for a step that is not: one nobody has found
yet that does work in proportion to a host value for a fixed charge. Every break of the cost model
that review found after the first needed a shape only a host can supply, a megabyte name, an
untyped object tens of thousands of fields wide, a list of empty values, since a program declares
its names and the widths of its records and its sequences flatten empty values away. With a budget
`B` and an input limit `C`:

- A step that is linear in a host value and is not charged costs at most about `B × C` units of
  work in one call, where a unit is a value or 64 code units of text. At a budget of 100,000 and a
  limit of 1,000 that is about 10^8: seconds at the worst, and a ceiling where there was none. It
  is the cost of a step nobody has found, not of an ordinary call, which costs its count.
- A step driven by a value the program builds itself is not bounded by the limit. Building a value
  costs its size, so such a step is bounded by the square of the budget, whatever the limit is.
- A step that is worse than linear in a host value is bounded only as a function of the limit: it
  no longer depends on what arrived, and that is all.
- What an instance holds is not input. Its state was written under the budget of the call that
  wrote it, so it is bounded by that budget when every earlier call had one, and by nothing when a
  host ran one without. A host that limits input and not operations has not bounded state.
- Refusing one very wide JavaScript object lists all its names once, since an engine lists an
  object's names before any is read. That is at most once in a call, for the one object the limit
  is passed in, and is not multiplied by the budget: about half a second for two million names.
  The names of members that are `undefined` are listed too and count nothing, so they are work
  the limit does not bound either; a value read from JSON has none, so only a host's own code can
  make them.
- A JavaScript value that holds one object in many places is as large as the tree it spells, and
  is measured and read as that tree: fifty objects that each hold the next one twice are 2^50
  values. The limit refuses such a value as soon as its size passes it. With no limit, reading it
  takes time in proportion to that tree and charges no operation, so the budget alone does not
  bound a call that is passed one. A value read from JSON holds nothing twice.

So the limit is not a guarantee of proportionality, and a host that runs code it did not write
still limits time and memory as well.

### What a call used

A runtime may report to the host what one call used: `usage` in the TypeScript runtime's options,
an object the runtime writes `operations` and `inputSize` to, and `RuntimeOptions::usage` in the
Rust runtime, a `Usage` the host shares with it. The numbers are the cost model's and no others.
The operations reported for a call that succeeds are its operation count, the least budget under
which it succeeds: charging is the same under every budget, so what was charged under a large one
is what a tight one must hold. For a call that fails they are the operations charged before the
failure. A charge the budget refused is not among them, so the number is no more than the budget
and is the same in every runtime: a call under a budget of 10 that has been charged 5 and is then
refused a charge of 31 for a concatenation reports 5. The input size reported is the input size of
the call as defined above.

Operations are reported only for a call that ran under an operation budget, and an input size only
for a call that ran under an input limit and whose input was within it, since input that is refused
is not measured to its end. A call refused for its input under a budget used no operations and
reports none used. With no budget a walk over a value skips its charges altogether, which is what
keeps the budget free for a host that does not use it, and a report does not bring that cost back:
a host that wants the count and no limit sets a budget it cannot reach. The report is cleared when
a call begins and filled when it ends, on success and on failure, and asking for one changes
neither what a call costs nor what it returns.

A runtime that offers the input limit also exports the measure, `measureInputSize(value, limit?)`
and `input_size(&NxValue, Option<u64>)` with `record_input_size` for a map of named values, so
that a host can hold one part of what it passes to a number of its own before it calls. Given a
limit it stops as soon as the size passes it and answers some number greater than the limit; a
JavaScript value that holds itself has no finite size and has to be measured with one. A
value measured alone has the size it adds to a call that is given it as one of the values a host
supplies. A list of positional arguments, of content or of batch entries measured as one value is
one more than its entries add to a call, since a call counts the entries and not the list.

### Validation against generated host values

The cost model's claim, that work which is not charged does not grow with the size of a value, is
checked mechanically as well as by review. `crates/nx-codegen/tests/cost/` holds a fixed library
of small NX functions and components, the *probes* (`probes.nx`), and a seeded generator of host
values (`mod.rs`). The probes perform on a host value every operation a runtime has for one:
holding it, passing it through a typed and an untyped parameter, binding it to a content
parameter, comparing it by `eq`, by a pattern and by `diff`, placing it in a sequence,
concatenating it and converting it to text, constructing a record that holds it as a property and
as content, capturing it in an action handler and comparing two such handlers, applying and
merging an update record that holds it, testing its presence, reading a member of it and iterating
over it, receiving it as a prop, as an action's payload and as an argument by name, storing it in
component state and patching that state, and returning it; and, where a host hands a call more
than it declares, receiving it as the content of a descriptor, as arguments by name the function
does not declare, and as members an invocation record does not define. `probes.nx` lists the
operations that have no probe, with the reason for each.

The generator draws a shape (a scalar, a string, a list, an object, a nesting of those, or one of
the names a runtime gives a meaning to) and then sizes weighted toward the boundaries the cost
model has: 0, 1, 63, 64 and 65 code units, a few hundred, a few thousand. Strings are drawn from
ASCII, two-byte and astral characters, so that code units and bytes differ. Objects have short and
long names, with and without a type name, shared or not shared between the two values of a case,
and lists and objects hold `null`s and empty lists. The reserved names are the type names
`nx.int`, `Function`, `ActionHandler`, `ActionHandlerInvocation` and one ending `.Update`, and the
key `$nxKind` with the TypeScript runtime's values, each with small and with large contents, in
the member the form defines and in one it does not. A case is a probe and its values at two
scales: a base, with a step repeated 16 times, and a larger one with the value eight times the
size and the step repeated 128 times. A shape reaches a probe only when both runtimes accept it
there: no `null` as a whole value at `object`, no `nx.int` record at a typed integer, a typed
probe only for a value that fits its type, and no nesting scaled past the 256 levels the Rust
runtime accepts. The same seed gives the same cases on every run, and the cases are generated
once and given to both runtimes as JSON.

Three things are checked of every case:

- **The runtimes agree** (`cost_differential.rs`, a blocking test). Both give the same operation
  count and input size at both scales, read from the usage report, and at the base scale the same
  result and, under budgets below the count, the same failure: its declaration, its span and the
  operations reported for the failed call, under every budget when the count is at most 200 and
  otherwise under a quarter, a half and one less. In each runtime the result under a budget equal
  to the count is the result under none. A case both refuse with no budget is compared by its
  diagnostic's code. Results are compared as the corpus compares them, and a case with a `null`
  inside a value at `object`, which the two runtimes return differently, is compared on its count
  and its failures alone.
- **The Rust runtime allocates in proportion** (`cost_allocation.rs`, a blocking test). The bytes
  a call requests are within a fixed multiple of its operation count plus its input size, and the
  bytes for each unit of that sum at most double between the scales. The second bound is what
  finds a copy that is not charged: such a step allocates sixty-four times as much at the larger
  scale for eight times the units.
- **Time follows the count** (the ignored test `time_report` of `cost_differential.rs`, which a
  job that does not block runs). A case is reported when a runtime's time, the least of five runs
  and of fifteen for a case that looks reportable, grows between the scales by more than twice
  the growth of its count plus its input size, and its time at the larger scale is past a floor
  set for that runtime from measured noise: 2 ms in the Rust runtime and 3 ms in the TypeScript
  runtime. The report repeats each step eight times as often as the blocking tests do, 128 and
  1,024 times, since a walk or a copy that is not charged takes a nanosecond or two for each
  value and shows beside the honest work of a call only when it is repeated several hundred
  times. At those scales a step of each kind review found, re-introduced in a runtime, is
  reported there: a copy made on every call, a walk of the whole state on every patch, and the
  lining-up of two wide records.

```bash
cargo test -p nx-codegen --test cost_differential   # the differential test
cargo test -p nx-codegen --test cost_allocation     # the allocation bounds
cargo test --release -p nx-codegen --test cost_differential -- --ignored --nocapture time_report
```

A failure names its seed and its case, after shrinking the case by size, and prints the command
that runs it again:

```bash
NX_COST_SEED=0x6e782d636f737431 NX_COST_CASES=45 cargo test -p nx-codegen --test cost_differential the_runtimes_agree
```

`NX_COST_SEED` and `NX_COST_CASES` also make a longer search by hand, best in an optimized build:

```bash
NX_COST_SEED=7 NX_COST_CASES=40000 cargo test --release -p nx-codegen --test cost_differential the_runtimes_agree
```

The Node runner, `runtime/typescript/test/cost-runner.mjs`, is the TypeScript runtime's side. The
harness starts it; with `NX_COST_DIR` naming a directory, the differential test keeps the images,
the cases and the answers there (and the time report under `time` in it), and the runner can be
run on them by hand:

```bash
NX_COST_DIR=/tmp/nx-cost cargo test -p nx-codegen --test cost_differential the_runtimes_agree
node runtime/typescript/test/cost-runner.mjs /tmp/nx-cost/cases.json /tmp/nx-cost/answers.json
```

A step these tests show to be uncharged is fixed in a change to the cost model, not in the tests.
Until then it is a *known finding*, an entry of `KNOWN_FINDINGS` in `tests/cost/mod.rs`: one
concrete case that shows the step, written out in full; a predicate saying which generated cases
it covers; what was found; and where it is tracked. The blocking tests skip the generated cases a
finding covers, run the finding's own case, and fail when that case passes its check, with a
message saying to remove the entry, so the list holds only what is still true and the change that
fixes a finding is the one that deletes it. A finding that only the time report shows is marked
in the report and is exempt from that rule, since time cannot be required to reproduce.

The differential test and the allocation bounds have found nothing: 120,000 generated cases under
three further seeds agree in both runtimes. The findings listed are the time report's, and none
is a step that is not charged. In the TypeScript runtime the time for each field of a wide object
rises about two and a half times between a few hundred fields and a few thousand, wherever one is
compared by name or written, whose names are sorted. In both runtimes the time for each value
written rises when a probe that writes its value on every repeat grows its result from thousands
of values to half a million and more: two to four times in the Rust runtime, and about twofold in
the TypeScript runtime, which the report names in some runs and not in others. Each is known to
the report for the runtime it was seen in only. All are
tracked in `openspec/changes/investigate-cost-time-report-findings`.

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

The prelude is the one module no host has to supply. Each runtime — the `@nx-lang/ir-runtime`
package and the `nx-ir-runtime` crate — ships the
compiled image of the prelude its release was built with, and linking serves it for the prelude's
reserved identity whenever the host's resolver returns nothing for that identity — at any depth of
the link, including for a module the resolver did supply. The built-in prelude is prepared at most
once and reused across linked programs, so a self-contained program is one whose only other module
is the prelude. Because the prelude's table entry carries a version, a package whose built-in prelude
is a different contract from the one the image was compiled against is reported as
`nx-ir-link-version` rather than misbehaving at evaluation — and a host that supplies its own prelude
is checked the same way. A host that wants its own prelude returns a prepared module for that
identity, and linking uses it instead. Another runtime gets the image by naming the prelude's
identity in an emit request.

A standard library module is not supplied this way. No runtime carries its image: the host emits it
with the program's other images and its resolver returns it, as for any library module, and a
resolver that returns nothing for `@nx/agent/agent.nx` fails the link with
`nx-ir-link-missing-module`.

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
is 848 bytes. Cells are shown as little-endian values, sixteen bytes per line, offsets in hex.

```
0000  "NXIR"  5         350       6           header: magic, schema 5, 848 bytes, 6 sections
0010  0       58        118       1           directory: strings at 0x58, 280 bytes; module …
0020  170     38        2         1a8         … at 0x170, 56 bytes; types at 0x1a8 …
0030  8       3         1b0       28          … 8 bytes; constants at 0x1b0, 40 bytes
0040  4       1d8       140       5           nodes at 0x1d8, 320 bytes; declarations …
0050  318     38                              … at 0x318, 56 bytes
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
0190  12      fa81ea66  3767a802  1           … module 1: "drawnui.nx", "9", fingerprint 3992344325433453158
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
0318  2       0         4         a           count 2; offsets 0, 4, 10
0328  1       0         0         ffffffff    declaration 0: value "title" = node 0, no declared type
0338  0       2         0         d           declaration 1: function "root", no params, body node 13,
0348  ffffffff 0                              …        no declared result, not optional
```

`nxlang ir explain` prints the same thing in words:

```
module input.nx fingerprint 2080198121860257358
links drawnui.nx version "9" fingerprint 3992344325433453158
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

Every evaluation function takes options: `maxCallDepth` (100 by default), `maxRangeLength` (one
million), `maxOperations`, the budget of *Evaluation cost*, and `maxInputSize`, the limit of
*Input size*, each of the last two unlimited unless set, and `usage`, an object the runtime reports
what the call used to. One budget covers one call, a dispatch's whole batch and the render after it
included. Expressions nest at most 1,000 deep, and a `RangeError` the JavaScript engine raises
during evaluation is reported as a diagnostic. Every `nx-ir-resource-limit` diagnostic carries
`limit`, the name and value of the limit reached: `maxOperations`, `maxInputSize`, `maxCallDepth`,
`maxRangeLength`, `maxExpressionNesting`, or `engine`, which has no value. A diagnostic for a
failure in a value the host passed as an argument of the function it called, through
`callFunction` or `evaluateFunction`, carries `argument`, the name of the parameter the value was
passed for, and so does one for a required parameter given nothing; a failure a default raises, a
resource limit and a failure in the function's body or result carry none, even when the runtime
finds them while it checks an argument. A value the host passed is read as *Host values*
describes before any of it is checked: one that is not canonical is refused, in the argument it is
in when it is in one, and a member that is `undefined` is left out. `measureInputSize` measures
one value as the input limit does. The package's `README.md` has the details.

## Rust runtime

The Rust runtime is the crate `crates/nx-ir-runtime`. It depends on the format crate `nx-ir` and
on `nx-value`, and on no part of the compiler, and offers the operations above as methods of a
linked `Program`: `PreparedModule::prepare`, `Program::link` and `Program::prepare`, then
`evaluate_function`, `call_function`, `construct_component_descriptor`, `initialize_component`,
`evaluate_component`, `dispatch_component_actions`, `normalize_component_state` and
`apply_component_state_patch`, with `apply`, `merge`, `diff` and `Program::changed` for the update
intrinsics. Values cross the API as `NxValue`, which is the Rust form of a canonical value (see
*Host values*), and every operation returns a `Result` whose error carries diagnostics with the
TypeScript runtime's codes.

It differs from the TypeScript runtime in what Rust makes possible or necessary:

- Integers are 64-bit. An integer outside JavaScript's safe range is an `NxValue::Int`, and it
  computes: `9007199254740992 + 1` is `9007199254740993`. The TypeScript runtime cannot hold such
  an integer: it refuses one where it reads it from an image, with `nx-ir-number` naming the
  function and the literal, so a program that reaches one runs here and fails there. Integer
  `add`, `sub`, `mul` and `mod` wrap at 64 bits, where a JavaScript number loses precision past
  2^53 instead. Inside the safe range the two runtimes agree.
- The `nx.int` wrapper, canonical JSON's spelling of such an integer, is that integer to the Rust
  runtime at a typed integer site, where the TypeScript runtime refuses it. At `object`, where no
  type says what it is, the wrapper is a record with one field in both runtimes, as a host that
  read it from JSON holds it: written back it is the same JSON, and it costs what a record with
  one string field costs. A Rust host that means the integer passes `NxValue::Int`, which is
  written to JSON as a number and read back as one.
- The host's `null` is the empty value before any type is known. At an `object` site, which holds
  any value, a `null` is therefore the empty list, at the top level and inside an untyped value,
  where the TypeScript runtime rejects the first and keeps the second. Either way it is one value
  where it is written or bound, so the two count such a value alike.
- An instance serializes, and `Program::restore_component_instance` validates a stored one against
  the program once, when it is restored. An instance belongs to the exact images that rendered it,
  compared by a hash of each image's bytes, since a handler names its node and its captured slots
  by index.

And because a native stack overflow cannot be caught, evaluation is bounded by a fixed
expression-nesting limit and a stack budget as well as by the host's call-depth limit.
`RuntimeOptions::max_operations` is the operation budget, counted exactly as the TypeScript runtime
counts it: the same images, input and budget stop at the same node.
`RuntimeOptions::max_input_size` is the input limit of *Input size*, measured as the TypeScript
runtime measures it, so the same input is refused under the same limit; `RuntimeOptions::usage`
reports what a call used; and `input_size` and `record_input_size` measure a value and a map of
named values as the limit does. A resource-limit diagnostic's `limit` uses the TypeScript
runtime's names for the limits the two share, and adds `maxStackBytes` and `maxValueNesting` for
the two only this runtime has. A diagnostic's `argument` is the TypeScript runtime's: for the same
image and the same arguments the two name the same parameter, or both name none. The crate's
`README.md` has the API, the limits and the diagnostic codes.

## Conformance corpus

`specs/ir-conformance/` holds NX programs with their expected images, with and without the debug
section, the explained text of each image beside it, the expected canonical value of every named
entrypoint, and, for every lifecycle a program names, the rendered output of initialization and
the rendered output and effects of each dispatched batch, tokens included. Each program's
`operations.json` records what every entrypoint, initialization and batch costs in operations, and
`evaluation-cost`, which has one entrypoint per charging rule, also records where budgets below
those counts stop each entrypoint: a declaration, and a span when the charge was for a node. An
entrypoint may name the arguments to evaluate it with, as a *case*, and `host-values` holds the
cases an entrypoint without arguments cannot supply (a list at a sequence parameter, a value at
`object`, long text and a long name, two objects compared, a list bound to a content parameter and
the JSON form of a wide integer) with the input size of each, which every runtime checks as it
checks a count, at the size and at one less. A case may be marked as one that fails: the code of
its diagnostic and the argument the diagnostic names, as the Rust runtime reports them, are then
recorded in `diagnostics.json` in place of a result, and every runtime must fail the case with
that code and name that argument, or none. `argument-diagnostics` holds those cases, and a case
whose recorded budget runs out while its argument is checked, a failure that names none. The
emitter's tests pin
the images byte for byte and check that each committed text is the explanation of its committed
image; the TypeScript runtime's tests and the Rust runtime's tests each evaluate the images, drive
the lifecycles, check every count by evaluating under it and under one less, and refuse every
truncation of them and every cell overwrite of the small ones; and the corpus is where another
runtime starts. It covers
every node, type and declaration kind, a program spanning two images, derived declarations, a
document that is a single trailing element, and components that bind action handlers. Two of its
programs are of the size a host runs, a snippet against a catalog of 45 external components and a
question flow with state over a library of 31 question kinds, and they are what the TypeScript
runtime's performance harness times. It also holds
the size budget: an image emitted without its debug section is at most six times the UTF-8 length
of its module's source.

## Non-goals

NX IR is eager. Every operand is evaluated before its parent and exactly once, except in the
branches of `if` and `ifIs` and in the right operand of `and`, `or` and `coalesce`, which its left
operand may skip. Nothing in the encoding mutates a value or reassigns a binding, and a node's
children precede it in the table, so a runtime is free to memoize by node index: only a changed
input can invalidate a cached result.

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
