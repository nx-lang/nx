# Prompt

Related, I'd like to NX to support drawn UI, with the kind of primitives that exist in [https://](https://drawnui.net)[drawnui.net](https://drawnui.net)  and [https://www.openui.com/](https://www.openui.com/).&#x20;
Implementation wise, such drawn UI, when expressed in NX, could actually be rendered for different platforms, with different languages (Rust, native iOS / Android, .NET, etc.) but the main implementation - the one I'd like to focus on initially - would be with JavaScript. It could use the current NX rendering support for JavaScript/TypeScript, normally in React, rendering NX JSON where that JSON includes drawn UI primitives (drawing shapes with outline and fill, ideally animations, text layout, etc.).

Given that, what's the best way to implement that kind of drawing support for NX JSON with JavaScript?
What drawing tech do you propose using?

# Response

For the initial JavaScript renderer, I’d build the NX drawn-UI backend on **PixiJS**, not directly on Canvas 2D, SVG, or CanvasKit/Skia.

The architectural split I’d use is:

**NX drawn UI model → NX scene/layout runtime → renderer adapter → PixiJS**

That gives NX its own stable abstraction instead of letting PixiJS’s object model become the language model. PixiJS is then simply the first high-performance renderer.

PixiJS is a particularly good fit because its scene graph already looks a lot like what you probably want NX JSON to represent: nested containers plus drawable leaves such as shapes, text, images and meshes. It supports inherited transforms, ordering, clipping/masks, filters, interaction and GPU rendering through WebGL/WebGPU. 

I’d imagine NX JSON along roughly these lines conceptually:

```json
{
  "type": "canvas",
  "children": [
    {
      "type": "group",
      "layout": {
        "direction": "column",
        "gap": 16,
        "padding": 24
      },
      "children": [
        {
          "type": "shape",
          "shape": "roundedRect",
          "fill": "#fff",
          "stroke": "#ddd",
          "radius": 12
        },
        {
          "type": "text",
          "text": "Hello",
          "fontSize": 24,
          "fontWeight": 600
        }
      ]
    }
  ]
}
```

But I would **not** make `Pixi.Graphics`, `Pixi.Container`, etc. first-class NX concepts. Your NX types should instead be things like `Canvas`, `Group`, `Shape`, `Path`, `Text`, `Image`, `Clip`, and perhaps `Effect`. That is what preserves the possibility of compiling the same tree later to Skia/CanvasKit, Swift/CoreGraphics/Metal, Android Compose/Skia, .NET/SkiaSharp, Rust/wgpu/vello, etc.

## Why PixiJS is my first choice

It hits an unusually good middle ground between “drawing library” and “game engine.”

Its `Graphics` primitive already handles rectangles, circles, polygons, arbitrary paths, fills, strokes, gradients, masks and related vector-like drawing. 

Its retained scene graph means NX doesn’t have to repaint an entire canvas imperatively every time something changes. You can preserve NX node identity and update only the corresponding Pixi object. Containers naturally support hierarchical transforms and nested grouping. 

It also gives you pointer/touch interaction with a DOM-like event system. That matters because “drawn UI” very quickly becomes more than rendering: hover, press, dragging, gestures, hit-testing, etc. 

And unlike a library primarily aimed at editors, PixiJS is explicitly optimized for rich animated 2D experiences, games, interactive ads and visualizations. That overlaps almost perfectly with the interactive-experience ideas we were discussing. 

### Layout is less of a problem than it used to be

One concern I would previously have had with PixiJS is layout. You don't want NX authors manually assigning x/y coordinates to everything.

Pixi now has `@pixi/layout`, a Flexbox-style layout layer built around Yoga. 

That means something like:

```nx
Column {
    gap: 16
    padding: 24

    Text {
        value: "Your results"
        fontSize: 28
    }

    Row {
        gap: 12

        ScoreCard { ... }
        ScoreCard { ... }
    }
}
```

could map naturally onto a Yoga-style layout model while still drawing everything into the Pixi scene.

That architecture is quite similar conceptually to DrawnUI: DrawnUI has its own virtual-control/layout tree and renders those controls through SkiaSharp rather than relying on native UI widgets. Its core `SkiaControl` participates in measuring, arranging, rendering, hit testing and invalidation, while its layout controls implement row/column/grid/wrap/absolute positioning. 

**I would borrow heavily from that architecture.**

## Text deserves special treatment

This is one of the areas where I'd avoid overabstracting too early.

