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

## New Findings Discovered During 2026-11-25 18:59 Review

**Scope of this pass:** the implementation, `git diff 61998c1..HEAD`: `crates/nx-language-service/src/{suggestions,suggestions_tests,source_tree,source_edit,source_edit_tests,environment,lib}.rs`, `crates/nx-program/src/artifacts.rs`, `crates/nx-program/src/std/graphics/graphics.nx`, `crates/nx-api/src/nx_text.rs`, `packages/language-protocol`, `packages/language-core`, `packages/language-http`, `bindings/node`, `bindings/wasm` (host and parity test), `packages/viewer/src/{named,index,source,edit,styles}.ts` and `test/named.test.ts`, `sites/playground` (reading, ReadingView, visual-test-cases example, browser check), docs and `specs/future.md`. Checked against every delta spec of the change.
**Ran:** `cargo test -p nx-language-service` (386 passed), `cargo test -p nx-api` (all passed), the viewer's `named.test.js` (15 passed), `openspec validate --strict` (valid). Probed `sourceTree` with `suggestions` through the wasm host on small documents and on `visual-test-cases.nx`. Skia tests and the Playwright check were not run.

### ✅ Verified - RF18 The HTTP handler drops `suggestions` from a `sourceTree` request that names no `visualizers`
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `validateQuery` parses `suggestions` (`packages/language-http/src/index.ts:287-293`) but returns it only from the `record.visualizers !== undefined` branch (`:294-312`). A request of `{ documents, uri, suggestions: true }` falls through to `return { documents, uri, position }` (`:327`), so the answer has no suggestions and no keys, and nothing reports why. The source-tree delta makes `suggestions` independent of `visualizers`, and task 3.1 says the HTTP handler passes it through. `packages/language-http/test/handler.test.ts` has no `suggestions` test, so this went unnoticed. The playground works only because it also sends `visualizers: {}`.
- **Recommendation:** Return `suggestions` from the plain `sourceTree` path as well. For example, build the `sourceTree` result once after both checks. Add handler tests for `suggestions` with and without `visualizers`, for a non-boolean `suggestions` (400), and for `name` and `color` edit values, which are also new in `sourceEditValueOf`.
- **Fix:** `validateQuery` now returns `suggestions` from a plain `sourceTree` request too. A handler test covers `suggestions` with and without `visualizers`, a non-boolean `suggestions` (400), a `name` edit (`{gap}`), a `color` edit value and a malformed `name` value (400).
- **Verification:** `validateQuery` now returns `{ documents, uri, position, suggestions }` for a `sourceTree` request without `visualizers` (`packages/language-http/src/index.ts:314-316`). The new handler test covers both shapes, the 400 for a non-boolean, `name` and `color` edit values and a malformed `name`.

### ✅ Verified - RF19 A `color` value on a color element with no static color now replaces the element with a hex literal instead of being refused
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** `edit_tree` now sends a `color` `setValue` on an element to `set_constructor_color` only when `tree.nodes[at].color.is_some()` (`crates/nx-language-service/src/source_edit.rs:158-163`). Before, every element went there and was refused unless its color was known. A canonical `Color` element whose channels are computed (`tint=<c.Color.srgb r={x} g=0 b=0 />`) or invalid (`<c.Color.srgb r=2 g=0 b=0 />`, which carries only `color_error`) now goes to `set_value`. There `SourceRole::Element => true` (`:322`) makes it replaceable, and the value is spelled as hex over the whole element. The source-edits delta says a `color` value "SHALL keep its current meaning on ... a color element", and color-tooling says computed colors "SHALL remain read-only; the picker SHALL NOT replace them with a constant". No test covers a color element without a swatch.
- **Recommendation:** Decide "is a color element" from the element's declaration (the canonical `Color` union, as `set_constructor_color` checks), not from whether a swatch was found. Send every such element to `set_constructor_color`, which refuses one it cannot edit. Add tests for a computed channel and an invalid channel.
- **Fix:** `edit_tree` sends a `color` value on any element whose declaration is the canonical `@nx/color` `Color` (new `is_color_element`), or whose color is known, to `set_constructor_color`, which refuses one it can't edit. A test in `color_edits_refuse_wrong_kinds_removed_nodes_and_noneditable_expressions` covers a computed channel (`r={x}`) and an out-of-range one (`r=2`): no edits, `notEditable`.
- **Verification:** `edit_tree` now also routes an element whose declaration is the canonical `@nx/color` `Color` to `set_constructor_color`. I probed it through the rebuilt wasm host: `<c.Color.srgb r={x} …/>`, and the same through `@nx/graphics`'s `g.Color` and `@nx/design`'s `d.Color` aliases, each answer `notEditable` with no edits, since the node's declaration resolves to `@nx/color/color.nx#Color` in all three cases. The test covers computed and out-of-range channels.

