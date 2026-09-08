# Fiddle check: the `drawnui-fiddle` scenarios in a real browser

Task 7.5 asks for a hand check in `pnpm run dev:all`. It was done headlessly instead — Chromium
driven over the DevTools protocol against the running dev servers — so the observations below are
what the page's DOM reported, not what a person saw. The driver script is not committed; it lived
in the session's scratch directory.

Setup: `pnpm run dev:all` in `sample-apps/drawnui-react`, page `http://localhost:5173/fiddle/cells`.

| Scenario | Observed |
| --- | --- |
| NX is syntax highlighted | 40 rendered lines; token spans in 14 Monaco token classes with 8 distinct colors, produced by the Shiki tokens provider over the published grammar. |
| Hover on a catalog component | Pointer rested on `SkiaLayout`. The hover widget showed `external component <SkiaLayout Children:DrawnNode[]? />` followed by "Inherited from SkiaLayoutBase, SkiaControl, DrawnNode:" and the inherited properties with their types, rendered as highlighted NX (179 highlighted spans inside the widget). Requires D9. |
| Hover on the visitor's own declarations | Not exercised in the browser; covered by `hover_over_a_component_declaration_reports_its_properties` and the route test in `server/language.test.mjs`, which answer through the same handler with the catalog as prelude. |
| Completions inside a catalog tag | Text replaced with `<SkiaLabel Text="hi"  />`, caret moved into the empty slot, Ctrl+Space: the suggest widget listed `AccessibilityCanInteract`, `AccessibilityHint`, `AccessibilityIsPressed`, `AccessibilityLabel`, `AccessibilityLive`, `AccessibilityRole`, `AnchorX`, … — the flattened catalog properties, `Text` not among them. |
| Positions are not shifted by catalog injection | The hover range came back as line 0-based coordinates inside the visitor's document (`server/language.test.mjs` asserts the exact line, column and byte offsets; the browser hover appeared over the hovered tag). |
| Language features degrade without breaking editing | Compile server stopped, Vite alone: the page loaded, highlighting was unchanged (8 colors), typing worked, the page's own `fetch` of `/api/language/hover` answered 502 in 1.0 s from Vite's probe, the client rejected with `LanguageServiceHttpError` which reached `onError` (a `console.debug` line), no exception was thrown, and the hover widget stayed hidden (`offsetParent === null`) for the 20 s watched. Completions fell back to Monaco's word-based suggestions. |

Also exercised:

- `POST /api/language/{hover,completions}` through the Vite proxy and directly against the compile
  server answer identically; a reserved query answers 501.
- A stress pass — every example under `src/examples/nx`, two rounds, hover and completions at a
  dozen positions each plus diagnostics, symbols and a half-typed tag: 720 requests, all 200, in
  2.6 s, server alive afterwards.
- The Docker image builds from the repository root and, run with `-p 18080:8080`, answers `/`,
  `POST /api/compile` and `POST /api/language/hover`.

## One thing seen and not explained

During the session the dev compile server exited once with `SIGSEGV`, while `pnpm -r test` was
running concurrently — that run rebuilds and copies `bindings/node/native/nx_sdk_node.node` over
the file the running server had loaded, which is the likely cause, but the file's final
modification time is ~50 s after the crash line in the log, so the timeline does not prove it.
The 720-request stress pass above, run afterwards against a fresh server, did not reproduce a
crash. Worth watching: if it recurs without a concurrent native rebuild, it is a real bug in the
language snapshot path and should be reported with the request that triggered it.
