//! VAT validation, VIES integration, and Kleinunternehmer tracking.
//!
//! Validates VAT IDs by format and via the EU VIES API,
//! determines VAT scenarios, and tracks §19 UStG revenue thresholds.
//!
//! # Example
//!
//! ```ignore
//! use faktura::vat::*;
//!
//! // Format-only validation (no network)
//! assert!(validate_vat_format("DE123456789").is_ok());
//!
//! // VIES API check (async, requires network)
//! let result = check_vies("DE", "123456789").await?;
//! assert!(result.valid);
//!
//! // Kleinunternehmer threshold check
//! let status = check_kleinunternehmer(dec!(24000), dec!(90000));
//! assert!(status.eligible);
//! ```

mod format;
mod kleinunternehmer;
mod scenario;
mod vies;

pub use format::{VatFormatError, validate_steuernummer, validate_vat_format};
pub use kleinunternehmer::{
    KU_CURR_YEAR_LIMIT, KU_PREV_YEAR_LIMIT, KleinunternehmerStatus, check_kleinunternehmer,
};
pub use scenario::determine_scenario;
pub use vies::{ViesError, ViesResult, check_vies};
