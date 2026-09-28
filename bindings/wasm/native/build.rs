//! Gives the WebAssembly module a linear-memory stack deep enough for its recursion limit.
//!
//! <para>A `wasm32` module keeps the part of its stack that does not fit in registers in linear
//! memory, a region fixed at link time and 1 MiB by default. Evaluating NX recurses through several
//! Rust frames per NX call, so at that size a recursive program such as
//! `let f(n:int): int = { f(n + 1) }` traps the instance around 220 calls deep, and a heavier call
//! sooner. 16 MiB leaves ample room for the module's `MAX_RECURSION_DEPTH`, so that limit, not the
//! linear-memory stack, is what ends runaway recursion.</para>
//!
//! <para>The engine's own native stack, which holds the wasm frames themselves, is a separate limit
//! a module cannot raise. `MAX_RECURSION_DEPTH` in `src/lib.rs` is set below it, as measured in a
//! browser worker.</para>

/// The linear-memory stack in bytes. It is memory the module reserves up front, not memory each
/// call touches.
const STACK_SIZE: u32 = 16 * 1024 * 1024;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size={STACK_SIZE}");
    }
}
