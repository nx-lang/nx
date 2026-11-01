## 0. Floating finiteness patterns

- [x] 0.1 Add contextual finite pattern syntax and AST/lowering support for float32/float64 where clauses and numeric is arms; verify syntax and type-checker tests cover conjunctions, aliases, declarations named finite, unsupported calls, and invalid integer/nonnumeric uses.
- [x] 0.2 Normalize finite to inclusive bounds at the selected base's maximum finite magnitudes using existing interval machinery; verify signed zeros, subnormals, maximum finite values, infinities, NaN, alias intersections, structural equivalence, widening, joins, and unchanged unconstrained arithmetic-result typing.
- [x] 0.3 Exercise existing interval IR emission and binding checks in shared Rust/TypeScript conformance fixtures, including stripped images and numeric-match narrowing; verify no IR encoding revision or runtime intrinsic is needed, and dynamic violations report nx-ir-constraint at the correct site.
- [x] 0.4 Update constraint diagnostics/help text, hover/source presentation, schema agreement coverage, and language documentation for finite; verify authored spelling, precision-specific finite endpoint round trips, and preservation of existing integer visualizer behavior.

## 1. Shared geometry declarations and registration

- [x] 1.1 Add documented Point, Size, and Rect float64 records in `crates/nx-api/src/std/geometry/geometry.nx`, including finite Point/Rect fields, nonnegative dimensions, potentially infinite Size dimensions, and zero-dimension/coordinate-space conventions; verify the module analyzes without diagnostics and required-field failures are covered by compiler tests.
- [x] 1.2 Register the unstable `@nx/geometry` source library and package/namespace metadata in nx-api; verify direct, aliased, implicit, and SDK-facing imports resolve through existing library entry points without filesystem or host registration.
- [x] 1.3 Add geometry-only image emission/linking tests; verify canonical module ownership, content-version stability across import aliases, absence of graphics/math dependencies, and missing-required-image errors.
- [x] 1.4 Add shared Rust/TypeScript IR fixtures for nested Point/Size/Rect values, function boundaries, zero dimensions, positive-infinite Size dimensions, and large fractional coordinates; verify equivalent values with no float32 narrowing or runtime geometry callback, and static/dynamic rejection of negative dimensions, NaN fields, and infinite Point/Rect fields.

## 2. Graphics source and identity migration

- [x] 2.1 Import geometry in graphics, re-export canonical Point/Size/Rect aliases, and replace ViewBox with Rect on Drawing.viewBox; verify direct geometry values and graphics re-export values are interchangeable in typed viewports and polygon/polyline point sequences.
- [x] 2.2 Rename the drawable Rect external component and visualizer to Rectangle without changing its props or defaults; verify Shape inheritance, its icon, and rounded/stroked/transformed rendering fixtures, plus negative tests for the removed ViewBox export and a Rect value used as a graphical child.
- [x] 2.3 Update nx-codegen graphics declaration adapters and nested-value identity handling for the moved geometry records and Rectangle component; verify canonical identities through aliased imports, helper output, generic holders, and same-named user declarations without accepting spoofed shapes.
- [x] 2.4 Update neutral scene preparation to recognize Rectangle and retain geometry consumer validation; verify zero-area drawable behavior, rejection of zero/negative viewBox dimensions, finite/range checks on raw SDK values, and rejection of geometry Rect records as scene elements.

- [x] 2.5 Annotate graphics.nx public numeric props, record/union fields, and exported helper parameters according to the design's scalar-domain table, using a constrained dash item alias; verify all defaults and omissions analyze and static/dynamic invalid values produce the expected constraint diagnostics in both IR runtimes.
- [x] 2.6 Update helper and graphics negative tests for earlier scalar failures while retaining relational/raw-SDK defenses; verify stop ordering, distinct endpoints, radius/arrow relationships, fractional fontWeight, computed overflow, backend precision checks, and resource limits remain enforced, and signed/zero transforms stay valid.

## 3. Profile, renderers, and preview integration

- [x] 3.1 Revise preparation/transport metadata, native/browser gates, SDK expectations, manifests, and active profile fixtures to graphics-core-v4; verify current consumers accept v4 and unsupported revisions fail explicitly.
- [x] 3.2 Update native preview and browser linking paths for geometry's transitive images; verify a graphics-only import renders a Drawing with shared geometry and Rectangle, while missing required images fail before publishing a preview.
- [x] 3.3 Migrate active graphics test sources, code-generation fixtures, and playground examples semantically; verify geometry Rect constructors are distinguished from drawable Rectangle constructors and internal kurbo types and archived change records remain untouched.
- [x] 3.4 Run cross-renderer regression fixtures for gradients, strokes, transforms, clipping, view-box mapping, text, and inspection/selection provenance; verify equivalent rendered geometry and existing tolerance checks, with Rectangle identified as the authored component.

## 4. Documentation and integrated verification

- [x] 4.1 Add the geometry standard-library reference and navigation/catalog entries with standalone non-graphics examples and property documentation; verify the documentation examples compile and expose only the intended initial three-type API.
- [x] 4.2 Update the graphics reference and examples with Rectangle/Rect migration guidance, shared Point ownership, graphics-core-v4, graphics scalar constraints and earlier failure diagnostics, and unchanged local-unit gradient behavior; verify examples follow the NX style guide and website code-block checks pass.
- [x] 4.3 Run the affected nx-api/nx-codegen, neutral graphics, native Skia, TypeScript IR, SDK, and playground suites using repository-supported test entry points; verify no unresolved migration failures and record any environmental limitations.
- [x] 4.4 Exercise native preview and the gradients playground end to end, then run `openspec validate add-geometry-standard-library --strict`; verify correct v4 manifests, updated source selection, and a valid OpenSpec change before declaring implementation complete.
