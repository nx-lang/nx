---
title: 'Environment values'
description: 'Values a program reads from where it is placed: declaring, reading and providing them.'
---

An **environment value** is a value with a type and a default that any body can read without it
being passed down as a property: the color mode, the locale, the spacing scale. A part of the
output can override it for everything inside, and the host can set it for the whole program.
Design-system tokens are environment values.

## Declaring a value

```nx
export type ColorMode = light | dark

/// The color scheme the program is shown in.
export environment colorMode: ColorMode = light

/// The gap between items, in pixels.
export environment space: int = 4
```

- `environment name: Type = default` declares one at the top level of a module. The type and the
  default are both required.
- `export` and `private` work as they do on a `let` value. Without one, the value is internal to
  its library.
- `environment` is a keyword only at the start of a declaration, so a field named `environment`
  keeps working.

## Reading a value

`env.name` reads a value, and its type is the declared type. A member access after it reads a field:

```nx
type Palette = { accent: string surface: string }

environment color: Palette = <Palette accent="#2563eb" surface="#ffffff" />
environment space: int = 4

let gap() = { env.space * 2 }
let accent() = { env.color.accent }
```

- The name is found among the environment values the module can see: its own, the internal ones of
  its library, and the exported ones of every module it imports, under any alias. A read is never
  qualified by the alias: `env.space`, not `design.env.space`.
- A name that matches none is `unknown-environment-value`. A name two imported modules both export
  is `ambiguous-environment-value`, naming both.
- `env` is reserved. A declaration, parameter, `for` item or property named `env` is `reserved-name`.
  A record field named `env`, read as `x.env`, still works.

## Overriding a value for an element

An `env.` attribute overrides a value for an element and everything inside it, on any element:

```nx
type ColorMode = light | dark
environment colorMode: ColorMode = light

component <Label text: string /> = {
  <span class={if env.colorMode is { light => "ink" dark => "paper" }}>{text}</span>
}

component <Footer /> = {
  <section env.colorMode=dark>
    <Label text="About" />
    <Label text="Contact" />
  </section>
}
```

- The override reaches the components inside any element, an external component's content
  included, though the element's own output carries nothing for it.
- The attribute is not a property: it is not checked against the element's props, doesn't satisfy a
  required one, and doesn't appear in the element's fields.
- `env.name.field=value` overrides one field of a record value and keeps the rest.
- Overrides of one value on one element apply in the order written, each on the result of the one
  before.

## `provide` blocks

`provide` overrides values for an expression, which is its body. It is written wherever an
expression or markup content can be:

```nx
environment locale: string = "en-US"

let label() = { env.locale }
let french() = { provide env.locale="fr-FR" { label() } }
```

`provide` begins a block wherever it starts an expression, so it can't be the name of a
declaration, parameter or binding (`reserved-name`); a record field named `provide` still works.

`french()` is `"fr-FR"`: a function has no environment of its own, and reads in the environment of
the call. An element with `env.` attributes means a `provide` with the same overrides around the
element.

### Computed where it is read

An override's value that reads the environment is computed where the value is read, not where the
override is written. Inside it, a read of the value it overrides sees the sources outside the
override, so it can build on the value it replaces:

```nx
environment space: int = 4

let doubled() = { provide env.space={env.space * 2} { env.space } }
```

`doubled()` is `8`. Because the value is computed at the read, a value it reads that is overridden
nearer the reader is the nearer one:

```nx
environment space: int = 4
environment gap: int = 0

let nested() = { provide env.gap={env.space * 2} { provide env.space=1 { env.gap } } }
```

`nested()` is `2`, not `8`: `gap` means "twice the space here", wherever it is read. Locals the
value reads keep the values they had where the override was written. A value that reads no
environment value, such as `env.locale="fr-FR"`, is the same wherever it is read.

### Bundles in a block

`provide env with` applies a bundle, an `env.Update` value, to the body:

```nx
environment space: int = 4
environment gap: int = 0

let dense: env.Update = <env.Update space=2 gap={env.space * 3} />

let list() = { provide env with dense { env.gap } }
let tighter() = { provide env with dense with { space=1 } { env.gap } }
let inline() = { provide env with { space=3 } { env.space } }
```

- The bundle may be a name, a call, an element or a braced expression, and `with` steps after it
  apply their entries after the bundle's. A field list right after `env with` is an inline bundle.
- The bundle's entries are computed where a value is read, like any override, so `list()` is `6`:
  `dense`'s `gap` reads the `space` the bundle sets. `tighter()` is `3`.
- A value or field the bundle doesn't set falls through to the sources outside it, so a bundle
  patches the place around it rather than resetting values to their defaults.
- A block holds `env.` overrides or one `env with` chain, never both. To set a value beside a
  bundle, write it as a step: `provide env with dense with { locale="fr-FR" } { … }`. A mixed block
  is a syntax error that suggests that chain, in the order written.
- `env with` is written only after `provide`; elsewhere it is `reserved-name`.

## App-wide values in the main module

The main module, the entry module the host runs, can set values for the whole program:

```nx fragment
import "./design.nx"

provide env.space = 8
provide env.color.accent = "#e11d48"
```

A module that another module imports, or a library module that exports a declaration, can't
contain a top-level `provide`; each one is `provide-outside-main-module`. A module nothing imports, such as a library's own `app.nx` for
previews, may contain them, and they take effect only when it is the entry module.

### Bundles

