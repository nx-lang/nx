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

- Attributes accept any expression inside `{}`; string literals may be provided without braces.
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
let <StyledButton variant:string = "primary" content:Element/> =
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
    {content}
  </button>
```

Inline styles follow the same attribute rules as other properties.

## Objects as Attributes

```nx
<UserCard user=<User id="456" name="Jane" email="jane@example.com"/> />
<DrawingCanvas points=<><Point x=10 y=20/> <Point x=30 y=40/></> />
```

Attributes may receive typed objects or sequences when the signature permits.

## See also
- Language Tour: [Elements](/language-tour/elements)
- Grammar: [nx-grammar.md – Elements](https://github.com/nx-lang/nx/blob/main/nx-grammar.md#elements)
