# NX Display List Architecture Proposal

## Purpose

This document proposes a display-list architecture for NX drawn UI,
including animation support. It is intended as technical design context
that can be handed to another LLM or engineer.

The proposal is designed to support two important NX rendering
environments:

1.  **Lightweight JavaScript/browser runtime**
    -   NX IL interpreted/rendered in JavaScript/TypeScript.
    -   Existing React-based NX renderer can host drawn UI.
    -   Canvas2D is the primary graphics backend.
    -   WebGL can optionally provide shaders and advanced effects.
    -   Goals: minimal code size, fast startup, and maximum
        device/browser compatibility.
2.  **Higher-performance native/WASM runtime**
    -   NX runtime may be implemented in Rust.
    -   Skia can provide the graphics backend.
    -   On the web, Skia may ultimately use WebGL or Graphite/WebGPU.
    -   Native platforms can use Skia or other platform renderers.

The architecture should allow the same NX graphics semantics to map
cleanly to both environments.

------------------------------------------------------------------------

# 1. High-Level Architecture

NX should distinguish four layers:

``` text
NX source
    ↓
NX IL
    ↓
Retained render tree
    ↓
NX Display List
    ↓
Rendering backend
```

More specifically:

``` text
                         NX IL
                           ↓
                      NX Runtime
                           ↓
                  Retained Render Tree
                           ↓
                     Display Lists
                           ↓
              ┌────────────┴─────────────┐
              ↓                          ↓
        Canvas2D / WebGL           Skia / Graphite
              ↓                          ↓
        browser canvas             WebGPU / native GPU
```

Each layer has a different responsibility.

## NX IL --- "What the UI is"

NX IL is the compiled/intermediate declarative representation of NX
source.

For example, conceptually:

``` text
Rect {
    width: 100
    height: 50
    fill: red
}
```

NX IL should remain declarative and semantic. It should **not** become a
sequence of low-level drawing calls.

JSON can be the canonical/debuggable NX IL encoding, with an optional
compact binary representation later.

## Retained render tree --- "What currently exists"

The NX runtime evaluates:

-   components
-   reactive expressions
-   state
-   layout
-   inheritance
-   animation bindings
-   conditional structure

and maintains a retained representation of the currently rendered UI.

The hierarchy belongs here.

## Display list --- "How this resolved state should be drawn"

After layout and property resolution, NX generates one or more display
lists containing backend-neutral rendering commands.

Examples:

``` text
Save
Transform
ClipRect
DrawRect
DrawRoundedRect
DrawPath
DrawText
DrawImage
BeginLayer
EndLayer
Restore
```

The display list should be viewed as a **compact rendering
program/value**, not as the UI hierarchy itself.

## Rendering backend

A backend interprets the display list using a specific graphics API.

Potential implementations include:

-   Canvas2D
-   Canvas2D + WebGL effect layers
-   Skia
-   CoreGraphics
-   Android Canvas
-   a future direct WebGPU renderer

NX semantics should not depend on any one backend.

------------------------------------------------------------------------

# 2. Core Display-List Model

The recommended conceptual representation is:

``` text
DisplayList
    = command stream
    + resource tables
    + dynamic parameter/slot tables
```

A display list should generally be:

-   linear
-   compact
-   append-only while being built
-   immutable after completion
-   cheap to replay
-   backend-neutral

It should **not** normally be:

-   a linked list
-   a mutable scene graph
-   a hierarchy of arbitrary objects
-   tied directly to Canvas2D, Skia, WebGL, etc.

------------------------------------------------------------------------

# 3. Commands

An initial command vocabulary might include:

## State

``` text
Save
Restore
```

## Transforms

``` text
Transform
```

## Clipping

``` text
ClipRect
ClipRoundedRect
ClipPath
```

## Drawing

``` text
DrawRect
DrawRoundedRect
DrawPath
DrawText
DrawImage
```

## Layers and effects

``` text
BeginLayer
EndLayer
ApplyEffect
```

## Composition

Eventually:

``` text
DrawDisplayList
```

This allows display lists to reference reusable sub-display-lists.

