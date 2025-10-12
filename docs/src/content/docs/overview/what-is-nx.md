---
title: 'What is NX?'
description: 'An overview of the goals and philosophy behind the NX language.'
---

NX is a next-generation functional markup language that merges structural markup and executable logic into a single, cohesive syntax. You can think of it as strongly-typed XML with the composability of JSX, but without the awkward split between "template" and "code" files.

## Why NX Exists
NX targets two overlapping problem spaces:

- Building UI components and applications where mark-up and data flows are tightly coupled.
- Authoring configuration-heavy systems that currently rely on JSON, YAML, or ad hoc DSLs.

Traditional approaches require developers to juggle two languages (for example HTML + JavaScript or XML + C#) and context-switch between tools optimised for one or the other. NX keeps everything in one language so that properties, content, and behavior share the same type system, tooling, and ergonomic surface.

## Key Characteristics
- **Unified surface area**: Elements, functions, and types all share the same declaration and invocation syntax.
- **Strong typing**: The compiler enforces correctness for both markup structure and data flows, providing early feedback.
- **Purely functional semantics**: Expressions always yield values, making composition and reasoning predictable.
- **Runtime agnostic**: The language is designed to target .NET, JavaScript, or any other runtime that can host the interpreter or generated code.

## Typical Use Cases
- Crafting rich UI components that compose like JSX but benefit from static types, expression trees, and predictable evaluation.
- Defining design systems where tokens, components, and layout logic all live in the same language.
- Describing configuration for services, pipelines, and infrastructure in a format that is still executable and verifiable.
- Embedding the language inside larger platforms that need a safe, declarative extension surface.

## How NX Evolves
The initial reference implementation focuses on:

- A C# toolchain that lexes, parses, type-checks, and evaluates NX modules.
- A TextMate grammar shared across the docs and the VS Code extension.
- Tooling foundations (LSP, formatting, debugging) so editors deliver first-class ergonomics.

As the specification stabilises, additional runtimes and transpilation targets will become available. The roadmap prioritises correctness and developer experience before taking on advanced features like generics or asynchronous workflows.
