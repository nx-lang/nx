# NX Display List Model

**Proposal status:** Draft for discussion
**Research current through:** September 2, 2026
**Scope:** The renderer-facing rendering program — one resolved frame, plus the dynamic values an animation may change between frames
**Explicitly excluded from this version:** shader effects, geometry animation, dirty rectangles, a packed binary encoding, 3D transforms, and display-list instancing
**Consolidated NX source:** [displaylist-proposal/](displaylist-proposal/) — the whole proposed model as NX and nothing else, for reading without the prose around it

## Executive recommendation

NX should define its display list as a **flat, immutable rendering program over side tables**: a linear op stream that indexes into append-only resource tables, a parent-linked spatial tree that supplies transforms, and a small slot table holding the values an animation changes. The op stream carries no state of its own — no current transform, no current paint, no attribute stream — so any op can be interpreted in isolation.

Three decisions distinguish this from a direct transcription of [nx-display-list-animation-architecture.md](nx-display-list-animation-architecture.md), and each follows from what current renderers actually do:

1. **Transforms live in a spatial tree, not a save/restore CTM stack.** Ops name a spatial node; the node tree supplies the matrix. This is WebRender's design and Vello's, and it is what lets a backend resolve every transform in one pass and then batch or reorder draws. It also makes subtree animation work without wrapping anything.
2. **The push/pop stack carries clips and layers only.** With transforms out of it, the stack has one job, and Vello's `push_layer`/`pop_layer` is the precedent.
3. **Dynamic values are whole-entity bindings on exactly three things** — a spatial node's transform, a layer's alpha, and a solid brush's color — carrying a fallback value in the same record. This is WebRender's `PropertyBinding`, and it is not a coincidence that it is the same set: those are the properties a compositor can change without re-rasterizing.

Specifying it in NX is worth doing for a reason beyond expressing the format: `nxlang typegen` emits the TypeScript types for the Canvas2D renderer and the C# or Rust types for a native one from a single declaration, so the two runtimes cannot drift. That claim was tested — see §13.

---

# 1. Purpose and relationship to the other documents

Three documents now describe adjacent layers, and they should be read as a stack.

| Document | Layer | Question it answers |
|---|---|---|
| [NX-Drawn-UI-MVP-Object-Model-Proposal.md](NX-Drawn-UI-MVP-Object-Model-Proposal.md) | authoring | What is this UI? |
| [nx-display-list-animation-architecture.md](nx-display-list-animation-architecture.md) | architecture | How should the layers be arranged? |
| this document | rendering | How should one resolved frame be drawn? |

The architecture note is the source of this proposal's shape, and its central claims hold up against how shipping renderers are built. This document supplies the part it left as an inventory: the actual types, with the ambiguities resolved.

**Everything here is already resolved.** By the time a display list exists, layout has run, inheritance has been applied, `Length` percentages have become numbers, view boxes have been mapped, SVG transform lists have become matrices, object-bounding-box gradient coordinates have become user-space coordinates, and elliptical arcs have become cubics. A display list never needs the bounds of a shape it has not drawn yet.

That resolution is the interesting part of the contract, and §6 flags each place it happens. It is also the reason two types that look shared with the object model are not: see §3.2.

**Terminology.** §1 of the object model bans renderer vocabulary — `Canvas`, `draw`, `paint` — from the portable authoring model. That ban is lifted here. This layer *is* the renderer boundary; brushes, strokes, layers, and blend modes are the right words, and using anything else would be a euphemism.

# 2. What other systems do

Every claim in this section was checked against a primary source; the sources are listed at the end of the document.

