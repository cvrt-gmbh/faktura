//! XRechnung XML generation and parsing (UBL + CII).
//!
//! Implements the German XRechnung standard (v3.0) based on EN 16931.
//!
//! # Supported syntaxes
//!
//! - **UBL 2.1** — OASIS Universal Business Language (`to_ubl_xml`, `from_ubl_xml`)
//! - **CII** — UN/CEFACT Cross Industry Invoice (`to_cii_xml`, `from_cii_xml`)
//!
//! # Example
//!
//! ```no_run
//! use faktura::core::*;
//! use faktura::xrechnung;
//!
//! let invoice: Invoice = todo!(); // build via InvoiceBuilder
//! let ubl_xml = xrechnung::to_ubl_xml(&invoice).unwrap();
//! let cii_xml = xrechnung::to_cii_xml(&invoice).unwrap();
//! ```

mod cii;
mod ubl;
mod validate;
pub(crate) mod xml_utils;

pub use cii::{from_cii_xml, to_cii_xml};
pub use ubl::{from_ubl_xml, to_ubl_xml};
pub use validate::validate_xrechnung;

/// XRechnung 3.0 specification identifier (BT-24).
pub const XRECHNUNG_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0";

/// Peppol BIS Billing 3.0 profile identifier (BT-23).
pub const PEPPOL_PROFILE_ID: &str = "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";

/// UBL 2.1 namespace URIs.
pub mod ubl_ns {
    pub const INVOICE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
    pub const CREDIT_NOTE: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
    pub const CAC: &str =
        "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
    pub const CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
}

/// CII namespace URIs.
pub mod cii_ns {
    pub const RSM: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
    pub const RAM: &str =
        "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
    pub const QDT: &str = "urn:un:unece:uncefact:data:standard:QualifiedDataType:100";
    pub const UDT: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";
}