An `env.Update` value is a bundle of overrides. A module can build one and the main module can apply
it with `provide env with`:

```nx fragment
// brand.nx
import "./design.nx"

export let theme: env.Update = <env.Update space=12 color=<Palette.Update accent="#e11d48" /> />
```

```nx fragment
// app.nx
import "./brand.nx" as brand

provide env with brand.theme
provide env with brand.theme with { radius=0 }
```

Each field of `<env.Update>` names an environment value as an `env.` read does, and a record value
takes that record's `.Update` type, so a bundle can set some of its fields. A bundle remembers which
declarations it means, so it applies to the right values in a main module that doesn't import them.
`bundle with { … }` and `bundle with other` are bundles too, with the right side's entries after the
left's, so a bundle can be built from another and passed around as a value.

## Which value a read sees

A read finds the value from the first of these that sets it:

1. the nearest override around the read: an `env.` attribute, a `provide` block or a bundle's
   entry, nearest first;
2. the host's root value;
3. the main module's provides, a later one before an earlier one;
4. the declaration's default.

An override that sets one field takes the rest of the value from the sources after it.

A component's body runs in the environment of the place its use ends up in the output. Content a
caller builds, placed by another component inside an override, sees that override. An override
written on the use itself is nearer, so it wins.

A default, and the value of a main-module provide, is evaluated each time a read uses it, in the
environment of the read. So a default that reads another value follows its overrides:

```nx
type ColorMode = light | dark
type Palette = { accent: string surface: string }

environment colorMode: ColorMode = light
environment color: Palette = {
  if env.colorMode is {
    light => <Palette accent="#2563eb" surface="#ffffff" />
    dark => <Palette accent="#60a5fa" surface="#0b1220" />
  }
}

let darkSurface() = { provide env.colorMode=dark { env.color.surface } }
```

A default that can read its own value, through other defaults or the functions it calls, never
finishes, so it is rejected with `environment-cycle`, naming the values on the way. An override, a
bundle entry or a main-module provide that reads the value it sets is fine: it sees the sources
after it. One of `V` that reads a `W` whose default reads `V` is `environment-cycle` at the
override: under it, a read of `W` would evaluate `W`'s default, then the override, then `W` again
with nothing left to give it. An override placed by another body can still close such a path while
the program runs, and the read then fails with the runtime error `nx-ir-environment-cycle`.

## In an action handler

An action handler reads the environment where it is written: the overrides around it, the
component's place and the host's root values, as a read in the same spot of the body would.

```nx
environment mode: string = "outer"

component <Page /> = {
  state { last: string = "" }
  <Stack>
    { provide env.mode="block" { <Button label="Block" onTapped=<State.Update last={env.mode} /> /> } }
    <Button env.mode="attr" label="Attr" onTapped=<State.Update last={env.mode} /> />
  </Stack>
}
```

A tap on *Block* sets `last` to `"block"`, and one on *Attr* to `"attr"`. An override whose value
reads the environment is still computed at the read, when the handler runs, in the environment of
that moment.

A handler sees where it is written, not where it is placed. A handler written in a parent's body
and passed to a child as content or as an `on…` property doesn't see the overrides inside the
child. A child that wants its parent to know a value of its environment passes it in a field of
the action it emits:

```nx
environment locale: string = "en-US"

component <LocaleList items?: string+ emits { Picked { item: string locale: string } } /> = {
  <Stack>
    {
      for item in items {
        <Button label={item} onTapped=<LocaleList.Picked item={item} locale={env.locale} /> />
      }
    }
  </Stack>
}

// Puts its content inside an override of its own.
component <Section content children: Element+ /> = {
  <Stack>{ provide env.locale="fr-FR" { children } }</Stack>
}

component <Chooser /> = {
  state { picked: string = "" }
  <Section>
    <LocaleList items={"a"} onPicked=<State.Update picked={action.locale + " " + env.locale} /> />
  </Section>
}
```

A tap on the row sets `picked` to `"fr-FR en-US"`. `LocaleList`'s body runs where `Section`
placed it, so its handler reads `"fr-FR"` and puts it in the action. The parent's handler was
written in `Chooser`'s body, outside `Section`'s override, so its own `env.locale` is `"en-US"`.
`action.locale` is a plain string, computed when the child's handler built the action: an action's
fields are ordinary values, not overrides computed at a later read.

## Root values from the host

A host sets root values by name when it evaluates a program, and they are checked against the
declared types. `nxlang run` takes them with `--env`:

```sh
nxlang run app.nx --env colorMode=dark --env locale=fr-FR
```

A value declared `string` is its text as written, so `--env version=1.0` is the string `1.0`. Any
other value is JSON, or a bare word, read as a union case's name or a string. An unknown name, or one
two environment values share, is an error before the program runs. NX code has no way to read the
operating system's environment.

## In the editor

- Completion after `env.` offers the values the module can see, with their types and documentation,
  and after `env.name.` the fields of the value.
- Hover on a read or an override shows the declaration, `environment space: int = 4`, and its
  documentation.
- Hover on a function, value or component lists the environment values it reads, directly or through
  the functions and values it calls.
- Hover on a top-level `provide`, or on a block's `env with`, lists what it sets, following a
  bundle to its `<env.Update>` and through its `with` steps.
- Completion in a bundle's field list, `provide env with { … }` or `theme with { … }`, offers the
  environment values, and after `name.` the fields of the value.
