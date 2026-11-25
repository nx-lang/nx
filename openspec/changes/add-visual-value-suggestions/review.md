# Review: add-visual-value-suggestions

## Scope
**Reviewed artifacts:** proposal.md, design.md, tasks.md, specs/value-suggestions/spec.md,
specs/source-edits/spec.md, specs/viewer/spec.md, specs/graphics-library/spec.md,
specs/playground/spec.md  
**Reviewed code:** spec-only change, nothing implemented. Claims were checked against
`nx-grammar.md`, `crates/nx-syntax/grammar.js`, `crates/nx-hir/src/lower.rs`,
`crates/nx-language-service/src/source_tree.rs`, `crates/nx-language-service/src/source_edit.rs`,
`crates/nx-program/src/std/design/{types,tokens,palette}.nx`,
`crates/nx-program/src/std/graphics/graphics.nx`, `crates/nx-program/src/std/color/color.nx`,
`crates/nx-program/src/artifacts.rs`, `packages/language-protocol/src/index.ts`,
`packages/viewer/src/{source,index}.ts`, `sites/playground/src/reading/reading.ts`, and the main specs
source-tree, source-edits, viewer, environment-values, color-tooling, color-literals,
unbraced-literal-forms, module-imports, declaration-visibility, graphics-library and playground.  

## Findings

### ✅ Verified - RF1 A name cannot be written bare as a `let` value: the grammar admits only a literal, a single contextual name, an element or a braced expression there
- **Severity:** High
- **Confidence:** High
- **Evidence:** `value_definition`'s value is `rhs_expression` (`crates/nx-syntax/grammar.js:224-234`),
  which is `element | literal | signed_numeric_literal | contextual_name | values_braced_expression`
  (`grammar.js:640-648`; `nx-grammar.md:406-417`). The unbraced-literal-forms spec says "an unbraced
  value SHALL be a literal ... SHALL NOT accept a dotted or otherwise qualified name in unbraced value
  position", and that a bare single identifier resolves against the site's union cases and
  "SHALL NOT consult variables" (`openspec/specs/unbraced-literal-forms/spec.md`). HIR lowers it to
  `Expr::ContextualName` (`crates/nx-hir/src/lower.rs:2086`). So `let gap: Spacing = env.space.sm`
  is a parse/validation error, and a one-part name such as `let accent: Color = blue` would read as
  the case `blue`, not the `let`. No `.nx` file in the repo writes a dotted unbraced `let` value. The
  rule appears in the source-edits delta ("bare inside braces, as a sequence item or as a `let`
  value", and the scenario "A name in a sequence and a `let`" expecting
  `let gap: Spacing = env.space.sm`), in design.md ("Picking writes `{name}`"), in the proposal
  ("bare where braces or a `let` already allow it"), and in task 2.1. The sequence half is right:
  `ValueListItemExpression` admits `Identifier` and `MemberAccess` (`nx-grammar.md:432-445`), so
  `{ 4 env.space.md }` parses.
- **Recommendation:** Spell a name braced wherever the value is unbraced, at an attribute and at a
  `let` value alike, and bare only inside braces. Change the scenario to expect
  `let gap: Spacing = {env.space.sm}`, and add one where a one-part name (`blue`) is set as a `let`
  value. Update design.md, the proposal and task 2.1 to match.
- **Fix:** A name is now braced at every unbraced place, an attribute or a `let` value, and bare only inside braces or as a sequence item. The scenario expects `let gap: Spacing = {env.space.sm}` and adds `let accent: Color = {blue}`. design.md, proposal and task 2.1 are updated.
- **Verification:** The source-edits delta spells a name braced at an attribute or `let` value and bare inside braces or as a sequence item, and the scenario expects `let gap: Spacing = {env.space.sm}` and `let accent: Color = {blue}`, both of which `rhs_expression`'s `values_braced_expression` admits (`grammar.js`). design.md ("Picking writes `{name}`"), the proposal and task 2.1 match.

