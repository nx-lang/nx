//! NX type system - type checking and inference.
//!
//! This crate provides:
//! - Type representation (`Type` enum)
//! - Type inference for expressions
//! - Type checking and validation
//! - Type environment for symbol resolution
//!
//! # Quick Start
//!
//! ```
//! use nx_types::check_str;
//!
//! let source = "let <Button text:string /> = <button>{text}</button>";
//! let result = check_str(source, "example.nx");
//!
//! // Module should parse successfully
//! assert!(result.module.is_some());
//! ```

pub mod check;
pub mod env;
pub mod infer;
pub mod ty;

// Re-export main types
pub use check::{check_file, check_str, TypeCheckResult, TypeCheckSession};
pub use env::{TypeBinding, TypeEnvironment};
pub use infer::{InferenceContext, TypeInference};
pub use ty::{Primitive, Type, TypeId};
