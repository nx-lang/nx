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
- One evaluation rule for every source of an environment value.
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

`with` is a left-associative infix operator at precedence 127: below prefix unary (130), member
access and calls (140), and above `??` (125, right-associative) and `*` (120). So `saved with
brand.theme` patches with `brand.theme`, `a with b with c` is `(a with b) with c`, `a ?? b with c` is
`a ?? (b with c)`, `x with y ?? z` is `(x with y) ?? z`, and `base with { size=2 } == other` compares
the patched value.

The right side is either:

- a **field list**: `{` then zero or more `name=value` or `name.path=value` entries in the attribute
  form, then `}`. A brace after `with` whose first entry is `name=` or `name.path=` is a field list;
  `=` is not an expression operator, so this is unambiguous. `with {}` is an empty field list and
  changes nothing.
- an **operand**: a name, member access, call, element, or braced expression, of an update type.

A field list holds plain entries only: no conditional entries and no spreads, which can come later
if they are needed. A field named `with` still works as an entry (`x with { with=3 }`), as an
attribute and after `.`.

In a `provide` block, `env with` takes the same operand, optionally followed by more `with` steps,
and then the block's other overrides and its body: `provide env with brand with { radius=4 }
env.colorMode=dark { … }`. `env with` anywhere else is `reserved-name`, with a message saying it is
written only after `provide`; the main module's old `provide brand.theme` is a syntax error that
suggests `provide env with brand.theme`.

Each field value is typed by the field it sets, so a bare union case works:
`settings with { mode=dark }`. A present empty value clears an optional field: `user with {
email={} }`, as in an update record.

### 3. Every environment value is computed where it is read

Bret asked for one rule that can be explained without exceptions (thread, 2026-11-24). Until now
there were two: defaults and main-module provides were evaluated where a value is read, local
overrides where they are written. A bundle applied to a section exposed the cost of that split: a
theme that varies by mode would be fixed to the mode at the section's edge, so `provide env with
marketing env.colorMode=dark { … }` would apply marketing's light colors in a dark section, unless
bundles got a third rule of their own.

The rule now is: **an environment value is always computed where it is read.** Every default,
provide, `env.` attribute and bundle is a formula that runs at the read, and inside it a read of the
same value sees the sources further out. The two old rules only give different answers when an
override reads a value that is overridden again nearer the reader:

```nx
<Section env.gap={env.space * 2}>   // space is 4 here
  <Inner env.space=1> …env.gap… </Inner>
