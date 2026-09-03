# What building this found

Every item here was found by trying to build the app, not by reading code. Each has a stable
number so it can be cited from a proposal, a commit, or another finding.

Seven were fixed in this change: three in the TypeScript IR runtime, which rejected values the
language itself produces (F1–F3); three in type checking, which never looked inside a component
body (F5, F7, F19); and one in the grammar, which rejected an element with an empty body (F20). The
rest are worked around here; each is a candidate for its own change. The last group is not bugs at
all — behavior that shaped the catalog and is worth knowing before writing NX against it.

Unless a finding says otherwise, the Rust interpreter is the reference: where the two runtimes
disagree, the interpreter is the one that matches the language.

## Fixed in this change — `runtime/typescript`

The TypeScript IR runtime rejected values the language itself produces. All three are the same
shape, and all three are aligned with the interpreter rather than newly invented.

### F1 — A single child of a list-typed content property was not a list

`<SkiaScroll><SkiaStack/></SkiaScroll>` — the opening shape of nearly every DrawnUI page — failed
with `Expected props.Children to be an array`, because content binding collapsed one child to the
child itself. Content binding now respects the content property's declared type.

### F2 — A record value could not normalize into a record-typed field

Record construction stamps a `$type` discriminator, and normalization then reported that
discriminator as an unknown field, so `Padding=<Thickness Left=4.0 />` failed — as did every
record-valued property in the catalog, which is to say `Padding` and `Margin` on every control. The
union branch beside it already discarded its discriminator; the record branch now does the same.

### F3 — A single value at a list-typed property was not coerced to a list of one

`Shadows={ <SkiaShadow Y=6.0 /> }` and `xs={3.0}` are one-element lists under the interpreter, which
performs the coercion during normalization — the IR records the value at its own type. The
TypeScript runtime failed instead. This is the general rule that F1 is one case of.

## Fixed in this change — `crates/nx-types`

Type checking walked `Item::Function` and skipped `Item::Component`, so a component body was never
inferred. Three findings fall out of that one line, and out of fixing it.

### F5 — A bare contextual union case could not be emitted to IR from a component body

`HorizontalOptions=Center` evaluated under the interpreter and generated correctly inside
`let root()`, but inside a component body code generation failed with `unresolved contextual name
cannot be emitted`. The same limit applied to prop defaults, where `= {LayoutType.Absolute}` was the
only working spelling.

A contextual name has no type of its own; inference resolves it against the declared type of its
binding site, and those resolutions are what rewrite the bare name into the qualified case before
anything downstream sees it. A body that is never inferred keeps its bare names, and codegen has
nothing to emit them as — which is why the diagnostic reads as an internal error rather than as
something an author did. Component bodies are now inferred, with props and state bound by name the
way a function's parameters are.

```nx
component <A extends Node /> = { <Paint colour=Red /> }        // now emits Hue.Red
component <B extends Node hue:Hue = Green /> = { ... }         // and so does a default
```

The examples write the bare form everywhere now. Across all twelve, the emitted IR is identical to
what the qualified form produced apart from source spans and slot numbering, which is what makes
that rewrite safe.

### F7 — A property type mismatch inside a component body was not reported

The same cause, and the same fix: an uninferred body is an unchecked body. Passing a case of the
wrong union was rejected at the top level and accepted inside a component body, failing only at
runtime. This cost real debugging time on the Layouts port: the example compiled with no
diagnostics and then refused to draw.

```nx
type Alpha = Red | Green
type Beta = Red | Blue
external component <Paint colour:Alpha? />
abstract external component <Node />
component <Wrapper extends Node /> = { <Paint colour={Beta.Red} /> }   // now rejected
let root() = { <Paint colour={Beta.Red} /> }                          // already rejected
```

A prop or state default is now checked against its declared type as well, under
`component-default-type-mismatch`.

### F19 — A record field had no type

`u.name` on a record-typed binding reported `Member access not yet implemented: .name`. Inference
had an arm for a union and for a union case and none for a record, so every read of a record field
was an error — in a function body as much as in a component body.

