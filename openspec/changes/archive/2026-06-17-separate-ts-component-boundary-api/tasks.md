## 1. Codegen Model And Naming

- [x] 1.1 Audit current component TypeScript/JavaScript emission paths and identify every place that assumes component entry classes or descriptor-first component expressions.
- [x] 1.2 Add generated-name planning for `Props`, `Element`, `State`, and `Schema` suffixes, including collision handling with source declarations.
- [x] 1.3 Extend the codegen component model so emitters can distinguish normal component render targets from external component element factories.
- [x] 1.4 Define internal generated type roles for caller props, resolved props, component state, component render result types, external elements, and schema values without introducing a public `Output` suffix.

## 2. Typed Component Emission

- [x] 2.1 Emit `Props`, `Element`, and unsuffixed factory functions for concrete external components.
- [x] 2.2 Emit normal component `Props` types, resolved-props helpers, and unsuffixed component functions that evaluate component bodies.
- [x] 2.3 Emit typed state surfaces for stateful normal components without exposing `NxValue` in public typed state APIs.
- [x] 2.4 Update component element expression emission so normal components render through generated typed functions and external components construct `Element` values.
- [x] 2.5 Preserve TypeScript module imports/exports for generated component functions, element types, props types, state types, render result referenced types, and schemas across modules.
- [x] 2.6 Keep JavaScript executable output semantically aligned with TypeScript after removing TypeScript-only syntax.

## 3. Schema Boundary Runtime

- [x] 3.1 Add shared runtime schema helpers for primitive fields, records, defaults, unknown-field rejection, enums, unions, component props, component state, and external elements.
- [x] 3.2 Emit `Schema`-suffixed values for external components and normal components that need JSON validation or JSON boundary operations.
- [x] 3.3 Route JSON prop/state normalization, serialization, diagnostics, and result-returning boundary APIs through generated schema values.
- [x] 3.4 Keep prop-dependent defaults and state initialization in generated typed code while reusing schema metadata for structural JSON validation.
- [x] 3.5 Ensure action-handler bindings remain explicitly unsupported in generated TS/JS codegen until dispatch/effect semantics are designed.

## 4. Tests

- [x] 4.1 Update existing component codegen golden tests that assert class-based APIs or descriptor-first behavior.
- [x] 4.2 Add TypeScript type-check tests proving typed component functions, `Props`, `State`, `Element`, and `Schema` exports compile without `NxValue` in primary typed props/state APIs and without generating public `Output` aliases.
- [x] 4.3 Add generated-output tests for a normal parent component that renders two external child elements of the same type.
- [x] 4.4 Add parity tests proving normal child components are rendered and external child components remain external element values.
- [x] 4.5 Add schema boundary tests for valid JSON normalization, missing fields, unknown fields, defaults, enum/union/record props, and explicit state JSON.
- [x] 4.6 Add cross-module tests for inherited external component defaults and generated `Element`/`Schema` imports.
- [x] 4.7 Run targeted `nx-codegen` and CLI codegen tests, then run broader workspace tests required by the touched crates.

## 5. Documentation And Migration Notes

- [x] 5.1 Update README or codegen documentation to describe typed component functions and schema-based JSON boundary APIs.
- [x] 5.2 Document the `Element` term for external component values and avoid exposing `Descriptor` as the generated TypeScript suffix.
- [x] 5.3 Add migration notes for callers moving from class-based `SearchBox.initialize/evaluate` APIs to `SearchBox` and `SearchBoxSchema` APIs.
