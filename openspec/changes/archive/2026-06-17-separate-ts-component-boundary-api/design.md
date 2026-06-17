## Context

Executable TS/JS component codegen currently emits a class-oriented API where the same TypeScript
name is used for a component value type and a static host API class. That API is useful at a JSON
boundary, but it makes ordinary TypeScript clients work with `NxValue`, `normalizeProps`, and
`initialize`/`evaluate` methods even when all inputs are already strongly typed.

NX also has two different component roles that the generated API should make visible. Normal NX
components have render bodies and can be evaluated by generated TypeScript. External components
have no render body; they are serializable UI element values that a client renderer maps to
framework-specific UI.

## Goals / Non-Goals

**Goals:**

- Make the primary generated TypeScript component surface strongly typed and React-adjacent.
- Keep JSON validation, conversion, diagnostics, and `NxValue` handling behind explicit
  `Schema`-suffixed boundary values or adapter APIs.
- Use `Props` for caller input types, including optional fields where NX defaults exist.
- Use `Element` for generated external component values, e.g. `TextInputElement`.
- Treat normal component calls as render-body evaluation, while external component calls construct
  external element values.
- Share JSON normalization through reusable schema/runtime helpers rather than emitting bespoke
  validation statements for every field.

**Non-Goals:**

- Generate React components or browser/client adapters.
- Add reactive scheduling, hook state, subscriptions, dispatch, or action-handler invocation to
  executable TS/JS codegen.
- Change the canonical JSON wire shape for component values; external elements still carry `$type`
  and normalized fields.
- Change the `nxlang typegen` DTO-only surface as part of this proposal.

## Decisions

### 1. Generate typed component functions as the primary TypeScript API

Normal concrete components should emit an exported function named after the component. The
function accepts a generated `Props` type and returns the statically known rendered output type
when that type can be expressed. It does not accept `NxValue` and does not expose JSON diagnostics.
The return type should follow the same TypeScript type conventions as ordinary generated NX
functions: use the generated name for a named NX return type when one exists, otherwise emit the
structural return type directly. Do not generate a public `<ComponentName>Output` alias by default;
`output` is reserved for other NX concepts and should not name the component render result API.

For stateful components, generated typed helpers should keep state explicit without turning the
main function into the JSON boundary. A component can expose a typed initial-state helper and a
typed render helper, either as named functions or as namespace members on the component function.

Alternatives considered:

- Keep generated classes and add more typed overloads. Rejected because the class remains the
  visible center of the API and keeps the JSON boundary mentally primary.
- Make `SearchBox(props, state?)` the only API. Rejected because it is less React-like and hides
  the difference between initial rendering and rendering with caller-managed state.

### 2. Split caller props from normalized/internal props

Generated `Props` types describe what a typed caller may pass. Props with NX defaults should be
optional in this type. Generated code may also use an internal normalized props type where defaulted
fields have been materialized. Public `Props` members should be mutable by default, matching common
React TypeScript practice and making dynamically built prop objects ergonomic for typed callers.
Generated code should treat props as input values and avoid mutating them, but it does not need to
encode that convention with `readonly` on the caller-facing `Props` type.

Example:

```ts
export type QuestionFlowProps = {
  title?: string;
};

type QuestionFlowResolvedProps = {
  readonly title: string;
};
```

The `Props` type is the public typed caller surface and should always be emitted for generated
components. The resolved props shape is a generated implementation detail: emit a private
`<ComponentName>ResolvedProps` type only when it is reused by generated internals such as render
helpers, state initialization, inheritance helpers, or schema adapters. For tiny components with a
single helper, the emitter can inline the resolved object type instead. Prefer `ResolvedProps` over
`NormalizedProps` so the typed API does not borrow JSON-boundary terminology. Internal resolved
props, generated state types, render result object types, and external element value types should
remain `readonly` because those are produced or owned by generated code rather than caller-side
builder inputs.

### 3. Use `Element` for external component values

External components should emit a `Props` type, an `Element` type, and an unsuffixed factory
function. The function is the typed way to construct the external element value.

```ts
export type TextInputProps = {
  id: string;
  label: string;
  value?: string;
};

export type TextInputElement = {
  readonly $type: "TextInput";
  readonly id: string;
  readonly label: string;
  readonly value: string;
};

export function TextInput(props: TextInputProps): TextInputElement {
  return {
    $type: "TextInput",
    id: props.id,
    label: props.label,
    value: props.value ?? "",
  };
}
```

This mirrors UI terminology: `TextInput` is the component factory from the typed caller's point of
view, and `TextInputElement` is the rendered serializable element that a client UI renderer
understands. Avoid `Descriptor` in public generated names because it is accurate but too
implementation-flavored.

### 4. Move JSON support into `Schema`-suffixed values

Each generated type that participates in JSON or host-boundary validation should have a
`Schema`-suffixed value. The schema is runtime metadata plus shared boundary operations. It should
be generic enough for the runtime helper to validate primitives, arrays, records, enums,
discriminated unions, external elements, component props, component state, defaults, unknown fields,
and diagnostic paths.

```ts
export const TextInputSchema = nxExternalComponentSchema<TextInputProps, TextInputElement>({
  name: "TextInput",
  props: nxRecordSchema({
    id: nxField(nxStringSchema),
    label: nxField(nxStringSchema),
    value: nxField(nxStringSchema, { default: "" }),
  }),
  create: TextInput,
});
```

Boundary callers use schema APIs instead of the typed component function directly:

```ts
const element = TextInputSchema.fromJson(input);
const result = QuestionFlowSchema.tryEvaluateJson(propsJson, stateJson);
```

The schema object may call generated typed functions after validation. The typed functions should
not depend on `NxValue` or schema diagnostics.

