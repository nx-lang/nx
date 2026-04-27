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

## Attributes can be conditional groups

Conditional property fragments choose a set of attributes without repeating the element call.

```nx
type LoadState = | idle | failed { message:string }

let <Notice message:string tone:string density:string /> =
  <aside tone={tone}>{message}</aside>

let view(state:LoadState, compact:bool) =
  <Notice
    if state is {
      LoadState.failed => message={state.message} tone="danger"
      else => message="Ready" tone="neutral"
    }
    if compact { density="tight" } else { density="normal" }
  />
```

Every required attribute must be present on every branch. Mutually exclusive branches can provide
the same attribute name, but a direct attribute and an active branch attribute cannot both provide
the same name.

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
