---
title: 'Textual Content'
description: 'Typed text elements, raw blocks, and embedded expressions.'
---

Text nodes in NX are typed, so you can opt into different text processors (e.g., markdown) or keep raw text when needed.

## Default text

```nx
<p>Hello, world.</p>
```

Plain elements without a text type use the host’s default (often plain UI text).

## Explicit text types with `:TextType`

```nx
<Note:markdown>
  **Bold** and _italic_ text.
</Note:markdown>
```

- `:markdown` (or any identifier) selects a text processor.
- Attributes can still appear on typed text elements.

## Raw text

```nx
<Snippet:raw>
  {"literal braces stay untouched"}
</Snippet:raw>
```

Use `raw` to prevent interpretation of braces or entities.

## Embedded expressions in text

```nx
<p>
  Account balance: @{balance} credits
</p>
```

- `@{}` interpolates expressions inside typed text content.
- Escape `@` as `\@` when you need a literal at-sign.

## Next up

## See also (Reference/Grammar)
- Reference: [Elements](/reference/syntax/elements)
- Reference: [Expressions](/reference/syntax/expressions)
- Grammar: [nx-grammar.md – Elements/Text](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
