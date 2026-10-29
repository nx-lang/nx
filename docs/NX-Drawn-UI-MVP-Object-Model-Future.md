# NX graphics/UI: decisions before compatibility stabilization

**Status:** Proposed design register, October 27, 2026. These are not implemented features or scheduled commitments.

The [MVP proposal](NX-Drawn-UI-MVP-Object-Model-Proposal.md) and [initial OpenSpec change](../openspec/changes/archive/2026-10-29-add-nx-graphics-core-and-direct-skia/proposal.md) define the immediate work. This register covers changes to evaluate before broad evangelism makes syntax, serialization, and rendering behavior expensive to revise. Keep initial libraries/profiles explicitly unstable. Implementing every item is not a prerequisite for experimentation; each release must clearly identify its supported subset.

## 1. Decide the authoring/rendering boundary

The initial graphics change depends on [add-math-standard-library](../openspec/changes/archive/2026-10-28-add-math-standard-library/proposal.md). Standard math uses radians; graphics authoring keeps explicit degree conventions and conversions. Numerical behavior and runtime support are shared NX contracts, not private graphics approximations.

Keep ordinary NX components/functions expressive while portable scenes remain literal, bounded geometry. Test the initial regular-polygon/star/arrow/arc helpers and promote only useful ones. Add rounded polygons, routed connectors, anchor/alignment helpers, repeated motifs, and semantic diagram components as libraries before adding native renderer opcodes. Compare helpers against primitive-only generation using fixed agent tasks and budgets. Resolve the public path-builder API, helper parameter/default conventions, and whether authoring components retain named identity after expansion before declaring stable authoring APIs.

**Gate:** agents can revise a requested dimension, shape parameter, and label placement without reconstructing unrelated geometry. Helper output retains useful provenance. Pure evaluation of source expressions does not become execution of serialized drawing callbacks.

## 2. Fix identities, metadata, and source provenance

Separate authored IDs, generated scene-local IDs, source occurrence identity, and persistent foreign-tool IDs. Decide how users name groups/elements without confusing identity with an accessible label. Reserve an adapter-side metadata/sidecar schema for Figma node IDs, original parametric shapes, and editability losses. Never let foreign metadata change rendering or execute code. Specify lifecycle, revision checking, reparenting/reordering, helper expansion, and many-to-one origins. Round trips need edit history/conflict rules rather than only a map of IDs.

**Gate:** a patch cannot accidentally update a different element after structural edits; unchanged structure produces deterministic preview IDs, and durable-ID guarantees are stated separately.

## 3. Resolve NX source, evaluated values, and wire formats

The proposal's JSON shorthands and NX-native record/union encodings are not identical. Specify one intentional projection, omission/default rules, discriminators, qualified component identity, and flat versus nested forms before publishing stable schemas. Revalidate Appendix B's historical syntax gaps against the current compiler; some have already been addressed by optional-property/sequence work. Do not enshrine stale limitations as design constraints.

Decouple graphics contracts and shared values from Level 2 UI declarations. The initial standalone Drawing must work without UI measurement. Resolve the long-term composition of Drawing sizing/decorations with UI placement, including which props can be used without a parent. Maintain a schema/capability source of truth and verify generated NX/JSON/host contracts agree.

**Gate:** alias-renamed imports, user components and generated helpers identify the same catalog declarations; JSON/native-value round trips preserve semantics with no dependency on a checkout path.

## 4. Complete the paint and geometry extension design

The simple fill/stroke model is useful for MVP rendering but leaves common design-tool content difficult to exchange. Before freezing it, design:

- Ordered paint layers with per-layer opacity/blending, without duplicating whole nodes and changing group effects.
- Affine gradient transforms with explicit coordinate spaces and matrix order; linear/radial first, angular/diamond gradients only through declared extensions.
- Image paints with crop/tile transforms, independent of rectangular image content.
- Inside/outside strokes on defined closed geometry; open/self-intersecting path policy; optional non-scaling strokes/hairlines; arrow endpoints.
- Independent rectangle corner radii and a deliberate policy for corner smoothing.
- Boolean geometry, reusable geometry resources, and optional parametric arc/star/polygon representations where preserving editability earns the complexity.

