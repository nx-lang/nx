# Review: NX UI MVP Object Model Proposal

**Reviewed document:** [NX-Drawn-UI-MVP-Object-Model-Proposal.md](NX-Drawn-UI-MVP-Object-Model-Proposal.md)
**Review date:** August 20, 2026
**Focus:** the proposed component types and properties — completeness, internal consistency, and the accuracy of the third-party precedents the design leans on.

Findings are numbered `RF1`…`RF39`. **RF1–RF22, RF38, and RF39 are applied in place**; the proposal has already been edited. **RF24–RF37 are suggestions** that were not applied, either because they are naming or scoping calls that belong to the author, or because they need an eval result the review cannot supply. **RF23 is superseded by RF39**, which resolved the same problem a different way. Where a suggestion has a home in the proposal's own §12 list, that entry was added so the open question is tracked in the document itself.

Sources consulted first-hand are listed in the appendix, along with three factual claims in the original draft that did not survive checking.

---

## Summary

| # | Area | Status | Finding |
|---|---|---|---|
| RF1 | §4.1 | Applied | Add a `format` discriminator; make `schemaVersion` semver |
| RF2 | §4.1 | Applied | Define a media type and file extension |
| RF3 | §4.4 | Applied | Constrain `ElementId` to a JSON-Pointer-safe charset |
| RF4 | §4.5 | Applied | Close the containment rules in both directions |
| RF5 | §5.1 | Applied | Allow partial `Insets`; state percentage resolution |
| RF6 | §5.2 | Applied | **Correction:** DrawnUI `SkiaGradient` has no `StartColor`/`EndColor` |
| RF7 | §5.2 | Applied | Require ordered gradient stops |
| RF8 | §5.3 | Applied | Define transform composition order and per-transform defaults |
| RF9 | §6.1 | Applied | Specify `opacity` as group opacity |
| RF10 | §6.1 | Applied | Specify `clipPath` coordinate space and fill rule |
| RF11 | §6.1 | Applied | **Correction:** SVG does fill `polyline` by default |
| RF12 | §6.2 | Applied | Replace `preserveAspectRatio` with the `fit` vocabulary; define intrinsic size |
| RF13 | §6.12, §7.11 | Applied | Require `alt` on both images; add `scaleDown`; resolve the `alt`/`accessibility.label` conflict |
| RF14 | §7.1 | Applied | Bind `alignSelf`/`justifySelf` to physical axes and tabulate per-parent meaning |
| RF15 | §7.2–7.3 | Applied | Define `justify` against `grow`, and `gap` under `wrap` |
| RF16 | §7.4 | Applied | Define auto-placement, overlap, and out-of-range placement |
| RF17 | §7.5 | Applied | Rename `ZStack`'s alignment props to match `Grid`; **correction:** json-render has no built-in catalog |
| RF18 | §7.6 | Applied | Define how a `Scroll` measures its child |
| RF19 | §7.10 | Applied | Make `variant` carry heading semantics; define `variant` under `format: "markdown"` |
| RF20 | §7.12 | Applied | Define `size` precedence; **correction:** A2UI's `Icon` does admit raw `svgPath` |
| RF21 | §8 | Applied | Remove the hard-coded caption color from the example |
| RF22 | §2, §9, §11, §12, §14 | Applied | Track A2UI v0.9.1/v1.0, add OpenUI, add the `SkiaGradient` mapping row, extend deferred and open-question lists |
| RF23 | §7.5 | Superseded | Rename `nx.ui.Stack` to `nx.ui.Overlay` — see RF39 |
| RF24 | §7.1 | Suggested | Trim decoration off leaf components |
| RF25 | §5.2 | Suggested | Add semantic color roles |
| RF26 | §6.10 | Suggested | Default `Path` fill to `"none"` |
| RF27 | §3, §4 | Suggested | Measure encoding cost before locking the flat map as the wire form |
| RF28 | §6.11 | Suggested | Wrapped text in drawing coordinates is a real MVP gap |
| RF29 | §7.1 | Suggested | `minWidth`/`maxWidth` should be `Length`, not `number` |
| RF30 | §7.1, §7.11 | Suggested | Promote `aspectRatio` to `UiCommonProps` |
| RF31 | §7.10 | Suggested | Emphasis is reachable only through Markdown |
| RF32 | §7.10 | Suggested | Reconcile the `variant` ramp with A2UI's |
| RF33 | §7.9 | Suggested | `Divider` ignoring common properties is a wart |
| RF34 | §6.3 | Suggested | Consider inheritable paint on `Group` |
| RF35 | §4.2 | Suggested | Exact catalog versions make every upgrade breaking |
| RF36 | §7.12 | Suggested | Icon names cannot be validated statically |
| RF37 | §4.5, §10 | Suggested | Strict unknown-property rejection needs an extension escape hatch |
| RF38 | title, §1, §4.2, §6, §7, §9 | Applied | Naming: `nx.draw` → `nx.graphics`, `Drawing` kept over `Canvas`, terminology layer rule, reserved catalog IDs |
| RF39 | §1, §7.1–§7.5, §8, §9, §12.3 | Applied | Layout containers renamed `Row`/`Column`/`Stack` → `HStack`/`VStack`/`ZStack` |