| System | Command model | Transforms | Paint | Animation without rebuilding | What NX takes |
|---|---|---|---|---|---|
| **Flutter `DisplayList`** | Linear op stream with a *stateful attribute stream* — `setColor`, `setStrokeWidth`, `setBlendMode` mutate a current paint that later draw ops consume | `save`/`restore` CTM stack, plus `translate`/`scale`/`rotate`/`transform2DAffine` ops | Attribute state, not per-op | Not in the display list; the layer tree above it | `drawDisplayList` for nesting; the op inventory; `saveLayer` semantics |
| **Chromium `cc::PaintOpBuffer`** | Linear op buffer; ops store paint **by value** in `PaintFlags` on `PaintOpWithFlags` | `kSave`/`kRestore`/`kConcat`/`kSetMatrix` CTM stack | Per-op, by value | Property trees above the buffer | `kDrawRecord` nesting; the `kDrawTextBlob` vs `kDrawSlug` split |
| **WebRender** | Flat display item list inside a tree of stacking contexts | **Spatial tree** of reference frames; items name a `SpatialId`, and `SpatialTree::update()` composes to-screen matrices each frame | Per-item | `PropertyBinding<T>` = `Value(T)` \| `Binding(key, fallback)`, on transforms, opacity, and rectangle color | The spatial tree; the binding shape; the restriction of bindings to those three properties |
| **Vello** | `Scene` with `push_layer`/`pop_layer` and `fill`/`stroke`/`draw_image`/`draw_glyphs`; encodes to parallel GPU streams (path tags, path data, draw tags, draw data, transforms, styles) processed by monoid prefix scans | **Per-draw `Affine`, no CTM stack** — "the transforms are *not* saved or modified by the layer stack" | Brush per draw, with an optional brush transform | Scene is re-encoded per frame | Per-draw transforms; fill and stroke as separate ops; the layer-only stack |
| **Skia** | `SkPicture` / `SkRecord`; `SkCanvas` API | CTM stack | `SkPaint` per draw | — | `saveLayer`, `SkColor4f`, the raw path verb/point iterator |

Two observations shaped this proposal more than the rest.

**The state-threading question is settled by what comes next.** Flutter's stateful attribute stream is smaller to record and perfectly correct, but every op's meaning depends on all ops before it. Chromium and Skia carry paint per op instead. Vello goes further and carries the *transform* per draw too. The direction of travel is toward ops that can be read in isolation, and the reason is that GPU-compute rasterization wants to process a scene with parallel prefix scans rather than a sequential interpreter. An op stream whose meaning depends on a running state machine cannot be scanned in parallel; one that indexes side tables can.

**WebRender independently converged on the animation design the architecture note proposes.** `PropertyBinding` is exactly the constant-or-slot idea, it carries a fallback in the same value, and — the part worth copying — it is available on transforms, opacity, and color, and nowhere else. That is direct evidence for the restriction agreed in §8 rather than a guess.

# 3. Shape of the model

```text
              Retained render tree
                       |
                       | display-list compiler: resolve, intern, index
                       v
        +--------------------------------------+
        |        DisplayListDocument           |
        |                                      |
        |  lists[]      linear op streams      |  immutable
        |  spatial[]    parent-linked tree     |  immutable structure
        |  resources    brushes, paths, ...    |  immutable, append-only
        |  slots        floats, colors, xforms |  MUTABLE
        +--------------------------------------+
                       |
          +------------+------------+
          v                         v
    Canvas2D renderer        Skia / wgpu renderer
```

Everything except the slot table is immutable once sealed. The slot table is the only part a renderer ever writes, and the animation engine is the only thing that writes it (§9).

## 3.1 Why a document rather than a bare list

Resources, the spatial tree, and slots live on the document rather than on each list so that regenerating one sub-list cannot renumber anything a sibling references, and so a single slot can drive values used by several lists. Tables are append-only for the same reason. This is what makes §10's selective regeneration a local operation instead of a global renumber.

## 3.2 Why the library imports nothing

The NX library is self-contained. It does not import the object model's `core`, and that reverses the recommendation made when this work was scoped.

Six scalar types are near-duplicates of `core` declarations — `GradientStop`, `LineCap`, `LineJoin`, `FontStyle`, `ImageSource`, and `Color`. Sharing them looks obviously right until you look at the other end of the import: `core` also carries `NxUiDocument`, `Length`, `Alignment`, `Distribution`, and the rest of the document model, none of which has meaning at the rendering layer. A Rust backend crate generated from this library should not have to carry a UI document model in order to draw a rectangle.

Two of the six are not duplicates at all:

