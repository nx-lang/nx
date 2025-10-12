---
title: 'Functions & Components'
description: 'Declaring reusable elements whose definitions mirror their usage.'
---

Functions in NX look exactly like the elements you invoke. Parameters, default values, and child slots appear in the tag signature, making APIs self-documenting.

## Definition Mirrors Invocation

```nx
let <UserCard user:User className:string = "card"/> =
  <div className={className}>
    <img src={user.avatarUrl} alt="User avatar"/>
    <h3>{user.name}</h3>
    <p>{user.email}</p>
  </div>

// Later in the module
<UserCard user={currentUser} className="featured"/>
```

- Attributes in the definition carry type annotations.
- Default values use `=` just like standard attributes.
- Invocation reuses the same structure but supplies values instead of types.

## Components with Children

```nx
let <Layout title:string> content:Element </Layout> =
  <html>
    <head><title>{title}</title></head>
    <body>
      <header><h1>{title}</h1></header>
      <main>{content}</main>
    </body>
  </html>

<Layout title="Home">
  <div>My content</div>
</Layout>
```

- Everything after the closing `>` in the signature represents child slots and named fragments.
- Use descriptive names—`content`, `footer`, `actions`—so call sites stay readable.

## Advanced Parameters

```nx
let <DataGrid
  data:object[]
  columns:object[]
  className:string? /> =
  <table className={if className { className } else { "data-grid" }}>
    <thead>
      <tr>
        for column in columns {
          <th>{column.Header}</th>
        }
      </tr>
    </thead>
    <tbody>
      for item in data {
        <tr>
          for column in columns {
            <td>{column.Render(item)}</td>
          }
        </tr>
      }
    </tbody>
  </table>
```

- Nullable types (`string?`) make optional props explicit.
- Complex defaults can reference other parameters or inline expressions.
- Iteration and conditionals in the body behave like any other expression.

## Self-Contained Helpers
Because functions are just expressions, you can nest local bindings, pattern matches, or calls without switching syntaxes. Future revisions will add attributes such as `JsonRpcMethod` for host integrations.
