# NX Drawn UI MVP Object Model

**Proposal status:** Draft for discussion  
**Research current through:** August 20, 2026  
**Scope:** Level 1 drawing and Level 2 generic, read-only UI  
**Explicitly excluded from this MVP:** input controls, actions/events, application state/data binding, animation, and Level 3 semantic/data UI

## Executive recommendation

NX should define a small, declarative, renderer-neutral UI document that an LLM can generate and a trusted client can validate and render. The document should use:

- a flat, ID-addressed element map, inspired by [A2UI](https://a2ui.org/specification/v0.9-a2ui/) and [json-render](https://json-render.dev/docs/specs);
- namespaced, versioned component catalogs (`nx.ui` and `nx.draw`);
- familiar UI names (`Row`, `Column`, `Grid`, `Stack`, `Scroll`, `Text`, `Image`, `Icon`);
- SVG vocabulary and data formats for portable drawing (`Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, SVG path data, fill/stroke, transforms);
- a clean evolution path from DrawnUI for .NET rather than a wire-level copy of its MAUI/Skia API.

The initial catalog proposed here contains:

- **Level 1 — drawing:** `Drawing`, `Group`, `Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, `Text`, `Image`.
- **Level 2 — output/layout:** `Row`, `Column`, `Grid`, `Stack`, `Scroll`, `Box`, `Card`, `Divider`, `Text`, `Image`, `Icon`. (`nx.draw.Drawing` is also embeddable as Level 2 content.)

The two namespaces intentionally allow both `nx.draw.Text` and `nx.ui.Text`. The former is positioned glyph content in drawing coordinates; the latter is measured, wrapped UI text. That distinction avoids a deceptively complex “one Text does everything” object.

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

## 2. Relevant precedents and what NX should take from them

| Source | Current architectural role | What NX should adopt | What NX should not copy into this MVP |
|---|---|---|---|
| [Google A2UI v0.9](https://a2ui.org/specification/v0.9-a2ui/) | Cross-platform, declarative, streaming agent-to-UI protocol with a basic component catalog | Catalog-constrained generation; stable component IDs; `Row`, `Column`, `Text`, `Image`, `Icon`, `Card`, `Divider`; host-owned rendering | Data model, functions, validation, actions, modal/input components, and the complete wire protocol |
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
        +-------------------+
        |                   |
      nx.ui              nx.draw
  output + layout     portable graphics
        |                   |
        +---------+---------+
                  v
       trusted renderer / theme
      DrawnUI, web, or native host
```

NX describes intent and structure. It does not prescribe whether `nx.draw.Rect` becomes a Skia draw call, an SVG `<rect>`, a Canvas 2D operation, or a native view.

### Why a flat element map

A normalized map gives every component a stable identity and makes partial generation, validation, replacement, and future JSON Patch streaming straightforward. Both A2UI and json-render use ID-addressed/flat representations. Recursive authoring syntax may be offered as sugar, but the canonical interchange form should be flat.

### Why explicit component types instead of DrawnUI-style `Type` enums

DrawnUI uses one `SkiaLayout` with `Type="Row|Column|Grid|Absolute|Wrap"` and one `SkiaShape` with `Type="Rectangle|Circle|Ellipse|Path|..."`. NX should use `nx.ui.Row`, `nx.ui.Grid`, `nx.draw.Rect`, and so on because:

- each component receives a smaller, clearer property schema;
- invalid combinations become harder to generate;
- catalog capabilities are easier to negotiate;
- names align with SVG and current GenUI catalogs;
- renderers may still implement these as one shared internal class.

## 4. Canonical document model

Type notation in this document uses TypeScript-like syntax. Every property marked `?` is optional. Unless stated otherwise, numeric values must be finite JSON numbers and default units are logical/device-independent pixels.

### 4.1 `NxUiDocument`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `schemaVersion` | `string` | yes | NX document schema version. MVP value: `"0.1"`. |
| `catalogs` | `CatalogUse[]` | yes | Catalogs and exact versions needed to interpret component types. |
| `root` | `ElementId` | yes | ID of the single root element. |
| `elements` | `Record<ElementId, Element>` | yes | Flat element map. Every referenced ID must exist. Unreachable elements are invalid in MVP. |
| `metadata` | `DocumentMetadata` | no | Non-rendered document information. |

Precedent: json-render's [`Spec`](https://github.com/vercel-labs/json-render/blob/main/packages/core/src/types.ts) uses `root` and `elements`; A2UI's [`updateComponents`](https://a2ui.org/specification/v0.9-a2ui/) uses stable component IDs within a surface.

### 4.2 `CatalogUse`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `id` | `string` | yes | Reverse-DNS or registered catalog ID. Core IDs: `nx.ui`, `nx.draw`. |
| `version` | `string` | yes | Exact semantic version understood by the producer. MVP core version: `0.1.0`. |

Exact versions make generated documents reproducible. A later negotiation protocol may select mutually supported versions before generation.

### 4.3 `DocumentMetadata`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `title` | `string` | no | Human-readable document title. |
| `description` | `string` | no | Human-readable summary. |
| `generator` | `string` | no | Producer identifier for diagnostics, not behavior. |

### 4.4 `Element`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `type` | `ComponentType` | yes | Namespaced catalog type, such as `nx.ui.Column` or `nx.draw.Path`. |
| `props` | component-specific object | yes | Literal properties validated against that component's catalog schema. Use `{}` when empty. |
| `children` | `ElementId[]` | conditional | Ordered child IDs. Allowed only by components documented as containers. |

`ElementId` and `ComponentType` are non-empty strings. IDs are unique within a document. MVP elements do **not** have `state`, `visible` expressions, `repeat`, `slots`, `on`, or `watch`; those are useful json-render precedents for post-MVP work.

### 4.5 Tree and validation rules

- The graph formed by `children` must be a rooted tree: no cycles and no child with multiple parents.
- Child order is paint order for `nx.draw.Drawing`, `nx.draw.Group`, and `nx.ui.Stack`; later children paint above earlier children.
- `nx.draw.*` primitives may appear only below `nx.draw.Drawing` or `nx.draw.Group`.
- `nx.draw.Drawing` may appear anywhere an `nx.ui` child is accepted.
- Unknown component types or properties fail validation in strict mode.
- Unknown enum values fail validation. A newer optional property can be ignored only after a schema-version policy explicitly permits it.
- All text, URIs, data sizes, element counts, and path complexity are subject to host limits.

## 5. Shared MVP value types

### 5.1 Geometry and sizing types

```ts
type Length = number | "auto" | `${number}%`;
type Insets = number | { top: number; right: number; bottom: number; left: number };
type Point = { x: number; y: number };
type ViewBox = { x: number; y: number; width: number; height: number };
type CornerRadii = number | {
  topLeft: number; topRight: number; bottomRight: number; bottomLeft: number;
};
type Alignment = "start" | "center" | "end" | "stretch";
type Distribution =
  | "start" | "center" | "end"
  | "spaceBetween" | "spaceAround" | "spaceEvenly";
type Axis = "horizontal" | "vertical";
```

`start`/`end` are chosen over `left`/`right` so layout can adapt to writing direction. The vocabulary follows [CSS Box Alignment](https://www.w3.org/TR/css-align-3/) and A2UI's `align`/`justify` conventions. DrawnUI adapters map these to MAUI layout options.

`Length` deliberately omits CSS `calc()`, viewport units, and arbitrary strings. `auto` means intrinsic sizing; percentages resolve against the containing content box.

### 5.2 Color and paint

```ts
type Color = string; // CSS Color syntax subset accepted by the host

type Paint = Color | LinearGradient | RadialGradient;

interface GradientStop {
  offset: number;       // 0..1
  color: Color;
  opacity?: number;     // 0..1, default 1
}

interface LinearGradient {
  type: "linearGradient";
  x1: number; y1: number;
  x2: number; y2: number;
  units?: "objectBoundingBox" | "userSpaceOnUse"; // default objectBoundingBox
  stops: GradientStop[];
}

interface RadialGradient {
  type: "radialGradient";
  cx: number; cy: number; r: number;
  fx?: number; fy?: number;
  units?: "objectBoundingBox" | "userSpaceOnUse"; // default objectBoundingBox
  stops: GradientStop[];
}
```

Use the portable subset of [CSS Color 4](https://www.w3.org/TR/css-color-4/) for `Color`: named colors, `transparent`, hex, `rgb()`/`rgba()`, and `hsl()`/`hsla()`. Renderers must normalize colors; unsupported CSS-wide keywords and environment-dependent values are invalid.

Gradient terms and coordinate modes intentionally follow [SVG gradients](https://www.w3.org/TR/SVG2/pservers.html). DrawnUI exposes equivalent linear/radial gradient concepts through `SkiaGradient`, though adapters will translate its `StartColor`/`EndColor` representation into ordered stops.

### 5.3 Stroke, shadow, transform, border

```ts
interface Stroke {
  paint: Paint;
  width?: number; // default 1
  lineCap?: "butt" | "round" | "square"; // default butt
  lineJoin?: "miter" | "round" | "bevel"; // default miter
  miterLimit?: number; // default 4
  dashArray?: number[];
  dashOffset?: number; // default 0
}

interface Shadow {
  color: Color;
  offsetX?: number; // default 0
  offsetY?: number; // default 0
  blur?: number;    // >= 0, default 0
}

type Transform =
  | { type: "translate"; x: number; y: number }
  | { type: "scale"; x: number; y?: number }
  | { type: "rotate"; degrees: number; cx?: number; cy?: number }
  | { type: "skewX"; degrees: number }
  | { type: "skewY"; degrees: number }
  | { type: "matrix"; a: number; b: number; c: number; d: number; e: number; f: number };

interface Border {
  paint: Paint;
  width?: number; // default 1
}
```

Stroke terminology follows [SVG painting](https://www.w3.org/TR/SVG2/painting.html). Transform names and the six-value affine matrix follow [SVG/CSS Transforms](https://www.w3.org/TR/css-transforms-1/). Transforms are applied in array order. DrawnUI's `StrokeColor`, `StrokeWidth`, `StrokeCap`, dash path, gradients, and multiple shadows map directly to these portable objects.

### 5.4 Image source and accessibility

```ts
type ImageSource =
  | { kind: "uri"; uri: string }
  | { kind: "resource"; name: string };

interface Accessibility {
  label?: string;
  description?: string;
  hidden?: boolean; // default false; true means decorative/ignored by accessibility APIs
}
```

The host—not the generated document—controls permitted URI schemes, domains, authentication, fetch limits, caching, and media decoding. Inline base64 data is intentionally omitted to reduce token volume and denial-of-service risk. `resource` names refer to a trusted host registry.

Accessibility names follow platform accessibility APIs and the [Accessible Name and Description Computation](https://www.w3.org/TR/accname-1.2/). Renderers derive semantic roles from component types; the MVP does not allow arbitrary ARIA roles.

## 6. Level 1 — drawing catalog (`nx.draw@0.1.0`)

The drawing catalog is an SVG-shaped, retained-mode scene tree. Coordinates are logical units within the nearest `Drawing` view box. It is not an imperative Canvas API, although it maps readily to [Canvas 2D](https://html.spec.whatwg.org/multipage/canvas.html#the-canvas-element) or Skia.

### 6.1 Shared drawing properties

Every drawing node accepts `DrawCommonProps`:

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `opacity` | `number` (0..1) | `1` | Node and descendant opacity. |
| `transform` | `Transform[]` | `[]` | Ordered local transforms. |
| `clipPath` | `string` | none | SVG path-data geometry used as a local clip. |
| `accessibility` | `Accessibility` | derived | Optional accessible name/description or decorative status. |

`clipPath` is purposefully a small MVP subset of SVG clipping. Referenced clip trees, masks, and filter regions are deferred.

Every filled/stroked geometry accepts `ShapeProps` in addition to its own properties:

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `fill` | `Paint \| "none"` | component-specific | Interior paint. |
| `stroke` | `Stroke` | none | Outline paint and stroke geometry. |
| `fillRule` | `"nonzero" \| "evenodd"` | `"nonzero"` | Interior rule; matches [SVG fill-rule](https://www.w3.org/TR/SVG2/painting.html#FillRuleProperty). |
| `shadows` | `Shadow[]` | `[]` | Ordered drop shadows. |

The default fill is `"none"` for `Line` and `Polyline`, and black for closed shapes and `Path`, matching SVG's familiar behavior while avoiding a meaningless line fill.

### 6.2 `nx.draw.Drawing`

Root drawing surface and bridge into Level 2 UI. Accepts ordered drawing children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Controls the surface's size and placement when embedded in UI. |
| `viewBox` | `ViewBox` | yes | Internal coordinate rectangle; width and height must be positive. |
| `preserveAspectRatio` | `"meet" \| "slice" \| "none"` | no | `meet`; fit, crop, or non-uniformly stretch the view box. |
| `contentAlignment` | `{ x: "start"\|"center"\|"end"; y: "start"\|"center"\|"end" }` | no | Center/center when aspect ratio is preserved. |

The common `background` paints the surface before its drawing children, and common `accessibility` describes the surface. Rationale: `viewBox` and aspect preservation come from [SVG coordinate systems](https://www.w3.org/TR/SVG2/coords.html). Unlike DrawnUI's canvas control, the portable object does not expose acceleration, cache, pixel density, or invalidation settings.

### 6.3 `nx.draw.Group`

Groups ordered drawing children without drawing its own geometry.

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps` | see §6.1 | no | Shared opacity, transform, clip, and accessibility. |

Rationale: equivalent to SVG [`g`](https://www.w3.org/TR/SVG2/struct.html#Groups) and a Skia save/restore scope. Grouping avoids repeating transforms or opacity.

### 6.4 `nx.draw.Rect`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared drawing and paint properties. |
| `x` | `number` | no | `0`; left coordinate. |
| `y` | `number` | no | `0`; top coordinate. |
| `width` | `number` | yes | Non-negative width. |
| `height` | `number` | yes | Non-negative height. |
| `rx` | `number` | no | `0`; horizontal corner radius. |
| `ry` | `number` | no | `rx`; vertical corner radius. |

Rationale: names and radius behavior follow SVG [`rect`](https://www.w3.org/TR/SVG2/shapes.html#RectElement). DrawnUI `Rectangle` + `CornerRadius` maps here; adapters may approximate unequal radii if necessary.

### 6.5 `nx.draw.Circle`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `cx` | `number` | yes | Center x. |
| `cy` | `number` | yes | Center y. |
| `r` | `number` | yes | Non-negative radius. |

Rationale: matches SVG [`circle`](https://www.w3.org/TR/SVG2/shapes.html#CircleElement) and DrawnUI's explicit `Circle` shape. Keeping it separate from `Ellipse` is more legible for models and people.

### 6.6 `nx.draw.Ellipse`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `cx` | `number` | yes | Center x. |
| `cy` | `number` | yes | Center y. |
| `rx` | `number` | yes | Non-negative horizontal radius. |
| `ry` | `number` | yes | Non-negative vertical radius. |

Rationale: matches SVG [`ellipse`](https://www.w3.org/TR/SVG2/shapes.html#EllipseElement) and DrawnUI `Ellipse`.

### 6.7 `nx.draw.Line`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties; `fill` has no effect. |
| `x1` | `number` | yes | Start x. |
| `y1` | `number` | yes | Start y. |
| `x2` | `number` | yes | End x. |
| `y2` | `number` | yes | End y. |

Rationale: follows SVG [`line`](https://www.w3.org/TR/SVG2/shapes.html#LineElement). DrawnUI models lines as point collections; NX adds the common two-point primitive and retains `Polyline` for multi-segment lines.

### 6.8 `nx.draw.Polyline`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Two or more ordered vertices; path remains open. |

### 6.9 `nx.draw.Polygon`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Three or more ordered vertices; closing segment is implicit. |

Rationale for both: follows SVG [`polyline` and `polygon`](https://www.w3.org/TR/SVG2/shapes.html#PolylineElement) and maps directly from DrawnUI `Points`. DrawnUI's non-standard `SmoothPoints` is deferred; smooth geometry should use an explicit `Path` in the MVP.

### 6.10 `nx.draw.Path`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `DrawCommonProps`, `ShapeProps` | see §6.1 | no | Shared properties. |
| `data` | `string` | yes | SVG path-data string. |

`data` uses the standard SVG command grammar (`M/L/H/V/C/S/Q/T/A/Z`, absolute or relative) from [SVG Paths](https://www.w3.org/TR/SVG2/paths.html). This is already the format of DrawnUI `SkiaShape.PathData`, so it is the strongest direct compatibility point. Renderers must impose command-count, numeric-range, and rendered-bounds limits.

### 6.11 `nx.draw.Text`

Single-line or explicitly line-broken text positioned in drawing coordinates. It does not perform UI box layout.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `DrawCommonProps` | see §6.1 | no | Shared properties. |
| `x` | `number` | yes | Text anchor x. |
| `y` | `number` | yes | Baseline y. |
| `text` | `string` | yes | Literal text; `\n` requests explicit line breaks. |
| `fontFamily` | `string` | no | Host default. |
| `fontSize` | `number` | no | `16`. |
| `fontWeight` | `number` (1..1000) | no | `400`. |
| `fontStyle` | `"normal" \| "italic"` | no | `"normal"`. |
| `textAnchor` | `"start" \| "middle" \| "end"` | no | `"start"`. |
| `dominantBaseline` | `"auto" \| "middle" \| "hanging"` | no | `"auto"`. |
| `letterSpacing` | `number` | no | `0`. |
| `fill` | `Paint \| "none"` | no | Black. |
| `stroke` | `Stroke` | no | None. |
| `shadows` | `Shadow[]` | no | `[]`. |

Rationale: positioning and anchor terms follow [SVG text](https://www.w3.org/TR/SVG2/text.html). Numeric weight follows modern CSS/OpenType and is more portable than `Bold` flags. DrawnUI `SkiaLabel` supplies the implementation precedent for family, size, weight, spacing, fill/stroke, and shadows. Rich spans, text-on-path, shaping controls, and automatic fit are deferred.

### 6.12 `nx.draw.Image`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `DrawCommonProps` | see §6.1 | no | Shared properties. |
| `x` | `number` | no | `0`. |
| `y` | `number` | no | `0`. |
| `width` | `number` | yes | Non-negative destination width. |
| `height` | `number` | yes | Non-negative destination height. |
| `source` | `ImageSource` | yes | Trusted resource or host-approved URI. |
| `fit` | `"contain" \| "cover" \| "fill" \| "none"` | no | `"contain"`. |

Rationale: destination rectangle follows SVG [`image`](https://www.w3.org/TR/SVG2/embedded.html#ImageElement); fit names follow familiar CSS `object-fit`. DrawnUI `SkiaImage.Source` and `Aspect` map naturally. Filters, tint, sprite sheets, loading strategy, resampling quality, and cache policy stay renderer-specific or post-MVP.

## 7. Level 2 — generic output/layout catalog (`nx.ui@0.1.0`)

Level 2 is deliberately read-only. It provides measured layout and ordinary content; custom visual composition drops into `nx.draw.Drawing`.

### 7.1 `UiCommonProps`

Every Level 2 component, plus `nx.draw.Drawing`, accepts these optional flattened properties:

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `width`, `height` | `Length` | `"auto"` | Preferred dimensions. |
| `minWidth`, `minHeight` | `number` | `0` | Minimum dimensions. |
| `maxWidth`, `maxHeight` | `number` | unbounded | Maximum dimensions. |
| `margin` | `Insets` | `0` | Space outside the component. |
| `padding` | `Insets` | `0` | Space between border and content. |
| `alignSelf` | `Alignment \| "auto"` | `"auto"` | Override cross-axis/grid alignment assigned by parent. |
| `justifySelf` | `Alignment \| "auto"` | `"auto"` | Override grid/stack alignment on the other axis. |
| `grow` | `number` | `0` | Share of positive free space in a `Row` or `Column`. |
| `shrink` | `number` | `1` | Relative shrink factor in a `Row` or `Column`. |
| `gridColumn`, `gridRow` | `integer >= 0` | auto-placement | Zero-based grid position. Set both or neither. |
| `columnSpan`, `rowSpan` | `integer >= 1` | `1` | Grid cell span. |
| `background` | `Paint \| "none"` | `"none"` | Background inside the border. |
| `border` | `Border` | none | Uniform border. |
| `cornerRadius` | `CornerRadii` | `0` | Box corner radii. |
| `shadows` | `Shadow[]` | `[]` | Ordered box shadows. |
| `clip` | `boolean` | `false` | Clip descendants to the padding box/corner radii. |
| `opacity` | `number` (0..1) | `1` | Component subtree opacity. |
| `accessibility` | `Accessibility` | derived | Accessible name/description/decorative status. |

These are constraints and portable appearance, not a CSS passthrough. Names balance CSS flex/grid familiarity with DrawnUI/MAUI's width request, margin/padding, alignment, background, clipping, and grid row/column concepts. Layout engines must document deterministic treatment of over-constrained values.

### 7.2 `nx.ui.Row`

Horizontal flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `number` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `"start"` | Distribution along the horizontal main axis. |
| `align` | `Alignment` | `"stretch"` | Alignment on the vertical cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional rows. |

### 7.3 `nx.ui.Column`

Vertical flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `number` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `"start"` | Distribution along the vertical main axis. |
| `align` | `Alignment` | `"stretch"` | Alignment on the horizontal cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional columns. |

Rationale for Row/Column: A2UI uses explicit `Row`/`Column` with `justify`, `align`, and weight-like sizing; json-render examples use `Row`; CSS Flexbox supplies the behavioral model. They map to DrawnUI `SkiaLayout` Row/Column and `Spacing`, while `grow` replaces a DrawnUI-specific sizing idiom.

### 7.4 `nx.ui.Grid`

Two-dimensional container; accepts zero or more UI children.

```ts
type TrackSize = number | "auto" | { fr: number };
```

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `columns` | `TrackSize[]` | yes | Fixed logical units, intrinsic `auto`, or fractional remaining space. At least one. |
| `rows` | `TrackSize[]` | no | Explicit rows; omitted rows are created as `auto`. |
| `columnGap` | `number` | no | `0`. |
| `rowGap` | `number` | no | `0`. |
| `justifyItems` | `Alignment` | no | `"stretch"`; horizontal alignment in cells. |
| `alignItems` | `Alignment` | no | `"stretch"`; vertical alignment in cells. |

Children use `gridColumn`, `gridRow`, `columnSpan`, and `rowSpan` from `UiCommonProps`. Auto-placement is row-major. The model borrows the useful subset of [CSS Grid](https://www.w3.org/TR/css-grid-2/) and directly maps DrawnUI's grid definitions and attached row/column properties. `minmax`, named lines, dense placement, and subgrid are deferred.

### 7.5 `nx.ui.Stack`

Overlay container; accepts zero or more UI children. Later children paint above earlier children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `horizontalAlignment` | `Alignment` | `"stretch"` | Default horizontal child alignment. |
| `verticalAlignment` | `Alignment` | `"stretch"` | Default vertical child alignment. |

Rationale: “Stack” is ambiguous across ecosystems—SwiftUI/ZStack and Compose Box use it for overlay, while some .NET APIs use stack for linear flow. NX reserves `Row` and `Column` for linear flow and defines `Stack` strictly as z-order overlay, matching json-render's common catalog wording and DrawnUI's `SkiaLayer`/absolute one-cell layering use case. Free x/y positioning belongs in `nx.draw.Drawing`.

### 7.6 `nx.ui.Scroll`

Viewport for exactly one UI child.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `axis` | `Axis \| "both"` | `"vertical"` | Allowed scroll direction. |
| `scrollbar` | `"auto" \| "visible" \| "hidden"` | `"auto"` | Host scrollbar presentation hint. |

Rationale: maps to DrawnUI [`SkiaScroll`](https://drawnui.net/articles/controls/scroll.html) `Orientation` + `Content`. Physics, offsets, zoom, snapping, sticky headers, refresh, virtualization, and load-more commands are intentionally outside a portable read-only MVP.

### 7.7 `nx.ui.Box`

Neutral single-child container. It has no component-specific properties beyond `UiCommonProps`.

Use it for padding, background, border, rounded clipping, shadow, or sizing around one child. It is the portable evolution of DrawnUI `ContentLayout` and the content-hosting aspect of `SkiaShape`, without conflating UI layout with geometric primitives.

### 7.8 `nx.ui.Card`

Themed single-child surface.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | theme | Explicit values override theme defaults. |
| `variant` | `"outlined" \| "elevated" \| "filled"` | `"elevated"` | Host-theme card profile. |

`Card` is retained despite overlap with `Box` because it is a highly familiar GenUI primitive in A2UI and json-render examples and lets a model request design-system intent without inventing border/shadow values. A renderer without card theming must fall back to a documented `Box` profile.

### 7.9 `nx.ui.Divider`

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared layout properties; background/border are ignored. |
| `axis` | `Axis` | `"horizontal"` | Line direction. |
| `color` | `Color` | theme | Divider color. |
| `thickness` | `number` | `1` | Line thickness. |

Rationale: directly matches A2UI's horizontal/vertical `Divider`. Length is controlled by common width/height and parent alignment rather than another bespoke property.

### 7.10 `nx.ui.Text`

Measured, wrapping output text; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `text` | `string` | yes | Literal displayed content. |
| `format` | `"plain" \| "markdown"` | no | `"plain"`; markdown uses a host-defined safe common subset. |
| `variant` | `"h1"\|"h2"\|"h3"\|"title"\|"body"\|"caption"\|"code"` | no | `"body"`; theme typography role. |
| `color` | `Color` | no | Theme value. |
| `fontFamily` | `string` | no | Variant/theme value. |
| `fontSize` | `number` | no | Variant/theme value. |
| `fontWeight` | `number` (1..1000) | no | Variant/theme value. |
| `fontStyle` | `"normal" \| "italic"` | no | `"normal"`. |
| `textAlign` | `"start"\|"center"\|"end"\|"justify"` | no | `"start"`. |
| `lineHeight` | `number` | no | Unitless multiplier; theme default. |
| `letterSpacing` | `number` | no | `0`. |
| `maxLines` | `integer >= 1` | no | Unlimited. |
| `overflow` | `"clip" \| "ellipsis"` | no | `"clip"`; applies when constrained/maxLines is reached. |

Rationale: A2UI `Text` combines simple Markdown with typography variants; DrawnUI `SkiaLabel` supplies detailed text properties. NX keeps the useful intersection and makes semantic `variant` preferable to low-level overrides. Hosts must sanitize Markdown, disallow raw HTML by default, and apply a link policy. Rich spans are deferred.

### 7.11 `nx.ui.Image`

Measured raster/vector image output; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `source` | `ImageSource` | yes | Trusted resource or host-approved URI. |
| `alt` | `string` | yes | Accessible alternative; use empty string only when decorative. |
| `fit` | `"contain" \| "cover" \| "fill" \| "none"` | no | `"contain"`. |
| `aspectRatio` | `number` | no | Intrinsic ratio when dimensions do not establish one. |

Rationale: aligns with A2UI `Image`, CSS object-fit vocabulary, and DrawnUI `SkiaImage.Source`/`Aspect`. SVG files may be supplied as trusted image resources; editable/composable SVG content should be represented with `nx.draw` primitives.

### 7.12 `nx.ui.Icon`

Host-provided symbolic icon; accepts no children.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Shared constraints and appearance. |
| `name` | `string` | yes | Icon name from the negotiated host icon catalog. |
| `set` | `string` | no | Host/default icon set if omitted. |
| `size` | `number` | no | Theme default, normally 24. |
| `color` | `Color` | no | Current theme/content color. |

Rationale: A2UI intentionally uses system-provided icons from a predefined list. NX follows that safe pattern instead of letting a model supply arbitrary SVG through `Icon`. Use common `accessibility.label` for a meaningful icon or `accessibility.hidden: true` for a decorative one. A catalog capability exchange must enumerate supported names; unknown icons use a documented fallback glyph.

## 8. Minimal example

```json
{
  "schemaVersion": "0.1",
  "catalogs": [
    { "id": "nx.ui", "version": "0.1.0" },
    { "id": "nx.draw", "version": "0.1.0" }
  ],
  "root": "card",
  "elements": {
    "card": {
      "type": "nx.ui.Card",
      "props": { "padding": 20, "width": 360 },
      "children": ["content"]
    },
    "content": {
      "type": "nx.ui.Column",
      "props": { "gap": 12 },
      "children": ["title", "graphic", "caption"]
    },
    "title": {
      "type": "nx.ui.Text",
      "props": { "text": "NX Drawn UI", "variant": "h2" }
    },
    "graphic": {
      "type": "nx.draw.Drawing",
      "props": {
        "height": 140,
        "viewBox": { "x": 0, "y": 0, "width": 320, "height": 140 }
      },
      "children": ["background", "curve", "dot"]
    },
    "background": {
      "type": "nx.draw.Rect",
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
      "type": "nx.draw.Path",
      "props": {
        "data": "M20 105 C90 15 210 125 300 35",
        "fill": "none",
        "stroke": { "paint": "white", "width": 5, "lineCap": "round" }
      }
    },
    "dot": {
      "type": "nx.draw.Circle",
      "props": { "cx": 300, "cy": 35, "r": 7, "fill": "white" }
    },
    "caption": {
      "type": "nx.ui.Text",
      "props": {
        "text": "Portable layout above; portable drawing below.",
        "variant": "caption",
        "color": "#555"
      }
    }
  }
}
```

## 9. DrawnUI evolution and adapter mapping

| DrawnUI today | NX MVP | Evolution rationale |
|---|---|---|
| `Canvas` + `SkiaControl` base capabilities | `NxUiDocument`, `Element`, `UiCommonProps`, `nx.draw.Drawing` | Separate portable document semantics from renderer lifecycle, invalidation, cache, and GPU configuration. |
| `SkiaLayout Type="Row"` / `SkiaRow` | `nx.ui.Row` | Explicit catalog type; `Spacing` becomes `gap`; MAUI alignment maps to `align`/`justify`/`alignSelf`. |
| `SkiaLayout Type="Column"` / `SkiaStack` alias | `nx.ui.Column` | Avoid cross-platform ambiguity around the word “Stack.” |
| `SkiaLayout Type="Grid"` / `SkiaGrid` | `nx.ui.Grid` | Preserve rows, columns, gaps, placement, and spans in a smaller typed subset. |
| `SkiaLayout Type="Absolute"` / `SkiaLayer` | `nx.ui.Stack` for overlay; `nx.draw.Drawing` for coordinates | Separate ordinary layering from coordinate-based graphics. |
| `SkiaLayout Type="Wrap"` / `SkiaWrap` | `Row` or `Column` with `wrap: true` | Avoid another component when flex-like wrapping is enough. |
| `ContentLayout` | `nx.ui.Box` | Neutral single-child decoration/layout primitive. |
| `SkiaShape Type="Rectangle\|Circle\|Ellipse\|Line\|Polygon\|Path"` | corresponding `nx.draw.*` types | Smaller schemas, SVG-standard names, direct validation. |
| `SkiaShape` hosting content/clipping | `nx.ui.Box`/`Card` for UI; `Drawing` + `clipPath` for graphics | Preserve the capability while separating UI content layout from shape geometry. |
| `BackgroundColor` / `BackgroundGradient` | `fill` for shapes; `background` for UI | SVG uses fill for geometry; UI ecosystems use background for boxes. This terminology distinction is intentional. |
| `StrokeColor`, `StrokeGradient`, `StrokeWidth`, `StrokeCap`, `StrokePath` | `Stroke { paint, width, lineCap, dashArray, ... }` | Groups related stroke state and aligns with SVG. |
| `SkiaLabel` | `nx.ui.Text` or `nx.draw.Text` | Explicitly distinguish measured UI text from positioned drawing text. |
| `SkiaImage` / `SkiaSvg` | `nx.ui.Image`, `nx.draw.Image`, `nx.ui.Icon`, or decomposed `nx.draw` geometry | Choose by intent: ordinary content, positioned image, host symbol, or editable vector scene. |
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

- arcs as a convenience primitive; rounded/smoothed polylines; compound/reusable geometry definitions
- rich text spans, text-on-path, advanced shaping controls, font resources
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
- localization, bidirectional text policy, responsive breakpoints, environment values

## 12. Decisions to validate in a prototype

1. **Flat canonical form:** measure generation error rate and streaming ergonomics against nested authoring syntax.
2. **Component count:** confirm `Box` and `Card` both earn their place; `Card` is the likelier convenience component to retain because GenUI models recognize it well.
3. **Stack terminology:** test whether model prompts reliably understand “Stack = overlay” when Row/Column handle linear layout.
4. **Gradient coordinates:** verify that `objectBoundingBox` defaults produce the most intuitive model output across Skia and SVG.
5. **Text parity:** quantify layout differences between Skia/DrawnUI and web/native renderers, especially `lineHeight`, weight, and ellipsis.
6. **Path safety:** establish practical complexity and bounds limits without rejecting normal illustrations.
7. **Catalog schemas:** publish JSON Schema for `nx.ui@0.1.0` and `nx.draw@0.1.0`, then run constrained-generation evals.
8. **DrawnUI adapter:** implement the example document through a thin NX-to-DrawnUI mapping before adding any new component.

## 13. Recommended MVP acceptance test

The MVP is credible when one unchanged validated document can render through both:

- a DrawnUI/Skia renderer; and
- a web renderer using normal layout plus SVG or Canvas for `nx.draw`.

The test corpus should include a text/image card, grid dashboard shell, layered composition, clipped image, gradient/shadow illustration, multi-segment path, and accessible decorative/informative images. It should also include intentionally invalid documents for every validation rule.

## 14. Primary references

### Generative UI

- [A2UI v0.9 specification and basic catalog](https://a2ui.org/specification/v0.9-a2ui/)
- [json-render specification anatomy](https://json-render.dev/docs/specs) and [core TypeScript types](https://github.com/vercel-labs/json-render/blob/main/packages/core/src/types.ts)
- [Vercel AI SDK: Generative User Interfaces](https://ai-sdk.dev/docs/ai-sdk-ui/generative-user-interfaces)
- [CopilotKit: Generative UI overview](https://docs.copilotkit.ai/concepts/generative-ui-overview)
- [CopilotKit: A2UI](https://docs.copilotkit.ai/a2a/generative-ui/a2ui)
- [AG-UI: Agent–User Interaction protocol](https://docs.ag-ui.com/)
- [MCP Apps overview](https://modelcontextprotocol.io/extensions/apps/overview) and [MCP Apps specification site](https://apps.extensions.modelcontextprotocol.io/)
- [Thesys C1 overview](https://docs.thesys.dev/guides/what-is-thesys-c1) and [`C1Component`](https://docs.thesys.dev/react-reference/c1-component)

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

---

### Short form of the proposal

Use a cataloged flat document, borrow structure from A2UI/json-render, borrow graphics vocabulary and data formats from SVG, preserve DrawnUI's composability, and keep renderer mechanics out of the generated model. Ship read-only layout/content plus a compact vector scene graph first; add state, interaction, semantic components, and richer effects as separately versioned layers.
