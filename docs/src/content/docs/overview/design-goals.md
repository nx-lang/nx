---
title: 'Design Goals'
description: 'High-level design goals that guide NX language decisions.'
---

NX’s design goals explain why the language looks and behaves the way it does. They act as guardrails when evaluating new syntax, runtime capabilities, and tooling investments.

## Unified Language Design
- Treat markup and logic as the same language surface so authors never leave "NX mode".
- Keep invocation and declaration syntax aligned: the way you use a component mirrors the way you define it.
- Allow any property to hold rich markup, not just strings, so composition feels natural.

## Strong Type System
- Provide compile-time guarantees for both data flow and structural markup.
- Blend inference with explicit types: obvious cases stay lightweight, complex cases stay explicit.
- Surface precise diagnostics that help developers discover errors before runtime.

## Familiar Yet Improved Syntax
- Start from XML and JSX patterns so developers recognise the structure immediately.
- Remove historical pain points (untyped props, stringly-typed attributes, unstructured children).
- Offer modern composition patterns without sacrificing readability.

## Performance & Tooling Parity
- Make sophisticated tooling—LSP features, debugging, formatting—part of the baseline experience.
- Design the runtime so expression trees can be interpreted, compiled, or transpiled depending on the host.
- Optimise for cross-platform delivery so teams can adopt NX wherever they build experiences.