Commands should be semantic graphics operations rather than backend
operations.

Prefer:

``` text
DrawRoundedRect
```

over something Canvas2D-specific such as:

``` text
ctx.beginPath()
ctx.roundRect()
ctx.fill()
```

Likewise, do not expose WebGL concepts such as `drawArrays` in the NX
display-list abstraction.

------------------------------------------------------------------------

# 4. Resource Side Tables

Large or reusable objects should not generally be embedded repeatedly in
commands.

Instead, commands should reference immutable resources by IDs.

For example:

``` text
DrawPath path=17 paint=4
DrawText textRun=12 paint=7
DrawImage image=3
```

with separate resource tables:

``` text
paths[17]
paints[4]
textRuns[12]
images[3]
```

Useful resource types include:

-   Paint
-   Path
-   Image
-   Gradient/Brush
-   TextRun
-   Font
-   Shader
-   Effect definition
-   Nested display list

Use small typed IDs rather than pointers:

``` rust
struct PaintId(u32);
struct PathId(u32);
struct ImageId(u32);
struct TextRunId(u32);
struct DisplayListId(u32);
```

This is important for:

-   serialization
-   WASM interoperability
-   caching
-   cross-language boundaries
-   renderer independence
-   avoiding pointer ownership problems

------------------------------------------------------------------------

# 5. Paints

Paint/style state is frequently reused and should preferably be
immutable and interned.

Conceptually:

``` rust
struct Paint {
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    opacity: f32,
    blend_mode: BlendMode,
}
```

Rather than:

``` text
SetFill red
DrawRect
SetFill red
DrawRect
SetFill red
DrawRect
```

NX can emit:

``` text
DrawRect paint=7
DrawRect paint=7
DrawRect paint=7
```

The runtime can intern identical paints:

``` text
paintId = paintCache.intern(paint)
```

------------------------------------------------------------------------

# 6. Paths

Paths should similarly be immutable resources.

A logical NX path may contain operations such as:

``` text
MoveTo
LineTo
QuadraticTo
CubicTo
Close
```

The display list references a `PathId`.

Each backend can cache its native representation:

``` text
NX PathId 42
     │
     ├── Canvas2D → Path2D
     └── Skia     → SkPath
```

This keeps backend-specific path objects out of the NX model.

------------------------------------------------------------------------

# 7. JavaScript Representation

For the lightweight JavaScript renderer, start simple.

A good MVP representation is a JavaScript array of tagged command
objects:

``` ts
const enum Op {
    Save,
    Restore,
    Transform,
    ClipRect,
    DrawRect,
    DrawPath,
    DrawText,
    DrawImage,
    BeginLayer,
    EndLayer,
}

type DrawOp =
    | { kind: Op.Save }
    | { kind: Op.Restore }
    | { kind: Op.Transform; transform: TransformValue }
    | { kind: Op.DrawRect; rect: Rect; paint: PaintId }
    | { kind: Op.DrawPath; path: PathId; paint: PaintId }
    | { kind: Op.DrawText; run: TextRunId; x: number; y: number };
```

A display list might be:

``` ts
interface DisplayList {
    ops: DrawOp[];
    paints: Paint[];
    paths: PathData[];
    textRuns: TextRun[];
    images: ImageResource[];
    slots: SlotTable;
}
```

Rendering is straightforward:

``` ts
for (const op of list.ops) {
    switch (op.kind) {
        case Op.DrawRect:
            renderRect(ctx, op, list);
            break;

        case Op.DrawPath:
            renderPath(ctx, op, list);
            break;
    }
}
```

### Why start with objects?

Modern JavaScript engines handle arrays of stable-shaped objects well.

This representation is:

-   easy to implement
-   easy to inspect in browser devtools
-   easy to debug
-   easy to profile
-   likely fast enough for the initial NX use cases

Avoid prematurely introducing a complicated binary command format.

------------------------------------------------------------------------

# 8. Optimized JavaScript Representation

If profiling later demonstrates that display-list representation or
garbage collection is significant, NX can move toward typed arrays.