It surfaced the moment F5's fix started inferring component bodies, because that is where the
examples read record props: `Text={Item.Title}` in `ContactCell` is the shape. Fixing F5 without it
would have broken the examples F5 was blocking, so both are here.

A field is resolved through the record's effective shape, so an inherited field is found as readily
as a declared one, and each field's type is resolved in the module that declared *that field* rather
than the module the record came from. An unknown field now names the fields that exist.

A **nullable** base reads its field the same way, which was not true at first. `Item:Contact?` with
`Item.Title` in the body is the natural thing to write against a catalog whose every property is
optional, and it reported `Member access not yet implemented: .Title` — nullable was unhandled for
records and unions alike, and had been all along, but nothing reached it while component bodies went
unchecked. NX has no narrowing construct to discharge the null with, so demanding one would make a
nullable record or union prop unreadable rather than safer; the base is unwrapped and the field's own
declared type is returned, not a nullable of it, since a `string?` would fail at every `string` site
downstream. The looseness is deliberate and matches what the interpreter does at runtime.

## Fixed in this change — `crates/nx-syntax`

### F20 — An element with an empty body was a syntax error

```nx
<SkiaLayer VerticalOptions=Fill>
</SkiaLayer>
```

An opening tag closed immediately by its own closing tag did not parse. The `element` rule required
body content, and body content is `repeat1`, so nothing stood between `>` and `</`. Body content is
now optional, and an empty body means what a self-closing tag means: the declared content property
is left unset. Supplying content is unchanged — a target declaring no content property still accepts
`<Plain></Plain>` and still rejects `<Plain><Kid /></Plain>` — and a closing tag naming a different
element is still reported as a mismatch.

The defect looked positional and was not. It was found while trying to write a file as a single
trailing element rather than as `let root() = { ... }`, which NX has always allowed, so the first
reading was that the top-level form was broken. It was not: `<App><Header /></App>` parsed fine at
the top level all along, and `let root() = { <App></App> }` failed just as hard. What the two cases
had in common was the empty body. Whitespace and comments do not count as content, so the shape an
author actually writes — an open tag, a blank line, a close tag — hit it too.

**The app's compile arrangement changed because of this.** A file may end in a single bare element,
and the grammar allows that element only as the file's **last** item. The server used to append the
catalog to the visitor's source, which put declarations after that element and made the form a
syntax error in the fiddle even once the grammar was fixed. The catalog now goes first, and
`classify` in `server/compile.mjs` subtracts the catalog's leading lines and bytes so a diagnostic
still reports the line and column the visitor sees. Columns need no adjustment, since the catalog
contributes whole lines. Appending had been chosen precisely to avoid that arithmetic; the
trailing-element form is what made it worth paying. F4 records the arrangement; F9 records the one
behavior it changes, where a name declared in both the catalog and the visitor's source now resolves
to the visitor's rather than the catalog's.

The examples are written in the trailing-element form now. Across all twelve, the emitted IR is
identical to what `let root() = { ... }` produced apart from source spans, slot and declaration
numbering, and the embedded source text with its fingerprint — all of which move when the text
moves. That is what makes the rewrite safe; a bare trailing element lowers to the same synthesized
`root` entrypoint, so nothing downstream can tell the two forms apart. Note that dedenting a body
into the trailing form is not a whitespace-only edit: a newline in a string literal is a newline in
the value (F11), so the continuation lines of a multi-line string must keep their exact columns.

## Compiler and analysis gaps

### F4 — Cross-module external components lose defaults and inherited props

Documented as NXE12/NXE13 in `docs/drawn-ui-proposal-nx-enhancements.md`. A workspace-sibling
import (`import "./skia"`) did not resolve at all. The app compiles the visitor's source and the
catalog as a single module, with the catalog first — see F20 for why that order, and for the
position arithmetic it costs.

### F6 — The two runtimes serialize union cases differently

For the same program the Rust interpreter emits `"Type": "Column"` and the TypeScript IR runtime
emits `"Type": {"$type": "LayoutType.Column"}`. The renderer unwraps the second form using the
generated catalog metadata. Whichever is chosen, they should agree.

### F8 — An unresolved type name is not reported by analysis