Pixi provides three text approaches: regular `Text`, GPU-oriented `BitmapText`, and `HTMLText` for richer formatting. Its regular text uses the browser's native text engine and turns the result into a texture. 

Initially I'd make NX's text abstraction relatively semantic:

```nx
Text {
    text: "Congratulations!"
    font: Inter
    size: 32
    weight: bold
    color: #223344
    align: center
    maxWidth: 400
}
```

rather than exposing concepts such as bitmap fonts.

The renderer decides how to implement it.

Later I'd add spans:

```nx
Text {
    "You saved "
    Span {
        text: "$4,280"
        weight: bold
        color: green
    }
    " this year."
}
```

And eventually things like text-on-path and per-word/per-character animation. Pixi itself is already moving in that direction with its `SplitText` support for individually manipulating lines, words and characters. 

## Animation should belong to NX, not Pixi

This is an important architectural choice.

I would **not** define animation in terms of Pixi's ticker or GSAP calls.

I'd give NX a declarative animation model:

```nx
Circle {
    radius: 30
    fill: blue

    animate {
        scale: 1 -> 1.15
        duration: 300ms
        easing: easeOut
        autoreverse: true
    }
}
```

Or, because NX is functional-reactive, perhaps even more naturally:

```nx
scale: hovered ? 1.08 : 1
transition: 180ms easeOut
```

Then the JS runtime turns that into efficiently interpolated Pixi properties.

That same NX semantics could eventually translate to Core Animation on iOS, Android animation APIs, Skia animation, etc.

I'd especially support animation of:

- position/size
- rotation
- scale
- opacity
- fill/stroke colors
- stroke width
- corner radius
- clipping geometry
- path morphing eventually
- filters/effects
- layout transitions

That last one could be extremely powerful. Change an NX tree and have objects smoothly move from their old layout position to their new position automatically.

## I'd make the NX hierarchy slightly higher-level than a pure drawing API

The primitives I would probably start with are:

- `Canvas`
- `Group`
- `Shape`
- `Rect`
- `RoundedRect`
- `Ellipse`
- `Line`
- `Path`
- `Text`
- `Image`
- `Icon`
- `Clip`
- `Transform`
- `Effect`

And layout containers:

- `Stack` / `Column` / `Row`
- `Grid`
- `Layer` / `ZStack`
- `Absolute`
- eventually `Flow` / `Wrap`

Then interactive abstractions can sit *above* those:

```nx
Button { ... }
Slider { ... }
DragTarget { ... }
Tooltip { ... }
```

A button could still be completely drawn—it isn't an HTML `<button>`—but it's a semantic component composed from lower-level primitives.

That's also philosophically close to DrawnUI. Its controls are virtual/drawn controls rather than native widgets, with everything ultimately rendered through Skia. 

## React should be the host, not necessarily the renderer

Given your existing NX → JSON → React architecture, I'd do something like:

```text
React DOM application

    <NxDrawSurface value={nxJson} />

              ↓

      NX Draw Runtime
      ├ layout
      ├ diff/reconciliation
      ├ interaction
      ├ animation
      └ accessibility

              ↓

        PixiJS scene graph

              ↓

       WebGL / WebGPU
```

React owns the surrounding application lifecycle.

But I would **not** necessarily create a React component for every drawn object and depend heavily on React reconciliation for a scene animating at 60 FPS.

Instead, have one React component create/manage the Pixi surface. Inside it, NX's own reconciler updates the Pixi scene graph.

So:

```tsx
<NxDraw value={json} />
```

might correspond to one `<canvas>` DOM element, even if the NX tree contains 2,000 objects.

That gives you much better control over animation/performance and also makes the renderer architecture portable.

## I would use retained identity aggressively

This fits NX particularly well.

Suppose:

```nx
Balloon #mainBalloon {
    x: 120
    y: 200
    fill: red
}
```

becomes an NX JSON node with persistent identity.

The JS renderer maintains:

```text
NX ID #mainBalloon
        ↓
Pixi Graphics instance 0x7ab...
```

When the state changes to:

```text
fill: blue
x: 180
```

you don't reconstruct the scene.

You mutate the corresponding retained Pixi node:

```text
graphics.position.x = 180
update fill
```

and animate between states if NX calls for it.

This starts looking much more like a genuine **UI rendering engine** than a JSON-to-canvas converter.

## Accessibility: don't ignore the DOM completely

Canvas-based UI has an obvious weakness: browser semantics/accessibility.

Pixi has an accessibility layer that creates DOM overlays for interactive visual objects, providing screen-reader and keyboard accessibility. 

