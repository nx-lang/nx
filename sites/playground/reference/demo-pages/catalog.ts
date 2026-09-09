/**
 * The demo's sample pages: one entry per route. Single source of truth for the root menu cards (RootPage)
 * and for the crawlable HTML list that vite.config.ts generates into index.html at build time.
 * Plain data, no React: it is imported from the Vite config too.
 */
export const SAMPLES: { route: string; title: string; text: string }[] = [
  { route: "cells", title: "Recycled cells", text: "100 000 items in a SkiaScroll, RecyclingTemplate + MeasureFirst, UseCache=Image" },
  { route: "uneven", title: "Uneven cells", text: "Rows of different heights — MeasureVisible, LoadMore at both ends, ImageDoubleBuffered cells" },
  { route: "images", title: "Images", text: "SkiaImage — every TransformAspect, alignment, clipping" },
  { route: "svg", title: "SVG", text: "SkiaSvg — file and inline sources, TintColor, LockRatio" },
  { route: "shapes", title: "Shapes", text: "SkiaShape — rectangle, circle, ellipse, arc, polygon, line, path; stroke, corner radii, clipping" },
  { route: "text", title: "Text", text: "SkiaLabel — word wrap, MaxLines, alignment, spans, weights, glyph fallback" },
  { route: "layouts", title: "Layouts", text: "Every SkiaLayout type — Absolute, Column, Row, Wrap, Grid (tracks, spans, spacing)" },
  { route: "looks", title: "Common Controls", text: "SkiaSwitch, SkiaCheckbox, SkiaRadioButton, SkiaProgress, SkiaSlider, SkiaButton — Default, Windows, Cupertino, Material, Material3" },
  { route: "snapping", title: "Carousel & Drawer", text: "SkiaCarousel (swipe, SidesOffset peek, SelectedIndex) and SkiaDrawer (drag from an edge, snap by velocity)" },
  { route: "animations", title: "Lottie & GIF", text: "SkiaLottie (Skottie: AutoPlay, Repeat, SpeedRatio, IsOn, ColorTint) and SkiaGif frames on the canvas frame loop" },
  { route: "shell", title: "Shell", text: "SkiaShell — page transitions, OpenPopupAsync, PushModalAsync (drawer), ShowToast" },
  { route: "editor", title: "Editor", text: "SkiaEditor — drawn text input: caret, selection, placeholder, password, multiline, ControlStyle looks" },
  { route: "keyboard", title: "Keyboard Input", text: "KeyboardManager — window-level KeyDown / KeyUp / KeyChar with modifier state, the Blazor sandbox probe" },
  { route: "scroll", title: "SkiaScroll", text: "Header in flow / sticky / behind with parallax, Footer, scroll bars, pull to refresh, SnapToChildren, TrackIndexPosition" },
  { route: "shaders", title: "Shaders", text: "SkiaShaderEffect — SkSL on any control (iImage1, iTime, iMouse, custom uniforms, touch ripples) and SkiaShaderCarousel gl-transitions" },
  { route: "sprites", title: "Sprites", text: "SkiaSprite spritesheets and a SkiaSpriteSet warrior on a tile board, moved with the keyboard (FastRepro sprites)" },
  { route: "transforms", title: "Transforms", text: "Rotation, Scale, Skew, Translation, Opacity — hit-testing through them, *ToAsync animations" },
  { route: "reorder", title: "Drag to reorder", text: "Language preferences, Android style: drag one by its grip and it lifts and floats over the list, which reorders live under it keeping its measured heights and its scroll offset, then the drop glides into the new slot" },
  { route: "a11y", title: "Accessibility", text: "ARIA overlay over the canvas — roles, labels, hints, toggles, live regions, keyboard" },
];