For example:

``` ts
opcodes: Uint8Array;
argOffsets: Uint32Array;
numbers: Float32Array;
ints: Uint32Array;
```

Or a packed command buffer.

This could represent:

``` text
DRAW_RECT
x
y
width
height
paintId
```

without allocating an object for every operation.

However, this should be an implementation optimization, not part of the
semantic NX API.

A useful evolution path is:

``` text
Development / initial implementation:
    DrawOp[] objects

Optimized implementation:
    typed/packed command buffers
```

------------------------------------------------------------------------

# 9. Rust Representation

For a Rust NX runtime, an excellent initial representation is:

``` rust
enum DrawOp {
    Save,
    Restore,

    Transform {
        transform: TransformValue,
    },

    DrawRect {
        rect: Rect,
        paint: PaintId,
    },

    DrawPath {
        path: PathId,
        paint: PaintId,
    },

    DrawText {
        run: TextRunId,
        x: f32,
        y: f32,
    },
}
```

with:

``` rust
struct DisplayList {
    ops: Vec<DrawOp>,
    paints: Vec<Paint>,
    paths: Vec<Path>,
    text_runs: Vec<TextRun>,
    images: Vec<Image>,
    slots: SlotTable,
}
```

A `Vec<DrawOp>` gives:

-   contiguous storage
-   good cache locality
-   efficient sequential traversal
-   strongly typed commands
-   straightforward implementation

This is probably the right starting point for a native/Rust NX renderer.

------------------------------------------------------------------------

# 10. Packed Native/Binary Representation

For maximum performance and interoperability, NX can eventually support
a packed command-buffer representation.

Conceptually:

``` rust
struct DisplayListBuffer {
    commands: Vec<u8>,
    resources: ResourceTable,
}
```

Commands can use a common header:

``` rust
#[repr(C)]
struct CommandHeader {
    opcode: u16,
    flags: u16,
    size: u32,
}
```

followed by the command payload.

Example:

``` text
┌───────────────────────┐
│ DrawRect header       │
│ x                     │
│ y                     │
│ width                 │
│ height                │
│ paint_id              │
├───────────────────────┤
│ DrawPath header       │
│ path_id               │
│ paint_id              │
├───────────────────────┤
│ DrawText header       │
│ text_run_id           │
│ x                     │
│ y                     │
└───────────────────────┘
```

The `size` field allows:

-   efficient sequential traversal
-   skipping unknown commands
-   forward-compatible versioning

A packed display list can be useful for:

-   Rust → C++ Skia boundaries
-   WASM → JavaScript boundaries
-   caching
-   serialization
-   recording/replay
-   shared memory
-   possibly remote rendering/debug tooling

The rest of NX should use a `DisplayListBuilder` abstraction rather than
depending directly on the packed encoding.

------------------------------------------------------------------------

# 11. DisplayListBuilder

Both JavaScript and Rust implementations should expose a builder-style
abstraction.

Conceptually:

``` text
builder.save()
builder.transform(...)
builder.drawRect(...)
builder.drawPath(...)
builder.drawText(...)
builder.restore()

displayList = builder.finish()
```

The builder is responsible for:

-   command construction
-   resource interning
-   assigning IDs
-   slot allocation
-   optional command optimization
-   eventually packing commands

This lets the physical representation evolve without changing the rest
of the NX runtime.

------------------------------------------------------------------------

# 12. Immutability

Display lists should generally follow:

``` text
Build → Finish/Seal → Replay many times
```

Once finished, a display list should be treated as immutable.

Benefits include:

-   safe caching
-   easier debugging
-   safe replay
-   easier multithreaded rendering
-   straightforward generation/version tracking
-   easier reuse by nested display lists

Dynamic animation state should therefore normally live **outside the
immutable command stream**.

------------------------------------------------------------------------

# 13. Animation: Parameterized Display Lists

NX should make parameterized display lists a first-class concept.

The basic architecture is:

``` text
NX Render Tree
      ↓
Immutable Display List
      │
      └── references
              ↓
       Dynamic Slot Table
              ↓
           Renderer
```

