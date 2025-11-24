---
title: 'Getting Started'
description: 'Spin up your first NX project with the essential tooling.'
---

This quickstart gets you from zero to a working NX file, validates your setup, and points to the next learning paths.

## Prerequisites
- Rust 1.75+ (`rustup` recommended)
- Node.js LTS (for docs site and editor tooling)
- Optional: `tree-sitter` CLI if you want to parse files from the terminal

## 1) Install dependencies
From the repo root:

```bash
cargo build --workspace
npm install --prefix docs
```

Validation checkpoint: `cargo test --workspace` should pass; this confirms the parser/type checker build cleanly.

## 2) Create your first NX file
Make `examples/nx/hello.nx`:

```nx
import * from "./ui"

type <User id:string name:string/>

let <Hello user:User/> =
  <div className="hello">
    <h1>Hello, {user.name}!</h1>
    <p>User id: {user.id}</p>
  </div>

<Hello user=<User id="123" name="Ada"/>> </Hello>
```

This file shows imports, a typed object, a component with a child slot, and a root element.

## 3) Validate the file
- If you have `tree-sitter` installed, run `cd crates/nx-syntax && tree-sitter parse ../../examples/nx/hello.nx` to confirm the syntax is accepted.
- For deeper checks, drop the snippet into a small Rust harness using `nx_types::check_str` (see README examples) to verify typing; the sample in `README.md` can be run from any Rust binary in your workspace.

## 4) Explore language fundamentals
Work through the **Language Tour** for a guided walkthrough of expressions, control flow, and components. Then jump to the Reference when you need exact rules.

- [Language Tour](/language-tour/elements)
- [Reference](/reference/)

## 5) Next steps
- Tweak the `Hello` component to accept optional fields (`string?`) and add conditional branches.
- Try a `for` loop to render a list of users.
- Move on to the **Building Your First Component** tutorial to flesh out interaction patterns.
