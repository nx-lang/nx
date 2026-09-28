## Context

`interface_union` converts a library's published union into a `UnionDef` for consumers, setting every
field's `default` to `None` because an interface carries no expressions. The checker then computed
`is_required = default.is_none() && !optional`, so defaulted fields became required. Records avoid this
because `EffectiveField::from_interface_field` copies the interface's `is_required`.

## Goals / Non-Goals

**Goals:** a consumer treats an imported union case's defaulted field as omissible, and the value
carries the default.

**Non-Goals:** carrying default expressions across the interface; the declaring module evaluates them.

## Decisions

### D1. Record that a default exists, separately from its expression
`UnionCaseField` gains `has_default: bool`, set from the expression where one exists and from the
interface's `is_required`/`optional` flags for an imported field. `UnionCaseField::is_required()`
(`!has_default && !optional`) is the one rule the checker and the published interface both use.

*Alternative*: look the interface item up at the check site. Rejected: it spreads interface lookups
into the checker for a fact the lowered field can simply carry.

## Risks / Trade-offs

- [Other readers of `default` for imported unions] → Codegen and evaluation read defaults only in the
  declaring module, where the expression exists.

## Migration Plan

None; sources that failed now validate.

## Open Questions

- None.
