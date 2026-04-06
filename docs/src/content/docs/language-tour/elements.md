---
title: 'Elements'
description: 'Start with NX elements: familiar like XML, but with typed expressions everywhere.'
---

NX elements look like XML/JSX, but every attribute and body item is a typed expression. This page highlights what’s the same, what’s different, and how to read element structure.

## A first element

```nx
<Button tone="primary">Click me</Button>
```

- Angle-bracket syntax is familiar.
- Attributes are not just strings—they can hold any expression.
- Body content is part of the expression model, not a separate templating layer.

### How it differs from XML/JSX
- Attributes accept expressions with braces, not string concatenation.
- Types are enforced by the component signature (see Reference).
- Namespaces are supported (`<UI.Button/>`) but optional for small files.

## Attributes can be expressions or inline markup

```nx
let isLoading = true

<Tooltip
  content=<span:>
    <strong>Bold</strong> helper text
  </span>
  tone={if isLoading { "muted" } else { "info" }}>
  Hover me
</Tooltip>
```

- Use `{}` to embed any expression (including `if`).
- Use inline markup for attribute values when richer structure is needed.

### Content-marked body parameters

```nx
let <Card title:string  content body:Element /> =
  <article>
    <h2>{title}</h2>
    {body}
  </article>

<Card title="Title">
  <p>Body copy</p>
</Card>
```

`content` is a contextual marker on exactly one property definition. The element body binds to that
property, and the body can still mix literals, loops, and conditionals.

## See also (Reference/Grammar)
- Reference: [Elements](/reference/syntax/elements)
- Grammar: [nx-grammar.md on GitHub](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
