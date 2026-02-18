//! DATEV Buchungsstapel EXTF CSV export.
//!
//! Generates EXTF-format CSV files compatible with DATEV import,
//! with BU-Schlüssel mapping and SKR03/SKR04 account plans.
//!
//! # Example
//!
//! ```ignore
//! use faktura::datev::*;
//!
//! let config = DatevConfig {
//!     consultant_number: 12345,
//!     client_number: 99999,
//!     fiscal_year_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
//!     account_length: 4,
//!     chart: ChartOfAccounts::SKR03,
//!     ..Default::default()
//! };
//!
//! let csv = to_extf(&[invoice], &config).unwrap();
//! ```

mod accounts;
mod bu_key;
mod extf;

pub use accounts::{
    AccountMapping, ChartOfAccounts, NamedAccount, account_by_name, account_by_number,
};
pub use bu_key::{BuSchluessel, bu_schluessel};
pub use extf::{DatevConfig, DatevConfigBuilder, DatevRow, to_extf};