For example, the display list might logically contain:

``` text
PushTransform transform=slot[3]
SetOpacity opacity=slot[7]
DrawDisplayList character
Restore
```

while the changing values are:

``` text
transformSlot[3] = ...
floatSlot[7] = 0.72
```

On the next animation frame:

``` text
floatSlot[7] = 0.76
```

The display list itself does not change.

------------------------------------------------------------------------

# 14. Why Use Slots?

Without slots, an animation may require:

``` text
animation tick
    ↓
evaluate NX
    ↓
update render tree
    ↓
regenerate display list
    ↓
render
```

even if the only meaningful change is:

``` text
opacity = 0.72 → 0.76
```

With slots:

``` text
animation tick
    ↓
evaluate animation
    ↓
update slot
    ↓
replay same display list
```

This is especially valuable when an entire complex subtree is moving or
fading.

For example:

``` text
DisplayList #23
    500 drawing commands
```

can be animated as:

``` text
PushTransform slot=12
PushOpacity slot=13
DrawDisplayList 23
Restore
Restore
```

Only a transform and one float need to change per frame.

------------------------------------------------------------------------

# 15. Slot Types

A slot is a typed dynamic value referenced by immutable rendering
commands.

Possible slot types include:

``` text
FloatSlot
Vec2Slot
ColorSlot
TransformSlot
RectSlot
EffectParameterSlot
```

A logical slot table might look like:

``` ts
interface SlotTable {
    floats: number[];
    colors: Color[];
    transforms: Transform[];
    vec2s: Vec2[];
}
```

For a more optimized JavaScript implementation:

``` ts
const floats = new Float32Array(...);
const transforms = new Float32Array(...); // e.g. six floats per 2D transform
```

Rust might use:

``` rust
struct SlotTable {
    floats: Vec<f32>,
    transforms: Vec<Transform>,
    colors: Vec<Color>,
    vec2s: Vec<Vec2>,
}
```

------------------------------------------------------------------------

# 16. Constants vs Dynamic Values

Not every property should incur dynamic indirection.

Conceptually, properties may be:

``` text
Constant<T>
Slot<T>
```

For example:

``` text
width = 200         → constant embedded in command
height = 100        → constant embedded in command
x = animated        → slot reference
opacity = animated  → slot reference
```

This avoids paying slot lookup costs for values that never change.

The display-list compiler should determine whether a resolved property
can be baked into the command or needs a dynamic slot.

------------------------------------------------------------------------

# 17. Prioritize Transform and Opacity

Transform and opacity are especially valuable dynamic properties.

They allow entire immutable subtrees to animate without regeneration.

Example:

``` text
RootDisplayList
    |
    +-- DrawDisplayList(background)
    |
    +-- PushTransform(slot=12)
    |       PushOpacity(slot=13)
    |           DrawDisplayList(character)
    |       Restore
    |   Restore
    |
    +-- DrawDisplayList(ui)
```

The character display list may contain hundreds of operations but remain
unchanged while moving, rotating, scaling, or fading.

This is analogous to compositor-friendly animation in web browsers.

------------------------------------------------------------------------

# 18. Three Animation Classes

NX should conceptually distinguish three categories of animation.

## 18.1 Compositor-style animation

Examples:

-   translation
-   rotation
-   scale
-   opacity
-   some clipping
-   effect/shader parameters

Preferred implementation:

``` text
same display list
+
updated slots
```

This should be the fastest animation path.

## 18.2 Geometry animation

Examples:

``` text
rectangle width
corner radius
stroke width
gradient position
simple shape coordinates
```

These can often also be represented as slots.

Example:

``` text
DrawRect
    x = constant
    y = constant
    width = slot[5]
    height = constant
```

Whether a particular geometry value should be parameterized can depend
on the backend and performance characteristics.

## 18.3 Structural animation/change

Examples:

-   adding/removing nodes
-   changing from one component subtree to another
-   changing the number/order of rendered objects
-   a state transition that changes drawing commands themselves

Example:

``` text
5 circles → 6 circles
```

