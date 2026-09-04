# The NX catalog and where it diverges from DrawnUI

`catalog/skia.nx` is generated from the vendored DrawnUI sources by `npm run generate-catalog`, and
both it and `catalog/catalog-meta.json` are committed. This file records the places where the
generated catalog is deliberately not a faithful copy of the object model it was derived from, so
that a surprise in the fiddle can be checked against a list rather than guessed at.

It covers 23 components, 22 unions and 5 record types.

## Every property is optional

The catalog declares no defaults. DrawnUI's constructors already establish them, and the renderer
drops null-valued properties, so an unset property is left to the control rather than restated in
NX.

The alternative was mirroring each default into the catalog, which the design originally proposed.
Two things argued against it. Most DrawnUI properties are accessor pairs over private fields, so a
generator reading initializers recovers some defaults and silently misses others — and a *wrong*
mirrored default changes what is drawn, with nothing to catch it. And a mirrored default is one more
thing that can drift from the vendored code between syncs. Nulls cost an author nothing: an omitted
property behaves exactly as it does in the TypeScript original.

The visible consequence is that NX sees `null` where DrawnUI sees `"Absolute"`. Since external
components are opaque to NX — nothing reads these values back — the difference has no effect beyond
the wire format.

## Types that were simplified

| DrawnUI | NX | Why |
|---|---|---|
| `Color` | `string` | already a string alias upstream |
| `GridLength` | `string` | `number \| "Auto" \| "*" \| \`${number}*\`` has no NX spelling yet; every demo writes the string form. Should become a discriminated union. |
| `string \| GridLength[]` (`ColumnDefinitions`, `RowDefinitions`) | `string` | the demos write `"*, 2*, Auto"`; the list form is unused |
| `number \| CornerRadius` | `CornerRadius` | the richer member. `CornerRadius={20}` becomes `CornerRadius=<CornerRadius TopLeft=20 ... />` — more verbose, but `ShapesPage` uses the asymmetric form and a number-only mapping would have lost it |
| `(SkiaShadow \| Partial<SkiaShadow>)[]` | `SkiaShadow[]` | the two members describe the same shape |
| `LayoutType \| ShapeType` (`SkiaLayout.Type`) | `SkiaLayoutType`, their merged case list | NX has no untagged unions; the two case sets do not overlap |

Record fields are optional for the same reason component properties are, including fields DrawnUI
treats as required — `SkiaGradient.Colors` among them. A gradient with no colors draws wrong rather
than failing to compile.

## `DrawnNode`, a root DrawnUI does not have

Content properties are typed `DrawnNode[]?`. `DrawnNode` is invented here: `TextSpan` is a legal
child of `SkiaLabel` but is not a `SkiaControl`, so there is no upstream type that covers both.
Rooting the hierarchy one level above `SkiaControl` lets `TextSpan` be content without granting it
the fifty properties `SkiaControl` carries.

## Bases that had to be split

Only abstract components may be extended in NX, so a control that is both a registered tag and the
base of another tag is emitted twice: an abstract `SkiaLayoutBase` carrying the properties, and a
concrete `SkiaLayout` extending it. This affects `SkiaLayout`, `SkiaLabel` and `SkiaShape`. The
`...Base` names are an artifact of that rule and are never written in NX source.

## Restated properties folded into their base

TypeScript lets a subclass restate an inherited member; NX rejects a redeclared inherited prop. In
each case below the ancestor's declaration is the wider of the two, so the restatement is dropped.

| Property | Resolution |
|---|---|
| `SkiaRichLabel.FontSize` | inherited from `SkiaLabel` |
| `SkiaRichLabel.Text` | inherited from `SkiaLabel` |
| `SkiaShape.Type` | inherited from `SkiaLayout` |
| `SkiaToggle.AccessibilityIsPressed` | inherited from `SkiaControl` |

The one with an effect is `SkiaShape.Type`, which narrows `SkiaLayout.Type` to shape cases only. In
the catalog a `SkiaShape` accepts layout cases too; passing one draws nothing useful, and NX will
not stop you.

## Properties with no NX expression

25 event handlers are omitted, since the TypeScript IR runtime has no action dispatch:
`SkiaButton.Down`, `SkiaButton.Up`, `SkiaCarousel.SelectedIndexChanged`, `SkiaControl.ChildTapped`, `SkiaControl.ConsumeGestures`, `SkiaControl.Tapped`, `SkiaDrawer.IsOpenChanged`, `SkiaDrawer.StateTransitionComplete`, `SkiaHotspot.Down`, `SkiaHotspot.Up`, `SkiaImage.Error`, `SkiaImage.Success`, `SkiaRichLabel.LinkTapped`, `SkiaScroll.LoadMoreCommand`, `SkiaScroll.LoadMoreTopCommand`, `SkiaScroll.Scrolled`, `SkiaSlider.EndChanged`, `SkiaSlider.StartChanged`, `SkiaSvg.Error`, `SkiaSvg.Success`, `SkiaToggle.Toggled`, `SnappingLayout.Scrolled`, `SnappingLayout.Stopped`, `SnappingLayout.TransitionChanged`, `TextSpan.Tapped`.

The rest:

| Property | TypeScript type |
|---|---|
| `SkiaControl.BindingContext` | `unknown` |
| `SkiaLayout.ItemsSource` | `readonly unknown[]` |
| `SkiaLayout.ItemTemplate` | `() => SkiaControl` |

`ItemsSource` and `ItemTemplate` are what `Cells` and `UnevenCells` are built on, which is why those
examples are ported as `reduced` over a short fixed list.

## Writing NX against this catalog

A **user-defined component must extend `DrawnNode`** to be usable as content — content properties
are typed as a list of `DrawnNode`, and a plain `component <Card ... />` is not one:

```nx
component <Card extends DrawnNode Title:string content Children:DrawnNode[] /> = { ... }
```

Its content property must be **non-optional** if the component splices it: `DrawnNode[]?` fails with
`expected DrawnNode[]?, found list object[]`.

Both rules come from how the catalog is shaped, not from NX. See `FINDINGS.md` F15 and F16.

## Edits to the vendored DrawnUI source

None. The catalog is generated from the tree exactly as `npm run sync-drawnui` copied it. If that
changes, the edit belongs in this section, because re-running the sync will overwrite it.
