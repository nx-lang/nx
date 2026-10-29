# NX UI MVP Object Model

**Proposal status:** Draft for discussion  
**Working title superseded:** “NX Drawn UI” — see “Names and terminology” in §1  
**Research checked:** October 27, 2026 (primary sources; release status is a snapshot)<br>
**Scope:** Level 1 graphics and Level 2 generic, read-only UI  
**Explicitly excluded from this MVP:** input controls, actions/events, application state/data binding, animation, and Level 3 semantic/data UI  
**Before compatibility stabilization:** [future design decisions](NX-Drawn-UI-MVP-Object-Model-Future.md)

**Consolidated NX source:** [drawnui-proposal/](drawnui-proposal/) — the whole proposed model as NX and nothing else, for reading without the prose around it

## Executive recommendation

NX should define a small, declarative, renderer-neutral UI document that an LLM can generate and a trusted client can validate and render. The document should use:

- a flat, ID-addressed element map, inspired by [A2UI](https://a2ui.org/specification/v0.9.1-a2ui/) and [json-render](https://json-render.dev/docs/specs);
- namespaced, versioned component catalogs (`nx.ui` for layout-positioned UI, `nx.graphics` for coordinate-positioned graphics);
- familiar UI names (`Row`, `Column`, `ZStack`, `Grid`, `Scroll`, `Text`, `Image`, `Icon`);
- SVG vocabulary and data formats for portable drawing (`Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, SVG path data, fill/stroke, transforms);
- a clean evolution path from DrawnUI for .NET rather than a wire-level copy of its MAUI/Skia API.

**Delivery priority:** graphics first. A standalone `nx.graphics.Drawing` must work before the Level 2 UI catalog or A2UI conversion is implemented; §9.6 gives the initial change sequence.

**Rendering recommendation:** support both direct rendering and an optional NX-defined display list through a small, pluggable host drawing interface. Ship the Skia direct path first and prove recording/replay against the same interface in the MVP prototype. Direct rendering must not require constructing an NX display list. The retained document, drawing interface, and recorded output are three distinct contracts; §9.1–§9.5 define their boundaries.

The initial catalog proposed here contains:

- **Level 1 — graphics:** `Drawing`, `Group`, `Rect`, `Circle`, `Ellipse`, `Line`, `Polyline`, `Polygon`, `Path`, `Text`, `Image`.
- **Level 2 — output/layout:** `Row`, `Column`, `ZStack`, `Grid`, `Scroll`, `List`, `Box`, `Card`, `Divider`, `Text`, `Image`, `Icon`. (`nx.graphics.Drawing` is also embeddable as Level 2 content.)

The two namespaces intentionally allow both `nx.graphics.Text` and `nx.ui.Text`. The former is positioned glyph content in drawing coordinates; the latter is measured, wrapped UI text. That distinction avoids a deceptively complex “one Text does everything” object.

## 1. Goals and non-goals

### MVP goals

1. **Safe for model generation.** A model selects only cataloged components and literal, schema-validated properties; it does not emit executable code.
2. **Portable.** The same document can target DrawnUI/Skia, DOM/SVG/Canvas, and future native renderers.
3. **Minimal but useful.** It can express ordinary read-only UI, diagrams, illustrations, custom graphics, and hybrid layouts.
4. **Familiar.** Prefer established SVG, CSS, MAUI, and GenUI terms where their meanings transfer cleanly.
5. **Streamable and patchable later.** Stable IDs and a flat element map permit incremental additions and replacements without making streaming part of the MVP protocol.
6. **An evolution of DrawnUI.** Preserve its compositional strengths while removing renderer-specific implementation details from the portable model.
7. **A2UI semantic coverage over time.** Target eventual Basic component/runtime coverage and selected richer catalogs, with versioned import/export profiles and explicit limits on NX → A2UI conversion (§4.6).

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
| `Row`, `Column`, `ZStack` | component | Linear horizontal/vertical containers and a z-order overlay container; see §7. |
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

### Research update — October 27, 2026

The sources below reinforce the catalog model while sharpening the rendering recommendation. These are observed capabilities, not claims that the projects implement NX semantics or that all their backends have feature parity.

- **A2UI:** the [official site](https://a2ui.org/) still identifies v0.9.1 as current and v1.0 as candidate. Separately, AndroidX published its first [A2UI `1.0.0-alpha01` libraries on September 23, 2026](https://developer.android.com/jetpack/androidx/releases/a2ui). Its latest listed libraries are `1.0.0-alpha02`, released October 21, 2026. Android library versioning must not be confused with protocol stability. This strengthens the case for adapters and exact catalog versions rather than making NX depend on a candidate protocol.
- **json-render:** its [current spec documentation](https://json-render.dev/docs/specs) explicitly describes the framework as schema-agnostic; `root` + `elements` is the React schema's normalized shape, rather than a universal requirement. Its [changelog](https://json-render.dev/docs/changelog) documents image output, multiple framework renderers, and MCP Apps integration. Adopt the separation of catalog, schema, and renderer; do not assume a shared envelope guarantees portable component semantics.
- **AnyRender:** the [project](https://github.com/dioxuslabs/anyrender) provides a Rust 2D drawing abstraction with multiple backends, including Skia and Vello implementations, plus an `anyrender_serialize` crate listed in its [compatibility table](https://raw.githubusercontent.com/DioxusLabs/anyrender/main/README.md). It is a particularly close precedent for an interface with both direct and recorded implementations. Study its operation boundaries and resource handling; do not introduce Rust or its API as an NX dependency.
- **Linebender:** its [April 2026 update](https://linebender.org/blog/tmil-25/) describes Vello Hybrid at roughly beta quality and explains the difficulty of building one unified low-level renderer API. It points to AnyRender and `imaging` as abstractions with different priorities. The [current AnyRender compatibility table](https://raw.githubusercontent.com/DioxusLabs/anyrender/main/README.md) records the upstream rename from `vello_hybrid` to `vello_gpu` in its 0.14 line; the adapter still uses the hybrid name. Consult [Vello releases](https://github.com/linebender/vello/releases) for backend capabilities rather than treating April’s maturity assessment as a current release guarantee. NX should keep its interface bounded by MVP semantics and validate it with a second backend before promising universal graphics portability.
- **Skia and Flutter:** [`SkPictureRecorder`](https://api.skia.org/classSkPictureRecorder.html) records through an `SkCanvas`; this is a useful example of a drawing API serving direct and recorded targets. Flutter's [Impeller FAQ](https://flutter.googlesource.com/mirrors/engine/+/HEAD/impeller/docs/faq.md) describes display-list dispatchers for different rendering packages and separates rasterization from text shaping. NX can adopt that separation without making Flutter's mandatory display-list route its own requirement. A Skia picture remains a backend cache, not the NX interchange format.
- **MAUI Graphics:** [`ICanvas` and `IDrawable`](https://learn.microsoft.com/en-us/dotnet/maui/user-interface/graphics/?view=net-maui-10.0) demonstrate a familiar .NET drawing boundary. Evaluate that API as an adapter target, especially for platform hosts; adopting it wholesale would couple NX's graphics contract to its operation and text semantics.

The design conclusion is a recommendation derived from these sources: retain semantic documents for generation and accessibility, expose backend-neutral drawing operations to trusted runtime code, and make recording an optional destination for those operations.

### Agent-generated graphics: authoring, rendering, and feedback

Most NX graphics are expected to be authored and revised by AI agents. Optimize for compact expressions of design intent, predictable edits, and fast visual feedback, while keeping the rendering contract small. A schema-valid drawing is not necessarily a good design.

Keep three layers distinct:

| Layer | Contract | Initial direction |
|---|---|---|
| Portable graphics | Literal, validated scene geometry and rendering semantics | Existing SVG-shaped primitives; direct backend dispatch. |
| NX authoring library | Ordinary reusable NX components/functions that expand into graphics | Regular polygons, stars, arrows, arc/ring sectors; shared dimensions and styles. |
| Agent/host tooling | Render, inspect, measure, diagnose, and revise | PNG previews plus element bounds, text metrics, revision IDs, and structured diagnostics. |

A convenience shape does not imply a new backend opcode. Keep `Polygon` as explicit vertices, and provide a separately named `RegularPolygon` helper; keep `Path` as an escape hatch. Initial helper contracts and angle/coordinate conventions are in [authoring-and-preview.md](drawnui-proposal/authoring-and-preview.md). Use NX composition and expressions to compute geometry before rendering; generated portable documents remain literal data and cannot inject drawing callbacks. Helpers must be evaluated against end-to-end authoring/revision tasks rather than accepted solely because their API looks elegant.

Prioritize named groups, local coordinates, reusable components, measured text, alignment helpers, and inspectable bounds over a large catalog of backend effects. Begin with explicit layout calculations and library helpers; a general constraint solver is not part of the initial graphics runtime. The first agent tool cycle is author → validate → render → inspect → revise. Return full-scene previews and ID-addressed geometry/ink/visible bounds, text metrics, unsupported-feature/resource errors, and clipping warnings. Overlap and empty space are often intentional; keep subjective design suggestions separate from objective validation failures.

The first change must include a bounded shaped-text profile and measurement from the same layout used to paint, even though multiline, rich text, and comprehensive fallback remain later work. A model cannot reliably place labels from guessed string lengths. Fonts and renderer configuration are supplied by the host; tools report their identities so the agent can reproduce the result.

Use repeatable tasks spanning icons, diagrams, geometric illustrations, and infographics. Measure renderability, successful requested revisions, output size, tool/model round trips, latency, and visual quality under a documented human/automated rubric. Compare raw primitives with helper-assisted authoring using the same model, assets, and budget. Model trials are development evaluations, not nondeterministic build tests. Rendering feedback research motivates this workflow but does not guarantee improvement for every agent or task: [IntroSVG](https://arxiv.org/abs/2603.09312) and [Render-in-the-Loop](https://yukinonooo.github.io/RenderInTheLoopProject/).

### Graphics precedents and chosen conventions

Use **SVG's established 2D imaging model for declarative authoring**, **Skia/Canvas 2D for familiar drawing operations**, and **AnyRender for modern backend abstraction**. These serve different contracts; none should be copied wholesale.

| Precedent | Adopt or study | Boundary |
|---|---|---|
| [SVG](https://www.w3.org/TR/SVG2/) | Geometry names, path data, view boxes, affine transforms, fill/stroke, clip rules, gradients, paint order. Best interchange baseline for illustrations, diagrams, and icons. | NX is a bounded typed scene model, not the SVG DOM, CSS cascade, scripting, or an unrestricted SVG importer. |
| [Canvas 2D](https://html.spec.whatwg.org/multipage/canvas.html#2dcontext) and [Skia](https://skia.org/docs/user/api/skcanvas_overview/) | Familiar fill/stroke/path/image operations, transforms and scoped state. Skia is the initial direct backend. | Do not inherit Canvas's entire mutable style/path state or expose Skia/GPU types in portable contracts. Backend defaults must not decide NX semantics. |
| [AnyRender `PaintScene`](https://docs.rs/anyrender/latest/anyrender/trait.PaintScene.html) | Explicit brushes, geometry and transforms, separate clipping/compositing scopes, positioned glyph output, and direct/recorded destinations. | A useful API precedent and possible Rust adapter dependency after evaluation, not NX's serialized object model or a guaranteed portability solution. |
| [PixiJS Graphics](https://pixijs.com/8.x/guides/components/scene-objects/graphics) | Separation of scene objects from reusable graphics contexts and resource ownership. Useful for later repeated graphics and browser integration. | Its retained graphics commands and game-oriented lifecycle need not become a mandatory NX display list. |

Modern practice here means explicit semantics, immutable/shareable geometry and paint descriptions, isolated compositing, shaped text, predictable resource lifetimes, and backend capability tests. GPU implementation details stay behind the adapter. Reusable parsed geometry belongs in host caches initially; it does not require public scene instancing in the first version. Keep SVG path strings as the initial authoring form, parse once per changed resource, and provide parametric helpers now and consider a typed path builder later for programmatic generation. Do not ask models to emit triangles or backend command buffers.

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

A normalized map gives every component a stable identity and makes partial generation, validation, replacement, and future JSON Patch streaming straightforward. A2UI uses ID-addressed component updates, and json-render’s React schema uses a flat element map. Recursive authoring syntax may be offered as sugar, but the canonical interchange form should be flat.

### Why explicit component types instead of DrawnUI-style `Type` enums

DrawnUI uses one `SkiaLayout` with `Type="Row|Column|Grid|Absolute|Wrap"` and one `SkiaShape` with `Type="Rectangle|Circle|Ellipse|Path|..."`. NX should use `nx.ui.Row`, `nx.ui.Grid`, `nx.graphics.Rect`, and so on because:

- each component receives a smaller, clearer property schema;
- invalid combinations become harder to generate;
- catalog capabilities are easier to negotiate;
- names align with SVG and current GenUI catalogs;
- renderers may still implement these as one shared internal class.

## 4. Canonical document model

Type notation in this document is [NX](../nx-grammar.md) source. Records use `type Name = { ... }` and discriminated unions use `type Name =` followed by their cases, with the leading `|` optional for two or more; a closed set of scalar choices is just a union whose cases carry no payload. Renderer-provided catalog components use `external component`, whose signature declares props and whose body is supplied by the host rather than by NX.

Three NX conventions carry meaning here. A `?` suffix makes a property nullable, which in this model means the property may be omitted — the **Default** column of each table says what the renderer does when it is. A `= value` clause gives a real default. And exactly one property per signature may be marked `content`, which is the property that receives element body content, so it is the property that the wire format spells as `children`.

Unless stated otherwise, numeric values must be finite JSON numbers and default units are logical/device-independent pixels. Appendix A lists both catalogs as complete NX source, and [drawnui-proposal/](drawnui-proposal/) carries the same declarations as an NX-only library; Appendix B records where NX's current syntax cannot express this model, and what the listing does instead.

### 4.1 `NxUiDocument`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| `format` | `string` | yes | Artifact discriminator. MVP value: `"nx-ui-json"`, following the `format` + `schemaVersion` convention [NX IR](nx-ir-format.md) used when this proposal was written. NX IR has since moved to a binary image with a magic header, so the convention now stands on its own here. |
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
| `type` | `ComponentType` | yes | Namespaced catalog type, such as `nx.ui.Column` or `nx.graphics.Path`. |
| `props` | component-specific object | yes | Literal properties validated against that component's catalog schema. Use `{}` when empty. |
| `children` | `ElementId[]` | conditional | Ordered child IDs. Allowed only by components documented as containers. |

There is no `Element` record to declare in NX source, because NX has no envelope: an element *is* an `external component` invocation, and the `type`/`props`/`children` envelope is what a compiler produces from it. `<ui.Column gap={12.0}> ... </ui.Column>` names the component, binds props by name, and binds body content to the component's `content` property. That is inherently the nested authoring form this section calls sugar, and NX cannot write the flat map at all — see Appendix B — so NX is the authoring surface over this model while the flat map remains its interchange and validation form.

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

### 4.6 A2UI interoperability and eventual coverage

**Long-term goal:** NX should be able to represent the semantics of the versioned A2UI Basic Catalog and its interaction/data runtime, then selected richer catalogs, while retaining NX-only graphics and layout. This is a semantic coverage goal, not a promise to reproduce every arbitrary third-party catalog or to embed untrusted HTML in the core model.

The conversion directions impose different requirements. A semantic superset makes **A2UI → NX** import feasible. **NX → A2UI** export is lossless only for a declared target-compatible subset, or when the receiving host implements a negotiated NX-specific A2UI catalog. A host that understands only Basic cannot render arbitrary NX vector graphics, overlapping layers, or grid constraints just because NX understands Basic. Matching component names helps authoring; it does not establish matching behavior.

Define an adapter profile by A2UI protocol version, exact catalog identity/schema revision, supported functions/actions, theme conventions, and host resource/accessibility policy. The initial target is Basic v0.9.1, including Android's Compose implementation; test the v1.0 candidate independently. Keep catalog IDs as published: the v0.9.1 Basic schema currently retains the v0.9 catalog ID. Pin the schema revision in adapter fixtures rather than inferring its contents solely from the URL.

A conversion returns the output plus element-ID-linked diagnostics. Classify each construct as exact, equivalent lowering under stated conditions, explicitly lossy fallback, or unsupported. Strict export rejects the last two. Never silently flatten headings, remove input actions, replace native content with screenshots, or drop unsupported geometry. Raster fallback may be an opt-in preview, with declared loss of semantics, scalability, and interaction. A display list is renderer output and cannot substitute for semantic A2UI components.

#### Coverage plan for Basic

The [v0.9.1 Basic schema](https://raw.githubusercontent.com/a2ui-project/a2ui/main/specification/v0_9_1/catalogs/basic/catalog.json) supplies the baseline below; the [v1.0 candidate schema](https://raw.githubusercontent.com/a2ui-project/a2ui/main/catalogs/basic/v1/catalog.json) must be tracked separately. These are planned mappings, not implemented converter support.

| A2UI components | NX destination | Coverage milestone |
|---|---|---|
| `Row`, `Column`, `Card`, `Divider` | Same-named `nx.ui` components | Read-only subset; reconcile sizing, theme spacing, and decoration. |
| `Text`, `Image`, `Icon` | Same-named `nx.ui` components | Read-only subset; reconcile typography, resource policy, and alternatives. |
| `List` | Static `nx.ui.List`; later collection templates | Explicit children now; dynamic lists after runtime work. |
| `Video`, `AudioPlayer` | Future media catalog | Playback, controls, resource loading, accessibility, and lifecycle. |
| `Tabs`, `Modal` | Future interaction/navigation components | Local state, dismissal/focus, trigger/content roles, named child slots. |
| `Button`, `TextField`, `CheckBox`, `ChoicePicker`, `Slider`, `DateTimeInput` | Future `nx.input` catalog | Typed values, two-way binding, checks, actions, and accessibility. |

The read-only adapter must explicitly resolve these differences: A2UI `weight` versus NX growth/shrink rules; A2UI main-axis `stretch`; host-default spacing versus NX `gap`/insets; `Image.fit` default `fill` versus NX `contain`; optional image descriptions versus required NX `alt`; A2UI icon path data versus NX graphics; and version-specific text variants. Import must request or apply a documented policy when required accessibility information is absent, never assume an unlabeled image is decorative. Converting an inline icon path to a `Drawing` preserves it on import; exporting an NX Drawing still requires target capability support. Primitive decomposition is conditional on equivalent layout and accessibility semantics, not automatically exact.

#### Catalog scope and runtime roadmap

A2UI's [catalog architecture](https://a2ui.org/concepts/catalogs/) permits custom catalogs; Basic is a starting point. Android's [Material 3 implementation](https://developer.android.com/develop/ui/compose/agentic) renders Basic using Material components. The separate [Material catalog documented for Gemini Enterprise](https://docs.cloud.google.com/gemini/enterprise/docs/a2ui-agents/a2ui-component-gallery-reference) is richer and host-specific. Its tables, badges, progress, disclosure, and richer selection controls motivate later semantic NX components, not a duplicate `Material*` core namespace. Prioritize `Badge`, `Progress`, and `DataTable` for the next read-only catalog. Charts, maps, MCP Apps, and enterprise-specific components need separate profiles; they are not implied by Basic coverage.

After the static subset, add runtime capabilities deliberately: typed local data, literal-or-bound values, JSON Pointer addressing, reactive updates, collection scope/templates and stable instance IDs, catalog-declared functions, validation checks, and typed action dispatch with context resolved at interaction time. Map A2UI's client-side function semantics explicitly rather than evaluating arbitrary generated code. Support named slots (`trigger`, `content`, tab panels) without forcing all references into ordered `children`; future templates and reusable definitions will also require revisiting the MVP's strict-tree/reachability rules. These capabilities require later document/catalog versions and remain excluded from the read-only MVP.

For NX → A2UI, pure NX expressions can become literals when exporting a snapshot; continuing NX behavior requires supported bindings/functions or a retained NX runtime behind the adapter. Sending an evaluated snapshot must not be described as preserving reactive behavior. Transport streaming likewise needs revision/order, atomic updates, and disposal rules beyond renaming envelope fields.

Acceptance should grow in stages: (1) validate and round-trip static Basic fixtures while preserving IDs, text/alternative semantics, order, defaults, and resources; (2) replay input and update traces to compare values, validation, and emitted actions; (3) add individually negotiated Material/media/chart/custom profiles. Exercise a web renderer and Android Compose for Basic. Mark unsupported cases in a machine-readable capability matrix and test failures, not only successful screenshots. Catalog coverage is complete only when both components and their behavioral dependencies pass.

## 5. Shared MVP value types

### 5.1 Geometry and sizing types

```nx
type Alignment = start | center | end | stretch
type AlignSelf = auto | start | center | end | stretch
type AxisAlignment = start | center | end
type Axis = horizontal | vertical
type Distribution = start | center | end | spaceBetween | spaceAround | spaceEvenly

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
  top: float64 = 0
  right: float64 = 0
  bottom: float64 = 0
  left: float64 = 0
}

type CornerRadii = {
  topLeft: float64 = 0
  topRight: float64 = 0
  bottomRight: float64 = 0
  bottomLeft: float64 = 0
}
```

Three of these declarations lose a shorthand that the JSON form keeps, all for the same reason: NX unions are named cases, so they cannot mix a scalar with a record or a string literal. `Length` becomes three cases instead of `120`/`"auto"`/`"50%"`; `Insets` and `CornerRadii` lose their uniform-scalar form, so `padding: 20` has to be written with all four sides; and `Alignment | "auto"` becomes a separate `AlignSelf` union. Appendix B gives the details and the wire-format consequences.

`start`/`end` are chosen over `left`/`right` so layout can adapt to writing direction. The vocabulary follows [CSS Box Alignment](https://www.w3.org/TR/css-align-3/) and A2UI's `align`/`justify` conventions. DrawnUI adapters map these to MAUI layout options.

`Length` deliberately omits CSS `calc()`, viewport units, and arbitrary strings. `auto` means intrinsic sizing; percentages resolve against the containing content box. A percentage against a parent that is itself intrinsically sized resolves as `auto`, so layout never depends on a circular constraint.

Omitted `Insets` sides default to `0`, so `{ "top": 8 }` is valid and cheap to generate. `Insets` uses physical `left`/`right` while alignment uses logical `start`/`end`; that split is intentional for the MVP, which has no bidirectional text policy, and §11 tracks logical insets with the rest of the localization work.

### 5.2 Color and paint

```nx
type Color = string   // CSS Color syntax subset accepted by the host

type GradientUnits = objectBoundingBox | userSpaceOnUse

type GradientStop = {
  offset: float64          // 0..1
  color: Color
  opacity: float64 = 1     // 0..1
}

type Paint =
  | none
  | solid { color: Color }
  | linearGradient {
      x1: float64
      y1: float64
      x2: float64
      y2: float64
      units: GradientUnits = objectBoundingBox
      stops: GradientStop[]
    }
  | radialGradient {
      cx: float64
      cy: float64
      r: float64
      fx: float64?
      fy: float64?
      units: GradientUnits = objectBoundingBox
      stops: GradientStop[]
    }
```

`Paint` is tidier as an NX union than it was as a TypeScript one: `"none"`, which the TypeScript form had to bolt on at each use site as `Paint | "none"`, is simply a case, and the discriminator stops being a hand-written `type` field. The cost is at the other end — a plain color is no longer a bare string but a `solid` case with a `$type` tag. See Appendix B.

Use the portable subset of [CSS Color 4](https://www.w3.org/TR/css-color-4/) for `Color`: named colors, `transparent`, hex, `rgb()`/`rgba()`, and `hsl()`/`hsla()`. Renderers must normalize colors; unsupported CSS-wide keywords and environment-dependent values are invalid.

`stops` must contain at least two entries with finite offsets in 0..1 in non-decreasing order; out-of-range offsets are validation errors, not backend-dependent clamping. Equal offsets are allowed for hard transitions and retain authored order. Gradient extension is pad (endpoint colors extend beyond the stop range).

Gradient coordinates are evaluated in the painted geometry's local space before the node transform. For `objectBoundingBox`, use its geometric bounds excluding stroke/shadow; map the unit square to that box, so a radial circle can become elliptical. Zero-width or zero-height geometric bounds disable an object-bounding-box gradient; producers drawing stroked lines should use `userSpaceOnUse`. A coincident linear start/end or zero radial radius paints the last stop's color; negative radius is invalid. Omitted focal coordinates use the center; the initial profile requires the focal point inside or on the outer circle and rejects an outside focus rather than relying on different backend adjustments. These rules deliberately constrain the broader SVG imaging model.

The MVP rendering profile uses sRGB colors and source-over compositing. Interpolate gradient stop colors in encoded sRGB with premultiplied alpha; the effective stop alpha is color alpha multiplied by stop opacity. Backends must select equivalent behavior explicitly, including for transparent colored stops. Wider-gamut/HDR colors, alternate interpolation spaces, and blend modes require later profiles. Round-trip SVG export must state NX's gradient interpolation contract instead of assuming every SVG renderer chooses the same one.

Gradient terms and coordinate modes intentionally follow [SVG gradients](https://www.w3.org/TR/SVG2/pservers.html). DrawnUI's [`SkiaGradient`](https://github.com/DrawnUi/DrawnUi.Net/blob/main/src/Shared/DrawnUi/Draw/SkiaGradient.cs) already carries the same information in parallel arrays — `Colors` plus `ColorPositions` — so an adapter zips those into ordered `stops` without loss. Its `StartXRatio`/`StartYRatio`/`EndXRatio`/`EndYRatio` are ratios of the painted bounds, which is exactly NX's default `objectBoundingBox` mode; that is independent evidence for the default chosen in §12.4. DrawnUI's `TileMode` (gradient spread) and `GradientType.Sweep`/`Conical` have no MVP equivalent and are deferred in §11.

#### Theme and content-color inheritance

Omitted `nx.ui.Text.color` and `nx.ui.Icon.color` inherit the nearest host-provided content foreground, with the document theme as the root fallback. A themed surface such as `Card` establishes its matching foreground and background as a pair; nested surfaces resolve through the same host theme. Explicit colors override inheritance. Implementations must not hard-code black for UI leaves or independently choose a foreground that loses contrast on the host surface. Graphics fill defaults remain those specified by the graphics catalog.

This host theme behavior is required for the MVP. Named semantic color tokens and image presentation presets (`avatar`, `header`, and similar hints) are useful next extensions, but their portable encodings and override precedence need a separate schema decision. `Color` remains a CSS-color subset, not an implicit token language. An A2UI adapter must preserve image hints as explicit resolved properties or a declared theme profile, rather than dropping them.

### 5.3 Stroke, shadow, transform, border

```nx
type LineCap = butt | round | square
type LineJoin = miter | round | bevel

type Stroke = {
  paint: Paint
  width: float64 = 1
  lineCap: LineCap = butt
  lineJoin: LineJoin = miter
  miterLimit: float64 = 4
  dashArray: float64[]?
  dashOffset: float64 = 0
}

type Shadow = {
  color: Color
  offsetX: float64 = 0
  offsetY: float64 = 0
  blur: float64 = 0        // Gaussian standard deviation in local logical units; >= 0
}

type Transform =
  | translate { x: float64 y: float64 }
  | scale { x: float64 y: float64? }        // null y means uniform scale
  | rotate { degrees: float64 cx: float64 = 0 cy: float64 = 0 }
  | skewX { degrees: float64 }
  | skewY { degrees: float64 }
  | matrix { a: float64 b: float64 c: float64 d: float64 e: float64 f: float64 }

type Border = {
  paint: Paint
  width: float64 = 1
}
```

`Transform` is the case NX models best: a closed set of variants with per-variant payloads is exactly what a discriminated union is for, and the hand-written `type` field disappears. The one thing it cannot carry is `scale.y` defaulting to `scale.x`, since an NX default is a literal or a constant expression and cannot reference a sibling property; `y` is therefore nullable, with null meaning uniform scale.

Stroke terminology follows [SVG painting](https://www.w3.org/TR/SVG2/painting.html). Transform names and the six-value affine matrix follow [SVG/CSS Transforms](https://www.w3.org/TR/css-transforms-1/). The array is equivalent to an SVG `transform` list read left to right: matrices are concatenated in array order, so the *last* entry is the one applied first to the node's own geometry. `rotate` uses positive degrees clockwise in NX's y-down coordinate space, matching SVG. A stroke is scaled by the transforms in effect, as in SVG; there is no MVP equivalent of `vector-effect: non-scaling-stroke`. DrawnUI's `StrokeColor`, `StrokeWidth`, `StrokeCap`, dash path, gradients, and multiple shadows map directly to these portable objects.

#### Stroke and shadow rules

Stroke width must be finite and non-negative; zero means no stroke, never a backend-specific device-pixel hairline. Miter limit must be at least 1. Dash entries must be finite and non-negative; an omitted/empty array is solid, an odd-length array repeats to form an even-length pattern, and an all-zero pattern is solid. Dash offset is finite and follows SVG dash phase semantics. Strokes are centered on geometry and scale with its transform. A future hairline or non-scaling-stroke option must be explicit.

Paint order is shadows, fill, then stroke. Each shadow is generated from the combined fill/stroke alpha silhouette before node opacity, blurred with Gaussian standard deviation `blur`, offset in local logical units, colored by its own color/alpha, and painted behind the source. Multiple shadows paint in array order, with later ones above earlier ones. Then apply the node's clip and composite the result at its opacity; ancestor clips remain in effect. This is NX's defined drop-shadow profile, not a promise that CSS `box-shadow`, Canvas `shadowBlur`, and Skia blur parameters are interchangeable. Approximation tolerances for blur must be documented. Offscreen allocations must include effect bounds and respect host limits.

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

Every node inside a `Drawing` inherits the `GraphicsElement` properties, declared in NX by an abstract external component that concrete components extend:

```nx
abstract external component <GraphicsElement
  opacity: float64 = 1
  transform: Transform[]?
  clipPath: string?
  clipRule: FillRule = nonzero
  accessibility: Accessibility?
/>
```

| Property | Type | Default | Meaning |
|---|---|---:|---|
| `opacity` | `float64` (0..1) | `1` | Group opacity: the node and its descendants are composited as one layer, then that layer is blended at this alpha. |
| `transform` | `Transform[]?` | `[]` | Ordered local transforms. |
| `clipPath` | `string?` | none | SVG path-data geometry used as a local clip, in the node's own coordinate system. |
| `clipRule` | `FillRule` | `nonzero` | Clip interior rule, independent of the painted shape's `fillRule`; ignored when no clip is present. |
| `accessibility` | `Accessibility?` | derived | Optional accessible name/description or decorative status. |

`opacity` is deliberately specified as group opacity rather than a per-descendant multiplier: the two differ visibly wherever descendants overlap, and SVG and Skia provide isolated group compositing; a Canvas 2D adapter may need an offscreen surface. Multiplying `globalAlpha` per child is not equivalent.

`clipPath` geometry is interpreted in the node's local coordinate system — the space established *after* the node's own `transform` is applied — and clips the node and all of its descendants. It uses `clipRule`, independently of the shape's `fillRule`, so even-odd holes are expressible without masks. It is purposefully a small MVP subset of SVG clipping; referenced clip trees, masks, and filter regions are deferred.

Every filled/stroked geometry inherits the `Shape` properties in addition to its own. `Shape` extends `GraphicsElement`, so shapes inherit both sets of properties through one base-type chain:

```nx
type FillRule = nonzero | evenodd

abstract external component <Shape extends GraphicsElement
  fill: Paint?
  stroke: Stroke?
  fillRule: FillRule = nonzero
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

The default fill is `Paint.none` for `Line` and `Polyline`, and black for closed shapes and `Path`. Closed-shape behavior matches SVG's initial `fill: black`. `Polyline` is a deliberate divergence: SVG also fills a polyline by default, which surprises nearly everyone and produces a wrong-looking region whenever a model draws an open multi-segment stroke. Path retains SVG-compatible black fill; stroke-only paths must explicitly disable fill.

### 6.2 `nx.graphics.Drawing`

Root drawing surface and bridge into Level 2 UI. Accepts ordered drawing children. A Drawing is also a valid standalone document root: a host supplies a logical viewport, device scale, surface, and resource services directly. This path needs no Row/Column/Grid measurement engine. Shared sizing declarations may be linked for type checking without requiring a UI renderer; non-default properties requiring parent layout are rejected diagnostically by a standalone host. Surface background, clipping, opacity, and accessibility still apply.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | no | Controls the surface's size and placement when embedded in UI. |
| `viewBox` | `ViewBox` | yes | Internal coordinate rectangle; width and height must be positive. |
| `fit` | `ViewBoxFit` | no | `contain`; fit, crop, or non-uniformly stretch the view box into the surface. |
| `contentAlignment` | `ContentAlignment?` | no | Center/center; only meaningful when `fit` preserves the aspect ratio. |

```nx
type ViewBoxFit = contain | cover | fill

type ContentAlignment = {
  x: AxisAlignment = center
  y: AxisAlignment = center
}
```

When `width` and `height` are `auto`, the surface's intrinsic size is the view box's `width` and `height` in logical pixels, and its intrinsic aspect ratio is `width / height`; a single specified dimension resolves the other through that ratio. Content is always clipped to the surface box. The common `background` paints the surface before its drawing children, and common `accessibility` describes the surface.

Rationale: `viewBox` semantics come from [SVG coordinate systems](https://www.w3.org/TR/SVG2/coords.html). `fit` is the one place where NX deliberately spells an SVG concept in CSS terms: `contain`/`cover`/`fill` map one-to-one onto `preserveAspectRatio` `meet`/`slice`/`none`, and reusing the `object-fit` vocabulary that `nx.ui.Image` and `nx.graphics.Image` already use means a generating model learns one fit vocabulary instead of two. SVG also folds alignment into the same string (`xMidYMid meet`); NX keeps `contentAlignment` separate so neither value has to be parsed. Unlike DrawnUI's canvas control, the portable object does not expose acceleration, cache, pixel density, or invalidation settings.

### 6.3 `nx.graphics.Group`

Groups ordered drawing children without drawing its own geometry.

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` properties | see §6.1 | no | Shared opacity, transform, clip, and accessibility. |

Rationale: equivalent to SVG [`g`](https://www.w3.org/TR/SVG2/struct.html#Groups) and a Skia save/restore scope. Grouping avoids repeating transforms or opacity.

### 6.4 `nx.graphics.Rect`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared drawing and paint properties. |
| `x` | `float64` | no | `0`; left coordinate. |
| `y` | `float64` | no | `0`; top coordinate. |
| `width` | `float64` | yes | Non-negative width. |
| `height` | `float64` | yes | Non-negative height. |
| `rx` | `float64` | no | `0`; horizontal corner radius. |
| `ry` | `float64?` | no | `rx`; vertical corner radius. NX cannot spell this default; see Appendix B. |

Rationale: names and radius behavior follow SVG [`rect`](https://www.w3.org/TR/SVG2/shapes.html#RectElement). DrawnUI `Rectangle` + `CornerRadius` maps here; resolve omitted `ry` to `rx`, clamp `rx` to half the width and `ry` to half the height, and reject negative radii. A zero dimension yields no painted rectangle; unequal radii must be supported or reported as unsupported, not silently replaced by uniform rounding.

### 6.5 `nx.graphics.Circle`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties. |
| `cx` | `float64` | yes | Center x. |
| `cy` | `float64` | yes | Center y. |
| `r` | `float64` | yes | Non-negative radius. |

Rationale: matches SVG [`circle`](https://www.w3.org/TR/SVG2/shapes.html#CircleElement) and DrawnUI's explicit `Circle` shape. Keeping it separate from `Ellipse` is more legible for models and people.

### 6.6 `nx.graphics.Ellipse`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties. |
| `cx` | `float64` | yes | Center x. |
| `cy` | `float64` | yes | Center y. |
| `rx` | `float64` | yes | Non-negative horizontal radius. |
| `ry` | `float64` | yes | Non-negative vertical radius. |

Rationale: matches SVG [`ellipse`](https://www.w3.org/TR/SVG2/shapes.html#EllipseElement) and DrawnUI `Ellipse`.

### 6.7 `nx.graphics.Line`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties; `fill` has no effect. |
| `x1` | `float64` | yes | Start x. |
| `y1` | `float64` | yes | Start y. |
| `x2` | `float64` | yes | End x. |
| `y2` | `float64` | yes | End y. |

Rationale: follows SVG [`line`](https://www.w3.org/TR/SVG2/shapes.html#LineElement). DrawnUI models lines as point collections; NX adds the common two-point primitive and retains `Polyline` for multi-segment lines.

### 6.8 `nx.graphics.Polyline`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Two or more ordered vertices; path remains open. |

### 6.9 `nx.graphics.Polygon`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties. |
| `points` | `Point[]` | yes | Three or more ordered vertices; closing segment is implicit. |

Rationale for both: follows SVG [`polyline` and `polygon`](https://www.w3.org/TR/SVG2/shapes.html#PolylineElement) and maps directly from DrawnUI `Points`. DrawnUI's non-standard `SmoothPoints` is deferred; smooth geometry should use an explicit `Path` in the MVP.

### 6.10 `nx.graphics.Path`

| Property | Type | Required | Meaning |
|---|---|---:|---|
| all `GraphicsElement` and `Shape` properties | see §6.1 | no | Shared properties. |
| `data` | `string` | yes | SVG path-data string. |

`data` uses the standard SVG command grammar (`M/L/H/V/C/S/Q/T/A/Z`, absolute or relative) from [SVG Paths](https://www.w3.org/TR/SVG2/paths.html). This is already the format of DrawnUI `SkiaShape.PathData`, so it is the strongest direct compatibility point. Parse the complete string before painting; malformed path data fails validation rather than rendering a backend-specific valid prefix. Use one tested SVG-path parser/normalizer for direct and recorded paths, including elliptical arc conversion. Preserve subpath boundaries and open/closed status; filling an open subpath uses implicit closure while stroking it does not. Impose command-count, numeric-range, and rendered-bounds limits. Closed-path fill remains SVG-compatible black by default; authors of stroke-only paths should specify `fill: none` explicitly.

### 6.11 `nx.graphics.Text`

Single-line or explicitly line-broken text positioned in drawing coordinates. It does not perform UI box layout. Shape each line through the host text service, including script shaping, bidirectional ordering, fallback fonts, and glyph positioning; drawing Unicode code points one by one is not conforming. The first line uses the stated anchor/baseline, subsequent baselines advance by `fontSize * lineHeight`; apply `textAnchor` independently to each line's shaped advance. `auto` baseline means the alphabetic baseline. Font selection is host-resolved and must match measurement and paint; exact cross-host metrics require pinned font resources.

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsElement` properties | see §6.1 | no | Shared properties. |
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
| `lineHeight` | `float64` (> 0) | no | `1.2`; baseline advance as a multiplier of `fontSize` for explicit line breaks. |
| `fill` | `Paint?` | no | Black. |
| `stroke` | `Stroke?` | no | None. |
| `shadows` | `Shadow[]?` | no | `[]`. |

Rationale: positioning and anchor terms follow [SVG text](https://www.w3.org/TR/SVG2/text.html). Numeric weight follows modern CSS/OpenType and is more portable than `Bold` flags. DrawnUI `SkiaLabel` supplies the implementation precedent for family, size, weight, spacing, fill/stroke, and shadows. Rich spans, text-on-path, shaping controls, and automatic fit are deferred.

### 6.12 `nx.graphics.Image`

| Property | Type | Required | Default / meaning |
|---|---|---:|---|
| all `GraphicsElement` properties | see §6.1 | no | Shared properties. |
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

#### Why `Row` / `Column` / `ZStack`

Use `Row` and `Column` as the canonical linear containers, with no aliases in the MVP. They match A2UI, [Flutter](https://docs.flutter.dev/ui/layout), and [Jetpack Compose](https://developer.android.com/develop/ui/compose/layouts/basics). React itself provides no layout components; common libraries vary: [MUI](https://mui.com/material-ui/react-stack/) uses `Stack` with a row/column direction, [Radix Themes](https://www.radix-ui.com/themes/docs/components/flex) uses `Flex` with that direction, and [Chakra](https://chakra-ui.com/docs/components/stack) offers axis-prefixed stacks. Row/column is the shared directional vocabulary, even when it is a property rather than a component name.

Keep `ZStack` for overlay. The explicit Z distinguishes overlap from linear flow, whereas bare `Stack` means linear layout in several ecosystems, including DrawnUI's vertical `SkiaStack`. Naming symmetry is less valuable than recognizable meaning. `Grid` continues to use rows and columns for its tracks; a `Row` or `Column` child is a container placed in a cell, not a track declaration. No evidence currently justifies avoiding these familiar names to prevent hypothetical grid confusion.

### 7.1 `UiCommonProps`

Every Level 2 component, plus `nx.graphics.Drawing`, accepts these optional flattened properties. In NX this is one abstract external component that every Level 2 component extends:

```nx
abstract external component <UiCommon
  width: Length = auto
  height: Length = auto
  minWidth: float64 = 0
  minHeight: float64 = 0
  maxWidth: float64?
  maxHeight: float64?
  margin: Insets?
  padding: Insets?
  alignSelf: AlignSelf = auto
  justifySelf: AlignSelf = auto
  grow: float64 = 0
  shrink: float64 = 1
  gridColumn: int?
  gridRow: int?
  columnSpan: int = 1
  rowSpan: int = 1
  background: Paint = none
  border: Border?
  cornerRadius: CornerRadii?
  shadows: Shadow[]?
  clip: boolean = false
  opacity: float64 = 1
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
| `grow` | `float64` | `0` | Share of positive free space in a `Row` or `Column`. |
| `shrink` | `float64` | `1` | Relative shrink factor in a `Row` or `Column`. |
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
| `Row` | ignored; use `justify` on the parent plus `grow`/`shrink` | overrides the parent's `align` |
| `Column` | overrides the parent's `align` | ignored; use `justify` on the parent plus `grow`/`shrink` |
| `Grid`, `ZStack` | overrides `justifyItems` | overrides `alignItems` |
| `Scroll`, `Box`, `Card` | positions the single child in the content box | positions the single child in the content box |

A property that a parent ignores is valid but has no effect; it is a diagnostic, not a validation error, so that a subtree can be re-parented without becoming invalid.

These are constraints and portable appearance, not a CSS passthrough. Names balance CSS flex/grid familiarity with DrawnUI/MAUI's width request, margin/padding, alignment, background, clipping, and grid row/column concepts. Layout engines must document deterministic treatment of over-constrained values; where `width` conflicts with `minWidth`/`maxWidth`, the minimum wins, matching CSS.

### 7.2 `nx.ui.Row`

Horizontal flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `float64` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `start` | Distribution along the horizontal main axis. |
| `align` | `Alignment` | `stretch` | Alignment on the vertical cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional lines. |

### 7.3 `nx.ui.Column`

Vertical flow container; accepts zero or more UI children.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `gap` | `float64` | `0` | Space between adjacent children. |
| `justify` | `Distribution` | `start` | Distribution along the vertical main axis. |
| `align` | `Alignment` | `stretch` | Alignment on the horizontal cross axis. |
| `wrap` | `boolean` | `false` | Wrap children into additional lines. |

`justify` distributes only the free space that remains after `grow` has been applied, so a child with `grow > 0` and a `spaceBetween` parent is not double-counted. When `wrap` is `true`, `gap` separates both adjacent children and adjacent lines, and lines are packed toward the cross-axis start; per-line distribution (CSS `align-content`) is deferred.

Rationale: A2UI's basic catalog defines the same two containers (as `Row` and `Column`) with similar `justify` (`start`/`center`/`end`/`spaceBetween`/`spaceAround`/`spaceEvenly`) and `align` (`start`/`center`/`end`/`stretch`) enums, defaulting to `start` and `stretch` as NX does, and expresses proportional sizing as a `weight` on the child. A2UI additionally accepts main-axis `justify: stretch`; NX does not equate that with cross-axis stretch or `spaceBetween`. Its import requires a specified sizing rule or a diagnostic. CSS Flexbox supplies the NX behavioral model. NX adds `gap`, which A2UI leaves to the host theme but DrawnUI exposes as `Spacing`. `grow`/`shrink` replace both A2UI's single `weight` and DrawnUI's sizing idiom, because a shrink factor is needed once `wrap` is false and content overflows.

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

`variant` is a semantic role, not only a type ramp: renderers must expose `h1` through `h6` as platform heading elements at the corresponding level, and `code` with a monospace face and a code role where the platform has one. Nothing else in the read-only MVP conveys document structure to assistive technology, so a model that reaches for `fontSize` instead of `variant` silently produces a flat, unnavigable document.

When `format` is `"markdown"`, `variant` sets the *baseline* role for the block and Markdown structure nests below it; heading syntax inside a `caption` must not escape its container's typography scale.

Rationale: A2UI v0.9.1 `Text` combines simple Markdown with typography variants (`h1`–`h5`, `body`, `caption`); DrawnUI `SkiaLabel` supplies detailed text properties (`FontFamily`, `FontSize`, `FontWeight`, `LineHeight`, `CharacterSpacing`, `MaxLines`, `LineBreakMode`, stroke and drop shadow). NX preserves explicit heading semantics through level 6 and makes semantic `variant` preferable to low-level overrides. The current A2UI v1.0 candidate Basic schema permits only `body` and `caption`; an exporter must check the selected catalog rather than assume v0.9.1 heading variants survive unchanged. Hosts must sanitize Markdown, disallow raw HTML by default, and apply a link policy. Rich spans, `TextTransform`, and `AutoSize`-style shrink-to-fit are deferred.

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

### 7.13 `nx.ui.List`

A scrollable, ordered sequence of zero or more explicitly listed UI children. This is ordinary read-only content, not a data-bound item generator.

| Property | Type | Default | Meaning |
|---|---|---:|---|
| all `UiCommonProps` | see §7.1 | — | Shared constraints and appearance. |
| `direction` | `Axis` | `vertical` | Sequence and scrolling axis. |
| `gap` | `float64` | `0` | Non-negative space between adjacent items. |
| `align` | `Alignment` | `stretch` | Alignment across the sequence axis. |
| `scrollbar` | `ScrollbarVisibility` | `auto` | Host scrollbar presentation hint. |
| `children` | `ElementId[]` | empty | Explicit ordered items; no templates or bindings in MVP. |

A List owns its viewport and scrolls along `direction`; items are measured unbounded along that axis and constrained across it. It requires a bounded viewport from its parent, like `Scroll`. Default margins/padding remain zero. It is equivalent to a `Scroll` containing a non-wrapping `Column` or `Row` with the specified `gap` and `align`; item growth along the scroll axis has no finite free space to consume. Expose list/item semantics and reading order to accessibility, without fabricating selection or focus behavior. Item positions are sibling order, not alphabetical or data sort order.

This gives A2UI's static `List` a direct semantic counterpart without introducing state. Dynamic child templates, stable data-item keys, recycling/virtualization guarantees, and selection remain post-MVP. A2UI export still has to account for target spacing and viewport conventions.

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
      "type": "nx.ui.Column",
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
import { Card as ui.Card, Column as ui.Column, Text as ui.Text } from "../ui"
import "../graphics" as gfx

<ui.Card width=<Length.px value=360.0 />
         padding=<Insets top=20.0 right=20.0 bottom=20.0 left=20.0 /> >
  <ui.Column gap=12.0>
    <ui.Text: variant=h2>NX UI</ui.Text>
    <gfx.Drawing
      height=<Length.px value=140.0 />
      viewBox=<ViewBox x=0.0 y=0.0 width=320.0 height=140.0 /> >
      <gfx.Rect
        width=320.0 height=140.0 rx=12.0
        fill=<Paint.linearGradient
                x1=0.0 y1=0.0 x2=1.0 y2=1.0
                stops={ <GradientStop offset=0.0 color="#5B5BD6" />
                        <GradientStop offset=1.0 color="#14B8A6" /> } /> />
      <gfx.Path
        data="M20 105 C90 15 210 125 300 35"
        fill=none
        stroke=<Stroke paint=<Paint.solid color="white" /> width=5.0 lineCap=round /> />
      <gfx.Circle cx=300.0 cy=35.0 r=7.0 fill=<Paint.solid color="white" /> />
    </gfx.Drawing>
    <ui.Text: variant=caption>Portable layout above; portable drawing below.</ui.Text>
  </ui.Column>
</ui.Card>
```

Four differences from the JSON are worth reading off directly, because each one is a property of NX rather than of this proposal.

The IDs are gone. NX nests, so structure is expressed by containment rather than by `children: ["title", "graphic"]` pointing into a map. Producing the flat form is a compilation step, not something NX source can state.

The prefixes are one segment deep. `ui.` and `gfx.` are import aliases, and NX allows exactly one qualifying segment, so `nx.ui.` and `nx.graphics.` are not spellable as prefixes — the alias is the local abbreviation for a catalog whose real ID lives in `catalogs`. Both catalogs can now use the same import style: contextual literal binding means `variant=h2` resolves against the property's declared type, so `TextVariant` never has to be named at the use site and no longer has to be imported unqualified to work around wildcard-alias member access. Appendix B has the detail.

Sizes and insets got longer. `"width": 360` becomes `<Length.px value=360.0 />` and `"padding": 20` becomes a four-sided `Insets`, both because NX cannot union a scalar with a structured alternative.

Text reads better. `<ui.Text: ...>NX UI</ui.Text>` uses NX's text-element form, which binds the body to the component's `content` property — the one place where NX authoring is more compact than the JSON, not less.

## 9. DrawnUI evolution and adapter mapping

| DrawnUI today | NX MVP | Evolution rationale |
|---|---|---|
| `Canvas` + `SkiaControl` base capabilities | `NxUiDocument`, `Element`, `UiCommonProps` | Separate portable document semantics from renderer lifecycle, invalidation, cache, and GPU configuration. DrawnUI's `Canvas` hosts an entire drawn UI tree, so it corresponds to the document host — not to `nx.graphics.Drawing`, which is an embeddable element. That mismatch is why the portable model does not reuse the word `Canvas`; see §1. |
| no direct equivalent | `nx.graphics.Drawing` | DrawnUI has no embeddable coordinate-space element; an adapter synthesizes one as an `Absolute` `SkiaLayout` sized to the surface, with the view-box mapping applied as a transform on its children. |
| `SkiaLayout Type="Row"` / `SkiaRow` | `nx.ui.Row` | Explicit catalog type; `Spacing` becomes `gap`; MAUI alignment maps to `align`/`justify`/`alignSelf`. |
| `SkiaLayout Type="Column"` / `SkiaStack` alias | `nx.ui.Column` | DrawnUI’s `SkiaStack` is a `Column`; the canonical name makes the vertical direction explicit. |
| `SkiaLayout Type="Grid"` / `SkiaGrid` | `nx.ui.Grid` | Preserve rows, columns, gaps, placement, and spans in a smaller typed subset. |
| `SkiaLayout Type="Absolute"` / `SkiaLayer` | `nx.ui.ZStack` for overlay; `nx.graphics.Drawing` for coordinates | Separate ordinary layering from coordinate-based graphics. |
| `SkiaLayout Type="Wrap"` / `SkiaWrap` | `Row` or `Column` with `wrap: true` | Avoid another component when flex-like wrapping is enough. |
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

### 9.1 Three contracts and two execution paths

A **retained NX UI document** describes component identity, layout constraints, graphics geometry, and accessibility. An **NX drawing context** is a trusted host API invoked after validation, resource resolution, text measurement, and layout. An **NX display list** is optional, immutable, ordered paint output returned to a host for later replay. It contains resolved drawing operations, not UI components or layout instructions. Neither the context nor the display list adds executable callbacks to generated documents.

```text
validated document + viewport + theme + approved resources
                         |
                measure / arrange / shape text
                         |
                 resolved paint traversal
                         |
                  NX drawing context
                   /              \
          direct backend       NX recorder
          Skia first               |
               |             NX display list
          host surface       returned to host
                                   |
                            replay to a backend
```

The host selects the destination before painting. In **direct mode**, each context operation invokes the selected backend API immediately, with no intermediate standard NX command buffer. Skia may still buffer GPU commands internally; immediate here describes NX's API dispatch, not synchronous pixel presentation. Backend-native caches, including Skia pictures, are permitted implementation choices.

In **recorded mode**, the same traversal writes an NX-defined display list and returns it with its resource manifest and semantic companion data. The receiving host replays it through a compatible backend. Recording is useful for separating producers from presentation threads/processes, repeated replay, inspection, and transport. It has allocation, serialization, and resource costs and should not be imposed on an in-process DrawnUI host without measurements.

A DOM/SVG renderer may map the semantic document directly to platform elements. It need not consume this drawing interface or a display list. The graphics interface is a portability boundary for painted renderers, not a replacement for every renderer architecture.

### 9.2 Pluggable immediate drawing contract

Use a small NX-owned interface (working name `INxDrawingContext`) with no `SKCanvas`, `SKPaint`, MAUI, DOM, or GPU types in its public contract. Register implementations through trusted host configuration. The first implementation wraps a host-supplied Skia canvas; a recorder implements the same operations. Keep layout independent of concrete backend types.

The initial operation families should cover:

| Family | Required semantics |
|---|---|
| State and transforms | Balanced save/restore; concatenated affine transforms with §5.3 ordering; rectangular/path clipping; host-established root device scale. |
| Compositing | Begin/end isolated opacity layers. Group opacity applies once to the composited subtree, not independently to each child's paint. |
| Geometry | Rectangles/rounded rectangles, ellipses, paths, fills, strokes, gradients, and the MVP shadow semantics. Lines and polygons may lower to paths. |
| Content | Images with resolved source/destination rectangles and clipping; shaped text runs positioned using the same metrics used for layout. |

Prefer explicit paint values per draw operation over a large mutable paint state machine. Parsed paths, images, and fonts can use host-local opaque handles, with ownership and lifetime specified; these handles must never leak into portable serialized output. A context is frame-scoped, used on its backend's permitted thread, and restores the host canvas state on completion or failure. The host owns the surface, device scale, scheduling, invalidation, and presentation.

Text measurement/shaping and resource resolution are companion services, not incidental canvas methods. A text layout result must retain the font selection, fallback runs, glyph positioning, and metrics that painting actually uses. A Skia implementation may initially use Skia text services, but their types stay behind adapters. Pluggable rasterization does not by itself provide portable text layout.

This is a host integration API, not a new public imperative NX language feature. An application-facing imperative graphics API can be considered separately after this contract proves useful.

### 9.3 NX display-list profile

Define a separately versioned output profile, provisionally `nx-display-list@0.1.0`, rather than adding commands to `nx.graphics` or putting a rendering-mode property on `NxUiDocument`. A recorder and replay implementation should prove these requirements before the output format is frozen:

- **Self-contained frame description:** format/profile version, logical viewport, device-scale assumptions, explicit color space (MVP sRGB), command sequence, resource manifest, and the rendering-environment fingerprint. Theme and layout choices are already resolved. Begin with full-frame lists; partial-list updates are deferred.
- **Bounded command vocabulary:** save/restore, affine transforms, clips, isolated opacity layers, geometry, images, and glyph runs. Document root state, coordinate units, stroke scaling, clip intersection, and compositing order. Preserve source element IDs in a side table for diagnostics, rather than turning every command into a retained element.
- **Portable resources:** versioned IDs resolve to approved immutable image/font/path data or a host-negotiated resource table. No native pointers, GPU texture handles, platform font handles, arbitrary callbacks, or Skia picture bytes. The list retains resources until the host releases it; a transmitted list must declare how its receiver obtains and authorizes every resource.
- **Text replay without reflow:** record glyph runs with font-resource identity, glyph IDs, positions, and necessary rendering attributes. The receiver must have the matching font bytes/face/variation identity, including fallback fonts. Specify handling for color glyphs; use a declared raster fallback or reject unsupported runs. Replacing a font or reshaping plain strings is not equivalent replay. Resource packaging is a rendering requirement even though author-selected font resources remain deferred from the document catalog.
- **Independent validation:** check versions, opcode payloads, finite values, resource references, balanced stacks, byte/command counts, layer depth/area, and glyph/path complexity before replay. Validating the source document does not validate a list received across a trust boundary. Unknown required operations fail explicitly.

The first representation can be an in-memory typed command sequence with a readable diagnostic encoding. A portable serialized representation needs its own schema and resource protocol before cross-process interoperability is claimed; choosing JSON versus binary should follow measurements. A list requiring undeclared local resources is only a local recording, not a transportable artifact.

Return accessibility/semantic nodes and their arranged bounds alongside a frame, linked by element ID and document revision. Paint commands alone cannot recover heading roles, image alternatives, reading order, or scroll structure. The host installs the semantic tree atomically with the corresponding frame; lists do not replace the source document's semantics. Internal arranged bounds needed for accessibility are required now; a public graphics geometry/hit-testing API remains deferred.

### 9.4 DrawnUI migration and capability policy

The requested `~/src/DrawnUI.Fiddle` checkout was not present under that exact name during this review. The available related checkout is `~/src/DrawnUi.FiddleEngine`; its `src/DrawnUi.Fiddle/FiddlePresets.cs`, `Paint` preset, obtains `ctx.Context.Canvas` and calls `DrawCircle` directly from `WhenPaint`. Its `README-NX.md` also documents an existing NX-to-DrawnUi.React integration backed by a generated DrawnUI catalog. That is a valuable migration reference, though it exposes DrawnUI controls rather than the portable catalog proposed here. The paint example confirms direct Skia dispatch; it does not establish that every DrawnUI cache path avoids recording.

Keep that direct integration viable. Initially the adapter can map NX documents to existing DrawnUI controls. A shared NX paint traversal can then target the Skia drawing context and the recorder. Wrapping existing controls is a migration technique, not proof of a portable backend: controls that take raw Skia objects must be adapted or bypassed before recording can share the same traversal. Avoid maintaining separate, subtly different layout algorithms solely for the two output modes.

Backend registration declares supported operations, text/resource services, and output profiles. The host selects a compatible backend before painting; it must not silently ignore clipping, opacity isolation, or other required semantics. A documented equivalent software/offscreen fallback is acceptable. Explicitly lossy fallback requires host policy and a diagnostic; it is not full conformance. Generated content cannot select or load a backend plugin.

### 9.5 Delivery recommendation

1. Specify the bounded drawing interface and implement direct Skia rendering. Preserve the current low-overhead host path.
2. In the same MVP prototype, add an NX recorder and replay it through Skia. Validate the operation semantics and resource lifetime before promising a stable display-list format.
3. Exercise a second immediate backend (for example Canvas 2D) with the shared traversal and a representative graphics corpus. This is the portability gate; a second Skia wrapper or a DOM-only renderer does not prove the drawing interface is independent of Skia.
4. Stabilize the portable display-list encoding and resource packaging when a host actually needs returned lists. Ship it as an optional output profile; hosts may support direct mode, recorded mode, or both and must declare which.

Recommend **both paths, with direct Skia as the first production path and recording as an optional output profile**. Do not make a stable cross-process graphics protocol a prerequisite for the read-only UI MVP. Conversely, do not claim display-list interoperability until independent validation, resource transfer, and matching semantic output are demonstrated.

### 9.6 Initial graphics-first OpenSpec changes

First implement and verify [the math standard library](../openspec/changes/archive/2026-10-28-add-math-standard-library/proposal.md), including its cross-runtime numeric contract. Graphics authoring helpers then use `@nx/math`, with explicit degree-to-radian conversion, instead of private numerical approximations. The first graphics deliverable is an NX-authored standalone drawing rendered directly to Skia. A2UI conversion and Level 2 UI layout are later work. Keep each implementation profile explicit: supporting a small primitive subset is not full `nx.graphics@0.1.0` conformance.

1. **`add-nx-graphics-core-and-direct-skia` — first vertical slice.** Add unstable graphics declarations and the shared types they actually need, plus lowering/validation from evaluated NX external-component values into a host-neutral prepared scene. Support standalone Drawing, Group, Rect, Circle, Ellipse, Line, Polyline, Polygon, Path, and a bounded single-line Text profile; solid fills, strokes, both fill rules, affine transforms, path clips, isolated opacity, and root view-box fit/clip. Publish an explicitly unstable `graphics-core` implementation profile. Include host-supplied font selection, shaped single-line text, and consistent measurement; unsupported multiline/text features fail with diagnostics. Add ordinary NX helpers for regular polygons, stars, filled arrows, and arc/ring sectors. Include an agent preview host returning PNG, revision-linked element bounds, text metrics, and diagnostics; no model/API connection is required to run it. Validate and resolve the entire scene before painting, preserve declaration identity through imports, and reuse existing compiler/runtime APIs; do not change generic NX value serialization to match the UI wire envelope. The prepared scene retains semantic nodes and cached parsed geometry; it is not a flattened NX display list. Traverse it directly to invoke the backend. Introduce the bounded drawing-context contract and a direct Skia adapter, with host-owned surfaces/resources and state restored on failure. A surface can be supplied offscreen for deterministic fixtures or by an in-repository preview host; DrawnUI host integration can then consume the same adapter API. Keep Skia bindings outside compiler/type-system crates; choose the adapter package/language to match the existing host, rather than forcing SkiaSharp types into Rust APIs. This change is done only when an NX example produces an actual image and is rendered by the in-repository preview host, not when declarations alone type-check. Other catalog features must fail explicitly in this profile.
2. **`add-nx-graphics-text-images-and-paints` — complete useful content.** Add linear/radial gradients, resolved image resources, expanded positioned text support (multiline and broader font fallback), and the specified shadows. Extend the host services for fonts, decoding, immutable resource caching, and disposal. Test missing/delayed resources, non-Latin scripts, fallback, effect bounds, and transparent gradients. Resource preparation is asynchronous if needed; painting consumes a prepared snapshot. Unresolved content must follow explicit placeholder/failure policy, and a newly available font may require re-preparation. This milestone covers the proposed graphics catalog without implementing general UI layout.
3. **`add-nx-graphics-recording-and-portability` — prove the abstraction.** Implement an optional typed in-memory NX recorder and replay through the same drawing context; compare against direct Skia. Add a genuinely independent backend, preferably Canvas 2D, using offscreen isolation where required. The second backend may begin on the core profile, with full-catalog portability claimed only after text/paint/resource coverage also passes. Keep full-frame recording immutable with explicit resource lifetimes and semantic companion data. Stabilizing a transportable encoding is a later task, conditional on a concrete host need.

The first change's fixtures must exercise transform order (a translated/scaled shape with known expected coordinates), view-box letterboxing/cropping, even-odd path and clip holes, overlapping children under group opacity, rounded rectangles, scaled/dashed strokes, arc paths, malformed geometry, standalone execution with pinned fonts, measured text, parametric-helper edits, and host canvas-state restoration. Check structural/coordinate invariants as well as same-backend image comparisons. Use a small SVG reference corpus for established semantics, documenting deliberate NX differences. Later fixtures compare direct versus replay and two independent backends with stated raster/font tolerances. The acceptance gate also includes editing star point count, arrow endpoints, and arc sweep without hand-editing a path, and revising label placement from measured text bounds. A recorder-only or SVG-only implementation must not be reported as completion of the first direct-Skia milestone.

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

See [the future design register](NX-Drawn-UI-MVP-Object-Model-Future.md) for changes to resolve before broad evangelism. Some require early decisions but do not belong in the first implementation.

### Post-MVP input and interaction components

- `Button`, `Link`, `TextField`, `TextArea`, `CheckBox`, `RadioGroup`, `Select`, `Slider`, `Switch`, `DateTimeInput`, `FilePicker`
- focus, keyboard navigation, pointer/gesture events, actions, command/tool calls, validation, disabled/read-only states, form submission
- dialogs/modals, tabs, disclosure/accordion, menus, tooltips, drag/drop, selection

### Post-MVP Level 3 semantic/data UI

- `Metric`, `Badge`, `Progress`, `Chart` families, `DataTable`, data-bound lists/collections, `Tree`, `Timeline`, `Map`
- `Form`, `Detail`, `Comparison`, `Gallery`, `Calendar`, `Schedule`
- domain catalogs such as `ContactCard`, `ProductCard`, `OrderSummary`, `FlightOption`

### Post-MVP drawing and media

- native arc opcodes (authoring helpers lower arcs to paths in the first change); continuous-corner “squircle” rounding (both exist as DrawnUI `ShapeType` values); rounded/smoothed polylines; compound/reusable geometry definitions; inheritable paint on `Group`
- rich text spans, text-on-path, advanced shaping controls, font resources, text decoration and transform, wrapped text inside drawing coordinates
- masks, arbitrary clip trees, blend/compositing modes, filters, blur/glow, inner shadows
- conic gradients, gradient spread modes, patterns, nine-patch images, tint/color filters
- animation, transitions, timelines, morphing, skeletal/vector animation, video, audio
- hit testing, gestures, canvas camera/pan/zoom, scene virtualization, accessibility geometry
- 3D, custom shaders, and renderer-specific extension profiles

### Post-MVP GenUI/runtime capabilities

- literal-or-bound property values, state model, JSON Pointer references, conditions, repeats, templates, and named slots
- actions/events and live AG-UI/A2UI/MCP transport adapters (the static A2UI conversion prototype is specified in §4.6)
- incremental surface operations and RFC 6902 JSON Patch streaming
- catalog negotiation, custom catalogs, themes/design tokens, capability fallbacks, migrations
- localization, bidirectional text policy, logical (`start`/`end`) insets, responsive breakpoints, environment values
- per-element extension metadata under a reserved namespace, as A2UI v1.0 added, and the forward-compatibility policy that would make it safe

## 12. Decisions to validate in a prototype

The graphics-first implementation profile is specified by [the initial OpenSpec change](../openspec/changes/archive/2026-10-29-add-nx-graphics-core-and-direct-skia/proposal.md). The complete catalog below remains a design target, not a claim that every effect ships in that change. The consolidated NX listings are illustrative proposal declarations; production declarations must use current NX syntax and pass the actual compiler tests. The initial `@nx/graphics` library must not import `@nx/ui`: sharing types must not require importing UI declarations or a layout engine.


1. **Flat canonical form:** measure generation error rate and streaming ergonomics against nested authoring syntax.
2. **Component count:** confirm `Box` and `Card` both earn their place; `Card` is the likelier convenience component to retain because GenUI models recognize it well.
3. **Layout naming and containment:** use canonical `Row`/`Column` and retain `ZStack`; test generation with nested grids and verify that models treat row/column containers as cell content, not track declarations. Do not introduce aliases without evidence of a usability need.
4. **Gradient coordinates:** verify that `objectBoundingBox` defaults produce the most intuitive model output across Skia and SVG.
5. **Text parity:** quantify layout differences between Skia/DrawnUI and web/native renderers, especially `lineHeight`, weight, and ellipsis.
6. **Path safety:** establish practical complexity and bounds limits without rejecting normal illustrations.
7. **Path compatibility:** retain SVG-compatible black Path fill; test explicit `fill: none`, subpath closure, arc normalization, dash behavior, and clip fill rules against reference fixtures. Reconsider defaults only with generation evidence.
8. **Common-property surface:** `UiCommonProps` puts roughly twenty properties on every Level 2 component, including leaves. A2UI's `ComponentCommon` carries only `id`, `accessibility`, and `weight`, leaving all decoration to the host theme. Measure whether decoration (`background`, `border`, `cornerRadius`, `shadows`, `clip`) belongs on every component or only on the containers — `Box`, `Card`, `Row`, `Column`, `ZStack`, `Grid`, `Drawing` — with leaves keeping only sizing, spacing, alignment, opacity, and accessibility.
9. **Theme contrast:** verify inherited text/icon foregrounds, explicit foreground/background overrides, dark themes, and nested cards. Keep named color tokens/design-token serialization as the next theme extension; do not overload CSS color strings with undocumented token names.
10. **Encoding cost:** the flat map repeats an ID for every element and pays JSON's punctuation everywhere. OpenUI Lang's premise is that this is expensive enough to justify a purpose-built surface syntax. Measure NX's tokens-per-rendered-element against a nested form and against a compact form before the flat map is locked in as the canonical *wire* representation; it can remain the canonical *validation* representation regardless.
11. **Catalog schemas:** publish JSON Schema for `nx.ui@0.1.0` and `nx.graphics@0.1.0`, then run constrained-generation evals.
12. **DrawnUI adapter:** implement the standalone graphics-first milestone in §9.6 before the mixed UI example; use a supplied host surface without requiring Level 2 layout.
13. **Drawing interface:** compare direct Skia with record/replay using the same layout and resources. Measure frame time, allocations, retained resource memory, list size, and replay cost on text-heavy cards, repeated paths, and nested clips/opacity layers. Include recording and serialization costs in the comparison.
14. **Backend independence:** implement the representative graphics corpus on a second immediate backend and identify missing semantics before freezing the interface.
15. **Replay environment:** establish font/image packaging, cache invalidation keys, and semantic-tree synchronization. A viewport, theme, font fallback, resource revision, or device-scale change must invalidate affected layout/recordings; replay is not responsive relayout.

16. **A2UI adapter profile:** implement static Basic v0.9.1 fixtures and diagnostic cases from §4.6; include a negative export of NX graphics to a Basic-only host. Keep full interaction/catalog coverage as a later acceptance milestone.

## 13. Recommended MVP acceptance test

The MVP is credible when one unchanged validated document can render through both:

- a DrawnUI/Skia renderer; and
- a web renderer using normal layout plus SVG or Canvas for `nx.graphics`.

For painted output, additionally compare direct Skia with NX-recorded output replayed through Skia using identical fonts and resources. Test overlapping children under group opacity, nested clips/transforms, fallback text, resource disposal, malformed lists, and cancellation/state restoration. Demonstrate the graphics subset on a second immediate backend before declaring the drawing interface portable. Cross-process output acceptance additionally requires portable resource resolution and a synchronized semantic companion tree. These gates are separate from the two semantic-document renderers above.

The test corpus should include a static List in both directions, all six heading levels, a dark-themed nested card with inherited foregrounds, a text/image card, grid dashboard shell, layered composition, clipped image, gradient/shadow illustration, multi-segment path, and accessible decorative/informative images. It should also include intentionally invalid documents for every validation rule.

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

### Rendering interfaces and recorded output

- [AnyRender](https://github.com/dioxuslabs/anyrender), [Linebender Q1 2026 update](https://linebender.org/blog/tmil-25/), and [Vello releases](https://github.com/linebender/vello/releases)
- [Skia picture recording](https://api.skia.org/classSkPictureRecorder.html) and [Flutter Impeller architecture FAQ](https://flutter.googlesource.com/mirrors/engine/+/HEAD/impeller/docs/faq.md)
- [.NET MAUI Graphics](https://learn.microsoft.com/en-us/dotnet/maui/user-interface/graphics/?view=net-maui-10.0)
- [AndroidX A2UI release notes](https://developer.android.com/jetpack/androidx/releases/a2ui) (library releases, distinct from A2UI protocol versions)

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

The same declarations exist on disk under [drawnui-proposal/](drawnui-proposal/), as the three libraries this appendix describes — `core/core.nx`, `ui/ui.nx`, and `graphics/graphics.nx` — for reading the model as NX without the prose interleaved. Their comments carry the per-property meaning and the “NX gap” notes that this appendix and Appendix B state at length, and `core/core.nx` additionally carries the document model from §4, which belongs to neither catalog. This appendix is the excerpt; that directory is the copy a tool can read, and the two must be kept in step.

Three libraries rather than three files in one is forced rather than chosen: a single NX library has a flat name space, so `Text` and `Image` — deliberately present in both catalogs, per §7 — collide with *“Library item 'Text' is defined in multiple files. Use unique names within one library.”* The library boundary is what gives each catalog its own name space, which is the same job the `nx.ui` / `nx.graphics` prefix does in the document format.

Each library checks on its own with `nxlang typegen`, including `graphics`'s cross-library `import { UiCommon } from "../ui"`. Declaring an external component that extends an imported base is fine; what does not work yet is *using* one at a call site, which is NXE12 and NXE13 below.

Every declaration below was checked with `nxlang typegen` against the NX toolchain in this repository. The one construct that does not check today is the cross-library use shown in §8.1; Appendix B says why.

### `nx/core/types.nx`

```nx
export type Color = string          // CSS Color syntax subset accepted by the host
export type ElementId = string      // ^[A-Za-z_][A-Za-z0-9_.-]{0,63}$

export type Alignment = start | center | end | stretch
export type AlignSelf = auto | start | center | end | stretch
export type AxisAlignment = start | center | end
export type Axis = horizontal | vertical
export type Distribution = start | center | end | spaceBetween | spaceAround | spaceEvenly
export type ObjectFit = contain | cover | fill | none | scaleDown
export type FontStyle = normal | italic
export type GradientUnits = objectBoundingBox | userSpaceOnUse
export type LineCap = butt | round | square
export type LineJoin = miter | round | bevel

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
  top: float64 = 0
  right: float64 = 0
  bottom: float64 = 0
  left: float64 = 0
}

export type CornerRadii = {
  topLeft: float64 = 0
  topRight: float64 = 0
  bottomRight: float64 = 0
  bottomLeft: float64 = 0
}

export type GradientStop = {
  offset: float64                       // 0..1
  color: Color
  opacity: float64 = 1                  // 0..1
}

export type Paint =
  | none
  | solid { color: Color }
  | linearGradient {
      x1: float64
      y1: float64
      x2: float64
      y2: float64
      units: GradientUnits = objectBoundingBox
      stops: GradientStop[]
    }
  | radialGradient {
      cx: float64
      cy: float64
      r: float64
      fx: float64?
      fy: float64?
      units: GradientUnits = objectBoundingBox
      stops: GradientStop[]
    }

export type Stroke = {
  paint: Paint
  width: float64 = 1
  lineCap: LineCap = butt
  lineJoin: LineJoin = miter
  miterLimit: float64 = 4
  dashArray: float64[]?
  dashOffset: float64 = 0
}

export type Shadow = {
  color: Color
  offsetX: float64 = 0
  offsetY: float64 = 0
  blur: float64 = 0                     // >= 0
}

export type Transform =
  | translate { x: float64 y: float64 }
  | scale { x: float64 y: float64? }        // null y means uniform scale
  | rotate { degrees: float64 cx: float64 = 0 cy: float64 = 0 }
  | skewX { degrees: float64 }
  | skewY { degrees: float64 }
  | matrix { a: float64 b: float64 c: float64 d: float64 e: float64 f: float64 }

export type Border = {
  paint: Paint
  width: float64 = 1
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

export type ScrollAxis = horizontal | vertical | both
export type ScrollbarVisibility = auto | visible | hidden
export type CardVariant = outlined | elevated | filled
export type TextFormat = plain | markdown
export type TextVariant = h1 | h2 | h3 | h4 | h5 | h6 | title | body | caption | code
export type TextAlign = start | center | end | justify
export type TextOverflow = clip | ellipsis

export type TrackSize =
  | auto
  | fixed { value: float64 }
  | fraction { value: float64 }

export abstract external component <UiCommon
  width: Length = auto
  height: Length = auto
  minWidth: float64 = 0
  minHeight: float64 = 0
  maxWidth: float64?
  maxHeight: float64?
  margin: Insets?
  padding: Insets?
  alignSelf: AlignSelf = auto
  justifySelf: AlignSelf = auto
  grow: float64 = 0
  shrink: float64 = 1
  gridColumn: int?
  gridRow: int?
  columnSpan: int = 1
  rowSpan: int = 1
  background: Paint = none
  border: Border?
  cornerRadius: CornerRadii?
  shadows: Shadow[]?
  clip: boolean = false
  opacity: float64 = 1
  accessibility: Accessibility?
/>

export external component <Row extends UiCommon
  gap: float64 = 0
  justify: Distribution = start
  align: Alignment = stretch
  wrap: boolean = false
  content children: Element[]?
/>

export external component <Column extends UiCommon
  gap: float64 = 0
  justify: Distribution = start
  align: Alignment = stretch
  wrap: boolean = false
  content children: Element[]?
/>

export external component <Grid extends UiCommon
  columns: TrackSize[]
  rows: TrackSize[]?
  columnGap: float64 = 0
  rowGap: float64 = 0
  justifyItems: Alignment = stretch
  alignItems: Alignment = stretch
  content children: Element[]?
/>

export external component <ZStack extends UiCommon
  justifyItems: Alignment = stretch
  alignItems: Alignment = stretch
  content children: Element[]?
/>

export external component <Scroll extends UiCommon
  axis: ScrollAxis = vertical
  scrollbar: ScrollbarVisibility = auto
  content child: Element
/>

export external component <Box extends UiCommon
  content child: Element
/>

export external component <Card extends UiCommon
  variant: CardVariant = elevated
  content child: Element
/>

export external component <Divider extends UiCommon
  axis: Axis = horizontal
  color: Color?
  thickness: float64 = 1
/>

export external component <Text extends UiCommon
  format: TextFormat = plain
  variant: TextVariant = body
  color: Color?
  fontFamily: string?
  fontSize: float64?
  fontWeight: float64?
  fontStyle: FontStyle = normal
  textAlign: TextAlign = start
  lineHeight: float64?
  letterSpacing: float64 = 0
  maxLines: int?
  overflow: TextOverflow = clip
  content text: string
/>

export external component <Image extends UiCommon
  source: ImageSource
  alt: string
  fit: ObjectFit = contain
  aspectRatio: float64?
/>

export external component <Icon extends UiCommon
  name: string
  set: string?
  size: float64?
  color: Color?
/>

// §7.13 — static sequence; template/data binding support is post-MVP.
export external component <List extends UiCommon
  direction: Axis = vertical
  gap: float64 = 0
  align: Alignment = stretch
  scrollbar: ScrollbarVisibility = auto
  content children: Element[] = []
/>
```

### `nx/graphics/catalog.nx`

```nx
import "../core"
import { UiCommon } from "../ui"

export type FillRule = nonzero | evenodd
export type ViewBoxFit = contain | cover | fill
export type TextAnchor = start | middle | end
export type DominantBaseline = auto | middle | hanging

export type ContentAlignment = {
  x: AxisAlignment = center
  y: AxisAlignment = center
}

export abstract external component <GraphicsElement
  opacity: float64 = 1
  transform: Transform[]?
  clipPath: string?
  clipRule: FillRule = nonzero
  accessibility: Accessibility?
/>

export abstract external component <Shape extends GraphicsElement
  fill: Paint?
  stroke: Stroke?
  fillRule: FillRule = nonzero
  shadows: Shadow[]?
/>

export external component <Drawing extends UiCommon
  viewBox: ViewBox
  fit: ViewBoxFit = contain
  contentAlignment: ContentAlignment?
  content children: Element[]?
/>

export external component <Group extends GraphicsElement
  content children: Element[]?
/>

export external component <Rect extends Shape
  x: float64 = 0
  y: float64 = 0
  width: float64
  height: float64
  rx: float64 = 0
  ry: float64?                          // null means "same as rx"
/>

export external component <Circle extends Shape
  cx: float64
  cy: float64
  r: float64
/>

export external component <Ellipse extends Shape
  cx: float64
  cy: float64
  rx: float64
  ry: float64
/>

export external component <Line extends Shape
  x1: float64
  y1: float64
  x2: float64
  y2: float64
/>

export external component <Polyline extends Shape
  points: Point[]
/>

export external component <Polygon extends Shape
  points: Point[]
/>

export external component <Path extends Shape
  data: string
/>

export external component <Text extends GraphicsElement
  x: float64
  y: float64
  fontFamily: string?
  fontSize: float64 = 16
  fontWeight: float64 = 400
  fontStyle: FontStyle = normal
  textAnchor: TextAnchor = start
  dominantBaseline: DominantBaseline = auto
  letterSpacing: float64 = 0
  lineHeight: float64 = 1.2
  fill: Paint?
  stroke: Stroke?
  shadows: Shadow[]?
  content text: string
/>

export external component <Image extends GraphicsElement
  x: float64 = 0
  y: float64 = 0
  width: float64
  height: float64
  source: ImageSource
  alt: string
  fit: ObjectFit = contain
/>
```

## Appendix B — Where NX syntax does not yet fit this model

The same findings are tracked as numbered NX language items in
[drawn-ui-proposal-nx-enhancements.md](drawn-ui-proposal-nx-enhancements.md), with reproducers and
suggested enhancements; this appendix states them from the object model's point of view.

NX turns out to fit this proposal better than expected in one important respect and worse in another, and the two are worth separating.

The fit is `external component`. A catalog entry is precisely a component signature with no NX body, rendered by a host — which is what `external component` already means, down to `abstract external component` for shared property bundles and `extends` for reuse. The `UiCommonProps`/`GraphicsElement`/`Shape` structure in §6.1 and §7.1 was designed before this was known and maps onto it without adjustment. NX also gets the `content` property right: the wire format's `children` is a declared property with a declared type, so a container that accepts many children (`content children: Element[]?`) and one that accepts exactly one (`content child: Element`) are different signatures rather than a prose rule.

The misfit is the shape of the data. What follows are the specific gaps, in rough order of how much they cost.

**No map type, and no generics.** `elements: Record<ElementId, Element>` has no NX spelling; NX has records, sequences, and unions, and generic type parameters are a post-1.0 item. The listing uses `Element[]`, which is not the same type — it loses key-uniqueness and O(1) ID lookup, and it forces `id` onto the element rather than onto the map key, which is why §4.1 keeps the map in the table and flags the divergence. It costs less than it looks: `elements` is the *only* map-typed site in the model, and NX has no occasion to author it, because NX source is the nested form and the flat map is what a compiler emits. The gap becomes real for the post-MVP catalogs in §11 — a state or data model, themes and design tokens, localization tables, per-element extension metadata — each of which needs a map-typed property inside a catalog.

**No literal types, so no compact scalar-or-structured values.** NX union cases are named, so a union cannot mix a bare *numeric* scalar with an alternative shape. Two MVP types depend on exactly that: `Length` (`120` / `"auto"` / `"50%"`) and `Insets`/`CornerRadii` (a scalar meaning all sides). The listing spells `Length` as a three-case union and drops the scalar shorthands entirely, which is why `"padding": 20` becomes a four-field record in §8.1. String-literal union types are on NX's roadmap as a 1.1 feature; if they arrive, `Length` and `Insets` should be revisited, because the shorthand is not cosmetic — it is most of the token cost of a generated layout.

`Paint | "none"` is no longer in that list. A fieldless case of a base-less union serializes as its bare name, so `Paint.none` is `"none"` on the wire and the union already mixes a string with a record shape. The source spelling is a case name rather than a string literal, which is the only remaining difference and is not one the wire format sees.

**Every record and every *payload* union case carries a `$type` tag.** NX's serialization gives each record a `$type` discriminator and each case that carries fields a qualified one, so a `Point` is `{"$type": "Point", "x": 10, "y": 20}` and a solid fill is `{"$type": "Paint.solid", "color": "#fff"}` rather than `"#fff"`. The key is fixed and not configurable.

A case that declares no fields, in a union that declares no base, is the exception: it serializes as its bare name. `Paint.none` is `"none"`, and `align: Alignment` is `"center"` — no wrapper at all. That covers every type in Appendix A's first block, which is most of the model's closed-set properties. What remains heavy is the record and payload-case encoding, and that is still the most consequential difference for a format whose §12.10 open question is token cost: the NX-native encoding is heavier than the JSON in §8, so NX source and NX serialization remain separable decisions. Adopting the language does not oblige the wire format to adopt its encoding.

**Nullable is the only way to say "optional."** NX has no absent-versus-null distinction, so a property with no meaningful default is written `T?` and the renderer resolves null. Most of the `?` marks in §6 and §7 exist for this reason rather than because null is a meaningful value.

**A derived component cannot override an inherited default.** Redeclaring an inherited property is rejected as a duplicate. §6.1 wants `fill` to default to `Paint.none` on `Line` and `Polyline` and to black on closed shapes; that is not expressible, so `fill` is nullable on `Shape` and the per-component default becomes renderer prose.

**A default cannot reference a sibling property.** `Transform.scale.y` defaulting to `x`, and `nx.graphics.Rect.ry` defaulting to `rx`, both become nullable with the relationship stated in a comment. (Component `state` defaults *can* read props; property defaults cannot.)

**A default cannot be an empty sequence.** `= []` and `= {}` are both rejected — a braced default must contain at least one item, and bracket-list literals are not accepted in default position at all. Every `T[]` property whose default is "empty" is therefore written `T[]?`. This is filed against NX as the `empty-list-spelling` change: an empty list has no source spelling at all, not merely none in default position, so it also cannot be round-tripped by the formatter.

**A default cannot be a *qualified* case name.** `= Alignment.start` is a syntax error and must be braced as `= {Alignment.start}`. The bare forms are fine and are what Appendix A uses: `= -1.0` and `= start` both parse, because an unbraced value position takes a literal and a bare name resolves against the property's declared type. An integer literal at a `float64` property is fine: `= 0` and `= 0.0` are accepted on the same terms, and Appendix A writes the shorter form.

**No refinement or pattern constraints.** `ElementId`'s regex, `opacity`'s 0..1 range, `fontWeight`'s 1..1000, `columnSpan >= 1`, and "at least two gradient stops" are all invisible to NX's type system. They stay in the JSON Schema, which means the schema — not the NX declarations — remains the normative validator. The NX listing is the shape; the schema is the contract.

**A union case cannot be reached through a wildcard import alias.** With `import "../ui" as ui`, the type `ui.TextVariant` resolves but `ui.TextVariant.h2` fails with *"Member access not yet implemented"* — the third qualifying segment is not supported. This no longer bites at a typed site: contextual literal binding lets `variant=h2` resolve against the property's declared type without naming the union at all, so §8.1 uses the same import style for both catalogs. The limitation remains for a case named inside a braced expression, where `ui.TextVariant.h2` is still the only spelling and still fails.

**Qualification is one segment deep.** `import "../ui" as ui` takes a single identifier, and the selective form is validated to contain *exactly* one dot; `import "../ui" as nx.ui` is a syntax error and `import { Column as nx.ui.Column } from "../ui"` is rejected with "Selective import alias must contain exactly one dot." A two-segment catalog prefix is therefore not reachable in NX source. `nx.ui.Column` remains the catalog type name in the document format; in NX it is written under a one-segment local alias, and the mapping from alias to catalog ID is the `catalogs` list.

**Structure is nested, and cannot be flat.** This follows from the first gap but is worth stating on its own: NX has no way to write the canonical flat element map, so NX is an authoring syntax that a compiler lowers into it. §3 already anticipated a recursive authoring form as sugar over the flat model; NX is that form.

Two further items are implementation gaps rather than language gaps, and both were reproduced against the toolchain in this repository:

- **Prop defaults on an imported external component are not applied at the call site.** Given `lib/a.nx` with `export external component <Rect extends Base x: float64 = 0 y: float64 />` and `app/main.nx` with `<lib.Rect y={5.0} />`, analysis reports *"Element 'lib.Rect' requires property 'x'"*. The same declarations in one library check cleanly.
- **Props inherited from an imported abstract external base are not visible at the call site.** With the same two libraries, `<lib.Rect ... width={2.0} fill={"red"} />` reports *"Element 'lib.Rect' has no property 'width'"* and the same for `fill`, though both are declared on `Base`.

Together these make the three-library layout in Appendix A uncheckable today: it is valid NX, and each library checks on its own, but a document that imports two catalogs cannot yet be type-checked against them. A single-library variant of the same catalog plus the §8.1 document does check end to end. Both should be filed against the NX toolchain before an NX-authored catalog is committed to.

Finally, one convention clash worth a decision rather than a fix: NX documents `snake_case` as the case-name convention and serializes case names verbatim, while this model's closed-set values are camelCase (`spaceBetween`, `objectBoundingBox`, `scaleDown`) because they follow CSS and SVG. camelCase case names are legal NX identifiers, so Appendix A keeps the wire spelling and diverges from the convention. Changing the wire format to satisfy an NX style rule would be the wrong trade.

---

### Short form of the proposal

Use a cataloged flat document, borrow structure from A2UI/json-render, borrow graphics vocabulary and data formats from SVG, preserve DrawnUI's composability, and keep renderer mechanics out of the generated model. Ship read-only layout/content plus a compact vector scene graph with pluggable direct drawing first; prove optional NX recording/replay through the same interface, then stabilize portable recorded output when its resource and text contracts are ready. Add state, interaction, semantic components, and richer effects as separately versioned layers.


## Delivered graphics-core profile

The unstable standalone `@nx/graphics` subset, math-backed helpers and direct Skia preview are
implemented for Rust and TypeScript IR runtimes. See the
[production graphics reference](../sites/website/src/content/docs/reference/libraries/graphics.md)
for supported declarations, commands, defaults, limits and explicit unsupported features. The
larger catalog in this proposal remains a future design. The legacy HIR interpreter will not be
updated for graphics support; its retirement is a separate change.
