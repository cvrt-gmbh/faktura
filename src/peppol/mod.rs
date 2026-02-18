//! Peppol BIS Billing 3.0 document generation and validation.
//!
//! Generates Peppol-compliant UBL 2.1 invoices for the Pan-European
//! Public Procurement OnLine network. Peppol BIS 3.0 is a CIUS of
//! EN 16931 — it uses the same UBL 2.1 schema with stricter rules.
//!
//! # Example
//!
//! ```ignore
//! use faktura::peppol;
//!
//! let xml = peppol::to_ubl_xml(&invoice).unwrap();
//! let errors = peppol::validate_peppol(&invoice);
//! ```

mod eas;
mod validate;

pub use eas::{EasScheme, eas_scheme_for_country};
pub use validate::validate_peppol;

use crate::core::{Invoice, RechnungError};

/// Peppol BIS Billing 3.0 customization identifier (BT-24).
pub const PEPPOL_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0";

/// Peppol BIS Billing 3.0 profile identifier (BT-23).
pub const PEPPOL_PROFILE_ID: &str = "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";

/// Peppol document type identifier for invoices (used in SMP routing).
pub const PEPPOL_INVOICE_DOCTYPE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2::Invoice##urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0::2.1";

/// Peppol document type identifier for credit notes (used in SMP routing).
pub const PEPPOL_CREDIT_NOTE_DOCTYPE: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2::CreditNote##urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0::2.1";

/// Generate a Peppol BIS 3.0 compliant UBL 2.1 Invoice XML.
///
/// This produces the same UBL structure as XRechnung but with the
/// Peppol-specific CustomizationID.
pub fn to_ubl_xml(invoice: &Invoice) -> Result<String, RechnungError> {
    // Generate XRechnung UBL (same structure, same ProfileID)
    let xml = crate::xrechnung::to_ubl_xml(invoice)?;

    // Replace XRechnung CustomizationID with Peppol CustomizationID
    Ok(xml.replace(
        crate::xrechnung::XRECHNUNG_CUSTOMIZATION_ID,
        PEPPOL_CUSTOMIZATION_ID,
    ))
}

/// Parse a Peppol BIS 3.0 UBL Invoice XML back into an Invoice.
///
/// Delegates to the XRechnung UBL parser since the structure is identical.
pub fn from_ubl_xml(xml: &str) -> Result<Invoice, RechnungError> {
    crate::xrechnung::from_ubl_xml(xml)
}
