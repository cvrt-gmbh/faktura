//! Core invoice types, validation, and numbering.
//!
//! This module provides the foundational types for German invoicing
//! based on the EN 16931 semantic model, with §14 UStG validation.

mod builder;
pub mod countries;
pub mod currencies;
mod error;
mod numbering;
pub mod reason_codes;
mod types;
pub mod units;
mod validation;

pub use builder::*;
pub use countries::is_known_country_code;
pub use currencies::is_known_currency_code;
pub use error::*;
pub use numbering::*;
pub use reason_codes::{is_known_allowance_reason, is_known_charge_reason};
pub use types::*;
pub use units::is_known_unit_code;
pub use validation::*;
