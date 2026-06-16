## Context

Generated C# DTOs have runtime construction semantics: if a property initializer is omitted, the
CLR implicit default is used. NX record, action, union case, and external component prop fields can
declare default expressions, and HIR preserves those expressions as `ExprId` values. The current
type generation export model collapses that information to `has_default: bool`, so the C# emitter
cannot distinguish `bool = true` from any other default-bearing field.

TypeScript generation is unaffected because it emits interfaces and type aliases. There is no
runtime initializer surface to preserve there.

## Goals / Non-Goals

**Goals:**

- Carry supported literal default values through the type generation model.
- Emit valid C# property initializers for NX string, integer, floating-point, boolean, and null
  literal defaults.
- Apply the behavior consistently to exported records/actions, exported union case DTO fields, and
  exported external component prop contracts.
- Warn when C# generation encounters an authored default expression that cannot be represented as a
  generated literal initializer.

**Non-Goals:**

- Evaluate arbitrary NX default expressions during type generation.
- Add runtime default factories or constructors for generated DTOs.
- Change TypeScript generated interfaces to represent defaults.
- Preserve defaults on generated external component state contracts; those represent host-managed
  state shape rather than constructor semantics.

## Decisions

1. Represent default values in the typegen model as an optional literal enum.

   The model should replace the field-level `has_default: bool` with an optional default value that
   can represent renderable literals. This keeps language emitters independent of HIR internals and
   avoids threading `LoweredModule` access through every emitter.

   Alternative considered: store raw `ExprId` in `ExportedRecordField`. That would force emitters to
   know how to look up HIR expressions and would make cross-module library generation harder to keep
   self-contained.

2. Treat unsupported default expressions as warning-worthy omissions.

   NX accepts default expressions beyond literals, but C# property initializers can only directly
   express a safe subset without evaluating NX semantics. For unsupported expressions, generation
   should continue and warn that the default was omitted. This preserves existing successful
   generation while making the semantic gap visible.

   Alternative considered: fail generation on unsupported defaults. That would be stricter but could
   break existing generation for fields whose defaults were already being silently ignored.

3. Let authored literal defaults take precedence over `default!`.

   `default!` is only a nullable-analysis suppression initializer for non-null reference types. When
   an authored literal default exists, the generated property should use that actual initializer
   instead.

## Risks / Trade-offs

- Unsupported expression defaults still cannot be preserved exactly → emit explicit warnings so users
  can decide whether to simplify the default or add a hand-written wrapper.
- Literal rendering must be type-aware enough for C# syntax → keep rendering small and covered by
  regression tests for each supported primitive family.
- Warning volume could increase for projects with many expression defaults → include field and
  declaration context in each warning so messages are actionable.
