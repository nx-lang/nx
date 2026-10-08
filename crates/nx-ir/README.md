# nx-ir

The NX IR image format for Rust: the form a compiled [NX](https://nxlang.org) module takes on disk
and on the wire.

An image is one module as flat tables over one string blob. This crate holds the format's constants
and kind numbers, the model an emitter fills, the writer that lays an image out, the reader that
validates one and reads it in place without copying, and an explainer that prints an image for
people. It depends on no other crate.

Most programs do not use it directly. To run compiled NX, depend on
[`nx-ir-runtime`](https://crates.io/crates/nx-ir-runtime), which reads images through this crate.
Use it directly to inspect or validate an image:

```rust,no_run
use nx_ir::{explain_nx_ir_image, NxIrImage, Table};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("module.nxir")?;
    // Validates the header, every section and every offset before anything is read.
    let image = NxIrImage::open(&bytes)?;
    println!("{} declarations", image.entry_count(Table::Declarations));
    println!("{}", explain_nx_ir_image(&bytes)?);
    Ok(())
}
```

The layout is documented in
[`docs/nx-ir-format.md`](https://github.com/nx-lang/nx/blob/main/docs/nx-ir-format.md). Images come
from `nxlang codegen --target nx-ir`.

The crate is versioned with the NX release it ships in, and its API may change between minor
versions while NX is `0.x`. Depend on it at the same version as `nx-ir-runtime`.