### ✅ Verified - RF2 The delta keeps silent about the existing coupling between hex targets and color values, which refuses the main color flow
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `set_value` refuses any edit where `written.starts_with('#') != matches!(value, Color)`
  (`crates/nx-language-service/src/source_edit.rs:259-261`). As written, that refuses a `name` value
  over `background=#ffffff`, and a `color` value over a name target (Replace with value on a color
  token). `edit_tree` also sends a `setValue` with a `color` value on a `Call` or `Element` target to
  `set_constructor_color` (`source_edit.rs:148-152`), which edits channels in place. That conflicts
  with the widened rule that "replacing an element SHALL replace its whole text" once elements are
  targets. Neither rule is in the main source-edits spec, so the delta's spelling list doesn't
  override them, and no scenario covers a hex literal or a color written as a constructor. Binding a
  hex color to `env.color.*` is the headline use case.
- **Recommendation:** Say in the setValue requirement that a `name` value may replace a hex literal,
  a constructor call or a color element, and that a `color` value may replace a name. Spell out what
  a `color` value does on an element or call target: channel edit, as today, or whole replacement.
  Add scenarios for `fill=#2563eb` → name `env.color.accent` and for `{env.color.surface}` → color
  `#fafafa`. Reflect all of this in task 2.2.
- **Fix:** setValue now says a `name` replaces a hex literal, constructor call or color element whole; a `color` value keeps today's channel edit on calls and elements; a `color` over a name writes a hex literal; other values on a hex literal stay refused. New scenarios cover `background=#ffffff` → `env.color.surface` and `{env.color.surface}` → `#fafafa`. Task 2.2 and design.md are updated.
- **Verification:** Mostly done: the hex coupling, `color` channel edits and `color` over a name are now stated, with scenarios. But the delta's target list is still "a literal, a bare or qualified union case, a name, or an element", and a name excludes calls, so a color constructor call (`fill={c.oklch(…)}`, a `SourceRole::Call` node that `edit_tree` already routes to `set_constructor_color`) is not a `setValue` target. "A `name` value SHALL replace any target this requirement admits" therefore does not cover a constructor call, while the Fix note, design.md ("a `name` replaces any of them whole") and task 2.2 say it does. Admit a color constructor call that is a whole value as a target (for `color` and `name` values only), and add a scenario such as `fill={c.oklch(0.5, 0.2, 260)}` → name `env.color.accent`. Also say which rule wins for a `color` value on a color element: the brace-dropping paragraph says a non-name value over an element is written as at an unbraced place, while the next paragraph keeps the channel edit; and state that a `color` value on a non-hex literal or a case stays `invalidValue`, as the code refuses it today.
- **Fix:** (second pass) Color constructor calls are added to the setValue target list. A `name` replaces a hex literal, call or element whole. A `color` value's channel edit keeps the written form and its braces and takes precedence over brace dropping. A `color` on another literal or a case, an uneditable call, or any other value on a hex literal or call is refused as `invalidValue`.
- **Verification:** (second pass) The requirement text is now right. Color constructor calls are targets, a `name` replaces a hex literal, a call or an element whole, the `color` channel edit takes precedence over brace dropping, and the refusals are stated. Task 2.2 and design.md agree. What's still missing is the scenarios: the delta has none for a constructor-call target. Task 2.3 derives its tests from the scenarios, so the new behavior has no test. Add two scenarios: `fill={c.oklch(0.6, 0.2, 250)}` set to the name `env.color.accent` reads `fill={env.color.accent}`, and a `color` value on `fill=<Paint.color color=#2563eb/>` edits the channel in place and leaves the element. A third, for a `color` value on a number literal giving `invalidValue`, would cover the refusal.
- **Fix:** (third pass) Added scenarios: `fill={c.oklch(…)}` → name `env.color.accent`; a `color` on a `<Color.srgb r g b/>` element edits its channels in place and keeps the element; a `color` on `padding=8` is refused. Also reworded value-suggestions' opening so parameters resolve in their declaration's body (RF16 nit).
- **Verification:** (third pass) The source-edits delta now has scenarios for a constructor-call target (`fill={c.oklch(0.6, 0.2, 250)}` → name `env.color.accent` reads `fill={env.color.accent}`), for a `color` on a `<Color.srgb r=0.2 g=0.4 b=0.8 />` element (stays an element, only channels edited; `r`, `g` and `b` are `Color.srgb` fields in `color.nx`), and for a `color` on `padding=8` (`invalidValue`, as `set_value` refuses today). Together with the second-pass text, the target list, the precedence over brace dropping and the refusals are all specified and testable through task 2.3.