This requires regenerating at least the affected display-list segment.

The general model is:

``` text
                    change
                      │
          ┌───────────┼────────────┐
          ↓           ↓            ↓
      transform/    geometry     structural
       opacity
          │           │            │
       update       update       rebuild
        slot         slot       affected list
```

------------------------------------------------------------------------

# 19. Nested/Sub-Display-Lists

NX should eventually support reusable/nested display lists.

Example:

``` text
Root
  ├── BackgroundDisplayList
  ├── ChartDisplayList
  ├── SpinnerDisplayList
  └── FooterDisplayList
```

If only the spinner changes, NX does not need to regenerate the
background, chart, or footer.

A command such as:

``` text
DrawDisplayList id=17
```

can reference an immutable sub-list.

Useful metadata per sub-list may include:

``` text
bounds
generation/version
resources
dynamic slots
```

This supports selective regeneration and caching.

For the MVP, a single flat list is acceptable. Nested display lists
should be viewed as an important optimization/evolution path.

------------------------------------------------------------------------

# 20. Selective Regeneration

The retained render tree should track dependencies and dirtiness.

A useful eventual lifecycle is:

``` text
NX state changes
      ↓
retained render tree determines affected nodes
      ↓
┌─────────────────────────────────────┐
│ dynamic property only?              │
│     update slot                     │
│                                     │
│ display-list content changed?       │
│     rebuild affected sub-list       │
│                                     │
│ structural change?                  │
│     reconcile tree + rebuild region │
└─────────────────────────────────────┘
      ↓
render
```

This combines retained-mode semantics with efficient immediate
rendering.

------------------------------------------------------------------------

# 21. Animation Engine Ownership

The graphics renderer should **not** own animation semantics.

It should not need to understand:

-   easing curves
-   springs
-   durations
-   delays
-   repeat behavior
-   state transitions

Instead:

``` text
NX Animation Engine
        ↓
evaluate animation at time T
        ↓
update dynamic slots
        ↓
Renderer
```

For example:

``` text
animation:
    from = 0
    to = 500
    duration = 1 second
    easing = easeOut
```

At a particular frame, the animation system computes:

``` text
slot[13] = 284.7
```

The renderer only consumes current values.

This keeps all rendering backends simple and consistent.

------------------------------------------------------------------------

# 22. Frame Scheduling in JavaScript

For browser rendering, NX should remain event/reactive rather than
permanently running a game loop.

When an animation is active:

``` text
requestAnimationFrame()
```

Each frame:

``` text
evaluate active animations
      ↓
update slots
      ↓
did any visual value change?
      ↓
yes → render
no  → skip render
```

When no animations or other frame-driven effects remain active, stop
requesting frames.

This minimizes CPU/battery use.

------------------------------------------------------------------------

# 23. Shader Animation

Slots map particularly well to shaders.

Suppose NX conceptually contains:

``` text
ShaderEffect {
    shader: water
    time: animated
    intensity: animated
}
```

The display list can reference:

``` text
DrawEffect
    shader = 14
    uniforms = UniformSet(8)
```

while frame updates modify:

``` text
uniforms[8].time
uniforms[8].intensity
```

This maps naturally to different backends:

``` text
NX dynamic values
       │
       ├── Canvas2D → CPU-side dynamic properties
       ├── WebGL    → uniforms
       ├── WebGPU   → uniform/storage buffers
       └── Skia     → shader/runtime-effect uniforms
```

This is a strong reason for making dynamic parameter slots
backend-neutral.

------------------------------------------------------------------------

# 24. Canvas2D Rendering

The lightweight NX web renderer should initially use Canvas2D.

Architecture:

``` text
NX IL
    ↓
NX/React runtime
    ↓
retained graphics subtree
    ↓
display list
    ↓
Canvas2D renderer
```

The Canvas2D renderer simply interprets commands.

For example:

``` ts
function drawRect(
    ctx: CanvasRenderingContext2D,
    op: DrawRectOp,
    list: DisplayList
) {
    const paint = list.paints[op.paint];

    ctx.fillStyle = paint.fill;
    ctx.fillRect(
        resolve(op.x, list.slots),
        resolve(op.y, list.slots),
        resolve(op.width, list.slots),
        resolve(op.height, list.slots)
    );
}
```

