//! The NX IR runtime for Rust: prepares schema 5 images, links them by name, and evaluates them.
//!
//! <para>An image carries one module as flat tables over one string blob. [`PreparedModule::prepare`]
//! validates every section, offset and index of an image and indexes its declarations by name;
//! [`Program::link`] resolves the modules the entry names through a host resolver and checks
//! versions and referenced declarations eagerly; every evaluation API is then a method of the
//! linked [`Program`]. A prepared module is never copied by linking, so one prepared catalog
//! serves any number of programs, on any number of threads.</para>
//!
//! <para>This crate depends on the format crate `nx-ir` and the value crate `nx-value` and on no
//! part of the NX compiler. Values cross the API as [`nx_value::NxValue`]. Every API returns a
//! [`Result`] whose error is a list of diagnostics carrying the same `nx-ir-*` codes the
//! TypeScript runtime reports; nothing an image, a host value or an instance holds makes the
//! runtime panic.</para>

// The runtime is handed bytes and values it did not make. Everything that could panic on them is
// denied outside tests, so the absence of a panic is checked by the compiler and not only by the
// damage tests.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod component;
mod error;
mod eval;
mod helpers;
mod module;
mod normalize;
mod program;
mod stored;
mod text;
mod update;
mod value;

pub use component::{
    ComponentDispatchResult, ComponentInit, ComponentInitResult, ComponentInstance,
};
pub use error::{Diagnostic, NxIrRuntimeError, Result, SourceSpan};
pub use eval::{RuntimeOptions, NX_DEFAULT_MAX_CALL_DEPTH, NX_DEFAULT_MAX_RANGE_LENGTH};
pub use helpers::{apply, diff, merge};
pub use module::PreparedModule;
pub use nx_ir::{NX_IR_RUNTIME_ABI, NX_IR_SCHEMA_VERSION};
pub use program::{LinkOptions, Program, NX_PRELUDE_MODULE_IDENTITY};
pub use text::float32_text;

/// The README's examples, compiled as doc tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_modules_programs_and_instances_are_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<PreparedModule>();
        shareable::<Program>();
        shareable::<ComponentInstance>();
        shareable::<NxIrRuntimeError>();
    }
}
