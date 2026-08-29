# NX UI MVP Object Model

**Proposal status:** Draft for discussion  
**Working title superseded:** “NX Drawn UI” — see “Names and terminology” in §1  
**Research current through:** August 20, 2026  
**Scope:** Level 1 graphics and Level 2 generic, read-only UI  
**Explicitly excluded from this MVP:** input controls, actions/events, application state/data binding, animation, and Level 3 semantic/data UI

## Executive recommendation

NX should define a small, declarative, renderer-neutral UI document that an LLM can generate and a trusted client can validate and render. The document should use:

- a flat, ID-addressed element map, inspired by [A2UI](https://a2ui.org/specification/v0.9.1-a2ui/) and [json-render](https://json-render.dev/docs/specs);
- namespaced, versioned component catalogs (`nx.ui` for layout-positioned UI, `nx.graphics` for coordinate-positioned graphics);
- familiar UI names (`HStack`, `VStack`, `ZStack`, `Grid`, `Scroll`, `Text`, `Image`, `Icon`);
- SVG vocabulary and data formats for portable drawing (`Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, SVG path data, fill/stroke, transforms);
- a clean evolution path from DrawnUI for .NET rather than a wire-level copy of its MAUI/Skia API.

The initial catalog proposed here contains:

- **Level 1 — graphics:** `Drawing`, `Group`, `Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, `Text`, `Image`.
- **Level 2 — output/layout:** `HStack`, `VStack`, `ZStack`, `Grid`, `Scroll`, `Box`, `Card`, `Divider`, `Text`, `Image`, `Icon`. (`nx.graphics.Drawing` is also embeddable as Level 2 content.)

The two namespaces intentionally allow both `nx.graphics.Text` and `nx.ui.Text`. The former is positioned glyph content in drawing coordinates; the latter is measured, wrapped UI text. That distinction avoids a deceptively complex “one Text does everything” object.

## 1. Goals and non-goals

### MVP goals

1. **Safe for model generation.** A model selects only cataloged components and literal, schema-validated properties; it does not emit executable code.
2. **Portable.** The same document can target DrawnUI/Skia, DOM/SVG/Canvas, and future native renderers.
3. **Minimal but useful.** It can express ordinary read-only UI, diagrams, illustrations, custom graphics, and hybrid layouts.
4. **Familiar.** Prefer established SVG, CSS, MAUI, and GenUI terms where their meanings transfer cleanly.
5. **Streamable and patchable later.** Stable IDs and a flat element map permit incremental additions and replacements without making streaming part of the MVP protocol.
6. **An evolution of DrawnUI.** Preserve its compositional strengths while removing renderer-specific implementation details from the portable model.

### MVP non-goals

- no buttons, checkboxes, text fields, selection controls, gestures, focus, or keyboard behavior;
- no actions, callbacks, commands, navigation, or tool calls;
- no state model, expressions, conditions, repetition, templates, or data binding;
- no animation or transitions;
- no charts, tables, maps, forms, or domain-specific components;
- no arbitrary HTML, CSS, JavaScript, XAML, shaders, or custom drawing callbacks;
- no guarantee of pixel-identical typography across renderers.

### Names and terminology

One naming rule governs the rest: **no word that names a renderer object appears in the portable model.** The document describes what to present; the renderer owns how. A term that belongs to Skia, MAUI, or the DOM is therefore reserved for the renderer layer, even when it is the more familiar word.

| Term | Layer | Meaning |
|---|---|---|
| **NX UI** | technology | The declarative UI document format described here. |
| **NX UI Document** | artifact | One serialized document: `NxUiDocument`, media type `application/nx-ui+json`, extension `.nxui.json`. |
| `nx.ui` | catalog | Layout-positioned UI: the layout engine measures and places children. |
| `nx.graphics` | catalog | Coordinate-positioned graphics: the document places geometry in a view box. |
| `Drawing` | component | The retained graphical artifact — root of a graphics subtree and its embedding point in UI. |
| `Group`, `Shape` | component | A transform/clip/opacity scope, and the filled-or-stroked geometry family within a `Drawing`. |
| `HStack`, `VStack`, `ZStack` | component | The three `nx.ui` layout containers, named for the axis they arrange along; see §7. |
| **Canvas** | renderer | A renderer's drawing surface: DrawnUI's `Canvas`, MAUI's `ICanvas`, HTML `<canvas>`. Never a component type in this model. |
| **draw**, **paint** | renderer | Operations a renderer performs. Used in prose (“paint order”), never as a type or catalog name. |
| **NX DrawnUI Renderer** | implementation | The renderer that targets DrawnUI/Skia. “DrawnUI” names that dependency and nothing else. |

Three choices in that table are worth their reasoning.

**`graphics`, not `draw` or `drawing`, for the catalog.** `draw` is a verb, and the catalog is a retained scene tree rather than a command stream — naming it for an action invites exactly the imperative reading §6 has to disclaim. `graphics` is also the settled word for this domain across the ecosystems NX borrows from: `Microsoft.Maui.Graphics`, `androidx.compose.ui.graphics`, `android.graphics`, `System.Drawing`'s successor naming, and SVG itself (Scalable Vector *Graphics*). `drawing` fixes the part of speech but carries the legacy GDI+ association.

**`Drawing`, not `Canvas`, for the root graphics component.** Every `Canvas` in the surrounding ecosystems is an immediate-mode painting surface — HTML's yields a `CanvasRenderingContext2D`, MAUI's an `ICanvas` inside `IDrawable.Draw`, Compose's a `DrawScope`, SwiftUI's a `GraphicsContext` — whereas this element is a retained, addressable, patchable tree. The precedent that matches is WPF's [`System.Windows.Media.Drawing`](https://learn.microsoft.com/en-us/dotnet/api/system.windows.media.drawing), an "abstract class that describes a 2-D drawing" whose subtypes (`DrawingGroup`, `GeometryDrawing`, `ImageDrawing`, `GlyphRunDrawing`) are documented as providing no layout, input, or focus — which is this catalog's contract exactly. Reusing `Canvas` would also collide inside NX's own adapter table: DrawnUI's `Canvas` is the host view for an entire drawn UI tree, mapped in §9 to the *document*, not to an embeddable graphics element.

**`nx.ui` stays.** Strictly, both catalogs produce UI, so `ui` is not a true sibling of `graphics` — the honest axis of the split is who decides position, not what is or is not an interface. `nx.ui` is kept anyway because it is the default catalog and the most predictable prefix a generating model can be given, and because the axis is stated wherever it matters (§7). Future catalogs are named for their content rather than nested beneath `nx.ui`; see §4.2.

## 2. Relevant precedents and what NX should take from them

| Source | Current architectural role | What NX should adopt | What NX should not copy into this MVP |
|---|---|---|---|
| Google A2UI — [v0.9.1](https://a2ui.org/specification/v0.9.1-a2ui/) current, [v1.0](https://a2ui.org/specification/v1.0-a2ui/) release candidate | Cross-platform, declarative, streaming agent-to-UI protocol with a basic component catalog | Catalog-constrained generation; stable component IDs; `Row`, `Column`, `Text`, `Image`, `Icon`, `Card`, `Divider`; host-owned rendering; v1.0's catalog-declared `allowedParents`/`allowedChildren` and its removal of protocol-level theming | Data model, functions, validation, actions, modal/input components, mixable per-component catalogs, and the complete wire protocol |
| [OpenUI / OpenUI Lang](https://www.openui.com/docs/openui-lang) (thesys) | Renderer-agnostic generative-UI framework built on a compact, streaming-first, non-JSON surface syntax over Zod-defined component libraries | Evidence that the *encoding* is a first-class design variable, not only the object model — its central claim is a large token reduction versus equivalent JSON | Its surface syntax as NX's canonical interchange form, and its React/Zod library mechanics |
| [Vercel json-render](https://json-render.dev/docs/specs) | Safe, schema/catalog-driven JSON UI renderer | Its especially clear `root` + `elements` + `{type, props, children}` normalized representation | State expressions, visibility expressions, repeats, actions, watchers, and renderer framework details |
| [Vercel AI SDK generative UI](https://ai-sdk.dev/docs/ai-sdk-ui/generative-user-interfaces) | Widely used tool-result-to-component pattern and streaming UI runtime | Typed tool/component boundaries are a future integration target | Treating React components as NX's portable object model |
| [CopilotKit GenUI](https://docs.copilotkit.ai/concepts/generative-ui-overview), [A2UI support](https://docs.copilotkit.ai/a2a/generative-ui/a2ui), and [AG-UI](https://docs.ag-ui.com/) | Agent/front-end integration plus a bidirectional, event-based agent-to-application protocol | Evidence that NX should remain a declarative catalog that can ride over an agent protocol | AG-UI runtime/event concepts in the read-only MVP schema |
| [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview) | Sandboxed HTML applications delivered by MCP servers | A future packaging and distribution target for an NX renderer | Using arbitrary web application code as the NX object model |
| [Thesys C1](https://docs.thesys.dev/guides/what-is-thesys-c1) | Model/API and React SDK specialized for streaming generated UI | Evidence that generation quality, theming, and custom catalogs matter in addition to schema design | Its response DSL as a portability dependency; its actions and forms in this MVP |
| [DrawnUI controls](https://drawnui.net/articles/controls/index.html), [layouts](https://drawnui.net/articles/controls/layouts.html), and [shapes](https://drawnui.net/articles/controls/shapes.html) | High-performance, Skia-rendered .NET UI toolkit | Direct drawing, composable controls, row/column/grid/stack concepts, shape content, SVG path data, gradients, shadows | MAUI bindable-property mechanics, cache/GPU controls, commands, gestures, and platform-specific tuning |

This is not a market-share ranking. It is a relevance ranking for NX's object-model design. A2UI and json-render are the closest architectural precedents; the other systems demonstrate integration, distribution, and generation patterns that NX should interoperate with rather than duplicate.

## 3. Architectural shape

```text
agent or application
        |
        | validated NX JSON
        v
  NxUiDocument
  root + element map
        |
        +--------------------+
        |                    |
      nx.ui              nx.graphics
  layout-positioned   coordinate-positioned
   output + content     retained graphics
        |                    |
        +---------+----------+
                  v
       trusted renderer / theme
      DrawnUI, web, or native host
```

NX describes intent and structure. It does not prescribe whether `nx.graphics.Rect` becomes a Skia draw call, an SVG `<rect>`, a Canvas 2D operation, or a native view.

### Why a flat element map

A normalized map gives every component a stable identity and makes partial generation, validation, replacement, and future JSON Patch streaming straightforward. Both A2UI and json-render use ID-addressed/flat representations. Recursive authoring syntax may be offered as sugar, but the canonical interchange form should be flat.

### Why explicit component types instead of DrawnUI-style `Type` enums

DrawnUI uses one `SkiaLayout` with `Type="Row|Column|Grid|Absolute|Wrap"` and one `SkiaShape` with `Type="Rectangle|Circle|Ellipse|Path|..."`. NX should use `nx.ui.HStack`, `nx.ui.Grid`, `nx.graphics.Rect`, and so on because:

- each component receives a smaller, clearer property schema;
- invalid combinations become harder to generate;
- catalog capabilities are easier to negotiate;
- names align with SVG and current GenUI catalogs;
- renderers may still implement these as one shared internal class.

## 4. Canonical document model

Type notation in this document is [NX](../nx-grammar.md) source. Records use `type Name = { ... }`, closed scalar choices use `enum`, discriminated unions use `type Name =` with leading-pipe cases, and renderer-provided catalog components use `external component`, whose signature declares props and whose body is supplied by the host rather than by NX.

Three NX conventions carry meaning here. A `?` suffix makes a property nullable, which in this model means the property may be omitted — the **Default** column of each table says what the renderer does when it is. A `= value` clause gives a real default. And exactly one property per signature may be marked `content`, which is the property that receives element body content, so it is the property that the wire format spells as `children`.

Unless stated otherwise, numeric values must be finite JSON numbers and default units are logical/device-independent pixels. Appendix A lists both catalogs as complete NX source; Appendix B records where NX's current syntax cannot express this model, and what the listing does instead.

### 4.1 `NxUiDocument`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `format` | `string` | yes | Artifact discriminator. MVP value: `"nx-ui-json"`, following the `format` + `schemaVersion` convention already used by [NX IR JSON](nx-ir-format.md). |
| `schemaVersion` | `string` | yes | NX document schema version, in semantic-version form. MVP value: `"0.1.0"`. |
| `catalogs` | `CatalogUse[]` | yes | Catalogs and exact versions needed to interpret component types. |
| `root` | `ElementId` | yes | ID of the single root element. |
| `elements` | map of `ElementId` to `Element` | yes | Flat element map. Every referenced ID must exist. Unreachable elements are invalid in MVP. NX has no map type; see Appendix B. |
| `metadata` | `DocumentMetadata?` | no | Non-rendered document information. |

```nx
type ElementId = string   // ^[A-Za-z_][A-Za-z0-9_.-]{0,63}$

type NxUiDocument = {
  format: string = "nx-ui-json"
  schemaVersion: string = "0.1.0"
  catalogs: CatalogUse[]
  root: ElementId
  elements: Element[]     // serialized as an object keyed by ElementId; see Appendix B
  metadata: DocumentMetadata?
}
```

Precedent: json-render's [`Spec`](https://github.com/vercel-labs/json-render/blob/main/packages/core/src/types.ts) uses `root` and `elements`; A2UI's [`updateComponents`](https://a2ui.org/specification/v0.9.1-a2ui/) uses stable component IDs within a surface, and A2UI v1.0 makes the implicit surface root explicit as a reserved `Surface` component whose `child` is the root ID.

Serialized documents use the media type `application/nx-ui+json` and the file extension `.nxui.json`. A2UI registers `application/a2ui+json` for the same reason: a discriminating media type keeps transports, caches, and tools from having to sniff the payload.

### 4.2 `CatalogUse`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `id` | `string` | yes | Reverse-DNS or registered catalog ID. Core IDs: `nx.ui`, `nx.graphics`. |
| `version` | `string` | yes | Exact semantic version understood by the producer. MVP core version: `0.1.0`. |

```nx
type CatalogUse = {
  id: string
  version: string
}
```

Catalogs are peers named for their content, never nested beneath one another. The MVP defines two and reserves the IDs the roadmap in §11 will need:

| Catalog ID | Status | Contents |
|---|---|---|
| `nx.ui` | MVP | Layout-positioned output: containers, text, image, icon, divider, card. |
| `nx.graphics` | MVP | Coordinate-positioned graphics: `Drawing`, groups, shapes, path, positioned text and image. |
| `nx.input` | reserved | Controls, focus, gestures, actions — the post-MVP interaction catalog. |
| `nx.data` | reserved | Level 3 semantic and data presentation: metrics, charts, tables, lists. |
| `nx.<domain>` | reserved | Domain catalogs such as `nx.commerce`, plus third-party reverse-DNS IDs. |

Reserving these now keeps `nx.ui` from having to become an umbrella later (`nx.ui.input`, `nx.ui.data`), which would change every existing component type name.

Exact versions make generated documents reproducible. A later negotiation protocol may select mutually supported versions before generation.

### 4.3 `DocumentMetadata`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `title` | `string?` | no | Human-readable document title. |
| `description` | `string?` | no | Human-readable summary. |
| `generator` | `string?` | no | Producer identifier for diagnostics, not behavior. |

```nx
type DocumentMetadata = {
  title: string?
  description: string?
  generator: string?
}
```

### 4.4 `Element`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `type` | `ComponentType` | yes | Namespaced catalog type, such as `nx.ui.VStack` or `nx.graphics.Path`. |
| `props` | component-specific object | yes | Literal properties validated against that component's catalog schema. Use `{}` when empty. |
| `children` | `ElementId[]` | conditional | Ordered child IDs. Allowed only by components documented as containers. |

There is no `Element` record to declare in NX source, because NX has no envelope: an element *is* an `external component` invocation, and the `type`/`props`/`children` envelope is what a compiler produces from it. `<ui.VStack gap={12.0}> ... </ui.VStack>` names the component, binds props by name, and binds body content to the component's `content` property. That is inherently the nested authoring form this section calls sugar, and NX cannot write the flat map at all — see Appendix B — so NX is the authoring surface over this model while the flat map remains its interchange and validation form.

`ElementId` matches `^[A-Za-z_][A-Za-z0-9_.-]{0,63}$` and is unique within a document. Constraining the charset keeps IDs usable as RFC 6901 JSON Pointer tokens for post-MVP patch streaming, keeps them safe as debug/accessibility identifiers, and rules out empty or whitespace-only IDs. `ComponentType` is a non-empty `<catalog>.<Component>` name whose catalog ID appears in `catalogs`. MVP elements do **not** have `state`, `visible` expressions, `repeat`, `slots`, `on`, or `watch`; those are useful json-render precedents for post-MVP work.

### 4.5 Tree and validation rules

- The graph formed by `children` must be a rooted tree: no cycles and no child with multiple parents.
- Child order is paint order for `nx.graphics.Drawing`, `nx.graphics.Group`, and `nx.ui.ZStack`; later children paint above earlier children.
- `nx.graphics.*` primitives may appear only below `nx.graphics.Drawing` or `nx.graphics.Group`.
- `nx.graphics.Drawing` may appear anywhere an `nx.ui` child is accepted.
- `nx.ui.*` components may **not** appear below a `Drawing` or `Group`. The MVP has no SVG `foreignObject` equivalent; measured UI content is composed above a `Drawing`, not inside one.
- A `Drawing` may **not** be nested inside another `Drawing` or `Group` in the MVP. Nested view boxes are deferred.
- These containment rules are normative prose here, but the catalog schema is the intended home for them. A2UI v1.0 added `allowedParents`/`allowedChildren` to catalog component definitions for exactly this reason: a flat adjacency list of ID references cannot express child-type restrictions in JSON Schema alone.
- Unknown component types or properties fail validation in strict mode.
- Unknown enum values fail validation. A newer optional property can be ignored only after a schema-version policy explicitly permits it.
- All text, URIs, data sizes, element counts, and path complexity are subject to host limits.

## 5. Shared MVP value types

### 5.1 Geometry and sizing types

```nx
enum Alignment = start | center | end | stretch
enum AlignSelf = auto | start | center | end | stretch
enum Anchor = start | center | end
enum Axis = horizontal | vertical
enum Distribution = start | center | end | spaceBetween | spaceAround | spaceEvenly

type Length =
  | auto
  | px { value: float64 }
  | percent { value: float64 }

type Point = {
  x: float64
  y: float64
}

type ViewBox = {
  x: float64
  y: float64
  width: float64
  height: float64
}

type Insets = {
  top: float64 = 0.0
  right: float64 = 0.0
  bottom: float64 = 0.0
  left: float64 = 0.0
}

type CornerRadii = {
  topLeft: float64 = 0.0
  topRight: float64 = 0.0
  bottomRight: float64 = 0.0
  bottomLeft: float64 = 0.0
}
```

Three of these declarations lose a shorthand that the JSON form keeps, all for the same reason: NX unions are named cases, so they cannot mix a scalar with a record or a string literal. `Length` becomes three cases instead of `120`/`"auto"`/`"50%"`; `Insets` and `CornerRadii` lose their uniform-scalar form, so `padding: 20` has to be written with all four sides; and `Alignment | "auto"` becomes a separate `AlignSelf` enum. Appendix B gives the details and the wire-format consequences.

`start`/`end` are chosen over `left`/`right` so layout can adapt to writing direction. The vocabulary follows [CSS Box Alignment](https://www.w3.org/TR/css-align-3/) and A2UI's `align`/`justify` conventions. DrawnUI adapters map these to MAUI layout options.

`Length` deliberately omits CSS `calc()`, viewport units, and arbitrary strings. `auto` means intrinsic sizing; percentages resolve against the containing content box. A percentage against a parent that is itself intrinsically sized resolves as `auto`, so layout never depends on a circular constraint.

Omitted `Insets` sides default to `0`, so `{ "top": 8 }` is valid and cheap to generate. `Insets` uses physical `left`/`right` while alignment uses logical `start`/`end`; that split is intentional for the MVP, which has no bidirectional text policy, and §11 tracks logical insets with the rest of the localization work.

### 5.2 Color and paint

```nx
type Color = string   // CSS Color syntax subset accepted by the host

enum GradientUnits = objectBoundingBox | userSpaceOnUse

type GradientStop = {
  offset: float64          // 0..1
  color: Color
  opacity: float64 = 1.0   // 0..1
}

type Paint =
  | none
  | solid { color: Color }
  | linearGradient {
      x1: float64
      y1: float64
      x2: float64
      y2: float64
      units: GradientUnits = {GradientUnits.objectBoundingBox}
      stops: GradientStop[]
    }
  | radialGradient {
      cx: float64
      cy: float64
      r: float64
      fx: float64?
      fy: float64?
      units: GradientUnits = {GradientUnits.objectBoundingBox}
      stops: GradientStop[]
    }
```

`Paint` is tidier as an NX union than it was as a TypeScript one: `"none"`, which the TypeScript form had to bolt on at each use site as `Paint | "none"`, is simply a case, and the discriminator stops being a hand-written `type` field. The cost is at the other end — a plain color is no longer a bare string but a `solid` case with a `$type` tag. See Appendix B.

Use the portable subset of [CSS Color 4](https://www.w3.org/TR/css-color-4/) for `Color`: named colors, `transparent`, hex, `rgb()`/`rgba()`, and `hsl()`/`hsla()`. Renderers must normalize colors; unsupported CSS-wide keywords and environment-dependent values are invalid.

`stops` must contain at least two entries with `offset` values in non-decreasing order; renderers clamp offsets to 0..1 and must not reorder them.

Gradient terms and coordinate modes intentionally follow [SVG gradients](https://www.w3.org/TR/SVG2/pservers.html). DrawnUI's [`SkiaGradient`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/SkiaGradient.cs) already carries the same information in parallel arrays — `Colors` plus `ColorPositions` — so an adapter zips those into ordered `stops` without loss. Its `StartXRatio`/`StartYRatio`/`EndXRatio`/`EndYRatio` are ratios of the painted bounds, which is exactly NX's default `objectBoundingBox` mode; that is independent evidence for the default chosen in §12.4. DrawnUI's `TileMode` (gradient spread) and `GradientType.Sweep`/`Conical` have no MVP equivalent and are deferred in §11.

### 5.3 Stroke, shadow, transform, border

```nx
enum LineCap = butt | round | square
enum LineJoin = miter | round | bevel

type Stroke = {
  paint: Paint
  width: float64 = 1.0
  lineCap: LineCap = {LineCap.butt}
  lineJoin: LineJoin = {LineJoin.miter}
  miterLimit: float64 = 4.0
  dashArray: float64[]?
  dashOffset: float64 = 0.0
}

type Shadow = {
  color: Color
  offsetX: float64 = 0.0
  offsetY: float64 = 0.0
  blur: float64 = 0.0      // >= 0
}

type Transform =
  | translate { x: float64 y: float64 }
  | scale { x: float64 y: float64? }        // null y means uniform scale
  | rotate { degrees: float64 cx: float64 = 0.0 cy: float64 = 0.0 }
  | skewX { degrees: float64 }
  | skewY { degrees: float64 }
  | matrix { a: float64 b: float64 c: float64 d: float64 e: float64 f: float64 }

type Border = {
  paint: Paint
  width: float64 = 1.0
}
```

`Transform` is the case NX models best: a closed set of variants with per-variant payloads is exactly what a discriminated union is for, and the hand-written `type` field disappears. The one thing it cannot carry is `scale.y` defaulting to `scale.x`, since an NX default is a literal or a constant expression and cannot reference a sibling property; `y` is therefore nullable, with null meaning uniform scale.

Stroke terminology follows [SVG painting](https://www.w3.org/TR/SVG2/painting.html). Transform names and the six-value affine matrix follow [SVG/CSS Transforms](https://www.w3.org/TR/css-transforms-1/). The array is equivalent to an SVG `transform` list read left to right: matrices are concatenated in array order, so the *last* entry is the one applied first to the node's own geometry. `rotate` uses positive degrees clockwise in NX's y-down coordinate space, matching SVG. A stroke is scaled by the transforms in effect, as in SVG; there is no MVP equivalent of `vector-effect: non-scaling-stroke`. DrawnUI's `StrokeColor`, `StrokeWidth`, `StrokeCap`, dash path, gradients, and multiple shadows map directly to these portable objects.

### 5.4 Image source and accessibility

```nx
type ImageSource =
  | uri { uri: string }
  | resource { name: string }

type Accessibility = {
  label: string?
  description: string?
  hidden: boolean = false   // true means decorative/ignored by accessibility APIs
}
```

The host—not the generated document—controls permitted URI schemes, domains, authentication, fetch limits, caching, and media decoding. Inline base64 data is intentionally omitted to reduce token volume and denial-of-service risk. `resource` names refer to a trusted host registry.

Accessibility names follow platform accessibility APIs and the [Accessible Name and Description Computation](https://www.w3.org/TR/accname-1.2/); the field names match A2UI's `AccessibilityAttributes` (`label`, `description`, `hidden`), minus its `live` region control, which has no meaning in a static document. Renderers derive semantic roles from component types; the MVP does not allow arbitrary ARIA roles.

## 6. Level 1 — graphics catalog (`nx.graphics@0.1.0`)

The graphics catalog is an SVG-shaped, retained-mode scene tree, and it is the half of the model where **the document decides position**: every node states its own coordinates, in logical units within the nearest `Drawing` view box. It is not an imperative Canvas API, although it maps readily to [Canvas 2D](https://html.spec.whatwg.org/multipage/canvas.html#the-canvas-element) or Skia.

### 6.1 Shared graphics properties

Every node inside a `Drawing` accepts `GraphicsCommonProps`, declared in NX as an abstract external component that concrete components extend:

```nx
abstract external component <GraphicsCommon
  opacity: float64 = 1.0
  transform: Transform[]?
  clipPath: string?
  accessibility: Accessibility?
/>
```

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `opacity` | `float64` (0..1) | `1` | Group opacity: the node and its descendants are composited as one layer, then that layer is blended at this alpha. |
| `transform` | `Transform[]?` | `[]` | Ordered local transforms. |
| `clipPath` | `string?` | none | SVG path-data geometry used as a local clip, in the node's own coordinate system. |
| `accessibility` | `Accessibility?` | derived | Optional accessible name/description or decorative status. |

`opacity` is deliberately specified as group opacity rather than a per-descendant multiplier: the two differ visibly wherever descendants overlap, and SVG, Skia, and Canvas 2D all provide the group form.

`clipPath` geometry is interpreted in the node's local coordinate system — the space established *after* the node's own `transform` is applied — and clips the node and all of its descendants. It uses the `nonzero` rule regardless of `fillRule`. It is purposefully a small MVP subset of SVG clipping; referenced clip trees, masks, and filter regions are deferred.

Every filled/stroked geometry accepts `ShapeProps` in addition to its own properties. NX allows only one base per component, so `ShapeProps` is expressed as a link in a chain rather than as a second mixin — which works here because everything that takes shape properties also takes graphics properties:

```nx
enum FillRule = nonzero | evenodd

abstract external component <ShapeCommon extends GraphicsCommon
  fill: Paint?
  stroke: Stroke?
  fillRule: FillRule = {FillRule.nonzero}
  shadows: Shadow[]?
/>
```

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `fill` | `Paint?` | component-specific | Interior paint. `Paint.none` is the explicit no-fill case. |
| `stroke` | `Stroke?` | none | Outline paint and stroke geometry. |
| `fillRule` | `FillRule` | `nonzero` | Interior rule; matches [SVG fill-rule](https://www.w3.org/TR/SVG2/painting.html#FillRuleProperty). |
| `shadows` | `Shadow[]?` | `[]` | Ordered drop shadows. |

`fill` is nullable rather than defaulted because the default is component-specific and NX has no way to override an inherited default in a derived component — declaring `fill` again on `Line` would be rejected as a duplicate property. The renderer resolves null per component type.

The default fill is `Paint.none` for `Line` and `Polyline`, and black for closed shapes and `Path`. Closed-shape behavior matches SVG's initial `fill: black`. `Polyline` is a deliberate divergence: SVG also fills a polyline by default, which surprises nearly everyone and produces a wrong-looking region whenever a model draws an open multi-segment stroke. §12 tracks whether `Path` should diverge the same way.

### 6.2 `nx.graphics.Drawing`

Root drawing surface and bridge into Level 2 UI. Accepts ordered drawing children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Controls the surface's size and placement when embedded in UI. |
| `viewBox` | `ViewBox` | yes | Internal coordinate rectangle; width and height must be positive. |
| `fit` | `ViewBoxFit` | no | `contain`; fit, crop, or non-uniformly stretch the view box into the surface. |
| `contentAlignment` | `ContentAlignment?` | no | Center/center; only meaningful when `fit` preserves the aspect ratio. |

```nx
enum ViewBoxFit = contain | cover | fill

type ContentAlignment = {
  x: Anchor = {Anchor.center}
  y: Anchor = {Anchor.center}
}
```

When `width` and `height` are `auto`, the surface's intrinsic size is the view box's `width` and `height` in logical pixels, and its intrinsic aspect ratio is `width / height`; a single specified dimension resolves the other through that ratio. Content is always clipped to the surface box. The common `background` paints the surface before its drawing children, and common `accessibility` describes the surface.

Rationale: `viewBox` semantics come from [SVG coordinate systems](https://www.w3.org/TR/SVG2/coords.html). `fit` is the one place where NX deliberately spells an SVG concept in CSS terms: `contain`/`cover`/`fill` map one-to-one onto `preserveAspectRatio` `meet`/`slice`/`none`, and reusing the `object-fit` vocabulary that `nx.ui.Image` and `nx.graphics.Image` already use means a generating model learns one fit vocabulary instead of two. SVG also folds alignment into the same string (`xMidYMid meet`); NX keeps `contentAlignment` separate so neither value has to be parsed. Unlike DrawnUI's canvas control, the portable object does not expose acceleration, cache, pixel density, or invalidation settings.

### 6.3 `nx.graphics.Group`

Groups ordered drawing children without drawing its own geometry.

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps` | see §6.1 | no | Shared opacity, transform, clip, and accessibility. |

Rationale: equivalent to SVG [`g`](https://www.w3.org/TR/SVG2/struct.html#Groups) and a Skia save/restore scope. Grouping avoids repeating transforms or opacity.

### 6.4 `nx.graphics.Rect`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared drawing and paint properties. |
| `x` | `float64` | no | `0`; left coordinate. |
| `y` | `float64` | no | `0`; top coordinate. |
| `width` | `float64` | yes | Non-negative width. |
| `height` | `float64` | yes | Non-negative height. |
| `rx` | `float64` | no | `0`; horizontal corner radius. |
| `ry` | `float64?` | no | `rx`; vertical corner radius. NX cannot spell this default; see Appendix B. |

Rationale: names and radius behavior follow SVG [`rect`](https://www.w3.org/TR/SVG2/shapes.html#RectElement). DrawnUI `Rectangle` + `CornerRadius` maps here; adapters may approximate unequal radii if necessary.

### 6.5 `nx.graphics.Circle`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `cx` | `float64` | yes | Center x. |
| `cy` | `float64` | yes | Center y. |
| `r` | `float64` | yes | Non-negative radius. |

Rationale: matches SVG [`circle`](https://www.w3.org/TR/SVG2/shapes.html#CircleElement) and DrawnUI's explicit `Circle` shape. Keeping it separate from `Ellipse` is more legible for models and people.

### 6.6 `nx.graphics.Ellipse`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `cx` | `float64` | yes | Center x. |
| `cy` | `float64` | yes | Center y. |
| `rx` | `float64` | yes | Non-negative horizontal radius. |
| `ry` | `float64` | yes | Non-negative vertical radius. |

Rationale: matches SVG [`ellipse`](https://www.w3.org/TR/SVG2/shapes.html#EllipseElement) and DrawnUI `Ellipse`.

### 6.7 `nx.graphics.Line`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties; `fill` has no effect. |
| `x1` | `float64` | yes | Start x. |
| `y1` | `float64` | yes | Start y. |
| `x2` | `float64` | yes | End x. |
| `y2` | `float64` | yes | End y. |

Rationale: follows SVG [`line`](https://www.w3.org/TR/SVG2/shapes.html#LineElement). DrawnUI models lines as point collections; NX adds the common two-point primitive and retains `Polyline` for multi-segment lines.

### 6.8 `nx.graphics.Polyline`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Two or more ordered vertices; path remains open. |

### 6.9 `nx.graphics.Polygon`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Three or more ordered vertices; closing segment is implicit. |

Rationale for both: follows SVG [`polyline` and `polygon`](https://www.w3.org/TR/SVG2/shapes.html#PolylineElement) and maps directly from DrawnUI `Points`. DrawnUI's non-standard `SmoothPoints` is deferred; smooth geometry should use an explicit `Path` in the MVP.

### 6.10 `nx.graphics.Path`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `data` | `string` | yes | SVG path-data string. |

`data` uses the standard SVG command grammar (`M/L/H/V/C/S/Q/T/A/Z`, absolute or relative) from [SVG Paths](https://www.w3.org/TR/SVG2/paths.html). This is already the format of DrawnUI `SkiaShape.PathData`, so it is the strongest direct compatibility point. Renderers must impose command-count, numeric-range, and rendered-bounds limits.

### 6.11 `nx.graphics.Text`

Single-line or explicitly line-broken text positioned in drawing coordinates. It does not perform UI box layout.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsCommonProps` | see §6.1 | no | Shared properties. |
| `x` | `float64` | yes | Text anchor x. |
| `y` | `float64` | yes | Baseline y. |
| `text` | `string` | yes | Literal text; `\n` requests explicit line breaks. |
| `fontFamily` | `string?` | no | Host default. |
| `fontSize` | `float64` | no | `16`. |
| `fontWeight` | `float64` (1..1000) | no | `400`. |
| `fontStyle` | `FontStyle` | no | `normal`. |
| `textAnchor` | `TextAnchor` | no | `start`. |
| `dominantBaseline` | `DominantBaseline` | no | `auto`. |
| `letterSpacing` | `float64` | no | `0`. |
| `fill` | `Paint?` | no | Black. |
| `stroke` | `Stroke?` | no | None. |
| `shadows` | `Shadow[]?` | no | `[]`. |

Rationale: positioning and anchor terms follow [SVG text](https://www.w3.org/TR/SVG2/text.html). Numeric weight follows modern CSS/OpenType and is more portable than `Bold` flags. DrawnUI `SkiaLabel` supplies the implementation precedent for family, size, weight, spacing, fill/stroke, and shadows. Rich spans, text-on-path, shaping controls, and automatic fit are deferred.

### 6.12 `nx.graphics.Image`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsCommonProps` | see §6.1 | no | Shared properties. |
| `x` | `float64` | no | `0`. |
| `y` | `float64` | no | `0`. |
| `width` | `float64` | yes | Non-negative destination width. |
| `height` | `float64` | yes | Non-negative destination height. |
| `source` | `ImageSource` | yes | Trusted resource or host-approved URI. |
| `alt` | `string` | yes | Accessible alternative; use `""` for a purely decorative image. |
| `fit` | `ObjectFit` | no | `contain`. |

Rationale: destination rectangle follows SVG [`image`](https://www.w3.org/TR/SVG2/embedded.html#ImageElement); fit names and values follow CSS `object-fit`, including `scaleDown` (contain, but never upscale past intrinsic size), which A2UI also carries. `alt` is required on both image components so that no image can be generated without an accessibility decision. DrawnUI `SkiaImage.Source` and `Aspect` map naturally. Filters, tint, sprite sheets, loading strategy, resampling quality, and cache policy stay renderer-specific or post-MVP.

## 7. Level 2 — generic output/layout catalog (`nx.ui@0.1.0`)

Level 2 is deliberately read-only. It provides measured layout and ordinary content; custom visual composition drops into `nx.graphics.Drawing`.

This catalog is the half of the model where **the layout engine decides position**: a component states constraints — size preferences, spacing, alignment, growth — and its parent resolves them against available space. That is the real boundary between the two catalogs, and it is worth stating plainly because the names do not carry it: graphics are UI too, and a `Drawing` is as much a part of the interface as a `Card`. What separates them is not what they depict but who computes where things go.

The duplicate pairs follow from that axis rather than working around it. `nx.ui.Text` and `nx.graphics.Text` are the same content under two positioning models — one measured and wrapped into a box the parent sized, one anchored at a baseline the document chose — and the same is true of `nx.ui.Image` versus `nx.graphics.Image`, and of `Box` versus `Rect`. A single merged component would have to carry both models and silently ignore half its properties depending on its parent.

#### Why `HStack` / `VStack` / `ZStack`

The three layout containers are named for the axis they arrange along, following SwiftUI's `HStack`/`VStack`/`ZStack` and Tamagui's `XStack`/`YStack`/`ZStack`. Three properties of that scheme matter here.

*It is one family.* The alternative under consideration was `Row`/`Column` for linear flow plus a third name for overlay, but every candidate third name (`Stack`, `Overlay`, `Layer`) reads as a different kind of word than `Row` and `Column`, leaving the catalog with two naming conventions for one concept. An axis prefix names all three the same way.

*It keeps “row” and “column” meaning grid structure.* §7.4 `Grid` gives children `gridRow`, `gridColumn`, `rowSpan`, and `columnSpan`, and takes `rows`, `columns`, `rowGap`, and `columnGap`. Had the linear containers also been called `Row` and `Column`, a document could read `{"type": "Row", "gridRow": 2}`, and a generating model carrying [Bootstrap](https://getbootstrap.com/docs/5.3/layout/grid/) or [Ant Design](https://ant.design/components/grid) priors — where `row`/`col` are *mandatory* grid parts, not standalone containers — could reasonably emit `Column` children inside a `Grid` to make cells. Flutter avoids the same collision by never letting the words meet: its `GridView` takes `crossAxisCount`/`mainAxisSpacing` rather than row and column counts, and its `Wrap` coins “run” for a wrapped line, reserving `row`/`column` for linear containers and for `Table`. NX cannot use that escape hatch, because its `Grid` is a [CSS Grid](https://www.w3.org/TR/css-grid-2/) derivative and CSS Grid vocabulary is the most valuable prior a generating model has for two-dimensional placement. Renaming the containers is the cheaper side of the trade.

*It resolves an inversion against the render target.* DrawnUI's own `SkiaStack` is `LayoutType.Column`, and OpenUI's default library, MUI, Fluent UI, Mantine, Atlassian, Braid, IBM Carbon, Shopify Polaris (`BlockStack`), WPF's `StackPanel`, .NET MAUI's `StackLayout`, and `UIStackView` all use an unprefixed “stack” to mean *linear*, almost always vertical by default. An unprefixed `Stack` meaning overlay would therefore have been the one genuinely wrong choice, and `nx.ui.Stack → SkiaLayer` beside `nx.ui.Column → SkiaStack` would have inverted the word across §9's own mapping table. `VStack → SkiaStack` and `ZStack → SkiaLayer` read correctly.

The cost is accepted rather than denied: `Row`/`Column` is what [A2UI's](https://a2ui.org/specification/v1.0-a2ui/) basic catalog uses, along with Flutter, Jetpack Compose, and Qt Quick, so NX gives up name-level alignment with the GenUI peer it otherwise tracks most closely. Catalogs are per-application in A2UI and interoperability needs a mapping rather than matching identifiers, so the loss is in generator familiarity only — and `HStack`/`VStack` carry a large prior of their own from SwiftUI. §12.3 records this as a decision to test rather than to assume.

### 7.1 `UiCommonProps`

Every Level 2 component, plus `nx.graphics.Drawing`, accepts these optional flattened properties. In NX this is one abstract external component that every Level 2 component extends:

```nx
abstract external component <UiCommon
  width: Length = {Length.auto}
  height: Length = {Length.auto}
  minWidth: float64 = 0.0
  minHeight: float64 = 0.0
  maxWidth: float64?
  maxHeight: float64?
  margin: Insets?
  padding: Insets?
  alignSelf: AlignSelf = {AlignSelf.auto}
  justifySelf: AlignSelf = {AlignSelf.auto}
  grow: float64 = 0.0
  shrink: float64 = 1.0
  gridColumn: int?
  gridRow: int?
  columnSpan: int = 1
  rowSpan: int = 1
  background: Paint = {Paint.none}
  border: Border?
  cornerRadius: CornerRadii?
  shadows: Shadow[]?
  clip: boolean = false
  opacity: float64 = 1.0
  accessibility: Accessibility?
/>
```

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `width`, `height` | `Length` | `auto` | Preferred dimensions. |
| `minWidth`, `minHeight` | `float64` | `0` | Minimum dimensions. |
| `maxWidth`, `maxHeight` | `float64?` | unbounded | Maximum dimensions. |
| `margin` | `Insets?` | `0` | Space outside the component. |
| `padding` | `Insets?` | `0` | Space between border and content. |
| `alignSelf` | `AlignSelf` | `auto` | Override the parent-assigned alignment on the block (vertical) axis. |
| `justifySelf` | `AlignSelf` | `auto` | Override the parent-assigned alignment on the inline (horizontal) axis. |
| `grow` | `float64` | `0` | Share of positive free space in an `HStack` or `VStack`. |
| `shrink` | `float64` | `1` | Relative shrink factor in an `HStack` or `VStack`. |
| `gridColumn`, `gridRow` | `int?` (>= 0) | auto-placement | Zero-based grid position. Set both or neither. |
| `columnSpan`, `rowSpan` | `int` (>= 1) | `1` | Grid cell span. |
| `background` | `Paint` | `none` | Background inside the border. |
| `border` | `Border?` | none | Uniform border. |
| `cornerRadius` | `CornerRadii?` | `0` | Box corner radii. |
| `shadows` | `Shadow[]?` | `[]` | Ordered box shadows. |
| `clip` | `boolean` | `false` | Clip descendants to the padding box/corner radii. |
| `opacity` | `float64` (0..1) | `1` | Component subtree opacity. |
| `accessibility` | `Accessibility?` | derived | Accessible name/description/decorative status. |

`alignSelf` and `justifySelf` are bound to fixed physical axes rather than to "main" and "cross," because a child does not know what kind of container holds it and the same document is generated once for every renderer. The resulting mapping is:

| Parent | `justifySelf` (horizontal) | `alignSelf` (vertical) |
|---|---|---|
| `HStack` | ignored; use `justify` on the parent plus `grow`/`shrink` | overrides the parent's `align` |
| `VStack` | overrides the parent's `align` | ignored; use `justify` on the parent plus `grow`/`shrink` |
| `Grid`, `ZStack` | overrides `justifyItems` | overrides `alignItems` |
| `Scroll`, `Box`, `Card` | positions the single child in the content box | positions the single child in the content box |

A property that a parent ignores is valid but has no effect; it is a diagnostic, not a validation error, so that a subtree can be re-parented without becoming invalid.

These are constraints and portable appearance, not a CSS passthrough. Names balance CSS flex/grid familiarity with DrawnUI/MAUI's width request, margin/padding, alignment, background, clipping, and grid row/column concepts. Layout engines must document deterministic treatment of over-constrained values; where `width` conflicts with `minWidth`/`maxWidth`, the minimum wins, matching CSS.

### 7.2 `nx.ui.HStack`

Horizontal flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `float64` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `start` | Distribution along the horizontal main axis. |
| `align` | `Alignment` | `stretch` | Alignment on the vertical cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional lines. |

### 7.3 `nx.ui.VStack`

Vertical flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `float64` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `start` | Distribution along the vertical main axis. |
| `align` | `Alignment` | `stretch` | Alignment on the horizontal cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional lines. |

`justify` distributes only the free space that remains after `grow` has been applied, so a child with `grow > 0` and a `spaceBetween` parent is not double-counted. When `wrap` is `true`, `gap` separates both adjacent children and adjacent lines, and lines are packed toward the cross-axis start; per-line distribution (CSS `align-content`) is deferred.

Rationale: A2UI's basic catalog defines the same two containers (as `Row` and `Column`) with exactly these `justify` (`start`/`center`/`end`/`spaceBetween`/`spaceAround`/`spaceEvenly`) and `align` (`start`/`center`/`end`/`stretch`) enums, defaulting to `start` and `stretch` as NX does, and expresses proportional sizing as a `weight` on the child; CSS Flexbox supplies the behavioral model. NX adds `gap`, which A2UI leaves to the host theme but DrawnUI exposes as `Spacing`. `grow`/`shrink` replace both A2UI's single `weight` and DrawnUI's sizing idiom, because a shrink factor is needed once `wrap` is false and content overflows.

### 7.4 `nx.ui.Grid`

Two-dimensional container; accepts zero or more UI children.

```nx
type TrackSize =
  | auto
  | fixed { value: float64 }
  | fraction { value: float64 }   // CSS `fr` units
```

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `columns` | `TrackSize[]` | yes | Fixed logical units, intrinsic `auto`, or fractional remaining space. At least one. |
| `rows` | `TrackSize[]?` | no | Explicit rows; omitted rows are created as `auto`. |
| `columnGap` | `float64` | no | `0`. |
| `rowGap` | `float64` | no | `0`. |
| `justifyItems` | `Alignment` | no | `stretch`; horizontal alignment in cells. |
| `alignItems` | `Alignment` | no | `stretch`; vertical alignment in cells. |

Children use `gridColumn`, `gridRow`, `columnSpan`, and `rowSpan` from `UiCommonProps`. Auto-placement is row-major and skips cells already claimed by explicitly placed children. Explicit placements may overlap; overlapping children paint in `children` order, like a `ZStack`. A span or explicit position past the last declared column is a validation error; past the last declared row it creates implicit `auto` rows. The model borrows the useful subset of [CSS Grid](https://www.w3.org/TR/css-grid-2/) and directly maps DrawnUI's grid definitions and attached row/column properties. `minmax`, named lines, dense placement, and subgrid are deferred.

### 7.5 `nx.ui.ZStack`

Overlay container; accepts zero or more UI children. Later children paint above earlier children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `justifyItems` | `Alignment` | `stretch` | Default horizontal child alignment. |
| `alignItems` | `Alignment` | `stretch` | Default vertical child alignment. |

The alignment properties are named `justifyItems`/`alignItems` so that a container that positions children on both axes spells it the same way `Grid` does, and so children override them with the same `justifySelf`/`alignSelf` pair.

Rationale: `ZStack` is z-order overlay and nothing else — children share one box and later children paint above earlier ones — which is what [SwiftUI's `ZStack`](https://developer.apple.com/documentation/swiftui/zstack) and Flutter's `Stack` mean, and what DrawnUI's `SkiaLayer` (an `Absolute` `SkiaLayout`, documented as behaving like a single-cell grid) provides. Free x/y positioning belongs in `nx.graphics.Drawing`, not here.

The `Z` prefix is doing real work. It names a geometry rather than a use case, so the component reads as correct for centering a single child or badging an avatar, not only for scrims and modals — which is the failure mode `Overlay`, the runner-up name, would have invited. It also survives contact with the surrounding ecosystem in a way a bare `Stack` does not: see §7 for why an unprefixed “stack” reliably means *linear* almost everywhere else, including in DrawnUI itself.

### 7.6 `nx.ui.Scroll`

Viewport for exactly one UI child.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `axis` | `ScrollAxis` | `vertical` | Allowed scroll direction. |
| `scrollbar` | `ScrollbarVisibility` | `auto` | Host scrollbar presentation hint. |

The child is measured unbounded along each scrollable axis and constrained to the viewport on any non-scrollable axis; `Length` percentages on the child resolve against the viewport, not against the scrollable content. A `Scroll` never sizes itself to its content along a scrollable axis, so it needs a bounded `height` (or `width`) from its parent or from `UiCommonProps`.

Rationale: maps to DrawnUI [`SkiaScroll`](https://drawnui.net/articles/controls/scroll.html) `Orientation` + `Content`. Physics, offsets, zoom, snapping, sticky headers, refresh, virtualization, and load-more commands are intentionally outside a portable read-only MVP.

### 7.7 `nx.ui.Box`

Neutral single-child container. It has no component-specific properties beyond `UiCommonProps`.

Use it for padding, background, border, rounded clipping, shadow, or sizing around one child. It is the portable evolution of DrawnUI `ContentLayout` and the content-hosting aspect of `SkiaShape`, without conflating UI layout with geometric primitives.

### 7.8 `nx.ui.Card`

Themed single-child surface.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | theme | Explicit values override theme defaults. |
| `variant` | `CardVariant` | `elevated` | Host-theme card profile. |

`Card` is retained despite overlap with `Box` because it is a highly familiar GenUI primitive in A2UI and json-render examples and lets a model request design-system intent without inventing border/shadow values. A renderer without card theming must fall back to a documented `Box` profile.

### 7.9 `nx.ui.Divider`

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared layout properties; background/border are ignored. |
| `axis` | `Axis` | `horizontal` | Line direction. |
| `color` | `Color?` | theme | Divider color. |
| `thickness` | `float64` | `1` | Line thickness. |

Rationale: directly matches A2UI's horizontal/vertical `Divider`. Length is controlled by common width/height and parent alignment rather than another bespoke property.

### 7.10 `nx.ui.Text`

Measured, wrapping output text; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `text` | `string` | yes | Literal displayed content. |
| `format` | `TextFormat` | no | `plain`; markdown uses a host-defined safe common subset. |
| `variant` | `TextVariant` | no | `body`; theme typography role. |
| `color` | `Color?` | no | Theme value. |
| `fontFamily` | `string?` | no | Variant/theme value. |
| `fontSize` | `float64?` | no | Variant/theme value. |
| `fontWeight` | `float64?` (1..1000) | no | Variant/theme value. |
| `fontStyle` | `FontStyle` | no | `normal`. |
| `textAlign` | `TextAlign` | no | `start`. |
| `lineHeight` | `float64?` | no | Unitless multiplier; theme default. |
| `letterSpacing` | `float64` | no | `0`. |
| `maxLines` | `int?` (>= 1) | no | Unlimited. |
| `overflow` | `TextOverflow` | no | `clip`; applies when constrained/maxLines is reached. |

`variant` is a semantic role, not only a type ramp: renderers must expose `h1`, `h2`, and `h3` as platform heading elements at the corresponding level, and `code` with a monospace face and a code role where the platform has one. Nothing else in the read-only MVP conveys document structure to assistive technology, so a model that reaches for `fontSize` instead of `variant` silently produces a flat, unnavigable document.

When `format` is `"markdown"`, `variant` sets the *baseline* role for the block and Markdown structure nests below it; heading syntax inside a `caption` must not escape its container's typography scale.

Rationale: A2UI `Text` combines simple Markdown with typography variants (`h1`–`h5`, `body`, `caption`); DrawnUI `SkiaLabel` supplies detailed text properties (`FontFamily`, `FontSize`, `FontWeight`, `LineHeight`, `CharacterSpacing`, `MaxLines`, `LineBreakMode`, stroke and drop shadow). NX keeps the useful intersection and makes semantic `variant` preferable to low-level overrides. Hosts must sanitize Markdown, disallow raw HTML by default, and apply a link policy. Rich spans, `TextTransform`, and `AutoSize`-style shrink-to-fit are deferred.

### 7.11 `nx.ui.Image`

Measured raster/vector image output; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `source` | `ImageSource` | yes | Trusted resource or host-approved URI. |
| `alt` | `string` | yes | Accessible alternative; use `""` only when decorative. |
| `fit` | `ObjectFit` | no | `contain`. |
| `aspectRatio` | `float64?` | no | Intrinsic ratio when dimensions do not establish one. |

`alt` is the image's accessible name: `alt: ""` is exactly equivalent to `accessibility.hidden: true`, and supplying both `alt` and `accessibility.label` is a validation error rather than a silently resolved conflict. `accessibility.description` remains available for a longer description alongside `alt`.

Rationale: aligns with A2UI `Image` (`url`, `description`, `fit`), CSS object-fit vocabulary, and DrawnUI `SkiaImage.Source`/`Aspect`. A dedicated `alt` is kept rather than pushing authors to `accessibility.label` because it is the token a generating model already produces for images, and because making it required is the only way to force the decision at generation time. SVG files may be supplied as trusted image resources; editable/composable SVG content should be represented with `nx.graphics` primitives. DrawnUI's `ColorTint`, sprite-sheet, and `Tile` aspects have no MVP equivalent.

### 7.12 `nx.ui.Icon`

Host-provided symbolic icon; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `name` | `string` | yes | Icon name from the negotiated host icon catalog. |
| `set` | `string?` | no | Host/default icon set if omitted. |
| `size` | `float64?` | no | Theme default, normally 24. Sets both dimensions; an explicit `width`/`height` from `UiCommonProps` wins over `size`. |
| `color` | `Color?` | no | Current theme/content color. |

Rationale: A2UI's `Icon.name` is primarily a predefined enum of system icon names, with an escape hatch (`{ "svgPath": ... }`) for supplying path data directly. NX takes the enumerated half and drops the escape hatch: a model that needs custom vector art already has `nx.graphics`, where the geometry is validated, bounded, and composable, so admitting a second, unmeasured path-data channel through `Icon` would add attack surface for no new capability. Use common `accessibility.label` for a meaningful icon or `accessibility.hidden: true` for a decorative one. A catalog capability exchange must enumerate supported names; unknown icons use a documented fallback glyph.

## 8. Minimal example

```json
{
  "format": "nx-ui-json",
  "schemaVersion": "0.1.0",
  "catalogs": [
    { "id": "nx.ui", "version": "0.1.0" },
    { "id": "nx.graphics", "version": "0.1.0" }
  ],
  "root": "card",
  "elements": {
    "card": {
      "type": "nx.ui.Card",
      "props": { "padding": 20, "width": 360 },
      "children": ["content"]
    },
    "content": {
      "type": "nx.ui.VStack",
      "props": { "gap": 12 },
      "children": ["title", "graphic", "caption"]
    },
    "title": {
      "type": "nx.ui.Text",
      "props": { "text": "NX Drawn UI", "variant": "h2" }
    },
    "graphic": {
      "type": "nx.graphics.Drawing",
      "props": {
        "height": 140,
        "viewBox": { "x": 0, "y": 0, "width": 320, "height": 140 }
      },
      "children": ["background", "curve", "dot"]
    },
    "background": {
      "type": "nx.graphics.Rect",
      "props": {
        "width": 320,
        "height": 140,
        "rx": 12,
        "fill": {
          "type": "linearGradient",
          "x1": 0, "y1": 0, "x2": 1, "y2": 1,
          "stops": [
            { "offset": 0, "color": "#5B5BD6" },
            { "offset": 1, "color": "#14B8A6" }
          ]
        }
      }
    },
    "curve": {
      "type": "nx.graphics.Path",
      "props": {
        "data": "M20 105 C90 15 210 125 300 35",
        "fill": "none",
        "stroke": { "paint": "white", "width": 5, "lineCap": "round" }
      }
    },
    "dot": {
      "type": "nx.graphics.Circle",
      "props": { "cx": 300, "cy": 35, "r": 7, "fill": "white" }
    },
    "caption": {
      "type": "nx.ui.Text",
      "props": {
        "text": "Portable layout above; portable drawing below.",
        "variant": "caption"
      }
    }
  }
}
```

Note what the example does *not* do: the caption takes its color from the `caption` variant instead of naming one. A literal such as `"#555"` is legible on the light surface a model imagines and nearly invisible on a dark one, and nothing else in the document can correct it. Literal colors are appropriate for graphics that supply their own ground — the white curve above sits on a gradient this document draws — and inappropriate for content the host themes. §12.9 tracks whether the MVP needs semantic color roles to make that distinction enforceable rather than advisory.

### 8.1 The same document as NX source

The JSON above is the interchange form. The same document authored in NX is the tree a person or a compiler would actually write:

```nx
import "../core"
import { Card as ui.Card, VStack as ui.VStack, Text as ui.Text, TextVariant } from "../ui"
import "../graphics" as gfx

<ui.Card width={<Length.px value={360.0} />}
         padding={<Insets top={20.0} right={20.0} bottom={20.0} left={20.0} />}>
  <ui.VStack gap={12.0}>
    <ui.Text: variant={TextVariant.h2}>NX UI</ui.Text>
    <gfx.Drawing
      height={<Length.px value={140.0} />}
      viewBox={<ViewBox x={0.0} y={0.0} width={320.0} height={140.0} />}>
      <gfx.Rect
        width={320.0} height={140.0} rx={12.0}
        fill={<Paint.linearGradient
                x1={0.0} y1={0.0} x2={1.0} y2={1.0}
                stops={ <GradientStop offset={0.0} color={"#5B5BD6"} />
                        <GradientStop offset={1.0} color={"#14B8A6"} /> } />} />
      <gfx.Path
        data={"M20 105 C90 15 210 125 300 35"}
        fill={Paint.none}
        stroke={<Stroke paint={<Paint.solid color={"white"} />} width={5.0} lineCap={LineCap.round} />} />
      <gfx.Circle cx={300.0} cy={35.0} r={7.0} fill={<Paint.solid color={"white"} />} />
    </gfx.Drawing>
    <ui.Text: variant={TextVariant.caption}>Portable layout above; portable drawing below.</ui.Text>
  </ui.VStack>
</ui.Card>
```

Four differences from the JSON are worth reading off directly, because each one is a property of NX rather than of this proposal.

The IDs are gone. NX nests, so structure is expressed by containment rather than by `children: ["title", "graphic"]` pointing into a map. Producing the flat form is a compilation step, not something NX source can state.

The prefixes are one segment deep. `ui.` and `gfx.` are import aliases, and NX allows exactly one qualifying segment, so `nx.ui.` and `nx.graphics.` are not spellable as prefixes — the alias is the local abbreviation for a catalog whose real ID lives in `catalogs`. The two catalogs are also imported differently on purpose: `nx.graphics` uses the wildcard alias form, while `nx.ui` uses the selective form so that `TextVariant` arrives unqualified, because an enum member cannot be reached through a wildcard alias today. Appendix B has the detail.

Sizes and insets got longer. `"width": 360` becomes `<Length.px value={360.0} />` and `"padding": 20` becomes a four-sided `Insets`, both because NX cannot union a scalar with a structured alternative.

Text reads better. `<ui.Text: ...>NX UI</ui.Text>` uses NX's text-element form, which binds the body to the component's `content` property — the one place where NX authoring is more compact than the JSON, not less.

## 9. DrawnUI evolution and adapter mapping

| DrawnUI today | NX MVP | Evolution rationale |
|---|---|---|
| `Canvas` + `SkiaControl` base capabilities | `NxUiDocument`, `Element`, `UiCommonProps` | Separate portable document semantics from renderer lifecycle, invalidation, cache, and GPU configuration. DrawnUI's `Canvas` hosts an entire drawn UI tree, so it corresponds to the document host — not to `nx.graphics.Drawing`, which is an embeddable element. That mismatch is why the portable model does not reuse the word `Canvas`; see §1. |
| no direct equivalent | `nx.graphics.Drawing` | DrawnUI has no embeddable coordinate-space element; an adapter synthesizes one as an `Absolute` `SkiaLayout` sized to the surface, with the view-box mapping applied as a transform on its children. |
| `SkiaLayout Type="Row"` / `SkiaRow` | `nx.ui.HStack` | Explicit catalog type; `Spacing` becomes `gap`; MAUI alignment maps to `align`/`justify`/`alignSelf`. |
| `SkiaLayout Type="Column"` / `SkiaStack` alias | `nx.ui.VStack` | DrawnUI’s `SkiaStack` is a `Column`; the axis prefix makes the direction explicit instead of implied. |
| `SkiaLayout Type="Grid"` / `SkiaGrid` | `nx.ui.Grid` | Preserve rows, columns, gaps, placement, and spans in a smaller typed subset. |
| `SkiaLayout Type="Absolute"` / `SkiaLayer` | `nx.ui.ZStack` for overlay; `nx.graphics.Drawing` for coordinates | Separate ordinary layering from coordinate-based graphics. |
| `SkiaLayout Type="Wrap"` / `SkiaWrap` | `HStack` or `VStack` with `wrap: true` | Avoid another component when flex-like wrapping is enough. |
| `ContentLayout` | `nx.ui.Box` | Neutral single-child decoration/layout primitive. |
| `SkiaShape Type="Rectangle\|Circle\|Ellipse\|Line\|Polygon\|Path"` | corresponding `nx.graphics.*` types | Smaller schemas, SVG-standard names, direct validation. |
| `SkiaShape` hosting content/clipping | `nx.ui.Box`/`Card` for UI; `Drawing` + `clipPath` for graphics | Preserve the capability while separating UI content layout from shape geometry. |
| `BackgroundColor` / `BackgroundGradient` | `fill` for shapes; `background` for UI | SVG uses fill for geometry; UI ecosystems use background for boxes. This terminology distinction is intentional. |
| `SkiaGradient` (`Colors` + `ColorPositions`, `Start*/End*Ratio`, `Angle`, `TileMode`) | `LinearGradient` / `RadialGradient` with ordered `stops` | Parallel arrays zip into stops; ratio coordinates already match `objectBoundingBox`. `Angle` is a producer-side convenience over the same two points; `TileMode` is deferred. |
| `StrokeColor`, `StrokeGradient`, `StrokeWidth`, `StrokeCap`, `StrokePath` | `Stroke { paint, width, lineCap, dashArray, ... }` | Groups related stroke state and aligns with SVG. |
| `SkiaLabel` | `nx.ui.Text` or `nx.graphics.Text` | Explicitly distinguish measured UI text from positioned drawing text. |
| `SkiaImage` / `SkiaSvg` | `nx.ui.Image`, `nx.graphics.Image`, `nx.ui.Icon`, or decomposed `nx.graphics` geometry | Choose by intent: ordinary content, positioned image, host symbol, or editable vector scene. |
| `SkiaScroll` | `nx.ui.Scroll` | Keep direction + content; leave physics, zoom, refresh, virtualization, and offsets to later profiles. |
| DrawnUI cache modes, resampling, acceleration, invalidation | renderer configuration | These affect implementation/performance, not portable UI meaning. |
| gestures, commands, binding, controls | post-MVP interaction/state catalogs | Strictly excluded from the read-only initial MVP. |

DrawnUI source links for adapter work: [`SkiaShape.cs`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/SkiaShape.cs), [`SkiaLabel.cs`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/Text/SkiaLabel.cs), [`SkiaImage.cs`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/Images/SkiaImage.cs), and [`SkiaScroll.cs`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/Scroll/SkiaScroll.cs).

## 10. Rendering, conformance, and safety requirements

An MVP renderer should:

1. validate the entire document against exact catalog schemas before rendering;
2. reject cycles, missing IDs, illegal child types, non-finite numbers, negative dimensions where prohibited, and excessive complexity;
3. enforce limits for element count, tree depth, text length, image byte/dimension count, gradient stops, shadows, path characters/commands, and render surface size;
4. resolve only host-approved resources and URI schemes/domains;
5. sanitize Markdown and apply a host link policy;
6. expose accessible text and image alternatives through the native platform accessibility tree;
7. render a deterministic error placeholder rather than executing or guessing at unknown content;
8. report unsupported-but-valid visual features diagnostically, with documented fallbacks;
9. keep renderer performance settings outside the generated document;
10. preserve element IDs in diagnostics and, where practical, the rendered accessibility/debug tree.

Conformance should be semantic, not pixel-perfect. A conforming renderer must preserve layout relationships, paint order, clipping, content, and accessibility meaning within documented font/rasterization tolerances.

## 11. Deliberately deferred features

### Post-MVP input and interaction components

- `Button`, `Link`, `TextField`, `TextArea`, `CheckBox`, `RadioGroup`, `Select`, `Slider`, `Switch`, `DateTimeInput`, `FilePicker`
- focus, keyboard navigation, pointer/gesture events, actions, command/tool calls, validation, disabled/read-only states, form submission
- dialogs/modals, tabs, disclosure/accordion, menus, tooltips, drag/drop, selection

### Post-MVP Level 3 semantic/data UI

- `Metric`, `Badge`, `Progress`, `Chart` families, `DataTable`, `List`, `Tree`, `Timeline`, `Map`
- `Form`, `Detail`, `Comparison`, `Gallery`, `Calendar`, `Schedule`
- domain catalogs such as `ContactCard`, `ProductCard`, `OrderSummary`, `FlightOption`

### Post-MVP drawing and media

- arcs as a convenience primitive; continuous-corner “squircle” rounding (both exist as DrawnUI `ShapeType` values); rounded/smoothed polylines; compound/reusable geometry definitions; inheritable paint on `Group`
- rich text spans, text-on-path, advanced shaping controls, font resources, text decoration and transform, wrapped text inside drawing coordinates
- masks, arbitrary clip trees, blend/compositing modes, filters, blur/glow, inner shadows
- conic gradients, gradient spread modes, patterns, nine-patch images, tint/color filters
- animation, transitions, timelines, morphing, skeletal/vector animation, video, audio
- hit testing, gestures, canvas camera/pan/zoom, scene virtualization, accessibility geometry
- 3D, custom shaders, and renderer-specific extension profiles

### Post-MVP GenUI/runtime capabilities

- literal-or-bound property values, state model, JSON Pointer references, conditions, repeats, templates, and named slots
- actions/events and AG-UI/A2UI/MCP transport adapters
- incremental surface operations and RFC 6902 JSON Patch streaming
- catalog negotiation, custom catalogs, themes/design tokens, capability fallbacks, migrations
- localization, bidirectional text policy, logical (`start`/`end`) insets, responsive breakpoints, environment values
- per-element extension metadata under a reserved namespace, as A2UI v1.0 added, and the forward-compatibility policy that would make it safe

## 12. Decisions to validate in a prototype

1. **Flat canonical form:** measure generation error rate and streaming ergonomics against nested authoring syntax.
2. **Component count:** confirm `Box` and `Card` both earn their place; `Card` is the likelier convenience component to retain because GenUI models recognize it well.
3. **Layout container naming:** measure generation accuracy for `HStack`/`VStack`/`ZStack` (see §7) against the `Row`/`Column`/`Overlay` alternative, on prompts that mix linear layout with `Grid` placement. Two specific errors are worth counting, because the naming was chosen to prevent them: linear containers emitted as `Grid` children in place of cell content, and `ZStack` used where a `VStack` was meant. The A2UI-alignment cost is real, so if `Row`/`Column` measurably wins on ordinary layout prompts and the `Grid` confusion does not appear, the decision should be revisited — renaming three catalog entries is cheap while the format is pre-1.0.
4. **Gradient coordinates:** verify that `objectBoundingBox` defaults produce the most intuitive model output across Skia and SVG.
5. **Text parity:** quantify layout differences between Skia/DrawnUI and web/native renderers, especially `lineHeight`, weight, and ellipsis.
6. **Path safety:** establish practical complexity and bounds limits without rejecting normal illustrations.
7. **Path fill default:** SVG's black default fill is the single most common generation error in hand-written path art — an open stroked curve arrives filled. Measure whether `nx.graphics.Path` should default to `fill: "none"` and diverge from SVG, as `Polyline` already does.
8. **Common-property surface:** `UiCommonProps` puts roughly twenty properties on every Level 2 component, including leaves. A2UI's `ComponentCommon` carries only `id`, `accessibility`, and `weight`, leaving all decoration to the host theme. Measure whether decoration (`background`, `border`, `cornerRadius`, `shadows`, `clip`) belongs on every component or only on the containers — `Box`, `Card`, `HStack`, `VStack`, `ZStack`, `Grid`, `Drawing` — with leaves keeping only sizing, spacing, alignment, opacity, and accessibility.
9. **Semantic color roles:** decide whether the MVP needs a small set of theme roles (surface, onSurface, muted, accent) usable anywhere a `Color` is accepted, so that generated content survives a dark theme. Read-only documents are the easiest place to get this right and the easiest place to accidentally hard-code a palette.
10. **Encoding cost:** the flat map repeats an ID for every element and pays JSON's punctuation everywhere. OpenUI Lang's premise is that this is expensive enough to justify a purpose-built surface syntax. Measure NX's tokens-per-rendered-element against a nested form and against a compact form before the flat map is locked in as the canonical *wire* representation; it can remain the canonical *validation* representation regardless.
11. **Catalog schemas:** publish JSON Schema for `nx.ui@0.1.0` and `nx.graphics@0.1.0`, then run constrained-generation evals.
12. **DrawnUI adapter:** implement the example document through a thin NX-to-DrawnUI mapping before adding any new component.

## 13. Recommended MVP acceptance test

The MVP is credible when one unchanged validated document can render through both:

- a DrawnUI/Skia renderer; and
- a web renderer using normal layout plus SVG or Canvas for `nx.graphics`.

The test corpus should include a text/image card, grid dashboard shell, layered composition, clipped image, gradient/shadow illustration, multi-segment path, and accessible decorative/informative images. It should also include intentionally invalid documents for every validation rule.

## 14. Primary references

### Generative UI

- [A2UI v0.9.1 specification](https://a2ui.org/specification/v0.9.1-a2ui/) (current), [A2UI v1.0 specification](https://a2ui.org/specification/v1.0-a2ui/) (release candidate), [v1.0 basic catalog implementation guide](https://a2ui.org/specification/v1.0-basic-catalog-implementation-guide/), and the [basic catalog component definitions](https://github.com/google/A2UI/blob/main/agent_sdks/python/a2ui_core/src/a2ui/core/basic_catalog/components.py)
- [json-render specification anatomy](https://json-render.dev/docs/specs) and [core TypeScript types](https://github.com/vercel-labs/json-render/blob/main/packages/core/src/types.ts)
- [Vercel AI SDK: Generative User Interfaces](https://ai-sdk.dev/docs/ai-sdk-ui/generative-user-interfaces)
- [CopilotKit: Generative UI overview](https://docs.copilotkit.ai/concepts/generative-ui-overview)
- [CopilotKit: A2UI](https://docs.copilotkit.ai/a2a/generative-ui/a2ui)
- [AG-UI: Agent–User Interaction protocol](https://docs.ag-ui.com/)
- [MCP Apps overview](https://modelcontextprotocol.io/extensions/apps/overview) and [MCP Apps specification site](https://apps.extensions.modelcontextprotocol.io/)
- [Thesys C1 overview](https://docs.thesys.dev/guides/what-is-thesys-c1) and [`C1Component`](https://docs.thesys.dev/react-reference/c1-component)
- [OpenUI Lang](https://www.openui.com/docs/openui-lang) and its [React component libraries](https://www.openui.com/docs/api-reference/react-ui)

### Drawing, layout, and accessibility standards

- [SVG 2: Shapes](https://www.w3.org/TR/SVG2/shapes.html), [Paths](https://www.w3.org/TR/SVG2/paths.html), [Painting](https://www.w3.org/TR/SVG2/painting.html), [Coordinates](https://www.w3.org/TR/SVG2/coords.html), [Text](https://www.w3.org/TR/SVG2/text.html), and [Gradients](https://www.w3.org/TR/SVG2/pservers.html)
- [WHATWG HTML: Canvas](https://html.spec.whatwg.org/multipage/canvas.html#the-canvas-element)
- [CSS Color 4](https://www.w3.org/TR/css-color-4/), [CSS Transforms](https://www.w3.org/TR/css-transforms-1/), [CSS Box Alignment](https://www.w3.org/TR/css-align-3/), [CSS Flexible Box Layout](https://www.w3.org/TR/css-flexbox-1/), and [CSS Grid](https://www.w3.org/TR/css-grid-2/)
- [W3C Accessible Name and Description Computation 1.2](https://www.w3.org/TR/accname-1.2/)

### DrawnUI for .NET

- [DrawnUI controls overview](https://drawnui.net/articles/controls/index.html)
- [DrawnUI layout controls](https://drawnui.net/articles/controls/layouts.html)
- [DrawnUI shapes and SVG paths](https://drawnui.net/articles/controls/shapes.html)
- [DrawnUI scroll controls](https://drawnui.net/articles/controls/scroll.html)
- [DrawnUI.Net GitHub repository](https://github.com/DrawnUi/DrawnUi.Net)

### NX language

- [NX grammar (EBNF)](../nx-grammar.md) and [machine-readable grammar spec](../nx-grammar-spec.md)
- [NX types reference](src/content/docs/reference/syntax/types.md) and [elements reference](src/content/docs/reference/syntax/elements.md)
- [`component-syntax` specification](../openspec/specs/component-syntax/spec.md) and [`component-contract-inheritance` specification](../openspec/specs/component-contract-inheritance/spec.md), which define `external component`

## Appendix A — The catalogs as NX source

Three libraries: `nx/core` holds the shared value types from §5, `nx/ui` holds Level 2, and `nx/graphics` holds Level 1. `nx/graphics` imports `UiCommon` from `nx/ui` because `Drawing` is the element that carries a graphics scene into a laid-out box; nothing flows the other way.

Every declaration below was checked with `nxlang typegen` against the NX toolchain in this repository. The one construct that does not check today is the cross-library use shown in §8.1; Appendix B says why.

### `nx/core/types.nx`

```nx
export type Color = string          // CSS Color syntax subset accepted by the host
export type ElementId = string      // ^[A-Za-z_][A-Za-z0-9_.-]{0,63}$

export enum Alignment = start | center | end | stretch
export enum AlignSelf = auto | start | center | end | stretch
export enum Anchor = start | center | end
export enum Axis = horizontal | vertical
export enum Distribution = start | center | end | spaceBetween | spaceAround | spaceEvenly
export enum ObjectFit = contain | cover | fill | none | scaleDown
export enum FontStyle = normal | italic
export enum GradientUnits = objectBoundingBox | userSpaceOnUse
export enum LineCap = butt | round | square
export enum LineJoin = miter | round | bevel

export type Length =
  | auto
  | px { value: float64 }
  | percent { value: float64 }

export type Point = {
  x: float64
  y: float64
}

export type ViewBox = {
  x: float64
  y: float64
  width: float64
  height: float64
}

export type Insets = {
  top: float64 = 0.0
  right: float64 = 0.0
  bottom: float64 = 0.0
  left: float64 = 0.0
}

export type CornerRadii = {
  topLeft: float64 = 0.0
  topRight: float64 = 0.0
  bottomRight: float64 = 0.0
  bottomLeft: float64 = 0.0
}

export type GradientStop = {
  offset: float64                       // 0..1
  color: Color
  opacity: float64 = 1.0                // 0..1
}

export type Paint =
  | none
  | solid { color: Color }
  | linearGradient {
      x1: float64
      y1: float64
      x2: float64
      y2: float64
      units: GradientUnits = {GradientUnits.objectBoundingBox}
      stops: GradientStop[]
    }
  | radialGradient {
      cx: float64
      cy: float64
      r: float64
      fx: float64?
      fy: float64?
      units: GradientUnits = {GradientUnits.objectBoundingBox}
      stops: GradientStop[]
    }

export type Stroke = {
  paint: Paint
  width: float64 = 1.0
  lineCap: LineCap = {LineCap.butt}
  lineJoin: LineJoin = {LineJoin.miter}
  miterLimit: float64 = 4.0
  dashArray: float64[]?
  dashOffset: float64 = 0.0
}

export type Shadow = {
  color: Color
  offsetX: float64 = 0.0
  offsetY: float64 = 0.0
  blur: float64 = 0.0                   // >= 0
}

export type Transform =
  | translate { x: float64 y: float64 }
  | scale { x: float64 y: float64? }        // null y means uniform scale
  | rotate { degrees: float64 cx: float64 = 0.0 cy: float64 = 0.0 }
  | skewX { degrees: float64 }
  | skewY { degrees: float64 }
  | matrix { a: float64 b: float64 c: float64 d: float64 e: float64 f: float64 }

export type Border = {
  paint: Paint
  width: float64 = 1.0
}

export type ImageSource =
  | uri { uri: string }
  | resource { name: string }

export type Accessibility = {
  label: string?
  description: string?
  hidden: boolean = false
}
```

### `nx/ui/catalog.nx`

```nx
import "../core"

export enum ScrollAxis = horizontal | vertical | both
export enum ScrollbarVisibility = auto | visible | hidden
export enum CardVariant = outlined | elevated | filled
export enum TextFormat = plain | markdown
export enum TextVariant = h1 | h2 | h3 | title | body | caption | code
export enum TextAlign = start | center | end | justify
export enum TextOverflow = clip | ellipsis

export type TrackSize =
  | auto
  | fixed { value: float64 }
  | fraction { value: float64 }

export abstract external component <UiCommon
  width: Length = {Length.auto}
  height: Length = {Length.auto}
  minWidth: float64 = 0.0
  minHeight: float64 = 0.0
  maxWidth: float64?
  maxHeight: float64?
  margin: Insets?
  padding: Insets?
  alignSelf: AlignSelf = {AlignSelf.auto}
  justifySelf: AlignSelf = {AlignSelf.auto}
  grow: float64 = 0.0
  shrink: float64 = 1.0
  gridColumn: int?
  gridRow: int?
  columnSpan: int = 1
  rowSpan: int = 1
  background: Paint = {Paint.none}
  border: Border?
  cornerRadius: CornerRadii?
  shadows: Shadow[]?
  clip: boolean = false
  opacity: float64 = 1.0
  accessibility: Accessibility?
/>

export external component <HStack extends UiCommon
  gap: float64 = 0.0
  justify: Distribution = {Distribution.start}
  align: Alignment = {Alignment.stretch}
  wrap: boolean = false
  content children: Element[]?
/>

export external component <VStack extends UiCommon
  gap: float64 = 0.0
  justify: Distribution = {Distribution.start}
  align: Alignment = {Alignment.stretch}
  wrap: boolean = false
  content children: Element[]?
/>

export external component <Grid extends UiCommon
  columns: TrackSize[]
  rows: TrackSize[]?
  columnGap: float64 = 0.0
  rowGap: float64 = 0.0
  justifyItems: Alignment = {Alignment.stretch}
  alignItems: Alignment = {Alignment.stretch}
  content children: Element[]?
/>

export external component <ZStack extends UiCommon
  justifyItems: Alignment = {Alignment.stretch}
  alignItems: Alignment = {Alignment.stretch}
  content children: Element[]?
/>

export external component <Scroll extends UiCommon
  axis: ScrollAxis = {ScrollAxis.vertical}
  scrollbar: ScrollbarVisibility = {ScrollbarVisibility.auto}
  content child: Element
/>

export external component <Box extends UiCommon
  content child: Element
/>

export external component <Card extends UiCommon
  variant: CardVariant = {CardVariant.elevated}
  content child: Element
/>

export external component <Divider extends UiCommon
  axis: Axis = {Axis.horizontal}
  color: Color?
  thickness: float64 = 1.0
/>

export external component <Text extends UiCommon
  format: TextFormat = {TextFormat.plain}
  variant: TextVariant = {TextVariant.body}
  color: Color?
  fontFamily: string?
  fontSize: float64?
  fontWeight: float64?
  fontStyle: FontStyle = {FontStyle.normal}
  textAlign: TextAlign = {TextAlign.start}
  lineHeight: float64?
  letterSpacing: float64 = 0.0
  maxLines: int?
  overflow: TextOverflow = {TextOverflow.clip}
  content text: string
/>

export external component <Image extends UiCommon
  source: ImageSource
  alt: string
  fit: ObjectFit = {ObjectFit.contain}
  aspectRatio: float64?
/>

export external component <Icon extends UiCommon
  name: string
  set: string?
  size: float64?
  color: Color?
/>
```

### `nx/graphics/catalog.nx`

```nx
import "../core"
import { UiCommon } from "../ui"

export enum FillRule = nonzero | evenodd
export enum ViewBoxFit = contain | cover | fill
export enum TextAnchor = start | middle | end
export enum DominantBaseline = auto | middle | hanging

export type ContentAlignment = {
  x: Anchor = {Anchor.center}
  y: Anchor = {Anchor.center}
}

export abstract external component <GraphicsCommon
  opacity: float64 = 1.0
  transform: Transform[]?
  clipPath: string?
  accessibility: Accessibility?
/>

export abstract external component <ShapeCommon extends GraphicsCommon
  fill: Paint?
  stroke: Stroke?
  fillRule: FillRule = {FillRule.nonzero}
  shadows: Shadow[]?
/>

export external component <Drawing extends UiCommon
  viewBox: ViewBox
  fit: ViewBoxFit = {ViewBoxFit.contain}
  contentAlignment: ContentAlignment?
  content children: Element[]?
/>

export external component <Group extends GraphicsCommon
  content children: Element[]?
/>

export external component <Rect extends ShapeCommon
  x: float64 = 0.0
  y: float64 = 0.0
  width: float64
  height: float64
  rx: float64 = 0.0
  ry: float64?                          // null means "same as rx"
/>

export external component <Circle extends ShapeCommon
  cx: float64
  cy: float64
  r: float64
/>

export external component <Ellipse extends ShapeCommon
  cx: float64
  cy: float64
  rx: float64
  ry: float64
/>

export external component <Line extends ShapeCommon
  x1: float64
  y1: float64
  x2: float64
  y2: float64
/>

export external component <Polyline extends ShapeCommon
  points: Point[]
/>

export external component <Polygon extends ShapeCommon
  points: Point[]
/>

export external component <Path extends ShapeCommon
  data: string
/>

export external component <Text extends GraphicsCommon
  x: float64
  y: float64
  fontFamily: string?
  fontSize: float64 = 16.0
  fontWeight: float64 = 400.0
  fontStyle: FontStyle = {FontStyle.normal}
  textAnchor: TextAnchor = {TextAnchor.start}
  dominantBaseline: DominantBaseline = {DominantBaseline.auto}
  letterSpacing: float64 = 0.0
  fill: Paint?
  stroke: Stroke?
  shadows: Shadow[]?
  content text: string
/>

export external component <Image extends GraphicsCommon
  x: float64 = 0.0
  y: float64 = 0.0
  width: float64
  height: float64
  source: ImageSource
  alt: string
  fit: ObjectFit = {ObjectFit.contain}
/>
```

## Appendix B — Where NX syntax does not yet fit this model

The same findings are tracked as numbered NX language items in
[drawn-ui-proposal-nx-enhancements.md](drawn-ui-proposal-nx-enhancements.md), with reproducers and
suggested enhancements; this appendix states them from the object model's point of view.

NX turns out to fit this proposal better than expected in one important respect and worse in another, and the two are worth separating.

The fit is `external component`. A catalog entry is precisely a component signature with no NX body, rendered by a host — which is what `external component` already means, down to `abstract external component` for shared property bundles and `extends` for reuse. The `UiCommonProps`/`GraphicsCommonProps`/`ShapeProps` structure in §6.1 and §7.1 was designed before this was known and maps onto it without adjustment. NX also gets the `content` property right: the wire format's `children` is a declared property with a declared type, so a container that accepts many children (`content children: Element[]?`) and one that accepts exactly one (`content child: Element`) are different signatures rather than a prose rule.

The misfit is the shape of the data. What follows are the specific gaps, in rough order of how much they cost.

**No map type, and no generics.** `elements: Record<ElementId, Element>` has no NX spelling; NX has records, sequences, unions, and enums, and generic type parameters are a post-1.0 item. The listing uses `Element[]`, which is not the same type — it loses key-uniqueness and O(1) ID lookup, and it forces `id` onto the element rather than onto the map key, which is why §4.1 keeps the map in the table and flags the divergence. It costs less than it looks: `elements` is the *only* map-typed site in the model, and NX has no occasion to author it, because NX source is the nested form and the flat map is what a compiler emits. The gap becomes real for the post-MVP catalogs in §11 — a state or data model, themes and design tokens, localization tables, per-element extension metadata — each of which needs a map-typed property inside a catalog.

**No literal types, so no compact scalar-or-structured values.** NX union cases are named, so a union cannot mix a bare scalar with an alternative shape. Three MVP types depend on exactly that: `Length` (`120` / `"auto"` / `"50%"`), `Insets` and `CornerRadii` (a scalar meaning all sides), and `Paint | "none"`. The listing spells `Length` as a three-case union and drops the scalar shorthands entirely, which is why `"padding": 20` becomes a four-field record in §8.1. String-literal union types are on NX's roadmap as a 1.1 feature; if they arrive, `Length` and `Insets` should be revisited, because the shorthand is not cosmetic — it is most of the token cost of a generated layout.

**Every record and union case carries a `$type` tag.** NX's serialization gives each record a `$type` discriminator and each union case a qualified one, so a `Point` is `{"$type": "Point", "x": 10, "y": 20}` and a solid fill is `{"$type": "Paint.solid", "color": "#fff"}` rather than `"#fff"`. The key is fixed and not configurable. This is the most consequential difference for a format whose §12.10 open question is token cost: the NX-native encoding is materially heavier than the JSON in §8, so NX source and NX serialization are separable decisions. Adopting the language does not oblige the wire format to adopt its encoding.

**Nullable is the only way to say "optional."** NX has no absent-versus-null distinction, so a property with no meaningful default is written `T?` and the renderer resolves null. Most of the `?` marks in §6 and §7 exist for this reason rather than because null is a meaningful value.

**A derived component cannot override an inherited default.** Redeclaring an inherited property is rejected as a duplicate. §6.1 wants `fill` to default to `Paint.none` on `Line` and `Polyline` and to black on closed shapes; that is not expressible, so `fill` is nullable on `ShapeCommon` and the per-component default becomes renderer prose.

**A default cannot reference a sibling property.** `Transform.scale.y` defaulting to `x`, and `nx.graphics.Rect.ry` defaulting to `rx`, both become nullable with the relationship stated in a comment. (Component `state` defaults *can* read props; property defaults cannot.)

**A default cannot be an empty sequence.** `= []` and `= {}` are both rejected — a braced default must contain at least one item, and bracket-list literals are not accepted in default position at all. Every `T[]` property whose default is "empty" is therefore written `T[]?`.

**A default cannot be a bare negative number or a bare enum member.** `= -1.0` and `= Alignment.start` are syntax errors; both must be braced, as `= {-1.0}` and `= {Alignment.start}`. Hence the `{...}` around every enum default in Appendix A. Integer literals also do not widen: `float64` properties must be written `= 0.0`, not `= 0`.

**No refinement or pattern constraints.** `ElementId`'s regex, `opacity`'s 0..1 range, `fontWeight`'s 1..1000, `columnSpan >= 1`, and "at least two gradient stops" are all invisible to NX's type system. They stay in the JSON Schema, which means the schema — not the NX declarations — remains the normative validator. The NX listing is the shape; the schema is the contract.

**An enum member cannot be reached through a wildcard import alias.** With `import "../ui" as ui`, the type `ui.TextVariant` resolves but `ui.TextVariant.h2` fails with *"Member access not yet implemented"* — the third qualifying segment is not supported. The workaround is the selective form used in §8.1, `import { ..., TextVariant } from "../ui"`, which brings the enum in unqualified; a wildcard import of the same library cannot be combined with it, because importing one library path twice in a file is an error.

**Qualification is one segment deep.** `import "../ui" as ui` takes a single identifier, and the selective form is validated to contain *exactly* one dot; `import "../ui" as nx.ui` is a syntax error and `import { VStack as nx.ui.VStack } from "../ui"` is rejected with "Selective import alias must contain exactly one dot." A two-segment catalog prefix is therefore not reachable in NX source. `nx.ui.VStack` remains the catalog type name in the document format; in NX it is written under a one-segment local alias, and the mapping from alias to catalog ID is the `catalogs` list.

**Structure is nested, and cannot be flat.** This follows from the first gap but is worth stating on its own: NX has no way to write the canonical flat element map, so NX is an authoring syntax that a compiler lowers into it. §3 already anticipated a recursive authoring form as sugar over the flat model; NX is that form.

Two further items are implementation gaps rather than language gaps, and both were reproduced against the toolchain in this repository:

- **Prop defaults on an imported external component are not applied at the call site.** Given `lib/a.nx` with `export external component <Rect extends Base x: float64 = 0.0 y: float64 />` and `app/main.nx` with `<lib.Rect y={5.0} />`, analysis reports *"Element 'lib.Rect' requires property 'x'"*. The same declarations in one library check cleanly.
- **Props inherited from an imported abstract external base are not visible at the call site.** With the same two libraries, `<lib.Rect ... width={2.0} fill={"red"} />` reports *"Element 'lib.Rect' has no property 'width'"* and the same for `fill`, though both are declared on `Base`.

Together these make the three-library layout in Appendix A uncheckable today: it is valid NX, and each library checks on its own, but a document that imports two catalogs cannot yet be type-checked against them. A single-library variant of the same catalog plus the §8.1 document does check end to end. Both should be filed against the NX toolchain before an NX-authored catalog is committed to.

Finally, one convention clash worth a decision rather than a fix: NX documents `snake_case` as the enum-member convention and serializes members verbatim, while this model's enum values are camelCase (`spaceBetween`, `objectBoundingBox`, `scaleDown`) because they follow CSS and SVG. camelCase members are legal NX identifiers, so Appendix A keeps the wire spelling and diverges from the convention. Changing the wire format to satisfy an NX style rule would be the wrong trade.

---

### Short form of the proposal

Use a cataloged flat document, borrow structure from A2UI/json-render, borrow graphics vocabulary and data formats from SVG, preserve DrawnUI's composability, and keep renderer mechanics out of the generated model. Ship read-only layout/content plus a compact vector scene graph first; add state, interaction, semantic components, and richer effects as separately versioned layers.