For the initial implementation, full display-list replay each frame is
acceptable.

Optimization should follow profiling.

------------------------------------------------------------------------

# 25. Canvas2D + WebGL

Advanced shader/effect functionality can optionally use WebGL without
requiring the entire NX renderer to become WebGL-based.

A possible architecture:

``` text
NX scene
    ↓
Display lists
    ↓
┌─────────────────────────────┐
│ Canvas2D                    │
│ paths                       │
│ text                        │
│ images                      │
│ gradients                   │
│ normal UI graphics          │
└─────────────────────────────┘

             +

┌─────────────────────────────┐
│ WebGL effect layers         │
│ shaders                     │
│ distortion                  │
│ procedural graphics         │
│ particles                   │
│ transitions                 │
└─────────────────────────────┘
```

Canvas2D and WebGL generally should not be treated as two
interchangeable contexts operating directly on the same canvas
simultaneously.

Instead, advanced effects can use:

-   separate canvases/layers
-   OffscreenCanvas where appropriate
-   WebGL-rendered intermediate surfaces composited into the final
    result

The NX display-list abstraction should hide those details.

------------------------------------------------------------------------

# 26. Skia Mapping

The same conceptual display list should map naturally to Skia.

``` text
NX Display List
       ↓
NX Skia Renderer
       ↓
Skia
       ↓
Ganesh or Graphite
       ↓
WebGL / WebGPU / native GPU API
```

Example mappings:

``` text
DrawRect       → SkCanvas drawRect
DrawPath       → SkPath + SkPaint
DrawText       → Skia text APIs
BeginLayer     → Skia saveLayer or equivalent
Shader params  → SkSL/runtime-effect uniforms
```

A Rust NX runtime can produce the same logical display-list commands as
the JavaScript runtime even if its physical representation is more
compact.

------------------------------------------------------------------------

# 27. React Integration

The existing NX React renderer can host a graphics subtree without
making the display list itself React-specific.

Conceptually:

``` text
NX IL
    ↓
NX React Runtime
    ↓
React reconciliation
    ↓
Host nodes
    ├── DOM
    └── NX Graphics Surface
             ↓
        retained graphics tree
             ↓
         display lists
             ↓
          Canvas2D
```

For example, a graphics surface could conceptually behave like:

``` tsx
function NxGraphicsSurface({ children }) {
    const scene = useGraphicsScene(children);
    const displayList = compileDisplayList(scene);

    useLayoutEffect(() => {
        renderer.render(displayList);
    });

    return <canvas />;
}
```

The actual implementation can optimize beyond this simple model, but
this is a useful conceptual separation.

React handles declarative reconciliation; the NX graphics subsystem owns
drawing.

------------------------------------------------------------------------

# 28. Recommended MVP

The initial implementation should favor simplicity.

## JavaScript/browser MVP

Use:

``` text
NX IL
    ↓
existing NX/React runtime
    ↓
retained graphics nodes
    ↓
DrawOp[] display list
    ↓
Canvas2D
```

Data structures:

``` text
DrawOp[]
Paint[]
Path[]
TextRun[]
Image[]
SlotTable
```

Implement:

-   stable numeric command tags
-   resource IDs
-   immutable finished display lists
-   float/transform/color slots for animation
-   full list replay on each required frame
-   `requestAnimationFrame` only while animation is active

Initially, rebuilding an entire small display list when structure
changes is fine.

Do **not** initially implement:

-   dirty rectangles
-   complicated binary command streams
-   direct WebGPU rendering
-   extensive incremental display-list mutation
-   sophisticated GPU compositing

------------------------------------------------------------------------

# 29. Recommended Evolution

A reasonable sequence is:

## Phase 1 --- Simple Canvas2D

``` text
DrawOp[]
resource arrays
slot arrays
full replay
```

## Phase 2 --- Better caching

Add:

