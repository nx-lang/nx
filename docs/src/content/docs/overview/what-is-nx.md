---
title: 'What is NX?'
description: 'An overview of the goals and philosophy behind the NX language.'
---

NX is a next-generation functional markup language that merges structural markup and executable logic into a single, cohesive syntax. You can think of it as
an improved XML, with strong typing, functions, expressions, and first-class support for sequences and objects. It's intended to be a better alternative to JSX and XAML - familiar to those devs but with a more modern, typed foundation.

## Why NX Exists

NX targets two overlapping problem spaces:

- Building UI: pages, components, and UI design systems
- Authoring configuration-heavy systems that currently rely on JSON, YAML, XML, or ad hoc DSLs

Traditional approaches require developers to juggle two languages (for example HTML + JavaScript or XML + C#) and context-switch between tools optimised for one or the other. NX keeps everything in one language so that properties, content, and behavior share the same type system, tooling, and ergonomic surface.

## Key Characteristics
- **Unified surface area**: Elements, functions, and types all share the same declaration and invocation syntax, akin to "functional XML".
- **Strong typing**: The compiler enforces correctness for both markup structure and data bindings.
- **Purely functional semantics**: Expressions always yield values, with no side effects. This keeps composition and reasoning predictable and allows optimisations beyond those possible in imperative languages.
- **Runtime agnostic**: Unlike JSX (JavaScript-only) and XAML (.NET only), NX isn't tied to a pariticular runtime. It can be used with JavaScript/TypeScript, C#, Rust (WASM or native), or theoretically any other language, via transpilation or interpretation.

## Typical Use Cases
- Crafting rich UI components that compose like JSX but benefit from static types, expression trees, and predictable evaluation.
- Defining design systems where tokens, components, and layout logic all live in the same language.
- Describing configuration for services, pipelines, and infrastructure in a format that is still executable and verifiable.
- Embedding the language inside larger platforms that need a safe, declarative extension surface.
