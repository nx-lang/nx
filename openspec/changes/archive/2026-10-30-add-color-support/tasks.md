## 1. Shared contract and numerical corpus

- [x] 1.1 Add the documented unstable `@nx/color` source module with float32 Color, ParseResult, channel records, InterpolationSpace, named constants, and the specified public functions; verify library registration, aliased imports, defaults, and analysis without diagnostics in nx-api standard-library tests.
- [x] 1.2 Add a renderer-independent Rust color utility implementing the specified standalone CSS grammar, float32 narrowing/validation, and formatters; verify a versioned corpus covers all names, hex lengths, alpha order, whitespace, unsupported syntax, non-finite/range failures, rounding boundaries, and exact float32 CSS round trips.
- [x] 1.3 Add the equivalent DOM-independent TypeScript color utility and run the same corpus; verify accepted inputs, numerical results, and error categories agree with Rust.

## 2. Expanded operation corpus

- [x] 2.1 Implement Rust/TypeScript RGB, HSL, HSV, HWB, linear-sRGB, Oklab/Oklch constructors and inverse conversions; verify independent reference vectors, hue wrapping, achromatic hue, HWB normalization, gamut clipping, overflow rejection, and <=2e-6 nonlinear parity.
- [x] 2.2 Implement immutable alpha/lightness/saturation/hue operations, complement, grayscale, and invert; verify endpoints, absolute units, clamping rules, alpha preservation, invalid inputs, and no mutation.
- [x] 2.3 Implement premultiplied mixing in the three specified spaces and linear-sRGB source-over; verify endpoints, transparent hidden RGB, all-transparent output, and encoded-versus-linear reference results.
- [x] 2.4 Implement WCAG luminance, contrast ratio, and bestTextColor; verify black/white ratio 21, equal-color ratio 1, transfer breakpoint, candidate ties, unrounded comparisons, and explicit rejection of translucent inputs.

## 3. Syntax, typing, and lowering

- [x] 3.1 Add hex literal grammar, parser/AST nodes, and diagnostics/recovery; regenerate parser artifacts and verify all four lengths, invalid-token spans, comments/strings, token boundaries, element properties, and ordinary expression positions.
- [x] 3.2 Infer canonical Color through standard-library registry identity without imports, record implicit module dependencies, and lower literals to ordinary record constants; verify shadowing, aliases, missing modules, cycle prevention, defaults, sequences, and literal-only evaluation without color intrinsics.
- [x] 3.3 Update syntax highlighters, formatter, source/value printers, and nx-grammar.md; verify source spelling round trips and lossless printing of colors not representable as hex bytes.

## 4. IR runtime and host transport

- [x] 4.1 Implement Rust IR standard-color-v1 function dispatch and feature checks, preserving source diagnostics; verify parse, tryParse, direct-invalid-record validation, aliases, and missing-feature/module tests.
- [x] 4.2 Implement TypeScript IR standard-color-v1 dispatch and emitted/executable runtime integration; verify the Rust parity fixtures and a color-only program without graphics dependencies.
- [x] 4.3 Integrate library image/version discovery and transitive linking through compiler/WASM APIs; verify missing images fail explicitly and color-only consumers require no renderer.
- [x] 4.4 Verify ordinary Color records/defaults through schema/type generation and Rust, TypeScript, WASM, and supported SDK JSON/MessagePack boundaries; add round-trip coverage and correct exporters only where those tests reveal gaps, without adding new generic value tags.

## 5. Semantic discovery and diagnostics

- [x] 5.1 Expose canonical color type/function identities and statically recognized color expressions in language-service metadata and binding APIs; verify aliases/re-exports, hex literals, direct records, default alpha, and same-name user declarations.
- [x] 5.2 Add literal parse/channel diagnostics using the shared utility; verify diagnostic codes/spans match runtime rules and that tryParse literals and computed expressions are not speculatively diagnosed or executed.
- [x] 5.3 Add revision-checked color edit operations for hex literals, parse literals, rgb/rgba/srgb arguments, and safe direct record fields; verify comments, alpha, unaffected precision, unsupported expressions, and stale revisions with source-edit tests.

## 6. Viewer and playground tooling

- [x] 6.1 Extend packages/viewer and playground Source/Visual integrations to show canonical record colors and typed evaluated values as swatches; verify color-only fixtures and existing string format=color fixtures both work.
- [x] 6.2 Add alpha-aware picker behavior using the color edit operations; verify unchanged open/close, RGB-only alpha preservation, explicit alpha changes, normal undo, stale edits, and read-only computed values in viewer tests.
- [x] 6.3 Exercise Source/Visual color editing in browser integration coverage without opening a graphics preview; verify source revisions, diagnostics, swatches, and keyboard-accessible controls.

## 7. Graphics integration

- [x] 7.1 Replace/re-export graphics Color with the canonical library type and update transitive dependencies; verify compiler/runtime graphics tests accept shared colors and reject direct strings.
- [x] 7.2 Replace authored-string parsing in neutral preparation with record/channel validation and adapt Skia and Canvas destinations; verify invalid later elements fail before painting and shared translucent colors preserve numerical channels on both backends.
- [x] 7.3 Advance graphics profile metadata to graphics-core-v2 and migrate native/browser prepared-scene schemas, examples, fixtures, expected values, and manifests; verify profile consistency, native rendering checks, and the browser graphics checks.

## 8. Adoption, documentation, and integration verification

- [x] 8.1 Migrate the checked-in survey example and design-token tutorial to shared colors and authored hex literals, including explicit CSS conversion for string host APIs; verify the survey analyzes/evaluates without any graphics import and regenerate its expected output where present.
- [x] 8.2 Publish a color API reference and graphics migration notes covering literal syntax, import-free resolution, CSS parsing subset, RGB versus normalized channel units, float32 rounding, transformations/mix/composite/contrast semantics, public-record validity limits, constructor/picker examples, external-data tryParse, runtime availability, and external ReachMe migration; verify documented snippets compile on the stated IR paths.
- [x] 8.3 Run affected Rust library/compiler/runtime/language-service tests, TypeScript runtime/viewer tests and builds, WASM/binding checks, and native/browser graphics fixture checks; record commands/results and any platform-specific limitations in the change's implementation notes.