- **`Color`** is a CSS color string in `core` and a four-float sRGB record here. At the authoring layer a string is right — it is what a model generates and a human reads. Here it is wrong: every backend wants floats, and a color slot has to *interpolate*, which is not defined on strings.
- **`Paint`** does not survive at all. `core`'s defaults to `objectBoundingBox` gradient units, which need the bounds of the shape being painted. A display list has already measured everything, so `Brush` resolves gradients to user space and drops the units field entirely.

Under NX's nominal type identity, same-named types in two libraries are distinct types anyway, which is the correct outcome here rather than an annoyance. The clean fix, if the duplication becomes a maintenance problem, is to split `core` into a values-only sub-library free of the document model and have both layers import that. That is a change to the object model's packaging, not to this one, so it is noted rather than assumed.

# 4. The spatial tree

This is the largest divergence from the architecture note, which proposed `Save` / `Transform` / `Restore` in the Canvas2D idiom.

```nx
type SpatialNode = {
  parent: SpatialId?
  transform: AnimatedTransform
}
```

Every op that paints or clips names a `spatial` node, and its coordinates are in that node's space. A node's effective transform is the concatenation of its ancestors' with its own. `parent` is null on the root and must otherwise be a strictly smaller index, so the tree is acyclic by construction and a single forward pass resolves every effective matrix.

## 4.1 Why not a CTM stack

**It forces sequential interpretation.** With `Save`/`Transform`/`Restore`, op *n*'s meaning depends on ops 0..*n*−1. Nothing can be culled, batched, reordered, or evaluated in parallel until the stack has been replayed. With a spatial tree, a backend resolves the tree once and then every op is independently interpretable — which is the precondition for the sorting and batching a GPU renderer does, and for the parallel prefix scans Vello's pipeline is built on.

**It is worse for animation, not better.** The architecture note's §17 wraps a cached sub-list in `PushTransform(slot)` … `Restore`. That works, but it means the animated transform must be *in the op stream*, so a subtree can only be animated where someone thought to emit a wrapper. With a spatial tree, binding a slot to any interior node moves everything below it — including ops in other display lists — with no display list edited and no wrapper emitted. The §17 example gets simpler, not harder.

**A stack is not actually simpler to implement.** A Canvas2D backend with a spatial tree does `ctx.setTransform(...resolved[op.spatial])` before each draw, against a resolved array computed once per frame. That is fewer moving parts than maintaining a correct save/restore stack, and it removes an entire class of unbalanced-stack bug.

## 4.2 What it costs

A resolved transform per op is a matrix multiply the CTM stack would have amortized, and `setTransform` per draw is a real cost in Canvas2D. Both are addressable: consecutive ops sharing a spatial id are extremely common and trivially skipped, and the resolved array is computed once per frame regardless of op count. The cost is also bounded in a way the stack's is not — a deep tree costs one pass, not one pass per traversal.

## 4.3 Consequence for clips

A clip's geometry is in its own spatial node's space, and it applies to descendant *ops in the stream*, not to descendant spatial nodes. Op nesting and spatial nesting are independent, which is deliberate: a clip established in one space frequently applies to content positioned in another. WebRender separates its clip chains from its spatial tree for the same reason.

# 5. Identity and resource tables

Resources are interned and referenced by index into the table of their type, per the architecture note's §4 and §5. All eight id types are declared, so the intent is legible:

```nx
type BrushId = int32
type PathId = int32
// ...
```

**These do not type-check.** `type PathId = int32` is a transparent alias in NX; nothing stops a `PathId` being passed where a `BrushId` is expected. Rust's `struct PathId(u32)` — the very form the architecture note's §4 proposes — does prevent it. For a format that is almost entirely integer indices, this is the single most valuable language addition, and §12 files it.

Interning is the display-list compiler's job (`paintCache.intern`, in the architecture note's terms). The format only requires that indices be valid and that tables be append-only.

# 6. Value types and where resolution happens

## 6.1 Geometry

`Transform2D` is a resolved affine matrix in the `a b c d e f` column-major order that SVG, Canvas2D, Skia, and kurbo all share — not a list of named operations. A backend that has to interpret `translate`/`rotate`/`scale` is redoing work the producer already did.

## 6.2 Color

