# Unstable graphics-core delivery

Added `@nx/graphics`, ordinary math-backed RegularPolygon/Star/Arrow/Arc/RingSector helpers,
neutral preparation/inspection/direct destination contracts and optional Linux Skia rendering.
The preview host links emitted images and runs Rust IR directly; the same authoring helpers are
verified on TypeScript IR. HIR interpreter graphics support will not be updated in this change.

Pinned upstream skia-safe 0.153.3, supplied font bytes/license, reusable shaped text measurement,
failure-safe PNG publication and revision-scoped manifests. Unsupported profile features produce
diagnostics. The graphics reference documents commands, defaults, supported targets and limits.
Native builds are opt-in. No stable graphics compatibility promise or empirical model-quality
improvement is claimed.

### Playground

The Graphics example now opens Preview backed by the TypeScript IR runtime and HTML Canvas
2D, with shared neutral preparation through the compiler WASM SDK. It includes supplied-font text,
shape helpers, isolated opacity, clips, source-linked element inspection and revision-safe errors.
No CanvasKit or Rust/Skia browser renderer is required. The component Preview remains available; both presentations now use TypeScript IR. The WASM ABI is 11; rebuild the SDK and module together.

The playground now routes evaluated values to registered previewers. Preview replaces the Drawing
tab and appears only for supported results; Output always exposes the same TypeScript IR value.
User tab choices survive edits. Ordinary records (including Survey) use Output with shared source
selection. Stateful component actions/timeline remain available and update both presentations.

`@nx/visualizer` adds PreviewVisualizer descriptors on ElementVisualizer and FunctionVisualizer
icons for element-style functions. Graphics now owns its preview association and shape icons in NX.
The playground discovers resolved library metadata and enables only bundled renderer IDs; dynamic
plugin installation remains future work. Language source-tree queries can opt into listing all
loaded visualizer targets with includeUnreferencedDeclarations.

### Authoring names

The unstable graphics API uses GraphicsElement/Shape (formerly GraphicsCommon/ShapeCommon),
AxisAlignment (formerly Anchor), and pointCount on RegularPolygon/Star (formerly count).
No old-name aliases are retained. Geometry, SVG-style coordinate names, and rendering behavior
are unchanged. Private helpers were renamed for clarity.