### 5. Generate normal component calls as render calls, external component calls as element factories

Inside generated TypeScript, an element expression targeting a normal component should call the
normal component's typed render path. An element expression targeting an external component should
call that external component's typed element factory.

This changes the current descriptor-first rule. The new rule is based on the NX declaration kind:

- normal component: evaluate the component body;
- external component: construct an external element value.

This preserves server-driven UI handoff because leaves intended for the client are declared
`external`, while allowing generated TS callers to compose normal components like ordinary
component functions.

### 6. Keep schema validation mostly metadata-driven

Most JSON validation can be reflection-like metadata interpreted by shared helpers:

- primitive kinds and nullable/optional wrappers;
- arrays/lists;
- record fields, defaults, and unknown-field rejection;
- enums and discriminated unions;
- component `$type` discriminators;
- external element props;
- state JSON shapes;
- diagnostic path construction.

Generated per-declaration code is still needed for:

- render bodies and expression evaluation;
- defaults derived from props or other expressions;
- typed factory functions that materialize normalized values;
- state initialization that depends on props;
- action-handler semantics, once supported;
- specialized performance paths if the generic schema walker becomes too slow.

## Example Generated Output

For source shaped like:

```nx
external component <TextInput id:string label:string value:string = "" />

component <QuestionFlow title:string = "Profile" /> = {
  {
    <TextInput id="firstName" label="First name" />
    <TextInput id="lastName" label="Last name" />
  }
}
```

the generated TypeScript should be shaped like:

```ts
import {
  nxComponentSchema,
  nxExternalComponentSchema,
  nxField,
  nxRecordSchema,
  nxStringSchema,
} from "./nx-runtime.js";

export type TextInputProps = {
  id: string;
  label: string;
  value?: string;
};

export type TextInputElement = {
  readonly $type: "TextInput";
  readonly id: string;
  readonly label: string;
  readonly value: string;
};

export function TextInput(props: TextInputProps): TextInputElement {
  return {
    $type: "TextInput",
    id: props.id,
    label: props.label,
    value: props.value ?? "",
  };
}

export const TextInputSchema = nxExternalComponentSchema<TextInputProps, TextInputElement>({
  name: "TextInput",
  props: nxRecordSchema({
    id: nxField(nxStringSchema),
    label: nxField(nxStringSchema),
    value: nxField(nxStringSchema, { default: "" }),
  }),
  create: TextInput,
});

export type QuestionFlowProps = {
  title?: string;
};

type QuestionFlowResolvedProps = {
  readonly title: string;
};

function resolveQuestionFlowProps(props: QuestionFlowProps = {}): QuestionFlowResolvedProps {
  return {
    title: props.title ?? "Profile",
  };
}

export function QuestionFlow(props: QuestionFlowProps = {}): readonly TextInputElement[] {
  const resolvedProps = resolveQuestionFlowProps(props);
  return renderQuestionFlow(resolvedProps);
}

function renderQuestionFlow(props: QuestionFlowResolvedProps): readonly TextInputElement[] {
  const title = props.title;
  return [
    TextInput({ id: "firstName", label: "First name" }),
    TextInput({ id: "lastName", label: "Last name" }),
  ];
}

export const QuestionFlowSchema = nxComponentSchema<
  QuestionFlowProps,
  readonly TextInputElement[]
>({
  name: "QuestionFlow",
  props: nxRecordSchema({
    title: nxField(nxStringSchema, { default: "Profile" }),
  }),
  evaluate: QuestionFlow,
});
```

If `QuestionFlow` later declares state, the typed layer should add `QuestionFlowState`,
`initialQuestionFlowState`, and a typed render helper that receives resolved props and state. The
`QuestionFlowSchema` should add JSON state validation and JSON initialize/evaluate methods without
changing the typed component function into a JSON API.

## Risks / Trade-offs

- [Generated TS diverges from generated JS ergonomics] -> Keep JS behavior semantically equivalent
  while treating the richer typed API as TypeScript-only syntax over the same emitted operations.
- [Schema walkers are slower than custom validation] -> Start with metadata-driven validation for
  clarity and sharing, then allow generated fast paths behind the same schema API if needed.
- [Component recursion can surprise callers] -> The render-vs-element rule is declaration-kind
  based and testable: only external components stop as serializable elements.
- [Generated names conflict with source declarations] -> Reuse the existing generated-name
  conflict machinery and reserve `Props`, `Element`, `State`, and `Schema` suffixes for
  component-related generated names. Avoid `Output` as a generated component suffix.
- [Boundary APIs become harder to find] -> Export schemas consistently and document that JSON
  callers enter through `*Schema` values.

## Migration Plan

1. Update codegen model/emitter terminology to distinguish normal component render functions from
   external element factories.
2. Emit `Props`, `Element`, `State`, and `Schema` names with collision handling.
3. Add generic schema runtime helpers for records, fields, primitives, enums, unions, component
   props, component state, and external element creation.
4. Refactor generated component validation to use schema values for JSON boundary APIs.
5. Update component codegen tests and golden snapshots to assert typed APIs, schema boundaries, and
   external `Element` output.
6. Roll back by restoring class-based component entry generation and descriptor-first component
   expression emission if the typed API split proves too disruptive before release.

## Open Questions

- Should schema values expose `initializeJson`/`evaluateJson` directly, or should component schemas
  only expose lower-level `fromJson` helpers plus a separately named boundary adapter?
- Should generated TypeScript export resolved props types for advanced callers, or keep them
  private until a concrete use case needs them?
- Should `QuestionFlow(props)` always use initial state for stateful components, or should stateful
  components require callers to use a named typed render helper when state matters?