Non-premultiplied sRGB, four floats in 0..1. Matches `SkColor4f`, WebRender's `ColorF`, and Vello's `AlphaColor<Srgb>`. Wide-gamut and explicit color spaces are deferred; when they arrive they should extend this record rather than replace it.

## 6.3 Brushes, and fill/stroke as separate ops

A `Brush` describes a fill only. Stroking is a separate op carrying a `StrokeStyleId`, so a shape with both a fill and a stroke lowers to two ops. This follows Vello and matches what every renderer does internally; it also keeps every op to exactly one draw, which is what makes op-level batching possible.

Gradient coordinates are in the user space of the referencing op's spatial node. This is the resolution described in §3.2, and it is the reason `GradientUnits` does not exist here.

## 6.4 Paths

A structured path — a verb sequence plus a flat coordinate stream — not SVG path-data text:

```nx
type PathVerb = moveTo | lineTo | quadTo | cubicTo | close
type Path = {
  verbs: PathVerb[]
  points: float64[]
}
```

`moveTo` and `lineTo` consume one point, `quadTo` two, `cubicTo` three, `close` none. This is Skia's raw iterator representation and the shape Vello's path encoder wants, and it means no backend parses anything in its draw loop. Elliptical arcs are absent: SVG's `A` becomes cubics at lowering, which every rasterizer does anyway, and the five-verb set has no special cases for a GPU path encoder.

The fill rule is on the fill op, not on the path, because the same path may be filled either way.

## 6.5 Text — the one thing that is not resolved

This is the format's weakest portability claim, and it should be stated plainly rather than buried.

A `TextRun` carries **characters, not positioned glyphs**. The backend shapes it. Two consequences follow, and both are real:

- **Identical documents may measure differently on two backends.** The display list is deterministic in every respect except text extent.
- **The backend must apply `anchor` and `baseline`**, because both need measurements the producer does not have.

The alternative — a positioned glyph run, the form Chromium calls a "slug" and distinguishes from a text blob with a separate op — is genuinely portable, but requires the producer to own the font file and a shaper. The lightweight JavaScript runtime that this whole architecture exists to serve cannot pay for that. So: characters now, with `GlyphRun` reserved as a second case for a runtime that can, and the divergence documented rather than denied.

A run holds one line. `\n` is invalid; the compiler emits one run per line, because line breaking needs measurement too.

## 6.6 Images

An `Image` carries its source and intrinsic pixel size; `drawImage` carries resolved source and destination rectangles, so `ObjectFit` resolution has already happened. A renderer that has not finished decoding an image paints nothing for ops referencing it and repaints when it resolves — the display list is not invalidated, because nothing about it changed.

# 7. Commands

Ops execute in order; later ops paint over earlier ones. Push ops open a scope on a single stack that `pop` closes, and the stack must be balanced within each list and within each nested list.

| Group | Ops |
|---|---|
| Scopes | `pushLayer`, `pushClipRect`, `pushClipRoundRect`, `pushClipPath`, `pop` |
| Fills | `fillRect`, `fillRoundRect`, `fillOval`, `fillPath` |
| Strokes | `strokeLine`, `strokeRect`, `strokeRoundRect`, `strokeOval`, `strokePath` |
| Content | `drawImage`, `drawText` |
| Composition | `drawDisplayList` |

Seventeen ops. Rect, rounded rect, and oval get dedicated ops rather than going through `fillPath` because they are the hot cases in UI and every backend has a fast path for them — the same reason Flutter and Chromium both carry them separately.

Geometry is **inlined on each op** rather than referencing a `Rect` record, so `fillRect` carries six numbers instead of six numbers plus a nested `$type` tag. Matching Chromium, whose `DrawRectOp` holds its `SkRect` by value.

`pop` is the only fieldless op, and it serializes as the bare string `"pop"` — verified against `nxlang typegen`.

## 7.1 Layers

`pushLayer` composites its contents as a unit, then blends the result at `alpha` with `blend`, optionally through a `filter`. This is Skia's `saveLayer`, Flutter's, and Vello's `push_layer`.

