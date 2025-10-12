---
title: 'Comparison'
description: 'Contrast NX with adjacent languages, tools, and design systems.'
---

Evaluating NX is easier when you can position it beside tools you already use today. The table and notes below focus on where NX brings value and when other options may still be preferable.

| Scenario | NX | JSX/TSX | XML/XAML | JSON/YAML |
| --- | --- | --- | --- | --- |
| Single-language workflow | ✅ Markup + logic share one syntax | ⚠️ Mixes JSX with JavaScript/TypeScript | ⚠️ Requires separate code-behind language | ⚠️ Configuration only, no executable logic |
| Static typing & safety | ✅ Structural and data typing | ⚠️ Types rely on surrounding TypeScript | ⚠️ XML schemas often optional | ❌ Untyped by default |
| Declarative UI composition | ✅ Function definitions mirror usage | ✅ Mature ecosystem | ✅ Layout-first but limited logic | ❌ Limited to data description |
| Runtime flexibility | ✅ Interpreter with planned transpilers | ✅ Browser/Node | ⚠️ Mostly .NET/Windows | ✅ Any platform |
| Tooling story | ✅ Built-in LSP + expression trees | ✅ Mature editors | ✅ Strong in IDEs, weaker elsewhere | ⚠️ Basic validation only |

### When NX Shines
- Your UI code needs strong guarantees about props, children, and layout but must still stay ergonomic.
- You want to share a component model between design tools and runtime without switching languages.
- Configuration files need conditional logic, iteration, or reuse that JSON/YAML cannot express safely.

### When to Reach for Alternatives
- You depend on the JavaScript ecosystem today and cannot introduce another runtime yet—stick with JSX for the moment.
- You have existing XAML investments and need full parity with Windows UI frameworks.
- You only need simple configuration storage with no logic; JSON or YAML keeps things lightweight.

NX’s goal is not to replace every DSL, but to provide a single, typed language that can stretch across UI rendering, configuration, and extensibility surfaces without fragmenting your stack.
