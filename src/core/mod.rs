//! Core invoice types, validation, and numbering.
//!
//! This module provides the foundational types for German invoicing
//! based on the EN 16931 semantic model, with §14 UStG validation.

mod builder;
mod error;
mod numbering;
mod types;
pub mod units;
mod validation;

pub use builder::*;
pub use error::*;
pub use numbering::*;
pub use types::*;
pub use units::is_known_unit_code;
pub use validation::*;