### ✅ Verified - RF3 Replace with value on a color token conflicts with color-tooling, and most palette tokens can't take it
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** color-tooling's "Picker edits preserve authored structure and alpha" says
  "Unsupported or computed expressions SHALL remain read-only; the picker SHALL NOT replace them with
  a constant", and "A recognized color in another space SHALL be shown read-only ... and SHALL NOT be
  offered to the sRGB picker" (`openspec/specs/color-tooling/spec.md:44-69`). The viewer delta has
  Replace with value open the color picker on a token's preview and write a literal over the token.
  No color-tooling delta changes that rule. Every `standardPalette` color is an `oklch(...)`
  constructor (`crates/nx-program/src/std/design/palette.nx:74+`), so by the delta's own rule ("a
  token ... with one the property's editor does not take SHALL NOT offer Replace with value")
  `env.color.accent` and most semantic colors in the light arm get no Replace with value. The
  proposal sells the feature as Figma's "detach" for tokens generally.
- **Recommendation:** Add a color-tooling MODIFIED requirement that allows an explicit Replace with
  value to write a hex over a name. Decide what happens for wide-gamut previews: offer a read-only
  picker, a nearest-sRGB hex with a warning, or nothing. Add a viewer scenario for an oklch-previewed
  token.
- **Fix:** New color-tooling ADDED requirement: Replace with value on a color token opens only when the preview is inside sRGB and writes hex rounded to 8 bits. A wide token offers no replacement, so a wide color is never rewritten as sRGB. There are scenarios for a gray token and for a wide green, and the viewer has a `standardPalette.green500` scenario. design.md explains why; writing the exact `oklch(…)` form is noted as a follow-up.
- **Verification:** The color-tooling delta adds the requirement, with scenarios for a gray token and a wide one; the viewer has the `standardPalette.green500` scenario, and "Choosing a named value writes its name" defers colors to color-tooling. `gray200` is `c.oklch(0.92, 0.004, 286.32)` (inside sRGB) and `green500` is `c.oklch(0.723, 0.219, 149.579)` (outside), so the examples hold. The delta frames this as an explicit replacement, not a picker edit of a computed expression, so it doesn't contradict "Picker edits preserve authored structure and alpha".

### ✅ Verified - RF4 The reachable set misses names visible without an import and doesn't handle selective imports or ambiguous imported names
- **Severity:** Medium
- **Confidence:** High
- **Evidence:**
  - Peer files: declaration-visibility makes a declaration without a modifier "visible to other files
    in the same library and to other root modules in the same non-library program"
    (`openspec/specs/declaration-visibility/spec.md:47-60`), with no import needed. The delta offers
    only "this document's" lets and lets of modules "the document imports", so a `brand.nx` peer's
    `let brandRed` isn't offered in `main.nx`.
  - Selective imports: `import { standardPalette } from "@nx/design"` brings in only the names it
    lists, and `as Prefix.Name` changes the spelling (`module-imports` spec lines 33-64). The delta
    knows only "the import's alias when it has one, and bare otherwise".
  - Ambiguous names: "Ambiguous imported names are diagnosed on use" (`module-imports` spec:195-210).
    The delta drops ambiguous environment values but would offer, and write, an ambiguous bare `let`
    name, which then fails to compile. A local `let` that shadows an imported one isn't addressed
    either.
- **Recommendation:** Define reachability as "what the module's top-level scope resolves without
  error". Add a group (or fold into `document`) for same-library and same-program peers. Spell
  selective imports by their alias prefix and list only the entries they import. Leave out ambiguous
  and shadowed names. Add a scenario for each.
- **Fix:** Reachability is now "names the top-level scope resolves without a diagnostic". A `peer` group was added for same-library and same-program files. Selective imports use their qualified alias and bring only the names they list. Ambiguous and shadowed names are left out. New scenarios cover peer, selective import and ambiguous name. Groups, design and task 1.3 are updated.
- **Verification:** Reachability, the `peer` group, ambiguous and shadowed names, and the scenarios are in place. But the selective-import bullet gives the example `import { standardPalette as sp } from "@nx/design"` gives `sp`, which module-imports rejects: a selective alias must be a single-dot qualified name ending in the imported name (`Prefix.standardPalette`). The scenario uses the valid `as d.standardPalette`, so the bullet contradicts it. Change the example to `import { standardPalette as d.standardPalette } from "@nx/design"`, spelled `d.standardPalette`.
- **Fix:** (second pass) The selective-import bullet now uses the valid qualified alias `import { standardPalette as d.standardPalette }` → `d.standardPalette`, matching the scenario.
- **Verification:** (second pass) The selective-import bullet now reads `import { standardPalette as d.standardPalette } from "@nx/design"` → `d.standardPalette`. That is valid under module-imports' single-dot alias rule and matches the "A selective import" scenario.