-   cached `Path2D`
-   cached text layout
-   interned paints
-   immutable resources
-   nested/sub-display-lists
-   selective regeneration

## Phase 3 --- Advanced effects

Add:

-   WebGL effect layers
-   shader uniforms mapped from NX slots
-   offscreen surfaces/layers

## Phase 4 --- Optimized command representation

If profiling justifies it:

-   typed arrays in JavaScript
-   packed binary display lists
-   stable C ABI
-   zero/low-copy WASM boundaries

## Phase 5 --- High-performance Skia backend

Use:

``` text
NX runtime
    ↓
same conceptual display list
    ↓
Skia adapter
    ↓
Skia / Graphite
    ↓
WebGPU or native GPU
```

The same NX graphics model and animation-slot concepts remain valid.

------------------------------------------------------------------------

# 30. Core Design Principles

The architecture can be summarized by several principles.

### 1. NX remains declarative

NX IL describes what the UI/graphics are, not imperative backend drawing
calls.

### 2. Hierarchy belongs in the retained render tree

The display list is not the UI scene graph.

### 3. The display list is a rendering program/value

It should be linear, compact, immutable after construction, and cheap to
replay.

### 4. Resources are immutable and referenced by IDs

Paths, paints, images, text runs, shaders, and nested lists should be
reusable resources rather than repeatedly embedded data.

### 5. Dynamic slots are separate from stable commands

**Display lists represent stable rendering instructions. Slots represent
values expected to change without changing those instructions.**

### 6. Prefer slot updates over regeneration for animation

Transforms, opacity, shader uniforms, and many geometry values should
update without rebuilding display lists.

### 7. Regenerate when rendering structure actually changes

Structural changes should rebuild only the affected display-list segment
where practical.

### 8. Animation semantics live above rendering

The NX animation engine computes current values; renderers consume those
values.

### 9. Keep the display-list contract backend-neutral

The same conceptual commands should work with Canvas2D, WebGL effects,
Skia, and future renderers.

### 10. Optimize physical representation independently

JavaScript can begin with ergonomic objects; Rust can use `Vec<DrawOp>`;
both can later support a shared packed format.

------------------------------------------------------------------------

# 31. Concise Reference Architecture

The intended long-term architecture is:

``` text
┌──────────────────────────────────────────┐
│                 NX Source                │
└────────────────────┬─────────────────────┘
                     ↓
┌──────────────────────────────────────────┐
│                  NX IL                   │
│       JSON + optional binary form        │
└────────────────────┬─────────────────────┘
                     ↓
┌──────────────────────────────────────────┐
│                NX Runtime                │
│                                          │
│ reactive state / components / layout     │
│ animation evaluation / reconciliation    │
└────────────────────┬─────────────────────┘
                     ↓
┌──────────────────────────────────────────┐
│          Retained Render Tree            │
└────────────────────┬─────────────────────┘
                     ↓
┌──────────────────────────────────────────┐
│              Display Lists               │
│                                          │
│ immutable commands                       │
│ immutable resource tables                │
│ references to dynamic slots              │
│ optional nested display lists            │
└───────────────┬──────────────────────────┘
                │
         ┌──────┴──────┐
         ↓             ↓
┌────────────────┐  ┌──────────────────────┐
│ Dynamic Slots  │  │ Immutable Resources  │
│                │  │                      │
│ floats         │  │ paths                │
│ transforms     │  │ paints               │
│ colors         │  │ images               │
│ shader params  │  │ text runs            │
└───────┬────────┘  └──────────┬───────────┘
        │                       │
        └───────────┬───────────┘
                    ↓
┌──────────────────────────────────────────┐
│                 Renderer                 │
└──────────┬───────────────────────┬───────┘
           ↓                       ↓
      Canvas2D                 Skia
           │                       │
    optional WebGL            Graphite
      effect layers                │
                                  WebGPU
```

The important result is that NX can have a **very small
JavaScript/Canvas2D implementation and a sophisticated Rust/Skia
implementation without defining two different graphics models**.

Animation slots and immutable display lists provide the bridge between
the two.
