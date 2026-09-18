# Verification record: NX in the DrawnUI fiddle dev host

Run on 2026-09-14 against `Fiddle.DevHost` at `http://localhost:5041/app` with `node
dev/local-backend.mjs` on port 5299, the NX runtime built from this checkout
(`npm run runtime -- --nx ../nx`), in headless Chromium through the engine's `window.fiddle` API and
Monaco's own API. The driver script is reproducible from the description below; every step of the
`docs/ADDING-A-LANGUAGE.md` checklist was exercised and screenshots were taken of each preset and of
the player.

| Check | Result |
| --- | --- |
| Language switch offers C#, React, NX in that order; `fiddle.listPresets()` lists the four NX presets with `lang` `nx` | pass |
| No request for `nx/nx-runtime.js` or `nx/nx.wasm` until NX is used | pass |
| Switching to NX loads the Welcome starter and draws it (`✓ Compiled and rendered`) | pass |
| The runtime bundle and the compiler module are requested once; a second run requests nothing | pass |
| A snippet with `Text=1.0` fails with `L2: Property 'Text' on 'SkiaLabel' expects string?, found float64` under the editor | pass |
| The same error is a squiggle at line 2, columns 14–22, severity error | pass |
| After fixing the snippet HotReload auto-compiles it clean and the markers clear | pass |
| A `"#FF8800"` literal shows a color swatch | pass |
| Completion after `<` offers the catalog's components (alphabetical, with their declarations as detail) | pass |
| Hover over `FontSize` shows `(property) SkiaLabel.FontSize: float64?` | pass |
| C# Welcome, React Welcome, NX Text, NX Shapes, NX Layouts each run without errors and redraw their surface | pass |
| Gestures `Enabled` / `Lock` switches with NX on the canvas | pass |
| A share posted with `lang: "nx"` and the compiled module round-trips `X-Fiddle-Lang: nx` | pass |
| `/p/{id}` draws the share, posts `ready` after the first drawn frame, produces a thumbnail | pass |
| The player's network log shows `nx-runtime.js` and not `nx.wasm` | pass |
| The share's editor link opens in NX with the same source | pass |

Not exercised by hand: opening an NX share on a host that has not registered NX. The dev host
registers NX, and the engine's existing unknown-language path (`LanguageOfShare` returning null
shows the source and says the language is unknown) is unchanged by this change.

## Share artifact sizes

Measured with `fiddleReactEmit(code, 'nx')` for each preset; the artifact is the compiled module,
compact NX IR included. The local backend stores them without a limit. The private drawfiddle.com
backend's limit has not been confirmed with its owner; that is the open question recorded in the
design and in the fiddle README's NX section.

| Preset | Source | Artifact |
| --- | --- | --- |
| Welcome | 1,428 B | 899,531 B |
| Text | 6,563 B | 989,073 B |
| Shapes | 7,884 B | 1,072,972 B |
| Layouts | 6,953 B | 1,019,201 B |

## Automated tests

- NX repository: `cargo test --workspace`, `pnpm -r test` (Rust, wasm SDK with the new prelude
  build and trap tests, Node SDK parity, IR runtime, Monaco with the 0.52 namespace test, playground
  with the example check), `dotnet test bindings/dotnet/NxLang.sln`, `pnpm run verify:packages`
  (the six publishable workspace packages packed and installed into a scratch project),
  `node scripts/pack-packages.mjs` plus `publish-packages.mjs --dry-run`, `actionlint` on the three
  workflows: all green.
- Fiddle repository: `node dev/build-nx-runtime.mjs --nx ../nx` type-checks `nx/`, runs the
  renderer, compiler, diagnostics, trap-recovery and preset tests (15 tests), and bundles
  `nx-runtime.js` (871 KB) next to `nx.wasm` (1,915 KB); `dotnet build src/Fiddle.DevHost` succeeds.