A type annotation naming a type that does not exist passes analysis and evaluates:

```nx
let x: NoSuchType? = null                        // evaluates to null, no diagnostic
external component <Box value: NoSuchType? />    // evaluates, no diagnostic
```

Code generation does reject it, with `type binding 'NoSuchType' is unavailable` — but only when the
whole program is built, and without pointing at the annotation (see F10).

### F9 — A duplicate declaration is silently accepted, and the last one wins

Two declarations of the same component name produce no diagnostic. The second shadows the first,
and using the first's property then gives a misleading error about the property rather than about
the duplicate:

```nx
external component <Box a:string? />
external component <Box b:string? />
let root() = { <Box a="first" /> }   // Element 'Box' has no property 'a'
let root() = { <Box b="second" /> }  // fine
```

This matters here because the catalog is concatenated with the visitor's source. Since the catalog
comes first (F20), a visitor who declares `SkiaLabel` silently replaces the catalog's rather than
being replaced by it: their own properties work, every catalog property on that tag is suddenly
unknown, and the diagnostic they eventually see names a property rather than the collision. The
shadowing runs the other way from how it read when the catalog was appended, which is a little
kinder to the visitor and no less confusing.

### F10 — Whole-program code generation failures carry no position

`codegen-missing-semantic-data` (F8's diagnostic, among others) arrives with a zero-width span at
line 1, column 1 rather than a location. Marking that in an editor would assert the error is on the
author's first line, which is a lie, so the app classifies such diagnostics as `program` and reports
them without a position.

## Syntax gaps

### F11 — String escapes are scanned but not decoded

`"a\nb"` is six characters with a literal backslash; `\t` and `\"` behave the same way, though `\"`
does stop the literal from ending. String literals may span lines, and a newline in the source is a
newline in the value, so the examples write multi-line text that way and use single-quoted
attributes in embedded XML.

### F12 — A bracketed list literal is not accepted in a `for` header in content position

`for label in ["a", "b"] { ... }` inside an element fails with `Invalid element syntax`; binding the
list to a `let` first works. Both forms are fine outside content position.

### F13 — An empty list has no spelling

`{}`, `{ }`, `[]` and `{[]}` are all syntax errors, so a list-typed property cannot be defaulted to
empty. Already proposed separately as the `empty-list-spelling` change; recorded here because the
catalog generator ran into it, and it is part of why the catalog declares no defaults.

## Not bugs — behavior worth knowing

### F14 — `Element[]?` does not accept an external component as content

A content property typed `Element[]?` rejects an external component child at runtime with
`expected Element, got Leaf`. Content in this catalog is therefore typed as a list of a declared
abstract external component. Worth noting because `docs/drawnui-proposal/ui/ui.nx` types its content
properties as `Element[]?`, which would not accept the components it is meant to hold.

### F15 — A wrapper's content property cannot be optional if it splices it

Declaring a wrapper's content `DrawnNode[]?` and splicing `{Children}` among literal siblings fails
with `expected DrawnNode[]?, found list object[]`; declaring it `DrawnNode[]` works. Catalog
components, which do not splice, are fine either way.

### F16 — A user-defined component must extend the catalog's node root to be content

Content is typed as a list of `DrawnNode`, and a plain `component <Card ... />` is not one, so every
wrapper in the examples is declared `component <Card extends DrawnNode ... />`. A consequence of how
the catalog is shaped rather than of NX, but it is the first thing anyone writing NX here hits.

### F17 — An unknown element name is not a compile error

`<NotAControl />` compiles: unknown tags are treated as intrinsic elements. A mistyped control name
therefore surfaces in the renderer as an unknown type rather than as a diagnostic, which is why the
renderer draws a visible placeholder and reports it instead of failing.

### F18 — Only abstract components may be extended

`external component <Leaf extends Middle />` where `Middle` is concrete is rejected with *only
abstract components may be extended*. Stated behavior with a clear diagnostic, not a gap — but it is
why the catalog emits an abstract twin (`SkiaLayoutBase`) for every control that is both a tag and a
base, and why a subclass may not restate an inherited prop. See `CATALOG.md`.