### ✅ Verified - RF5 Giving every value declaration entry a key, while also requiring each suggestion's declaration in the table, has no fixed point
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** Every "value declaration entry" whose type has a key carries `suggestionKey`. The list
  holds entries whose key "some `suggestionKey` of the answer carries". Each entry's `declaration`
  "the table SHALL then list". Listing `standardPalette` (type `StandardPalette`) or `color` (type
  `SemanticColors`) as suggestion declarations adds value entries whose keys pull in more
  suggestions, such as `env.color` for `SemanticColors` and `env.shadow` for `ShadowScale`, whose
  declarations are added in turn. The viewer only offers a `let`'s named values for a `let` it can
  edit, which is this document's parameterless lets.
- **Recommendation:** Give `suggestionKey` only to value declarations this document declares
  without parameters, and say that declarations added for `declaration` don't add keys, as
  source-tree's "A declaration listed only this way SHALL NOT bring the types of its own
  properties" does.
- **Fix:** Only this document's parameterless `let`s (and properties, state and case properties) carry `suggestionKey`. A declaration listed only for a suggestion brings no keys or property types.
- **Verification:** The source-tree delta gives `suggestionKey` to the property, state and case-property lists of declaration entries and to this document's parameterless `let`s only, and says a declaration listed only for a suggestion brings no keys or property types. Keys now come only from what the document's nodes refer to, so the set is finite and has a fixed point. design.md and task 1.1 match.

### ✅ Verified - RF6 `@nx/graphics` importing `@nx/design` without an alias collides with graphics' own exported names
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** `graphics.nx:23-25` declares `export type Color = colors.Color`, and `@nx/design`
  exports `Color` too (`types.nx:9`), along with generic names such as `Shadow`, `Easing`,
  `Duration` and `TextStyle`. The delta and task 4.1 say only "import `@nx/design`". The repo's
  style everywhere else in std is an aliased import (`import "@nx/color" as colors`,
  `graphics.nx:9-11`). Separately, a user program that imports both `@nx/graphics` and
  `@nx/design` without aliases, as the graphics delta's scenario does, gets an ambiguous `Color` on
  use under module-imports' rule. That is pre-existing, but this change encourages the combination.
- **Recommendation:** Specify `import "@nx/design" as design` in graphics and type with
  `design.BorderWidth` and the rest. Note the two-library `Color` ambiguity in the tutorial (task 7.2),
  or have graphics stop exporting its own `Color` alias.
- **Fix:** Graphics now imports `@nx/design` as `design` and writes `design.BorderWidth` and the rest. design.md adds the two-`Color` risk, and task 7.2 tells the tutorial to alias one import.
- **Verification:** The graphics delta and task 4.1 specify `import "@nx/design" as design` and `design.BorderWidth` and the rest. `@nx/design` imports only `@nx/color` and its own files, so no cycle. The design aliases' constraints match the graphics properties' constraints (`BorderWidth`/`Radius` `finite && >= 0.0`, `FontSize` `> 0.0`, `Opacity` `in 0.0..=1.0`). design.md lists the two-`Color` risk, and task 7.2 covers it.

### ✅ Verified - RF7 `scope` and `self` say "the key of" a declaration, which collides with suggestion keys, node keys and the table index
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `declaration` is "the index in the declaration table". `scope` is "the key of the
  top-level declaration". `self` is marked "with that declaration's key", and the scenario says
  "marked `self` with the key of `brand`". "Key" already means the suggestion key, and a source node's
  `key` (`source_tree.rs:96-101`). The suggestion key's wire format isn't given either, though
  task 3.2 needs Rust/TypeScript parity on it.
- **Recommendation:** Name one identifier for `scope` and `self`: the declaration-table index, which
  `declaration` already uses, or the top-level declaration node's `key`. State that `key` and
  `suggestionKey` are opaque strings that are equal exactly when the declaration identities are.
- **Fix:** `scope` and `self` now use declaration-table indices. Suggestion keys are opaque strings, equal exactly when the declarations are the same.
- **Verification:** `scope` and `self` are declaration-table indices in the source-tree delta and design.md, and the delta states that keys are opaque strings, equal exactly when the declarations are.

