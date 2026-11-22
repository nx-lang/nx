## Context

The design-system tokens design (https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG, sections
"Environment values", "Main module" and "Reactivity") settled on the language feature. This
change specifies and builds it. What the code has today:

- **Grammar.** `module_definition` is the imports, then the definitions, then an optional root
  element. `visualizer` is a contextual keyword, valid only as the first token of a module member.
  An element property's name is a `qualified_markup_name`, so `<Section env.colorMode=dark>`
  already parses. Lowering doesn't give the dotted name any meaning yet.
- **Values.** A `let` value is evaluated each time a `reference` node reads it, and normalized to
  its declared type (`eval.rs`, `DeclarationKind::Value`). The cost model charges the initializer
  each time.
- **Components.** A component use is an atomic descriptor. Building it never runs the body. A body
  runs when the lifecycle initializes the component, or renders it again, from props and state.
  The instance tree does that for a Rust host. A TypeScript host, and a host of generated code,
  composes its components itself.
- **Instance tree (#73).** A node visited with a descriptor equal to its old one keeps its
  instance and output, and is rebound to the parent's new handlers. Handlers in rendered output
  are tokens that the instance resolves, and the descriptor comparison resolves them through the
  instance on each side.
- **Main module.** A build, and a link, already has an entry module that the host chooses. The
  runtimes call it `entry`.

## Goals / Non-Goals

**Goals:**

- Typed values with a default that the host sets at the root, any element or `provide` block can
  override for what is inside it, and the main module can set app-wide.
- Scoping by the element tree. A component sees the overrides around the place where it is used,
  including content that a caller builds and another component places.
- Overrides cascade through defaults: `env.colorMode=dark` changes `env.color` inside, because
  `color`'s default is evaluated where it is read.
- The Rust IR runtime and the TypeScript IR runtime give the same results and operation counts,
  and generated JavaScript gives the same results.
- A change to a root value renders again only the nodes that read something that changed, found
  through an index without a walk of the tree.

**Non-Goals:**

- `@nx/design`, the token naming convention and the tutorial. They are `add-design-tokens`.
- Standard host inputs (`env.now`, viewport size, platform), and values only the host may set.
- Showing overrides in Visual beyond what it already shows for properties, environment pickers in
  the playground and previewer, value suggestions and token export.
- Memoizing defaults across reads, and lazy local overrides (see Decision 4).

## Decisions

### 1. Syntax

```nx
export environment colorMode: ColorMode = light
export environment color: SemanticColors = if env.colorMode is {
  light => <SemanticColors accent=#2563eb surface=#ffffff />
  dark => <SemanticColors accent=#60a5fa surface=#111827 />
}

component <Footer /> = {
  <Section env.colorMode=dark env.color.accent=#f472b6>
    <Links />
  </Section>
}

let caption(when: date) = provide env.locale="fr-FR" { formatDate(when) }

// app.nx, the main module
import "@acme/brand" as brand
provide brand.theme
provide env.radius = <Radius small=4 medium=8 large=12 />
```

- **`environment` and `provide` are contextual keywords.** As `visualizer` is, each is a keyword
  only as the first token of a module member, and `provide` is one at the start of an expression
  too. A record field named `provide` or `environment` keeps working.
- **`env` is reserved.** `env.` followed by a name is an environment read, and it lowers to its
  own HIR expression, never to a member access on a local. A declaration, parameter, `for` item,
  pattern binding or property named `env` is rejected with `reserved-name`. A field named `env` on
  a record stays legal, since `x.env` is never read as the head of a name. `env.Update` is the
  name of the bundle type, so no environment value can be named `Update`.
- **A block's overrides use the attribute form:** `env.name=value`, separated by spaces, as on an
  element. A top-level provide uses `=` with spaces (`provide env.x = …`), as a `let` does, so a
  long value reads like a declaration.

### 2. Resolution and visibility

`env.name` resolves among the environment values a module can see:

- the ones it declares;
- the internal ones of its library;
- the exported ones of the modules it imports, under any alias.

Two with the same name are ambiguous at the read, with `ambiguous-environment-value`. A qualifier
syntax can come later, if libraries collide in practice. Requiring an import keeps a dependency
visible: an app that overrides `env.color` imports the library that declares it.

A value's identity is its declaring module and name. That identity is what the IR, the runtimes and
the host use. A host names a root value by its simple name when exactly one environment value in
the program has it, and otherwise by its module identity and name (`@acme/design.nx:colorMode`).

### 3. Precedence and scoping

To read slot `s` at a place, the runtime looks at the place's environment and takes the first
of these that sets `s`:

1. the nearest local override, found by walking outward: the provides around the read in the
   current body, then the overrides the current component's descriptor carries, then those of each
   component above it;
2. the host's root value;
3. the main module's provides, where a later one wins over an earlier one;
4. the declaration's default.

An override with a field path (`env.color.accent=X`) is a partial one. Its base is whatever the
same lookup finds outside it, with `accent` set to `X`. The same holds for a main-module provide
with a path, and for a bundle field that is an update record.

A function, a `let` value and a default have no environment of their own. They read in the
environment of the read or call that evaluates them. That is dynamic scoping along the evaluation,
and it is what makes cascading work. A component body is the exception: the runtime runs it
later, from a descriptor, so the descriptor has to carry what was around it.

### 4. Local overrides are evaluated where they are written

Defaults and main-module provides are evaluated where they are read. The value of an `env.`
attribute or a `provide` block is evaluated where it is written, as a prop expression is. Then an
override is plain data, which matters for four reasons:

- **Descriptors stay values.** A component use carries its overrides in a `$env` field the host
  can see, compare and send over JSON. A lazy override would be a closure over the caller's
  locals, which needs tokens resolved through an instance, as handlers do. Then a function that
  returns a descriptor outside any component couldn't carry one at all.
- **The skip check stays a value comparison.** An override that changed is a different value.
- **TypeScript and generated code stay simple.** Neither one has an instance to resolve tokens
  through.
- **Cascading still works where tokens need it.** `color`'s default reads `colorMode` at the
  reader, so `<Footer env.colorMode=dark>` gives the dark colors inside the footer. The doc's
  `env.palette` example works the same way, through the default.

What is lost is an override whose own value reads another environment value that is overridden
further in. For example, `<Banner env.color.accent={ if env.colorMode is {…} }>` with a
`colorMode` override inside `Banner`: the accent is fixed by the colorMode at `<Banner>`. The fix
is to override `colorMode` or the default, not `accent`, and `specs/future.md` records an explicit
opt-in for lazy overrides (for example, an override whose value is a function, called where it is
read) as a possible extension.

For the same reason, the values of an element's `env.` attributes, and of a block's overrides,
are evaluated outside the overrides, so `env.space={env.space * 2}` doubles the outer value. The
element's other attributes and its content are evaluated inside them. An element with `env.`
attributes means exactly what `provide` around the element means.

### 5. How a descriptor carries its overrides

Evaluating a `provide`, including the one an element's `env.` attributes lower to, does two
things:

- **The body evaluates with the overrides pushed on the environment stack**, so reads, function
  calls and defaults inside it see them.
- **The overrides are added to each component descriptor in the result.** The runtime walks the
  result through sequences, records and the fields and content of elements that aren't component
  descriptors, and stops at each component descriptor. There it puts the provide's overrides in
  front of any the descriptor already has in `$env`, so the outermost come first and the
  innermost win.

The walk stops at a component descriptor because the content and props of that use are placed by
the component's body, which already runs inside the overrides. A descriptor built in one place and
placed in another gets the overrides around both: those where it was built, which are innermost
since they were added first, and those where it was placed, which are added by each provide its
value passes through on the way out. That is the rule "content sees the override around where it
is placed", with the overrides written directly on a use winning.

A child's environment is then its parent's environment followed by the child's `$env`. A node
under no parent starts from the root. The rendered output of an intrinsic or host element carries
no `$env`. The overrides on it have been handed to the components inside it, and a host that
wants an environment value reads it through the runtime (see Open Questions).

`$env` is a list of `{ value, path, to }` entries:

- `value` is the qualified name;
- `path` is a list of field names, which may be empty;
- `to` is the value, in canonical form.

A descriptor without overrides has no `$env` field, so today's output doesn't change.

### 6. IR

All of it sits behind the required feature `environment-v1`, with no schema version change, as
`ranges-v1` did. A module lists the feature when it declares an environment value, contains one
of the new nodes or has top-level provides.

- **The declaration kind `environment`** holds the type and the default node. A module exports it
  like any other declaration.
- **`envRead`** is node kind `30`, holding a reference to the declaration. A field read on the
  result is an ordinary `member` node.
- **`envProvide`** is node kind `31`, holding a list of overrides and the body. Each override is
  the declaration reference, the path of field names and the value node.
- **The module entry gains `provides`.** It is a list of the same override shape, plus bundle
  entries that hold one node of `env.Update` type. Only the entry module's list takes effect,
  since it is the only module no other module imports.
- **An `env.Update` value** is a record whose fields are named by the qualified names of
  environment values. The emitter writes the qualified names, so a bundle that `brand` builds
  applies to the right values in a main module that doesn't import them by those names.

Cost, in line with the existing rules:

- an `envRead` node costs one operation, and a default or main-module provide evaluated for it is
  charged node by node each time;
- an `envProvide` node costs one operation, and its override values and body are charged as usual;
- adding overrides to one component descriptor in the result costs one operation;
- the lookup through the environment costs nothing.

These are rules both runtimes can follow exactly, and the corpus records them.

### 7. Runtimes and the host API

- **The environment type.** Both runtimes add an environment: an immutable chain made of the root
  values, then the overrides of each descriptor down to a node.
  - Rust: `Environment::root(values)` and `environment.enter(descriptor)`.
  - TypeScript: `runtime.environment(values)` and `environment.enter(descriptor)`.
  - Root values are validated against their declared types at the boundary, with the usual
    boundary diagnostics. An unknown name is `nx-ir-environment-name`.
- **Evaluation.** Every evaluation, including function entrypoints and the lifecycle (initialize,
  render, dispatch), takes an environment through its options. The default is the root with no
  values. A component's own environment is the one it was initialized in. A dispatch renders in
  the environment the host passes, which the tree keeps per node.
- **Generated JavaScript.**
  - It keeps a module-level current environment, which a provide sets and restores around its
    body.
  - Entry functions and component evaluation take the environment as an option.
  - The program-module runtime package provides the lookup, so generated code and the TypeScript
    runtime share it.
  - Evaluation in JavaScript is synchronous, so a module-level stack is safe.
- **CLI.** `nxlang run --env name=value` can be repeated. The value is NX value text, parsed and
  checked against the declared type, so `--env colorMode=dark` and `--env locale="fr-FR"` both
  work.

### 8. Read tracking in the instance tree

- **Recording reads.** While a node's body renders, the runtime records each environment slot a
  lookup consults, with the value it found. A read of `env.color` whose default reads `colorMode`
  records both, so the node depends on `colorMode`. The list replaces the node's last one on each
  render.
- **The reverse index.** The tree keeps a map from each slot to the nodes that read it, and
  updates it as nodes render, are dropped or are replaced.
- **`set_environment(values, options)`** replaces the root values, all or nothing.
  - It takes the union of the readers of the slots whose root value changed. It is not a walk.
  - For each reader, from the top of the tree down, it looks up the recorded slots again in the
    node's environment.
  - A node with any value that differs renders again with the state it held. Its output is then
    unwalked, and the host's next pass walks it as it would after a dispatch.
  - A node whose values are all the same, for example because a local override hides the root,
    is left alone.
- **The skip check.** A visit compares descriptors without their `$env`, and then checks the
  node's recorded reads against its new environment. A node is kept when both match. So a parent
  that adds `env.locale` around a child that never reads `locale` doesn't render it again. When
  only the overrides changed, the node is kept and takes the new `$env`. Its recorded reads are
  still right, because they are all equal.
- **Rebinding.** It has nothing new to do: overrides are data and hold no handlers.

### 9. Static read sets and the cycle check

The checker computes, for each function, value, default and component body, the environment
values it reads:

- directly;
- through the functions and values it calls;
- through the defaults of the values it reads.

It doesn't follow the component uses it builds, because their bodies run on their own.

- **Hover** uses the direct reads and those through calls: "Reads `env.colorMode`, `env.color`".
- **The cycle check** uses the full set. A default, or a main-module provide, whose set reaches
  its own slot is `environment-cycle`, reported at the declaration with the path.
- **Local overrides can't form a cycle.** They are evaluated outside themselves.
- **The runtimes keep their call-depth limit** as the backstop.

## Risks / Trade-offs

- **Reading a default is charged every time.** A body that reads `env.color` ten times evaluates
  its default ten times. Hoisting with `let c = env.color` inside the body helps. Memoizing per
  environment is a later optimization, and it would change operation counts, so the cost model
  has to change with it.
- **Stamping walks the result of each provide.** The walk is bounded by the result and stops at
  component uses, so it is cheap for markup. A provide around a big data list walks the list.
- **`reserved-name` breaks code that uses `env`.** A search of the repository's examples and tests
  is part of the tasks.
- **Ambiguity between libraries.** Two libraries that both export `colorMode` can't both be
  imported by a module that reads it. This is accepted for now, and the error names both.

## Migration Plan

None. This is a new feature behind a new IR feature flag. Programs that don't use it emit the same
IR as before.

## Open Questions

- **Host elements.** Should a host element be able to read the environment, for example to set
  `dir` or a CSS color scheme? For now a host asks the runtime, and an intrinsic element carries
  no `$env`. Adding `$env` to host elements is easy later, if hosts want it.
