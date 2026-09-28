# Release note (draft): report-unresolved-type-names

Entry for the GitHub Release body of the next NX release (`docs/deployment.md`, "Publish A Package
Release"). Merge it into that release's notes under **Breaking changes**.

---

## Breaking changes

**An unresolved type name is an error.** A type reference that names no visible type used to be
accepted silently: `let f(x: MissingType)` validated, and a typo surfaced later, if at all, as a
confusing mismatch such as "expects MissingType, found string". Analysis now reports
`unresolved-type` where the name was written, with a "did you mean" suggestion:

```nx
type Contact = { name:string }
let people: Contatc+ = {}   // error: `Contatc` is not a visible type; did you mean `Contact`?
```

- It applies to fields, union-case and action fields, props, state, shared emits, parameters,
  return types, `let` annotations, aliases and function types.
- A name is resolved in the module that wrote it. A library that names an unresolved type fails to
  load, and a consumer never reports a library's own references.
- An unresolved `extends` base and type argument keep their existing diagnostics.
- A type that did not parse is reported only as the syntax error, and a name an unresolvable import
  would have provided only as the import's error. While an imported module has lost a declaration to
  a syntax error (including one an unclosed declaration before it swallowed), its importers report
  only that error, not each name it failed to provide.
- A union case is not a type: `Shape.circle` in type position is reported, naming `Shape` as the
  type to write.
- An undeclared `void` is now reported the same way, as `unresolved-type`, rather than only where a
  value failed to match it.

Sources that relied on the silent acceptance fail validation; there is no compatibility mode. Fix
each report by correcting the name, declaring the type, or importing it.