### ✅ Verified - RF8 The change modifies the `sourceTree` request and answer without a source-tree delta
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The new option, `suggestions`, `suggestionKey` and the extra declaration-table
  entries are specified in a new `value-suggestions` capability. Meanwhile source-tree's "The
  declaration table describes what nodes refer to" (`openspec/specs/source-tree/spec.md:152-163`)
  lists exactly what entries carry and when they are listed, and `visualizers`, the analogous
  option, lives in source-tree (`spec.md:189-257`). The proposal's Modified Capabilities omits
  source-tree, and language-protocol (`spec.md:94`) owns the request shapes.
- **Recommendation:** Either add a source-tree MODIFIED requirement for the option and the property,
  value and case `suggestionKey` fields, pointing to value-suggestions for the matching rules, or
  state in value-suggestions that it extends the source-tree table and the language-protocol shapes.
- **Fix:** Added a source-tree delta (ADDED requirement) holding the option, the `suggestions` list's entry shape and the `suggestionKey` fields. value-suggestions keeps the matching, reach, grouping and preview rules. Proposal lists source-tree and color-tooling under Modified Capabilities, and Impact names the language-protocol shapes.
- **Verification:** `specs/source-tree/spec.md` adds the option, the entry shape, the `suggestionKey` fields and JSON/wasm/language-core parity. The proposal lists source-tree and color-tooling under Modified Capabilities, and Impact names the language-protocol shapes. Strict validation passes.

### ✅ Verified - RF9 Static previews need color discovery on other modules, but discovery today runs only inside the requested document's source-tree walk
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** Swatches are computed while building this document's nodes
  (`source_tree.rs:540-576` for color elements, `1340-1366` for constructor calls, `3319` for
  `set_swatch`), and they depend on this document's resolution of the constructor's alias. The
  `env.color.accent` preview ends at `c.oklch(...)` in `palette.nx`, under palette's own alias `c`.
  The preview's `color` shape (`toCss`, hex when inside sRGB, outside-sRGB flag) also differs from the
  node's existing `color`, which is `swatch()`: `#rrggbbaa` for sRGB and CSS otherwise
  (`crates/nx-value/src/color.rs:540-547`), plus `colorOutsideSrgb`.
- **Recommendation:** In design.md, say that discovery is factored to work on any module's syntax and
  resolution. Reuse the node's `color`/`colorOutsideSrgb` shape for previews, so the viewer draws
  swatches one way.
- **Fix:** value-suggestions requires discovery to resolve constructors through the writing module's imports, and previews reuse the node's swatch text and outside-sRGB mark. design.md and task 1.4 say discovery is factored to work on any module.
- **Verification:** value-suggestions requires discovery to resolve a constructor through the writing module's imports, citing `palette.nx`'s `c.oklch(…)`, and previews carry the node's swatch text and outside-sRGB mark. design.md and task 1.4 say discovery is factored to work on any module.

### ✅ Verified - RF10 Dropping a name's braces applies only at attributes, which leaves `let` values inconsistent
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The delta drops braces only "when the target is a name or an element that is an
  attribute's whole value". With RF1 fixed, a `let` value such as `let gap: Spacing = {env.space.md}`
  replaced by `12` would read `{12}`, and replacing it with a case would be refused as
  `invalidValue`, since a case may only go unbraced.
- **Recommendation:** Apply the same rule to the value of a `let` without parameters, and add a
  scenario.
- **Fix:** Brace dropping now applies at a `let` value too, with the scenario `let gap: Spacing = {env.space.sm}` → `4`.
- **Verification:** Brace dropping covers an attribute's or `let`'s whole value, with the scenario `let gap: Spacing = {env.space.sm}` → `4` reading `let gap: Spacing = 4`. Task 2.2 matches.

### ✅ Verified - RF11 Ruling out "self" stops at the declaration itself, so a pick can still create a dependency cycle
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** Inside `environment font = …`, `env.text.body.size` is still offered, but
  `env.text`'s default reads `env.font` (`tokens.nx`, `environment text`), and environment-values
  rejects "A default that depends on itself". The same applies to `let a` / `let b` cycles.
- **Recommendation:** Mark `self` for every suggestion whose declaration depends transitively on the
  enclosing one, which needs a dependency walk on the same static follow, or say the type checker
  reports it and accept that. Add a scenario either way.