---

## Part A — Applied in place

### RF1 — `format` discriminator and semver `schemaVersion` (§4.1)

The document had `schemaVersion: "0.1"` while catalogs used `0.1.0`, and no way to tell an NX UI document from any other JSON. The sibling artifact format in this repo ([nx-ir-format.md](nx-ir-format.md)) already establishes the convention: `format: "nx-ir-json"` plus `schemaVersion`. Added `format: "nx-ui-json"` and moved `schemaVersion` to `"0.1.0"` so one version syntax is used throughout.

### RF2 — Media type and file extension (§4.1)

Nothing named the serialized artifact. A2UI standardized `application/a2ui+json` in v1.0 for the practical reason that transports and caches otherwise have to sniff payloads. Added `application/nx-ui+json` and `.nxui.json`, which also matches the existing `.nxir.json` extension convention.

### RF3 — `ElementId` charset (§4.4)

"Non-empty strings, unique within a document" admits IDs containing `/` and `~`, which are exactly the characters RFC 6901 escapes — and the proposal's own roadmap is JSON Patch streaming addressed by element ID. It also admits whitespace-only IDs. Constrained to `^[A-Za-z_][A-Za-z0-9_.-]{0,63}$`. A2UI v1.0 reached for the same kind of rule (UAX #31 identifiers) for catalog entity names.

### RF4 — Containment rules closed in both directions (§4.5)

The rules said where `nx.graphics.*` may appear but never answered two questions a generating model will hit immediately: may `nx.ui.Text` appear inside a `Drawing` (there is no `foreignObject` in this MVP — no), and may a `Drawing` nest inside a `Drawing` (nested view boxes are deferred — no). Both are now stated. Also noted that these rules belong in the catalog schema rather than in prose: A2UI v1.0 added `allowedParents`/`allowedChildren` to catalog component definitions precisely because a flat adjacency list of ID references cannot express child-type restrictions in JSON Schema alone. That is the mechanism NX will need too.

### RF5 — Partial `Insets`, and percentage resolution (§5.1)

`Insets` required all four sides whenever the object form was used, so "8px of top padding" cost four properties. Sides are now optional and default to `0`. Also documented that a percentage `Length` against an intrinsically sized parent resolves as `auto`, which closes an otherwise circular constraint, and acknowledged the deliberate `left`/`right` (physical) versus `start`/`end` (logical) split, with logical insets added to the deferred list.

### RF6 — DrawnUI gradient mapping was wrong (§5.2)

The draft said adapters "will translate its `StartColor`/`EndColor` representation into ordered stops." [`SkiaGradient`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/SkiaGradient.cs) has no such properties — it carries `Colors` (`IList<Color>`) and `ColorPositions` (`IList<double>`), plus `StartXRatio`/`StartYRatio`/`EndXRatio`/`EndYRatio`, `Angle`, `TileMode`, and `Type`. (`StartColor`/`EndColor` do exist, but on `SkiaImage`, as a tint-overlay feature.) This matters in NX's favor twice over: the parallel arrays zip into ordered stops losslessly, and the ratio-based endpoints *are* `objectBoundingBox` coordinates, which is independent support for the default the proposal picks in §12.4. Corrected, and a `SkiaGradient` row was added to the §9 adapter table.

### RF7 — Gradient stop ordering (§5.2)

`stops: GradientStop[]` had no cardinality or ordering rule, leaving "one stop" and "descending offsets" as undefined behavior across renderers. Now: at least two stops, non-decreasing offsets, renderers clamp but never reorder.

### RF8 — Transform composition order and defaults (§5.3)