Blend modes are the separable and non-separable set shared by Canvas2D's `globalCompositeOperation`, `SkBlendMode`, and CSS `mix-blend-mode`. The Porter-Duff operators beyond source-over are deferred: they interact with layer bounds differently across backends, and shipping them without deciding that would make the format's portability claim false.

A drop shadow is a `LayerFilter`, not a property on a fill. That costs an offscreen in the general case, and it is the only lowering that composites correctly when the shadowed thing is a group; Canvas2D may still take its native `shadowBlur` path for the single-shape case, since the result is identical there.

# 8. Dynamic values

```nx
type AnimatedTransform = {
  value: Transform2D
  slot: TransformSlotId?
}
```

`value` is the constant, and also what a renderer uses before the slot is first written. A non-null `slot` says to read the slot table instead. This is WebRender's `PropertyBinding::Binding(key, fallback)` with the fallback kept in the record rather than in a union case — which avoids a `$type` tag on every animatable value, and makes "is this animated?" a single nullable field rather than a two-case union at every site.

## 8.1 Exactly three binding sites

A spatial node's transform, a layer's alpha, and a solid brush's color. Plus `drawImage.alpha`, which exists because an image has no brush to carry its opacity and fading one would otherwise need a layer and an offscreen.

This is the same set WebRender allows, and the reason is not aesthetic: these are the properties a compositor can change without re-rasterizing anything.

## 8.2 Why not a constant-or-slot wrapper on every field

The architecture note's §16 proposes `Constant<T> | Slot<T>` per property. In NX that makes every number a two-case union, so every geometry value serializes as a `$type`-tagged record, and the renderer pays a branch on every field of every op in its innermost loop. The cost lands everywhere; the benefit lands on the few fields that actually animate.

## 8.3 Why geometry is not bindable

The architecture note's §18.2 wants slots on width, corner radius, stroke width, and gradient position. Animating any of those re-rasterizes the affected geometry regardless — a slot changes nothing about that, it only avoids rebuilding the op. Rebuilding one op is cheap; re-rasterizing is not. So the slot buys the cheap half of a cost it cannot avoid, and charges the whole format for it.

Geometry animation is therefore structural in this version: rebuild the affected sub-list (§10). If profiling later shows sub-list rebuilds dominating a real workload, geometry bindings can be added to specific ops without changing anything else — which is the argument for leaving them out now rather than guessing.

## 8.4 The slot table

Sequence lengths declare how many slots exist; the entries are their initial values. Slot writes never change a list's `generation`, which is precisely what lets a backend keep a cached rasterization across an animated frame.

# 9. The animation boundary

Unchanged from the architecture note's §21, which is right: the renderer must not know about easing, springs, durations, delays, repeats, or state transitions.

```text
animation engine  --evaluate at time T-->  slot writes  -->  replay
```

A frame in which only slots changed replays the same lists against a mutated slot table. A frame with a structural change rebuilds affected lists first. A backend distinguishes the two by comparing generations, and needs to know nothing else.

# 10. Nested lists, caching, and selective regeneration

`drawDisplayList` replays another list in the same document, sharing its spatial tree, resources, and slots. It takes no spatial node: the sub-list's ops name their own.

## 10.1 It is for caching, not instancing

Because a sub-list's ops name fixed spatial ids, the same list cannot be drawn at two positions. Instancing would need a spatial rebase — either a CTM, which §4 rejected, or an id remap, which needs a design. WebRender has no nested display lists at all and solves the caching problem with picture caching instead; Flutter's `drawDisplayList` does rebase, because it has a CTM.

This is a real limitation and it is deferred deliberately. What it does *not* cost is the architecture note's §17 payoff: a cached sub-list whose spatial ancestors carry a bound transform still animates as a unit, without instancing and without a wrapper.

## 10.2 Regeneration

The retained tree decides which lists changed and rebuilds those, appending any new resources and spatial nodes. Existing indices stay valid; siblings are untouched; `generation` increments on rebuilt lists only. The MVP may legitimately treat the whole document as one list and rebuild it wholesale — the structure is here so that the optimization does not require a format change later.

# 11. Validation

None of this is expressible in NX's type system, so it lives in the validator — the same split the object model's Appendix B describes, where the NX declarations are the shape and the schema is the contract.

