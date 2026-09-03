---
title: 'Elements'
description: 'Markup syntax, attributes, and advanced element capabilities in NX.'
---

This page describes element syntax and attribute rules. For full grammar, see [nx-grammar.md](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements).

## Basic Elements

```nx
<Button variant="primary">Click Me</Button>
<Input value={user.name} disabled={isLocked}/>
```

- A property value needs no braces when it is a literal, a negated numeric literal, a union case
  name, or a single element: `variant="primary"`, `count=3`, `tone=primary`,
  `padding=<Thickness Left=4.0/>`.
- Braces hold any other expression, and they hold sequences: `value={user.name}`,
  `points={ <Point x=0 y=0/> <Point x=1 y=1/> }`. A sequence keeps its braces even when it holds a
  single item, which is also how a sequence-valued property formats.
- Attribute names are case-sensitive and must match the component signature.

## Nested Markup in Attributes

```nx
<Tooltip
  content=<span:>
    <strong>Bold</strong> and <em>italic</em> text
  </span>
>
  Hover over me
</Tooltip>
```

Attributes may contain markup as values when the parameter type accepts elements.

## Conditional Property Fragments

Property lists can include conditional fragments. A fragment contributes zero or more properties
to the invocation before the target component, function, record, or union case is bound.

```nx
<Button
  if primary {
    label="Save"
    tone="strong"
  } else {
    label="Cancel"
    tone="neutral"
  }
/>
```

Condition-list fragments choose the first true arm:

```nx
<Badge
  if {
    isError => tone="danger"
    isWarning => tone="warning"
    else => tone="neutral"
  }
/>
```

Match-style fragments use the same union case validation and local identifier narrowing as value
matches:

```nx
<Notice
  if state is {
    LoadState.failed => message={state.message}
    else => message=""
  }
/>
```

Required properties must be supplied on every reachable branch. Duplicate property names are
rejected when they can occur on the same path, including a direct property plus a conditional
branch property. The same property name is allowed in mutually exclusive branches.

Content properties follow the same rules. Body content conflicts with a named content property on
any reachable branch, while mutually exclusive named content-property branches are accepted.

## Namespaces

```nx
<UI.Controls.Button variant="primary">Click</UI.Controls.Button>
```

Namespaces qualify element names and help avoid collisions in large libraries.

## Complex Attribute Expressions

```nx
<Form
  onSubmit={(data) => validateAndSubmit(data)}
  validationRules={[
    { field: "email", validator: isValidEmail },
    { field: "age", validator: (val) => val >= 18 }
  ]}
  className={`form ${if isLoading { "loading" } else { "" }} ${if hasErrors { "error" } else { "" }}`}
>
  Form content
</Form>
```

Expressions, conditionals, and sequences are valid attribute values.

## Styling

```nx
let <StyledButton variant:string = "primary"  content body:Element /> =
  <button
    style=<Style
      backgroundColor={if (variant == "primary") { "#007bff" } else { "#6c757d" }}
      color="white"
      border="none"
      padding="8px 16px"
      borderRadius="4px"
      cursor="pointer"
    />
  >
    {body}
  </button>
```

Inline styles follow the same attribute rules as other properties.
When a declaration marks exactly one property with `content`, element body content binds to that
property during invocation.

## Objects as Attributes

```nx
<UserCard user=<User id="456" name="Jane" email="jane@example.com"/> />
<DrawingCanvas points={ <Point x=10 y=20/> <Point x=30 y=40/> } />
```

Attributes may receive typed objects or sequences when the signature permits. A single object
stands on its own; a sequence goes in braces.

## See also
- Language Tour: [Elements](/language-tour/elements)
- Grammar: [nx-grammar.md – Elements](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
