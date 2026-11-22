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

The value of an override is evaluated where it is written, outside the override, so it can read
the value it replaces:

```nx
environment space: int = 4

let doubled() = { provide env.space={env.space * 2} { env.space } }
```

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
it with `provide bundle`:

```nx fragment
// brand.nx
import "./design.nx"

export let theme: env.Update = <env.Update space=12 color=<Palette.Update accent="#e11d48" /> />
```

```nx fragment
// app.nx
import "./brand.nx" as brand

provide brand.theme
```

Each field of `<env.Update>` names an environment value as an `env.` read does, and a record value
takes that record's `.Update` type, so a bundle can set some of its fields. A bundle remembers which
declarations it means, so it applies to the right values in a main module that doesn't import them.

## Which value a read sees

A read finds the value from the first of these that sets it:

1. the nearest override around the read: an `env.` attribute or a `provide` block, nearest first;
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
finishes, so it is rejected with `environment-cycle`, naming the values on the way. A main-module
provide that reads the value it sets is fine: it sees the sources after it.

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
- Hover on a top-level `provide` lists what it sets, following a bundle to its `<env.Update>`.
