# Future Considerations: Environment Values, Design Tokens, Color And Named Values

Follow-ups for environment values (`env.`, `provide`), the `@nx/design` tokens built on them, the
`@nx/color` wide-gamut colors they use, and the named values Visual offers for them. Other future
work is in [`future.md`](future.md).

## Tools: The Playground And Visual

Ideas from `add-visual-value-suggestions` (PR #79) and its follow-up discussion, in rough order of
value.

### An Environment panel in the playground

The playground has no way to change an environment value without editing source, so trying a theme
in dark mode means writing a `provide`. A panel would show a control for each environment value the
program can see: a segmented control for a union such as `colorMode`, a toggle for a `bool`, a
stepper for a number, a field for a string and a color picker for a color. An unset control falls
back to the default, and one action resets them all. A change sets the host's root values and
renders again without reloading; the Rust runtime already tracks root-value revisions
(`crates/nx-ir-runtime/src/kept.rs`, `tree.rs`). A value the host refuses shows its error in the
panel. The values are carried in the URL, so a link opens the same preview, and each change is a
timeline entry.

### Where a value comes from, in Visual

For a selected element, list each environment value in effect and where it was set: its default,
a host root value, the main module's `provide`, a `provide` block, or an `env.` attribute at line N,
each linked to its source. It works like computed styles in browser devtools and pairs with named
values: it says why a token shows what it shows.

### Named values in context

The named-values popover previews each name by its default and says so in its note ("Values shown
are defaults, before any override"). Previewing the value in effect at the property's place,
inside the `provide` blocks around it and under the host's root values, would make the note
unneeded or let it say which values differ. A running preview session knows these values, so the
viewer could ask it (see "Previews that follow `provide`s" below).

### Small changes to the named-values popover

- **A key hint in the footer.** "Enter to bind · Ctrl/⌘+Enter to write the value", with
  Ctrl/⌘+Enter doing Replace with value for the active row. Bret approved it during PR #79; it
  did not land, and main has neither the hint nor the shortcut.
- **A tooltip that names the shortcut.** The hexagon button's tooltip says only "Named values";
  "Use a named value (Ctrl+Space)" would make the shortcut discoverable.
- **Color tokens ordered by role.** The Environment group lists colors in declaration order.
  Ordering them by role (surface, text and border; then accent; then danger, warning and success),
  each beside its `on*` partner, would read like a design system's color page.
- **An empty strip under a `let`'s list.** After the hexagon moved into a `let` row's head
  (c42162d), the `let` popover under `outline.paint` showed an empty strip at its bottom. It may be
  gone since the detail lines are left out when nothing fills them; check it.
- **A width that fits a narrow pane.** The popover is at least 22rem wide, so in a pane narrower
  than that it runs past the pane's edge. Its minimum could give way to the room it has, with long
  names cut as they already are.
- **A resizable popover.** A drag handle like VS Code's suggestion widget, only if fitting the
  popover to its content proves not to be enough.

### A light and dark example

The playground's examples read tokens, but none is built around the environment. An example where
a `provide env.colorMode=dark { … }` flips one section, beside the same content in light, would
show how overrides scope.

### A warning for a provide block after prose

In element content, a `provide env.…` that follows prose on the same text run is read as more
prose and a braced value, as bare `if` and `for` are (PR #81), so its overrides are lost without a
diagnostic. Validation could warn when content text ends in `provide env.` and is followed by a
braced value.

## Environment Values: What `add-environment-values` Left For Later

### Limitations of the first version

- **A handler captures every override around it, not only the values it reads.** A handler whose
  body can read the environment captures all the overrides of the provides around it
  (`handlers-keep-local-provides`). Under a provide whose value changes, it is unequal across
  renders even for a value it never reads, so the children that see it render again. Narrowing the
  capture to the values the body can read would keep them.
- **An emit dispatched through the child applies a self-reading override twice.** In the bare
  lifecycle API, a host that dispatches an emit against the child, in the child's environment,
  applies the overrides on the child's descriptor and the parent handler's captured overrides one
  after the other. An override that reads the value it sets, such as `env.space={env.space * 2}`,
  then applies twice. Dispatching the parent's token against the parent, as the instance tree does,
  is exact. The fix is for an instance to carry the environment it was initialized in, so the
  runtime can run a parent-bound handler in its parent's environment.
- **A place formula that reads a value the body overrides is never skipped.** When a node's place
  sets a value with a formula, and that formula reads a value the node's own body overrides, the
  value the render saw cannot be compared with a lookup at the place. The instance tree records the
  read as unchecked, so the node renders again on every visit (`add-tracked-reactivity`). Every
  other cascaded read records its direct inputs and is skipped precisely.
- **Restored instances have no recorded reads.** An instance restored from saved state has no read
  list until it renders once, so a root value change does not reach it before then.
- **Generated path overrides are not checked.** Generated code applies `env.size.width=…` without
  checking that the field exists in the value it patches, where the IR runtimes refuse a path that
  does not.
- **`environment-cycle` appears when a program is built.** The check runs in program building, so
  the editor shows it with the build's diagnostics rather than as the default is typed.
- **Go to definition.** Not offered for `env.` reads and overrides, because the language service has
  no definition query yet (see "Editor Navigation" in `specs/future.md`). Hover shows the declaration meanwhile.
- **The hover's "Reads" list follows calls by name.** It follows functions and values a body names,
  not a function-component used as an element, and not a value passed around and called later.

### Later work

- **Lazy local overrides.** A local override is evaluated where it is written, even when nothing
  under it reads the value. Evaluating it at the first read would save work for overrides that are
  rarely read, at the cost of a less obvious evaluation order.
- **Values only the host sets.** An environment value with no default, which a program can read but
  only the host can give, such as the window size.
- **Standard host inputs.** A library of environment values every host is expected to set, such as
  the color scheme, the locale and the reduced-motion preference.
- **`$env` on host elements.** A host element carries no `$env` today, so a host that wants to set
  `dir` or a CSS color scheme from the environment asks the runtime. Stamping host elements is easy
  to add if hosts want it.
- **An element form of `with`.** `<ada with age=37 />` or similar, if markup-heavy code wants it;
  `{ ada with { age=37 } }` covers it today.
- **`with` on union cases.** Patching a value of a union type whose cases are records, keeping the
  case. Today the left side must be a record, an update record or a bundle.
- **Conditional entries in a field list.** `{ ada with { if older { age=37 } } }`, to set a field
  only sometimes, without writing the field's current value in an `else`.

## Design Tokens: What `add-design-tokens` Left For Later

- **Reading a token evaluates the whole record it is a field of.** Within one call, defaults,
  entry-module provides and top-level values are evaluated once for each environment they depend on
  (`add-tracked-reactivity`), so in `specs/ir-conformance/design-tokens-theme` one
  `env.button.radius` read costs 2,328 operations, down from 48,905, and one `env.color.accent`
  read 1,957, down from 23,475. What remains is that reading one field of a record default
  evaluates the whole record once, so `env.button.radius` still builds all of `env.button` and
  what it reads. Evaluating fields lazily would change when errors appear.
- **A bundle's formula entries are evaluated at every read.** An entry-module bundle such as the
  brand's `theme` is reused, but a formula among its entries (`color={if env.colorMode is { … }}`)
  is held by no layer, so it is evaluated again at each read and applied on top of the reused
  default. In `design-tokens-theme`, a later `env.color.*` read in the same call costs 33
  operations in light mode and 44 in dark, against 2 for `env.button.radius`. Reusing these
  formulas would need a new kind of kept result with its own key and read tracking, in the area
  where `add-tracked-reactivity`'s review found its stale-output bugs, so it waits until a real app
  shows color reads mattering.
- **A palette generator.** `paletteFromHue(…)` building eleven steps from an OKLCH lightness curve,
  mapped with `toGamut(…, displayP3)`, so a brand can make a hue that matches the standard steps.
- **Contrast-target tokens.** Roles defined by a target ratio against another role, as Adobe's
  Leonardo does, so a theme that changes `background` keeps its text legible.
- **Generated TypeScript types.** Generated TypeScript names a type the file doesn't import when a value's type
  comes from a module it doesn't import, such as `@nx/color`'s `Color` through `@nx/design`'s
  alias, or an inferred `b.X.p` returned through another module; it then doesn't type-check.
  Importing such types where they are named would fix both. Matches on patterns other than union
  cases are still refused.
- **Token export.** `add-token-export`: DTCG `color` values with `colorSpace: "oklch"`, and CSS
  `oklch()` custom properties.

## Value Suggestions In Visual: What `add-visual-value-suggestions` Left For Later

A property offers the named values of its declared type (`suggestions` on the source tree, the
viewer's named-values popover). Left for later:

- **Modules not yet imported.** A property typed `Spacing` in a document that does not import
  `@nx/design` offers nothing from it; a later version could list a library's values with the import
  the choice would add.
- **Local bindings.** `let` bindings inside a body and `for` bindings are not offered, only
  top-level values, parameters and state fields.
- **List items.** An item of a list (`stops={ … }`) has no suggestion key of its own; only whole
  property and `let` values do.
- **Previews that follow `provide`s.** A preview reads defaults statically. The value a name has
  under the main module's `provide`, the host's root values or an enclosing section's override is
  what a running preview session knows, so a later version could ask it.
- **Element-typed properties.** A property typed by an element, function or sequence type has no
  key, so a shared `let card = <Card … />` is not offered for it.
- **Peer files in the workspace.** The language service gives a workspace document no peer
  bindings, even with a filesystem root, so the `peer` group stays empty there and a sibling file's
  value is offered only through an import (the spec's peer scenario holds for programs whose peers
  are bound). Peer binding in the language service would fill it.
- **A silent name collision between workspace imports.** Two unaliased imports of workspace files
  that both export `accent` give no diagnostic; the first wins. Suggestions leave such a name out,
  but the compiler should report it as it does for libraries.
- **Qualified type names in the viewer and value text.** The viewer and the canonical value
  formatter resolve a type spelled through an import alias (`design.Radius`) by its last part, which
  picks the wrong declaration when two modules declare the same name. The source tree could carry
  each property's resolved type declaration instead.
- **A button for properties read on a line.** A property shown in a folded element value's head or
  on its field's line offers its named values from Ctrl+Space and a click on its token, but draws no
  hexagon there, so a pointer reader finds it only by unfolding. A button that does not crowd the
  line (shown while the property is selected) would close the gap.
- **A P3 and sRGB comparison in the footer.** A wide color's row reads `P3` and its nearest sRGB
  hex, and the lines under the list give its exact value. On a wide-gamut screen, the footer could
  also show the highlighted color and its sRGB version as two larger, labelled patches, which is
  where Mac and OLED users would see the difference. Bret chose to wait (2026-11-26); see the
  OKLCH picker note under Wide-Gamut Color.
- **The defaults note for a `let` that reads `env.`.** The popover's note shows when the list holds
  an environment value or a parameter previewed by its default. A file's `let tint: Color =
  {env.color.accent}` also previews a default, but when no environment row is listed the note stays
  hidden. The service could flag such previews (`preview.fromDefault`) so the viewer shows it.
- **Visual at phone width in the playground.** At a 390-pixel viewport the playground's NX pane
  gives `<nx-viewer>` about 62 pixels, so Visual is unreadable on a phone. This predates value
  suggestions (every example shows it); the pane's layout, not the viewer, needs the fix.

## Wide-Gamut Color

`Color` is a union of color spaces (`srgb`, `displayP3`, `oklab`, `oklch`) with explicit `inGamut`
and `toGamut`. Left for later:

- Drawing on P3 surfaces in Skia and Canvas, instead of mapping into sRGB at preparation.
- An OKLCH picker, so wide colors can be edited and not only shown. On a wide-gamut screen it
  should show the color beside its `toGamut(…, srgb)` version as two large, labelled patches
  ("P3" and "sRGB"), so a designer can judge whether the fallback an sRGB screen shows is
  acceptable. A split swatch was tried in the named-values list (`add-visual-value-suggestions`)
  and dropped: at 12 pixels the halves looked the same, even on a P3 screen with HDR on. A screen
  can also report `(color-gamut: p3)` and still show no difference (LG HDR 4K panels under Windows
  HDR, with Chrome on its scRGB F16 buffer, drew pure P3 red and green like `#ff0000` and
  `#00ff00`), so the patches need labels and can't be the only signal; Macs, which manage P3 by
  default, are where they pay off.
- Further CSS spaces (`rec2020`, `a98-rgb`, `lab`, `lch`) if a use appears.
- `@nx/design`'s palette could adopt Tailwind v4's OKLCH values.
- `@nx/graphics` docs still call `Paint.color`'s `color` and `GradientStop.color` a "Shared sRGB
  color", which is out of date now that `Color` can be wide-gamut (`oklch`, `displayP3`). The docs
  should say a wide color is mapped into sRGB when drawn (until P3 surfaces land). Left out of
  `add-visual-value-suggestions` because editing `graphics.nx` shifts library source offsets, so the
  graphics preview fixtures (`crates/nx-graphics-skia/fixtures/revisions/*/manifest.json`) must be
  regenerated with it.
- Percentage literals for color (Bret, 2026-11-22). NX will get `23%` literals as a general
  language feature. Once they exist, color creation should accept them where CSS does. Channels,
  lightness and alpha would take `50%` for 0.5, as in `c.rgb(100%, 50%, 0%)`,
  `<c.Color.srgb r=100% … />` and `alpha=50%`. Visual and hovers could then show these 0 to 1
  fields as percentages. The stored value stays 0 to 1, which is why `color.nx` writes the range
  inline instead of naming it `Percentage`. Note that `Length.percent` in the drawn-UI proposal
  stores 50 for 50%, so the literal's meaning has to be settled per target type.
