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
pub use validate::{validate_xrechnung, validate_xrechnung_full};

use crate::core::{Invoice, RechnungError};

/// The detected XML syntax of an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlSyntax {
    /// UBL 2.1 (root element `Invoice` or `CreditNote`).
    Ubl,
    /// UN/CEFACT CII (root element `CrossIndustryInvoice`).
    Cii,
}

/// Parse an invoice from XML, auto-detecting whether it is UBL or CII.
///
/// Peeks at the root element to determine the syntax, then delegates to
/// [`from_ubl_xml`] or [`from_cii_xml`].
///
/// ```no_run
/// use faktura::xrechnung;
///
/// let xml = std::fs::read_to_string("invoice.xml").unwrap();
/// let (invoice, syntax) = xrechnung::from_xml(&xml).unwrap();
/// println!("Parsed {:?} invoice: {}", syntax, invoice.number);
/// ```
pub fn from_xml(xml: &str) -> Result<(Invoice, XmlSyntax), RechnungError> {
    match detect_syntax(xml) {
        Some(XmlSyntax::Ubl) => from_ubl_xml(xml).map(|inv| (inv, XmlSyntax::Ubl)),
        Some(XmlSyntax::Cii) => from_cii_xml(xml).map(|inv| (inv, XmlSyntax::Cii)),
        None => Err(RechnungError::Xml(
            "cannot detect XML syntax: root element is neither UBL (Invoice/CreditNote) nor CII (CrossIndustryInvoice)".into(),
        )),
    }
}

/// Detect the XML syntax by scanning for the root element name.
fn detect_syntax(xml: &str) -> Option<XmlSyntax> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = e.local_name();
                let name = std::str::from_utf8(local.as_ref()).unwrap_or("");
                return match name {
                    "Invoice" | "CreditNote" => Some(XmlSyntax::Ubl),
                    "CrossIndustryInvoice" => Some(XmlSyntax::Cii),
                    _ => None,
                };
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

/// XRechnung 3.0 specification identifier (BT-24).
pub const XRECHNUNG_CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0";

/// Peppol BIS Billing 3.0 profile identifier (BT-23).
pub const PEPPOL_PROFILE_ID: &str = "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";

/// UBL 2.1 namespace URIs.
pub mod ubl_ns {
    /// UBL Invoice namespace.
    pub const INVOICE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
    /// UBL CreditNote namespace.
    pub const CREDIT_NOTE: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
    /// Common Aggregate Components namespace.
    pub const CAC: &str =
        "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
    /// Common Basic Components namespace.
    pub const CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
}

/// CII namespace URIs.
pub mod cii_ns {
    /// CrossIndustryInvoice root namespace.
    pub const RSM: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
    /// Reusable Aggregate Business Information Entity namespace.
    pub const RAM: &str =
        "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
    /// Qualified Data Type namespace.
    pub const QDT: &str = "urn:un:unece:uncefact:data:standard:QualifiedDataType:100";
    /// Unqualified Data Type namespace.
    pub const UDT: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";
}
