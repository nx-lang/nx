---
title: 'Elements'
description: 'Markup syntax, attributes, and advanced element capabilities in NX.'
---

NX elements look familiar to anyone who has written XML or JSX, but they fully participate in the type system and expression model.

## Basic Elements

```nx
<Button variant="primary">Click Me</Button>
<Input value={user.name} disabled={isLocked}/>
```

- Use camelCase for attributes to align with host platform conventions.
- Attribute values accept expressions, not just strings.

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

By treating attributes as expressions, you can inline complex fragments without escaping into strings.

## Spread Attributes

```nx
let commonButtonProps = <Button.properties className="btn" disabled=false/>

<Button ...commonButtonProps onClick={handleClick}>
  Submit
</Button>
```

- Spread syntax makes it easy to compose configuration objects or share default props.
- The `.properties` convention distinguishes attribute objects from rendered elements.

## Namespaces

```nx
<UI.Controls.Button variant="primary">Click</UI.Controls.Button>
```

Namespaces keep large component libraries organised and reduce naming conflicts.

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

- Interpolate conditionals and loops directly inside attribute expressions.
- Arrays, objects, and functions are all first-class citizens.

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

- Inline styles use the same attribute syntax as any other element.
- Because `style` accepts markup, you can compose tokens, variables, or nested structures without switching formats.

## Objects as Attributes

```nx
<UserCard user=<User id="456" name="Jane" email="jane@example.com"/> />
<DrawingCanvas points=<><Point x=10 y=20/> <Point x=30 y=40/></> />
```

Passing fully-typed objects into attributes is straightforward, unlocking strongly-typed component APIs.