### ✅ Verified - RF20 The viewer offers no named values for a property whose written value is an element without a color, so a `Paint` cannot be bound to a named `Paint`
- **Severity:** Medium
- **Confidence:** Medium
- **Evidence:** `replaceable` in `packages/viewer/src/named.ts:76-91` accepts an `element` or `call` only when `node.color !== undefined`. So `fill=<Paint.color color=#2563eb />`, with `let blue: Paint = …` in reach, gets no button, no Ctrl+Space and no popover, though `fill` carries a `Paint` suggestion key and the tree lists `blue`. The viewer delta says such a property "SHALL offer its named values", with no exception for element values. The source-edits delta's "A name replaces an element" (`fill=<Paint.color …/>` → `fill={blue}`), which the service implements, can't be reached from Visual. Graphics' `fill`/`stroke` written as elements are the commonest case. `specs/future.md` defers only element-typed *properties*, not element *values* of record- or union-typed properties.
- **Recommendation:** Treat any whole element value as replaceable, as the service does. Keep the color requirement only for `call`. Add a viewer test that binds `fill=<Paint.color …/>` to a `Paint` `let`.
- **Fix:** `replaceable` accepts any whole element value (a call still needs a color). New viewer test "binds a paint written as an element to a named paint": `fill=<g.Paint.color …/>` has the button, lists `blue: g.Paint`, and choosing it sends `setValue` `{name: blue}` at the element.
- **Verification:** `replaceable` returns true for any element (a call still needs a color). The new viewer test binds `fill=<g.Paint.color …/>` to `blue` with a `setValue` name edit at the element, which the service supports ("A name replaces an element").

### ✅ Verified - RF21 Properties read on a field's line and `let` tokens get no named-values button, and a click on them doesn't open the popover
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The button is added only for rows with the `prop-field` class (`packages/viewer/src/index.ts:1919`), and a token click opens the popover only inside `.sheet-row[data-key]` (`:5096`). The properties of a short element value read on its field's line (`.line-property`), which the viewer spec says are "edited and scrubbed there as an inspector's fields are", get neither. That is why the graphics test must unfold the paint first ("offers colors with their swatches for a paint's color, unfolded"). The viewer delta's scenario "Colors with their swatches" speaks of the inspector without requiring an unfold. A `let` whose value is a token, such as `let tight: Spacing = {env.space.sm}`, gets the button, but a click on its token doesn't open the popover, as "A click on a token value SHALL open the named-values popover" requires.
- **Recommendation:** Give line properties the same button (and Ctrl+Space through their selection), and handle token clicks for line properties and `let` values. Otherwise, record the gap in `specs/future.md` and narrow the scenario.
- **Fix:** A token click now opens the popover for any place whose value is a name: an inspector row, a `let` value, a line property, and a name in a folded element value's head (it unfolds the value first). Properties on a Source-style line (`.line-property`) get the hexagon beside their menu while selected. Inspector inline properties and folded-head fields register their places, so Ctrl+Space on their selection opens it, but draw no hexagon, to keep the line uncrowded; that gap is in `specs/future.md`. Tests: "opens from a click on a let's token" and "offers a line property's named values from Ctrl+Space and a click on its token" (no unfold needed).
- **Verification:** Token clicks now go through `#tokenPlaceOf`, which covers inspector rows, `let` values, line properties and folded-head names, the last unfolding first. Line properties get the hexagon while selected. Inline and folded-head properties register their places for Ctrl+Space but draw no button, which is deliberate and recorded in `specs/future.md` ("A button for properties read on a line"). The new tests cover a `let` token click and a line property's Ctrl+Space and click.

### ✅ Verified - RF22 Every reachable value is added to the declaration table, and its read closure computed, even when none of its suggestions match
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `attach_suggestions` calls `self.entry(&root.start)` and `reads_closure` for each root before `visit` decides whether anything matches (`crates/nx-language-service/src/suggestions.rs:1039-1068`). A document with one `Spacing` property got 10 extra table entries (`env.colorMode`, `env.color`, `env.radius`, `env.borderWidth`, `env.font`, `env.text`, `env.shadow`, `env.motion`, `standardPalette`, …), and only `env.space` produced suggestions (3 declarations without suggestions, 13 with). The design keeps the list small ("Only entries whose key some property or `let` of the declaration table uses are sent"). The source-tree delta only requires each suggestion's declaration to be listed.
- **Recommendation:** Run `visit` into a temporary list first. Allocate the `declaration`, `scope` and `self` table indices only when that root produced at least one suggestion, and compute `reads_closure` lazily too.
- **Fix:** Each root is matched first into a temporary list. Its `declaration`, `scope` and `self` table entries, and `reads_closure`, are computed only when it produced a suggestion. Test `a_value_with_no_matching_suggestion_adds_no_declaration`: a document with one `Spacing` property lists only `Box`, `Spacing`, `root`, `env.space`.
- **Verification:** Each root is now matched into a temporary list, and table entries and `reads_closure` are computed only when something matched. Suggestions keep their declaration, scope and self. The new test checks that `env.color`, `env.radius`, `env.shadow` and `standardPalette` aren't listed for a `Spacing`-only document.

