## 1. Sampling contract and adapter feasibility

- [x] 1.1 Add an independent analytic gradient sampling corpus covering linear projection, radial distance, endpoint clamping, duplicate offsets and premultiplied encoded-sRGB alpha; verify exact samples including opaque-red/transparent-blue midpoint and hard-stop tie behavior.
- [x] 1.2 Build a minimal Skia/Canvas interpolation probe using that corpus on explicit sRGB surfaces; record the required adapter settings and verify interior premultiplied channel error is at most 2/255 for unequal-alpha transitions, hard stops and transformed text. Resolve adapter differences with compatible bounded sampling before proceeding; verify any temporary raster allocation respects peak resource accounting.

## 2. NX authoring contracts and runtime evaluation

- [x] 2.1 Add GradientStop and Paint.linearGradient/radial declarations with explicit local coordinates and documented constraints, preserving solid/none; regenerate checked-in library artifacts by the repository workflow and verify imported alias/default checking and `cargo test -p nx-codegen graphics_`.
- [x] 2.2 Extend Rust and TypeScript IR fixtures for named reusable paints, Paint-returning functions, shape helpers, fill, stroke, text and background; verify equivalent evaluated typed values with the runtime graphics suites, including `pnpm --dir runtime/typescript build` and `node runtime/typescript/test/graphics.test.mjs`.

## 3. Neutral preparation and host transport

- [x] 3.1 Introduce tagged neutral prepared paints and migrate Node, Stroke and Destination contracts; verify round-trip serialization retains geometry, stop ordering and float32 Color precision and that neutral graphics still has no Skia dependency.
- [x] 3.2 Implement complete gradient validation and contextual diagnostics using shared Color validation; verify `cargo test -p nx-graphics` covers malformed raw SDK records, missing/one stop, descending/out-of-range/non-finite offsets, invalid channels, coincident endpoints, nonpositive radius and backend range failures, including invisible content.
- [x] 3.3 Add the scene-wide gradient_stops limit with default 100000 and positive-integer validation; update native CLI/config and WASM host handling, and verify boundary cases, shared-paint occurrence accounting, hidden paints and rejection before excess resource allocation.
- [x] 3.4 Centralize potential-visibility handling for solid and gradient paints and retain conservative bounds through transparency; verify all-transparent paints yield empty painted bounds while partially transparent gradients retain conservative bounds and existing opacity/clip geometry remains correct.
- [x] 3.5 Migrate WASM native scene transport, TypeScript SDK exports, playground protocol aliases, preview schemas and profile checks to graphics-core-v3; verify bindings type-check, schema fixtures pass and unsupported-profile consumers reject changed payloads explicitly.

## 4. Native and browser painting

- [x] 4.1 Implement Skia shaders for prepared linear/radial paints in path fills, strokes, retained shaped text and viewport backgrounds using the sampling settings established in 1.2; verify native corpus output with `cargo test -p nx-graphics-skia --features native` and canvas-state restoration on injected destination failure.
- [x] 4.2 Implement Canvas fillStyle/strokeStyle gradient handling and preserve element-local text paint coordinates through reference-font translation/scale; verify browser fixtures cover multiple text sizes/anchors, shared paints, stroke dashes, nonuniform transforms, nonzero viewBox origins and contain/cover/fill placement.
- [x] 4.3 Verify gradient rendering through clips and isolated nested opacity without exceeding surface/peak-layer budgets; run native and browser composite fixtures and injected rendering/allocation failure cases to confirm no successful partial frame is published.

## 5. Inspection, selection and authoring examples

- [x] 5.1 Expose paint kind, local geometry and ordered original stop values in native inspection and the playground inspector; verify revision checks, precision retention, helper source provenance and existing source-selection behavior in integration fixtures.
- [x] 5.2 Update browser hit testing to use potential visibility and existing geometric fill/stroke tests; verify transparent intervals remain selectable, all-transparent paints do not hit, clips still apply and every element remains selectable through the dropdown.
- [x] 5.3 Add checked-in gradient illustration, transparent area-chart and reusable-paint/helper examples; update the graphics reference with interpolation, validation, limits, coordinates, selection and exclusions, and verify each example evaluates and previews under both supported IR paths.

## 6. Integrated acceptance and migration verification

- [x] 6.1 Extend shared native/browser raster fixtures with the analytic corpus, fixed interior tolerance and edge exclusions specified in the design; verify every supported paint location, hard-stop side, alpha case, transform, root placement and compositing case passes without model calls or network assets.
- [x] 6.2 Extend preview replacement regressions for invalid gradient data and exhausted budgets; verify native current.json remains unchanged on failure and browser diagnostics prevent a stale frame from appearing current using the existing preview and browser-check scripts.
- [x] 6.3 Finish graphics-core-v3 profile/schema/fixture migration, version renderer configuration if required by the interpolation probe and remove obsolete gradient-unsupported expectations; verify `node scripts/verify-graphics-fixtures.mjs`, `python3 scripts/check-graphics-preview.py`, the playground graphics tests and `sites/playground/scripts/check-graphics-browser.mjs` through their documented invocation, plus the focused Rust/runtime/bindings suites above.
