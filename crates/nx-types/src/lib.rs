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

pub mod ty;
pub mod infer;
pub mod env;
pub mod check;

// Re-export main types
pub use ty::{Type, TypeId, Primitive};
pub use infer::{TypeInference, InferenceContext};
pub use env::{TypeEnvironment, TypeBinding};
pub use check::{TypeCheckResult, TypeCheckSession, check_str, check_file};