### ✅ Verified - RF23 A compiler change to duplicate-import reporting ships without a test, a spec delta or a mention in the proposal
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `apply_graph_imports` no longer reports "imported more than once" for a written import that follows a synthesized one, such as `import "@nx/color" as c` after the one a hex literal synthesizes. The written import now binds normally (`crates/nx-program/src/artifacts.rs:2894-2915`). This changes compiler diagnostics and bindings. The proposal's Impact says "No compiler, IR or runtime change", no delta (module-imports) covers it, and no test in `nx-program` exercises it. Only the commit message mentions it.
- **Recommendation:** Add an `nx-program` test: a hex literal plus a later `import "@nx/color" as c` gives no diagnostic, `c.` names resolve, and a second written import is still reported. Mention the fix in the proposal's Impact, or add a small module-imports delta.
- **Fix:** New `nx-program` test `a_written_color_import_beside_a_hex_literal_binds_and_a_repeat_is_reported` (the written `c.` import binds with no diagnostic; a second written import is still reported). The proposal's Impact now names both compiler fixes.
- **Verification:** The new `nx-program` test checks that a written `import "@nx/color" as c` beside a hex literal binds with no diagnostic and that a second written import is still reported. The proposal's Impact names this fix and the ambiguity record.

### ✅ Verified - RF24 Token swatches and Replace with value find a name's preview by its text alone, ignoring scope
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `suggestionNamed` (`packages/viewer/src/named.ts`, used for a token's swatch at `packages/viewer/src/source.ts:870`) and `#replacementOf` (`packages/viewer/src/index.ts:4896`) pick the first suggestion with that `name` and a preview, without the `scope`/`self` filter `namedValues` applies. With RF16's case (`let tint: Color = {env.color.accent}` and `component <Badge tint: Color = #000000 />`), the `declaration`-group `tint` scoped to `Badge` comes before the document's `tint`. So a top-level `{tint}` token shows Badge's default swatch, and Replace with value writes `#000000`, not the `let`'s value.
- **Recommendation:** Resolve the preview with the same filter as `namedValues`, for the place's declaration (`within`). For a token outside any place, prefer unscoped entries, and scoped ones only inside their scope.
- **Fix:** `suggestionNamed(tree, name, within, key?)` filters by the same scope and `self` rules as `namedValues` (shared `reaches`). Token swatches pass the token's enclosing declaration, pending tokens and `#replacementOf` pass the place's. Test: with a top-level `tint` and Badge's `tint` parameter, `<Card fill={tint} />` shows the accent swatch and Replace with value offers `#155dfc`.
- **Verification:** `suggestionNamed(tree, name, within, key?)` now applies the same `reaches` filter as `namedValues`. Token swatches pass `enclosingDeclaration`, while pending tokens and `#replacementOf` pass the place's declaration and key. In the RF16 setup, Badge's scoped `tint` no longer reaches a top-level place.

### ✅ Verified - RF25 The first row is active as soon as the popover opens, so Enter right away resets a written property to its default
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** `#openNamed` calls `#filterNamed(session, current row)`, and with no marked row `#filterNamed` makes `shown[0]` active (`packages/viewer/src/index.ts:4664`, `#filterNamed`). For `padding=8` that row is `Default`, so Ctrl+Space then Enter fires `remove`. The viewer delta says "Up and Down SHALL move through the rows, from the marked row when none is active yet", which implies nothing is active until the reader moves or types. No test covers the keyboard path: Up/Down from the marked row, or Enter with nothing typed.
- **Recommendation:** Leave no row active until Up/Down, typing or the pointer picks one (keep the marked row active when there is one). Make Enter with no active row do nothing. Add keyboard tests.
- **Fix:** No row is active at open unless one is marked; typing makes the first match active; Enter with none active does nothing. Tests: "walks the rows with the arrows, and Enter with none active changes nothing" and "walks from the marked row".
- **Verification:** `#filterNamed` leaves no row active at open unless one is marked (the current name, or Default on an unset place, either of which chooses nothing). Typing activates the first match, and Enter with no active row is a no-op. Tests cover both, and the 23 named viewer tests pass.

### ✅ Verified - RF26 The popover's accessibility has small gaps: a non-option inside the listbox, and hover cards available only as a `title`
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The "No named values match" element is a plain `div` inside the `role="listbox"` list (`packages/viewer/src/index.ts:4649`), which ARIA doesn't allow there. Screen readers won't announce an empty result, because the combobox's `aria-activedescendant` is just removed. Each row's "hover card" (type and doc) is only a `title` attribute (`:4645`), which keyboard and touch users can't reach. The viewer elsewhere uses real hover cards, which the delta's "A row's hover card SHALL give its type and doc" suggests. The button says `aria-haspopup="listbox"`, but what opens is a combobox's container, and it holds focus.
- **Recommendation:** Move the empty message out of the listbox and announce it through a polite live region (or `aria-describedby` on the input). Show the active row's type and doc in the viewer's card, or in the foot, for the keyboard path. Consider `aria-haspopup="dialog"` for the button.
- **Fix:** The empty message is now outside the listbox, `role=status` with `aria-live=polite`, text set only when shown. A `.named-detail` line below the list shows the active row's type and doc and is the input's `aria-describedby`. The button says `aria-haspopup="dialog"`. Asserted in the first viewer test.
- **Verification:** The empty message is outside the listbox as `role=status`, the `.named-detail` line is the input's `aria-describedby` and shows the active row's type and doc, and the button says `aria-haspopup="dialog"`. A nit that doesn't block this: the status region is `hidden` until it has content, and a live region that enters the accessibility tree with its text already set isn't announced reliably by every screen reader. Keeping it shown and empty, and setting only its text, would be more robust.
- **Follow-up (verification nit):** the status region now stays in the popover, empty while rows show, and only its text changes, so screen readers that skip a live region appearing with text already set still announce it.

### ✅ Verified - RF27 The source-edits delta says an uneditable constructor call refuses a `color` value as `invalidValue`, but the service answers `notEditable`
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The delta: "A `color` value on ... a constructor call it cannot edit ... SHALL be refused as `invalidValue`." `set_constructor_color` returns `NotEditable` for `c.rgb(0,0,0)`, `c.withAlpha(…)`, `c.tryParse(…)` and wide color elements, and the existing test `color_edits_refuse_wrong_kinds_removed_nodes_and_noneditable_expressions` asserts `NotEditable`. The code keeps today's behavior and the delta misstates it.
- **Recommendation:** Change the delta to say `notEditable` for a constructor call or color element the color tooling cannot edit, and keep `invalidValue` for a `color` value on another literal or a case.
- **Fix:** The source-edits delta now says a `color` value on a constructor call or color element the color tooling can't edit is refused as `notEditable`, and is not written over it. `invalidValue` stays for other literals and cases. New scenario: "A color on a computed color element is refused".
- **Verification:** The source-edits delta now says `notEditable` for a constructor call or color element the tooling can't edit, keeps `invalidValue` for other literals and cases, and adds the scenario "A color on a computed color element is refused". This matches the code and the tests.

### ✅ Verified - RF28 The suggestion pass detects ambiguous imports by parsing diagnostic message text
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `ambiguous_names` (`crates/nx-language-service/src/suggestions.rs:387-415`) strips the prefixes `"Imported name '"` and `"Ambiguous imported name '"` from diagnostic messages, which `crates/nx-program/src/artifacts.rs:3351,3734,3854` format. Rewording either message silently brings ambiguous names back into the suggestions, and only one test (`an_ambiguous_name_is_left_out`) would catch it, for one of the two messages.
- **Recommendation:** Expose ambiguity as data: a diagnostic code, or the set of ambiguous names on the prepared module. Use that instead of the message text, or at least share the prefix constants with the code that formats them.
- **Fix:** `PreparedModule` records ambiguous names (`mark_name_ambiguous`, `ambiguous_names()`) where the three ambiguity diagnostics are added in `artifacts.rs`, and `ambiguous_names` in `suggestions.rs` reads that instead of message text. `an_ambiguous_name_is_left_out` still passes.
- **Verification:** `PreparedModule` gains `mark_name_ambiguous`/`ambiguous_names()`, called at all three sites in `artifacts.rs` that report an ambiguous import, and `suggestions.rs` reads it instead of message text. `an_ambiguous_name_is_left_out` still passes.

### ✅ Verified - RF29 The playground example and the viewer tests miss parts of the scenarios they claim
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The playground delta asks for an example "that ... writes properties with tokens, with literals and unset". `let tiles = <Tile padding={env.space.md} fill={env.color.surface} />` (`sites/playground/src/examples/nx/visual-test-cases.nx:237`) has tokens and an unset `radius`, but no literal-written property to bind. The viewer test "adds an unset property with a name, and Default leaves it unset" never chooses `Default` on the unset property. There are no tests for the token-click path on a `let`, for Escape keeping the selection after a click-opened popover, or for a token without a preview offering no Replace with value. Task 5.6 is checked.
- **Recommendation:** Add a literal property to `Tile` (for example `radius=4`, leaving another unset) and cover binding it in the browser check. Add the missing viewer assertions, or rename the test to match what it checks.
- **Fix:** Tile gains `border: design.BorderWidth = 0`, written as `border=1` in `tiles`. The browser check binds it to `env.borderWidth.thin` and undoes it. The viewer test for unset properties now chooses Default on `tokened.body.radius` (no edit, still unset). New tests cover Escape after a click-opened popover (selection stays), a `let` token click, and a token without a preview (`{sum}`, no Replace with value).
- **Verification:** `Tile` now writes `border=1` (a `BorderWidth` literal), and the browser check binds it to `env.borderWidth.thin` and undoes it. The viewer tests now choose Default on an unset property, check that Escape after a click-opened popover keeps the selection, click a `let` token, and check that a token without a preview (`{sum}`) offers no Replace with value.

## Questions (2026-11-25 18:59 pass)
- (Answered by the fix: yes, element values are offered named values.) RF20: are element values of record- or union-typed properties (`fill=<Paint.color …/>`) meant to be offered named values in this change? The service supports replacing them and the viewer delta doesn't exclude them, but the viewer filters them out. If it's deliberate, record it in `specs/future.md`.

## Summary (2026-11-25 18:59 pass)
- The implementation is substantial and mostly faithful to the deltas. Keys, matching, grouping, ordering, `self`, shadowing and previews behaved as specified in every probe, and the Rust and viewer suites pass.
- Three Medium issues: the HTTP host drops `suggestions` without `visualizers` (RF18); a `color` edit can now overwrite a computed color element with hex (RF19); and the viewer can't bind element-valued properties such as `Paint` (RF20).
- The rest are Low: line properties and `let` tokens (RF21), table bloat (RF22), an unflagged compiler change (RF23), scope-blind previews (RF24), keyboard default activation (RF25), popover ARIA details (RF26), a spec/code mismatch on refusal codes (RF27), fragile ambiguity detection (RF28), and example and test gaps (RF29).

## New Findings Discovered During 2026-11-26 Review

**Scope of this pass:** `git diff 9ed6891..HEAD`, this PR's work since main was merged in: `packages/viewer/src/{index,styles,named,source}.ts` and `test/named.test.ts`, `crates/nx-language-service/src/{suggestions,suggestions_tests}.rs`, `packages/language-protocol/src/index.ts`, `sites/playground/src/examples/nx/{settings-screen,graphics}{.nx,.out.nx}`, `sites/playground/scripts/check-named-values-browser.mjs`, `sites/playground/src/reading/ReadingView.tsx`, `scripts/check-graphics-preview.py`, `specs/future.md`, and the change's `design.md` and its `viewer` and `value-suggestions` deltas.
**Ran:** `pnpm run build && pnpm test` in `packages/viewer` (269 passed), `cargo test -p nx-language-service suggestions` (23 passed), `pnpm run test:named-values-browser` (passed), `pnpm run check-examples` (26 examples match). I also ran a throwaway jsdom probe (since deleted) to check where a `let`'s button lands for element, list, optional and multi-line element values.

### ✅ Verified - RF30 The defaults note is hidden for lists whose previews are still defaults, such as parameters
- **Severity:** Medium
- **Confidence:** High
- **Evidence:** The foot shows only when some row is in the `environment` group (`packages/viewer/src/index.ts:4796-4800`), and the comment there, like the viewer delta ("environment values, the only ones an override can change"), says only environment values can be overridden. That isn't true. A `declaration`-group row (a parameter or state field) previews its **default** (`crates/nx-language-service/src/suggestions.rs:1340-1352`, `value: default.map(…)`), and any caller replaces that value. A document `let` that reads `env.` (CARD's `let tint: Color = {env.color.accent}`) previews the env default under `This file`, even if the list has no environment rows (for example, when scope or type filtering leaves none). In those cases the popover shows default values as if they were the values, with no note. Only the "no note" case is tested (`test/named.test.ts:419`, a list with no declaration rows).
- **Recommendation:** Show the note whenever a row's preview depends on a default: any `declaration` row with a preview, or any row whose preview followed an `env.` read. The service could flag the second case (for example `preview.fromDefault`). Alternatively, show the note whenever the list holds environment or declaration rows. Reword the spec sentence and the code comment, and add a test with a component parameter row.
- **Fix:** The foot now shows when the list holds an environment value or a `declaration` row with a preview (a parameter or state field previewed by its default) (`index.ts`, foot rule). Spec, design and code comment reworded. New test "notes that a parameter's preview is its default…" opens a component parameter's popover (foot shown) and a `let`'s (foot hidden). A document `let` that reads `env.` is not flagged; that needs a service-side `fromDefault` flag and is left for later.
- **Verification:** Foot rule in `index.ts` now also shows for a `declaration` row with a preview; comment, viewer delta and the design risk bullet reworded. The new test opens `Box.body.size` (a parameter defaulting to 4, no environment rows) and sees the foot, and `root.body.size` (only the file's `step`) and sees none, so it tells the two rules apart. The fix follows the recommendation's alternative; the `let` that reads `env.` with no environment rows listed is still unflagged, which the Fix note states openly (its follow-up record and one stale design line are RF36). Viewer 271/271 pass.

### ✅ Verified - RF31 Search now matches typed words anywhere in a row's doc as substrings, so short queries keep unrelated rows
- **Severity:** Low
- **Confidence:** High
- **Evidence:** `data-search` is `${name} ${doc}` (`index.ts:4766`), and `nameHolds` tests `spelled.includes(word)` (`named.ts:224-227`). Name matching was already by substring, but a name's words are short identifiers. A doc is prose, so `red` now keeps any row whose doc says "covered", "preferred" or "reduced", and `on`, `in` and `a` keep almost every documented row. The match is also invisible: a row's doc shows only in the detail line while that row is active. The search also reads a doc that was left out of the detail line for restating the value (`/// 12`), so `12` keeps that row with nothing on screen to explain why. Nothing ranks name matches above doc matches. The "Searching descriptions" scenario (`ring`) passes, but no test checks that doc matching doesn't over-match.
- **Recommendation:** Match doc text by word prefix (split the doc on non-word characters and require `docWord.startsWith(word)`), and keep substring matching for names. Or list name matches first. Leave out a dropped doc from `data-search` too. Add a test where a short word such as `on` doesn't keep a row whose name lacks it and whose doc only contains it mid-word.
- **Fix:** Search data is now the name (`data-search`) plus only the doc the row shows (`data-search-doc`; a dropped doc is left out). New `rowHolds` in `named.ts` matches a word as a substring of the name or as the start of a word of the doc. The "Searching descriptions" test now also asserts `ing` doesn't keep `color.focus` while `ring` still does.
- **Verification:** `data-search` is the name and `data-search-doc` the shown doc only (a restating doc is left out, and the cut-name prefix that `#fitNamed` adds to `data-doc` does not leak into search). `rowHolds` keeps substring matching for names and word-prefix matching for doc words split on non-letters/digits. The test asserts `ing` no longer keeps `color.focus` while `ring` does. Spec and design wording match.

### ✅ Verified - RF32 A `let`'s button inside an element row's head is hidden while the `let` itself is selected, which is the state the popover leaves behind
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** For a `let` whose value is drawn as a row, the button goes into `.row-head` (`index.ts:1957-1964`). The stylesheet shows it on `[data-role="declaration"]:hover` and on `[data-role="declaration"]:has(> .row-element.selected)` (`styles.ts:1093-1094`). It does not show it on `[data-role="declaration"].selected`, which is the rule the inline `let` button relies on (`styles.ts:1092`). `#openNamed` selects the place's key, which is the declaration (`index.ts:4635-4637`, `place.key` = `blue`). So after opening and closing the popover with Escape, or after choosing a value, the `let` is selected but its button is hidden until the pointer returns. The viewer delta says the button shows while the place is "selected". The new test only checks that the button is inside `.row-head` (`test/named.test.ts:436`), not when it's visible.
- **Recommendation:** Add `.source [data-role="declaration"].selected > .row-element > .row-head > .named-anchor > .named-values` to the visible list, or fold both cases into one `:is(…)` selector. Add a test, or a check in the browser script, that the button is visible with the declaration selected.
- **Fix:** Added `.source [data-role="declaration"].selected > .row-element > .row-head > .named-anchor > .named-values` to the shown list in `styles.ts`. No automated check (jsdom computes no `:has`/visibility).
- **Verification:** The missing `[data-role="declaration"].selected > .row-element > .row-head > .named-anchor > .named-values` selector is in the visible list, next to the hover and `:has` ones, which is the state `#openNamed` leaves. No automated check, as the Fix note says (jsdom does not compute visibility); I read the selector against the button's DOM path and it matches.

### ✅ Verified - RF33 `#fitNamed`'s middle cut is unverified, and its comment promises an end-preserving cut it doesn't make
- **Severity:** Low
- **Confidence:** Medium
- **Evidence:** The doc comment and inline comment say the cut keeps "the end whole where possible, then the start" (`index.ts:4826-4830`, `4845`). The search actually keeps a fixed 60/40 tail/head split of characters (`tail = Math.ceil(keep * 0.6)`), with no regard for `.` boundaries. A long last segment is therefore cut too (`color.background.surface.ele…vated` style), even when dropping middle segments would keep it whole. Under jsdom, `getBoundingClientRect().width` is 0 and the function returns early (`:4835-4837`), so the unit tests never reach the cut. The browser check only asserts that no palette name is cut (`check-named-values-browser.mjs:122-125`). Neither suite covers a name that is too long, the `…` text, or the detail line's `name — doc` fallback that the spec requires ("the lines under the list SHALL then give the whole name"). The cut also replaces the option's text, so its accessible name becomes the truncated string, and no `aria-label` carries the whole name.
- **Recommendation:** Prefer cutting whole middle segments at `.` boundaries before cutting characters, or reword the comment to describe the ratio. Set `aria-label` on a cut row to its full name. Add a browser-check case with an over-long name (for example a deep record path in a fixture) that asserts the `…`, the kept suffix, and the detail line's full name.
- **Fix:** New `cutName` in `named.ts` tries `first.….last parts` (dropping whole middle parts), then `….last`, then a character middle cut keeping 60% tail; comments reworded. A cut row gets `aria-label` = whole name + preview, and its detail doc line leads with the whole name. Unit test "cuts a long name in whole middle parts before characters"; the browser check narrows the popover and asserts a cut name's `…`, kept end, `aria-label` and detail line.
- **Verification:** `cutName` drops whole middle parts first (`first.….last…`), then `….last`, then a character middle cut keeping 60% tail; doc comments now describe exactly that. A cut row gets `aria-label` (whole name + preview) and its detail line leads with the whole name. Unit test covers part and character cuts; the browser check narrows the popover and asserts the `…`, the kept end, the `aria-label` and the detail line, and passes. `#fitNamed` runs once per open, so the doc prefix is not doubled.

### ✅ Verified - RF34 The `wide` (outside Display P3) path has no test in the service or the viewer
- **Severity:** Low
- **Confidence:** High
- **Evidence:** The Rust test checks `nearest_hex` and `!outside_p3` for `standardPalette.green500` and no `nearest_hex` for an sRGB color (`suggestions_tests.rs:543-559`). No case has `outside_p3 == true`. The viewer tests assert the `P3` tag and the `· outside sRGB` text (`test/named.test.ts:222-228`), but never the `wide` tag or `outside sRGB and Display P3` (`index.ts:4751`, `4773-4775`). The spec requires both.
- **Recommendation:** Add a color outside P3 (for example `c.oklch(0.7, 0.4, 150)`) to the Rust fixture and assert `outside_p3` and `nearest_hex`. In the viewer, assert the `wide` tag and the exact-line wording.
- **Fix:** Rust fixture adds `let vivid: Color = { c.oklch(0.85, 0.4, 145) }` and asserts `outside_srgb`, `outside_p3` and a `nearest_hex`. The viewer CARD fixture adds the same `vivid`, and the test asserts its `wide` tag and `· outside sRGB and Display P3` exact line.
- **Verification:** Rust fixture's `vivid` (`c.oklch(0.85, 0.4, 145)`) asserts `outside_srgb`, `outside_p3` and a 7-char `nearest_hex`; `cargo test -p nx-language-service suggestions` 23/23 pass. The viewer test asserts the `wide` tag and the `· outside sRGB and Display P3` exact line.

### ✅ Verified - RF35 Small spec/design/code drift in this pass
- **Severity:** Low
- **Confidence:** High
- **Evidence:** (a) `index.ts:4708-4709` still describes subheadings ("read only their last part, under a subheading"), which 84fe9ac reverted. (b) The viewer delta's exact-value line names only `outside sRGB`, but the code writes `outside sRGB and Display P3` for a `wide` color (`index.ts:4773`). (c) The `let` popover's new placeholder (`Named g.Paint values for blue`, `index.ts:4652-4654`) and the button's place in a row's head (after the kind or title, before the pin) are in no delta. (d) `source.ts:1471-1475` now prefers a union case's own doc for the inspector's doc line, but `openspec/specs/viewer/spec.md:887` says the line is "the element's declaration['s] doc", and no delta changes it. (e) `design.md:182-185` describes the row preview without the `P3`/`wide` tag, the nearest hex, the detail lines or the sizing and cutting, and `design.md:213` still says "The footer says previews are defaults" without the new condition. (f) Task 5.3 still lists "hover cards", which rows no longer have. (g) `.popover .named-preview .swatch.outside-srgb` (`styles.ts:1176`) is now dead, because popover swatches are always drawn with `outsideSrgb` false (`index.ts:4756`).
- **Recommendation:** Drop the stale comment and the dead rule. Add the exact-line wording for `wide`, the placeholder, the row-head placement and the union-case doc line to the viewer delta (the last as a MODIFIED requirement). Bring `design.md` and task 5.3 up to date.
- **Fix:** (a) stale subheading comment removed; (g) dead `.swatch.outside-srgb` popover rule removed. (b) the viewer delta gives the `wide` wording; (c) it gives the placeholder (`Search named values` for a property, `Named <type> values for <name>` for a `let`) and the row-head placement; (d) a MODIFIED "Selecting an element opens its inspector under its row" requirement with a union-case-doc scenario; (e) `design.md` row and footer text brought up to date; (f) task 5.3 no longer says hover cards. The delta also now says how names are cut and searched.
- **Verification:** (a) and (g) removed; (b) and (c) in the viewer delta, placeholder wording matching `index.ts`; (d) the MODIFIED requirement copies the main spec's text and all four scenarios unchanged apart from the union-case doc sentence, plus a new scenario; (e) design row text and risk bullet updated; (f) task 5.3 updated. `openspec validate --strict` passes. One design line the fix missed is in RF36.

## New Findings Discovered During 2026-11-26 16:38 Verification

### ✅ Verified - RF36 The defaults-note fix leaves one stale design line, an unrecorded follow-up, and a dead export
- **Severity:** Low
- **Confidence:** High
- **Evidence:** (a) `design.md:139` still says the footer is "shown when environment values are listed", while the delta, the design risk bullet (`design.md:218-220`) and the code now also show it for a parameter or state field previewed by its default. (b) The RF30 Fix note leaves "a document `let` that reads `env.`" for later, behind a service-side `fromDefault` flag, but `specs/future.md`'s "What `add-visual-value-suggestions` Left For Later" has no entry for it. The "Previews that follow `provide`s" bullet is about what a running session knows, not about flagging when the note is missing. (c) `nameHolds` (`packages/viewer/src/named.ts:224`) is no longer called anywhere. `rowHolds` replaced it, and only `rowHolds`' doc comment still names it.
- **Recommendation:** Reword `design.md:139` to the new condition. Add a future.md bullet for flagging previews that followed an `env.` read so the note shows for them. Remove `nameHolds`, or have `rowHolds` call it for the name half, and fix the doc comment to match.
- **Fix:** `design.md:139` now says the footer shows when environment values, or parameters previewed by their defaults, are listed. `specs/future.md` (value suggestions follow-ups) records the defaults note for a `let` that reads `env.` with a `preview.fromDefault` idea. The unused `nameHolds` is removed and `rowHolds`' comment no longer names it.
- **Verification:** (a) `design.md:139` now gives the new condition, matching the delta and the risk bullet. It says "parameters" where the delta says "a parameter or state field", which is close enough for a design note. (b) The new `specs/future.md` bullet sits under the value suggestions follow-ups and describes the unflagged `env.`-reading `let` and the `preview.fromDefault` idea. (c) `nameHolds` is gone, nothing references it, and `rowHolds`' comment describes the name match itself. The viewer build and 271/271 tests pass.


## Questions (2026-11-26 pass)
- RF30: should a parameter's default preview carry the note too, or should declaration rows show no preview at all, since their value comes from the caller?

## Summary (2026-11-26 pass)
- The polish is careful, and all suites pass, including the browser check and the examples' `.out.nx`. `roundedColor`, the nearest-hex and `outside_p3` plumbing, the union-case doc and the row-head placement behave as intended in the cases probed.
- One Medium: the defaults note is now hidden for lists whose previews are still defaults (RF30).
- The rest are Low: doc search over-matches (RF31), the row-head button is hidden while its `let` is selected (RF32), the middle cut is untested and its comment is wrong (RF33), the `wide` path is untested (RF34), and assorted doc and spec drift (RF35).
