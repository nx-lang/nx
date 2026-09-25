## REMOVED Requirements

### Requirement: The unit type is inference-internal and has no source spelling
**Reason**: Every clause of this requirement is superseded by `conditional-result-types`. It named
the three forms that take the unit type — an `if` with no `else`, a block with no trailing
expression, and a match that may match nothing — and this change retypes all of them: no items in a
sequence position, the nullable form of the present branches in a value position. With no construct
assigning it, the unit type has no producer to describe.

Its second clause, that the system "SHALL continue to render the unit type as `void` in
diagnostics", is also no longer safe. `empty-list-spelling` freed `void` for user declarations in
the same change that added this requirement, so an inferred type rendered as `void` can name two
different types identically in one message — `expects void, found void` is reachable in three
lines. `conditional-result-types` replaces the clause with the stronger rule that no diagnostic
names an inferred type `void`, and that any internal type still rendered in a message is rendered as
something that is not a legal identifier.

**Migration**: None for authors. `void` was already unwritable in type position and stays that way:
the guarantee that it resolves as an ordinary named type rather than a primitive lives in "NX
defines exactly one spelling for each primitive type", which this change does not touch. The only
observable difference is that diagnostics which used to report `void` now report the conditional's
actual type, and that a conditional with no `else` binds correctly at a nullable site where it was
previously rejected.
