//! # faktura
//!
//! Comprehensive German e-invoicing library covering the full lifecycle:
//! XRechnung, ZUGFeRD, DATEV, GDPdU, VAT validation, and Peppol.
//!
//! All monetary values use [`rust_decimal::Decimal`] — never floating point.
//! The core types follow the [EN 16931](https://standards.cencenelec.eu/dyn/www/f?p=205:110:0::::FSP_PROJECT:60602) semantic model.
//!
//! ## Quick Start
//!
//! ```rust
//! use chrono::NaiveDate;
//! use faktura::core::*;
//! use rust_decimal_macros::dec;
//!
//! let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
//!     .seller(PartyBuilder::new("ACME GmbH", AddressBuilder::new("Berlin", "10115", "DE").build())
//!         .vat_id("DE123456789").build())
//!     .buyer(PartyBuilder::new("Kunde AG", AddressBuilder::new("München", "80331", "DE").build()).build())
//!     .add_line(LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150))
//!         .tax(TaxCategory::StandardRate, dec!(19)).build())
//!     .build()
//!     .unwrap();
//!
//! assert!(validate_14_ustg(&invoice).is_empty());
//! assert_eq!(invoice.totals.unwrap().gross_total, dec!(1785.00));
//! ```
//!
//! ## Feature Flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `core` (default) | Invoice types, §14 UStG validation, numbering |
//! | `xrechnung` | XRechnung UBL/CII generation & parsing |
//! | `zugferd` | ZUGFeRD PDF/A-3 embed/extract |
//! | `datev` | DATEV Buchungsstapel EXTF CSV export |
//! | `gdpdu` | GDPdU/IDEA tax audit export |
//! | `vat` | VAT validation, VIES, Kleinunternehmer |
//! | `peppol` | Peppol BIS Billing 3.0 |
//! | `all` | Everything |

#[cfg(feature = "core")]
pub mod core;

#[cfg(feature = "xrechnung")]
pub mod xrechnung;

#[cfg(feature = "zugferd")]
pub mod zugferd;

#[cfg(feature = "datev")]
pub mod datev;

#[cfg(feature = "gdpdu")]
pub mod gdpdu;

#[cfg(feature = "vat")]
pub mod vat;

#[cfg(feature = "peppol")]
pub mod peppol;

// Re-export core types at crate root for convenience
#[cfg(feature = "core")]
pub use crate::core::*;