I'd adopt the same architectural concept but make accessibility semantics part of NX:

```nx
Shape {
    role: button
    accessibilityLabel: "Start simulation"
    onPress: start()
}
```

The web renderer can create invisible/transparent DOM semantic counterparts where appropriate.

A native renderer would map those same semantics to native accessibility APIs.

This is another reason not to simply expose a graphics library's API directly through NX.

---

## What about the alternatives?

**Canvas 2D directly:** I wouldn't. It looks attractive initially because there are no dependencies, but you'd end up implementing your own scene graph, dirty tracking, event dispatch, hit testing, text caching, batching, etc. MDN itself describes Canvas as fundamentally an immediate drawing API rather than a retained UI system. 

**SVG:** I'd support SVG input/output, but not use the SVG DOM as the primary runtime. It is wonderful for relatively static diagrams and illustrations, but I think you'd eventually run into limits for game-like animation and large dynamic scenes. The fact that every shape becomes a browser DOM node is simultaneously SVG's strength and weakness.

**Konva:** This would probably be my **second choice**. It's very pleasant for interactive editors because it provides shapes, nesting, layers, event handling, drag/drop and React integration.  But I'd choose it if you were building something Canva/Figma-ish first. Pixi is a better foundation if the target includes sophisticated animated interactive experiences and mini-games.

**Fabric.js:** Similar story. Fabric has an excellent interactive object model, serialization, SVG handling and unusually strong on-canvas text editing.  I'd reach for it for an illustration/design editor, not as the fundamental NX rendering engine.

**Skia CanvasKit:** This is the most interesting alternative. CanvasKit is Google's Skia compiled to WebAssembly.  It would give you conceptual alignment with DrawnUI and with future native Skia implementations.

But I wouldn't start there.

CanvasKit is significantly lower-level. You'd be building much more of the runtime yourself, and the WASM/Skia payload and integration complexity buy you relatively little for your first web implementation.

I could imagine **CanvasKit becoming NX's second JS renderer**, though:

```text
NX Draw API
    ├── PixiRenderer        ← default web
    ├── CanvasKitRenderer   ← pixel-perfect Skia web
    ├── SvgRenderer         ← export/static
    └── HeadlessRenderer
```

That's a nice architectural proof that NX is actually renderer-neutral.

## The model I'd borrow from DrawnUI

DrawnUI is useful here not primarily because of Skia itself, but because of the abstraction it builds *above* Skia.

Its fundamental control handles positioning in a rendering tree, coordinate transforms, invalidation, hit testing, effects, visibility, measure, arrange and draw. 

That's approximately the runtime NX needs:

```text
NxDrawNode
  identity
  parent / children

  measure()
  arrange()

  transform
  opacity
  clip
  visibility

  hitTest()
  pointerEvents

  invalidateLayout()
  invalidatePaint()

  animate()

  accessibility
```

Then:

```text
NxRect
NxPath
NxText
NxImage
NxLayout
...
```

And finally renderer-specific objects.

This is the key thing I would build rather than thinking of the feature as simply:

> “NX JSON translated into Pixi calls.”

It gives NX a real **drawn UI runtime**.

---

### One architectural wrinkle I think is especially important for NX

I'd actually allow **DOM UI and drawn UI to coexist naturally**.

For example:

```nx
Page {
    Header { ... }       // ordinary DOM/React UI

    DrawSurface {
        InteractiveDiagram { ... }    // Pixi
    }

    ContactForm { ... }  // ordinary DOM/React UI
}
```

And perhaps eventually the reverse, selectively:

```nx
DrawSurface {
    Scene { ... }

    NativeControl {
        TextInput { ... }
    }
}
```

That's conceptually similar to DrawnUI's ability to integrate native controls when needed rather than insisting absolutely everything be drawn. 

For marketing experiences especially, this hybrid model is ideal: **DOM for conventional content/forms/SEO/accessibility; Pixi/NX Draw for the visually sophisticated experience.**

So my concrete recommendation is:

**Use PixiJS v8 as the first renderer, Yoga/`@pixi/layout` or an NX-owned equivalent for layout, and build a renderer-neutral retained NX Draw scene graph above it. Keep animation, layout, interaction semantics and accessibility in NX rather than exposing Pixi-specific concepts.**

That gets you something quite close philosophically to **“DrawnUI for NX”**, while being much more naturally aligned with your JavaScript/React starting point and with the game-like interactive marketing experiences that make this capability especially valuable.