- **Fix:** `self` now lists every declaration the entry reads, directly or transitively, using hover's environment-read walk. The tokens.nx scenario expects `env.text.*` to list `font`.
- **Verification:** "A value is not suggested inside itself" lists in `self` the start declaration and every declaration it reads transitively, using hover's read walk. The tokens.nx scenario expects `env.text.*` to list `font`, which holds, since `environment text` reads `env.font.*` (`tokens.nx:194+`). design.md and task 1.3 match.

### ✅ Verified - RF12 The name-validation rule is garbled and lets through a lone `env`
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The delta reads "each an identifier other than `true` and `false` and the first `env`
  or a name". environment-values rejects "`env` without a member ... as an expression", and
  `is_identifier` (`source_edit.rs:372-376`) accepts keywords such as `if` or `for`.
- **Recommendation:** Reword: "one or more dot-separated identifiers, none `true`, `false` or a
  keyword; a first part `env` needs at least one more part". Add scenarios refusing `env` and
  `if.x`.
- **Fix:** Reworded: one or more dot-separated identifiers, none `true`, `false` or a keyword, with at least two parts when the first is `env`. The scenario refuses `env.space.2xl`, `env` and `if.x`, and task 2.1 matches.
- **Verification:** The rule now reads as recommended, the scenario refuses `env.space.2xl`, `env` and `if.x`, and task 2.1 matches.

### ✅ Verified - RF13 It isn't stated whether union-case payload fields and state fields carry `suggestionKey`
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The requirement says "each property of a declaration entry". Case payload fields
  live in `cases[].properties` (`source_tree.rs:459-468`) and state fields in `state`. The viewer
  scenario "Colors with their swatches" depends on `Paint.color`'s `color` field, a case payload
  field (`graphics.nx:41-47`).
- **Recommendation:** Name the lists: `properties`, `state` and each case's `properties`, as source-tree's
  bounds requirement already does (`source-tree/spec.md:303-308`).
- **Fix:** The source-tree requirement names `properties`, `state` and each case's `properties`. value-suggestions says a case element has its union's type, which answers the `Paint` question.
- **Verification:** The source-tree delta names `properties`, `state` and each case's `properties`, matching the bounds requirement's wording. value-suggestions says a union case's element has its union's type.

### ✅ Verified - RF14 Several scenarios have unclear or missing preconditions
- **Severity:** Low
- **Confidence:** High
- **Evidence:**
  - viewer "Colors with their swatches" expects rows under `@nx/design` without saying the document
    imports it.
  - value-suggestions "A token's default does not read itself" says "is shown" without saying
    whose source tree is asked for (tokens.nx inside the std library).
  - graphics "other than one passing an opacity outside `[0,1]` to those helpers": the two helpers
    are `arrowChecked` and `arrowPoints`, which are internal (`graphics.nx:468,520`) and called only
    with already-constrained values, so no program can hit the exception.
- **Recommendation:** Add the imports. Phrase the tokens.nx scenario as a source-tree request for
  that module. Drop the exception clause and say the helpers are internal.
- **Fix:** The viewer swatch scenario imports `@nx/graphics` and `@nx/design` and names the case field. The tokens.nx scenario is a source-tree request for that module. The opacity exception is dropped: the helpers are internal.
- **Verification:** The viewer swatch scenario imports `@nx/graphics` and `@nx/design` and names the case field `color`. The tokens.nx scenario is a source-tree request for that module. The graphics delta drops the exception and gives the internal helpers' `opacity` parameters the `[0,1]` constraint, which is safe because their only callers pass already-constrained values (`graphics.nx:480,533`).

### ✅ Verified - RF15 Ctrl+Space is the default macOS input-source shortcut
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The viewer delta binds Ctrl+Space alone. On macOS the system takes it by default,
  so the keyboard path may never reach the page. Elsewhere the viewer spec pairs Ctrl or Cmd
  (`viewer/spec.md:447`).
- **Recommendation:** Name a mac fallback (for example, Option+Esc, as VS Code does, or Enter on the
  focused button via a roving shortcut), or note the conflict in the docs task.
- **Fix:** Alt+Escape (Option+Esc on a Mac) also opens the popover, as VS Code does. Covered in the viewer delta, design.md and task 5.2.
- **Verification:** The viewer delta, design.md and task 5.2 add Alt+Escape (Option+Esc on a Mac). The proposal's sentence for this is garbled; see RF17.

