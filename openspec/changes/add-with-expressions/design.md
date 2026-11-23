## Context

- `environment-values` (#74) gives NX environment values, `env.` attributes, `provide` blocks of
  single overrides, and app-wide `provide bundle` at the top of the main module only. A local
  override's value is evaluated where it is written and carried as plain data in a component
  descriptor's `$env`. A default, and a main-module provide, is evaluated where a value is read.
- `update-records` gives every record `T.Update` and the intrinsics `apply`, `merge`, `diff` and
  `changed`. Patching a record means naming its update type, and patching a nested field means one
  `apply` per level.
- `add-design-tokens` (#75) makes themes `env.Update` bundles. A theme that varies by mode matches
  on `env.colorMode` in its fields, which only works because the main-module bundle is evaluated
  where it is read.
- Bret preferred an explicit form for applying a bundle to a section, `provide env = env with
  marketing { … }`, over `provide marketing { … }`. The form here keeps its meaning and drops the
  repetition (thread, 2026-11-23).

## Goals / Non-Goals

**Goals:**

- One word, `with`, for "this value with these changes on top", on records, update records and
  bundles.
- A bundle applied to one section, with the same meaning as the app-wide one.
- No new IR node for record `with`.

**Non-Goals:**

- `env` as a first-class value (`let saved = env`, `env with` outside `provide`).
- An element attribute form for bundles (`<Banner env with={marketing}>`).
- `with` on a union value or union-case value.
- Caching bundle evaluation (left to `add-derived-value-caching`).

## Decisions

### 1. `provide env with bundle`, not `provide env = env with bundle`

`env = env with X` would make `env` a value: the whole environment as a record. Then `let saved =
env` and `env with …` anywhere would be expected too. The runtime records reads value by value, so a
read of the whole environment would tie a node to every value and lose the skipping that
`skip-unchanged-children` and `environment-values` built. It also reads as eager, a record built at
the section's edge, where the behavior has to be lazy (decision 3), and its only sensible right-hand
side is `env with …`: `provide env = marketing` would read as replacing the whole environment, host
values and app theme included.

`provide env with X` keeps the explicit "the environment, with X on top" and is one form for the
main module and for blocks. `env with` is allowed only right after `provide`.

Alternatives: `provide marketing { … }` (shorter, but less explicit about patching, which Bret
wanted), and an `env={marketing}` attribute (deferred with the element form).

### 2. `with` grammar and precedence

`with` is a left-associative infix operator at precedence 125: below prefix unary (130), member
access and calls (140), and above `*` (120). So `saved with brand.theme` patches with `brand.theme`,
`a with b with c` is `(a with b) with c`, and `base with { size=2 } == other` compares the patched
value.

The right side is either:

- a **field list**: `{` then zero or more `name=value` or `name.path=value` entries in the attribute
  form, then `}`. A brace after `with` whose first entry is `name=` or `name.path=` is a field list;
  `=` is not an expression operator, so this is unambiguous. `with {}` is an empty field list and
  changes nothing.
- an **operand** at precedence above 125: a name, member access, call, element, or braced
  expression, of an update type.

Each field value is typed by the field it sets, so a bare union case works:
`settings with { mode=dark }`. A present empty value clears an optional field: `user with {
email={} }`, as in an update record.

### 3. A section bundle is named, and evaluated where a value is read

Single local overrides are evaluated where they are written. A section bundle can't follow that
rule: a theme that varies by mode would be fixed to the mode at the section's edge, so `provide env
with marketing env.colorMode=dark { … }` would apply marketing's light colors in a dark section. So
a section bundle is evaluated where a value is read, as a main-module provide is, with the same
rule for its own value: inside the evaluation of an override of `V`, a read of `V` sees the sources
after it.

A component descriptor's `$env` holds plain data, which a host can store and compare. A bundle that
is evaluated later can't be a value there, but a reference to a top-level declaration can: top-level
values have no captured locals. So a section bundle must be a named top-level value
(`section-bundle-name` otherwise), and `$env` carries a bundle reference, by declaration, which the
runtime evaluates at a read. An inline bundle has no advantage over single overrides in a block,
which the error message suggests.

The main module's `provide env with X` keeps taking any `env.Update` expression, since it is already
evaluated where read and has no locals.

### 4. Where a section bundle sits in the lookup

A section bundle is a local override: it counts among "the nearest override around the read". Its
entries apply in the order written with the block's other overrides, each on the result of the one
before. A value or field the bundle doesn't set falls through to the sources after it, so the bundle
patches the place around it rather than resetting to defaults. This is how nested `ThemeProvider`s in
MUI and `FluentProvider`s in Fluent UI behave, and how CSS custom properties cascade.

### 5. Paths go through required record fields

`r with { a.b=1 }` means `r with { a=(r.a with { b=1 }) }`, with `r` evaluated once. Each step but
the last must name a required field of a record type; an optional field, a union, or anything else
is `with-path`, since there may be nothing to patch. Setting the whole field works instead. The same
rule already holds for `env.` paths (`environment-path`).

### 6. `with` on update records and bundles

- `u with v` on `T.Update` values is `merge(u, v)`; a field list builds `v` as a `T.Update`. Paths
  are `with-path` here, since a nested field of an update record may be absent.
- `b with v` on `env.Update` bundles is a bundle with `v`'s overrides after `b`'s. A field list may
  use paths, as a bundle's own `<env.Update color.text.muted=… />` can.
- `with` on any other type is `with-target`, naming the type, and suggesting `apply` or a field list
  where that would help.

### 7. NX IR

- Record and update-record `with` lower to `apply` and `merge` calls, with a local binding for the
  left side when a path needs it more than once. Images for programs that use only these are what an
  equivalent `apply` program produces.
- Bundle `with` adds an optional `base` operand to `envUpdate`: the bundle's entries are the base's
  followed by its own.
- A section bundle adds a bundle-reference entry to `envProvide`, and to `$env` as
  `{ "$bundle": "<module>::<name>" }`. A runtime evaluates it at a read.
- These are behind a new required feature, `environment-v2`. A program that uses neither still
  declares only `environment-v1`.

### 8. Generated code

Generated modules evaluate top-level values once at load, so a bundle in a generated program would
keep the mode of that moment, in sections and app-wide alike (the `darkAccent` divergence in #75). A
top-level value of type `env.Update` is therefore emitted as a function evaluated at each read,
like a default. That fixes the app-wide case too. Other top-level values keep being evaluated at
load.

## Risks / Trade-offs

- **Cost.** A section bundle is evaluated at every read inside the section, as the main-module one
  is. Theme reads are already expensive (`specs/future.md`, "Reading a token is expensive"); this
  adds no new kind of cost, and `add-derived-value-caching` is meant to remove it.
- **`with` becomes reserved.** Code that names something `with` must rename it. The repository is
  searched and updated as part of this change.
- **Two evaluation rules for local overrides.** A single override is evaluated where written, a
  bundle where read. The rule follows what each is for, and the reference page states it beside the
  example that needs it.
- **#75 ordering.** Whichever of #75 and this change merges second updates the other's uses of
  `provide brand.theme`.

## Open Questions

- Should an element get a bundle form later, and with what spelling (`<Banner env with={marketing}>`
  reads awkwardly)? A `provide` block around the element covers it meanwhile.
- Should `with` work on a union-case value (`failed with { message=… }`) when its case is known
  statically?