</Section>
```

Computed where written, `gap` inside `Inner` is 8; computed where read, it is 2, so "the gap is twice
the space" holds wherever it is read. That is what themes need, and what platforms with dynamic
theme values do: SwiftUI's dynamic colors and UIKit's trait-resolved `UIColor` resolve at the point
of use. CSS custom properties resolve where declared, and the familiar complaint about them is that
`--accent: var(--mode-color)` doesn't follow a mode set further down.

An override still sees the locals where it is written, with their values there, as any closure does:
`for item in items { <Row env.accent={item.color} /> }` gives each row its own color.

**Descriptors and bundles carry formulas.** A component descriptor's `$env` must stay data a host can
store and compare. An override that reads no environment value gives the same value at every read,
so a runtime evaluates it once and carries the value as before (`env.colorMode=dark` is unchanged).
Any other override is carried as a formula: the module, the value's node, the module's fingerprint
and the locals it captured, which are data. Whether a value reads the environment is decided from its
static read set, so a value that reads one only on some branches is a formula.

An `env.Update` value holds its entries the same way. So a bundle built anywhere, passed through a
prop, a local or a `for` item, and applied anywhere, still computes each entry where a value is read:
`component <Banner theme: env.Update /> = { provide env with theme { … } }` follows a mode set inside
the banner. Inline and named bundles behave the same, and no bundle needs to be named. A read of a
value evaluates only the entries for that value of the bundles on its way.

A formula is valid only with the build of the module that made it: its fingerprint, written in the
lossless form program fingerprints use, must match. `enter` has no program, so it checks only the
shape of `$env`; a stale formula is refused with `nx-ir-boundary-type` at the read that evaluates
it, as a stale handler token is refused when it is used. A host that keeps
descriptors across a rebuild renders again.

**Cycles.** While a source of `V` is evaluated, a read of `V` at any depth sees only the sources
after it, so an override of a value by itself can't loop. One shape can: an override of `V` that
reads `W`, where `W`'s default reads `V`. A read of `W` under it evaluates the default, then the
override, then `W` again, with nothing left after the default. The static `environment-cycle` check
is extended to overrides, bundles and main-module provides for that shape. Overrides placed by
another body can still close it at run time, so such a read fails with `nx-ir-environment-cycle`.

Alternatives considered:

- **Top-level things where read, inline things where written.** It matches how functions read where
  they are called, but `provide env with marketing` and the same bundle written inline would behave
  differently, so replacing a name with its definition would change the meaning.
- **Always where written.** Defaults could no longer follow overrides, which the whole token design
  relies on.

### 4. Where a bundle in a block sits in the lookup

A bundle in a `provide` block is a local override: it counts among "the nearest override around the
read". Its entries apply in the order written with the block's other overrides, each on the result
of the one before. A value or field the bundle doesn't set falls through to the sources after it, so
the bundle patches the place around it rather than resetting to defaults. This is how nested
`ThemeProvider`s in MUI and `FluentProvider`s in Fluent UI behave.

### 5. Paths go through required record fields

`r with { a.b=1 }` means `r with { a=(r.a with { b=1 }) }`, with `r` evaluated once. Each step but
the last must name a required field of a record type; an optional field, a union, or anything else
is `with-path`, since there may be nothing to patch. Setting the whole field works instead. This is
stricter than the static rule for `env.` paths (`environment-path`), which this change leaves as it
is.

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
- `envUpdate` gains an optional `base` node: the bundle's entries are the base's followed by its own.
- Each `envProvide` and `envUpdate` override gains a formula flag, set from the static read set, and
  `envProvide` gains bundle overrides (no reference, empty path, an `env.Update` node).
- `$env` and `$entries` entries hold `to` or `formula` (`module`, `node`, `fingerprint`, `locals`),
  and a bundle in `$env` is an entry holding `bundle`.
- A module that uses a formula, a bundle in a block or a `base` lists `environment-v2` beside
  `environment-v1`. A program that uses none of them still declares only `environment-v1` and emits
  the same IR.
- Cost: a formula is charged node by node at each read, as a default is; capturing one costs one
  operation; a read doesn't evaluate a bundle's entries for other values.

### 8. Runtimes and generated code

An environment holds formulas as data, and a runtime evaluates them with the prepared program of the
evaluation that reads them, as it evaluates defaults, so `enter(descriptor)` needs no program. Both
runtimes accept `environment-v2`. The instance tree records the reads a formula or bundle entry
makes, so `set_environment` reaches a node that reads through one.

Generated code carries a formula as a function of no arguments that closes over its locals and reads
the environment through the generated helpers, beside the runtime's `module` and `locals`; `$env`
entries otherwise have the runtime's shape. Serializing a generated formula throws rather than
dropping it, and the conformance comparison matches formula entries by `module`, `locals` and the
values reads through them give. An entry that only names a top-level value still has that value's
load-time result in generated code, as other top-level values do.
Since a bundle's entries are formulas, a top-level bundle evaluated when a generated module loads
still computes its entries at each read, which fixes the `darkAccent` divergence from #75 without
treating `env.Update` values specially.

## Risks / Trade-offs

- **Cost.** An override that reads the environment, and every bundle entry, is evaluated at every
  read, as defaults and the main-module bundle are. Theme reads are already expensive
  (`specs/future.md`, "Reading a token is expensive"); this adds no new kind of cost, and
  `add-derived-value-caching` is meant to remove it.
- **A cycle can appear at run time.** Overrides placed by another body can close the one cycle shape
  the static check can't see; it fails with `nx-ir-environment-cycle` naming the path.
- **`with` becomes reserved.** Code that names something `with` must rename it. The repository is
  searched and updated as part of this change.
- **A behavior change for existing overrides.** An override that reads a value overridden again
  nearer the reader now follows that nearer override. The corpus and docs are checked for any case
  that relied on the old rule; reading the outer value into a local first keeps it.
- **Descriptors are less readable when they carry formulas.** Only overrides that read the
  environment become formulas; plain values stay as they were.
- **#75 merged first.** This change updates #75's uses of `provide brand.theme`, its "a bundle can't
  be applied to one section" notes, and `design-tokens-theme`'s `darkAccent` divergence.

## Open Questions

- Should an element get a bundle form later, and with what spelling (`<Banner env with={marketing}>`
  reads awkwardly)? A `provide` block around the element covers it meanwhile.
- Should `with` work on a union-case value (`failed with { message=… }`) when its case is known
  statically?
