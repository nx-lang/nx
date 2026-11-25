## Why

`add-design-tokens` (#75) gave NX a standard token set: `env.color.accent`, `env.space.md`,
`standardPalette.blue600`, and the value types (`Spacing`, `Radius`, `Color`, …) that say what kind
of value a property takes. Visual can't use any of it yet. A reader who wants a card's padding to be
the medium spacing step has to know the token exists, switch to Source and type
`padding={env.space.md}`. Design tools make this a pick: Figma's variables, Penpot's tokens and
Framer's color styles all show, beside a field, the named values that fit it, and choosing one binds
the field to the name rather than copying its value.

Tokens are only one kind of named value, though. A brand's `let brandRed = #e11d48`, a library's
exported `let cardShadow: Shadow = …`, a component's own `tint` property and a field of a palette
record are just as useful to pick, and Bret's rule for this feature is that tools must not offer only
tokens. So this change offers every reachable named value of a property's type, with environment
values first.

## What Changes

- **Value suggestions in the source tree.** A `sourceTree` request can ask for `suggestions`. The
  answer then lists the named values the document can reach, each with the name as this document
  would write it (`env.space.md`, `standardPalette.blue600`, `d.standardPalette.blue600` under an
  import alias, `tint`), a group, its type, its doc, and a preview when its value can be found
  without running the program (a color, a number, a string, a boolean or a case). Each property and
  `let` in the declaration table carries the key that selects its suggestions.
- **Suggestions match by declared type, not by base type.** A property typed `Spacing` gets values
  declared `Spacing` (or by an alias of `Spacing`), not every `float64`, so radius tokens are not
  offered for padding. A property typed by a record or union, such as `Color`, `Paint` or `Shadow`,
  gets values of that type. A property typed by a bare primitive (`float64`, `string`) gets none.
- **What is reachable.** Environment values the module can see and their record fields, then the
  enclosing component's or function's parameters and state, then this document's top-level `let`
  values, then those of other files of its library or program, then each imported module's, each
  with their record fields, spelled as the module resolves them. A value is not offered inside
  its own declaration.
- **A named-values popover in Visual.** A field whose property has suggestions shows a quiet
  named-values button, as Figma shows its variable button, while the property is under the pointer or
  selected; it, Ctrl+Space and Option+Esc (on a Mac, where the system may take Ctrl+Space) open a
  popover that lists the suggestions by group, each with its preview, filtered as the reader types,
  the current one marked and the default first. Choosing one
  writes its name; on an unset property it adds the property.
- **Tokens read as tokens.** A value written as an `env.` read or a member chain, such as
  `{env.color.accent}` or `{standardPalette.blue600}`, reads as a token, with a swatch when its
  preview is a color, as a reference to a `let` does today. A click on a token opens the popover. Its
  menu adds Replace with value, which opens the property's own editor on the token's preview (for a
  color, only inside sRGB, written as hex).
- **Source edits write names.** `sourceEdit` takes a new typed value, `name`, for `setValue` and
  `addProperty`, spelled in braces at an attribute or `let` value (`padding={env.space.md}`) and bare
  inside braces. `setValue` can now replace a value written as a name or an element, and a name can
  replace a hex color, so a token can be swapped for another token or for a literal.
- **`@nx/graphics` names the design value types.** `Stroke.width` becomes `BorderWidth`,
  `Text.fontSize` `FontSize`, `Text.fontFamily` `FontFamily`, `Rectangle.rx` and `ry` `Radius`, and
  every `opacity` `Opacity`, with the same constraints and defaults, so graphics properties get
  token suggestions. `@nx/graphics` then imports `@nx/design` under an alias.
- **The playground asks for suggestions** in Visual.

## Capabilities

### New Capabilities

- `value-suggestions`: which named values the language service suggests for a property or `let`,
  and how they are matched, ordered, spelled and previewed.

### Modified Capabilities

- `source-tree`: the `suggestions` option, the answer's `suggestions` list and the `suggestionKey`
  fields of the declaration table.
- `color-tooling`: replacing a color token with its value writes hex only for an sRGB color.
- `source-edits`: the `name` value kind, and `setValue` on a value written as a name or an element.
- `viewer`: the named-values popover, and tokens for `env.` reads and member chains.
- `graphics-library`: graphics properties typed by `@nx/design` value types.
- `playground`: the Visual view asks for suggestions.

## Impact

- **Language service** (`crates/nx-language-service`): a suggestion pass over the analysis that
  answers `sourceTree`, using the type checker's scope and alias information; `name` values and the
  wider `setValue` target in `source_edit.rs`. No compiler, IR or runtime change.
- **Protocol and TypeScript** (`packages/language-protocol` request and answer shapes, `language-core`, the wasm and Node
  snapshots, the HTTP handler and client): the option, the answer's `suggestions`, the property
  `suggestionKey` and the `name` edit value, with Rust and TypeScript parity tests.
- **Viewer** (`packages/viewer`): the button, the popover, the token reading and Replace with value.
- **Standard libraries**: `@nx/graphics` imports `@nx/design`; corpus images regenerate.
- **Playground**: passes `suggestions: true`; the Visual test cases page gains a token example.
- **Docs**: the viewer and language-service reference pages, and the design-tokens tutorial's
  section on picking tokens in Visual.
- **Follow-ups**: suggestions from modules not yet imported (adding the import), `for` bindings and
  local `let`s, list items, previews that follow main-module `provide`s, templates (element-typed
  properties), and `add-token-export`.
