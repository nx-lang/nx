## Context

The design-tokens doc (https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG) set the goal: in Visual, a
property of type T offers every reachable named value of type T, environment values first, then
exported `let`s, record fields and user aliases such as `let brandRed`; choosing one writes the name
into the source. Bret's rule: tools must not offer only tokens.

What exists:
- `sourceTree` (source-tree spec) gives the viewer nodes, a declaration table with each property's
  type spelled in NX, and, with `visualizers`, each property's label, format and editor hints.
- The inline inspector (#72) shows each property as a labeled field. A reference to a top-level `let`
  reads as a token, `{blue}`, with a swatch when the `let` is a literal color, and its menu offers Go
  to and Show in Source.
- `sourceEdit` (source-edits spec) writes typed values (`string`, `color`, `number`, `boolean`,
  `case`) at a literal or bare case, and adds properties.
- Static color discovery (color-tooling spec) recognizes hex literals and `@nx/color` constructors
  with literal numbers, without running the program.
- `@nx/design` types its tokens with aliases (`Spacing`, `Radius`, `FontSize`, …) whose doc says
  they exist so tools can tell spacing from radius. `@nx/graphics` still types its properties as
  `float64 where …`.

## Goals / Non-Goals

**Goals:**
- One mechanism for every named value, tokens included, with tokens first.
- Suggestions that fit: padding is offered spacing, not radii or every number in the program.
- Opening the list is instant: no round trip after the reader asks.
- Choosing a value writes the name, so the field stays bound to it, and the reader can always get
  back to a plain value.

**Non-Goals:**
- Values from modules the document does not import (which would add the import). A follow-up.
- `for` bindings and local `let`s inside a body, and items of a list property.
- Previews that follow the main module's `provide`s or the host's root values.
- Element-typed properties (templates, phase 6 of Visual editing) and functions.
- Changing the language or the runtime.

## Decisions

### Suggestions travel with the source tree

`sourceTree` takes `suggestions: true`, and the answer carries one list, `suggestions`, for the
whole document. Each property in the declaration table (record, component and case properties, and
state fields), and each parameterless `let` this document declares, carries a `suggestionKey` when
it has suggestions; the viewer shows the entries with that key. A declaration listed only because a
suggestion starts from it brings no keys of its own, so the list has a fixed point. `scope` and
`self` name declarations by their table index.

The set of candidates depends only on the module's imports and, for parameters and state, on the
enclosing top-level declaration, never on which property is asked about. So one list per document,
filtered by key and scope in the viewer, holds everything, and the popover opens without a query.
The list is small in practice: a program that imports `@nx/design` has about 120 leaf tokens, the
palette's 55 colors and the program's own values.

*Alternative:* a separate `valueSuggestions(uri, key, property)` query, called when the popover
opens. It sends less, but the viewer is a web component that only receives trees and fires events;
a query would need an async callback from host to viewer, a loading state, and a stale-answer rule.
Rejected for now; it is the way to go if lists grow large, and the shape of an entry would not
change.

### Match by declared type, not by base type

A property's suggestion key comes from its declared type, with `?` removed:

- **An alias of a primitive**, such as `Spacing` (`float64 where …`): the key is that alias. A
  candidate matches when its declared type is that alias or an alias that names it through a chain
  (`type ButtonGap = Spacing` matches `Spacing`). So `padding: Spacing` gets `env.space.*` and not
  `env.radius.*`, though both are `float64`.
- **A record, union or action type**, reached directly or through aliases (`Color`, `Paint`,
  `Shadow`, `Size`): the key is that type. A candidate matches when its type is that type, or a
  record the type checker accepts there through inheritance. `@nx/design`'s `Color` and `@nx/color`'s
  `Color` are one type, so they share a key.
- **A bare primitive** (`int`, `float64`, `string`, `boolean`, `uitext`), or one constrained inline:
  no key, no suggestions.
- **An element, function, sequence or object type**: no key in this change.

Keys compare declaration identity (module and name), not the spelled name, so a user's own type
named `Spacing` is not `@nx/design`'s.

This is why the alias names exist (types.nx: "the names let tools tell spacing from radius"), and it
keeps the list short. A value without a declared type matches by its inferred type: `let brandRed =
#e11d48` is a `Color`; `let gap = 12` is an `int` and suggests nothing, so the docs tell authors to
write `let gap: Spacing = 12`.

*Alternative:* match anything assignable. For numbers that offers every numeric value in reach,
radii for padding and font weights for widths. Rejected. *Alternative:* also offer, under a quiet
"Other numbers" group, values of the base type. Left for later if readers miss them.

### What is reachable, in what order

Groups, in this order:

1. **Environment**: each environment value the module can see (its own, its library's internal ones
   and every imported module's exported ones), spelled `env.name`, then its record fields,
   `env.color.text.muted`. An ambiguous name, which the module could not read, is left out.
2. **The enclosing declaration**: the parameters, and a component's state fields, of the top-level
   component or function the value is written in, with their record fields. Labeled with its name.
3. **This document**: its top-level parameterless `let` values, with their record fields.
4. **Each peer file**: other files of the same library, and other root modules of the same
   program, whose non-private values the module sees without an import (declaration-visibility).
5. **Each imported module**, in import order: the `let` values it makes visible, spelled through the
   import's alias, or a selective import's qualified alias, when it has one. Labeled with the import
   as written, such as `@nx/design` or `./brand.nx`.

A name is offered only under a spelling the module's top-level scope resolves without a
diagnostic, so ambiguous and shadowed names are left out.

Within a group, entries follow declaration order, and fields follow their record's field order, so
scales read small to large as the library writes them (`none`, `xxs`, … `xxxl`).

Fields are followed through records only: not through optional fields, sequences, unions or
elements' content, and at most four levels deep, so a recursive type cannot loop. An intermediate
record is itself an entry when its type matches (a `TextColors` property gets `env.color.text`).

A value is not offered where choosing it would make a cycle: inside `let brand = …` no `brand.*`
entry, inside `environment color = …` no `env.color.*`, and inside `environment font = …` no
`env.text.*`, since `text`'s default reads `env.font`. Since the list is one per document, each entry
lists in `self` the declarations it reads, directly or transitively, as hover's environment-read
list follows them, and the viewer leaves it out inside those. Functions, element
functions, components and `env.Update` bundles are not values for this purpose.

### Previews are found statically

An entry carries a preview when its value at the defaults can be found without running the program:
follow the name to its declaration's value or default; follow a reference to another `let` or an
`env.` read; follow a field into a record or element construction's attribute (or the field's
default when the construction leaves it out); and take, for an `if env.x is { … }` or a match on an
environment value whose own default is a bare case, the arm for that case. Stop at a literal (a
number, string, boolean or bare case), or at a color the color-tooling discovery recognizes, and
give that. Stop with no preview at anything else, or after 16 steps. Color discovery today runs while
the requested document's nodes are built; it is factored so it works on any module's syntax and
resolves constructors through that module's imports (`palette.nx` writes `c.oklch(…)`). A preview
color uses the node's existing swatch text and outside-sRGB mark, so the viewer draws swatches one
way.

`env.color.accent` previews as `standardPalette.blue600` followed to `c.oklch(0.546, 0.245,
262.881)`, in the light arm because `colorMode` defaults to `light`. A preview is the default
theme's value, not what a main-module theme or a dark section makes it; the popover says so in its
footer ("Previews show default values"). Following provides is a later change.

*Alternative:* evaluate entries with the IR runtime. The language service does not depend on the
runtime, previews must not run arbitrary code (as color discovery doesn't), and evaluation would
cost each tree. Rejected.

### Picking writes `{name}`; a token is replaced through its value

A new `sourceEdit` value kind, `{ kind: "name", name: "env.space.md" }`, is accepted by `setValue`
and `addProperty`. It is spelled braced at an unbraced place, an attribute or a `let` value
(`padding={env.space.md}`, `let accent: Color = {blue}`), since the grammar takes no dotted name
there and reads a bare one as a case, and bare inside braces or as a sequence item. Each
dot-separated part must be an identifier and not a keyword, and `env` needs a member. As for other values, the service does not check the type or that the name resolves; the
type checker reports a wrong one as it would a hand edit.

`setValue` today targets only a literal or a bare case. A token must be replaceable by another token
and by a literal, and an element value such as `fill=<Paint.color color=#2563eb />` by a token, so
the target widens to any whole value written as a literal, a case, a name (a reference or a member
chain on one, `env.` reads included) or an element. Undo is the host editor's, so replacing an
element is recoverable. Today a hex literal takes only a `color` value, and a `color` value on a
constructor call or color element edits its channels in place; both stay, and a `name` replaces any
of them whole, while a `color` over a name writes a hex literal.

In the viewer, a token reads as `{blue}` does today, now also for `env.` reads and member chains. A
click on a token opens the named-values popover, so swapping tokens is one click and one choice. Its
`⋯` menu gains **Replace with value**: it opens the property's own editor (number field, color
picker, case popover) on the token's preview, and committing writes that literal over the token,
which is Figma's "detach". The default row (as in the choices list) leaves the property out.

For a color, color-tooling never rewrites a wide color as sRGB, and the palette is OKLCH. So Replace
with value is offered for a color token only when its preview is inside sRGB, and writes hex rounded
to 8 bits, as the picker's hex is. A wide token keeps its name; the reader can pick another or write
it in Source. Writing the exact `oklch(…)` form would need `@nx/color` in scope and is a follow-up.

### The button and the keyboard

A quiet named-values button follows a field's value, before its `⋯`, shown while the property is
under the pointer or selected or its editor is open, and on unset properties too: Figma's model,
where the variable button appears beside a field on hover. It is not a Tab stop, so Tab still moves
field to field. Ctrl+Space, the usual "suggest" key, opens the popover from a selected property or an
open field, and so does Alt+Escape, since macOS may take Ctrl+Space for switching input sources (VS
Code offers Option+Esc there for the same reason). The host draws the icon by name (`named-values`), with a built-in fallback, as row
icons work.

The popover is a combobox: a search field over a list with group headings, each row reading the
name, its `env.` or module alias quieter, and its preview after it in a second column (swatch and
hex for a color, the number, the case). Typing filters rows whose name holds every typed word,
ignoring case and treating `.` as a space. Up and Down move, Enter chooses, Escape closes. The list
scrolls inside the popover, headings sticky.

*Alternative:* put named values into the existing choices list under number and string fields. That
list exists only for some fields, cases and colors have their own popovers, and record values have
none, so the feature would look different on each kind of field. Rejected in favor of one popover.

### `@nx/graphics` names the design types

Suggestions need declared types, and graphics is what the playground's Visual examples edit. So
graphics properties take `@nx/design`'s aliases, through `import "@nx/design" as design` so that
`@nx/design`'s `Color`, `Shadow` and the rest don't join graphics' own names, where the meaning
matches: `Stroke.width:
BorderWidth`, `Text.fontSize: FontSize`, `Text.fontFamily: FontFamily`, `Rectangle.rx` and `ry:
Radius`, and every `opacity: Opacity`. Constraints and defaults stay the same. `Text.fontWeight` stays
`float64` (the graphics spec keeps it so), and geometric radii (`Circle.radius`, `Star.outerRadius`)
are not corner radii and stay plain.

`@nx/graphics` then imports `@nx/design`. A graphics program still sees no environment values unless
it imports `@nx/design` itself, since imports are not transitive.

## Risks / Trade-offs

- **Tree size.** Every tree refresh carries the list. → Only entries whose key some property or
  `let` of the declaration table uses are sent, and entries carry no ranges. Measure on the corpus;
  switch to a query (the alternative above) if a list passes 500 entries.
- **Previews can mislead** under a theme or in a dark section. → The footer says previews are
  defaults, and the hover card of a token shows its declaration and doc.
- **Two `Color`s in scope.** A program that imports `@nx/graphics` and `@nx/design` without aliases
  sees two exported `Color` aliases (of the same type) and gets an ambiguous-name diagnostic on use.
  → The tutorial imports one under an alias; making the two re-exports count as one name is a
  separate fix.
- **A user alias of a token type gets only its own values.** `padding: ButtonGap` with
  `type ButtonGap = Spacing` is not offered `env.space.*`. That keeps a narrower role narrow; if
  readers miss the base role's values, add them as a quieter group later.
- **Matching by alias misses untyped values.** `let gap = 12` is not offered for `Spacing`. →
  Documented; the style guide asks for declared token types.
- **`@nx/graphics` gains a dependency.** It is unstable and the aliases are transparent, so no
  program breaks; corpus images that embed graphics regenerate.

## Migration Plan

None. Without `suggestions` the source tree answer is unchanged, and the new edit value is additive.

## Open Questions

- **The button's icon**: Figma's four-diamond "variable" glyph, a hexagon, or `{}`? I'd use a
  hexagon, which reads as "token" in Tokens Studio and Penpot.
- **Group labels**: `Environment` or `Tokens`? I'd say `Environment`, since not every environment
  value is a design token and the language calls them environment values.