## New Findings Discovered During 2026-11-25 06:35 Verification

### ✅ Verified - RF16 A parameter or state field that shadows a top-level name inside its declaration is still offered under the shadowed spelling
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The RF4 fix says "a name ... that a nearer declaration shadows, SHALL be left out under that
  spelling" and that each suggestion is "a name the module's top-level scope resolves without a
  diagnostic". Parameters and state fields aren't in the top-level scope, yet they're listed, and
  shadowing by them depends on where the value is written. With `let tint: Color = #e11d48` and
  `component <Badge tint: Color />`, the one list per document holds both the `document` entry `tint`
  and the `declaration` entry `tint` scoped to `Badge`. Inside `Badge` the viewer offers both, since
  `self` only marks cycles, and choosing the document's `tint` writes a name that resolves to the
  parameter. Nothing in the source-tree entry shape lets the viewer drop it.
- **Recommendation:** Say that the top-level-scope rule applies to entries without a `scope`, and that
  an entry without a `scope` lists in `self` (or in a new field) each top-level declaration whose
  parameters or state shadow its first name part, so that the viewer leaves it out there. Add a scenario.
- **Fix:** A top-level name shadowed by a parameter or state field lists that declaration in `self`. A new scenario covers `let tint` with `<Badge tint: Color/>`, and task 1.3 matches.
- **Verification:** value-suggestions now says that a top-level name a parameter or state field shadows lists that declaration in `self`, and the scenario "A parameter shadows a top-level value" expects the unscoped `tint` to list `Badge` and the scoped `tint` to carry `Badge` as its scope. Task 1.3 matches. The viewer's existing `self` filter then leaves the shadowed entry out inside `Badge`, with no change to the entry shape. A small leftover: the requirement's lead-in still says every entry is "a name the module's top-level scope resolves", which parameters aren't, but the bullets make the intent clear.

### ✅ Verified - RF17 The proposal's sentence on the Mac shortcut is garbled
- **Severity:** Low
- **Confidence:** High
- **Evidence:** proposal.md: "it and Ctrl+Space open a popover that lists the suggestions by group, ...
  the current one marked and the default first (Ctrl+Space, or Option+Esc on a Mac)." Ctrl+Space
  appears twice, and the parenthetical attaches to "the default first".
- **Recommendation:** Reword as "it, Ctrl+Space and Alt+Escape (Option+Esc on a Mac) open a popover
  that ...".
- **Fix:** The proposal sentence is rewritten: the button, Ctrl+Space and Option+Esc (on a Mac) open the popover.
- **Verification:** proposal.md now reads "it, Ctrl+Space and Option+Esc (on a Mac, where the system may take Ctrl+Space) open a popover ... the current one marked and the default first." It is no longer garbled or duplicated. It names the Mac spelling only, where the viewer delta binds Alt+Escape on every platform, which is fine for a summary.

## Questions
_Answers (fix pass): ButtonGap keeps only its own values, by design, noted as a risk in design.md. A union case's element has its union's type (value-suggestions). Previews do not see through `with`: it is not in the follow list, so such values get no preview._

- Should a property typed by a user alias of a token type, such as `ButtonGap = Spacing`, really get
  none of `env.space.*`? The rule is consistent, but a narrower role losing the tokens of its base
  role may surprise users. An "also offer the alias's target" fallback might be wanted.
- What type does the checker infer for an untyped element value such as
  `let blue = <Paint.color color=#2563eb />`: `Paint`, or a case type? That decides whether it matches
  a `Paint` key.
- Should values from `with` expressions or update records count as "computed" (no preview), or
  should the static follow see through them?

## Summary
- The design is coherent and well motivated: one list per document, matching by declared alias,
  static previews. Most claims about `@nx/design`, `@nx/graphics`, the source-tree types and the
  viewer's existing token reading check out against the code.
- One High issue: names spelled bare at `let` values aren't valid NX (RF1).
- The Medium issues are an unspecified hex/color coupling in `setValue` (RF2), a conflict with
  color-tooling for Replace with value (RF3), incomplete reachability (RF4), suggestion-key inflation
  (RF5) and a graphics import collision (RF6).
- The rest are clarity and scenario gaps. Fix RF1 to RF6 in the artifacts before implementation.
