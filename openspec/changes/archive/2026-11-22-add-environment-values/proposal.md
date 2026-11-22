## Why

Design-system tokens need a value that every component can read and any subtree can change: a
color mode, a palette, a spacing scale, a locale. Today NX has only props, which have to be passed
through every component on the way down, and module `let`s, which nothing can change. The
design-system tokens design (https://claude.ai/artifact/VrkdfYN3jv2Y8KjZWtxshG) settled on one
general feature instead of a token-specific one: **environment values**, typed values with a
default that the host sets at the root and any element can override for everything inside it.
Tokens then become a convention on top, added by the next change, `add-design-tokens`.

`skip-unchanged-children` (#73) made a parent that renders again leave alone the children whose
descriptors are unchanged. This change adds the other half of the reactivity plan: each node
records which environment values it read, so a change reaches only the nodes that read what
changed, through an index from each value to its readers, without a walk of the tree.

## What Changes

- **Declarations.** `export environment colorMode: ColorMode = light` declares an environment
  value at the top level of a module, with a required type and a required default. `environment`
  is a contextual keyword, as `visualizer` is. Visibility works as it does for `let`.
- **Reads.** `env.colorMode` and `env.color.accent` read a value anywhere an expression is allowed.
  `env` is reserved: nothing can be declared or bound under that name.
- **Overrides.**
  - An `env.` attribute on any element, including intrinsic and host elements:
    `<Section env.colorMode=dark>`. It applies to the element and everything inside it, and it is
    not a property.
  - A `provide env.locale="fr-FR" { … }` block. It works in markup content and in any expression,
    and its value is its body's.
  - A field path, such as `env.color.accent=…`, overrides one field and keeps the others, as
    applying an update record does.
- **The main module.** A `provide` at the top level of a module sets a value app-wide:
  `provide env.radius = …`, or `provide brand.theme`, which applies an `env.Update` bundle. Only
  the program's main module, the entry module the host chooses, applies its top-level provides. A
  module another module imports can't have one: that is the error `provide-outside-main-module`.
- **Precedence**, lowest to highest: the declaration's default, then the main module's provides,
  then the host's root values, then `env.` attributes and `provide` blocks around the place that
  reads the value.
- **Scoping by the element tree.**
  - A component's body sees the environment of the place where its use ends up in the output.
  - An expression sees the overrides written around it.
  - A function or value sees the environment of the place that calls or reads it.
- **Evaluation.**
  - A default, and a main-module provide, is evaluated where the value is read. So
    `<Footer env.colorMode=dark>` changes what `env.color` is inside it.
  - The value of an `env.` attribute or a `provide` block is evaluated where it is written. A
    component use carries the overrides around it as plain data in its descriptor.
- **Diagnostics:**
  - `unknown-environment-value`, `ambiguous-environment-value` and `duplicate-environment`;
  - `environment-default-required`, `environment-path` and `environment-cycle`;
  - `reserved-name` for `env`;
  - `provide-outside-main-module`.
  - A wrong-typed value gets the usual type mismatch.
- **NX IR:**
  - an `environment` declaration kind;
  - the `envRead` and `envProvide` node kinds;
  - the module's top-level provides;
  - the reserved `$env` field of a component descriptor.
  - All of it is behind the required feature `environment-v1`, and the schema version doesn't
    change.
- **Runtimes:**
  - The Rust IR runtime, the TypeScript IR runtime and generated JavaScript all evaluate the
    feature with the same results and the same operation counts.
  - Each takes root values from the host, validated against their declared types.
  - Each lets a host that composes components itself work out a child's environment from its
    parent's and the child's descriptor.
- **Instance tree:**
  - Each node runs in the environment of its place and records the values its last render read.
  - The tree keeps an index from each value to the nodes that read it.
  - Setting a root value renders again only the readers that now see a different value.
  - The skip check from #73 also keeps a node whose descriptor differs only in overrides of values
    it didn't read.
- **Hosts:**
  - `nxlang run --env colorMode=dark` (repeatable), in the Rust runtime options.
  - The TypeScript runtime's evaluation options.
  - The instance tree's `set_environment`.
  - The playground and previewer pass no root values yet. Environment pickers come with
    `add-visual-value-suggestions`.
- **Tooling:**
  - completion after `env.`, in expressions and in attribute position;
  - hover on a read, showing its declaration, type and default;
  - hover on a component or function, listing the environment values it reads;
  - go to definition;
  - syntax highlighting for `environment`, `provide` and `env`.
- **Docs:**
  - a language reference page for environment values, and the style guide's layout of `provide`;
  - the runtime READMEs and `docs/nx-ir-format.md`.

Not in this change:

- `@nx/design` and the token convention (`add-design-tokens`);
- standard host inputs such as `env.now` or the viewport size;
- values that only the host may set;
- record `with`;
- Visual value suggestions and token export.

## Capabilities

### New Capabilities

- `environment-values`: declaring, reading and overriding environment values, the main module and
  its provides, precedence, scoping by the element tree, where defaults and overrides are
  evaluated, and the diagnostics.

### Modified Capabilities

- `nx-ir-format`: environment declarations, the `envRead` and `envProvide` nodes, module
  provides, the descriptor's `$env` field, the `environment-v1` feature and what it costs.
- `rust-ir-runtime`: evaluating environment values, taking root values from the host, giving a
  host a child's environment, and the CLI's `--env`.
- `typescript-ir-runtime`: the same, for the TypeScript runtime.
- `component-instance-tree`:
  - each node's environment and what it read;
  - setting root values through the tree;
  - a skip that ignores overrides of values a node didn't read.
- `executable-code-generation`: generated JavaScript evaluates environment values and takes root
  values as an option.
- `editor-language-service`: completion, hover and go to definition for environment values.
- `editor-syntax-highlighting`: `environment`, `provide` and `env`.

## Impact

- **Syntax:** `crates/nx-syntax/grammar.js` gains `environment_definition`, a top-level
  `provide_definition` and a `provide_expression`. Element properties already take dotted names,
  so `env.colorMode=dark` parses today.
- **Front end:**
  - `crates/nx-hir`: lowering, the scope rules and the reserved name;
  - `crates/nx-types`: checking, read sets and the cycle check;
  - `crates/nx-program`: the main-module rule.
- **IR:** `crates/nx-ir` (model, emitter and reader), `docs/nx-ir-format.md` and the conformance
  corpus.
- **Runtimes:**
  - `crates/nx-ir-runtime`: evaluation, the environment type, the lifecycle and the tree;
  - `runtime/typescript`;
  - `crates/nx-codegen` for generated JavaScript.
- **CLI and API:** `crates/nx-cli` (`--env`) and `crates/nx-api` (the facade and the tests).
- **Tooling:**
  - `crates/nx-language-service`, for completion and hover;
  - the TextMate grammar in `src/vscode` and the tree-sitter highlights.
- **Docs:**
  - the language reference page on the website, and the style guide;
  - `crates/nx-ir-runtime/README.md`;
  - `specs/future.md`, for host-only values, standard host inputs and lazy local overrides.
- This is a new feature, so no program written today changes meaning, except one that already
  uses `env` as a name. Nothing needs to stay compatible yet (AGENTS.md).