- Every id is a valid index into its table.
- `SpatialNode.parent` is null on exactly one node (the root) and otherwise strictly less than the node's own index.
- The push/pop stack is balanced within every list.
- `drawDisplayList` references form a DAG; `root` reaches every list, and unreachable lists are invalid.
- `Path.points` length matches exactly what `verbs` consumes; the first verb is `moveTo`.
- Gradients carry at least two stops with non-decreasing offsets in 0..1.
- Colors, `alpha`, and `offset` are in 0..1; `width`, `height`, `r`, `rx`, `ry`, and stroke `width` are non-negative; `weight` is in 1..1000.
- `TextRun.text` contains no line terminators.
- Slot ids are within the declared slot-table lengths.
- All resource counts, path complexity, text length, and image dimensions are subject to host limits.

# 12. Backend mapping

| This model | Canvas2D | Skia | Vello / wgpu |
|---|---|---|---|
| resolved spatial transform | `setTransform` | `setMatrix` | per-draw `Affine` — a direct match |
| `pushClipRect` / `pop` | `save` + `clip` / `restore` | `save` + `clipRect` / `restore` | `push_clip_layer` / `pop_layer` |
| `pushLayer` | offscreen canvas, or `globalAlpha` when trivial | `saveLayer` | `push_layer(blend, alpha, …)` |
| `fillPath` | cached `Path2D` + `fill` | `SkPath` + `SkPaint` | encoded path stream |
| `strokePath` | `stroke` with dash state | `SkPaint` stroke style | `stroke(style, …)` |
| `drawText` | `fillText` | `SkTextBlob` | `draw_glyphs` |
| `drawDisplayList` | inline replay | `SkPicture` or inline | `Scene::append` |
| slot write | mutate array, replay | mutate array, replay | re-encode scene |

The Canvas2D column is the whole MVP renderer and it is small — a `switch` over seventeen ops, a resolved-transform array, and lazily built `Path2D` and font-string caches.

The Vello column is the reason for §4 and §6.4. A backend that encodes to parallel GPU streams wants a transform *table*, a style *table*, and structured paths — which is what this format already is, rather than something a CTM-based display list would have to be unwound into first.

# 13. Authoring a display list directly in NX

Per-index authoring is the agreed approach for this version: ids are integers, written literally. A symbolic authoring layer that computes indices from names is a later convenience and does not change the format.

The types are data, not components, so a hand-authored list uses NX's element-style construction of payload union cases and record values. Both the library and the round trip were checked against the toolchain in this repository:

```
$ nxlang typegen docs/displaylist-proposal/displaylist --language typescript -o out/ts
$ nxlang typegen docs/displaylist-proposal/displaylist --language csharp    -o out/cs
```

Both succeed. The TypeScript output is the renderer's types verbatim, including `Op` as a discriminated union with `"pop"` as a bare string literal member — confirming that the one fieldless op needs no wrapper on the wire. The C# output turns each constant union into a real enum with a wire-format converter.

This is the concrete payoff: the Canvas2D renderer's types and a native renderer's types are generated from one declaration and cannot drift.

The C# run reports thirteen warnings of the form *"omitted default for 'Op.pushLayer.blend' because only literal defaults can be emitted as property initializers"* — every default whose value is a union case name is dropped. That is a toolchain gap, filed in §14 as NXE20, and it is not merely cosmetic: see the analysis there.

# 14. NX gaps this model surfaces

Numbered continuing from [drawn-ui-proposal-nx-enhancements.md](drawn-ui-proposal-nx-enhancements.md), which this list should be merged into.

**NXE19 — No nominal newtype over a primitive.** `type PathId = int32` is a transparent alias, so the eight id types provide documentation and no checking. A format built almost entirely from integer indices is exactly the case that wants it, and Rust's newtype pattern is what the architecture note itself proposes. Highest-value addition for this model by a wide margin.

**NXE20 — C# typegen drops union-case defaults, and the result is silently wrong.** Every property whose default is a union case name — thirteen of them in this library, including `extend: Extend = pad`, `cap: LineCap = butt`, `blend: BlendMode = srcOver`, and `sampling: Sampling = linear` — emits a warning and no property initializer, so the C# property falls back to `default(TEnum)`, which is the enum's *first* member.