These remain separate from a full Figma vector-network editing graph. Paths can preserve appearance while losing branch topology, boolean operands, or parameter handles. Distinguish appearance fidelity from editability in adapter reports. Add fixtures for paint order, transparency, bounds, and lowering before selecting encodings. Keep core SVG-compatible semantics where practical; document deviations instead of inheriting backend defaults.

## 5. Stabilize text and resources

Extend the initial single-line host-font profile with explicit line breaking/line height, direction/language, fallback chains, rich runs, decoration, variable/color fonts, and eventually text-on-path. Determine how font identity, licensed asset packaging, variation coordinates, shaping configuration and logical/ink/visible bounds are represented. Measurement and painting must consume the same layout, not merely similarly configured APIs. Do not outline text silently in export: that loses editability and accessible content.

Define immutable resource identities, versions, resolver lifetimes, asynchronous preparation, cancellation, cache invalidation, and placeholder behavior. Host-approved assets remain distinct from model-supplied arbitrary URLs.

**Gate:** font/resource replacement invalidates affected prepared output, text metrics are reproducible under pinned resources, and missing glyphs/resources are diagnosable.

## 6. Theme, layout, accessibility, and semantic content

Choose semantic color/token encoding and content-foreground inheritance before users hard-code palettes. Evaluate Design Tokens Community Group compatibility rather than inventing an unrelated token format. Keep literal CSS colors distinct from token references. Specify spacing, theme override precedence, responsive/bidirectional behavior, constraints, and Card nesting defaults.

Extend semantic UI with Badge, Progress, DataTable and chart profiles; keep component names design-system-neutral. Define list/item roles, headings, image alternatives, graphical group descriptions, and accessible semantic output paired with rendered revisions. A graphics scene and an accessibility tree serve different purposes; neither can be recovered reliably from a display list alone.

**Gate:** themes and viewports change without destroying meaning, and rendering preserves accessible content even when the visual backend changes.

## 7. Recording, backend portability, and interoperability

Prove direct Skia first, then an optional recorder and a second backend. Freeze neither a low-level command set nor a portable byte format until layers, text, resources, and failure restoration work independently of Skia. Benchmark direct rendering versus recording, including allocations, preparation, serialization, resource memory and replay.

Track versioned A2UI and Adaptive Cards subsets, a json-render registry, and Figma graphics import/export separately. Distinguish snapshot export from retained runtime behavior. Report exact mapping, equivalent lowering, approximation, editability loss and unsupported constructs. Figma vector networks, multi-paint shapes, masks, blend groups, rich text, auto layout, components/instances and variables need distinct coverage decisions. Preserve foreign editing metadata in an adapter layer; do not promise arbitrary Figma-file round trips from SVG interchange.

MCP Apps is a hosting target and AG-UI a transport/runtime integration. Neither should dictate geometry names. Keep remote UI documents, executable runtime representations and resolved display lists separate, including when investigating Remote Compose.

**Gate:** unsupported content never silently disappears; exported versions and host capabilities are explicit; reference fixtures run on independent implementations.

## 8. Graduation criteria

Before calling graphics/UI stable or broadly promoting compatibility:

1. Publish exact supported profiles, explicit unstable areas, and a version/migration policy.
2. Reconcile illustrative declarations, production standard libraries, validation schemas, docs and example encodings.
3. Pass geometry, text/resource, accessibility, failure-restoration and second-backend tests for the claimed profile.
4. Run representative agent authoring/revision evaluations and document limitations; do not mistake syntactic validity for design quality.
5. Close or explicitly defer the encoding/identity/paint/text decisions above, with reserved extension/version boundaries for deferrals.

Prioritize hard-to-change public contracts over adding every feature. A missing effect can be introduced later; ambiguous identity, paint semantics, or serialization spreads into every tool and document.