"Transforms are applied in array order" is ambiguous — it is the sentence every transform bug starts with. Specified as equivalent to an SVG `transform` list read left to right, so the last entry is applied first to the geometry, and pinned the two silent defaults (`scale.y` defaults to `x`; `rotate` center defaults to the origin, not the shape's center) plus the fact that strokes scale with transforms, since there is no `vector-effect` in the MVP.

### RF9 — `opacity` is group opacity (§6.1)

"Node and descendant opacity" reads equally well as a per-descendant alpha multiplier or as one composited layer, and the two differ visibly wherever descendants overlap — a very common case in generated illustrations. Pinned to group opacity, which SVG, Skia, and Canvas 2D all provide.

### RF10 — `clipPath` semantics (§6.1)

Path data as a clip needed three things it did not have: which coordinate space it is in (the node's local space, after the node's own `transform`), what it clips (the node and its descendants), and which fill rule applies (`nonzero`, regardless of the node's `fillRule`, matching SVG's `clip-rule` default).

### RF11 — The SVG polyline-fill claim was backwards (§6.1)

The draft justified `fill: "none"` on `Line` and `Polyline` as "matching SVG's familiar behavior." SVG's initial `fill` is black for *every* shape including `polyline`; a stroked open polyline in SVG arrives with a filled region, which is a well-known surprise. NX's choice is better than SVG's — it just is not SVG's. Reframed as a deliberate divergence, which also sets up RF26.

### RF12 — `Drawing` fit vocabulary and intrinsic size (§6.2)

Two issues. First, the same document used `object-fit` names (`contain`/`cover`/`fill`/`none`) for images and SVG names (`meet`/`slice`/`none`) for the drawing surface, so a model had to learn two fit vocabularies with overlapping-but-different values. Since the mapping is exactly one-to-one, `preserveAspectRatio` is now `fit: "contain" | "cover" | "fill"`, with the SVG equivalence documented. Second, nothing said what a `Drawing` measures to when `width`/`height` are `auto` — added: intrinsic size and aspect ratio come from the view box, and content is always clipped to the surface box.

### RF13 — Image accessibility and `fit` parity (§6.12, §7.11)

`nx.ui.Image` required `alt` while `nx.graphics.Image` had no equivalent, so the same picture was accessible or not depending on which catalog drew it; `alt` also silently competed with `accessibility.label` as a second source of accessible name. Now `alt` is required on both, `alt: ""` is defined as exactly equivalent to `accessibility.hidden: true`, and supplying both `alt` and `accessibility.label` is a validation error rather than a renderer-specific tiebreak. Both components also gained `scaleDown` (contain, never upscale), which CSS `object-fit` and A2UI's `Image.fit` both carry and which is the right default behavior for a generated image of unknown intrinsic size.

### RF14 — `alignSelf`/`justifySelf` had no fixed meaning (§7.1)

They were defined relative to the parent's axes ("cross-axis/grid alignment", "the other axis"), which is undefined for `Stack`, `Box`, and `Scroll`, and forces a generating model to reason about its parent's type before choosing a property name. Both are now bound to physical axes — `justifySelf` horizontal, `alignSelf` vertical — with a table giving the meaning under each container type, and a rule that a property the parent ignores is a diagnostic rather than a validation error, so subtrees stay valid when re-parented. Also pinned the over-constrained case (`minWidth` wins, as in CSS).

### RF15 — Distribution against `grow`, and `gap` under `wrap` (§7.2, §7.3)

Two undefined interactions between properties that models combine freely: what `justify: "spaceBetween"` does when a child has `grow: 2` (distribution applies to the space left after `grow`), and whether `gap` separates wrapped lines as well as siblings (it does; per-line distribution à la `align-content` is deferred). The Row/Column rationale was also made concrete against the real A2UI catalog — its `justify` and `align` enums and defaults match NX's exactly, and its child sizing is a single `weight`, which is why NX's `grow`/`shrink` pair is a genuine addition rather than a rename.

### RF16 — Grid placement rules (§7.4)

"Auto-placement is row-major" left three cases open: whether auto-placement flows through cells claimed by explicitly placed children (it skips them), what happens when two children claim the same cell (they overlap and paint in `children` order), and what an out-of-range placement means (past the last column is an error; past the last row creates implicit `auto` rows, consistent with `rows` being optional).

### RF17 — Stack alignment naming, and a bad citation (§7.5)

`Stack` used `horizontalAlignment`/`verticalAlignment` while `Grid` used `justifyItems`/`alignItems` for the identical concept, and children override both with `justifySelf`/`alignSelf`. Renamed to match `Grid`. Separately, the rationale claimed the overlay reading matches "json-render's common catalog wording" — json-render ships no built-in catalog at all (a catalog is something you define), so that citation was replaced with the evidence that actually exists, including the DrawnUI naming inversion described in RF23.

### RF18 — How a `Scroll` measures (§7.6)

A viewport with one child is meaningless without a measurement rule. Added: the child is measured unbounded along each scrollable axis and viewport-constrained on the others, child percentages resolve against the viewport rather than the content, and a `Scroll` never sizes to content along a scrollable axis — so it requires a bounded extent from its parent, which is the failure a first renderer will otherwise hit.

### RF19 — `variant` carries the document's only semantics (§7.10)

Nothing in a read-only MVP conveys structure to assistive technology except `Text.variant`, but the document described it only as a "theme typography role" — so a model that sets `fontSize: 28` instead of `variant: "h2"` produces a flat, unnavigable document that still validates. Renderers are now required to expose `h1`–`h3` as real headings and `code` with a code role. Also defined `variant` under `format: "markdown"` as the baseline role that Markdown structure nests below, so heading syntax inside a caption cannot escape its typography scale.

### RF20 — Icon sizing, and the A2UI icon claim (§7.12)

`size` and the inherited `width`/`height` could both be set with no stated precedence; explicit dimensions now win. The rationale also said A2UI "intentionally uses system-provided icons from a predefined list" — true but incomplete: A2UI's `Icon.name` is a union of the predefined enum *and* an `{ "svgPath": ... }` escape hatch. Reframed so NX's decision reads as the deliberate narrowing it is, with the reason stated (a second, unmeasured path-data channel adds attack surface but no capability, because `nx.graphics` already exists).

### RF21 — The example modeled the wrong habit (§8)

The example's caption carried `"color": "#555"`, which is legible on the light surface a model imagines and nearly invisible on a dark one, in a document whose whole theming story is host-owned. Removed, with a short note on when literal colors are and are not appropriate — the white curve in the same example is fine, because that drawing supplies its own ground. See RF25 for making the distinction enforceable.

### RF22 — Precedents, mapping, and open questions refreshed (§2, §9, §11, §12, §14)

- A2UI is cited throughout as v0.9. v0.9.1 is the current spec and **v1.0 is a release candidate**; the table now cites both, and calls out the two v1.0 changes that bear on this design (catalog-declared `allowedParents`/`allowedChildren`, and the removal of protocol-level `theme`/`primaryColor` to separate layout from branding — which is the same instinct as NX's host-owned theming).
- Added an **OpenUI / OpenUI Lang** row. It was named in the original brief alongside DrawnUI but appears nowhere in the proposal, and it is the one precedent that argues about the *encoding* rather than the object model (see RF27). It is also from thesys, the same vendor as C1, which the table already lists.
- Added the `SkiaGradient` row to the §9 adapter mapping (RF6).
- §11 gained the DrawnUI shape types NX drops (`Arc`, `Squricle`), inheritable group paint, text decoration/transform, wrapped drawing text, logical insets, and per-element extension metadata.
- §12 gained four new decisions (RF24–RF27) and a sharper overlay-naming entry (RF23).

---

## Part B — Suggestions not applied

### RF23 — Rename `nx.ui.Stack` to `nx.ui.Overlay` (superseded by RF39)

**Superseded.** The problem this finding identified is real and was fixed, but by renaming the whole container family rather than the overlay alone; see RF39. The evidence below stands and is what motivated that decision.

The proposal defines `Stack` as z-order overlay and acknowledges the term is contested. The evidence is worse than the draft assumed, in the direction that matters most: **DrawnUI's own `SkiaStack` is `LayoutType.Column`** (`src/Shared/DrawnUi/Features/Fluent/SkiaStack.cs`), OpenUI's default library uses `Stack` as its root layout component, and MUI's `Stack` is linear. An NX-to-DrawnUI adapter would map `nx.ui.Stack` → `SkiaLayer` and `nx.ui.Column` → `SkiaStack`, an inversion that is easy to misread in exactly the ecosystem NX is evolving from. Flutter's `Stack` and SwiftUI's `ZStack` support the overlay reading, so this is genuinely split — but `Overlay` is unambiguous to both camps and costs one catalog entry now versus a systematic generation error later. Not applied because renaming a headline component is the author's call; §12.3 now carries the evidence and a recommendation to rename unless an eval is decisive.

### RF24 — Trim decoration off leaf components

`UiCommonProps` applies roughly twenty properties to every Level 2 component, `Text` and `Icon` and `Divider` included. A2UI's `ComponentCommon` carries only `id` and `accessibility` (plus `weight` on catalog components) and leaves all decoration to the host theme; v1.0 went further and removed theming from the protocol entirely. NX has a `Box` whose entire purpose is decoration, so `background`/`border`/`cornerRadius`/`shadows`/`clip` on a `Text` is a second way to express what `Box` already expresses. Recommendation: keep sizing, spacing, alignment, opacity, and accessibility universal; restrict decoration to containers. The counter-argument is real — it costs an extra element and ID per decorated leaf — which is why this is an eval, now tracked as §12.8.

### RF25 — Semantic color roles

Every `Color` in the model is a literal. A document generated against an imagined light theme is unreadable on a dark one, and RF21 shows the draft's own example fell into it. A small closed set of roles (`surface`, `onSurface`, `muted`, `accent`, …) accepted anywhere `Color` is, resolved by the host theme, would make the right thing expressible — and would let a validator warn when literal colors are used for themed content. This is cheap to add now and expensive to retrofit once documents exist. Not applied because it adds a value type and a theming contract to an MVP that deliberately has none; tracked as §12.9.

### RF26 — Default `Path` fill to `"none"`

`Path` inherits SVG's black default fill. In generated illustrations the dominant use of `Path` is an open stroked curve, and the dominant symptom of the SVG default is that such a curve arrives filled — the proposal's own example has to write `"fill": "none"` explicitly to draw a line. Since `Polyline` already diverges from SVG for this exact reason (RF11), `Path` diverging too would be more consistent, not less. Not applied because it trades SVG fidelity for generation ergonomics and deserves a measurement; tracked as §12.7.

### RF27 — Measure encoding cost before locking the flat map as the wire form

The flat map costs an ID declaration, an ID reference, and a `type`/`props`/`children` scaffold for every element. That is a real per-element tax on generation latency and cost, and OpenUI Lang exists on the premise that it is large enough to justify a purpose-built streaming syntax. The flat map is clearly right as the *validation and patch* representation; whether it should also be the *generation* representation is an open question the proposal currently answers by assumption. §12.1 already asks about nested authoring syntax; §12.10 now adds the token measurement explicitly.

### RF28 — Wrapped text in drawing coordinates

`nx.graphics.Text` is single-line or explicitly line-broken, and `nx.ui.*` cannot appear inside a `Drawing` (RF4). Together these mean the most common thing anyone draws — a labeled box in a diagram — has no way to wrap its label. The model can only guess line breaks at generation time, without knowing the font. This is the deferral most likely to make the MVP feel unfinished in its headline use case. Options, in increasing cost: add `wrapWidth` (or `maxWidth` + `lineHeight`) to `nx.graphics.Text`; or allow a `Box`/`Text` subtree inside `Drawing` at fixed coordinates (an explicit `foreignObject` equivalent). Recommend the first. Not applied because it expands Level 1's text model, which §11 currently defers wholesale.

### RF29 — `minWidth`/`maxWidth`/`minHeight`/`maxHeight` should accept `Length`

`width`/`height` are `Length` (number, `auto`, or percentage) but the min/max bounds are plain `number`, so `maxWidth: "60%"` — an extremely ordinary constraint, and the usual way to make generated layouts responsive — is unrepresentable. Recommend `Length` minus `"auto"` for all four. Not applied only because the percentage-resolution rules for min/max under intrinsic sizing need to be written alongside it.

### RF30 — Promote `aspectRatio` to `UiCommonProps`

`aspectRatio` exists only on `nx.ui.Image`, but the need is general: a `Box` reserving 16:9 for a media slot, or a `Drawing` whose view box implies a ratio (§6.2 now defines exactly that behavior for `Drawing`, in effect a special-cased `aspectRatio`). Promoting it would remove the special case and cover the common layout need. Not applied because it interacts with the min/max question in RF29 and should land with it.

### RF31 — Emphasis is reachable only through Markdown

`nx.ui.Text` has no `textDecoration`, and no span model, so bold, italic, underline, and strikethrough inside a sentence are expressible only by switching `format` to `"markdown"` — which simultaneously enables link and list parsing and obliges the host to sanitize. The MVP has effectively made Markdown load-bearing for basic emphasis while treating it as an optional convenience. Either accept that explicitly (and say so where `format` is defined), or add the minimal span model. `textDecoration` alone would not fix it, since the need is per-run, not per-block.

### RF32 — Reconcile the `variant` ramp with A2UI's

NX offers `h1`/`h2`/`h3`/`title`/`body`/`caption`/`code`; A2UI's basic catalog offers `h1`–`h5`/`body`/`caption`. The overlap is close enough that a model trained on A2UI will emit `h4` or `h5` into NX and fail validation (§4.5 makes unknown enum values invalid), and `title` has no A2UI counterpart while overlapping `h1` in meaning. Recommend either adopting `h1`–`h5` and dropping `title`, or documenting the fallback explicitly. Not applied because it is a design-system decision and the `code` variant is a deliberate NX addition worth keeping either way.

### RF33 — `Divider` ignoring common properties is a wart

`Divider` accepts all of `UiCommonProps` but documents that `background` and `border` are ignored, then reintroduces the same capability as `color` and `thickness`. Two spellings for one concept, one of which silently does nothing. Alternatives: drop `color`/`thickness` and let a divider be a `Box` with `background` and a `height` (loses the theme default and the semantic separator role), or keep them and state that they *override* the common properties rather than that the common ones are ignored. The second is a one-line fix but changes documented behavior, so it is flagged rather than applied.

### RF34 — Consider inheritable paint on `Group`

Every shape carries its own `fill` and `stroke`. In SVG, paint inherits, which is why real-world SVG sets `fill` once on a `<g>` and omits it on fifty children. For a token-metered generator that difference is significant, and it recurs in every multi-part illustration. Recommend evaluating inheritable `fill`/`stroke` on `Group` (with explicit child values winning) against the cost of introducing inheritance into an otherwise fully explicit model — inheritance also makes partial-document replacement harder to reason about, which is why it is not an obvious win.

### RF35 — Exact catalog versions make every upgrade breaking

`CatalogUse.version` is an exact version and §4.5 rejects unknown properties and enum values in strict mode. Taken together, a renderer at `nx.ui@0.1.1` must reject a document written against `0.1.0`, and negotiation is deferred to post-MVP — so in the MVP window the only workable answer is that every producer and renderer upgrade in lockstep. Recommend stating a compatibility rule now (for example: a renderer accepts any document whose catalog version is compatible under semver with one it implements, and additive minor versions may not change the meaning of existing properties). That is a policy sentence, not a mechanism, and it is what makes the exact version reproducible rather than brittle.

### RF36 — Icon names cannot be validated statically

`Icon.name` comes from "the negotiated host icon catalog," but `catalogs` carries only `nx.ui` and `nx.graphics` versions, and negotiation is post-MVP. So the one property whose valid values are host-defined is also the one thing a validator cannot check, and §10's "validate the entire document before rendering" cannot be satisfied for it. Recommend either fixing a small enumerated MVP icon set inside `nx.ui@0.1.0` (A2UI's approach — its icon names are a closed enum in the catalog schema), or making the icon set an explicit `CatalogUse` entry so the document declares what it was generated against. The current documented fallback glyph handles the runtime case but not the validation-time one.

### RF37 — Strict unknown-property rejection needs an escape hatch or a stated policy

§4.5 rejects unknown properties, and "a newer optional property can be ignored only after a schema-version policy explicitly permits it" — but that policy is not written, so today the only conforming behavior is rejection. That makes any additive change a breaking change for every existing renderer, which will bite during exactly the period when the catalog is changing fastest. A2UI v1.0 addressed the adjacent need with `metadata.extensions` under a reserved namespace. Recommend writing the forward-compatibility rule as part of the MVP (RF35 is the same gap from the version side) even if the extension slot itself stays deferred.

### RF38 — Naming (applied)

Reviewed separately after the first pass, and applied across the proposal.

**`nx.draw` → `nx.graphics`.** `draw` is a verb, and §6 specifies a retained scene tree rather than a command stream, so an action-shaped namespace invites the imperative reading the section has to disclaim. `graphics` is also the settled word for the domain in the ecosystems NX borrows from — `Microsoft.Maui.Graphics`, `androidx.compose.ui.graphics`, `android.graphics`, and SVG itself. `drawing` fixes the part of speech but carries the `System.Drawing`/GDI+ association. Token cost against `draw` is a wash. `DrawCommonProps` became `GraphicsCommonProps` for the same reason.

**`Drawing` kept as the root graphics component; `Canvas` rejected.** This one was initially argued the other way in review, on familiarity grounds, and the argument does not hold up:

- Every `Canvas` in the surrounding ecosystems is an **immediate-mode painting surface** — HTML's yields a `CanvasRenderingContext2D`, MAUI's an `ICanvas` inside `IDrawable.Draw`, Compose's a `DrawScope`, SwiftUI's a `GraphicsContext`. NX's element is a retained, addressable, patchable tree, which is a different thing wearing a familiar name.
- The precedent that actually matches is WPF's [`System.Windows.Media.Drawing`](https://learn.microsoft.com/en-us/dotnet/api/system.windows.media.drawing): "abstract class that describes a 2-D drawing," retained and freezable, with `DrawingGroup`/`GeometryDrawing`/`ImageDrawing`/`GlyphRunDrawing` beneath it, documented as providing no layout, input, or focus. That is this catalog's contract, and `DrawingGroup` sits where NX's `Group` sits.
- Reusing `Canvas` would collide inside NX's own §9 adapter table: DrawnUI's `Canvas` hosts an entire drawn UI tree and corresponds to the *document*, while `nx.graphics.Drawing` is an embeddable element. Two unrelated Canvases in the table meant to prevent conflation. §9 now says this explicitly and carries a row for the fact that DrawnUI has no embeddable coordinate-space element at all.

The generalization was added to §1 as a rule: **no word that names a renderer object appears in the portable model.** `Canvas`, `draw`, and `paint` are reserved for the renderer layer; the document uses `Drawing`, `Group`, and `Shape`.

**`nx.ui` kept, with the axis stated.** Strictly it is a category error — graphics are UI too, so `ui` is not a true sibling of `graphics`. The honest axis is *who decides position*: the layout engine in `nx.ui`, the document in `nx.graphics`. Rather than rename the most model-predictable prefix available, §7 now states the axis and derives the duplicate `Text`/`Image`/`Box`-vs-`Rect` pairs from it, so they read as a consequence of the split rather than an accident of it.

**Reserved catalog IDs.** §4.2 now reserves `nx.input`, `nx.data`, and `nx.<domain>` alongside the two MVP catalogs, so `nx.ui` never has to become an umbrella (`nx.ui.input`) later — which would rename every existing component type.

**Technology name.** The document is retitled *NX UI MVP Object Model*; "NX Drawn UI" is recorded as a superseded working title. It was DrawnUI's product name applied to a format the proposal explicitly declines to copy at the wire level, and it named an implementation technique rather than the artifact. The format already had better names in `NxUiDocument`, `application/nx-ui+json`, and `.nxui.json`. "DrawnUI" now appears only where it names the actual dependency — the **NX DrawnUI Renderer**. Note that the two file names in `docs/` were left unchanged to avoid breaking existing links.

### RF39 — Layout container naming: `HStack` / `VStack` / `ZStack` (applied)

RF23 recommended renaming the overlay container because an unprefixed `Stack` reads as *linear* nearly everywhere. Following that thread further turned up a second, independent problem in the same three components, and one rename fixes both.

**The second problem: `Row`/`Column` collide with `Grid`.** §7.4 `Grid` takes `rows`, `columns`, `rowGap`, and `columnGap`, and gives children `gridRow`, `gridColumn`, `rowSpan`, and `columnSpan`. With linear containers also named `Row` and `Column`, a valid document reads `{"type": "Row", "gridRow": 2}` — the same word meaning two things three tokens apart. The generation failure this invites is concrete and testable: in [Bootstrap](https://getbootstrap.com/docs/5.3/layout/grid/) ("rows are wrappers for columns") and [Ant Design](https://ant.design/components/grid) ("only `col` should be placed directly in `row`"), `row`/`col` are *mandatory paired grid parts*, not standalone containers, so a model carrying web priors can reasonably emit `Column` children inside a `Grid` to make cells.

Flutter is the instructive case, because it faced the same collision and solved it by quarantining the vocabulary rather than renaming anything: [`GridView`](https://api.flutter.dev/flutter/widgets/GridView-class.html) takes `crossAxisCount`/`mainAxisSpacing`/`crossAxisSpacing` and never says "row" or "column"; [`Wrap`](https://api.flutter.dev/flutter/widgets/Wrap-class.html) coins "**run**" for a wrapped line; and the words reappear only in [`Table`](https://api.flutter.dev/flutter/widgets/Table-class.html) (`TableRow`, `columnWidths`), which is a table. NX cannot copy that, because its `Grid` is a CSS Grid derivative and CSS Grid vocabulary is the highest-value prior a generator has for 2D placement — confirmed by [`flutter_layout_grid`](https://pub.dev/packages/flutter_layout_grid), the community package that exists because Flutter core lacks CSS Grid semantics and which immediately reintroduces `columnSizes`/`rowSizes`/`columnGap`/`rowGap`/`GridPlacement`. Renaming the linear containers is the cheaper side of that trade.

**The survey behind the choice.** Unprefixed "stack" means *linear* — and almost always vertical by default — in [MUI](https://mui.com/material-ui/react-stack/), Fluent UI v8, [Mantine](https://mantine.dev/core/stack/) ("a vertical flex container"), [Atlassian](https://atlassian.design/components/primitives/stack/examples), [Braid](https://seek-oss.github.io/braid-design-system/components/Stack), IBM Carbon, [Primer](https://primer.style/components/stack), [Polaris](https://polaris-react.shopify.com/components/layout-and-structure/block-stack) (`BlockStack`/`InlineStack`), WPF/WinUI/Avalonia `StackPanel`, .NET MAUI `StackLayout`, `UIStackView`/`NSStackView`, [Bootstrap](https://getbootstrap.com/docs/5.3/helpers/stacks/) `.vstack`/`.hstack`, [Every Layout](https://every-layout.dev/layouts/stack/), OpenUI's default library — and in **DrawnUI itself**, where `SkiaStack` is `LayoutType.Column`. The axis-prefixed form comes from [SwiftUI](https://developer.apple.com/documentation/swiftui/zstack), Tamagui (`XStack`/`YStack`/`ZStack`), and Chakra; the adjacent "Box" family (JavaFX `HBox`/`VBox`, Swing, GTK) is the same shape with a different noun. `Row`/`Column` as standalone containers is narrower than it appears — essentially a Flutter / Jetpack Compose / Qt Quick dialect that A2UI adopted.

**Applied:** `nx.ui.Row` → `nx.ui.HStack`, `nx.ui.Column` → `nx.ui.VStack`, `nx.ui.Stack` → `nx.ui.ZStack`, across §Executive recommendation, §3, §4.1, §4.3, §7.1's per-parent alignment table, §7.2–§7.5, §8's example, and §9's adapter table. §1's terminology table gains a row for the family. §7 gains a "Why `HStack` / `VStack` / `ZStack`" subsection carrying the three arguments — one naming convention rather than two, `row`/`column` left meaning grid structure, and the DrawnUI inversion resolved (`VStack` → `SkiaStack`, `ZStack` → `SkiaLayer`, which now read correctly in §9). §7.2/§7.3's `wrap` description changed from "additional rows"/"additional columns" to "additional lines," which also removes the `Column`-containing-columns phrasing. §12.3 was rewritten from an open naming question into a concrete eval with two named error classes to count.

**The cost, stated in the proposal rather than hidden:** NX gives up name-level alignment with A2UI's basic catalog, the GenUI peer it otherwise tracks most closely, and takes the minority *form* within the stack camp — the more common shape today is a single `Stack` plus a `direction` prop (MUI, Primer, Carbon, Polaris's current `s-stack`), and Fluent v9 dropped layout components altogether. A direction prop was rejected as worse for a generator choosing from an enumerated catalog and as an extra property on every container. Since A2UI catalogs are per-application and interop needs a mapping rather than matching identifiers, the loss is in generator familiarity only — which is exactly what §12.3 now asks the prototype to measure.

---

## Appendix — what was verified

Checked first-hand while reviewing, rather than taken from the draft:

- **A2UI**: [v0.9.1 spec](https://a2ui.org/specification/v0.9.1-a2ui/) (marked current) and [v1.0 spec](https://a2ui.org/specification/v1.0-a2ui/) (marked release candidate); the [v0.9→v1.0 evolution guide](https://github.com/google/A2UI/blob/main/specification/v1_0/docs/evolution_guide.md); and the generated [basic catalog component definitions](https://github.com/google/A2UI/blob/main/agent_sdks/python/a2ui_core/src/a2ui/core/basic_catalog/components.py), which is where `Row`/`Column` `justify`/`align` enums, `weight`, `Image.fit`, `Text.variant`, `Divider.axis`, `Card.child`, `Icon.name`, and `AccessibilityAttributes` were read.
- **json-render**: the [spec anatomy](https://json-render.dev/docs/specs) and [catalog docs](https://json-render.dev/docs/catalog) — `root` + `elements` + `{type, props, children, slots, visible}` confirmed; **no built-in component catalog** confirmed.
- **OpenUI**: [OpenUI Lang](https://www.openui.com/docs/openui-lang/overview) and the [React library reference](https://www.openui.com/docs/api-reference/react-ui) — Zod-defined libraries, streaming line-oriented parser, `Stack` as the default library root.
- **DrawnUI**: `ShapeType` (`Rectangle`, `Circle`, `Ellipse`, `Arc`, `Squricle`, `Path`, `Polygon`, `Line`, `Custom`), `LayoutType` (`Absolute`, `Column`, `Row`, `Wrap`, `Grid`), `SkiaGradient` members, `SkiaStack` = `LayoutType.Column`, `SkiaLayer : SkiaLayout`, `ContentLayout.Content`, `SkiaScroll.Orientation`/`Content`, `SkiaLabel` and `SkiaImage` property sets, and the [layouts article](https://drawnui.net/articles/controls/layouts.html) description of `Absolute` as a single-cell grid.
- **WPF**: [`System.Windows.Media.Drawing`](https://learn.microsoft.com/en-us/dotnet/api/system.windows.media.drawing) class summary, derived types, and the "no Layout, Input, focus" remark; **MAUI**: [`Microsoft.Maui.Graphics`](https://learn.microsoft.com/en-us/dotnet/maui/user-interface/graphics/) with `GraphicsView`/`ICanvas`/`IDrawable`, alongside the separate `Controls.Shapes` family.
- **Layout naming survey** (RF39): [Flutter `GridView`](https://api.flutter.dev/flutter/widgets/GridView-class.html), [`Table`](https://api.flutter.dev/flutter/widgets/Table-class.html), [`Wrap`](https://api.flutter.dev/flutter/widgets/Wrap-class.html), [`Stack`](https://api.flutter.dev/flutter/widgets/Stack-class.html) and the [layout guide](https://docs.flutter.dev/ui/layout); [`flutter_layout_grid`](https://pub.dev/packages/flutter_layout_grid); [Compose layout basics](https://developer.android.com/develop/ui/compose/layouts/basics); [Qt Quick positioners](https://doc.qt.io/qt-6/qtquick-positioning-layouts.html) and [`QStackedLayout`](https://doc.qt.io/qt-6/qstackedlayout.html); [MDN `flex-direction`](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction), which defines `row`/`column` purely as main-axis directions; [Bootstrap grid](https://getbootstrap.com/docs/5.3/layout/grid/) and [stacks](https://getbootstrap.com/docs/5.3/helpers/stacks/); [Ant Design grid](https://ant.design/components/grid); [MUI](https://mui.com/material-ui/react-stack/), [Mantine](https://mantine.dev/core/stack/), [Atlassian](https://atlassian.design/components/primitives/stack/examples), [Braid](https://seek-oss.github.io/braid-design-system/components/Stack), [Primer](https://primer.style/components/stack), [Polaris](https://polaris-react.shopify.com/components/layout-and-structure/block-stack); [.NET MAUI layouts](https://learn.microsoft.com/en-us/dotnet/maui/user-interface/layouts/); [JavaFX `VBox`](https://openjfx.io/javadoc/21/javafx.graphics/javafx/scene/layout/VBox.html); [Every Layout](https://every-layout.dev/layouts/stack/).
- **SVG/CSS**: initial `fill` is black for all shapes including `polyline`; `clip-rule` defaults to `nonzero`; `object-fit` includes `scale-down`; `preserveAspectRatio` folds alignment and meet/slice into one value.

Three claims in the original draft did not survive that check and are corrected in place: the DrawnUI gradient representation (RF6), the SVG polyline fill default (RF11), the json-render "common catalog" citation (RF17) — plus one incomplete claim about A2UI's `Icon` (RF20). The remaining precedent claims checked out, including the A2UI `Row`/`Column` enums and defaults, `Divider`'s horizontal/vertical axis, `Card` as a single-child surface, DrawnUI's `SkiaShape.PathData` being SVG path data, and DrawnUI's layout type set.
