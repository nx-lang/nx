# Visual UX improvements

NX's Visual view integrates document structure and element properties in one reading. This is worth
preserving: it lets users understand composition, compare siblings, and review properties without
continually switching selection. Users familiar with outline-and-inspector design tools will still
need a clear way to navigate the composition and focus on one element.

The current view makes an individual element readable, but expanded properties push other elements
out of view. Nested values such as gradients also compete visually with drawable elements. The
following suggestions aim to preserve the integrated approach while making navigation and editing
more predictable. These are design proposals, not descriptions of implemented behavior.

## 1. Use stable, compact element rows with properties that unfold in place

Give each element a compact identity row that remains recognizable whether its properties are open
or closed. Opening an element's properties should create an inspector inside the outline while
keeping the surrounding composition visible.

For example, selecting Star could produce this conceptual presentation:

```text
▾ Drawing
    ▸ Properties                         2 set

    Rectangle                            320 × 150 · Gradient
    Circle                               Radial gradient
    Star                                 5 points · White stroke
      Position       X 60       Y 70
      Shape          Outer 35   Inner 17   Points 5
      Fill           [gradient] sunset() ↗
      Stroke         ■ White    Width 2

    Text             “Gradient paints”
    Path                                 Gradient fill
    Path                                 Blue stroke
```

The summaries are illustrative. Choosing which properties belong in a compact summary needs further
design work, potentially informed by visualizer declarations.

## 2. Separate child disclosure from property disclosure

A container should be able to show its children while its own properties remain closed. Conversely,
users should be able to inspect a container's properties without expanding its descendants.

Use distinct affordances for children and property details, with keyboard equivalents. This allows
users to navigate the element hierarchy without also opening every element's property sheet.

## 3. Reveal properties on selection and support pinning for comparison

Selecting an element should normally open its properties and close the previous element's temporary
details. Pinning an element should keep its properties open when selection moves elsewhere.

Details explicitly opened by the user should also remain open until dismissed. Distinguish temporary
selection-driven expansion from deliberate expansion so that selection does not undo the user's
chosen workspace.

This provides a manageable default while preserving the integrated view's ability to compare several
elements at once.

## 4. Make editable values easier to discover

Property values currently resemble syntax-highlighted text. Add subtle field boundaries on hover and
keyboard focus, clear color controls, and numeric scrubbing where appropriate.

Keep the resting presentation lightweight, but make the transition from reading to editing obvious.
A selected property row should clearly communicate both focus and how to edit its value.

## 5. Present compound values semantically

Distinguish an element in the composition from a structured value belonging to that element. Circle
and its radial-gradient paint should not compete as equivalent objects in the visual hierarchy.

A stroke could initially show a swatch, paint description, and width. A gradient could show a small
gradient strip and stop count, with its stops available through expansion. Group related values such
as position coordinates where that improves reading.

Keep the exact NX expression accessible through disclosure or Show in Source. Semantic presentation
should preserve access to the source behind it.

## 6. Share selection between Visual and Preview

Clicking a rendered element in Preview should reveal and highlight its source element and properties
in Visual. Selecting an element in Visual should identify it on the canvas.

Property focus should retain a clear connection to the owning element. The preview's inspection
controls and Visual's selection should communicate a shared focus.

Repeated components and generated elements need a deliberate mapping between rendered instances and
their source declaration; a single source element may correspond to multiple rendered instances.

## 7. Distinguish written expressions from evaluated preview values

Visual reads what was written, while Preview may know what was produced. Preserve expressions such
as `sunset()` as visibly authored expressions.

If Preview supplies an evaluated gradient or another computed value, present it as additional
information with clear provenance. Avoid implying that a runtime result is a literal value written
in the source, or that Visual can determine it without evaluation.

## 8. Make layout changes preserve the apparent document structure

The current width-dependent choice between inline properties and property rows is useful for fitting
content, but resizing a pane can change the apparent structure of the document.

Prefer stable element identity rows and deliberate detail expansion. Responsive wrapping should
preserve the user's place and keep the hierarchy recognizable as the pane changes width.

## 9. Explore an optional docked inspector after improving inline details

A permanent third properties pane consumes canvas space and reduces the contextual value of the
integrated view. First prototype the improved inline interaction.

An optional docked inspector could later help with large property sets or workflows where users
prefer a fixed editing location. It should share selection and editing behavior with the integrated
view.

## 10. Validate the interaction with larger documents and concrete tasks

Prototype compact element rows, independent property disclosure, selection-driven inline details,
and pinning together. Test with a substantially larger document than the example screenshot.

Use three tasks: find an element, adjust it, and compare it with a sibling. Observe whether users can
complete those tasks while retaining their place in the composition, whether editing controls are
discoverable, and whether selection-driven expansion causes disruptive movement.

The key question is whether the integrated view makes focused editing and contextual comparison
comfortable as the document grows.