That is correct by luck in twelve of the thirteen cases, because the declared default happens to be the union's first case. It is wrong in the thirteenth: `Sampling = nearest | linear` with `sampling: Sampling = linear` generates `public Sampling Sampling { get; set; }`, and `default(Sampling)` is `Nearest`. A C# renderer built from these types silently point-samples every image and every image brush.

Reproduced against this repository's toolchain. TypeScript is unaffected — its interfaces carry no initializers either way, so the schema default governs. The severity here is that the warning reads as cosmetic and the failure is not; a C# consumer gets no error and wrong output. Declaring the intended default first in the union is a workaround, but ordering a union's cases to protect a codegen backend is the wrong constraint to accept.

Three previously filed gaps recur here and need no new numbers: **NXE7** (no empty-sequence literal — every table is `T[]?` rather than `T[] = []`), **NXE9** (no refinement constraints — all of §11 lives outside the type system), and **NXE3** (`$type` on every record, which is what motivated inlining geometry into ops in §7 and keeping bindings as records rather than unions in §8).

One gap is conspicuously *absent*: **NXE1**, the missing map type, does not bite. The resource tables are positional, so `Brush[]` with index-as-id is the correct type rather than a workaround for a missing `Record<BrushId, Brush>`. The display list is the one part of the NX UI stack whose natural shape NX can already express.

# 15. Open questions

1. **Should `core` be split into a values-only sub-library** so the six genuinely identical scalar types are shared rather than duplicated (§3.2)? This is a change to the object model's packaging.
2. **Is device-pixel-ratio handling in or out?** Currently out: coordinates are logical pixels and the backend owns the device scale. That is right for Canvas2D and probably right generally, but it means a display list cannot be rasterized correctly without out-of-band knowledge of its target scale.
3. **Anti-aliasing is unspecified.** Flutter carries a per-op `is_aa` flag; Canvas2D cannot turn AA off for paths at all. Currently omitted, on the grounds that a flag no backend can honor uniformly is worse than none. Worth confirming.
4. **Does `bounds` need to be mandatory** for culling to be worth anything, or is optional-with-fallback right?
5. **The token-cost question the object model's §12.10 raises applies here too**, if NX-native serialization is ever the wire format. It is more acute here — a display list is far larger than a UI document — and is the strongest argument for the packed encoding in the architecture note's §10.

---

## Sources

- [Flutter `dl_op_receiver.h`](https://raw.githubusercontent.com/flutter/engine/main/display_list/dl_op_receiver.h) — the complete DisplayList op inventory, the attribute-stream design, and `drawDisplayList`
- [Chromium `cc/paint/paint_op.h`](https://chromium.googlesource.com/chromium/src/+/main/cc/paint/paint_op.h) — `PaintOpType`, `PaintOpWithFlags`, paint stored by value, `kDrawTextBlob` vs `kDrawSlug`
- [WebRender `PropertyBinding`](https://doc.servo.org/webrender_api/enum.PropertyBinding.html) and [`display_item.rs`](https://raw.githubusercontent.com/servo/webrender/main/webrender_api/src/display_item.rs) — the `Value` / `Binding(key, fallback)` shape and the three property types it is used for
- [WebRender clipping and positioning](https://github.com/servo/webrender/blob/main/webrender/doc/CLIPPING_AND_POSITIONING.md) and [`spatial_node.rs`](https://doc.servo.org/src/webrender/spatial_node.rs.html) — the spatial tree, reference frames, and `SpatialTree::update()`
- [Vello `Scene`](https://docs.rs/vello/latest/vello/struct.Scene.html) — per-draw transforms, no CTM stack, `push_layer`/`pop_layer`
- [Vello](https://github.com/linebender/vello) and its [architecture notes](https://github.com/linebender/vello/blob/main/doc/vision.md) — the parallel stream encoding and monoid prefix-scan pipeline
- [Skia `SkTextBlob`](https://api.skia.org/classSkTextBlob.html) — text run structure and serialization
